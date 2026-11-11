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
