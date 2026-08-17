use std::path::{Path, PathBuf};

use image::{imageops::FilterType, DynamicImage, GenericImageView};
use ndarray::{Array2, Array4};
use ort::{
    session::{builder::GraphOptimizationLevel, Session, SessionOutputs},
    value::Tensor,
};
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer, TruncationParams};

const VISION_INPUT: &str = "pixel_values";
const TEXT_INPUT: &str = "input_ids";
const ATTENTION_MASK_INPUT: &str = "attention_mask";
const VISION_OUTPUT: &str = "image_embeds";
const TEXT_OUTPUT: &str = "text_embeds";

/// A unit-length CLIP vector in the shared image/text embedding space.
pub type Embedding = Vec<f32>;

#[derive(Debug, thiserror::Error)]
pub enum ClipError {
    #[error("required CLIP file does not exist: {0}")]
    MissingFile(PathBuf),
    #[error("invalid CLIP configuration: {0}")]
    InvalidConfig(String),
    #[error("incompatible CLIP ONNX export: {0}")]
    IncompatibleModel(String),
    #[error("failed to load or preprocess image: {0}")]
    Image(#[from] image::ImageError),
    #[error("ONNX Runtime error: {0}")]
    Onnx(#[from] ort::Error),
    #[error("CLIP tokenizer error: {0}")]
    Tokenizer(String),
    #[error("cannot compare embeddings with dimensions {left} and {right}")]
    DimensionMismatch { left: usize, right: usize },
    #[error("the model returned an empty or zero-length embedding")]
    EmptyEmbedding,
}

/// Files required by a standard split CLIP ONNX export.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClipModelPaths {
    pub vision_model: PathBuf,
    pub text_model: PathBuf,
    pub tokenizer: PathBuf,
}

impl ClipModelPaths {
    /// Resolves the conventional filenames within an AI model directory.
    pub fn from_dir(directory: impl AsRef<Path>) -> Self {
        let directory = directory.as_ref();
        Self {
            vision_model: directory.join("vision_model.onnx"),
            text_model: directory.join("text_model.onnx"),
            tokenizer: directory.join("tokenizer.json"),
        }
    }

    fn validate(&self) -> Result<(), ClipError> {
        for path in [&self.vision_model, &self.text_model, &self.tokenizer] {
            if !path.is_file() {
                return Err(ClipError::MissingFile(path.clone()));
            }
        }
        Ok(())
    }
}

/// Preprocessing settings for OpenAI CLIP-compatible models.
#[derive(Clone, Debug, PartialEq)]
pub struct ClipConfig {
    pub image_size: u32,
    pub context_length: usize,
    pub image_mean: [f32; 3],
    pub image_std: [f32; 3],
}

impl Default for ClipConfig {
    fn default() -> Self {
        Self {
            image_size: 224,
            context_length: 77,
            image_mean: [0.481_454_66, 0.457_827_5, 0.408_210_73],
            image_std: [0.268_629_54, 0.261_302_6, 0.275_777_1],
        }
    }
}

impl ClipConfig {
    fn validate(&self) -> Result<(), ClipError> {
        if self.image_size == 0 {
            return Err(ClipError::InvalidConfig(
                "image_size must be greater than zero".into(),
            ));
        }
        if self.context_length < 2 {
            return Err(ClipError::InvalidConfig(
                "context_length must allow CLIP start and end tokens".into(),
            ));
        }
        if self.image_std.iter().any(|value| *value <= 0.0) {
            return Err(ClipError::InvalidConfig(
                "all image standard deviations must be positive".into(),
            ));
        }
        Ok(())
    }
}

/// Runs split CLIP vision and text encoders with ONNX Runtime.
///
/// `ClipModel` owns mutable ONNX sessions. Put it behind a mutex when sharing it
/// from Tauri managed state.
pub struct ClipModel {
    vision_session: Session,
    text_session: Session,
    tokenizer: Tokenizer,
    config: ClipConfig,
    text_uses_attention_mask: bool,
}

impl ClipModel {
    pub fn load(paths: ClipModelPaths) -> Result<Self, ClipError> {
        Self::load_with_config(paths, ClipConfig::default())
    }

    pub fn load_with_config(paths: ClipModelPaths, config: ClipConfig) -> Result<Self, ClipError> {
        paths.validate()?;
        config.validate()?;

        let vision_session = build_session(&paths.vision_model)?;
        validate_input(&vision_session, VISION_INPUT, "vision")?;
        validate_output(&vision_session, VISION_OUTPUT, "vision")?;

        let text_session = build_session(&paths.text_model)?;
        validate_input(&text_session, TEXT_INPUT, "text")?;
        validate_output(&text_session, TEXT_OUTPUT, "text")?;
        let text_uses_attention_mask = text_session
            .inputs()
            .iter()
            .any(|input| input.name() == ATTENTION_MASK_INPUT);

        let unsupported_text_inputs = text_session
            .inputs()
            .iter()
            .map(|input| input.name())
            .filter(|name| *name != TEXT_INPUT && *name != ATTENTION_MASK_INPUT)
            .collect::<Vec<_>>();
        if !unsupported_text_inputs.is_empty() {
            return Err(ClipError::IncompatibleModel(format!(
                "text encoder has unsupported required inputs: {}",
                unsupported_text_inputs.join(", ")
            )));
        }

        let mut tokenizer = Tokenizer::from_file(&paths.tokenizer)
            .map_err(|error| ClipError::Tokenizer(error.to_string()))?;
        configure_tokenizer(&mut tokenizer, config.context_length)?;

        Ok(Self {
            vision_session,
            text_session,
            tokenizer,
            config,
            text_uses_attention_mask,
        })
    }

    pub fn embed_image_path(&mut self, path: impl AsRef<Path>) -> Result<Embedding, ClipError> {
        let image = image::open(path)?;
        self.embed_image(&image)
    }

    pub fn embed_image(&mut self, image: &DynamicImage) -> Result<Embedding, ClipError> {
        let pixel_values = preprocess_image(image, &self.config)?;
        let outputs = self.vision_session.run(ort::inputs![
            VISION_INPUT => Tensor::from_array(pixel_values)?,
        ])?;
        extract_normalized_embedding(&outputs, VISION_OUTPUT)
    }

    pub fn embed_text(&mut self, text: &str) -> Result<Embedding, ClipError> {
        if text.trim().is_empty() {
            return Err(ClipError::Tokenizer("search text cannot be empty".into()));
        }

        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|error| ClipError::Tokenizer(error.to_string()))?;
        let sequence_length = encoding.get_ids().len();
        let input_ids = Array2::from_shape_vec(
            (1, sequence_length),
            encoding.get_ids().iter().map(|id| i64::from(*id)).collect(),
        )
        .map_err(|error| ClipError::Tokenizer(error.to_string()))?;

        let outputs = if self.text_uses_attention_mask {
            let attention_mask = Array2::from_shape_vec(
                (1, sequence_length),
                encoding
                    .get_attention_mask()
                    .iter()
                    .map(|value| i64::from(*value))
                    .collect(),
            )
            .map_err(|error| ClipError::Tokenizer(error.to_string()))?;

            self.text_session.run(ort::inputs![
                TEXT_INPUT => Tensor::from_array(input_ids)?,
                ATTENTION_MASK_INPUT => Tensor::from_array(attention_mask)?,
            ])?
        } else {
            self.text_session
                .run(ort::inputs![TEXT_INPUT => Tensor::from_array(input_ids)?])?
        };

        extract_normalized_embedding(&outputs, TEXT_OUTPUT)
    }

    /// Cosine similarity for an image/text embedding pair.
    pub fn cosine_similarity(left: &[f32], right: &[f32]) -> Result<f32, ClipError> {
        if left.len() != right.len() {
            return Err(ClipError::DimensionMismatch {
                left: left.len(),
                right: right.len(),
            });
        }
        if left.is_empty() {
            return Err(ClipError::EmptyEmbedding);
        }

        let dot = left
            .iter()
            .zip(right)
            .map(|(left, right)| left * right)
            .sum::<f32>();
        let left_norm = left.iter().map(|value| value * value).sum::<f32>().sqrt();
        let right_norm = right.iter().map(|value| value * value).sum::<f32>().sqrt();
        if left_norm <= f32::EPSILON || right_norm <= f32::EPSILON {
            return Err(ClipError::EmptyEmbedding);
        }
        Ok((dot / (left_norm * right_norm)).clamp(-1.0, 1.0))
    }
}

fn build_session(model_path: &Path) -> Result<Session, ClipError> {
    Ok(Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(ort::Error::from)?
        .commit_from_file(model_path)?)
}

fn validate_input(session: &Session, expected: &str, encoder: &str) -> Result<(), ClipError> {
    if session
        .inputs()
        .iter()
        .any(|input| input.name() == expected)
    {
        return Ok(());
    }
    Err(ClipError::IncompatibleModel(format!(
        "{encoder} encoder is missing `{expected}`; available inputs: {}",
        names(session.inputs().iter().map(|input| input.name()))
    )))
}

fn validate_output(session: &Session, expected: &str, encoder: &str) -> Result<(), ClipError> {
    if session
        .outputs()
        .iter()
        .any(|output| output.name() == expected)
    {
        return Ok(());
    }
    Err(ClipError::IncompatibleModel(format!(
        "{encoder} encoder is missing `{expected}`; available outputs: {}",
        names(session.outputs().iter().map(|output| output.name()))
    )))
}

fn names<'a>(values: impl Iterator<Item = &'a str>) -> String {
    let values = values.collect::<Vec<_>>();
    if values.is_empty() {
        "<none>".into()
    } else {
        values.join(", ")
    }
}

fn configure_tokenizer(tokenizer: &mut Tokenizer, context_length: usize) -> Result<(), ClipError> {
    tokenizer
        .with_truncation(Some(TruncationParams {
            max_length: context_length,
            ..Default::default()
        }))
        .map_err(|error| ClipError::Tokenizer(error.to_string()))?;

    let mut padding = tokenizer.get_padding().cloned().unwrap_or_else(|| {
        let pad_id = tokenizer.token_to_id("<|endoftext|>").unwrap_or(0);
        PaddingParams {
            pad_id,
            pad_token: "<|endoftext|>".into(),
            ..Default::default()
        }
    });
    padding.strategy = PaddingStrategy::Fixed(context_length);
    tokenizer.with_padding(Some(padding));
    Ok(())
}

fn preprocess_image(image: &DynamicImage, config: &ClipConfig) -> Result<Array4<f32>, ClipError> {
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return Err(ClipError::InvalidConfig("image has zero dimensions".into()));
    }

    let size = config.image_size;
    let scale = f64::from(size) / f64::from(width.min(height));
    let resized_width = (f64::from(width) * scale).round().max(f64::from(size)) as u32;
    let resized_height = (f64::from(height) * scale).round().max(f64::from(size)) as u32;
    let rgb = image.to_rgb8();
    let resized =
        image::imageops::resize(&rgb, resized_width, resized_height, FilterType::CatmullRom);
    let crop_x = (resized_width - size) / 2;
    let crop_y = (resized_height - size) / 2;
    let cropped = image::imageops::crop_imm(&resized, crop_x, crop_y, size, size).to_image();

    let mut tensor = Array4::<f32>::zeros((1, 3, size as usize, size as usize));
    for (x, y, pixel) in cropped.enumerate_pixels() {
        for channel in 0..3 {
            let scaled = f32::from(pixel[channel]) / 255.0;
            tensor[[0, channel, y as usize, x as usize]] =
                (scaled - config.image_mean[channel]) / config.image_std[channel];
        }
    }
    Ok(tensor)
}

fn extract_normalized_embedding(
    outputs: &SessionOutputs<'_>,
    output_name: &str,
) -> Result<Embedding, ClipError> {
    let output = outputs.get(output_name).ok_or_else(|| {
        ClipError::IncompatibleModel(format!("inference did not return `{output_name}`"))
    })?;
    let values = output.try_extract_array::<f32>()?;
    let mut embedding = values.iter().copied().collect::<Vec<_>>();
    normalize(&mut embedding)?;
    Ok(embedding)
}

fn normalize(embedding: &mut [f32]) -> Result<(), ClipError> {
    let norm = embedding
        .iter()
        .map(|value| value * value)
        .sum::<f32>()
        .sqrt();
    if embedding.is_empty() || norm <= f32::EPSILON || !norm.is_finite() {
        return Err(ClipError::EmptyEmbedding);
    }
    embedding.iter_mut().for_each(|value| *value /= norm);
    Ok(())
}
