use std::collections::HashSet;

use crate::models::image_processing_model::ImageProcessingOutput;
use crate::utils::constants::{CATEGORY_TAG_PREFIX, OBJECT_TAG_PREFIX, SEARCH_TAG_PREFIX};

pub fn search_tags(output: &ImageProcessingOutput) -> Vec<String> {
    let categories = output
        .classification
        .secondary
        .iter()
        .map(|prediction| prediction.label.as_str());
    let objects = output
        .object_detection
        .iter()
        .flat_map(|detection| detection.detections.iter())
        .map(|detection| detection.label.as_str());

    let mut seen = HashSet::new();
    categories
        .clone()
        .map(|category| (CATEGORY_TAG_PREFIX, category))
        .chain(objects.clone().map(|object| (OBJECT_TAG_PREFIX, object)))
        .chain(categories.map(|category| (SEARCH_TAG_PREFIX, category)))
        .chain(objects.map(|object| (SEARCH_TAG_PREFIX, object)))
        .chain(
            output
                .search_keywords
                .iter()
                .map(|keyword| (SEARCH_TAG_PREFIX, keyword.as_str())),
        )
        .filter_map(|(prefix, value)| namespaced_tag(prefix, value))
        .filter(|tag| seen.insert(tag.clone()))
        .collect()
}

fn namespaced_tag(prefix: &str, value: &str) -> Option<String> {
    let mut normalized = String::new();
    let mut separator_pending = false;

    for character in value.trim().chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            if separator_pending && !normalized.is_empty() {
                normalized.push('_');
            }
            normalized.push(character);
            separator_pending = false;
        } else if !normalized.is_empty() {
            separator_pending = true;
        }
    }

    (!normalized.is_empty()).then(|| format!("{prefix}{normalized}"))
}
