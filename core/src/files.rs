use std::{
    collections::BTreeMap,
    fmt::Display,
    fs,
    ops::Index,
    path::{Path, PathBuf},
};

use ionic::{
    IonError, ReadOptions, WriteOptions,
    mzml::{bin_to_mzml, parse_mzml as read_mzml},
};

use crate::{
    FromTo, TimeUnit,
    error::{
        ErrorKind, QuantionError, check_finite, check_not_negative, check_positive,
        check_tolerances,
    },
    sample_file::{FileSource, MzmlInput, SampleFile, is_url_path},
    utilities::{
        calculate_eic::{FastError, calculate_eic as read_eic},
        get_scan_metadata::{SCAN_METADATA_TAGS, get_tag},
        get_scans::{
            CentroidScan, ScanArrays, get_scan_arrays as read_scan_arrays, get_scans as read_scans,
        },
        ion::{ScanSource, write_mzml_to_ion},
        ion_image::{IonImage, image_session_begin},
        parse_ion::{IonInput, parse_ion_with_prefetch},
        structs::{DataXY, Position, ScanQuery},
    },
};

const MAX_COMPRESSION_LEVEL: u8 = 22;
const IMAGE_SCANS_PER_STEP: usize = 256;
const RESERVED_HEADERS: [&str; 3] = ["host", "content-length", "transfer-encoding"];

/// **Scan** · Results
///
/// One scan from get_scans: its retention time, centroided m/z and intensity values, and its details.
///
/// Fields:
/// - `index`: Position of the scan in the spectrum list, from 0. Use it with get_scan_metadata.
/// - `rt`: Retention time (minutes).
/// - `mz`: m/z values.
/// - `intensity`: Intensities, same length as mz.
/// - `level`: MS level.
/// - `precursor_mz`: Precursor m/z (MS2 and higher).
/// - `base_peak_mz`: Base peak m/z.
/// - `base_peak_intensity`: Base peak intensity.
/// - `total_ion_current`: Total ion current.
/// - `polarity`: Polarity: 1 positive, 2 negative, 0 unknown.
/// - `x`: Imaging x position. 0 in non-imaging files.
/// - `y`: Imaging y position. 0 in non-imaging files.
/// - `z`: Imaging z position. 0 in non-imaging files.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#Scan)
#[derive(Clone, Debug, PartialEq)]
pub struct Scan {
    pub index: usize,
    pub rt: f64,
    pub mz: Vec<f64>,
    pub intensity: Vec<f64>,
    pub level: u8,
    pub precursor_mz: f64,
    pub base_peak_mz: f64,
    pub base_peak_intensity: f64,
    pub total_ion_current: f64,
    pub polarity: u8,
    pub x: u32,
    pub y: u32,
    pub z: u32,
    arrays: BTreeMap<String, Vec<f64>>,
}

/// **ScanParamKind** · Enums
///
/// What a row from get_scan_metadata holds.
///
/// Fields:
/// - `ATTRIBUTE`: An attribute of the tag, such as id or defaultArrayLength.
/// - `CV`: A cvParam: a param from the controlled vocabulary, with an accession.
/// - `USER`: A userParam: a param the software that wrote the file named itself.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#ScanParamKind)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::upper_case_acronyms)]
pub enum ScanParamKind {
    ATTRIBUTE,
    CV,
    USER,
}

impl ScanParamKind {
    pub(crate) fn as_text(self) -> &'static str {
        match self {
            Self::ATTRIBUTE => "ATTRIBUTE",
            Self::CV => "CV",
            Self::USER => "USER",
        }
    }
}

/// **ScanParam** · Results
///
/// One row from get_scan_metadata: an attribute, cvParam or userParam of one tag in a spectrum.
///
/// Fields:
/// - `path`: Where the tag sits, from spectrum down. Repeated tags carry their position from 0, such as spectrum/precursorList/precursor\[0\]/selectedIonList/selectedIon\[0\].
/// - `kind`: What the row holds: ATTRIBUTE, CV or USER. See ScanParamKind.
/// - `accession`: Accession of a cvParam, such as MS:1000511. Missing for attributes and userParams. Optional.
/// - `name`: Name of the attribute or param, such as ms level.
/// - `value`: Value as text, such as 2. Missing when the file gives none. Optional.
/// - `unit`: Unit name, such as minute. Missing when there is none. Optional.
/// - `unit_accession`: Unit accession, such as UO:0000031. Missing when there is none. Optional.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#ScanParam)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanParam {
    pub path: String,
    pub kind: ScanParamKind,
    pub accession: Option<String>,
    pub name: String,
    pub value: Option<String>,
    pub unit: Option<String>,
    pub unit_accession: Option<String>,
}

impl Index<&str> for Scan {
    type Output = Vec<f64>;

    fn index(&self, key: &str) -> &Vec<f64> {
        match key {
            MZ_KEY => &self.mz,
            INTENSITY_KEY => &self.intensity,
            _ => &self.arrays[key],
        }
    }
}

impl From<CentroidScan> for Scan {
    fn from(scan: CentroidScan) -> Self {
        let details = scan.metadata;
        Self {
            index: scan.index,
            rt: scan.rt,
            mz: scan.mz.to_vec(),
            intensity: scan.intensity.to_vec(),
            level: details.ms_level,
            precursor_mz: details.selected_ion_mz,
            base_peak_mz: details.base_peak_mz,
            base_peak_intensity: details.base_peak_int,
            total_ion_current: details.total_ion_current,
            polarity: details.polarity,
            x: details.position_x,
            y: details.position_y,
            z: details.position_z,
            arrays: BTreeMap::new(),
        }
    }
}

pub(crate) fn read_scans_with_arrays(
    file: &mut SampleFile,
    query: ScanQuery,
    options: &GetScansOptions,
    accessions: &[u32],
) -> Vec<ScanArrays> {
    read_scan_arrays(file, query, options.time_unit, options.level, accessions)
}

fn get_keyed_scan(found: ScanArrays, keys: &[ScanArray]) -> Scan {
    let mut scan = Scan::from(found.scan);
    for (array, values) in keys.iter().zip(found.arrays) {
        match array.key.as_str() {
            MZ_KEY => scan.mz = values,
            INTENSITY_KEY => scan.intensity = values,
            _ => {
                scan.arrays.insert(array.key.clone(), values);
            }
        }
    }
    scan
}

pub(crate) fn read_error(function: &'static str, error: impl Display) -> QuantionError {
    QuantionError::new(
        ErrorKind::ParseFailed,
        function,
        format!("the file could not be read: {error}"),
    )
}

pub(crate) fn from_fast_error(function: &'static str, error: FastError) -> QuantionError {
    match error {
        FastError::InvalidRequest => QuantionError::new(
            ErrorKind::InvalidArgument,
            function,
            "the request is not valid",
        ),
        other => read_error(function, other),
    }
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

/// **parse_ion** · Files
///
/// Open an Ionic file (.ion) from a path, a URL or bytes. Paths and URLs are read lazily: only the bytes a query needs are fetched.
///
/// **Warning:** The sample is freed when it goes out of scope, so nothing else is needed in normal use. When processing many samples, drop each one after using it, with drop(file), so its memory is freed right away instead of piling up.
///
/// - `input`: Where the file is: a path, an http(s) URL, or raw bytes.
/// - `options`: Optional settings: `max_cache_size` = 0. See [ParseIonOptions](https://phenological.github.io/quantion/?language=rust#ParseIonOptions).
///
/// Returns: The opened sample.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#parse_ion)
pub fn parse_ion(
    input: impl Into<IonInput>,
    options: ParseIonOptions,
) -> Result<SampleFile, QuantionError> {
    const FUNCTION: &str = "parse_ion";
    let input = input.into();
    if let IonInput::Path(path) = &input
        && !path.exists()
    {
        return Err(QuantionError::new(
            ErrorKind::FileNotFound,
            FUNCTION,
            format!("{} does not exist", path.display()),
        ));
    }
    let is_remote = matches!(input, IonInput::Url(_));
    let read_options = ReadOptions {
        max_cached_bytes: options.max_cache_size,
        ..Default::default()
    };
    let (reader, remote) = parse_ion_with_prefetch(input, &read_options)
        .map_err(|error| read_error(FUNCTION, error))?;
    let source = if is_remote {
        FileSource::Remote(Box::new(reader))
    } else {
        FileSource::Lazy(Box::new(reader))
    };
    Ok(SampleFile::with_remote(source, remote))
}

/// **parse_mzml** · Files
///
/// Open an mzML file from a path or bytes. It is always loaded fully into memory. URLs are refused.
///
/// **Warning:** The sample is freed when it goes out of scope, so nothing else is needed in normal use. When processing many samples, drop each one after using it, with drop(file), so its memory is freed right away instead of piling up.
///
/// - `input`: Where the file is: a path or raw bytes.
///
/// Returns: The loaded sample.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#parse_mzml)
pub fn parse_mzml(input: impl Into<MzmlInput>) -> Result<SampleFile, QuantionError> {
    const FUNCTION: &str = "parse_mzml";
    let bytes = match input.into() {
        MzmlInput::Path(path) if is_url_path(&path) => {
            return Err(QuantionError::remote_file(FUNCTION));
        }
        MzmlInput::Path(path) => fs::read(&path).map_err(|_| {
            QuantionError::new(
                ErrorKind::FileNotFound,
                FUNCTION,
                format!("{} could not be opened", path.display()),
            )
        })?,
        MzmlInput::Bytes(bytes) => bytes,
    };
    let mzml = read_mzml(&bytes).map_err(|error| read_error(FUNCTION, error))?;
    Ok(SampleFile::new(FileSource::Full(Box::new(mzml))))
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

/// **write_ion** · Files
///
/// Write a sample as an Ionic file (.ion). With no out, it returns the bytes. A path writes a file; a URL uploads it in one PUT request.
///
/// - `file`: The sample to write.
/// - `options`: Optional settings: `out` = None, `overwrite` = false, `level` = 12, `f32_compress` = false, `block_size` = 8, `mz_window` = 20, `headers` = empty. See [WriteIonOptions](https://phenological.github.io/quantion/?language=rust#WriteIonOptions).
///
/// Returns: The bytes when out is left out; nothing when it is written or uploaded.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#write_ion)
pub fn write_ion(
    file: &mut SampleFile,
    options: WriteIonOptions,
) -> Result<Option<Vec<u8>>, QuantionError> {
    const FUNCTION: &str = "write_ion";
    let bytes = encode_ion(file, &options)?;
    match options.out {
        None => Ok(Some(bytes)),
        Some(IonOutput::Path(path)) => {
            write_file(FUNCTION, &path, &bytes)?;
            Ok(None)
        }
        Some(IonOutput::Url(url)) => {
            upload(FUNCTION, &url, &options.headers, bytes)?;
            Ok(None)
        }
    }
}

const BYTES_PER_MEGABYTE: usize = 1024 * 1024;

fn get_block_bytes(function: &'static str, block_size: u32) -> Result<usize, QuantionError> {
    let bytes = usize::try_from(block_size)
        .ok()
        .and_then(|size| size.checked_mul(BYTES_PER_MEGABYTE));
    match bytes {
        Some(bytes) if bytes > 0 => Ok(bytes),
        Some(_) => Err(QuantionError::invalid(
            function,
            "block_size",
            "must be at least 1 MB, got 0",
        )),
        None => Err(QuantionError::invalid(
            function,
            "block_size",
            format!(
                "must be at most {} MB, got {block_size}",
                usize::MAX / BYTES_PER_MEGABYTE
            ),
        )),
    }
}

pub(crate) fn encode_ion(
    file: &mut SampleFile,
    options: &WriteIonOptions,
) -> Result<Vec<u8>, QuantionError> {
    const FUNCTION: &str = "write_ion";
    if options.level > MAX_COMPRESSION_LEVEL {
        return Err(QuantionError::invalid(
            FUNCTION,
            "level",
            format!(
                "must be between 0 and {MAX_COMPRESSION_LEVEL}, got {}",
                options.level
            ),
        ));
    }
    let block_bytes = get_block_bytes(FUNCTION, options.block_size)?;
    let is_upload = matches!(options.out, Some(IonOutput::Url(_)));
    if !options.headers.is_empty() && !is_upload {
        return Err(QuantionError::invalid(
            FUNCTION,
            "headers",
            "can only be used when out is a URL",
        ));
    }
    check_headers(FUNCTION, &options.headers)?;
    if let Some(IonOutput::Path(path)) = &options.out {
        check_can_write(FUNCTION, path, options.overwrite)?;
    }
    file.with_mzml(FUNCTION, |mzml| {
        let mut bytes = Vec::new();
        let write_options = WriteOptions {
            compression_level: options.level,
            force_f32: options.f32_compress,
            block_size: block_bytes,
            mz_window: f64::from(options.mz_window),
            ..Default::default()
        };
        write_mzml_to_ion(mzml, write_options, &mut bytes).map_err(|error| {
            QuantionError::internal(
                FUNCTION,
                format!("the .ion file could not be written: {error}"),
            )
        })?;
        Ok(bytes)
    })
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

/// **write_mzml** · Files
///
/// Write a sample as mzML. With no out, it returns the bytes. Only a path is allowed, no URL.
///
/// - `file`: The sample to write.
/// - `options`: Optional settings: `out` = None, `overwrite` = false. See [WriteMzmlOptions](https://phenological.github.io/quantion/?language=rust#WriteMzmlOptions).
///
/// Returns: The bytes when out is left out; nothing when it is written.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#write_mzml)
pub fn write_mzml(
    file: &mut SampleFile,
    options: WriteMzmlOptions,
) -> Result<Option<Vec<u8>>, QuantionError> {
    const FUNCTION: &str = "write_mzml";
    if let Some(path) = &options.out {
        if is_url_path(path) {
            return Err(QuantionError::invalid(
                FUNCTION,
                "out",
                "must be a path; mzML cannot be written to a URL",
            ));
        }
        check_can_write(FUNCTION, path, options.overwrite)?;
    }
    let bytes = file.with_mzml(FUNCTION, |mzml| {
        bin_to_mzml(mzml).map_err(|error| {
            QuantionError::internal(FUNCTION, format!("the mzML could not be written: {error}"))
        })
    })?;
    match options.out {
        None => Ok(Some(bytes)),
        Some(path) => {
            write_file(FUNCTION, &path, &bytes)?;
            Ok(None)
        }
    }
}

fn check_headers(function: &'static str, headers: &[Header]) -> Result<(), QuantionError> {
    for (index, header) in headers.iter().enumerate() {
        let name = header.name.trim();
        if name.is_empty() {
            return Err(QuantionError::invalid(
                function,
                format!("headers[{index}].name"),
                "must not be empty",
            ));
        }
        let has_line_break = |text: &str| text.contains('\r') || text.contains('\n');
        if has_line_break(&header.name) || has_line_break(&header.value) {
            return Err(QuantionError::invalid(
                function,
                format!("headers[{index}]"),
                "must not contain a line break",
            ));
        }
        if RESERVED_HEADERS.contains(&name.to_ascii_lowercase().as_str()) {
            return Err(QuantionError::invalid(
                function,
                format!("headers[{index}].name"),
                format!("must not be {name}; quantion sets it"),
            ));
        }
    }
    Ok(())
}

fn check_can_write(
    function: &'static str,
    path: &Path,
    overwrite: bool,
) -> Result<(), QuantionError> {
    if path.exists() && !overwrite {
        return Err(QuantionError {
            kind: ErrorKind::FileExists,
            function,
            argument: Some("out".to_string()),
            message: format!(
                "{} already exists; set overwrite to replace it",
                path.display()
            ),
        });
    }
    Ok(())
}

fn write_file(function: &'static str, path: &Path, bytes: &[u8]) -> Result<(), QuantionError> {
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".part");
    let could_not_write = |error: std::io::Error| {
        QuantionError::invalid(
            function,
            "out",
            format!("could not be written ({}): {error}", path.display()),
        )
    };
    fs::write(&temporary, bytes).map_err(could_not_write)?;
    fs::rename(&temporary, path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        could_not_write(error)
    })
}

#[cfg(all(
    feature = "http",
    not(all(target_arch = "wasm32", not(target_os = "wasi")))
))]
fn upload(
    function: &'static str,
    url: &str,
    headers: &[Header],
    bytes: Vec<u8>,
) -> Result<(), QuantionError> {
    let agent = ureq::Agent::new_with_defaults();
    let mut request = agent.put(url);
    for header in headers {
        request = request.header(header.name.trim(), header.value.as_str());
    }
    match request.send(&bytes[..]) {
        Ok(_) => Ok(()),
        Err(ureq::Error::StatusCode(status)) => Err(QuantionError::new(
            ErrorKind::UploadFailed,
            function,
            format!("PUT returned {status}"),
        )),
        Err(error) => Err(QuantionError::new(
            ErrorKind::UploadFailed,
            function,
            format!("PUT failed: {error}"),
        )),
    }
}

#[cfg(not(all(
    feature = "http",
    not(all(target_arch = "wasm32", not(target_os = "wasi")))
)))]
fn upload(
    function: &'static str,
    _url: &str,
    _headers: &[Header],
    _bytes: Vec<u8>,
) -> Result<(), QuantionError> {
    Err(QuantionError::invalid(
        function,
        "out",
        "cannot be a URL: this build of quantion has no HTTP support",
    ))
}

/// **get_scans** · Files
///
/// Read scans by retention time or precursor m/z. A range returns every scan in it; one number returns the closest scan, as a list of one.
///
/// - `file`: The sample to read, from parse_ion or parse_mzml.
/// - `query`: Which scans, by retention time: a range or one value. Each ScanQuery has `RtRange(FromTo)`, `Rt(f64)`, `MzRange(FromTo)`, `Mz(f64)`. See [ScanQuery](https://phenological.github.io/quantion/?language=rust#ScanQuery).
///   Example: `let query = ScanQuery { rt: Position::Range(FromTo { from: 1.0, to: 2.5 }), ..Default::default() };`
/// - `query`: Or by precursor m/z: a range or one value. Give rt or mz, not both. Each ScanQuery has `RtRange(FromTo)`, `Rt(f64)`, `MzRange(FromTo)`, `Mz(f64)`. See [ScanQuery](https://phenological.github.io/quantion/?language=rust#ScanQuery).
///   Example: `let query = ScanQuery { rt: Position::Range(FromTo { from: 1.0, to: 2.5 }), ..Default::default() };`
/// - `options`: Optional settings: `level` = 1, `time_unit` = MINUTES. See [GetScansOptions](https://phenological.github.io/quantion/?language=rust#GetScansOptions).
///
/// Returns: The scans found, in rt order.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#get_scans)
pub(crate) fn check_scan_query(
    function: &'static str,
    query: ScanQuery,
) -> Result<(), QuantionError> {
    if query.rt == Position::Any && query.mz == Position::Any {
        return Err(QuantionError::invalid(
            function,
            "rt",
            "or mz must be given",
        ));
    }
    check_position(function, "rt", query.rt, check_finite)?;
    check_position(function, "mz", query.mz, check_positive)
}

fn check_position(
    function: &'static str,
    argument: &str,
    position: Position,
    check_value: fn(&'static str, &str, f64) -> Result<(), QuantionError>,
) -> Result<(), QuantionError> {
    match position {
        Position::Any => Ok(()),
        Position::Closest(value) => check_value(function, argument, value),
        Position::Range(range) => {
            check_finite(function, "from", range.from)?;
            if range.to.is_nan() || range.to < range.from {
                return Err(QuantionError::invalid(
                    function,
                    "to",
                    format!("must be at least from ({}), got {}", range.from, range.to),
                ));
            }
            Ok(())
        }
    }
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
        ScanArray {
            key: MZ_KEY.to_string(),
            accession: MZ_ACCESSION.to_string(),
        },
        ScanArray {
            key: INTENSITY_KEY.to_string(),
            accession: INTENSITY_ACCESSION.to_string(),
        },
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
            return Err(QuantionError::invalid(
                function,
                "arrays",
                "must have at least one array",
            ));
        }
        let mut accessions = Vec::with_capacity(self.arrays.len());
        for (position, array) in self.arrays.iter().enumerate() {
            let number = check_scan_array(function, array)?;
            if self.arrays[..position]
                .iter()
                .any(|other| other.key == array.key)
            {
                return Err(QuantionError::invalid(
                    function,
                    "arrays.key",
                    format!("repeats {:?}", array.key),
                ));
            }
            accessions.push(number);
        }
        Ok(accessions)
    }
}

fn check_scan_array(function: &'static str, array: &ScanArray) -> Result<u32, QuantionError> {
    if array.key.is_empty() {
        return Err(QuantionError::invalid(
            function,
            "arrays.key",
            "must not be empty",
        ));
    }
    if RESERVED_SCAN_KEYS.contains(&array.key.as_str()) {
        return Err(QuantionError::invalid(
            function,
            "arrays.key",
            format!("cannot be {:?}: a scan already has that field", array.key),
        ));
    }
    let Some(number) = read_accession_number(&array.accession) else {
        return Err(QuantionError::invalid(
            function,
            "arrays.accession",
            format!("must look like MS:1000514, got {:?}", array.accession),
        ));
    };
    let pairs = [(MZ_KEY, MZ_ACCESSION), (INTENSITY_KEY, INTENSITY_ACCESSION)];
    for (key, accession) in pairs {
        if (array.key == key) != (array.accession == accession) {
            return Err(QuantionError::invalid(
                function,
                "arrays",
                format!("{key} and {accession} must go together"),
            ));
        }
    }
    Ok(number)
}

/// **get_scans** · Files
///
/// Read scans by retention time, by precursor m/z, or by both. Ranges return every scan inside them; when rt or mz is one value, you get the closest scan, as a list of one.
///
/// - `file`: The sample to read, from parse_ion or parse_mzml.
/// - `query.rt`: Which scans by retention time: a range or one value. Optional when mz is given. Each Position has `Any`, `Range(FromTo)`, `Closest(f64)`. See [Position](https://phenological.github.io/quantion/?language=rust#Position).
///   Example: `let window = Position::Range(FromTo { from: 2.3, to: 2.4 });`
/// - `query.mz`: Which scans by precursor m/z: a range or one value. Give rt, mz or both. Each Position has `Any`, `Range(FromTo)`, `Closest(f64)`. See [Position](https://phenological.github.io/quantion/?language=rust#Position).
///   Example: `let window = Position::Range(FromTo { from: 2.3, to: 2.4 });`
/// - `options`: Optional settings: `level` = 1, `time_unit` = MINUTES, `arrays` = { key: "mz", accession: "MS:1000514" }, { key: "intensity", accession: "MS:1000515" }. See [GetScansOptions](https://phenological.github.io/quantion/?language=rust#GetScansOptions).
///
/// Returns: The scans found, in rt order.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#get_scans)
pub fn get_scans(
    file: &mut SampleFile,
    query: ScanQuery,
    options: GetScansOptions,
) -> Result<Vec<Scan>, QuantionError> {
    const FUNCTION: &str = "get_scans";
    check_scan_query(FUNCTION, query)?;
    let accessions = options.check_arrays(FUNCTION)?;
    if !options.has_default_arrays() {
        let found = read_scans_with_arrays(file, query, &options, &accessions);
        return Ok(found
            .into_iter()
            .map(|scan| get_keyed_scan(scan, &options.arrays))
            .collect());
    }
    let scans = read_scans(file, query, options.time_unit, options.level);
    Ok(scans.into_iter().map(Scan::from).collect())
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
                return Err(QuantionError::invalid(
                    function,
                    "tags",
                    format!(
                        "must be one of {}, got {tag:?}",
                        SCAN_METADATA_TAGS.join(", ")
                    ),
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn keeps(&self, path: &str) -> bool {
        self.tags.is_empty() || self.tags.iter().any(|tag| tag == get_tag(path))
    }
}

/// **get_scan_metadata** · Metadata
///
/// Read the attributes, cvParams and userParams of one scan, one row each, parents before children. Find the scan with get_scans, then pass its index.
///
/// - `file`: The sample to read, from parse_ion or parse_mzml.
/// - `index`: Position of the scan in the spectrum list, from 0: the index of a scan from get_scans.
/// - `options`: Optional settings: `tags` = []. See [GetScanMetadataOptions](https://phenological.github.io/quantion/?language=rust#GetScanMetadataOptions).
///
/// Returns: One row per attribute or param, in the order of the mzML tree. Values are text. A field the file does not give is missing: None in Rust and Python, null in JS, NA in R.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#get_scan_metadata)
pub fn get_scan_metadata(
    file: &mut SampleFile,
    index: usize,
    options: GetScanMetadataOptions,
) -> Result<Vec<ScanParam>, QuantionError> {
    options.check("get_scan_metadata")?;
    let found = match &mut file.source {
        FileSource::Full(mzml) => mzml.load_metadata(index),
        FileSource::Lazy(ion) | FileSource::Remote(ion) => ion.load_metadata(index),
    };
    let rows = found.map_err(|error| scan_metadata_error(index, error))?;
    Ok(rows
        .into_iter()
        .filter(|row| options.keeps(&row.path))
        .collect())
}

pub(crate) fn scan_metadata_error(index: usize, error: IonError) -> QuantionError {
    const FUNCTION: &str = "get_scan_metadata";
    match error {
        IonError::OutOfRange { count, .. } => QuantionError::invalid(
            FUNCTION,
            "index",
            format!("must be less than the scan count ({count}), got {index}"),
        ),
        other => read_error(FUNCTION, other),
    }
}

/// **calculate_eic** · Files
///
/// Extract the ion chromatogram of one m/z.
///
/// - `file`: The sample to read, from parse_ion or parse_mzml.
/// - `mz`: The m/z to extract (Da).
/// - `options`: Optional settings: `from` = 0.0, `to` = f64::INFINITY, `ppm_tolerance` = 20.0, `mz_tolerance` = 0.005, `time_unit` = MINUTES. See [CalculateEicOptions](https://phenological.github.io/quantion/?language=rust#CalculateEicOptions).
///
/// Returns: The chromatogram: x = retention times, y = summed intensities.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#calculate_eic)
pub(crate) fn check_eic_request(
    function: &'static str,
    mz: f64,
    options: &CalculateEicOptions,
) -> Result<(), QuantionError> {
    check_positive(function, "mz", mz)?;
    options.check(function, "")
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
        check_tolerances(function, prefix, self.ppm_tolerance, self.mz_tolerance)
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

/// **calculate_eic** · Files
///
/// Extract the ion chromatogram of one m/z.
///
/// - `file`: The sample to read, from parse_ion or parse_mzml.
/// - `mz`: The m/z to extract (Da).
/// - `options`: Optional settings: `from` = 0.0, `to` = f64::INFINITY, `ppm_tolerance` = 20.0, `mz_tolerance` = 0.005, `time_unit` = MINUTES. See [CalculateEicOptions](https://phenological.github.io/quantion/?language=rust#CalculateEicOptions).
///
/// Returns: The chromatogram: x = retention times, y = summed intensities.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#calculate_eic)
pub fn calculate_eic(
    file: &mut SampleFile,
    mz: f64,
    options: CalculateEicOptions,
) -> Result<DataXY, QuantionError> {
    const FUNCTION: &str = "calculate_eic";
    check_eic_request(FUNCTION, mz, &options)?;
    read_eic(
        &mut file.reader(),
        mz,
        options.get_window(),
        options.in_minutes(),
    )
    .map_err(|error| from_fast_error(FUNCTION, error))
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

/// **get_ion_image** · Files
///
/// Build an imaging map of one m/z: summed intensity at each x, y position.
///
/// **Warning:** Only Ionic files (.ion). The sample must come from parse_ion; a sample from parse_mzml gives an invalid_argument error.
///
/// - `file`: The sample to read. Must come from parse_ion.
/// - `mz`: The m/z to map (Da).
/// - `options`: Optional settings: `tolerance` = 0.125, `level` = 1, `on_progress` = None. See [GetIonImageOptions](https://phenological.github.io/quantion/?language=rust#GetIonImageOptions).
///
/// Returns: The image.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#get_ion_image)
pub fn get_ion_image(
    file: &mut SampleFile,
    mz: f64,
    options: GetIonImageOptions,
) -> Result<IonImage, QuantionError> {
    const FUNCTION: &str = "get_ion_image";
    if !file.is_ion() {
        return Err(QuantionError::invalid(
            FUNCTION,
            "file",
            "must be an .ion file from parse_ion; an mzML sample cannot be imaged",
        ));
    }
    check_positive(FUNCTION, "mz", mz)?;
    check_not_negative(FUNCTION, "tolerance", options.tolerance)?;
    let mut reader = file.reader();
    let mut session = image_session_begin(&mut reader, mz, options.tolerance, options.level);
    let total = session.scan_count();
    let mut done = 0;
    while done < total {
        let count = IMAGE_SCANS_PER_STEP.min(total - done);
        session.fold(&mut reader, done, count);
        done += count;
        if let Some(on_progress) = options.on_progress {
            on_progress(done as u64, total as u64, 0);
        }
    }
    Ok(session.finish())
}
