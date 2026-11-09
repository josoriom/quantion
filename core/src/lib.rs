mod annotations;
mod error;
mod features;
mod ffi;
mod files;
mod models;
mod sample_file;
mod targets;
#[cfg(test)]
mod tests;
mod traces;
#[allow(dead_code)]
pub(crate) mod utilities;

pub use annotations::{AnnotateFeaturesOptions, annotate_features};
pub use error::{ErrorKind, QuantionError};
#[cfg(not(all(target_arch = "wasm32", not(target_os = "wasi"))))]
pub use features::match_features;
pub use features::{
    DeisotopeFeaturesOptions, FindFeaturesOptions, GridOptions, GroupingOptions,
    MatchFeaturesOptions, deisotope_features, find_features,
};
pub use files::{
    CalculateEicOptions, GetIonImageOptions, GetScanMetadataOptions, GetScansOptions, Header,
    IonOutput, ParseIonOptions, Scan, ScanArray, ScanParam, ScanParamKind, WriteIonOptions,
    WriteMzmlOptions, calculate_eic, get_ion_image, get_scan_metadata, get_scans, parse_ion,
    parse_mzml, write_ion, write_mzml,
};
pub use models::{load_model, load_warp};
pub use sample_file::{MzmlInput, SampleFile};
pub use targets::{FindTargetsOptions, FindTransitionsOptions, find_targets, find_transitions};
pub use traces::{
    CalculateBaselineOptions, DrawPeaksOptions, FindPeaksOptions, FitPeakOptions, GetPeakOptions,
    calculate_baseline, draw_peaks, find_noise_level, find_peaks, fit_peak, get_peak,
};
pub use utilities::{
    annotate_features::{CompoundMatch, FeatureAnnotation, FileAnnotation},
    calculate_eic::TimeUnit,
    find_features::Feature,
    find_noise_level::NoiseLevel,
    fit_peak::{PeakParameters, PeakShape},
    ion_image::IonImage,
    load_model::{Model, ModelInput},
    parse_ion::IonInput,
    structs::{DataXY, FromTo, Peak, Position, ScanQuery, Target, Transition},
    warp_model::{PredictedRt, WarpModel},
};
