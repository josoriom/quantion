pub mod calculate_eic;
pub mod ion;
pub(crate) mod get_scan_metadata;
pub mod ion_image;
pub mod parse_ion;

pub mod parallel;

pub mod find_features;

pub mod deisotope_features;

pub mod find_masses;

pub mod bridge;

pub mod find_noise_level;

pub mod find_peaks;

pub(crate) mod resolve_apexes;

pub mod shape_filter;

pub mod fit_peak;

pub mod functions;

pub mod calculate_baseline;

pub mod get_boundaries;

pub mod get_peak;

pub mod find_transitions;

pub mod find_targets;

pub mod load_model;

pub mod warp_model;

pub mod scan_for_peaks;

pub mod structs;

pub mod cheminfo;

pub mod math;
pub use math::closest_index;

#[cfg(not(all(target_arch = "wasm32", not(target_os = "wasi"))))]
pub mod match_features;
pub mod mz_estimator;
#[cfg(test)]
mod tests;
