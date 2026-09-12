use super::*;

#[test]
#[ignore = "requires bundled models; compares cached and uncached inference"]
fn cached_decoder_matches_uncached_generation() {
    let directory =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/ai-models/florence-2-base-ft");
    let mut model = Florence2Model::load(Florence2ModelPaths::from_dir_with_suffix(
        directory,
        Some("_int8"),
    ))
    .unwrap();
    assert!(
        model.decoder_with_past.is_some(),
        "cached model must be bundled"
    );
    let image = DynamicImage::ImageRgb8(image::RgbImage::from_fn(224, 224, |x, y| {
        image::Rgb([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8])
    }));
    let prepared = model.prepare_image(&image).unwrap();
    for task in [
        Florence2Task::DetailedCaption,
        Florence2Task::ObjectDetection,
        Florence2Task::OcrWithRegion,
    ] {
        let started = std::time::Instant::now();
        let cached = model.generate_prepared(&prepared, task.clone()).unwrap();
        let cached_time = started.elapsed();
        let cached_session = model.decoder_with_past.take();
        let started = std::time::Instant::now();
        let uncached = model.generate_prepared(&prepared, task.clone()).unwrap();
        let uncached_time = started.elapsed();
        model.decoder_with_past = cached_session;
        eprintln!(
            "{task:?}: cached={cached_time:?} uncached={uncached_time:?} tokens={}",
            cached.token_ids.len()
        );
        assert_eq!(cached, uncached);
    }
    model.config.max_new_tokens = 64;
    model.config.eos_token_id = u32::MAX;
    let started = std::time::Instant::now();
    let cached = model
        .generate_prepared(&prepared, Florence2Task::DetailedCaption)
        .unwrap();
    let cached_time = started.elapsed();
    let cached_session = model.decoder_with_past.take();
    let started = std::time::Instant::now();
    let uncached = model
        .generate_prepared(&prepared, Florence2Task::DetailedCaption)
        .unwrap();
    model.decoder_with_past = cached_session;
    eprintln!(
        "64-token workload: cached={cached_time:?} uncached={:?}",
        started.elapsed()
    );
    assert_eq!(cached, uncached);
}

#[test]
fn selects_only_the_last_token_logits() {
    let logits = Array3::from_shape_vec((1, 2, 3), vec![99.0, 0.0, 0.0, 0.0, 1.0, 2.0]).unwrap();
    assert_eq!(argmax_last_token(logits.view().into_dyn()).unwrap(), 2);
    assert!(argmax_last_token(Array3::<f32>::zeros((1, 0, 3)).view().into_dyn()).is_err());
}

#[test]
#[ignore = "requires bundled Florence-2 models; runs real inference"]
fn shared_image_features_preserve_generated_tokens() {
    let directory =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/ai-models/florence-2-base-ft");
    let mut model = Florence2Model::load_with_config(
        Florence2ModelPaths::from_dir_with_suffix(directory, Some("_int8")),
        Florence2Config {
            max_new_tokens: 8,
            ..Default::default()
        },
    )
    .unwrap();
    let image = DynamicImage::new_rgb8(80, 60);
    let tasks = [
        Florence2Task::DetailedCaption,
        Florence2Task::ObjectDetection,
    ];
    let started = std::time::Instant::now();
    let separate = tasks
        .iter()
        .map(|task| model.generate(&image, task.clone()).unwrap())
        .collect::<Vec<_>>();
    let separate_time = started.elapsed();
    let started = std::time::Instant::now();
    let prepared = model.prepare_image(&image).unwrap();
    for (task, expected) in tasks.into_iter().zip(separate) {
        let actual = model.generate_prepared(&prepared, task).unwrap();
        assert_eq!(actual, expected);
    }
    eprintln!(
        "separate encodings: {separate_time:?}; shared encoding: {:?}",
        started.elapsed()
    );
}
