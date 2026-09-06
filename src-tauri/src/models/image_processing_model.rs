use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::ai_models::clip::Embedding;

#[derive(Debug, Deserialize)]
pub(crate) struct ClassificationConfigDefinition {
    pub(crate) modality: String,
    pub(crate) label: String,
    pub(crate) parent_label: Option<String>,
    pub(crate) level: String,
    pub(crate) task: String,
    pub(crate) enabled: bool,
    pub(crate) configuration: ClassificationSettings,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ClassificationSettings {
    pub(crate) labels: BTreeMap<String, String>,
    pub(crate) threshold: f32,
    pub(crate) multilabel: bool,
    pub(crate) normalize_scores: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageProcessingJob {
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassificationPrediction {
    pub label: String,
    pub parent_label: Option<String>,
    pub score: f32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageClassificationOutput {
    pub primary: Vec<ClassificationPrediction>,
    pub secondary: Vec<ClassificationPrediction>,
    pub tertiary: Vec<ClassificationPrediction>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageProcessingOutput {
    pub version: u32,
    #[serde(default, alias = "originalName")]
    pub name: String,
    #[serde(default)]
    pub created_at_ms: u64,
    #[serde(default)]
    pub updated_at_ms: u64,
    #[serde(default)]
    pub metadata: ImageMetadata,
    pub caption: Option<String>,
    pub ocr: Option<ImageOcrOutput>,
    pub object_detection: Option<ImageObjectDetectionOutput>,
    pub search_keywords: Vec<String>,
    pub classification: ImageClassificationOutput,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageMetadata {
    pub media_type: Option<String>,
    pub size_bytes: u64,
    pub width: u32,
    pub height: u32,
    pub location: Option<ImageLocation>,
    /// A 64-bit DCT perceptual hash encoded as 16 lowercase hexadecimal digits.
    pub perceptual_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageLocation {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageOcrOutput {
    pub text: String,
    pub raw_text_with_regions: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageObjectDetectionOutput {
    pub raw_text: String,
    pub image_width: u32,
    pub image_height: u32,
    pub detections: Vec<ImageObjectDetection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageObjectDetection {
    pub label: String,
    pub bounding_box: ImageBoundingBox,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageBoundingBox {
    pub x_min: u32,
    pub y_min: u32,
    pub x_max: u32,
    pub y_max: u32,
}

pub(crate) struct PreparedClassificationConfig {
    pub(crate) label: String,
    pub(crate) parent_label: Option<String>,
    pub(crate) level: String,
    pub(crate) threshold: f32,
    pub(crate) multilabel: bool,
    pub(crate) normalize_scores: bool,
    pub(crate) labels: Vec<PreparedLabel>,
}

pub(crate) struct PreparedLabel {
    pub(crate) label: String,
    pub(crate) embedding: Embedding,
}
