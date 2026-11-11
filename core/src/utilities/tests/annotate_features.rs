use crate::{
    AnnotateFeaturesOptions, ErrorKind, FeatureAnnotation, ParseIonOptions, QuantionError,
    SampleFile, annotate_features, parse_ion, utilities::find_features::Feature,
};

const FOLDER: &str = "tests/fixtures/annotate";
const SAMPLES: [&str; 3] = ["sample_peak_and_ms2", "sample_peak_only", "sample_no_peak"];

fn get_files() -> Vec<SampleFile> {
    SAMPLES
        .iter()
        .map(|name| {
            parse_ion(
                format!("{FOLDER}/{name}.ion").as_str(),
                ParseIonOptions::default(),
            )
            .expect("sample")
        })
        .collect()
}

fn get_options(model: &str) -> AnnotateFeaturesOptions {
    AnnotateFeaturesOptions {
        library: format!("{FOLDER}/library.ion"),
        model: format!("{FOLDER}/{model}.onnx"),
        ..Default::default()
    }
}

fn annotate(
    features: &[Feature],
    options: AnnotateFeaturesOptions,
) -> Result<Vec<FeatureAnnotation>, QuantionError> {
    annotate_features(&mut get_files(), features, options)
}

fn get_features() -> Vec<Feature> {
    vec![Feature::new(195.0877, 2.0)]
}

#[test]
fn the_best_compound_names_the_feature() {
    let annotations = annotate(&get_features(), get_options("model")).expect("annotations");
    let first = &annotations[0];
    assert_eq!(first.label.as_deref(), Some("Caffeine"));
    assert_eq!(first.id.as_deref(), Some("ENTRY_1"));
    assert_eq!(first.database.as_deref(), Some("SOURCE_A"));
    assert!((first.frequency - 1.0 / 3.0).abs() < 1e-12);
    assert!(first.score.expect("score") > 0.99);
    assert!(first.cosine.expect("cosine") > 0.99);
}

#[test]
fn each_file_keeps_its_best_matches_in_order() {
    let annotations = annotate(&get_features(), get_options("model")).expect("annotations");
    let files = &annotations[0].files;
    assert_eq!(
        files.iter().map(|file| file.file).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert!(files[0].scan_index.is_some());
    assert_eq!(files[1].scan_index, None);
    assert_eq!(files[2].scan_index, None);
    let labels: Vec<&str> = files[0]
        .matches
        .iter()
        .map(|found| found.label.as_str())
        .collect();
    assert_eq!(labels, vec!["Caffeine", "Compound Y"]);
    assert!(files[0].matches[0].score >= files[0].matches[1].score);
    let best = &files[0].matches[0];
    assert_eq!(best.mz, vec![110.0713, 123.0427, 138.0662]);
    assert_eq!(
        best.metadata.get("family").map(String::as_str),
        Some("FAMILY_C")
    );
    assert_eq!(
        best.metadata.get("note").map(String::as_str),
        Some("synthetic 1")
    );
    assert!(!best.metadata.contains_key("entry"));
    assert!(!best.metadata.contains_key("source"));
    assert!(!best.metadata.contains_key("title"));
}

#[test]
fn matches_limits_the_list_of_each_file() {
    let options = AnnotateFeaturesOptions {
        matches: 1,
        ..get_options("model")
    };
    let annotations = annotate(&get_features(), options).expect("annotations");
    assert_eq!(annotations[0].files[0].matches.len(), 1);
    assert_eq!(annotations[0].files[0].matches[0].label, "Caffeine");
}

#[test]
fn a_feature_without_candidates_has_no_label() {
    let features = [Feature::new(350.05, 2.0), Feature::new(500.0, 2.0)];
    let annotations = annotate(&features, get_options("model")).expect("annotations");
    for annotation in &annotations {
        assert_eq!(annotation.label, None);
        assert_eq!(annotation.id, None);
        assert_eq!(annotation.score, None);
        assert_eq!(annotation.frequency, 0.0);
        assert!(annotation.files.iter().all(|file| file.matches.is_empty()));
    }
    assert!(annotations[0].files[0].scan_index.is_some());
    assert_eq!(annotations[1].files[0].scan_index, None);
}

#[test]
fn no_features_give_no_annotations() {
    assert!(
        annotate(&[], get_options("model"))
            .expect("annotations")
            .is_empty()
    );
}

#[test]
fn no_files_are_refused() {
    let error =
        annotate_features(&mut [], &get_features(), get_options("model")).expect_err("no files");
    assert_eq!(error.argument.as_deref(), Some("files"));
}

#[test]
fn invalid_options_are_refused() {
    let cases = [
        (
            AnnotateFeaturesOptions {
                matches: 0,
                ..get_options("model")
            },
            "matches",
        ),
        (
            AnnotateFeaturesOptions {
                rt_range: 0.0,
                ..get_options("model")
            },
            "rt_range",
        ),
        (
            AnnotateFeaturesOptions {
                min_relative_intensity: 2.0,
                ..get_options("model")
            },
            "min_relative_intensity",
        ),
        (
            AnnotateFeaturesOptions {
                library: String::new(),
                ..get_options("model")
            },
            "library",
        ),
    ];
    for (options, argument) in cases {
        let error = annotate(&get_features(), options).expect_err(argument);
        assert_eq!(error.kind, ErrorKind::InvalidArgument);
        assert_eq!(error.argument.as_deref(), Some(argument));
    }
}

#[test]
fn a_model_without_a_key_is_refused() {
    let error =
        annotate(&get_features(), get_options("model_without_label")).expect_err("missing key");
    assert_eq!(error.argument.as_deref(), Some("model"));
    assert!(error.message.contains("label"), "{}", error.message);
}

#[test]
fn a_model_with_an_invalid_key_is_refused() {
    let error = annotate(&get_features(), get_options("model_with_bad_max_peaks"))
        .expect_err("invalid key");
    assert_eq!(error.argument.as_deref(), Some("model"));
    assert!(error.message.contains("max_peaks"), "{}", error.message);
}
