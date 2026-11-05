use std::path::PathBuf;

use crate::{
    error::{QuantionError, check_finite, check_not_negative, check_positive},
    utilities::{
        calculate_eic::TimeUnit,
        get_scan_metadata::{SCAN_METADATA_TAGS, get_tag},
        fit_peak::PeakShape,
        structs::FromTo,
    },
};

/// **FindPeaksOptions** · Options
///
/// Peak-picking settings. Every field is optional.
///
/// Fields:
/// - `min_integral`: Drop peaks with a smaller area. Off when not given. Default `None`.
/// - `min_intensity`: Drop peaks with a lower apex. Default `500.0`.
/// - `min_peak_width_points`: Drop peaks with fewer points. Default `3`.
/// - `noise`: Fixed noise level, used when auto_noise is false. Default `None`.
/// - `auto_noise`: Estimate the noise from the trace. Default `true`.
/// - `auto_baseline`: Subtract an airPLS baseline first. Default `true`.
/// - `lambda`: airPLS smoothness. Default `20.0`.
/// - `max_iterations`: Maximum airPLS iterations. Default `4`.
/// - `allow_overlap`: Let neighbouring peaks share points. Default `false`.
/// - `min_snr`: Minimum signal-to-noise ratio. Default `2.0`.
/// - `min_r2`: Minimum fit quality of the peak shape (0–1). 0 keeps every peak. Default `0.0`.
/// - `shape`: Curve used for the fit. Default `EMG`.
/// - `kernel_size`: Smoothing window in points. 0 = no smoothing. Default `0`.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#FindPeaksOptions)
#[derive(Clone, Debug, PartialEq)]
pub struct FindPeaksOptions {
    pub min_integral: Option<f64>,
    pub min_intensity: f64,
    pub min_peak_width_points: usize,
    pub noise: Option<f64>,
    pub auto_noise: bool,
    pub auto_baseline: bool,
    pub lambda: f64,
    pub max_iterations: usize,
    pub allow_overlap: bool,
    pub min_snr: f64,
    pub min_r2: f64,
    pub shape: PeakShape,
    pub kernel_size: usize,
}

impl Default for FindPeaksOptions {
    fn default() -> Self {
        Self {
            min_integral: None,
            min_intensity: 500.0,
            min_peak_width_points: 3,
            noise: None,
            auto_noise: true,
            auto_baseline: true,
            lambda: 20.0,
            max_iterations: 4,
            allow_overlap: false,
            min_snr: 2.0,
            min_r2: 0.0,
            shape: PeakShape::EMG,
            kernel_size: 0,
        }
    }
}

impl FindPeaksOptions {
    pub(crate) fn check(&self, function: &'static str) -> Result<(), QuantionError> {
        if let Some(min_integral) = self.min_integral {
            check_not_negative(function, "find_peaks.min_integral", min_integral)?;
        }
        check_not_negative(function, "find_peaks.min_intensity", self.min_intensity)?;
        if let Some(noise) = self.noise {
            check_not_negative(function, "find_peaks.noise", noise)?;
        }
        check_positive(function, "find_peaks.lambda", self.lambda)?;
        if self.max_iterations == 0 {
            return Err(QuantionError::invalid(function, "find_peaks.max_iterations", "must be greater than 0, got 0"));
        }
        check_not_negative(function, "find_peaks.min_snr", self.min_snr)?;
        if !(0.0..=1.0).contains(&self.min_r2) {
            return Err(QuantionError::invalid(
                function,
                "find_peaks.min_r2",
                format!("must be between 0 and 1, got {}", self.min_r2),
            ));
        }
        Ok(())
    }
}

/// **GetPeakOptions** · Options
///
/// Settings for get_peak.
///
/// Fields:
/// - `range`: Half-width of the search window around rt. 0 searches the whole trace. Default `0.0`.
/// - `find_peaks`: Settings for peak picking. See [FindPeaksOptions](https://phenological.github.io/quantion/?language=rust#FindPeaksOptions).
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#GetPeakOptions)
#[derive(Clone, Debug, PartialEq, Default)]
pub struct GetPeakOptions {
    pub range: f64,
    pub find_peaks: FindPeaksOptions,
}

impl GetPeakOptions {
    pub(crate) fn check(&self, function: &'static str) -> Result<(), QuantionError> {
        check_not_negative(function, "range", self.range)?;
        self.find_peaks.check(function)
    }
}

/// **FitPeakOptions** · Options
///
/// Settings for fit_peak.
///
/// Fields:
/// - `shape`: Curve to fit. Default `EMG`.
/// - `intensity`: Apex intensity to start the fit from. 0 = take it from the peak at rt. Default `0.0`.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#FitPeakOptions)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FitPeakOptions {
    pub shape: PeakShape,
    pub intensity: f64,
}

impl Default for FitPeakOptions {
    fn default() -> Self {
        Self {
            shape: PeakShape::EMG,
            intensity: 0.0,
        }
    }
}

impl FitPeakOptions {
    pub(crate) fn check(&self, function: &'static str) -> Result<(), QuantionError> {
        check_not_negative(function, "intensity", self.intensity)
    }
}

/// **DrawPeaksOptions** · Options
///
/// Settings for draw_peaks.
///
/// Fields:
/// - `from`: Start of the chromatogram. With to, 0 and 0 = fit the range around all the peaks. Default `0.0`.
/// - `to`: End of the chromatogram. Default `0.0`.
/// - `points`: Number of points drawn between from and to. Default `1000`.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#DrawPeaksOptions)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawPeaksOptions {
    pub from: f64,
    pub to: f64,
    pub points: usize,
}

impl Default for DrawPeaksOptions {
    fn default() -> Self {
        Self {
            from: 0.0,
            to: 0.0,
            points: 1000,
        }
    }
}

impl DrawPeaksOptions {
    pub(crate) fn check(&self, function: &'static str) -> Result<(), QuantionError> {
        check_finite(function, "from", self.from)?;
        check_finite(function, "to", self.to)?;
        let fit_range = self.from == 0.0 && self.to == 0.0;
        if !fit_range && self.to <= self.from {
            return Err(QuantionError::invalid(
                function,
                "to",
                format!("must be greater than from ({}), got {}", self.from, self.to),
            ));
        }
        if self.points == 0 {
            return Err(QuantionError::invalid(function, "points", "must be greater than 0, got 0"));
        }
        Ok(())
    }
}

/// **CalculateBaselineOptions** · Options
///
/// airPLS baseline settings for calculate_baseline.
///
/// Fields:
/// - `lambda`: Smoothness. Larger = smoother. Default `20.0`.
/// - `max_iterations`: Maximum iterations. Default `4`.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#CalculateBaselineOptions)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CalculateBaselineOptions {
    pub lambda: f64,
    pub max_iterations: usize,
}

impl Default for CalculateBaselineOptions {
    fn default() -> Self {
        Self {
            lambda: 20.0,
            max_iterations: 4,
        }
    }
}

impl CalculateBaselineOptions {
    pub(crate) fn check(&self, function: &'static str) -> Result<(), QuantionError> {
        check_positive(function, "lambda", self.lambda)?;
        if self.max_iterations == 0 {
            return Err(QuantionError::invalid(function, "max_iterations", "must be greater than 0, got 0"));
        }
        Ok(())
    }
}

/// **CalculateEicOptions** · Options
///
/// Which scans are read and how wide the m/z window is. Used by calculate_eic, and as the eic group of find_targets, find_features and match_features. The wider of the ppm and absolute m/z windows is used.
///
/// Fields:
/// - `from`: Start of the retention-time range read. Default `0.0`.
/// - `to`: End of the retention-time range read. The default reads to the end of the run. Default `f64::INFINITY`.
/// - `ppm_tolerance`: Tolerance in ppm. Default `20.0`.
/// - `mz_tolerance`: Absolute tolerance in Da. Default `0.005`.
/// - `time_unit`: Unit of from, to and rt values given. Default `MINUTES`.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#CalculateEicOptions)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CalculateEicOptions {
    pub from: f64,
    pub to: f64,
    pub ppm_tolerance: f64,
    pub mz_tolerance: f64,
    pub time_unit: TimeUnit,
}

impl Default for CalculateEicOptions {
    fn default() -> Self {
        Self {
            from: 0.0,
            to: f64::INFINITY,
            ppm_tolerance: 20.0,
            mz_tolerance: 0.005,
            time_unit: TimeUnit::MINUTES,
        }
    }
}

impl CalculateEicOptions {
    pub(crate) fn check(&self, function: &'static str, prefix: &str) -> Result<(), QuantionError> {
        check_finite(function, &format!("{prefix}from"), self.from)?;
        if self.to.is_nan() || self.to <= self.from {
            return Err(QuantionError::invalid(
                function,
                format!("{prefix}to"),
                format!("must be greater than from ({}), got {}", self.from, self.to),
            ));
        }
        check_not_negative(function, &format!("{prefix}ppm_tolerance"), self.ppm_tolerance)?;
        check_not_negative(function, &format!("{prefix}mz_tolerance"), self.mz_tolerance)?;
        if self.ppm_tolerance == 0.0 && self.mz_tolerance == 0.0 {
            return Err(QuantionError::invalid(
                function,
                format!("{prefix}ppm_tolerance"),
                "and mz_tolerance cannot both be 0",
            ));
        }
        Ok(())
    }

    pub(crate) fn to_minutes(&self, value: f64) -> f64 {
        self.time_unit.to_minutes(value)
    }

    pub(crate) fn get_window(&self) -> FromTo {
        FromTo {
            from: self.to_minutes(self.from),
            to: self.to_minutes(self.to),
        }
    }

    pub(crate) fn in_minutes(self) -> CalculateEicOptions {
        let window = self.get_window();
        CalculateEicOptions {
            from: window.from,
            to: window.to,
            time_unit: TimeUnit::MINUTES,
            ..self
        }
    }
}

/// **GridOptions** · Options
///
/// The m/z grid that find_features and match_features search.
///
/// Fields:
/// - `start`: Lowest m/z. Default `40.0`.
/// - `end`: Highest m/z. Default `1000.0`.
/// - `step_size`: Step between grid m/z values (Da). Default `0.005`.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#GridOptions)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridOptions {
    pub start: f64,
    pub end: f64,
    pub step_size: f64,
}

impl Default for GridOptions {
    fn default() -> Self {
        Self {
            start: 40.0,
            end: 1000.0,
            step_size: 0.005,
        }
    }
}

impl GridOptions {
    pub(crate) fn check(&self, function: &'static str) -> Result<(), QuantionError> {
        check_positive(function, "grid.start", self.start)?;
        if !self.end.is_finite() || self.end <= self.start {
            return Err(QuantionError::invalid(
                function,
                "grid.end",
                format!("must be greater than grid.start ({}), got {}", self.start, self.end),
            ));
        }
        check_positive(function, "grid.step_size", self.step_size)
    }
}

/// **GroupingOptions** · Options
///
/// How match_features matches features across samples.
///
/// Fields:
/// - `ppm_tolerance`: Same feature when m/z is within this ppm…. Default `5.0`.
/// - `mz_tolerance`: …or within this absolute m/z (Da). Default `0.0025`.
/// - `rt_tolerance`: …and rt is within this many minutes. Default `0.05`.
/// - `min_frequency`: Keep a feature only if found in at least this fraction of samples (0–1). Default `0.0`.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#GroupingOptions)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroupingOptions {
    pub ppm_tolerance: f64,
    pub mz_tolerance: f64,
    pub rt_tolerance: f64,
    pub min_frequency: f64,
}

impl Default for GroupingOptions {
    fn default() -> Self {
        Self {
            ppm_tolerance: 5.0,
            mz_tolerance: 0.0025,
            rt_tolerance: 0.05,
            min_frequency: 0.0,
        }
    }
}

impl GroupingOptions {
    #[cfg(not(all(target_arch = "wasm32", not(target_os = "wasi"))))]
    pub(crate) fn check(&self, function: &'static str) -> Result<(), QuantionError> {
        check_not_negative(function, "grouping.ppm_tolerance", self.ppm_tolerance)?;
        check_not_negative(function, "grouping.mz_tolerance", self.mz_tolerance)?;
        check_positive(function, "grouping.rt_tolerance", self.rt_tolerance)?;
        if !(0.0..=1.0).contains(&self.min_frequency) {
            return Err(QuantionError::invalid(
                function,
                "grouping.min_frequency",
                format!("must be between 0 and 1, got {}", self.min_frequency),
            ));
        }
        Ok(())
    }
}

/// **FindTargetsOptions** · Options
///
/// Settings for find_targets.
///
/// Fields:
/// - `eic`: EIC retention-time range (from, to), tolerances and time unit. See [CalculateEicOptions](https://phenological.github.io/quantion/?language=rust#CalculateEicOptions).
/// - `find_peaks`: Settings for peak picking. See [FindPeaksOptions](https://phenological.github.io/quantion/?language=rust#FindPeaksOptions).
/// - `threads`: Number of threads. Default `1`.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#FindTargetsOptions)
#[derive(Clone, Debug, PartialEq)]
pub struct FindTargetsOptions {
    pub eic: CalculateEicOptions,
    pub find_peaks: FindPeaksOptions,
    pub threads: usize,
}

impl Default for FindTargetsOptions {
    fn default() -> Self {
        Self {
            eic: CalculateEicOptions::default(),
            find_peaks: FindPeaksOptions::default(),
            threads: 1,
        }
    }
}

/// **FindTransitionsOptions** · Options
///
/// Settings for find_transitions.
///
/// Fields:
/// - `find_peaks`: Settings for peak picking. See [FindPeaksOptions](https://phenological.github.io/quantion/?language=rust#FindPeaksOptions).
/// - `threads`: Number of threads. Default `1`.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#FindTransitionsOptions)
#[derive(Clone, Debug, PartialEq)]
pub struct FindTransitionsOptions {
    pub find_peaks: FindPeaksOptions,
    pub threads: usize,
}

impl Default for FindTransitionsOptions {
    fn default() -> Self {
        Self {
            find_peaks: FindPeaksOptions::default(),
            threads: 1,
        }
    }
}

/// **FindFeaturesOptions** · Options
///
/// Settings for find_features.
///
/// Fields:
/// - `eic`: EIC retention-time range (from, to), tolerances and time unit. See [CalculateEicOptions](https://phenological.github.io/quantion/?language=rust#CalculateEicOptions).
/// - `grid`: The m/z grid. See [GridOptions](https://phenological.github.io/quantion/?language=rust#GridOptions).
/// - `find_peaks`: Settings for peak picking. See [FindPeaksOptions](https://phenological.github.io/quantion/?language=rust#FindPeaksOptions).
/// - `threads`: Number of threads. Default `1`.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#FindFeaturesOptions)
#[derive(Clone, Debug, PartialEq)]
pub struct FindFeaturesOptions {
    pub eic: CalculateEicOptions,
    pub grid: GridOptions,
    pub find_peaks: FindPeaksOptions,
    pub threads: usize,
}

impl Default for FindFeaturesOptions {
    fn default() -> Self {
        Self {
            eic: CalculateEicOptions::default(),
            grid: GridOptions::default(),
            find_peaks: FindPeaksOptions::default(),
            threads: 1,
        }
    }
}

/// **MatchFeaturesOptions** · Options
///
/// Settings for match_features: the find_features settings plus grouping.
///
/// Fields:
/// - `eic`: EIC retention-time range (from, to), tolerances and time unit. See [CalculateEicOptions](https://phenological.github.io/quantion/?language=rust#CalculateEicOptions).
/// - `grid`: The m/z grid. See [GridOptions](https://phenological.github.io/quantion/?language=rust#GridOptions).
/// - `grouping`: How features are matched across samples. See [GroupingOptions](https://phenological.github.io/quantion/?language=rust#GroupingOptions).
/// - `find_peaks`: Settings for peak picking. See [FindPeaksOptions](https://phenological.github.io/quantion/?language=rust#FindPeaksOptions).
/// - `threads`: Number of threads. Default `1`.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#MatchFeaturesOptions)
#[derive(Clone, Debug, PartialEq)]
pub struct MatchFeaturesOptions {
    pub eic: CalculateEicOptions,
    pub grid: GridOptions,
    pub grouping: GroupingOptions,
    pub find_peaks: FindPeaksOptions,
    pub threads: usize,
}

impl Default for MatchFeaturesOptions {
    fn default() -> Self {
        Self {
            eic: CalculateEicOptions::default(),
            grid: GridOptions::default(),
            grouping: GroupingOptions::default(),
            find_peaks: FindPeaksOptions::default(),
            threads: 1,
        }
    }
}

/// **DeisotopeFeaturesOptions** · Options
///
/// How deisotope_features links isotope peaks.
///
/// Fields:
/// - `rt_tolerance`: Same compound when rt is within this many minutes of M. Default `0.02`.
/// - `mz_tolerance`: Isotope peak when m/z is within this many Da of the expected position. Default `0.005`.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#DeisotopeFeaturesOptions)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeisotopeFeaturesOptions {
    pub rt_tolerance: f64,
    pub mz_tolerance: f64,
}

impl Default for DeisotopeFeaturesOptions {
    fn default() -> Self {
        Self {
            rt_tolerance: 0.02,
            mz_tolerance: 0.005,
        }
    }
}

impl DeisotopeFeaturesOptions {
    pub(crate) fn check(&self, function: &'static str) -> Result<(), QuantionError> {
        check_not_negative(function, "rt_tolerance", self.rt_tolerance)?;
        check_not_negative(function, "mz_tolerance", self.mz_tolerance)
    }
}

pub(crate) fn check_threads(function: &'static str, threads: usize) -> Result<(), QuantionError> {
    if threads == 0 {
        return Err(QuantionError::invalid(function, "threads", "must be 1 or more, got 0"));
    }
    Ok(())
}

/// **GetScansOptions** · Options
///
/// Settings for get_scans.
///
/// Fields:
/// - `level`: MS level. 0 = every level. Default `1`.
/// - `time_unit`: Unit of rt in the query. Default `MINUTES`.
/// - `arrays`: Which arrays to read from each scan, as keys with accessions. Every other field of Scan always comes back. Default `{ key: "mz", accession: "MS:1000514" }, { key: "intensity", accession: "MS:1000515" }`. See [ScanArray](https://phenological.github.io/quantion/?language=rust#ScanArray).
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#GetScansOptions)
#[derive(Clone, Debug, PartialEq)]
pub struct GetScansOptions {
    pub level: u8,
    pub time_unit: TimeUnit,
    pub arrays: Vec<ScanArray>,
}

impl Default for GetScansOptions {
    fn default() -> Self {
        Self {
            level: 1,
            time_unit: TimeUnit::MINUTES,
            arrays: default_scan_arrays(),
        }
    }
}

/// **GetScanMetadataOptions** · Options
///
/// Settings for get_scan_metadata.
///
/// Fields:
/// - `tags`: Which tags to keep, by name. A row is kept when the last tag of its path matches, so scan keeps spectrum/scanList/scan\[0\] but not its scanWindow. Empty keeps every tag. The names are: spectrum, referenceableParamGroupRef, spectrumDescription, scanList, scan, scanWindowList, scanWindow, precursorList, precursor, isolationWindow, selectedIonList, selectedIon, activation, productList, product, binaryDataArrayList, binaryDataArray. Default `[]`.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#GetScanMetadataOptions)
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GetScanMetadataOptions {
    pub tags: Vec<String>,
}

impl GetScanMetadataOptions {
    pub(crate) fn check(&self, function: &'static str) -> Result<(), QuantionError> {
        for tag in &self.tags {
            if !SCAN_METADATA_TAGS.contains(&tag.as_str()) {
                return Err(QuantionError::invalid(function, "tags", format!("must be one of {}, got {tag:?}", SCAN_METADATA_TAGS.join(", "))));
            }
        }
        Ok(())
    }

    pub(crate) fn keeps(&self, path: &str) -> bool {
        self.tags.is_empty() || self.tags.iter().any(|tag| tag == get_tag(path))
    }
}

/// **ScanArray** · Options
///
/// One array to read from each scan in get_scans.
///
/// Fields:
/// - `key`: Name of the array in the result, as a field of the scan.
/// - `accession`: Accession of the array in the file, such as MS:1000514 for m/z.
///
/// Example:
/// ```ignore
/// let array = ScanArray { key: "mz".to_string(), accession: "MS:1000514".to_string() };
/// ```
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#ScanArray)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanArray {
    pub key: String,
    pub accession: String,
}

pub(crate) const MZ_KEY: &str = "mz";
pub(crate) const INTENSITY_KEY: &str = "intensity";
const MZ_ACCESSION: &str = "MS:1000514";
const INTENSITY_ACCESSION: &str = "MS:1000515";
const RESERVED_SCAN_KEYS: [&str; 15] = [
    "index",
    "rt",
    "level",
    "precursor_mz",
    "precursorMz",
    "base_peak_mz",
    "basePeakMz",
    "base_peak_intensity",
    "basePeakIntensity",
    "total_ion_current",
    "totalIonCurrent",
    "polarity",
    "x",
    "y",
    "z",
];

fn default_scan_arrays() -> Vec<ScanArray> {
    vec![
        ScanArray { key: MZ_KEY.to_string(), accession: MZ_ACCESSION.to_string() },
        ScanArray { key: INTENSITY_KEY.to_string(), accession: INTENSITY_ACCESSION.to_string() },
    ]
}

pub(crate) fn read_accession_number(accession: &str) -> Option<u32> {
    let digits = accession.strip_prefix("MS:")?;
    if digits.is_empty() || !digits.bytes().all(|digit| digit.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

impl GetScansOptions {
    pub(crate) fn has_default_arrays(&self) -> bool {
        self.arrays == default_scan_arrays()
    }

    pub(crate) fn check_arrays(&self, function: &'static str) -> Result<Vec<u32>, QuantionError> {
        if self.arrays.is_empty() {
            return Err(QuantionError::invalid(function, "arrays", "must have at least one array"));
        }
        let mut accessions = Vec::with_capacity(self.arrays.len());
        for (position, array) in self.arrays.iter().enumerate() {
            let number = check_scan_array(function, array)?;
            if self.arrays[..position].iter().any(|other| other.key == array.key) {
                return Err(QuantionError::invalid(function, "arrays.key", format!("repeats {:?}", array.key)));
            }
            accessions.push(number);
        }
        Ok(accessions)
    }
}

fn check_scan_array(function: &'static str, array: &ScanArray) -> Result<u32, QuantionError> {
    if array.key.is_empty() {
        return Err(QuantionError::invalid(function, "arrays.key", "must not be empty"));
    }
    if RESERVED_SCAN_KEYS.contains(&array.key.as_str()) {
        return Err(QuantionError::invalid(function, "arrays.key", format!("cannot be {:?}: a scan already has that field", array.key)));
    }
    let Some(number) = read_accession_number(&array.accession) else {
        return Err(QuantionError::invalid(function, "arrays.accession", format!("must look like MS:1000514, got {:?}", array.accession)));
    };
    let pairs = [(MZ_KEY, MZ_ACCESSION), (INTENSITY_KEY, INTENSITY_ACCESSION)];
    for (key, accession) in pairs {
        if (array.key == key) != (array.accession == accession) {
            return Err(QuantionError::invalid(function, "arrays", format!("{key} and {accession} must go together")));
        }
    }
    Ok(number)
}

/// **ParseIonOptions** · Options
///
/// Settings for parse_ion.
///
/// Fields:
/// - `max_cache_size`: Cache size in bytes. 0 = no limit. Default `0`.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#ParseIonOptions)
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct ParseIonOptions {
    pub max_cache_size: usize,
}

/// **Header** · Inputs
///
/// One HTTP header sent with an upload in write_ion. Values are never printed. A line break, or Host, Content-Length or Transfer-Encoding, gives an invalid_argument error.
///
/// Fields:
/// - `name`: Name of the header, such as Authorization.
/// - `value`: Value of the header.
///
/// Example:
/// ```ignore
/// let header = Header { name: "Authorization".into(), value: "Bearer <token>".into() };
/// ```
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#Header)
#[derive(Clone, Debug, PartialEq)]
pub struct Header {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum IonOutput {
    Path(PathBuf),
    Url(String),
}

impl From<&str> for IonOutput {
    fn from(text: &str) -> Self {
        if crate::utilities::parse_ion::is_url(text) {
            IonOutput::Url(text.to_string())
        } else {
            IonOutput::Path(PathBuf::from(text))
        }
    }
}

impl From<String> for IonOutput {
    fn from(text: String) -> Self {
        IonOutput::from(text.as_str())
    }
}

impl From<PathBuf> for IonOutput {
    fn from(path: PathBuf) -> Self {
        IonOutput::Path(path)
    }
}

/// **WriteIonOptions** · Options
///
/// Settings for write_ion.
///
/// Fields:
/// - `out`: Path or URL to write to. Not given = return the bytes. Default `None`.
/// - `overwrite`: Replace an existing file. Default `false`.
/// - `level`: Compression level, 0 to 22. Default `12`.
/// - `f32_compress`: Store intensities as 32-bit floats. Default `false`.
/// - `block_size`: Size of each data block before compression, in MB. A whole number, 1 or more. Default `8`.
/// - `mz_window`: Width of the m/z windows the spectra are split into, in Da, so a query reads only the windows it needs. A whole number, 0 or more; 0 keeps each spectrum whole. Default `20`.
/// - `headers`: Extra HTTP headers. Only with a URL. Default `empty`.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#WriteIonOptions)
#[derive(Clone, Debug, PartialEq)]
pub struct WriteIonOptions {
    pub out: Option<IonOutput>,
    pub overwrite: bool,
    pub level: u8,
    pub f32_compress: bool,
    pub block_size: u32,
    pub mz_window: u32,
    pub headers: Vec<Header>,
}

impl Default for WriteIonOptions {
    fn default() -> Self {
        Self {
            out: None,
            overwrite: false,
            level: 12,
            f32_compress: false,
            block_size: 8,
            mz_window: 20,
            headers: Vec::new(),
        }
    }
}

/// **WriteMzmlOptions** · Options
///
/// Settings for write_mzml.
///
/// Fields:
/// - `out`: Path to write to. Not given = return the bytes. Default `None`.
/// - `overwrite`: Replace an existing file. Default `false`.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#WriteMzmlOptions)
#[derive(Clone, Debug, PartialEq, Default)]
pub struct WriteMzmlOptions {
    pub out: Option<PathBuf>,
    pub overwrite: bool,
}

/// **GetIonImageOptions** · Options
///
/// Settings for get_ion_image.
///
/// Fields:
/// - `tolerance`: Absolute m/z tolerance (Da). Default `0.125`.
/// - `level`: MS level. Default `1`.
/// - `on_progress`: Progress callback: fetched bytes, total bytes, bytes held. Default `None`.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#GetIonImageOptions)
#[derive(Clone, Copy, Debug)]
pub struct GetIonImageOptions {
    pub tolerance: f64,
    pub level: u8,
    pub on_progress: Option<fn(u64, u64, u64)>,
}

impl Default for GetIonImageOptions {
    fn default() -> Self {
        Self {
            tolerance: 0.125,
            level: 1,
            on_progress: None,
        }
    }
}
