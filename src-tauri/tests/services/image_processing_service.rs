use std::collections::BTreeMap;

use super::*;

fn prepared_config(
    level: &str,
    label: &str,
    threshold: f32,
    multilabel: bool,
    normalize_scores: bool,
) -> PreparedClassificationConfig {
    PreparedClassificationConfig {
        label: label.into(),
        parent_label: None,
        level: level.into(),
        threshold,
        multilabel,
        normalize_scores,
        labels: vec![
            PreparedLabel {
                label: "first".into(),
                embedding: vec![1.0],
            },
            PreparedLabel {
                label: "second".into(),
                embedding: vec![1.0],
            },
        ],
    }
}

#[test]
fn normalized_single_label_config_returns_only_the_best_match() {
    let config = prepared_config("primary", "primary", 0.2, false, true);
    let predictions = select_predictions(&config, vec![0.31, 0.20]);
    assert_eq!(predictions.len(), 1);
    assert_eq!(predictions[0].label, "first");
    assert!(predictions[0].score > 0.99);
    assert_eq!(predictions[0].parent_label, None);
}

#[test]
fn single_label_config_returns_the_best_match_below_threshold() {
    let config = prepared_config("secondary", "test_parent", 0.9, false, true);
    let predictions = select_predictions(&config, vec![0.21, 0.20]);
    assert_eq!(predictions.len(), 1);
    assert_eq!(predictions[0].label, "first");
    assert!(predictions[0].score < config.threshold);
}

#[test]
fn removes_florence_location_tokens_from_searchable_ocr_text() {
    assert_eq!(
        strip_florence_location_tokens(
            "NexFile<loc_10><loc_20><loc_300><loc_80> Search<loc_40><loc_100><loc_200><loc_160>"
        ),
        "NexFile Search"
    );
}

#[test]
fn raw_multilabel_config_keeps_every_match_above_threshold() {
    let config = prepared_config("tertiary", "people", 0.27, true, false);
    let predictions = select_predictions(&config, vec![0.31, 0.29]);
    assert_eq!(predictions.len(), 2);
    assert_eq!(predictions[0].parent_label.as_deref(), Some("people"));
}

#[test]
fn raw_multilabel_config_falls_back_to_the_best_match() {
    let config = prepared_config("tertiary", "animals", 0.24, true, false);
    let predictions = select_predictions(&config, vec![0.23, 0.22]);
    assert_eq!(predictions.len(), 1);
    assert_eq!(predictions[0].label, "first");
    assert_eq!(predictions[0].score, 0.23);
}

#[test]
fn extracts_one_to_three_word_candidates_from_a_detailed_caption() {
    let candidates = extract_keyword_candidates(
        "A red vintage car parked beside a brick building on a rainy city street.",
    );

    assert!(candidates.contains(&"red vintage car".to_owned()));
    assert!(candidates.contains(&"vintage car".to_owned()));
    assert!(candidates.contains(&"car parked".to_owned()));
    assert!(candidates.contains(&"parked".to_owned()));
    assert!(candidates.contains(&"brick building".to_owned()));
    assert!(candidates.contains(&"rainy city street".to_owned()));
    assert!(!candidates
        .iter()
        .any(|candidate| candidate.contains("beside")));
}

#[test]
fn extracts_individual_concepts_from_a_punctuated_caption_list() {
    let candidates = extract_keyword_candidates(
        "This is an animated image. In this image we can also see many trees, plants, flowers, grass and sky with clouds.",
    );

    assert_eq!(
        candidates,
        ["trees", "plants", "flowers", "grass", "sky", "clouds"]
    );
}

#[test]
fn parses_florence_object_labels_and_boxes_in_original_image_coordinates() {
    let detections = parse_object_detections(
        "<s><od>Tree<loc_0><loc_100><loc_500><loc_900>Flower<loc_250><loc_300><loc_750><loc_800></od></s>",
        2000,
        1000,
    );

    assert_eq!(
        detections,
        [
            ImageObjectDetection {
                label: "tree".to_owned(),
                bounding_box: ImageBoundingBox {
                    x_min: 1,
                    y_min: 100,
                    x_max: 1001,
                    y_max: 900,
                },
            },
            ImageObjectDetection {
                label: "flower".to_owned(),
                bounding_box: ImageBoundingBox {
                    x_min: 501,
                    y_min: 300,
                    x_max: 1501,
                    y_max: 800,
                },
            },
        ]
    );
}

#[test]
fn keeps_multiple_boxes_for_one_florence_object_label() {
    let detections = parse_object_detections(
        "tree<loc_10><loc_20><loc_30><loc_40><loc_50><loc_60><loc_70><loc_80>",
        1000,
        1000,
    );

    assert_eq!(detections.len(), 2);
    assert!(detections.iter().all(|detection| detection.label == "tree"));
}

#[test]
fn keeps_searchable_activity_words_as_candidates() {
    let candidates = extract_keyword_candidates(
        "People standing, sitting, holding signs, wearing uniforms, walking, looking, with parked cars.",
    );

    for activity in [
        "standing", "sitting", "holding", "wearing", "walking", "looking", "parked",
    ] {
        assert!(
            candidates
                .iter()
                .any(|candidate| candidate.split_whitespace().any(|word| word == activity)),
            "missing searchable activity: {activity}"
        );
    }
}

#[test]
fn removes_viewpoint_pronoun_and_incomplete_descriptor_candidates() {
    let candidates = extract_keyword_candidates(
        "A large green and red bird is looking directly at the camera. Its head has blue eyes.",
    );

    assert!(candidates.contains(&"red bird".to_owned()));
    assert!(candidates.contains(&"blue eyes".to_owned()));
    assert!(candidates.contains(&"eyes".to_owned()));
    assert!(candidates.contains(&"head".to_owned()));
    assert!(!candidates.contains(&"large green".to_owned()));
    for rejected in ["camera", "directly", "its"] {
        assert!(!candidates
            .iter()
            .any(|candidate| candidate.split_whitespace().any(|word| word == rejected)));
    }
}

#[test]
fn keeps_body_and_animal_part_words_as_candidates() {
    for word in [
        "eye", "eyes", "nose", "ear", "ears", "head", "mouth", "hair", "fur", "skin", "body",
        "neck", "leg", "legs", "paw", "paws", "hand", "hands", "foot", "feet", "wing", "wings",
        "feather", "feathers", "tail", "whisker", "whiskers", "beak", "horn", "horns", "mane",
    ] {
        assert_eq!(
            extract_keyword_candidates(word),
            vec![word.to_owned()],
            "missing searchable body or animal part: {word}"
        );
    }
}

#[test]
fn selects_highest_scoring_non_redundant_search_tags() {
    let tags = select_search_tags(vec![
        ("car".to_owned(), 0.34),
        ("red vintage car".to_owned(), 0.36),
        ("brick building".to_owned(), 0.33),
        ("building".to_owned(), 0.31),
        ("city street".to_owned(), 0.30),
        ("carpet".to_owned(), 0.29),
    ]);

    assert_eq!(
        tags,
        ["red vintage car", "brick building", "city street", "carpet"]
    );
}

#[test]
fn loads_the_complete_resource_configuration_hierarchy() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("ai-configs");
    let definitions = load_config_definitions(&directory).expect("configs should load");
    assert_eq!(definitions.len(), 21);
    assert!(definitions.iter().any(|config| {
        config.level == "primary" && config.label == "primary" && config.parent_label.is_none()
    }));
    assert!(definitions.iter().any(|config| {
        config.level == "tertiary"
            && config.label == "people"
            && config.parent_label.as_deref() == Some("visual")
    }));

    let primary = definitions
        .iter()
        .find(|config| config.level == "primary" && config.label == "primary")
        .expect("primary config should exist");
    assert_eq!(
        primary.configuration.labels["ocr"],
        "an image containing clearly readable and meaningful words, sentences, labels, signs, or document text"
    );
    assert_eq!(
        primary.configuration.labels["visual"],
        "a photograph or visual image without clearly readable or meaningful text"
    );
    assert_eq!(primary.configuration.labels.len(), 2);

    assert!(!definitions
        .iter()
        .any(|config| config.level == "secondary" && config.label == "semi_ocr"));

    let visual = definitions
        .iter()
        .find(|config| config.level == "secondary" && config.label == "visual")
        .expect("visual config should exist");
    assert!(!visual.configuration.multilabel);
    assert_eq!(
        visual.configuration.labels["fungi"],
        "An image primarily focused on fungi, such as mushrooms, toadstools, molds, bracket fungi, puffballs, or other fungal growth."
    );

    for tertiary in definitions
        .iter()
        .filter(|config| config.level == "tertiary")
    {
        let expected_threshold = if tertiary.label == "animals" {
            0.26
        } else {
            0.24
        };
        assert_eq!(tertiary.configuration.threshold, expected_threshold);
        assert!(!tertiary.configuration.normalize_scores);
    }
}

#[test]
fn rejects_a_configuration_without_labels() {
    let root = std::env::temp_dir().join(format!(
        "nexfile-empty-classification-config-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&root).expect("test directory should be created");
    std::fs::write(
        root.join("empty.json"),
        r#"{
            "modality":"image",
            "label":"primary",
            "parent_label":null,
            "level":"primary",
            "task":"image-classification",
            "enabled":true,
            "configuration":{
                "labels":{},
                "threshold":0.2,
                "multilabel":false,
                "normalize_scores":true
            }
        }"#,
    )
    .expect("test configuration should be written");

    let error = load_config_definitions(&root).expect_err("empty labels should be rejected");
    assert!(error
        .user_message()
        .contains("classification configuration has no labels"));
    std::fs::remove_dir_all(root).expect("test directory should be removed");
}

#[test]
fn rejects_duplicate_configuration_keys() {
    let root = std::env::temp_dir().join(format!(
        "nexfile-duplicate-classification-config-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&root).expect("test directory should be created");
    let configuration = r#"{
        "modality":"image",
        "label":"primary",
        "parent_label":null,
        "level":"primary",
        "task":"image-classification",
        "enabled":true,
        "configuration":{
            "labels":{"photo":"a photograph"},
            "threshold":0.2,
            "multilabel":false,
            "normalize_scores":true
        }
    }"#;
    std::fs::write(root.join("first.json"), configuration)
        .expect("first configuration should be written");
    std::fs::write(root.join("second.json"), configuration)
        .expect("second configuration should be written");

    let error = load_config_definitions(&root).expect_err("duplicates should be rejected");
    assert!(error
        .user_message()
        .contains("Duplicate image-classification configuration"));
    std::fs::remove_dir_all(root).expect("test directory should be removed");
}

#[test]
fn prepares_resized_jpeg_and_removes_it_on_drop() {
    let root = std::env::temp_dir().join(format!(
        "nexfile-image-preparation-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&root).expect("test directory should be created");
    let source = root.join("transparent.png");
    DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
        2050,
        1025,
        image::Rgba([255, 0, 0, 0]),
    ))
    .save(&source)
    .expect("source fixture should encode");

    let temporary_path;
    {
        let prepared = PreparedModelImage::prepare(&source).expect("model JPEG should be prepared");
        temporary_path = prepared.path().to_path_buf();
        assert!(temporary_path.is_file());
        assert_eq!(
            temporary_path.extension().and_then(|value| value.to_str()),
            Some("jpg")
        );

        let model_image = image::open(&temporary_path).expect("model JPEG should decode");
        assert_eq!((model_image.width(), model_image.height()), (2048, 1024));
        let pixel = model_image.to_rgb8().get_pixel(0, 0).0;
        assert!(pixel.iter().all(|channel| *channel >= 250));

        let metadata = extract_image_metadata(&source, &prepared)
            .expect("source image metadata should be extracted");
        assert_eq!(metadata.media_type.as_deref(), Some("image/png"));
        assert_eq!((metadata.width, metadata.height), (2050, 1025));
        assert_eq!(metadata.location, None);
        assert!(metadata.size_bytes > 0);
        assert_eq!(metadata.perceptual_hash.len(), 16);
        assert!(metadata
            .perceptual_hash
            .chars()
            .all(|character| character.is_ascii_hexdigit()));
        assert!(source.is_file(), "the original image must remain untouched");
    }

    assert!(
        !temporary_path.exists(),
        "the temporary JPEG must be deleted when processing ends"
    );
    assert!(source.is_file(), "the original image must still exist");
    std::fs::remove_dir_all(root).expect("test directory should be removable");
}

#[test]
fn preserves_the_json_sidecar_creation_date_across_schema_versions() {
    let root = std::env::temp_dir().join(format!(
        "nexfile-sidecar-created-at-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&root).expect("test directory should be created");
    let output_path = root.join("image.jpg.json");

    std::fs::write(&output_path, r#"{"createdAtMs":41}"#)
        .expect("current sidecar fixture should be written");
    assert_eq!(existing_sidecar_created_at_ms(&output_path), Some(41));

    std::fs::write(&output_path, r#"{"metadata":{"createdAtMs":42}}"#)
        .expect("version 23 sidecar fixture should be written");
    assert_eq!(existing_sidecar_created_at_ms(&output_path), Some(42));

    std::fs::remove_dir_all(root).expect("test directory should be removable");
}

#[test]
fn appends_json_without_replacing_the_image_extension() {
    assert_eq!(
        classification_output_path(Path::new("C:/images/photo.jpg")),
        PathBuf::from("C:/images/photo.jpg.json")
    );
}

#[test]
fn treats_unversioned_classification_output_as_stale() {
    let root = std::env::temp_dir().join(format!(
        "nexfile-stale-classification-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&root).expect("test directory should be created");
    let output_path = root.join("image.jpg.json");
    std::fs::write(
        &output_path,
        r#"{"classification":{"primary":[],"secondary":[],"tertiary":[]}}"#,
    )
    .expect("legacy output should be written");

    assert!(!valid_existing_output(&output_path).expect("legacy output should be checked"));
    std::fs::remove_dir_all(root).expect("test directory should be removed");
}

#[test]
fn treats_previous_primary_label_version_as_stale() {
    let root = std::env::temp_dir().join(format!(
        "nexfile-old-classification-version-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&root).expect("test directory should be created");
    let output_path = root.join("image.jpg.json");
    std::fs::write(
        &output_path,
        r#"{"version":10,"caption":null,"ocr":{"text":"NexFile","rawTextWithRegions":"NexFile"},"searchKeywords":["nexfile"],"classification":{"primary":[{"label":"contains_readable_text","parentLabel":null,"score":0.9}],"secondary":[],"tertiary":[]}}"#,
    )
    .expect("old output should be written");

    assert!(!valid_existing_output(&output_path).expect("old output should be checked"));
    std::fs::remove_dir_all(root).expect("test directory should be removed");
}

#[test]
fn treats_an_empty_caption_as_stale() {
    let root = std::env::temp_dir().join(format!("nexfile-empty-caption-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).expect("test directory should be created");
    let output_path = root.join("image.jpg.json");
    std::fs::write(
        &output_path,
        format!(
            r#"{{"version":{IMAGE_PROCESSING_OUTPUT_VERSION},"caption":"  ","ocr":null,"searchKeywords":["test"],"classification":{{"primary":[],"secondary":[],"tertiary":[]}}}}"#
        ),
    )
    .expect("empty caption output should be written");

    assert!(!valid_existing_output(&output_path).expect("output should be checked"));
    std::fs::remove_dir_all(root).expect("test directory should be removed");
}

#[test]
fn accepts_a_current_ocr_output_without_a_caption() {
    let root = std::env::temp_dir().join(format!("nexfile-ocr-output-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).expect("test directory should be created");
    let output_path = root.join("image.jpg.json");
    std::fs::write(
        &output_path,
        format!(
            r#"{{"version":{IMAGE_PROCESSING_OUTPUT_VERSION},"caption":null,"ocr":{{"text":"NexFile","rawTextWithRegions":"NexFile<loc_1><loc_2><loc_3><loc_4>"}},"searchKeywords":["nexfile"],"classification":{{"primary":[{{"label":"ocr","parentLabel":null,"score":0.9}}],"secondary":[],"tertiary":[]}}}}"#
        ),
    )
    .expect("OCR output should be written");

    assert!(valid_existing_output(&output_path).expect("OCR output should be checked"));
    std::fs::remove_dir_all(root).expect("test directory should be removed");
}

#[tokio::test]
#[ignore = "loads the bundled CLIP and Florence-2 models"]
async fn bundled_models_write_binary_analysis_and_tags_for_real_images() {
    let root = std::env::temp_dir().join(format!(
        "nexfile-image-classification-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&root).expect("test directory should be created");
    let avip_path = root.join("red-square.avip");
    let rgb_image = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
        224,
        224,
        image::Rgb([255, 0, 0]),
    ));
    rgb_image
        .save_with_format(&avip_path, image::ImageFormat::Avif)
        .expect("test image should encode as AVIF");

    let ico_path = root.join("red-square.ico");
    let rgba_image = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
        64,
        64,
        image::Rgba([255, 0, 0, 255]),
    ));
    rgba_image
        .save_with_format(&ico_path, image::ImageFormat::Ico)
        .expect("test image should encode as ICO");

    let svg_path = root.join("red-square.svg");
    std::fs::write(
        &svg_path,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="224" height="224"><rect width="224" height="224" fill="#ff0000"/></svg>"##,
    )
    .expect("test SVG should be written");

    let heic_fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("flat_red_64.heic");
    let heic_path = root.join("red-square.heic");
    let heif_path = root.join("red-square.heif");
    std::fs::copy(&heic_fixture, &heic_path).expect("HEIC fixture should be copied");
    std::fs::copy(&heic_fixture, &heif_path).expect("HEIF fixture should be copied");

    let resources = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources");
    let service = ImageProcessingService::new(
        resources.join("ai-models").join("clip-vit-base-patch32"),
        resources.join("ai-models").join("florence-2-base-ft"),
        resources.join("ai-configs"),
    );

    for image_path in [avip_path, ico_path, svg_path, heic_path, heif_path] {
        let output_path = service
            .process(ImageProcessingJob {
                path: image_path.clone(),
            })
            .await
            .unwrap_or_else(|error| {
                panic!("image processing should succeed for {image_path:?}: {error:?}")
            });
        let output = serde_json::from_slice::<ImageProcessingOutput>(
            &std::fs::read(&output_path).expect("classification output should be readable"),
        )
        .expect("classification output should be valid JSON");

        assert_eq!(output_path, classification_output_path(&image_path));
        assert_eq!(output.version, IMAGE_PROCESSING_OUTPUT_VERSION);
        assert!(
            !output.search_keywords.is_empty(),
            "search keywords should not be empty for {image_path:?}"
        );
        assert!(
            !output.classification.primary.is_empty(),
            "primary classification should not be empty for {image_path:?}"
        );
        match output.classification.primary[0].label.as_str() {
            "visual" => {
                assert!(output
                    .caption
                    .as_deref()
                    .is_some_and(|text| !text.trim().is_empty()));
                assert!(output.ocr.is_none());
                let object_detection = output
                    .object_detection
                    .as_ref()
                    .expect("visual images should include Florence-2 object detection");
                assert!(object_detection.image_width > 0);
                assert!(object_detection.image_height > 0);
                assert!(!output.classification.secondary.is_empty());
                assert!(output.classification.tertiary.is_empty());
            }
            "ocr" => {
                assert!(output.caption.is_none());
                assert!(output.ocr.is_some());
                assert!(output.object_detection.is_none());
                assert!(output.classification.secondary.is_empty());
                assert!(output.classification.tertiary.is_empty());
            }
            label => panic!("unexpected primary label for {image_path:?}: {label}"),
        }
    }

    std::fs::remove_dir_all(root).expect("test directory should be removed");
}

#[derive(Default)]
struct CalibrationStats {
    top_scores: Vec<f32>,
    runner_up_scores: Vec<f32>,
}

#[test]
#[ignore = "requires NEXFILE_CALIBRATION_DIR and runs the bundled CLIP model"]
fn reports_classification_score_distributions() {
    let image_directory = std::env::var_os("NEXFILE_CALIBRATION_DIR")
        .map(PathBuf::from)
        .expect("NEXFILE_CALIBRATION_DIR must point to a directory of images");
    let resources = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources");
    let mut classifier = PreparedImageClassifier::load(
        &resources.join("ai-models").join("clip-vit-base-patch32"),
        &resources.join("ai-configs"),
    )
    .expect("classifier should load");
    let mut paths = std::fs::read_dir(&image_directory)
        .expect("calibration directory should be readable")
        .map(|entry| entry.expect("directory entry should be readable").path())
        .filter(|path| path.is_file() && has_image_extension(path))
        .collect::<Vec<_>>();
    paths.sort();
    assert!(!paths.is_empty(), "calibration directory has no images");

    let primary_index = classifier
        .configs
        .iter()
        .position(|config| config.level == "primary" && config.label == "primary")
        .expect("primary config should exist");
    let mut statistics = BTreeMap::<usize, CalibrationStats>::new();
    let mut calibrated_count = 0_usize;
    let mut skipped_count = 0_usize;

    for path in &paths {
        let prepared = match PreparedModelImage::prepare(path) {
            Ok(prepared) => prepared,
            Err(error) => {
                skipped_count += 1;
                eprintln!(
                    "skipped unreadable calibration image {}: {error}",
                    path.display()
                );
                continue;
            }
        };
        let embedding = match classifier.model.embed_image_path(prepared.path()) {
            Ok(embedding) => embedding,
            Err(error) => {
                skipped_count += 1;
                eprintln!("skipped calibration image {}: {error}", path.display());
                continue;
            }
        };
        calibrated_count += 1;
        let mut config_index = Some(primary_index);

        while let Some(index) = config_index {
            let config = &classifier.configs[index];
            let ranking = ranked_config_scores(config, &embedding);
            let stats = statistics.entry(index).or_default();
            stats.top_scores.push(ranking[0].1);
            if let Some((_, score)) = ranking.get(1) {
                stats.runner_up_scores.push(*score);
            }

            let winner = ranking[0].0.as_str();
            config_index = match config.level.as_str() {
                "primary" => classifier.configs.iter().position(|candidate| {
                    candidate.level == "secondary"
                        && candidate.label == winner
                        && candidate.parent_label.as_deref() == Some("primary")
                }),
                "secondary" => classifier.configs.iter().position(|candidate| {
                    candidate.level == "tertiary"
                        && candidate.label == winner
                        && candidate.parent_label.as_deref() == Some(config.label.as_str())
                }),
                _ => None,
            };
        }
    }

    assert!(
        calibrated_count > 0,
        "no calibration image could be processed"
    );
    eprintln!("calibrated {calibrated_count} image(s); skipped {skipped_count}");
    for (index, stats) in statistics {
        let config = &classifier.configs[index];
        let passing = stats
            .top_scores
            .iter()
            .filter(|score| **score >= config.threshold)
            .count();
        eprintln!(
            "{}/{}/{} mode={} labels={} threshold={:.3} samples={} passing={} top[min={:.3} p05={:.3} p10={:.3} p25={:.3} p50={:.3} p75={:.3} max={:.3}] runner[p50={:.3} p75={:.3} max={:.3}]",
            config.level,
            config.parent_label.as_deref().unwrap_or("-"),
            config.label,
            if config.normalize_scores { "softmax" } else { "raw" },
            config.labels.len(),
            config.threshold,
            stats.top_scores.len(),
            passing,
            percentile(&stats.top_scores, 0.0),
            percentile(&stats.top_scores, 0.05),
            percentile(&stats.top_scores, 0.10),
            percentile(&stats.top_scores, 0.25),
            percentile(&stats.top_scores, 0.50),
            percentile(&stats.top_scores, 0.75),
            percentile(&stats.top_scores, 1.0),
            percentile(&stats.runner_up_scores, 0.50),
            percentile(&stats.runner_up_scores, 0.75),
            percentile(&stats.runner_up_scores, 1.0),
        );
    }
}

fn ranked_config_scores(
    config: &PreparedClassificationConfig,
    image_embedding: &[f32],
) -> Vec<(String, f32)> {
    let raw_scores = config
        .labels
        .iter()
        .map(|label| {
            ClipModel::cosine_similarity(image_embedding, &label.embedding)
                .expect("calibration embedding dimensions should match")
        })
        .collect::<Vec<_>>();
    let scores = if config.normalize_scores {
        softmax(&raw_scores, CLIP_LOGIT_SCALE)
    } else {
        raw_scores
    };
    let mut ranking = config
        .labels
        .iter()
        .zip(scores)
        .map(|(label, score)| (label.label.clone(), score))
        .collect::<Vec<_>>();
    ranking.sort_by(|left, right| right.1.total_cmp(&left.1));
    ranking
}

fn percentile(values: &[f32], percentile: f32) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f32::total_cmp);
    let index = ((sorted.len() - 1) as f32 * percentile).round() as usize;
    sorted[index]
}

fn has_image_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "avif"
                    | "avip"
                    | "bmp"
                    | "gif"
                    | "heic"
                    | "heif"
                    | "ico"
                    | "jpeg"
                    | "jpg"
                    | "png"
                    | "svg"
                    | "tif"
                    | "tiff"
                    | "webp"
            )
        })
}
