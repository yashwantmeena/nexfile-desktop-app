use std::path::{Path, PathBuf};

use image::{imageops::FilterType, DynamicImage, GenericImageView};
use ndarray::{concatenate, Array2, Array3, Array4, ArrayD, Axis, Ix3};
use ort::{
    session::{builder::GraphOptimizationLevel, Session, SessionOutputs},
    value::Tensor,
};
use serde::{Deserialize, Serialize};
use tokenizers::{Tokenizer, TruncationParams};

const PIXEL_VALUES: &str = "pixel_values";
const INPUT_IDS: &str = "input_ids";
const INPUTS_EMBEDS: &str = "inputs_embeds";
const ATTENTION_MASK: &str = "attention_mask";
const ENCODER_ATTENTION_MASK: &str = "encoder_attention_mask";
const ENCODER_HIDDEN_STATES: &str = "encoder_hidden_states";
const IMAGE_FEATURES: &str = "image_features";
const LAST_HIDDEN_STATE: &str = "last_hidden_state";
const LOGITS: &str = "logits";

#[derive(Debug, thiserror::Error)]
pub enum Florence2Error {
    #[error("required Florence-2 file does not exist: {0}")]
    MissingFile(PathBuf),
    #[error("invalid Florence-2 configuration: {0}")]
    InvalidConfig(String),
    #[error("incompatible Florence-2 ONNX export: {0}")]
    IncompatibleModel(String),
    #[error("failed to load or preprocess image: {0}")]
    Image(#[from] image::ImageError),
    #[error("ONNX Runtime error: {0}")]
    Onnx(#[from] ort::Error),
    #[error("Florence-2 tokenizer error: {0}")]
    Tokenizer(String),
    #[error("Florence-2 tensor error: {0}")]
    Tensor(String),
    #[error("Florence-2 returned no usable token logits")]
    EmptyLogits,
}

/// Paths for the standard `onnx-community/Florence-2-base-ft` export layout.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Florence2ModelPaths {
    pub vision_encoder: PathBuf,
    pub token_embeddings: PathBuf,
    pub multimodal_encoder: PathBuf,
    pub decoder: PathBuf,
    pub tokenizer: PathBuf,
}

impl Florence2ModelPaths {
    /// Resolves model files from a Hugging Face snapshot directory containing
    /// `tokenizer.json` at its root and ONNX graphs under `onnx/`.
    pub fn from_dir(directory: impl AsRef<Path>) -> Self {
        Self::from_dir_with_suffix(directory, None)
    }

    /// Resolves a quantized export such as `_int8`, `_q4`, or `_quantized`.
    pub fn from_dir_with_suffix(directory: impl AsRef<Path>, suffix: Option<&str>) -> Self {
        let directory = directory.as_ref();
        let onnx_directory = directory.join("onnx");
        let suffix = suffix.unwrap_or("");
        Self {
            vision_encoder: onnx_directory.join(format!("vision_encoder{suffix}.onnx")),
            token_embeddings: onnx_directory.join(format!("embed_tokens{suffix}.onnx")),
            multimodal_encoder: onnx_directory.join(format!("encoder_model{suffix}.onnx")),
            decoder: onnx_directory.join(format!("decoder_model{suffix}.onnx")),
            tokenizer: directory.join("tokenizer.json"),
        }
    }

    fn validate(&self) -> Result<(), Florence2Error> {
        for path in [
            &self.vision_encoder,
            &self.token_embeddings,
            &self.multimodal_encoder,
            &self.decoder,
            &self.tokenizer,
        ] {
            if !path.is_file() {
                return Err(Florence2Error::MissingFile(path.clone()));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Florence2Config {
    pub image_size: u32,
    pub image_mean: [f32; 3],
    pub image_std: [f32; 3],
    pub max_prompt_tokens: usize,
    pub max_new_tokens: usize,
    pub decoder_start_token_id: u32,
    pub forced_bos_token_id: Option<u32>,
    pub eos_token_id: u32,
}

impl Default for Florence2Config {
    fn default() -> Self {
        Self {
            image_size: 768,
            image_mean: [0.485, 0.456, 0.406],
            image_std: [0.229, 0.224, 0.225],
            // The base model has 1024 encoder positions and uses 577 image tokens.
            max_prompt_tokens: 447,
            max_new_tokens: 128,
            decoder_start_token_id: 2,
            forced_bos_token_id: Some(0),
            eos_token_id: 2,
        }
    }
}

impl Florence2Config {
    fn validate(&self) -> Result<(), Florence2Error> {
        if self.image_size == 0 {
            return Err(Florence2Error::InvalidConfig(
                "image_size must be greater than zero".into(),
            ));
        }
        if self.max_prompt_tokens == 0 || self.max_new_tokens == 0 {
            return Err(Florence2Error::InvalidConfig(
                "token limits must be greater than zero".into(),
            ));
        }
        if self.image_std.iter().any(|value| *value <= 0.0) {
            return Err(Florence2Error::InvalidConfig(
                "all image standard deviations must be positive".into(),
            ));
        }
        Ok(())
    }
}

/// Prompt-based Florence-2 tasks supported by the official processor.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "task", content = "input")]
pub enum Florence2Task {
    Ocr,
    OcrWithRegion,
    Caption,
    DetailedCaption,
    MoreDetailedCaption,
    ObjectDetection,
    DenseRegionCaption,
    RegionProposal,
    CaptionToPhraseGrounding(String),
    ReferringExpressionSegmentation(String),
    RegionToSegmentation(String),
    OpenVocabularyDetection(String),
    RegionToCategory(String),
    RegionToDescription(String),
    RegionToOcr(String),
}

impl Florence2Task {
    pub fn prompt(&self) -> String {
        match self {
            Self::Ocr => "What is the text in the image?".into(),
            Self::OcrWithRegion => "What is the text in the image, with regions?".into(),
            Self::Caption => "What does the image describe?".into(),
            Self::DetailedCaption => "Describe in detail what is shown in the image.".into(),
            Self::MoreDetailedCaption => {
                "Describe with a paragraph what is shown in the image.".into()
            }
            Self::ObjectDetection => "Locate the objects with category name in the image.".into(),
            Self::DenseRegionCaption => {
                "Locate the objects in the image, with their descriptions.".into()
            }
            Self::RegionProposal => "Locate the region proposals in the image.".into(),
            Self::CaptionToPhraseGrounding(input) => {
                format!("Locate the phrases in the caption: {input}")
            }
            Self::ReferringExpressionSegmentation(input) => {
                format!("Locate {input} in the image with mask")
            }
            Self::RegionToSegmentation(input) => {
                format!("What is the polygon mask of region {input}")
            }
            Self::OpenVocabularyDetection(input) => format!("Locate {input} in the image."),
            Self::RegionToCategory(input) => format!("What is the region {input}?"),
            Self::RegionToDescription(input) => {
                format!("What does the region {input} describe?")
            }
            Self::RegionToOcr(input) => format!("What text is in the region {input}?"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Florence2Output {
    pub task: Florence2Task,
    /// Raw Florence-2 text. Location tokens are preserved for detection tasks.
    pub text: String,
    pub token_ids: Vec<u32>,
    pub image_width: u32,
    pub image_height: u32,
}

/// Florence-2 ONNX inference using a non-cached, greedy decoder.
///
/// The four sessions correspond to image feature extraction, token embedding,
/// multimodal encoding, and autoregressive text decoding.
pub struct Florence2Model {
    vision_encoder: Session,
    token_embeddings: Session,
    multimodal_encoder: Session,
    decoder: Session,
    tokenizer: Tokenizer,
    config: Florence2Config,
}

impl Florence2Model {
    pub fn load(paths: Florence2ModelPaths) -> Result<Self, Florence2Error> {
        Self::load_with_config(paths, Florence2Config::default())
    }

    pub fn load_with_config(
        paths: Florence2ModelPaths,
        config: Florence2Config,
    ) -> Result<Self, Florence2Error> {
        paths.validate()?;
        config.validate()?;

        let vision_encoder = build_session(&paths.vision_encoder)?;
        validate_session(
            &vision_encoder,
            "vision encoder",
            &[PIXEL_VALUES],
            &[IMAGE_FEATURES],
        )?;

        let token_embeddings = build_session(&paths.token_embeddings)?;
        validate_session(
            &token_embeddings,
            "token embedding model",
            &[INPUT_IDS],
            &[INPUTS_EMBEDS],
        )?;

        let multimodal_encoder = build_session(&paths.multimodal_encoder)?;
        validate_session(
            &multimodal_encoder,
            "multimodal encoder",
            &[INPUTS_EMBEDS, ATTENTION_MASK],
            &[LAST_HIDDEN_STATE],
        )?;

        let decoder = build_session(&paths.decoder)?;
        validate_session(
            &decoder,
            "decoder",
            &[
                INPUTS_EMBEDS,
                ATTENTION_MASK,
                ENCODER_ATTENTION_MASK,
                ENCODER_HIDDEN_STATES,
            ],
            &[LOGITS],
        )?;

        let mut tokenizer = Tokenizer::from_file(&paths.tokenizer)
            .map_err(|error| Florence2Error::Tokenizer(error.to_string()))?;
        tokenizer
            .with_truncation(Some(TruncationParams {
                max_length: config.max_prompt_tokens,
                ..Default::default()
            }))
            .map_err(|error| Florence2Error::Tokenizer(error.to_string()))?;
        tokenizer.with_padding(None);

        Ok(Self {
            vision_encoder,
            token_embeddings,
            multimodal_encoder,
            decoder,
            tokenizer,
            config,
        })
    }

    pub fn generate_path(
        &mut self,
        path: impl AsRef<Path>,
        task: Florence2Task,
    ) -> Result<Florence2Output, Florence2Error> {
        let image = image::open(path)?;
        self.generate(&image, task)
    }

    pub fn generate(
        &mut self,
        image: &DynamicImage,
        task: Florence2Task,
    ) -> Result<Florence2Output, Florence2Error> {
        let original_size = image.dimensions();
        let pixel_values = preprocess_image(image, &self.config)?;
        let prompt_encoding = self
            .tokenizer
            .encode(task.prompt(), true)
            .map_err(|error| Florence2Error::Tokenizer(error.to_string()))?;

        let prompt_ids = to_i64_ids(prompt_encoding.get_ids());
        let prompt_mask = prompt_encoding
            .get_attention_mask()
            .iter()
            .map(|value| i64::from(*value))
            .collect::<Vec<_>>();

        let image_features = run_f32(
            &mut self.vision_encoder,
            ort::inputs![PIXEL_VALUES => Tensor::from_array(pixel_values)?],
            IMAGE_FEATURES,
        )?
        .into_dimensionality::<Ix3>()
        .map_err(|error| Florence2Error::Tensor(error.to_string()))?;

        let text_features = self.embed_tokens(&prompt_ids)?;
        let inputs_embeds = concatenate(Axis(1), &[image_features.view(), text_features.view()])
            .map_err(|error| Florence2Error::Tensor(error.to_string()))?;

        let image_token_count = image_features.shape()[1];
        let mut encoder_mask = vec![1_i64; image_token_count];
        encoder_mask.extend(prompt_mask);
        let encoder_attention_mask = Array2::from_shape_vec((1, encoder_mask.len()), encoder_mask)
            .map_err(|error| Florence2Error::Tensor(error.to_string()))?;

        let encoder_hidden_states = run_f32(
            &mut self.multimodal_encoder,
            ort::inputs![
                INPUTS_EMBEDS => Tensor::from_array(inputs_embeds)?,
                ATTENTION_MASK => Tensor::from_array(encoder_attention_mask.clone())?,
            ],
            LAST_HIDDEN_STATE,
        )?
        .into_dimensionality::<Ix3>()
        .map_err(|error| Florence2Error::Tensor(error.to_string()))?;

        let token_ids = self.decode_greedy(&encoder_hidden_states, &encoder_attention_mask)?;
        let decoded = self
            .tokenizer
            .decode(&token_ids, false)
            .map_err(|error| Florence2Error::Tokenizer(error.to_string()))?;

        Ok(Florence2Output {
            task,
            text: clean_decoded_text(&decoded),
            token_ids,
            image_width: original_size.0,
            image_height: original_size.1,
        })
    }

    fn embed_tokens(&mut self, token_ids: &[i64]) -> Result<Array3<f32>, Florence2Error> {
        let ids = Array2::from_shape_vec((1, token_ids.len()), token_ids.to_vec())
            .map_err(|error| Florence2Error::Tensor(error.to_string()))?;
        run_f32(
            &mut self.token_embeddings,
            ort::inputs![INPUT_IDS => Tensor::from_array(ids)?],
            INPUTS_EMBEDS,
        )?
        .into_dimensionality::<Ix3>()
        .map_err(|error| Florence2Error::Tensor(error.to_string()))
    }

    fn decode_greedy(
        &mut self,
        encoder_hidden_states: &Array3<f32>,
        encoder_attention_mask: &Array2<i64>,
    ) -> Result<Vec<u32>, Florence2Error> {
        let mut generated = vec![self.config.decoder_start_token_id];

        for step in 0..self.config.max_new_tokens {
            let next_token = if step == 0 {
                self.config.forced_bos_token_id
            } else {
                None
            };

            let next_token = match next_token {
                Some(token) => token,
                None => {
                    let generated_i64 = generated
                        .iter()
                        .map(|token| i64::from(*token))
                        .collect::<Vec<_>>();
                    let decoder_inputs = self.embed_tokens(&generated_i64)?;
                    let decoder_attention_mask = Array2::from_elem((1, generated.len()), 1_i64);

                    let logits = run_f32(
                        &mut self.decoder,
                        ort::inputs![
                            INPUTS_EMBEDS => Tensor::from_array(decoder_inputs)?,
                            ATTENTION_MASK => Tensor::from_array(decoder_attention_mask)?,
                            ENCODER_ATTENTION_MASK => Tensor::from_array(encoder_attention_mask.clone())?,
                            ENCODER_HIDDEN_STATES => Tensor::from_array(encoder_hidden_states.clone())?,
                        ],
                        LOGITS,
                    )?;
                    argmax_last_token(&logits)? as u32
                }
            };

            generated.push(next_token);
            if next_token == self.config.eos_token_id {
                break;
            }
        }

        Ok(generated)
    }
}

fn build_session(model_path: &Path) -> Result<Session, Florence2Error> {
    Ok(Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(ort::Error::from)?
        .commit_from_file(model_path)?)
}

fn validate_session(
    session: &Session,
    label: &str,
    required_inputs: &[&str],
    required_outputs: &[&str],
) -> Result<(), Florence2Error> {
    let input_names = session
        .inputs()
        .iter()
        .map(|input| input.name())
        .collect::<Vec<_>>();
    let output_names = session
        .outputs()
        .iter()
        .map(|output| output.name())
        .collect::<Vec<_>>();

    let missing_inputs = required_inputs
        .iter()
        .copied()
        .filter(|name| !input_names.contains(name))
        .collect::<Vec<_>>();
    let missing_outputs = required_outputs
        .iter()
        .copied()
        .filter(|name| !output_names.contains(name))
        .collect::<Vec<_>>();

    if missing_inputs.is_empty() && missing_outputs.is_empty() {
        return Ok(());
    }

    Err(Florence2Error::IncompatibleModel(format!(
        "{label} is missing inputs [{}] or outputs [{}]; available inputs [{}], outputs [{}]",
        missing_inputs.join(", "),
        missing_outputs.join(", "),
        input_names.join(", "),
        output_names.join(", ")
    )))
}

fn preprocess_image(
    image: &DynamicImage,
    config: &Florence2Config,
) -> Result<Array4<f32>, Florence2Error> {
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return Err(Florence2Error::InvalidConfig(
            "image has zero dimensions".into(),
        ));
    }

    let size = config.image_size;
    let resized = image::imageops::resize(&image.to_rgb8(), size, size, FilterType::CatmullRom);
    let mut tensor = Array4::<f32>::zeros((1, 3, size as usize, size as usize));
    for (x, y, pixel) in resized.enumerate_pixels() {
        for channel in 0..3 {
            let scaled = f32::from(pixel[channel]) / 255.0;
            tensor[[0, channel, y as usize, x as usize]] =
                (scaled - config.image_mean[channel]) / config.image_std[channel];
        }
    }
    Ok(tensor)
}

fn run_f32<'a>(
    session: &mut Session,
    inputs: Vec<(
        std::borrow::Cow<'a, str>,
        ort::session::SessionInputValue<'a>,
    )>,
    output_name: &str,
) -> Result<ArrayD<f32>, Florence2Error> {
    let outputs = session.run(inputs)?;
    extract_f32(&outputs, output_name)
}

fn extract_f32(
    outputs: &SessionOutputs<'_>,
    output_name: &str,
) -> Result<ArrayD<f32>, Florence2Error> {
    let output = outputs.get(output_name).ok_or_else(|| {
        Florence2Error::IncompatibleModel(format!("inference did not return `{output_name}`"))
    })?;
    Ok(output.try_extract_array::<f32>()?.to_owned())
}

fn argmax_last_token(logits: &ArrayD<f32>) -> Result<usize, Florence2Error> {
    let shape = logits.shape();
    if shape.len() != 3 || shape[0] != 1 || shape[1] == 0 || shape[2] == 0 {
        return Err(Florence2Error::EmptyLogits);
    }

    let sequence_index = shape[1] - 1;
    let vocabulary_size = shape[2];
    (0..vocabulary_size)
        .max_by(|left, right| {
            logits[[0, sequence_index, *left]].total_cmp(&logits[[0, sequence_index, *right]])
        })
        .ok_or(Florence2Error::EmptyLogits)
}

fn to_i64_ids(token_ids: &[u32]) -> Vec<i64> {
    token_ids.iter().map(|token| i64::from(*token)).collect()
}

fn clean_decoded_text(text: &str) -> String {
    ["<s>", "</s>", "<pad>"]
        .into_iter()
        .fold(text.to_owned(), |text, token| text.replace(token, ""))
        .trim()
        .to_owned()
}
