use ionic::{
    ArrayKind, IonError, IonReader, Range,
    mzml::structs::{CvParam, MzML},
    source::ByteRange,
};

use crate::{
    files::CalculateEicOptions,
    utilities::{
        ion::{ScanSource, ScanSummary},
        structs::{DataXY, FromTo, Peak},
    },
};

pub(crate) const MS1_LEVEL: u8 = 1;

fn rt_to_minutes(rt: f64, unit: ionic::TimeUnit) -> f64 {
    match unit {
        ionic::TimeUnit::Second => rt / 60.0,
        ionic::TimeUnit::Millisecond => rt / 60_000.0,
        _ => rt,
    }
}

pub(crate) fn scan_in_minutes(mut summary: ScanSummary) -> ScanSummary {
    summary.rt = rt_to_minutes(summary.rt, summary.rt_unit);
    summary.rt_unit = ionic::TimeUnit::Minute;
    summary
}

/// **TimeUnit** · Enums
///
/// Unit of the retention times you give to a function. Results are always in minutes.
///
/// Fields:
/// - `MINUTES`: Minutes (default).
/// - `SECONDS`: Seconds.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#TimeUnit)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(clippy::upper_case_acronyms)]
pub enum TimeUnit {
    SECONDS,
    #[default]
    MINUTES,
}

impl TimeUnit {
    #[inline]
    pub fn to_minutes(self, value: f64) -> f64 {
        match self {
            TimeUnit::SECONDS => value / 60.0,
            TimeUnit::MINUTES => value,
        }
    }
}

pub enum EicReader<'a> {
    Ion(&'a mut IonReader),
    Mzml(&'a mut MzML),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpectrumKind {
    Centroid,
    Profile,
}

const CENTROID_ACCESSION: &str = "MS:1000127";
const PROFILE_ACCESSION: &str = "MS:1000128";

pub fn get_spectrum_kind(reader: &mut EicReader) -> SpectrumKind {
    match reader {
        EicReader::Ion(ion) => match ion.spectrum_metadata_at(0) {
            Ok(spectrum) => kind_from_params(&spectrum.cv_params),
            Err(_) => SpectrumKind::Centroid,
        },
        EicReader::Mzml(mzml) => match first_spectrum_params(mzml) {
            Some(params) => kind_from_params(params),
            None => SpectrumKind::Centroid,
        },
    }
}

fn kind_from_params(params: &[CvParam]) -> SpectrumKind {
    for param in params {
        match param.accession.as_deref() {
            Some(PROFILE_ACCESSION) => return SpectrumKind::Profile,
            Some(CENTROID_ACCESSION) => return SpectrumKind::Centroid,
            _ => {}
        }
    }
    SpectrumKind::Centroid
}

fn first_spectrum_params(mzml: &MzML) -> Option<&[CvParam]> {
    let list = mzml.run.spectrum_list.as_ref()?;
    list.spectra
        .first()
        .map(|spectrum| spectrum.cv_params.as_slice())
}

#[derive(Clone, Copy, Debug)]
pub struct ScanTime {
    pub index: usize,
    pub rt: f64,
}

#[derive(Clone, Debug)]
pub enum FastError {
    InvalidRequest,
    UnsupportedBackend,
    MissingWindows,
    BadWindowsChecksum,
    MalformedWindows(String),
    ReadFailed(String),
}

impl std::fmt::Display for FastError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FastError::InvalidRequest => write!(f, "invalid EIC request"),
            FastError::UnsupportedBackend => write!(f, "unsupported file backend"),
            FastError::MissingWindows => write!(
                f,
                "spectrum has no window directory; re-encode the file with the current Ionic"
            ),
            FastError::BadWindowsChecksum => write!(f, "spectrum window directory checksum failed"),
            FastError::MalformedWindows(reason) => {
                write!(f, "spectrum window directory malformed: {}", reason)
            }
            FastError::ReadFailed(reason) => write!(f, "read failed: {}", reason),
        }
    }
}

impl std::error::Error for FastError {}

impl From<IonError> for FastError {
    fn from(error: IonError) -> Self {
        match error {
            IonError::MissingSpectrumBounds => FastError::MissingWindows,
            IonError::BadSpectrumBoundsChecksum => FastError::BadWindowsChecksum,
            IonError::MalformedSpectrumBounds(reason) => FastError::MalformedWindows(reason),
            other => FastError::ReadFailed(other.to_string()),
        }
    }
}

pub fn get_scan_times(
    reader: &mut EicReader,
    rt_from: f64,
    rt_to: f64,
    level: u8,
) -> Vec<ScanTime> {
    let mut result = Vec::new();
    let rt_min = rt_from.min(rt_to);
    let rt_max = rt_from.max(rt_to);

    match reader {
        EicReader::Ion(ion) => {
            ion.for_each_summary(&mut |idx, summary| {
                let summary = scan_in_minutes(summary);
                if summary.rt >= rt_min && summary.rt <= rt_max && summary.ms_level == level {
                    result.push(ScanTime {
                        index: idx,
                        rt: summary.rt,
                    });
                }
            });
        }
        EicReader::Mzml(mzml) => {
            mzml.for_each_summary(&mut |idx, summary| {
                let summary = scan_in_minutes(summary);
                if summary.rt >= rt_min && summary.rt <= rt_max && summary.ms_level == level {
                    result.push(ScanTime {
                        index: idx,
                        rt: summary.rt,
                    });
                }
            });
        }
    }
    result
}

pub fn read_mz_window(
    reader: &mut EicReader,
    scan_index: usize,
    mz_from: f64,
    mz_to: f64,
    mz_out: &mut Vec<f64>,
    intensity_out: &mut Vec<f64>,
) -> Result<(), FastError> {
    mz_out.clear();
    intensity_out.clear();

    match reader {
        EicReader::Ion(ion) => {
            let window = ion
                .spectrum_window(
                    scan_index,
                    ArrayKind::Mz,
                    ArrayKind::Intensity,
                    Range {
                        from: mz_from,
                        to: mz_to,
                    },
                )
                .map_err(FastError::from)?;
            *mz_out = window.x.to_f64();
            *intensity_out = window.y.to_f64();
            Ok(())
        }
        EicReader::Mzml(mzml) => {
            if !mzml.load_scan(scan_index, mz_out, intensity_out) {
                return Err(FastError::ReadFailed("failed to load scan".to_string()));
            }

            let start = lower_bound(mz_out, mz_from);
            let end = upper_bound(mz_out, mz_to).min(mz_out.len());
            if end <= start {
                mz_out.clear();
                intensity_out.clear();
                return Ok(());
            }

            let kept = end - start;
            mz_out.copy_within(start..end, 0);
            mz_out.truncate(kept);
            intensity_out.copy_within(start..end, 0);
            intensity_out.truncate(kept);
            Ok(())
        }
    }
}

fn rt_from_minutes(minutes: f64, unit: ionic::TimeUnit) -> f64 {
    match unit {
        ionic::TimeUnit::Second => minutes * 60.0,
        ionic::TimeUnit::Millisecond => minutes * 60_000.0,
        _ => minutes,
    }
}

fn rt_bounds_in_file_units(ion: &mut IonReader, from: f64, to: f64) -> Option<Range> {
    let mut unit = None;
    ion.for_each_summary(&mut |_, summary| {
        if unit.is_none() && summary.rt.is_finite() {
            unit = Some(summary.rt_unit);
        }
    });
    let unit = unit?;
    Some(Range {
        from: rt_from_minutes(from, unit),
        to: rt_from_minutes(to, unit),
    })
}

pub fn plan_window_ranges(
    ion: &mut IonReader,
    from: f64,
    to: f64,
    mz_from: f64,
    mz_to: f64,
) -> Result<Vec<ByteRange>, FastError> {
    ion.require_bounds().map_err(FastError::from)?;
    let rt = rt_bounds_in_file_units(ion, from.min(to), from.max(to));
    ion.eic_byte_ranges(
        Range {
            from: mz_from,
            to: mz_to,
        },
        rt,
        0,
    )
    .map_err(FastError::from)
}

pub fn plan_eic_ranges(
    ion: &mut IonReader,
    mz: f64,
    from: f64,
    to: f64,
    ppm_tolerance: f64,
    mz_tolerance: f64,
) -> Result<Vec<ByteRange>, FastError> {
    if !mz.is_finite() || mz <= 0.0 {
        return Err(FastError::InvalidRequest);
    }

    let options = CalculateEicOptions {
        ppm_tolerance,
        mz_tolerance,
        ..Default::default()
    };

    let tolerance = mz_tolerance_for(mz, options);
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(FastError::InvalidRequest);
    }

    plan_window_ranges(ion, from, to, mz - tolerance, mz + tolerance)
}

pub fn calculate_eic(
    reader: &mut EicReader,
    mz: f64,
    window: FromTo,
    options: CalculateEicOptions,
) -> Result<DataXY, FastError> {
    if !mz.is_finite() || mz <= 0.0 {
        return Err(FastError::InvalidRequest);
    }

    let tolerance = mz_tolerance_for(mz, options);
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(FastError::InvalidRequest);
    }

    let mz_lower = mz - tolerance;
    let mz_upper = mz + tolerance;

    let rt_min = options.time_unit.to_minutes(window.from.min(window.to));
    let rt_max = options.time_unit.to_minutes(window.from.max(window.to));

    let scan_times = get_scan_times(reader, rt_min, rt_max, MS1_LEVEL);

    if scan_times.is_empty() {
        return Ok(DataXY::empty());
    }

    let mut x = Vec::new();
    let mut y = Vec::new();
    let mut mz_buf = Vec::new();
    let mut intensity_buf = Vec::new();

    for scan_time in scan_times {
        x.push(scan_time.rt);
        read_mz_window(
            reader,
            scan_time.index,
            mz_lower,
            mz_upper,
            &mut mz_buf,
            &mut intensity_buf,
        )?;

        let intensity_sum: f64 = intensity_buf.iter().sum();
        y.push(intensity_sum);
    }

    Ok(DataXY { x, y })
}

pub fn with_eic_apex_intensity(rt: &[f64], y: &[f64], mut p: Peak) -> Peak {
    let apex_intensity = max_in_range(rt, y, p.from, p.to);
    if apex_intensity.is_finite() && apex_intensity > 0.0 {
        p.intensity = apex_intensity;
    }
    p
}

#[inline]
pub fn lower_bound(values: &[f64], target: f64) -> usize {
    let mut low = 0usize;
    let mut high = values.len();
    while low < high {
        let mid = (low + high) / 2;
        if values[mid] < target {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    low
}

#[inline]
pub fn upper_bound(values: &[f64], target: f64) -> usize {
    let mut low = 0usize;
    let mut high = values.len();
    while low < high {
        let mid = (low + high) / 2;
        if values[mid] <= target {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    low
}

#[inline]
pub fn mz_tolerance_for(mz: f64, options: CalculateEicOptions) -> f64 {
    let ppm_window = if options.ppm_tolerance > 0.0 {
        (options.ppm_tolerance * 1e-6) * mz.abs()
    } else {
        0.0
    };
    ppm_window.max(options.mz_tolerance.max(0.0))
}

#[inline]
pub(crate) fn summed_intensity_in_window(
    mz: &[f64],
    intensity: &[f64],
    mz_lower: f64,
    mz_upper: f64,
) -> f64 {
    if mz.is_empty() || intensity.is_empty() || mz.len() != intensity.len() {
        return 0.0;
    }
    let start = lower_bound(mz, mz_lower);
    let end = upper_bound(mz, mz_upper);
    if end <= start {
        return 0.0;
    }
    unsafe { intensity.get_unchecked(start..end) }.iter().sum()
}

fn max_in_range(retention_times: &[f64], intensities: &[f64], from_rt: f64, to_rt: f64) -> f64 {
    let start = lower_bound(retention_times, from_rt);
    let end = upper_bound(retention_times, to_rt).min(intensities.len());
    if start >= intensities.len() || end <= start {
        return 0.0;
    }
    let mut maximum = 0.0f64;
    for &value in &intensities[start..end] {
        if value > maximum {
            maximum = value;
        }
    }
    maximum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lower_bound_empty() {
        assert_eq!(lower_bound(&[], 5.0), 0);
    }

    #[test]
    fn lower_bound_before_all() {
        assert_eq!(lower_bound(&[1.0, 2.0, 3.0], 0.0), 0);
    }

    #[test]
    fn lower_bound_after_all() {
        assert_eq!(lower_bound(&[1.0, 2.0, 3.0], 4.0), 3);
    }

    #[test]
    fn lower_bound_exact_match() {
        assert_eq!(lower_bound(&[1.0, 2.0, 3.0], 2.0), 1);
    }

    #[test]
    fn upper_bound_empty() {
        assert_eq!(upper_bound(&[], 5.0), 0);
    }

    #[test]
    fn upper_bound_exact_match() {
        assert_eq!(upper_bound(&[1.0, 2.0, 3.0], 2.0), 2);
    }

    #[test]
    fn upper_bound_after_all() {
        assert_eq!(upper_bound(&[1.0, 2.0, 3.0], 5.0), 3);
    }

    #[test]
    fn summed_intensity_sums_window() {
        let mz = vec![100.0, 200.0, 300.0];
        let int = vec![10.0, 20.0, 30.0];
        assert_eq!(summed_intensity_in_window(&mz, &int, 150.0, 250.0), 20.0);
    }

    #[test]
    fn summed_intensity_empty_input() {
        assert_eq!(summed_intensity_in_window(&[], &[], 0.0, 100.0), 0.0);
    }

    #[test]
    fn summed_intensity_no_match_in_window() {
        let mz = vec![100.0, 200.0];
        let int = vec![10.0, 20.0];
        assert_eq!(summed_intensity_in_window(&mz, &int, 300.0, 400.0), 0.0);
    }

    #[test]
    fn summed_intensity_multiple_in_window() {
        let mz = vec![100.0, 150.0, 200.0, 250.0];
        let int = vec![10.0, 15.0, 20.0, 25.0];
        assert_eq!(summed_intensity_in_window(&mz, &int, 100.0, 200.0), 45.0);
    }

    #[test]
    fn mz_tolerance_ppm_dominates() {
        let opts = CalculateEicOptions {
            ppm_tolerance: 10.0,
            mz_tolerance: 0.001,
            ..Default::default()
        };
        let tol = mz_tolerance_for(500.0, opts);
        assert!((tol - 0.005).abs() < 1e-9);
    }

    #[test]
    fn mz_tolerance_abs_dominates() {
        let opts = CalculateEicOptions {
            ppm_tolerance: 1.0,
            mz_tolerance: 0.1,
            ..Default::default()
        };
        let tol = mz_tolerance_for(500.0, opts);
        assert!((tol - 0.1).abs() < 1e-9);
    }
}
