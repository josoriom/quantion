use crate::utilities::structs::FromTo;

/// **ScanQuery** · Inputs
///
/// Which scans get_scans returns: by retention time, by precursor m/z, or by both. Two ranges give every scan inside both; when rt or mz is one value, you get the one scan closest to it.
///
/// Fields:
/// - `rt`: Retention time: a range, one value for the closest scan, or nothing. Default Any.
/// - `mz`: Precursor m/z: a range, one value for the closest scan, or nothing. Default Any.
///
/// Example:
/// ```ignore
/// let query = ScanQuery { rt: Position::Range(FromTo { from: 2.3, to: 2.4 }), mz: Position::Closest(300.0) };
/// ```
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#ScanQuery)
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScanQuery {
    pub rt: Position,
    pub mz: Position,
}

/// **Position** · Inputs
///
/// One part of a ScanQuery in Rust: where rt or m/z must be. Python, JS and R give a number, a {from, to} range, or nothing instead.
///
/// Fields:
/// - `Any`: No filter (default).
/// - `Range(FromTo)`: Every scan with the value in the range.
/// - `Closest(f64)`: Only the scan with the value closest to this one.
///
/// Example:
/// ```ignore
/// let window = Position::Range(FromTo { from: 2.3, to: 2.4 });
/// ```
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#Position)
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Position {
    #[default]
    Any,
    Range(FromTo),
    Closest(f64),
}
