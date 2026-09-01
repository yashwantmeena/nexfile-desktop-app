use std::collections::HashSet;
use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use image::{codecs::jpeg::JpegEncoder, imageops::FilterType, DynamicImage};

use crate::ai_models::clip::{ClipModel, ClipModelPaths, Embedding};
use crate::ai_models::florence2::{Florence2Model, Florence2ModelPaths, Florence2Task};
use crate::error::{
    AppError, AppResult, CaptionerLockPoisoned, ClassifierLockPoisoned, ImagePreparationError,
};
use crate::models::image_processing_model::{
    ClassificationConfigDefinition, ClassificationPrediction, ImageBoundingBox,
    ImageClassificationOutput, ImageLocation, ImageMetadata, ImageObjectDetection,
    ImageObjectDetectionOutput, ImageOcrOutput, ImageProcessingJob, ImageProcessingOutput,
};
use crate::utils::constants::{
    CLIP_LOGIT_SCALE, IMAGE_PROCESSING_OUTPUT_VERSION, MAX_MODEL_IMAGE_DIMENSION,
    MODEL_IMAGE_TEMP_DIRECTORY, MODEL_JPEG_QUALITY, OCR_LABEL, VISUAL_LABEL,
};
use crate::utils::image_decoder::decode_image;
use crate::utils::image_hash::{calculate_phash, format_phash};
use crate::utils::operation_logger::OperationLogger;
use crate::utils::search_tags::{extract_keyword_candidates, select_search_tags};

#[derive(Clone)]
pub struct ImageProcessingService {
    clip_model_directory: PathBuf,
    florence2_model_directory: PathBuf,
    configs_directory: PathBuf,
    classifier: Arc<Mutex<Option<PreparedImageClassifier>>>,
    captioner: Arc<Mutex<Option<Florence2Model>>>,
}

impl ImageProcessingService {
    pub fn new(
        clip_model_directory: PathBuf,
        florence2_model_directory: PathBuf,
        configs_directory: PathBuf,
    ) -> Self {
        Self {
            clip_model_directory,
            florence2_model_directory,
            configs_directory,
            classifier: Arc::new(Mutex::new(None)),
            captioner: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn process(&self, job: ImageProcessingJob) -> AppResult<PathBuf> {
        let service = self.clone();
        tauri::async_runtime::spawn_blocking(move || service.process_blocking(job))
            .await
            .map_err(AppError::internal)?
    }

    fn process_blocking(&self, job: ImageProcessingJob) -> AppResult<PathBuf> {
        let logger = OperationLogger::start("image-processing", job.path.display());
        let result = self.process_blocking_logged(&job, &logger);
        if let Err(error) = &result {
            logger.failed(error);
        }
        result
    }

    fn process_blocking_logged(
        &self,
        job: &ImageProcessingJob,
        logger: &OperationLogger,
    ) -> AppResult<PathBuf> {
        if !job.path.is_file() {
            return Err(AppError::validation(
                "The queued image-processing path is no longer a file.",
            ));
        }

        let output_path = classification_output_path(&job.path);
        if valid_existing_output(&output_path)? {
            logger.cached(output_path.display());
            return Ok(output_path);
        }

        let preparation_started = logger.stage("prepare normalized model image");
        let prepared_image = PreparedModelImage::prepare(&job.path).map_err(AppError::internal)?;
        logger.stage_complete(
            "model image preparation",
            preparation_started,
            format_args!("temporary={}", prepared_image.path().display()),
        );

        let classification_started = logger.stage("run CLIP classification");
        let (classification, image_embedding) = {
            let mut classifier = self
                .classifier
                .lock()
                .map_err(|_| AppError::internal(ClassifierLockPoisoned))?;
            if classifier.is_none() {
                *classifier = Some(PreparedImageClassifier::load(
                    &self.clip_model_directory,
                    &self.configs_directory,
                )?);
            }
            classifier
                .as_mut()
                .expect("classifier is initialized before use")
                .classify(prepared_image.path())?
        };
        logger.stage_complete(
            "CLIP classification",
            classification_started,
            classification_summary(&classification),
        );

        let primary_label = classification
            .primary
            .first()
            .map(|prediction| prediction.label.as_str())
            .ok_or_else(|| {
                AppError::validation("Primary image classification returned no label.")
            })?;
        let use_ocr = match primary_label {
            OCR_LABEL => true,
            VISUAL_LABEL => false,
            label => {
                return Err(AppError::validation(format!(
                    "Unsupported primary image-classification label: {label}"
                )))
            }
        };
        let florence_task = if use_ocr {
            Florence2Task::OcrWithRegion
        } else {
            Florence2Task::DetailedCaption
        };
        let florence_stage_name = if use_ocr {
            "generate Florence-2 OCR with regions"
        } else {
            "generate Florence-2 detailed caption"
        };
        let florence_started = logger.stage(florence_stage_name);
        let (florence_text, object_detection) = {
            let mut captioner = self
                .captioner
                .lock()
                .map_err(|_| AppError::internal(CaptionerLockPoisoned))?;
            if captioner.is_none() {
                *captioner = Some(
                    Florence2Model::load(Florence2ModelPaths::from_dir_with_suffix(
                        &self.florence2_model_directory,
                        Some("_int8"),
                    ))
                    .map_err(AppError::internal)?,
                );
            }
            let captioner = captioner
                .as_mut()
                .expect("captioner is initialized before use");
            let florence_output = captioner
                .generate_path(prepared_image.path(), florence_task)
                .map_err(AppError::internal)?;
            logger.stage_complete(
                if use_ocr {
                    "Florence-2 OCR"
                } else {
                    "Florence-2 caption"
                },
                florence_started,
                format_args!(
                    "characters={} words={}",
                    florence_output.text.chars().count(),
                    florence_output.text.split_whitespace().count()
                ),
            );

            let object_detection = if use_ocr {
                None
            } else {
                let detection_started = logger.stage("generate Florence-2 object detections");
                let detection_output = captioner
                    .generate_path(prepared_image.path(), Florence2Task::ObjectDetection)
                    .map_err(AppError::internal)?;
                let detections = parse_object_detections(
                    &detection_output.text,
                    prepared_image.original_width(),
                    prepared_image.original_height(),
                );
                logger.stage_complete(
                    "Florence-2 object detection",
                    detection_started,
                    format_args!("detections={}", detections.len()),
                );
                Some(ImageObjectDetectionOutput {
                    raw_text: detection_output.text,
                    image_width: prepared_image.original_width(),
                    image_height: prepared_image.original_height(),
                    detections,
                })
            };

            (florence_output.text, object_detection)
        };
        let (caption, ocr, mut tag_source) = if use_ocr {
            let text = strip_florence_location_tokens(&florence_text);
            (
                None,
                Some(ImageOcrOutput {
                    text: text.clone(),
                    raw_text_with_regions: florence_text,
                }),
                text,
            )
        } else {
            (Some(florence_text.clone()), None, florence_text)
        };
        if let Some(detection) = &object_detection {
            for detected in &detection.detections {
                tag_source.push_str(", ");
                tag_source.push_str(&detected.label);
            }
        }

        let tags_started = logger.stage("extract and rank search tags with CLIP");
        let (tags, candidate_count) = {
            let mut classifier = self
                .classifier
                .lock()
                .map_err(|_| AppError::internal(ClassifierLockPoisoned))?;
            classifier
                .as_mut()
                .expect("classifier is initialized before use")
                .extract_tags(&tag_source, &image_embedding)?
        };
        logger.stage_complete(
            "search-tag extraction",
            tags_started,
            format_args!(
                "candidates={candidate_count} selected={} tags=[{}]",
                tags.len(),
                tags.join(", ")
            ),
        );

        let updated_at_ms = current_time_ms();
        let created_at_ms = existing_sidecar_created_at_ms(&output_path).unwrap_or(updated_at_ms);
        let output = ImageProcessingOutput {
            version: IMAGE_PROCESSING_OUTPUT_VERSION,
            created_at_ms,
            updated_at_ms,
            metadata: extract_image_metadata(&job.path, &prepared_image)?,
            caption,
            ocr,
            object_detection,
            search_keywords: tags,
            classification,
        };
        let write_started = logger.stage("write JSON sidecar");
        write_output(&output_path, &output)?;
        logger.stage_complete("JSON sidecar write", write_started, output_path.display());
        logger.complete(output_path.display());
        Ok(output_path)
    }
}

fn extract_image_metadata(path: &Path, prepared: &PreparedModelImage) -> AppResult<ImageMetadata> {
    let filesystem = std::fs::metadata(path)?;
    let format = image::ImageFormat::from_path(path).ok();
    let exif_metadata = OpenOptions::new()
        .read(true)
        .open(path)
        .ok()
        .and_then(|file| {
            exif::Reader::new()
                .read_from_container(&mut BufReader::new(file))
                .ok()
        });
    let location = exif_metadata.as_ref().and_then(exif_location);

    Ok(ImageMetadata {
        media_type: format.map(|format| format.to_mime_type().to_owned()),
        size_bytes: filesystem.len(),
        width: prepared.original_width(),
        height: prepared.original_height(),
        location,
        perceptual_hash: format_phash(prepared.perceptual_hash),
    })
}

fn existing_sidecar_created_at_ms(path: &Path) -> Option<u64> {
    if let Ok(bytes) = std::fs::read(path) {
        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) {
            if let Some(created_at_ms) =
                value.get("createdAtMs").and_then(serde_json::Value::as_u64)
            {
                return Some(created_at_ms);
            }
            // Version 23 stored this sidecar timestamp inside metadata.
            if let Some(created_at_ms) = value
                .get("metadata")
                .and_then(|metadata| metadata.get("createdAtMs"))
                .and_then(serde_json::Value::as_u64)
            {
                return Some(created_at_ms);
            }
        }
    }

    std::fs::metadata(path)
        .ok()?
        .created()
        .ok()
        .and_then(system_time_ms)
}

fn current_time_ms() -> u64 {
    system_time_ms(std::time::SystemTime::now()).unwrap_or_default()
}

fn exif_location(metadata: &exif::Exif) -> Option<ImageLocation> {
    let latitude = gps_coordinate(metadata, exif::Tag::GPSLatitude)?;
    let longitude = gps_coordinate(metadata, exif::Tag::GPSLongitude)?;
    let latitude = apply_gps_direction(
        latitude,
        gps_direction(metadata, exif::Tag::GPSLatitudeRef)?,
    )?;
    let longitude = apply_gps_direction(
        longitude,
        gps_direction(metadata, exif::Tag::GPSLongitudeRef)?,
    )?;

    (latitude.abs() <= 90.0 && longitude.abs() <= 180.0).then_some(ImageLocation {
        latitude,
        longitude,
    })
}

fn gps_coordinate(metadata: &exif::Exif, tag: exif::Tag) -> Option<f64> {
    let field = metadata.fields().find(|field| field.tag == tag)?;
    let exif::Value::Rational(parts) = &field.value else {
        return None;
    };
    let [degrees, minutes, seconds, ..] = parts.as_slice() else {
        return None;
    };
    let value = |part: &exif::Rational| {
        (part.denom != 0).then(|| f64::from(part.num) / f64::from(part.denom))
    };
    Some(value(degrees)? + value(minutes)? / 60.0 + value(seconds)? / 3600.0)
}

fn gps_direction(metadata: &exif::Exif, tag: exif::Tag) -> Option<u8> {
    let field = metadata.fields().find(|field| field.tag == tag)?;
    let exif::Value::Ascii(values) = &field.value else {
        return None;
    };
    values.first()?.first().map(u8::to_ascii_uppercase)
}

fn apply_gps_direction(coordinate: f64, direction: u8) -> Option<f64> {
    match direction {
        b'N' | b'E' => Some(coordinate),
        b'S' | b'W' => Some(-coordinate),
        _ => None,
    }
}

fn system_time_ms(time: std::time::SystemTime) -> Option<u64> {
    time.duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
}

/// A normalized JPEG that is deleted when the processing scope ends.
struct PreparedModelImage {
    path: PathBuf,
    original_width: u32,
    original_height: u32,
    perceptual_hash: u64,
}

impl PreparedModelImage {
    fn prepare(source: &Path) -> Result<Self, ImagePreparationError> {
        let decoded = decode_image(source)?;
        if decoded.width() == 0 || decoded.height() == 0 {
            return Err(ImagePreparationError::Encode(image::ImageError::Limits(
                image::error::LimitError::from_kind(image::error::LimitErrorKind::DimensionError),
            )));
        }
        let original_width = decoded.width();
        let original_height = decoded.height();
        let perceptual_hash = calculate_phash(&decoded);

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
        let prepared = Self {
            path,
            original_width,
            original_height,
            perceptual_hash,
        };
        if let Err(error) = write_jpeg(prepared.path(), &rgb) {
            drop(prepared);
            return Err(error);
        }
        Ok(prepared)
    }

    fn path(&self) -> &Path {
        &self.path
    }

    const fn original_width(&self) -> u32 {
        self.original_width
    }

    const fn original_height(&self) -> u32 {
        self.original_height
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
        let mut configs = Vec::new();

        for definition in definitions {
            if definition.level == "tertiary" {
                continue;
            }
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

    fn classify(&mut self, image_path: &Path) -> AppResult<(ImageClassificationOutput, Embedding)> {
        let image_embedding = self
            .model
            .embed_image_path(image_path)
            .map_err(AppError::internal)?;
        let primary_config = find_config(&self.configs, "primary", "primary", None)
            .expect("primary configuration is validated during loading");
        let primary = classify_with_config(primary_config, &image_embedding)?;
        let mut secondary = Vec::new();
        for primary_prediction in &primary {
            let Some(secondary_config) = find_config(
                &self.configs,
                "secondary",
                &primary_prediction.label,
                Some("primary"),
            ) else {
                continue;
            };
            secondary.extend(classify_with_config(secondary_config, &image_embedding)?);
        }
        Ok((
            ImageClassificationOutput {
                primary,
                secondary,
                ..Default::default()
            },
            image_embedding,
        ))
    }

    fn extract_tags(
        &mut self,
        caption: &str,
        image_embedding: &[f32],
    ) -> AppResult<(Vec<String>, usize)> {
        let candidates = extract_keyword_candidates(caption);
        let candidate_count = candidates.len();
        let scored = candidates
            .into_iter()
            .map(|candidate| {
                let text_embedding = self
                    .model
                    .embed_text(&candidate)
                    .map_err(AppError::internal)?;
                let similarity = ClipModel::cosine_similarity(image_embedding, &text_embedding)
                    .map_err(AppError::internal)?;
                Ok((candidate, similarity))
            })
            .collect::<AppResult<Vec<_>>>()?;

        Ok((select_search_tags(scored), candidate_count))
    }
}

fn classification_summary(classification: &ImageClassificationOutput) -> String {
    format!(
        "primary={} secondary={}",
        prediction_labels(&classification.primary),
        prediction_labels(&classification.secondary)
    )
}

fn strip_florence_location_tokens(text: &str) -> String {
    let mut remaining = text;
    let mut cleaned = String::with_capacity(text.len());
    while let Some(start) = remaining.find("<loc_") {
        cleaned.push_str(&remaining[..start]);
        let token = &remaining[start..];
        let Some(end) = token.find('>') else {
            cleaned.push_str(token);
            remaining = "";
            break;
        };
        cleaned.push(' ');
        remaining = &token[end + 1..];
    }
    cleaned.push_str(remaining);
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn parse_object_detections(
    text: &str,
    image_width: u32,
    image_height: u32,
) -> Vec<ImageObjectDetection> {
    if image_width == 0 || image_height == 0 {
        return Vec::new();
    }

    let mut detections = Vec::new();
    let mut current_label = String::new();
    let mut cursor = 0;

    while let Some(offset) = text[cursor..].find("<loc_") {
        let token_start = cursor + offset;
        let label = clean_detection_label(&text[cursor..token_start]);
        if !label.is_empty() {
            current_label = label;
        }

        let mut location_bins = Vec::new();
        let mut position = token_start;
        while let Some((bin, next_position)) = parse_location_token(text, position) {
            location_bins.push(bin);
            position = next_position;
        }

        if position == token_start {
            cursor = token_start + "<loc_".len();
            continue;
        }

        if !current_label.is_empty() {
            for coordinates in location_bins.chunks_exact(4) {
                if coordinates.iter().any(|coordinate| *coordinate >= 1000) {
                    continue;
                }
                let first_x = dequantize_location(coordinates[0], image_width);
                let first_y = dequantize_location(coordinates[1], image_height);
                let second_x = dequantize_location(coordinates[2], image_width);
                let second_y = dequantize_location(coordinates[3], image_height);
                let detection = ImageObjectDetection {
                    label: current_label.clone(),
                    bounding_box: ImageBoundingBox {
                        x_min: first_x.min(second_x),
                        y_min: first_y.min(second_y),
                        x_max: first_x.max(second_x),
                        y_max: first_y.max(second_y),
                    },
                };
                if !detections.contains(&detection) {
                    detections.push(detection);
                }
            }
        }

        cursor = position;
    }

    detections
}

fn parse_location_token(text: &str, start: usize) -> Option<(u32, usize)> {
    let remainder = text.get(start..)?.strip_prefix("<loc_")?;
    let closing = remainder.find('>')?;
    let bin = remainder[..closing].parse::<u32>().ok()?;
    Some((bin, start + "<loc_".len() + closing + 1))
}

fn clean_detection_label(text: &str) -> String {
    let mut cleaned = String::with_capacity(text.len());
    let mut remaining = text;
    while let Some(start) = remaining.find('<') {
        cleaned.push_str(&remaining[..start]);
        let Some(end) = remaining[start..].find('>') else {
            cleaned.push_str(&remaining[start..]);
            remaining = "";
            break;
        };
        remaining = &remaining[start + end + 1..];
    }
    cleaned.push_str(remaining);
    cleaned
        .trim_matches(|character: char| {
            character.is_whitespace()
                || matches!(character, ',' | '.' | ';' | ':' | '!' | '?' | '|' | '-')
        })
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn dequantize_location(bin: u32, dimension: u32) -> u32 {
    ((((f64::from(bin) + 0.5) * f64::from(dimension)) / 1000.0).floor() as u32)
        .min(dimension.saturating_sub(1))
}

fn prediction_labels(predictions: &[ClassificationPrediction]) -> String {
    if predictions.is_empty() {
        return "-".to_owned();
    }
    predictions
        .iter()
        .map(|prediction| prediction.label.as_str())
        .collect::<Vec<_>>()
        .join(",")
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

fn valid_existing_output(path: &Path) -> AppResult<bool> {
    if !path.try_exists()? {
        return Ok(false);
    }
    let bytes = std::fs::read(path)?;
    Ok(
        serde_json::from_slice::<ImageProcessingOutput>(&bytes).is_ok_and(|output| {
            let has_caption = output
                .caption
                .as_deref()
                .is_some_and(|caption| !caption.trim().is_empty());
            output.version == IMAGE_PROCESSING_OUTPUT_VERSION
                && ((has_caption && output.object_detection.is_some()) || output.ocr.is_some())
        }),
    )
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
