use crate::{
    CalculateEicOptions,
    error::{QuantionError, check_mz_and_rt, check_positive, check_tolerances},
    files::{ParseIonOptions, from_fast_error, read_error},
    sample_file::{FileSource, SampleFile},
    utilities::{
        annotate_features::{
            AnnotationError, FeatureAnnotation, annotate_features as read_matches,
            plan_library_ranges, plan_sample_ranges,
        },
        calculate_eic::mz_tolerance_for,
        find_features::Feature,
        spectrum_model::{SpectrumModel, SpectrumModelError},
        structs::FromTo,
    },
};

pub(crate) const DEFAULT_LIBRARY: &str =
    "https://github.com/josoriom/cipher/releases/latest/download/library_positive.ion";

pub(crate) const DEFAULT_MODEL: &str =
    "https://github.com/josoriom/cipher/releases/latest/download/model.onnx";

/// **AnnotateFeaturesOptions** · Options
///
/// Settings for annotate_features.
///
/// Fields:
/// - `library`: Spectral library: an Ionic (.ion) file path or URL. Each spectrum has a precursor m/z, a vector array and the params named by the model. Default `"https://github.com/josoriom/cipher/releases/latest/download/library_positive.ion"`.
/// - `model`: Spectrum model: an ONNX file path or URL. Default `"https://github.com/josoriom/cipher/releases/latest/download/model.onnx"`.
/// - `matches`: How many matches to keep per file, best first. Default `3`.
/// - `ppm_tolerance`: Precursor tolerance in ppm. The larger of the two tolerances is used. Default `20.0`.
/// - `mz_tolerance`: Precursor tolerance in Da. Default `0.005`.
/// - `rt_range`: Largest distance in minutes from the peak apex to the MS2 scan. Default `0.3`.
/// - `min_relative_intensity`: MS2 peaks below this share of the base peak are dropped (0–1). Default `0.02`.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#AnnotateFeaturesOptions)
#[derive(Clone, Debug, PartialEq)]
pub struct AnnotateFeaturesOptions {
    pub library: String,
    pub model: String,
    pub matches: usize,
    pub ppm_tolerance: f64,
    pub mz_tolerance: f64,
    pub rt_range: f64,
    pub min_relative_intensity: f64,
}

impl Default for AnnotateFeaturesOptions {
    fn default() -> Self {
        Self {
            library: DEFAULT_LIBRARY.to_string(),
            model: DEFAULT_MODEL.to_string(),
            matches: 3,
            ppm_tolerance: 20.0,
            mz_tolerance: 0.005,
            rt_range: 0.3,
            min_relative_intensity: 0.02,
        }
    }
}

impl AnnotateFeaturesOptions {
    pub(crate) fn check(&self, function: &'static str) -> Result<(), QuantionError> {
        if self.library.trim().is_empty() {
            return Err(QuantionError::invalid(
                function,
                "library",
                "must be a path or a URL, got an empty text",
            ));
        }
        if self.model.trim().is_empty() {
            return Err(QuantionError::invalid(
                function,
                "model",
                "must be a path or a URL, got an empty text",
            ));
        }
        self.check_settings(function)
    }

    pub(crate) fn check_settings(&self, function: &'static str) -> Result<(), QuantionError> {
        if self.matches == 0 {
            return Err(QuantionError::invalid(
                function,
                "matches",
                "must be 1 or more, got 0",
            ));
        }
        check_tolerances(function, "", self.ppm_tolerance, self.mz_tolerance)?;
        check_positive(function, "rt_range", self.rt_range)?;
        if !(0.0..=1.0).contains(&self.min_relative_intensity) {
            return Err(QuantionError::invalid(
                function,
                "min_relative_intensity",
                format!(
                    "must be between 0 and 1, got {}",
                    self.min_relative_intensity
                ),
            ));
        }
        Ok(())
    }

    pub(crate) fn get_precursor_window(&self, mz: f64) -> FromTo {
        let precursor = CalculateEicOptions {
            ppm_tolerance: self.ppm_tolerance,
            mz_tolerance: self.mz_tolerance,
            ..Default::default()
        };
        let tolerance = mz_tolerance_for(mz, precursor);
        FromTo {
            from: mz - tolerance,
            to: mz + tolerance,
        }
    }
}

/// **annotate_features** · Untargeted
///
/// Annotate features by matching their MS2 spectra against a spectral library. For each feature and sample, the positive MS2 scan closest to the peak apex is used: its precursor must match the feature m/z within the tolerance, and its RT must be within rt_range. A spectrum encoder turns the scan into a vector, which is scored against the library spectra with the same precursor m/z. The library (an Ionic .ion file) and the encoder (an ONNX model) are read from a path or a URL; by default both come from the josoriom/cipher releases.
///
/// **Warning:** This is an experimental feature.
///
/// - `files`: The samples, from parse_ion or parse_mzml. Must not be empty.
/// - `features`: The features to name: mz and rt each. find_features and match_features output works as is. Empty gives an empty result.
/// - `options`: Optional settings: `library` = "https://github.com/josoriom/cipher/releases/latest/download/library_positive.ion", `model` = "https://github.com/josoriom/cipher/releases/latest/download/model.onnx", `matches` = 3, `ppm_tolerance` = 20.0, `mz_tolerance` = 0.005, `rt_range` = 0.3, `min_relative_intensity` = 0.02. See [AnnotateFeaturesOptions](https://phenological.github.io/quantion/?language=rust#AnnotateFeaturesOptions).
///
/// Returns: One FeatureAnnotation per feature, in the same order.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#annotate_features)
pub fn annotate_features(
    files: &mut [SampleFile],
    features: &[Feature],
    options: AnnotateFeaturesOptions,
) -> Result<Vec<FeatureAnnotation>, QuantionError> {
    const FUNCTION: &str = "annotate_features";
    options.check(FUNCTION)?;
    check_files(FUNCTION, files.len())?;
    check_features(FUNCTION, features)?;
    if features.is_empty() {
        return Ok(Vec::new());
    }
    let model = crate::load_model(options.model.as_str()).map_err(|error| QuantionError {
        function: FUNCTION,
        ..error
    })?;
    let model =
        SpectrumModel::read(&model).map_err(|error| from_spectrum_model_error(FUNCTION, error))?;
    let mut library = crate::parse_ion(options.library.as_str(), ParseIonOptions::default())
        .map_err(|error| QuantionError {
            function: FUNCTION,
            ..error
        })?;
    let mut files: Vec<&mut SampleFile> = files.iter_mut().collect();
    for file in files.iter_mut() {
        prefetch_sample(FUNCTION, file, features, &options)?;
    }
    annotate_with_library(
        FUNCTION,
        &mut library,
        &model,
        &mut files,
        features,
        &options,
    )
}

pub(crate) fn prefetch_sample(
    function: &'static str,
    file: &mut SampleFile,
    features: &[Feature],
    options: &AnnotateFeaturesOptions,
) -> Result<(), QuantionError> {
    if !file.is_remote() {
        return Ok(());
    }
    let FileSource::Remote(ion) = &mut file.source else {
        return Ok(());
    };
    let ranges = plan_sample_ranges(ion, features, options)
        .map_err(|error| from_fast_error(function, error))?;
    file.prefetch(ranges)
        .map_err(|error| read_error(function, error))
}

pub(crate) fn annotate_with_library(
    function: &'static str,
    library: &mut SampleFile,
    model: &SpectrumModel,
    files: &mut [&mut SampleFile],
    features: &[Feature],
    options: &AnnotateFeaturesOptions,
) -> Result<Vec<FeatureAnnotation>, QuantionError> {
    let is_remote = library.is_remote();
    let (FileSource::Lazy(ion) | FileSource::Remote(ion)) = &mut library.source else {
        return Err(QuantionError::invalid(
            function,
            "library",
            "must be an .ion file",
        ));
    };
    if is_remote {
        let ranges = plan_library_ranges(ion, files, features, options, model.vector_accession)
            .map_err(|error| from_fast_error(function, error))?;
        library
            .prefetch(ranges)
            .map_err(|error| read_error(function, error))?;
    }
    let (FileSource::Lazy(ion) | FileSource::Remote(ion)) = &mut library.source else {
        return Err(QuantionError::invalid(
            function,
            "library",
            "must be an .ion file",
        ));
    };
    read_matches(files, features, ion, model, options)
        .map_err(|error| from_annotation_error(function, error))
}

pub(crate) fn from_spectrum_model_error(
    function: &'static str,
    error: SpectrumModelError,
) -> QuantionError {
    QuantionError::invalid(function, "model", error.to_string())
}

fn from_annotation_error(function: &'static str, error: AnnotationError) -> QuantionError {
    match error {
        AnnotationError::ReadFailed(error) => from_fast_error(function, error),
        AnnotationError::Model(error) => from_spectrum_model_error(function, error),
        other => QuantionError::invalid(function, "library", other.to_string()),
    }
}

pub(crate) fn check_features(
    function: &'static str,
    features: &[Feature],
) -> Result<(), QuantionError> {
    for (index, feature) in features.iter().enumerate() {
        check_mz_and_rt(
            function,
            &format!("features[{index}]"),
            feature.mz,
            feature.rt,
        )?;
    }
    Ok(())
}

pub(crate) fn check_files(function: &'static str, file_count: usize) -> Result<(), QuantionError> {
    if file_count == 0 {
        return Err(QuantionError::invalid(
            function,
            "files",
            "must have at least one sample",
        ));
    }
    Ok(())
}
