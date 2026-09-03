use crate::mappers::search_mapper::*;
use crate::models::image_processing_model::{
    ClassificationPrediction, ImageBoundingBox, ImageClassificationOutput, ImageObjectDetection,
    ImageObjectDetectionOutput, ImageProcessingOutput,
};

fn prediction(label: &str) -> ClassificationPrediction {
    ClassificationPrediction {
        label: label.to_owned(),
        parent_label: None,
        score: 0.9,
    }
}

#[test]
fn namespaces_categories_objects_and_search_keywords() {
    let output = ImageProcessingOutput {
        version: 18,
        original_name: "photo.jpg".to_owned(),
        created_at_ms: 0,
        updated_at_ms: 0,
        metadata: Default::default(),
        caption: None,
        ocr: None,
        object_detection: Some(ImageObjectDetectionOutput {
            raw_text: String::new(),
            image_width: 100,
            image_height: 100,
            detections: vec![
                ImageObjectDetection {
                    label: "Red Fox".to_owned(),
                    bounding_box: ImageBoundingBox {
                        x_min: 0,
                        y_min: 0,
                        x_max: 10,
                        y_max: 10,
                    },
                },
                ImageObjectDetection {
                    label: "red-fox".to_owned(),
                    bounding_box: ImageBoundingBox {
                        x_min: 20,
                        y_min: 20,
                        x_max: 30,
                        y_max: 30,
                    },
                },
            ],
        }),
        search_keywords: vec!["Forest Trail".to_owned(), "red fox".to_owned()],
        classification: ImageClassificationOutput {
            primary: vec![prediction("visual")],
            secondary: vec![prediction("Nature & Outdoors")],
            tertiary: vec![prediction("wildlife")],
        },
    };

    assert_eq!(
        search_tags(&output),
        vec![
            "ct::nature_outdoors",
            "ob::red_fox",
            "tg::nature_outdoors",
            "tg::red_fox",
            "tg::forest_trail",
        ]
    );
}
