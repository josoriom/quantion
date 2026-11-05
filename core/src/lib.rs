mod error;
mod features;
mod ffi;
mod files;
mod models;
mod options;
mod sample_file;
mod targets;
mod traces;
#[cfg(test)]
mod tests;
#[allow(dead_code)]
pub(crate) mod utilities;

pub use error::{ErrorKind, QuantionError};
#[cfg(not(all(target_arch = "wasm32", not(target_os = "wasi"))))]
pub use features::match_features;
pub use features::{deisotope_features, find_features};
pub use files::{Scan, ScanParam, ScanParamKind, calculate_eic, get_ion_image, get_scan_metadata, get_scans, parse_ion, parse_mzml, write_ion, write_mzml};
pub use models::{load_model, load_warp};
pub use options::{
    CalculateBaselineOptions, DeisotopeFeaturesOptions, DrawPeaksOptions, CalculateEicOptions, FindFeaturesOptions, FindPeaksOptions, FindTargetsOptions,
    FindTransitionsOptions, FitPeakOptions, GetPeakOptions, GetScanMetadataOptions, GetScansOptions, GridOptions, ScanArray, GroupingOptions, Header,
    GetIonImageOptions, IonOutput, MatchFeaturesOptions, ParseIonOptions, WriteIonOptions, WriteMzmlOptions,
};
pub use sample_file::{MzmlInput, SampleFile};
pub use targets::{find_targets, find_transitions};
pub use traces::{calculate_baseline, draw_peaks, find_noise_level, find_peaks, fit_peak, get_peak};
pub use utilities::{
    calculate_eic::{ScanQuery, TimeUnit},
    find_features::Feature,
    find_noise_level::NoiseLevel,
    fit_peak::{PeakParameters, PeakShape},
    ion_image::IonImage,
    load_model::{Model, ModelInput},
    parse_ion::IonInput,
    structs::{DataXY, FromTo, Peak, Target, Transition},
    warp_model::{PredictedRt, WarpModel},
};
