use serde::Deserialize;

/// **DataXY** · Results
///
/// A trace: x = retention times, y = intensities. Returned by calculate_eic, taken by find_peaks.
///
/// Fields:
/// - `x`: Retention times (minutes).
/// - `y`: Intensities.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#DataXY)
#[derive(Clone, Debug, Default, Deserialize)]
pub struct DataXY {
    pub x: Vec<f64>,
    pub y: Vec<f64>,
}

impl DataXY {
    pub fn empty() -> DataXY {
        Self {
            x: Vec::new(),
            y: Vec::new(),
        }
    }
}

/// **FromTo** · Inputs
///
/// A range: retention-time windows and m/z ranges.
///
/// Fields:
/// - `from`: Start of the range.
/// - `to`: End of the range.
///
/// Example:
/// ```ignore
/// let window = FromTo { from: 1.0, to: 2.5 };
/// ```
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#FromTo)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FromTo {
    pub from: f64,
    pub to: f64,
}

/// **Peak** · Results
///
/// One chromatographic peak. No peak gives every field 0.
///
/// Fields:
/// - `rt`: Apex retention time (minutes).
/// - `from`: Start of the peak.
/// - `to`: End of the peak.
/// - `intensity`: Apex intensity.
/// - `integral`: Area of the peak.
/// - `n_points`: Points across the peak.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#Peak)
#[derive(Clone, Copy, Deserialize)]
pub struct Peak {
    pub from: f64,
    pub to: f64,
    pub rt: f64,
    pub integral: f64,
    pub intensity: f64,
    pub n_points: usize,
    pub(crate) noise: f64,
    pub(crate) r2: f64,
}

impl std::fmt::Debug for Peak {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Peak")
            .field("from", &self.from)
            .field("to", &self.to)
            .field("rt", &self.rt)
            .field("integral", &self.integral)
            .field("intensity", &self.intensity)
            .field("n_points", &self.n_points)
            .finish()
    }
}

impl Default for Peak {
    fn default() -> Self {
        Self {
            from: 0.0,
            to: 0.0,
            rt: 0.0,
            integral: 0.0,
            intensity: 0.0,
            r2: 0.0,
            n_points: 0,
            noise: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Roi {
    pub rt: f64,
    pub range: f64,
}

/// **Target** · Inputs
///
/// One compound. range limits the search in find_targets; missing or 0 searches the whole EIC. predict_rts and calibrate do not use range.
///
/// Fields:
/// - `id`: Your name for it. Not used in the search. Optional.
/// - `mz`: m/z (Da).
/// - `rt`: Expected retention time.
/// - `range`: Half-width of the search window around rt. Missing or 0 = search the whole EIC. Optional.
///
/// Example:
/// ```ignore
/// let target = Target { id: Some("compound_1".into()), mz: 260.103, rt: 2.383, range: Some(0.05) };
/// ```
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#Target)
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Target {
    pub id: Option<String>,
    pub mz: f64,
    pub rt: f64,
    pub range: Option<f64>,
}

impl Target {
    pub(crate) fn get_id(&self) -> &str {
        self.id.as_deref().unwrap_or("")
    }

    pub(crate) fn get_range(&self) -> f64 {
        self.range.unwrap_or(0.0)
    }
}

/// **Transition** · Inputs
///
/// One SRM transition.
///
/// Fields:
/// - `id`: Your name for it. Optional.
/// - `precursor`: Precursor (Q1) m/z.
/// - `product`: Product (Q3) m/z.
/// - `rt`: Expected retention time.
/// - `range`: Half-width of the search window around rt.
///
/// Example:
/// ```ignore
/// let transition = Transition { id: Some("compound_1".into()), precursor: 89.04768, product: 89.04768, rt: 2.385, range: 0.1 };
/// ```
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#Transition)
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Transition {
    pub id: Option<String>,
    pub precursor: f64,
    pub product: f64,
    pub rt: f64,
    pub range: f64,
}
