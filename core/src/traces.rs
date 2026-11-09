use crate::{
    PeakShape,
    error::{QuantionError, check_finite, check_not_negative, check_positive},
    utilities::{
        calculate_baseline::{STANDALONE_EDGE_SLOPE_LEVEL, calculate_baseline as read_baseline},
        find_noise_level::{NoiseLevel, find_noise_level as read_noise_level},
        find_peaks::find_peaks as pick_peaks,
        fit_peak::{PeakParameters, PeakSeed, draw_peaks as draw_curves, fit_peak as fit_curve},
        get_peak::get_peak as pick_peak,
        structs::{DataXY, Peak, Roi},
    },
};

const MIN_TRACE_POINTS: usize = 3;
const FIT_PEAK_RANGE: f64 = 0.05;

pub(crate) fn check_trace(function: &'static str, data: &DataXY) -> Result<(), QuantionError> {
    if data.x.len() != data.y.len() {
        return Err(QuantionError::invalid(
            function,
            "y",
            format!(
                "must have the same length as x ({}), got {}",
                data.x.len(),
                data.y.len()
            ),
        ));
    }
    if data.x.len() < MIN_TRACE_POINTS {
        return Err(QuantionError::invalid(
            function,
            "x",
            format!(
                "must have at least {MIN_TRACE_POINTS} points, got {}",
                data.x.len()
            ),
        ));
    }
    Ok(())
}

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
            return Err(QuantionError::invalid(
                function,
                "find_peaks.max_iterations",
                "must be greater than 0, got 0",
            ));
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

/// **find_peaks** · Traces
///
/// Find every peak in one trace.
///
/// - `data.x`: Retention times, ascending. At least 3 points.
/// - `data.y`: Intensities, same length as x.
/// - `options`: Optional settings: `min_integral` = None, `min_intensity` = 500.0, `min_peak_width_points` = 3, `noise` = None, `auto_noise` = true, `auto_baseline` = true, `lambda` = 20.0, `max_iterations` = 4, `allow_overlap` = false, `min_snr` = 2.0, `min_r2` = 0.0, `shape` = EMG, `kernel_size` = 0. See [FindPeaksOptions](https://phenological.github.io/quantion/?language=rust#FindPeaksOptions).
///
/// Returns: Every peak that passes the filters.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#find_peaks)
pub fn find_peaks(data: &DataXY, options: FindPeaksOptions) -> Result<Vec<Peak>, QuantionError> {
    const FUNCTION: &str = "find_peaks";
    check_trace(FUNCTION, data)?;
    options.check(FUNCTION)?;
    Ok(pick_peaks(data, &options))
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

/// **get_peak** · Traces
///
/// Find the one peak near rt. With a range, only rt ± range is searched; without one, the whole trace is searched and the peak is chosen by its size and its distance to rt.
///
/// - `data.x`: Retention times, ascending. At least 3 points.
/// - `data.y`: Intensities, same length as x.
/// - `rt`: Expected retention time of the peak.
/// - `options`: Optional settings: `range` = 0.0, `find_peaks`. See [GetPeakOptions](https://phenological.github.io/quantion/?language=rust#GetPeakOptions).
///
/// Returns: The best peak near rt. No peak gives a Peak with every field 0.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#get_peak)
pub fn get_peak(data: &DataXY, rt: f64, options: GetPeakOptions) -> Result<Peak, QuantionError> {
    const FUNCTION: &str = "get_peak";
    check_trace(FUNCTION, data)?;
    check_finite(FUNCTION, "rt", rt)?;
    options.check(FUNCTION)?;
    let roi = Roi {
        rt,
        range: options.range,
    };
    Ok(pick_peak(data, &roi, &options.find_peaks))
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

/// **fit_peak** · Traces
///
/// Fit a Gaussian or EMG curve to the peak near rt. rt says which peak; its apex intensity is the starting point unless you force one.
///
/// - `data.x`: Retention times, ascending. At least 3 points.
/// - `data.y`: Intensities, same length as x.
/// - `rt`: Expected retention time of the peak.
/// - `options`: Optional settings: `shape` = EMG, `intensity` = 0.0. See [FitPeakOptions](https://phenological.github.io/quantion/?language=rust#FitPeakOptions).
///
/// Returns: The fitted curve, or none when no peak is found or the fit fails.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#fit_peak)
pub fn fit_peak(
    data: &DataXY,
    rt: f64,
    options: FitPeakOptions,
) -> Result<Option<PeakParameters>, QuantionError> {
    const FUNCTION: &str = "fit_peak";
    check_trace(FUNCTION, data)?;
    check_finite(FUNCTION, "rt", rt)?;
    options.check(FUNCTION)?;
    let roi = Roi {
        rt,
        range: FIT_PEAK_RANGE,
    };
    let peak = pick_peak(data, &roi, &FindPeaksOptions::default());
    if peak.intensity <= 0.0 || peak.to <= peak.from {
        return Ok(None);
    }
    let intensity = if options.intensity > 0.0 {
        options.intensity
    } else {
        peak.intensity
    };
    let seed = PeakSeed {
        rt: peak.rt,
        intensity,
    };
    Ok(fit_curve(
        &get_section(data, peak.from, peak.to),
        &seed,
        options.shape,
    ))
}

fn get_section(data: &DataXY, from: f64, to: f64) -> DataXY {
    let (x, y) = data
        .x
        .iter()
        .zip(&data.y)
        .filter(|(rt, _)| **rt >= from && **rt <= to)
        .map(|(rt, value)| (*rt, *value))
        .unzip();
    DataXY { x, y }
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
            return Err(QuantionError::invalid(
                function,
                "points",
                "must be greater than 0, got 0",
            ));
        }
        Ok(())
    }
}

/// **draw_peaks** · Traces
///
/// Simulate a chromatogram from one or more peak shapes. Every peak is drawn into the same chromatogram; where they overlap, intensities add up. No real data is needed.
///
/// - `params`: The peaks to draw, one PeakParameters per peak. Each PeakParameters has `height`, `center`, `fwhm`, `tail`, `r2`. See [PeakParameters](https://phenological.github.io/quantion/?language=rust#PeakParameters).
///   Example: `let peak = PeakParameters { height: 9800.0, center: 2.38, fwhm: 0.021, tail: 0.006, r2: 0.0 };`
/// - `options`: Optional settings: `from` = 0.0, `to` = 0.0, `points` = 1000. See [DrawPeaksOptions](https://phenological.github.io/quantion/?language=rust#DrawPeaksOptions).
///
/// Returns: The simulated chromatogram: x = retention times, y = summed intensities.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#draw_peaks)
pub fn draw_peaks(
    params: &[PeakParameters],
    options: DrawPeaksOptions,
) -> Result<DataXY, QuantionError> {
    const FUNCTION: &str = "draw_peaks";
    options.check(FUNCTION)?;
    for (index, peak) in params.iter().enumerate() {
        check_finite(FUNCTION, &format!("params[{index}].center"), peak.center)?;
        if !(peak.fwhm.is_finite() && peak.fwhm > 0.0) {
            return Err(QuantionError::invalid(
                FUNCTION,
                format!("params[{index}].fwhm"),
                format!("must be greater than 0, got {}", peak.fwhm),
            ));
        }
        if !(peak.tail.is_finite() && peak.tail >= 0.0) {
            return Err(QuantionError::invalid(
                FUNCTION,
                format!("params[{index}].tail"),
                format!("must be 0 or more, got {}", peak.tail),
            ));
        }
    }
    Ok(draw_curves(
        params,
        options.from,
        options.to,
        options.points,
    ))
}

/// **find_noise_level** · Traces
///
/// Estimate the noise level of a signal.
///
/// - `y`: Intensities.
///
/// Returns: The noise intensity and the window width it came from.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#find_noise_level)
pub fn find_noise_level(y: &[f64]) -> NoiseLevel {
    read_noise_level(y)
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
            return Err(QuantionError::invalid(
                function,
                "max_iterations",
                "must be greater than 0, got 0",
            ));
        }
        Ok(())
    }
}

/// **calculate_baseline** · Traces
///
/// Estimate a signal's baseline with airPLS.
///
/// - `y`: Intensities.
/// - `options`: Optional settings: `lambda` = 20.0, `max_iterations` = 4. See [CalculateBaselineOptions](https://phenological.github.io/quantion/?language=rust#CalculateBaselineOptions).
///
/// Returns: The baseline, same length as y.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#calculate_baseline)
pub fn calculate_baseline(
    y: &[f64],
    options: CalculateBaselineOptions,
) -> Result<Vec<f64>, QuantionError> {
    const FUNCTION: &str = "calculate_baseline";
    options.check(FUNCTION)?;
    Ok(read_baseline(y, options, STANDALONE_EDGE_SLOPE_LEVEL))
}
