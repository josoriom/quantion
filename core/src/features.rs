#[cfg(not(all(target_arch = "wasm32", not(target_os = "wasi"))))]
use std::path::Path;

#[cfg(not(all(target_arch = "wasm32", not(target_os = "wasi"))))]
use crate::{
    error::ErrorKind,
    options::{GroupingOptions, MatchFeaturesOptions},
    utilities::{
        find_features::MzTolerance,
        match_features::{AlignmentError, MatchSettings, match_features as read_matches},
    },
};
use crate::{
    error::{QuantionError, check_finite},
    files::{from_fast_error, read_error},
    options::{CalculateEicOptions, DeisotopeFeaturesOptions, FindFeaturesOptions, FindPeaksOptions, GridOptions, check_threads},
    sample_file::SampleFile,
    utilities::{
        deisotope_features::deisotope_features as remove_isotopes,
        find_features::{Feature, FeatureError, FeatureSettings, find_features as read_features},
    },
};

fn get_feature_settings(eic: &CalculateEicOptions, grid: &GridOptions, find_peaks: &FindPeaksOptions, threads: usize) -> FeatureSettings {
    let mut settings = FeatureSettings::default();
    settings.final_eic_options = eic.in_minutes();
    settings.mz_scan_grid.min_mz = grid.start;
    settings.mz_scan_grid.max_mz = grid.end;
    settings.mz_scan_grid.step = grid.step_size;
    settings.peak_options = find_peaks.clone();
    settings.threads = threads;
    settings
}

fn from_feature_error(function: &'static str, error: FeatureError) -> QuantionError {
    match error {
        FeatureError::ReadFailed(error) => from_fast_error(function, error),
        FeatureError::InvalidStepSize(_) | FeatureError::EmptyGrid | FeatureError::GridTooLarge(_) => {
            QuantionError::invalid(function, "grid", format!("is not usable: {error}"))
        }
        FeatureError::TooManyCandidateMasses(_) => {
            QuantionError::invalid(function, "grid", format!("is too dense: {error}; narrow it or raise step_size"))
        }
        FeatureError::NoScans | FeatureError::NoCandidateMasses => read_error(function, error),
    }
}

/// **find_features** · Untargeted
///
/// Untargeted: find every feature in one file by scanning an m/z grid.
///
/// - `file`: The sample to read, from parse_ion or parse_mzml.
/// - `options`: Optional settings: `eic`, `grid`, `find_peaks`, `threads` = 1. See [FindFeaturesOptions](https://phenological.github.io/quantion/?language=rust#FindFeaturesOptions).
///
/// Returns: Every feature found, without frequency.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#find_features)
pub fn find_features(file: &mut SampleFile, options: FindFeaturesOptions) -> Result<Vec<Feature>, QuantionError> {
    const FUNCTION: &str = "find_features";
    options.eic.check(FUNCTION, "eic.")?;
    options.grid.check(FUNCTION)?;
    options.find_peaks.check(FUNCTION)?;
    check_threads(FUNCTION, options.threads)?;
    let settings = get_feature_settings(&options.eic, &options.grid, &options.find_peaks, options.threads);
    match read_features(&mut file.reader(), options.eic.get_window(), settings) {
        Ok(features) => Ok(features),
        Err(FeatureError::NoScans | FeatureError::NoCandidateMasses) => Ok(Vec::new()),
        Err(error) => Err(from_feature_error(FUNCTION, error)),
    }
}

/// **match_features** · Untargeted
///
/// Untargeted across samples: find features in every file of a folder and match them. Values are medians over the samples where a feature was found.
///
/// - `directory`: Folder with the Ionic (.ion) or mzML files.
/// - `options`: Optional settings: `eic`, `grid`, `grouping`, `find_peaks`, `threads` = 1. See [MatchFeaturesOptions](https://phenological.github.io/quantion/?language=rust#MatchFeaturesOptions).
///
/// Returns: One Feature per matched feature; frequency is the fraction of samples (0–1).
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#match_features)
#[cfg(not(all(target_arch = "wasm32", not(target_os = "wasi"))))]
pub fn match_features(directory: impl AsRef<Path>, options: MatchFeaturesOptions) -> Result<Vec<Feature>, QuantionError> {
    const FUNCTION: &str = "match_features";
    let directory = directory.as_ref();
    if !directory.is_dir() {
        return Err(QuantionError {
            kind: ErrorKind::FileNotFound,
            function: FUNCTION,
            argument: Some("directory".to_string()),
            message: format!("{} is not a folder", directory.display()),
        });
    }
    options.eic.check(FUNCTION, "eic.")?;
    options.grid.check(FUNCTION)?;
    options.grouping.check(FUNCTION)?;
    options.find_peaks.check(FUNCTION)?;
    check_threads(FUNCTION, options.threads)?;
    let features = get_feature_settings(&options.eic, &options.grid, &options.find_peaks, options.threads);
    let GroupingOptions {
        ppm_tolerance,
        mz_tolerance,
        rt_tolerance,
        min_frequency,
    } = options.grouping;
    let settings = MatchSettings {
        features,
        mz_tolerance: MzTolerance {
            mz_absolute: mz_tolerance,
            ppm: ppm_tolerance,
        },
        rt_tolerance,
        min_frequency,
        eic_options: options.eic.in_minutes(),
        peak_options: options.find_peaks.clone(),
        threads: options.threads,
        ..MatchSettings::default()
    };
    let text = directory.to_string_lossy();
    read_matches(&text, options.eic.get_window(), settings).map_err(|error| from_alignment_error(FUNCTION, error))
}

#[cfg(not(all(target_arch = "wasm32", not(target_os = "wasi"))))]
fn from_alignment_error(function: &'static str, error: AlignmentError) -> QuantionError {
    match error {
        AlignmentError::NoFiles => QuantionError {
            kind: ErrorKind::FileNotFound,
            function,
            argument: Some("directory".to_string()),
            message: "has no .ion or mzML files".to_string(),
        },
        AlignmentError::FeatureDetection { path, source } => {
            let inner = from_feature_error(function, source);
            QuantionError {
                message: format!("{path}: {}", inner.message),
                ..inner
            }
        }
        AlignmentError::FastPath { path, source } => read_error(function, format!("{path}: {source}")),
        other => read_error(function, other),
    }
}

/// **deisotope_features** · Untargeted
///
/// Remove 13C isotope peaks from a feature list, keeping the monoisotopic peak (M) of each compound. A feature is an isotope when it sits 1.00335/charge above the previous peak (charge = 1 or 2), has the same rt as M, and its intensity is not too high: M+1 at most M × mz × charge / 12 × 0.0108157.
///
/// - `features`: The features to clean, from find_features or match_features.
/// - `options`: Optional settings: `rt_tolerance` = 0.02, `mz_tolerance` = 0.005. See [DeisotopeFeaturesOptions](https://phenological.github.io/quantion/?language=rust#DeisotopeFeaturesOptions).
///
/// Returns: The features that are not isotopes, sorted by m/z.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#deisotope_features)
pub fn deisotope_features(features: Vec<Feature>, options: DeisotopeFeaturesOptions) -> Result<Vec<Feature>, QuantionError> {
    const FUNCTION: &str = "deisotope_features";
    options.check(FUNCTION)?;
    for (index, feature) in features.iter().enumerate() {
        check_finite(FUNCTION, &format!("features[{index}].mz"), feature.mz)?;
        check_finite(FUNCTION, &format!("features[{index}].rt"), feature.rt)?;
        check_finite(FUNCTION, &format!("features[{index}].intensity"), feature.intensity)?;
    }
    Ok(remove_isotopes(features, options))
}
