use std::collections::HashSet;
use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use image::{codecs::jpeg::JpegEncoder, imageops::FilterType, DynamicImage};

use crate::ai_models::clip::{ClipModel, ClipModelPaths, Embedding};
use crate::error::{AppError, AppResult, ClassifierLockPoisoned, ImagePreparationError};
use crate::models::image_processing_model::{
    ClassificationConfigDefinition, ClassificationPrediction, ImageClassificationOutput,
    ImageProcessingJob, ImageProcessingOutput, IMAGE_PROCESSING_OUTPUT_VERSION,
};
use crate::utils::constants::{
    CLIP_LOGIT_SCALE, MAX_MODEL_IMAGE_DIMENSION, MODEL_IMAGE_TEMP_DIRECTORY, MODEL_JPEG_QUALITY,
};
use crate::utils::image_decoder::decode_image;

#[derive(Clone)]
pub struct ImageProcessingService {
    model_directory: PathBuf,
    configs_directory: PathBuf,
    classifier: Arc<Mutex<Option<PreparedImageClassifier>>>,
}

impl ImageProcessingService {
    pub fn new(model_directory: PathBuf, configs_directory: PathBuf) -> Self {
        Self {
            model_directory,
            configs_directory,
            classifier: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn process(&self, job: ImageProcessingJob) -> AppResult<PathBuf> {
        let service = self.clone();
        tauri::async_runtime::spawn_blocking(move || service.process_blocking(job))
            .await
            .map_err(AppError::internal)?
    }

    fn process_blocking(&self, job: ImageProcessingJob) -> AppResult<PathBuf> {
        if !job.path.is_file() {
            return Err(AppError::validation(
                "The queued image-processing path is no longer a file.",
            ));
        }

        let output_path = classification_output_path(&job.path);
        if valid_existing_output(&output_path)? {
            return Ok(output_path);
        }

        let prepared_image = PreparedModelImage::prepare(&job.path).map_err(AppError::internal)?;

        let mut classifier = self
            .classifier
            .lock()
            .map_err(|_| AppError::internal(ClassifierLockPoisoned))?;
        if classifier.is_none() {
            *classifier = Some(PreparedImageClassifier::load(
                &self.model_directory,
                &self.configs_directory,
            )?);
        }
        let output = classifier
            .as_mut()
            .expect("classifier is initialized before use")
            .classify(prepared_image.path())?;
        write_output(&output_path, &output)?;
        Ok(output_path)
    }
}

/// A normalized JPEG that is deleted when the processing scope ends.
struct PreparedModelImage {
    path: PathBuf,
}

impl PreparedModelImage {
    fn prepare(source: &Path) -> Result<Self, ImagePreparationError> {
        let decoded = decode_image(source)?;
        if decoded.width() == 0 || decoded.height() == 0 {
            return Err(ImagePreparationError::Encode(image::ImageError::Limits(
                image::error::LimitError::from_kind(image::error::LimitErrorKind::DimensionError),
            )));
        }

        let resized = if decoded.width().max(decoded.height()) > MAX_MODEL_IMAGE_DIMENSION {
            decoded.resize(
                MAX_MODEL_IMAGE_DIMENSION,
                MAX_MODEL_IMAGE_DIMENSION,
                FilterType::Lanczos3,
            )
        } else {
            decoded
        };
        let rgb = flatten_onto_white(&resized);

        let directory = std::env::temp_dir().join(MODEL_IMAGE_TEMP_DIRECTORY);
        std::fs::create_dir_all(&directory)?;
        let path = directory.join(format!("{}.jpg", uuid::Uuid::new_v4()));
        let prepared = Self { path };
        if let Err(error) = write_jpeg(prepared.path(), &rgb) {
            drop(prepared);
            return Err(error);
        }
        Ok(prepared)
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for PreparedModelImage {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn flatten_onto_white(image: &DynamicImage) -> image::RgbImage {
    let rgba = image.to_rgba8();
    image::RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let pixel = rgba.get_pixel(x, y);
        let alpha = u16::from(pixel[3]);
        let inverse_alpha = 255 - alpha;
        image::Rgb([
            ((u16::from(pixel[0]) * alpha + 255 * inverse_alpha + 127) / 255) as u8,
            ((u16::from(pixel[1]) * alpha + 255 * inverse_alpha + 127) / 255) as u8,
            ((u16::from(pixel[2]) * alpha + 255 * inverse_alpha + 127) / 255) as u8,
        ])
    })
}

fn write_jpeg(path: &Path, image: &image::RgbImage) -> Result<(), ImagePreparationError> {
    let file = OpenOptions::new().write(true).create_new(true).open(path)?;
    let mut writer = BufWriter::new(file);
    JpegEncoder::new_with_quality(&mut writer, MODEL_JPEG_QUALITY).encode_image(image)?;
    writer.flush()?;
    Ok(())
}

struct PreparedImageClassifier {
    model: ClipModel,
    configs: Vec<PreparedClassificationConfig>,
}

impl PreparedImageClassifier {
    fn load(model_directory: &Path, configs_directory: &Path) -> AppResult<Self> {
        let mut model = ClipModel::load(ClipModelPaths::from_dir(model_directory))
            .map_err(AppError::internal)?;
        let definitions = load_config_definitions(configs_directory)?;
        let mut configs = Vec::with_capacity(definitions.len());

        for definition in definitions {
            let mut labels = Vec::with_capacity(definition.configuration.labels.len());
            for (label, prompt) in definition.configuration.labels {
                let embedding = model.embed_text(&prompt).map_err(AppError::internal)?;
                labels.push(PreparedLabel { label, embedding });
            }
            configs.push(PreparedClassificationConfig {
                label: definition.label,
                parent_label: definition.parent_label,
                level: definition.level,
                threshold: definition.configuration.threshold,
                multilabel: definition.configuration.multilabel,
                normalize_scores: definition.configuration.normalize_scores,
                labels,
            });
        }

        if find_config(&configs, "primary", "primary", None).is_none() {
            return Err(AppError::validation(
                "The primary image-classification configuration is missing.",
            ));
        }

        Ok(Self { model, configs })
    }

    fn classify(&mut self, image_path: &Path) -> AppResult<ImageProcessingOutput> {
        let image_embedding = self
            .model
            .embed_image_path(image_path)
            .map_err(AppError::internal)?;
        let primary_config = find_config(&self.configs, "primary", "primary", None)
            .expect("primary configuration is validated during loading");
        let primary = classify_with_config(primary_config, &image_embedding)?;

        let mut classification = ImageClassificationOutput {
            primary: primary.clone(),
            ..Default::default()
        };

        for primary_prediction in primary {
            let Some(secondary_config) = find_config(
                &self.configs,
                "secondary",
                &primary_prediction.label,
                Some("primary"),
            ) else {
                continue;
            };
            let secondary = classify_with_config(secondary_config, &image_embedding)?;
            classification.secondary.extend(secondary.clone());

            for secondary_prediction in secondary {
                let Some(tertiary_config) = find_config(
                    &self.configs,
                    "tertiary",
                    &secondary_prediction.label,
                    Some(&primary_prediction.label),
                ) else {
                    continue;
                };
                classification
                    .tertiary
                    .extend(classify_with_config(tertiary_config, &image_embedding)?);
            }
        }

        Ok(ImageProcessingOutput {
            version: IMAGE_PROCESSING_OUTPUT_VERSION,
            classification,
        })
    }
}

struct PreparedClassificationConfig {
    label: String,
    parent_label: Option<String>,
    level: String,
    threshold: f32,
    multilabel: bool,
    normalize_scores: bool,
    labels: Vec<PreparedLabel>,
}

struct PreparedLabel {
    label: String,
    embedding: Embedding,
}

fn load_config_definitions(directory: &Path) -> AppResult<Vec<ClassificationConfigDefinition>> {
    if !directory.is_dir() {
        return Err(AppError::validation(format!(
            "The image-classification configuration directory does not exist: {}",
            directory.display()
        )));
    }

    let mut paths = std::fs::read_dir(directory)?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
        })
        .collect::<Vec<_>>();
    paths.sort();

    let mut definitions = Vec::new();
    let mut keys = HashSet::new();
    for path in paths {
        let bytes = std::fs::read(&path)?;
        let definition = serde_json::from_slice::<ClassificationConfigDefinition>(&bytes)
            .map_err(AppError::serialization)?;
        if !definition.enabled
            || definition.modality != "image"
            || definition.task != "image-classification"
        {
            continue;
        }
        if definition.configuration.labels.is_empty() {
            return Err(AppError::validation(format!(
                "The classification configuration has no labels: {}",
                path.display()
            )));
        }
        if !(0.0..=1.0).contains(&definition.configuration.threshold) {
            return Err(AppError::validation(format!(
                "The classification threshold must be between zero and one: {}",
                path.display()
            )));
        }
        let key = (
            definition.level.clone(),
            definition.parent_label.clone(),
            definition.label.clone(),
        );
        if !keys.insert(key) {
            return Err(AppError::validation(format!(
                "Duplicate image-classification configuration: {}",
                path.display()
            )));
        }
        definitions.push(definition);
    }
    Ok(definitions)
}

fn find_config<'a>(
    configs: &'a [PreparedClassificationConfig],
    level: &str,
    label: &str,
    parent_label: Option<&str>,
) -> Option<&'a PreparedClassificationConfig> {
    configs.iter().find(|config| {
        config.level == level
            && config.label == label
            && config.parent_label.as_deref() == parent_label
    })
}

fn classify_with_config(
    config: &PreparedClassificationConfig,
    image_embedding: &[f32],
) -> AppResult<Vec<ClassificationPrediction>> {
    let raw_scores = config
        .labels
        .iter()
        .map(|label| ClipModel::cosine_similarity(image_embedding, &label.embedding))
        .collect::<Result<Vec<_>, _>>()
        .map_err(AppError::internal)?;
    Ok(select_predictions(config, raw_scores))
}

fn select_predictions(
    config: &PreparedClassificationConfig,
    raw_scores: Vec<f32>,
) -> Vec<ClassificationPrediction> {
    let scores = if config.normalize_scores {
        softmax(&raw_scores, CLIP_LOGIT_SCALE)
    } else {
        raw_scores
    };
    let mut predictions = config
        .labels
        .iter()
        .zip(scores)
        .map(|(label, score)| ClassificationPrediction {
            label: label.label.clone(),
            parent_label: (config.level != "primary").then(|| config.label.clone()),
            score,
        })
        .collect::<Vec<_>>();
    predictions.sort_by(|left, right| right.score.total_cmp(&left.score));

    if config.multilabel {
        let best = predictions.first().cloned();
        predictions.retain(|prediction| prediction.score >= config.threshold);
        if predictions.is_empty() {
            predictions.extend(best);
        }
    } else {
        predictions.truncate(1);
    }
    predictions
}

fn softmax(scores: &[f32], scale: f32) -> Vec<f32> {
    if scores.is_empty() {
        return Vec::new();
    }
    let maximum = scores.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let exponentials = scores
        .iter()
        .map(|score| ((score - maximum) * scale).exp())
        .collect::<Vec<_>>();
    let total = exponentials.iter().sum::<f32>();
    if total <= f32::EPSILON || !total.is_finite() {
        return vec![0.0; scores.len()];
    }
    exponentials
        .into_iter()
        .map(|value| value / total)
        .collect()
}

pub fn classification_output_path(image_path: &Path) -> PathBuf {
    let mut file_name = image_path
        .file_name()
        .map(OsString::from)
        .unwrap_or_else(|| OsString::from("image"));
    file_name.push(".json");
    image_path.with_file_name(file_name)
}

pub(crate) fn has_valid_classification_output(image_path: &Path) -> AppResult<bool> {
    valid_existing_output(&classification_output_path(image_path))
}

fn valid_existing_output(path: &Path) -> AppResult<bool> {
    if !path.try_exists()? {
        return Ok(false);
    }
    let bytes = std::fs::read(path)?;
    Ok(serde_json::from_slice::<ImageProcessingOutput>(&bytes)
        .is_ok_and(|output| output.version == IMAGE_PROCESSING_OUTPUT_VERSION))
}

fn write_output(path: &Path, output: &ImageProcessingOutput) -> AppResult<()> {
    let bytes = serde_json::to_vec_pretty(output).map_err(AppError::serialization)?;
    let mut temporary_name = path
        .file_name()
        .map(OsString::from)
        .unwrap_or_else(|| OsString::from("image.json"));
    temporary_name.push(format!(".{}.tmp", uuid::Uuid::new_v4()));
    let temporary = path.with_file_name(temporary_name);
    std::fs::write(&temporary, bytes)?;

    if path.try_exists()? {
        std::fs::remove_file(path)?;
    }
    if let Err(error) = std::fs::rename(&temporary, path) {
        let _ = std::fs::remove_file(&temporary);
        return Err(error.into());
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/services/image_processing_service.rs"]
mod tests;
