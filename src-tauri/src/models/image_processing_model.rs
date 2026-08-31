use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

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
    pub caption: String,
    pub classification: ImageClassificationOutput,
}
