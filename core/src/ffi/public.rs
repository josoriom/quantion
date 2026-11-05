use core::ffi::c_int;
use std::{
    cell::RefCell,
    panic::{AssertUnwindSafe, catch_unwind},
    slice,
};

use ionic::ByteRangesOptions;

use super::{Buf, CPeakOptions, ParsedFile, build_features_bridge, build_scan_arrays_bridge, build_fit_bridge, build_image_bridge, build_peaks_bridge, build_scans_bridge, clear_buf, f64_to_u8, mark_file_broken, pack_byte_ranges, read_ids, write_buf};
use crate::{
    DataXY, CalculateEicOptions, ErrorKind, FindFeaturesOptions, FindPeaksOptions, FindTargetsOptions, FindTransitionsOptions,
    FitPeakOptions, GetPeakOptions, GetScanMetadataOptions, GetScansOptions, GridOptions, Header, GetIonImageOptions, PeakParameters, PeakShape,
    QuantionError, SampleFile, ScanParam, ScanQuery, Target, TimeUnit, Transition, WriteIonOptions, WriteMzmlOptions, FromTo,
    files::{from_fast_error, scan_metadata_error},
    options::{CalculateBaselineOptions, DrawPeaksOptions, IonOutput},
    sample_file::FileSource,
    utilities::{
        bridge::{
            Column, QUANTION_PAYLOAD_SCAN_METADATA, QUANTION_SECTION_SCAN_METADATA_ACCESSION_BYTES,
            QUANTION_SECTION_SCAN_METADATA_ACCESSION_MISSING, QUANTION_SECTION_SCAN_METADATA_ACCESSION_STARTS,
            QUANTION_SECTION_SCAN_METADATA_KIND_BYTES, QUANTION_SECTION_SCAN_METADATA_KIND_STARTS,
            QUANTION_SECTION_SCAN_METADATA_NAME_BYTES, QUANTION_SECTION_SCAN_METADATA_NAME_STARTS,
            QUANTION_SECTION_SCAN_METADATA_PATH_BYTES, QUANTION_SECTION_SCAN_METADATA_PATH_STARTS,
            QUANTION_SECTION_SCAN_METADATA_UNIT_ACCESSION_BYTES, QUANTION_SECTION_SCAN_METADATA_UNIT_ACCESSION_MISSING,
            QUANTION_SECTION_SCAN_METADATA_UNIT_ACCESSION_STARTS, QUANTION_SECTION_SCAN_METADATA_UNIT_BYTES,
            QUANTION_SECTION_SCAN_METADATA_UNIT_MISSING, QUANTION_SECTION_SCAN_METADATA_UNIT_STARTS,
            QUANTION_SECTION_SCAN_METADATA_VALUE_BYTES, QUANTION_SECTION_SCAN_METADATA_VALUE_MISSING,
            QUANTION_SECTION_SCAN_METADATA_VALUE_STARTS, build_record_bridge,
        },
        calculate_eic::plan_eic_ranges,
        find_features::{FeatureSettings, plan_features_ranges},
        find_targets::plan_targets_ranges,
    },
};

const FIELD_SEPARATOR: char = '\u{1f}';

thread_local! {
    static LAST_ERROR: RefCell<Option<QuantionError>> = const { RefCell::new(None) };
}

pub(super) fn remember(error: QuantionError) -> c_int {
    let code = error.kind.code() as c_int;
    LAST_ERROR.with(|last| *last.borrow_mut() = Some(error));
    code
}

pub(super) fn fail(code: c_int, function: &'static str) -> c_int {
    let error = match code {
        1 => QuantionError::new(ErrorKind::InvalidArgument, function, "the arguments are not valid"),
        4 => QuantionError::remote_file(function),
        6 => QuantionError::new(ErrorKind::ParseFailed, function, "the file could not be read"),
        _ => QuantionError::internal(function, "the call failed"),
    };
    remember(error)
}

pub(super) fn run(function: &'static str, work: impl FnOnce() -> Result<(), QuantionError>) -> c_int {
    match catch_unwind(AssertUnwindSafe(work)) {
        Ok(Ok(())) => 0,
        Ok(Err(error)) => remember(error),
        Err(_) => remember(QuantionError::internal(function, "a panic happened")),
    }
}

fn run_on_file(
    function: &'static str,
    handle: *mut ParsedFile,
    work: impl FnOnce(&mut SampleFile) -> Result<(), QuantionError>,
) -> c_int {
    if handle.is_null() {
        return remember(QuantionError::invalid(function, "file", "is not open; it was disposed or never opened"));
    }
    if unsafe { (*handle).is_broken() } {
        return remember(QuantionError::internal(function, "the file is unusable after an earlier panic; open it again"));
    }
    match catch_unwind(AssertUnwindSafe(|| work(unsafe { &mut (*handle).file }))) {
        Ok(Ok(())) => 0,
        Ok(Err(error)) => remember(error),
        Err(_) => {
            mark_file_broken(handle);
            remember(QuantionError::internal(function, "a panic happened"))
        }
    }
}

fn read_numbers<'a>(pointer: *const f64, count: usize) -> &'a [f64] {
    if pointer.is_null() || count == 0 {
        &[]
    } else {
        unsafe { slice::from_raw_parts(pointer, count) }
    }
}

fn read_trace(x: *const f64, y: *const f64, count: usize) -> DataXY {
    DataXY {
        x: read_numbers(x, count).to_vec(),
        y: read_numbers(y, count).to_vec(),
    }
}

fn read_text(function: &'static str, argument: &str, pointer: *const core::ffi::c_char) -> Result<Option<String>, QuantionError> {
    if pointer.is_null() {
        return Ok(None);
    }
    unsafe { core::ffi::CStr::from_ptr(pointer) }
        .to_str()
        .map(|text| Some(text.to_string()))
        .map_err(|_| QuantionError::invalid(function, argument, "must be valid UTF-8 text"))
}

pub(super) fn get_time_unit(code: u8) -> TimeUnit {
    if code == 1 { TimeUnit::SECONDS } else { TimeUnit::MINUTES }
}

fn get_shape(code: c_int) -> PeakShape {
    if code == 0 { PeakShape::GAUSSIAN } else { PeakShape::EMG }
}

pub(super) fn get_peak_options(options: *const CPeakOptions) -> FindPeaksOptions {
    let defaults = FindPeaksOptions::default();
    if options.is_null() {
        return defaults;
    }
    let given = unsafe { *options };
    FindPeaksOptions {
        min_integral: given.min_integral.is_finite().then_some(given.min_integral),
        min_intensity: if given.min_intensity.is_finite() { given.min_intensity } else { defaults.min_intensity },
        min_peak_width_points: usize::try_from(given.min_peak_width_points).unwrap_or(defaults.min_peak_width_points),
        noise: given.noise.is_finite().then_some(given.noise),
        auto_noise: given.auto_noise != 0,
        auto_baseline: given.auto_baseline != 0,
        lambda: if given.lambda.is_finite() && given.lambda > 0.0 { given.lambda } else { defaults.lambda },
        max_iterations: usize::try_from(given.max_iterations).ok().filter(|count| *count > 0).unwrap_or(defaults.max_iterations),
        allow_overlap: given.allow_overlap != 0,
        min_snr: if given.min_snr.is_finite() { given.min_snr } else { defaults.min_snr },
        min_r2: if given.min_r2.is_finite() { given.min_r2 } else { defaults.min_r2 },
        shape: get_shape(given.shape),
        kernel_size: usize::try_from(given.kernel_size).unwrap_or(0),
    }
}

fn get_eic_options(from: f64, to: f64, ppm_tolerance: f64, mz_tolerance: f64, time_unit: u8) -> CalculateEicOptions {
    CalculateEicOptions {
        from,
        to,
        ppm_tolerance,
        mz_tolerance,
        time_unit: get_time_unit(time_unit),
    }
}

fn get_threads(threads: usize) -> usize {
    threads.max(1)
}

fn write_ranges(out: *mut Buf, ranges: &[ionic::source::ByteRange]) {
    write_buf(out, pack_byte_ranges(ranges.iter().map(|range| (range.offset, range.length))));
}

/// Write the error of the last failed call on this thread to `out` as UTF-8 text:
/// code, kind, function, argument and message, separated by the unit separator (0x1F).
///
/// # Safety
/// `out` must be a valid writable `Buf` pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quantion_last_error(out: *mut Buf) -> c_int {
    if out.is_null() {
        return ErrorKind::InvalidArgument.code() as c_int;
    }
    clear_buf(out);
    let text = LAST_ERROR.with(|last| match &*last.borrow() {
        Some(error) => [
            error.kind.code().to_string(),
            error.kind.name().to_string(),
            error.function.to_string(),
            error.argument.clone().unwrap_or_default(),
            error.message.clone(),
        ]
        .join(&FIELD_SEPARATOR.to_string()),
        None => String::new(),
    });
    write_buf(out, text.into_bytes().into_boxed_slice());
    0
}

/// Open an mzML file from a path.
///
/// # Safety
/// `path` must be a valid NUL-terminated string. `dest` must be a valid writable pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_mzml_path(path: *const core::ffi::c_char, dest: *mut *mut ParsedFile) -> c_int {
    const FUNCTION: &str = "parse_mzml";
    if dest.is_null() {
        return remember(QuantionError::internal(FUNCTION, "the output handle is null"));
    }
    run(FUNCTION, || {
        let path = read_text(FUNCTION, "source", path)?.ok_or_else(|| QuantionError::missing(FUNCTION, "source"))?;
        let file = crate::parse_mzml(path)?;
        unsafe { *dest = Box::into_raw(Box::new(ParsedFile::from_file(file))) };
        Ok(())
    })
}

/// Write a sample as .ion. A null `out_path` or a URL returns the bytes in `out`; a path writes a file.
/// For a URL the headers are checked here and the caller uploads the bytes with its own HTTP client. `headers` holds name, value, name, value… each ending in a NUL byte.
///
/// # Safety
/// `handle` must be a valid `ParsedFile`. `out_path` must be null or NUL-terminated.
/// `headers` must point to `headers_len` readable bytes. `out` must be a valid writable `Buf`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn write_ion(
    handle: *mut ParsedFile,
    out_path: *const core::ffi::c_char,
    overwrite: u8,
    level: u8,
    f32_compress: u8,
    block_size: u32,
    mz_window: u32,
    headers: *const u8,
    headers_len: usize,
    out: *mut Buf,
) -> c_int {
    const FUNCTION: &str = "write_ion";
    if out.is_null() {
        return remember(QuantionError::internal(FUNCTION, "the output buffer is null"));
    }
    clear_buf(out);
    run_on_file(FUNCTION, handle, |file| {
        let out_path = read_text(FUNCTION, "out", out_path)?;
        let options = WriteIonOptions {
            out: out_path.map(IonOutput::from),
            overwrite: overwrite != 0,
            level,
            f32_compress: f32_compress != 0,
            block_size,
            mz_window,
            headers: read_headers(FUNCTION, headers, headers_len)?,
        };
        if let Some(IonOutput::Url(_)) = options.out {
            let bytes = crate::files::encode_ion(file, &options)?;
            write_buf(out, bytes.into_boxed_slice());
            return Ok(());
        }
        if let Some(bytes) = crate::write_ion(file, options)? {
            write_buf(out, bytes.into_boxed_slice());
        }
        Ok(())
    })
}

fn read_headers(function: &'static str, pointer: *const u8, length: usize) -> Result<Vec<Header>, QuantionError> {
    if pointer.is_null() || length == 0 {
        return Ok(Vec::new());
    }
    let bytes = unsafe { slice::from_raw_parts(pointer, length) };
    let text = std::str::from_utf8(bytes).map_err(|_| QuantionError::invalid(function, "headers", "must be valid UTF-8 text"))?;
    let mut parts: Vec<&str> = text.split('\0').collect();
    if parts.last() == Some(&"") {
        parts.pop();
    }
    if parts.len() % 2 != 0 {
        return Err(QuantionError::invalid(function, "headers", "must be name and value pairs"));
    }
    Ok(parts
        .chunks(2)
        .map(|pair| Header {
            name: pair[0].to_string(),
            value: pair[1].to_string(),
        })
        .collect())
}

/// Write a sample as mzML. A null `out_path` returns the bytes in `out`.
///
/// # Safety
/// `handle` must be a valid `ParsedFile`. `out_path` must be null or NUL-terminated.
/// `out` must be a valid writable `Buf`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn write_mzml(handle: *mut ParsedFile, out_path: *const core::ffi::c_char, overwrite: u8, out: *mut Buf) -> c_int {
    const FUNCTION: &str = "write_mzml";
    if out.is_null() {
        return remember(QuantionError::internal(FUNCTION, "the output buffer is null"));
    }
    clear_buf(out);
    run_on_file(FUNCTION, handle, |file| {
        let options = WriteMzmlOptions {
            out: read_text(FUNCTION, "out", out_path)?.map(Into::into),
            overwrite: overwrite != 0,
        };
        if let Some(bytes) = crate::write_mzml(file, options)? {
            write_buf(out, bytes.into_boxed_slice());
        }
        Ok(())
    })
}

/// Find every peak in a trace.
///
/// # Safety
/// `x` and `y` must point to `count` readable values. `options` must be null or valid.
/// `out` must be a valid writable `Buf`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_peaks(x: *const f64, y: *const f64, count: usize, options: *const CPeakOptions, out: *mut Buf) -> c_int {
    const FUNCTION: &str = "find_peaks";
    if out.is_null() {
        return remember(QuantionError::internal(FUNCTION, "the output buffer is null"));
    }
    clear_buf(out);
    run(FUNCTION, || {
        let peaks = crate::find_peaks(&read_trace(x, y, count), get_peak_options(options))?;
        write_buf(out, build_peaks_bridge(&peaks).ok_or_else(|| QuantionError::internal(FUNCTION, "the peaks could not be packed"))?);
        Ok(())
    })
}

/// Find the one peak near `rt`. A `range` of 0 searches the whole trace.
///
/// # Safety
/// `x` and `y` must point to `count` readable values. `options` must be null or valid.
/// `out` must be a valid writable `Buf`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_peak(
    x: *const f64,
    y: *const f64,
    count: usize,
    rt: f64,
    range: f64,
    options: *const CPeakOptions,
    out: *mut Buf,
) -> c_int {
    const FUNCTION: &str = "get_peak";
    if out.is_null() {
        return remember(QuantionError::internal(FUNCTION, "the output buffer is null"));
    }
    clear_buf(out);
    run(FUNCTION, || {
        let options = GetPeakOptions {
            range,
            find_peaks: get_peak_options(options),
        };
        let peak = crate::get_peak(&read_trace(x, y, count), rt, options)?;
        write_buf(out, build_peaks_bridge(&[peak]).ok_or_else(|| QuantionError::internal(FUNCTION, "the peak could not be packed"))?);
        Ok(())
    })
}

/// Fit a curve to the peak at `rt`. `shape` is 0 for Gaussian, 1 for EMG.
/// An `intensity` of 0 takes it from the peak found. No fit gives an empty record.
///
/// # Safety
/// `x` and `y` must point to `count` readable values. `out` must be a valid writable `Buf`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fit_peak(x: *const f64, y: *const f64, count: usize, rt: f64, shape: c_int, intensity: f64, out: *mut Buf) -> c_int {
    const FUNCTION: &str = "fit_peak";
    if out.is_null() {
        return remember(QuantionError::internal(FUNCTION, "the output buffer is null"));
    }
    clear_buf(out);
    run(FUNCTION, || {
        let options = FitPeakOptions {
            shape: get_shape(shape),
            intensity,
        };
        let params = crate::fit_peak(&read_trace(x, y, count), rt, options)?;
        write_buf(out, build_fit_bridge(&params).ok_or_else(|| QuantionError::internal(FUNCTION, "the fit could not be packed"))?);
        Ok(())
    })
}

/// Draw peak curves into one chromatogram. `from` and `to` both 0 fit the range around the peaks.
///
/// # Safety
/// `heights`, `centers`, `fwhms` and `tails` must point to `count` readable values.
/// `out_x` and `out_y` must be valid writable `Buf` pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn draw_peaks(
    heights: *const f64,
    centers: *const f64,
    fwhms: *const f64,
    tails: *const f64,
    count: usize,
    from: f64,
    to: f64,
    points: usize,
    out_x: *mut Buf,
    out_y: *mut Buf,
) -> c_int {
    const FUNCTION: &str = "draw_peaks";
    if out_x.is_null() || out_y.is_null() {
        return remember(QuantionError::internal(FUNCTION, "the output buffer is null"));
    }
    clear_buf(out_x);
    clear_buf(out_y);
    run(FUNCTION, || {
        let (heights, centers, fwhms, tails) =
            (read_numbers(heights, count), read_numbers(centers, count), read_numbers(fwhms, count), read_numbers(tails, count));
        let params: Vec<PeakParameters> = (0..count)
            .map(|index| PeakParameters {
                height: heights[index],
                center: centers[index],
                fwhm: fwhms[index],
                tail: tails[index],
                r2: 0.0,
            })
            .collect();
        let drawn = crate::draw_peaks(&params, DrawPeaksOptions { from, to, points })?;
        write_buf(out_x, f64_to_u8(&drawn.x));
        write_buf(out_y, f64_to_u8(&drawn.y));
        Ok(())
    })
}

/// Estimate a baseline with airPLS.
///
/// # Safety
/// `y` must point to `count` readable values. `out` must be a valid writable `Buf`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn calculate_baseline(y: *const f64, count: usize, lambda: f64, max_iterations: usize, out: *mut Buf) -> c_int {
    const FUNCTION: &str = "calculate_baseline";
    if out.is_null() {
        return remember(QuantionError::internal(FUNCTION, "the output buffer is null"));
    }
    clear_buf(out);
    run(FUNCTION, || {
        let baseline = crate::calculate_baseline(read_numbers(y, count), CalculateBaselineOptions { lambda, max_iterations })?;
        write_buf(out, f64_to_u8(&baseline));
        Ok(())
    })
}

/// Extract the EIC of one m/z. `time_unit` is 0 for minutes, 1 for seconds.
///
/// # Safety
/// `handle` must be a valid `ParsedFile`. `out_x` and `out_y` must be valid writable `Buf`s.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn calculate_eic(
    handle: *mut ParsedFile,
    mz: f64,
    from: f64,
    to: f64,
    ppm_tolerance: f64,
    mz_tolerance: f64,
    time_unit: u8,
    out_x: *mut Buf,
    out_y: *mut Buf,
) -> c_int {
    const FUNCTION: &str = "calculate_eic";
    if out_x.is_null() || out_y.is_null() {
        return remember(QuantionError::internal(FUNCTION, "the output buffer is null"));
    }
    clear_buf(out_x);
    clear_buf(out_y);
    run_on_file(FUNCTION, handle, |file| {
        let options = get_eic_options(from, to, ppm_tolerance, mz_tolerance, time_unit);
        let eic = crate::calculate_eic(file, mz, options)?;
        write_buf(out_x, f64_to_u8(&eic.x));
        write_buf(out_y, f64_to_u8(&eic.y));
        Ok(())
    })
}

/// Plan the byte ranges `calculate_eic` reads from an .ion file.
///
/// # Safety
/// `handle` must be a valid `ParsedFile`. `out` must be a valid writable `Buf`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn plan_eic(
    handle: *mut ParsedFile,
    mz: f64,
    from: f64,
    to: f64,
    ppm_tolerance: f64,
    mz_tolerance: f64,
    time_unit: u8,
    out: *mut Buf,
) -> c_int {
    const FUNCTION: &str = "calculate_eic";
    if out.is_null() {
        return remember(QuantionError::internal(FUNCTION, "the output buffer is null"));
    }
    clear_buf(out);
    run_on_file(FUNCTION, handle, |file| {
        let options = get_eic_options(from, to, ppm_tolerance, mz_tolerance, time_unit);
        crate::files::check_eic_request(FUNCTION, mz, &options)?;
        let window = options.get_window();
        if let FileSource::Lazy(ion) | FileSource::Remote(ion) = &mut file.source {
            let ranges = plan_eic_ranges(ion, mz, window.from, window.to, ppm_tolerance, mz_tolerance)
                .map_err(|error| from_fast_error(FUNCTION, error))?;
            write_ranges(out, &ranges);
        }
        Ok(())
    })
}

fn get_scan_query(query_type: u8, from: f64, to: f64) -> Option<ScanQuery> {
    match query_type {
        0 => Some(ScanQuery::RtRange(FromTo { from, to })),
        1 => Some(ScanQuery::Rt(from)),
        2 => Some(ScanQuery::MzRange(FromTo { from, to })),
        3 => Some(ScanQuery::Mz(from)),
        _ => None,
    }
}

/// Read scans. `query_type`: 0 rt range, 1 closest rt, 2 m/z range, 3 closest m/z.
/// `time_unit` is 0 for minutes, 1 for seconds.
///
/// # Safety
/// `handle` must be a valid `ParsedFile`. `out` must be a valid writable `Buf`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_scans(handle: *mut ParsedFile, query_type: u8, from: f64, to: f64, level: u8, time_unit: u8, out: *mut Buf) -> c_int {
    const FUNCTION: &str = "get_scans";
    if out.is_null() {
        return remember(QuantionError::internal(FUNCTION, "the output buffer is null"));
    }
    clear_buf(out);
    run_on_file(FUNCTION, handle, |file| {
        let query = get_scan_query(query_type, from, to).ok_or_else(|| QuantionError::internal(FUNCTION, "unknown query type"))?;
        let options = GetScansOptions {
            level,
            time_unit: get_time_unit(time_unit),
            ..Default::default()
        };
        let scans = crate::get_scans(file, query, options)?;
        write_buf(out, build_scans_bridge(&scans).ok_or_else(|| QuantionError::internal(FUNCTION, "the scans could not be packed"))?);
        Ok(())
    })
}

fn read_accessions<'a>(accessions: *const u32, array_count: usize) -> Result<&'a [u32], QuantionError> {
    if accessions.is_null() || array_count == 0 {
        return Err(QuantionError::invalid("get_scans", "arrays", "must have at least one array"));
    }
    Ok(unsafe { std::slice::from_raw_parts(accessions, array_count) })
}

/// Read chosen arrays of each scan. `accessions` holds the number of each array, such as 1000786.
///
/// # Safety
/// `handle` must be a valid `ParsedFile`. `accessions` must hold `array_count` numbers. `out` must be a valid writable `Buf`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_scans_with_arrays(handle: *mut ParsedFile, query_type: u8, from: f64, to: f64, level: u8, time_unit: u8, accessions: *const u32, array_count: usize, out: *mut Buf) -> c_int {
    const FUNCTION: &str = "get_scans";
    if out.is_null() {
        return remember(QuantionError::internal(FUNCTION, "the output buffer is null"));
    }
    clear_buf(out);
    run_on_file(FUNCTION, handle, |file| {
        let query = get_scan_query(query_type, from, to).ok_or_else(|| QuantionError::internal(FUNCTION, "unknown query type"))?;
        crate::files::check_scan_query(FUNCTION, query)?;
        let accessions = read_accessions(accessions, array_count)?;
        let options = GetScansOptions { level, time_unit: get_time_unit(time_unit), ..Default::default() };
        let found = crate::files::read_scans_with_arrays(file, query, &options, accessions);
        write_buf(out, build_scan_arrays_bridge(found).ok_or_else(|| QuantionError::internal(FUNCTION, "the scans could not be packed"))?);
        Ok(())
    })
}

/// Plan the byte ranges `get_scans_with_arrays` reads from an .ion file.
///
/// # Safety
/// `handle` must be a valid `ParsedFile`. `accessions` must hold `array_count` numbers. `out` must be a valid writable `Buf`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn plan_scans_with_arrays(handle: *mut ParsedFile, query_type: u8, from: f64, to: f64, level: u8, time_unit: u8, accessions: *const u32, array_count: usize, out: *mut Buf) -> c_int {
    const FUNCTION: &str = "get_scans";
    if out.is_null() {
        return remember(QuantionError::internal(FUNCTION, "the output buffer is null"));
    }
    clear_buf(out);
    run_on_file(FUNCTION, handle, |file| {
        let query = get_scan_query(query_type, from, to).ok_or_else(|| QuantionError::internal(FUNCTION, "unknown query type"))?;
        crate::files::check_scan_query(FUNCTION, query)?;
        let arrays = ByteRangesOptions { arrays: read_accessions(accessions, array_count)?.to_vec() };
        if let FileSource::Lazy(ion) | FileSource::Remote(ion) = &mut file.source {
            let ranges = crate::utilities::calculate_eic::plan_scan_ranges(ion.as_mut(), query, get_time_unit(time_unit), level, &arrays)
                .map_err(|error| from_fast_error(FUNCTION, error))?;
            write_ranges(out, &ranges);
        }
        Ok(())
    })
}

/// Plan the byte ranges `get_scans` reads from an .ion file.
///
/// # Safety
/// `handle` must be a valid `ParsedFile`. `out` must be a valid writable `Buf`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn plan_scans(handle: *mut ParsedFile, query_type: u8, from: f64, to: f64, level: u8, time_unit: u8, out: *mut Buf) -> c_int {
    const FUNCTION: &str = "get_scans";
    if out.is_null() {
        return remember(QuantionError::internal(FUNCTION, "the output buffer is null"));
    }
    clear_buf(out);
    run_on_file(FUNCTION, handle, |file| {
        let query = get_scan_query(query_type, from, to).ok_or_else(|| QuantionError::internal(FUNCTION, "unknown query type"))?;
        crate::files::check_scan_query(FUNCTION, query)?;
        if let FileSource::Lazy(ion) | FileSource::Remote(ion) = &mut file.source {
            let ranges = crate::utilities::calculate_eic::plan_scan_ranges(ion.as_mut(), query, get_time_unit(time_unit), level, &ByteRangesOptions::default())
                .map_err(|error| from_fast_error(FUNCTION, error))?;
            write_ranges(out, &ranges);
        }
        Ok(())
    })
}

/// Read the attributes, cvParams and userParams of one scan, one row each. `tags` holds tag names split by the unit separator 0x1F; null or empty keeps every tag.
///
/// # Safety
/// `handle` must be a valid `ParsedFile`. `tags` must be null or NUL-terminated text. `out` must be a valid writable `Buf`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_scan_metadata(handle: *mut ParsedFile, index: usize, tags: *const core::ffi::c_char, out: *mut Buf) -> c_int {
    const FUNCTION: &str = "get_scan_metadata";
    if out.is_null() {
        return remember(QuantionError::internal(FUNCTION, "the output buffer is null"));
    }
    clear_buf(out);
    let tags = match read_text(FUNCTION, "tags", tags) {
        Ok(text) => text.unwrap_or_default(),
        Err(error) => return remember(error),
    };
    run_on_file(FUNCTION, handle, |file| {
        let tags = tags.split(FIELD_SEPARATOR).filter(|tag| !tag.is_empty()).map(str::to_string).collect();
        let rows = crate::get_scan_metadata(file, index, GetScanMetadataOptions { tags })?;
        let bytes = build_scan_metadata_bridge(&rows).ok_or_else(|| QuantionError::internal(FUNCTION, "the metadata could not be packed"))?;
        write_buf(out, bytes);
        Ok(())
    })
}

fn build_scan_metadata_bridge(rows: &[ScanParam]) -> Option<Box<[u8]>> {
    let paths: Vec<&str> = rows.iter().map(|row| row.path.as_str()).collect();
    let kinds: Vec<&str> = rows.iter().map(|row| row.kind.as_text()).collect();
    let accessions: Vec<Option<&str>> = rows.iter().map(|row| row.accession.as_deref()).collect();
    let names: Vec<&str> = rows.iter().map(|row| row.name.as_str()).collect();
    let values: Vec<Option<&str>> = rows.iter().map(|row| row.value.as_deref()).collect();
    let units: Vec<Option<&str>> = rows.iter().map(|row| row.unit.as_deref()).collect();
    let unit_accessions: Vec<Option<&str>> = rows.iter().map(|row| row.unit_accession.as_deref()).collect();
    build_record_bridge(
        QUANTION_PAYLOAD_SCAN_METADATA,
        rows.len() as u64,
        &[
            Column::Text { starts_id: QUANTION_SECTION_SCAN_METADATA_PATH_STARTS, bytes_id: QUANTION_SECTION_SCAN_METADATA_PATH_BYTES, values: &paths },
            Column::Text { starts_id: QUANTION_SECTION_SCAN_METADATA_KIND_STARTS, bytes_id: QUANTION_SECTION_SCAN_METADATA_KIND_BYTES, values: &kinds },
            Column::OptionalText {
                starts_id: QUANTION_SECTION_SCAN_METADATA_ACCESSION_STARTS,
                bytes_id: QUANTION_SECTION_SCAN_METADATA_ACCESSION_BYTES,
                missing_id: QUANTION_SECTION_SCAN_METADATA_ACCESSION_MISSING,
                values: &accessions,
            },
            Column::Text { starts_id: QUANTION_SECTION_SCAN_METADATA_NAME_STARTS, bytes_id: QUANTION_SECTION_SCAN_METADATA_NAME_BYTES, values: &names },
            Column::OptionalText {
                starts_id: QUANTION_SECTION_SCAN_METADATA_VALUE_STARTS,
                bytes_id: QUANTION_SECTION_SCAN_METADATA_VALUE_BYTES,
                missing_id: QUANTION_SECTION_SCAN_METADATA_VALUE_MISSING,
                values: &values,
            },
            Column::OptionalText {
                starts_id: QUANTION_SECTION_SCAN_METADATA_UNIT_STARTS,
                bytes_id: QUANTION_SECTION_SCAN_METADATA_UNIT_BYTES,
                missing_id: QUANTION_SECTION_SCAN_METADATA_UNIT_MISSING,
                values: &units,
            },
            Column::OptionalText {
                starts_id: QUANTION_SECTION_SCAN_METADATA_UNIT_ACCESSION_STARTS,
                bytes_id: QUANTION_SECTION_SCAN_METADATA_UNIT_ACCESSION_BYTES,
                missing_id: QUANTION_SECTION_SCAN_METADATA_UNIT_ACCESSION_MISSING,
                values: &unit_accessions,
            },
        ],
    )
}

/// Plan the byte ranges `get_scan_metadata` reads from an .ion file.
///
/// # Safety
/// `handle` must be a valid `ParsedFile`. `out` must be a valid writable `Buf`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn plan_scan_metadata(handle: *mut ParsedFile, index: usize, out: *mut Buf) -> c_int {
    const FUNCTION: &str = "get_scan_metadata";
    if out.is_null() {
        return remember(QuantionError::internal(FUNCTION, "the output buffer is null"));
    }
    clear_buf(out);
    run_on_file(FUNCTION, handle, |file| {
        if let FileSource::Lazy(ion) | FileSource::Remote(ion) = &mut file.source {
            let ranges = ion.spectrum_metadata_byte_ranges(index).map_err(|error| scan_metadata_error(index, error))?;
            write_ranges(out, &ranges);
        }
        Ok(())
    })
}

/// Build the ion image of one m/z. Only .ion files.
///
/// # Safety
/// `handle` must be a valid `ParsedFile`. `out` must be a valid writable `Buf`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_ion_image(handle: *mut ParsedFile, mz: f64, tolerance: f64, level: u8, out: *mut Buf) -> c_int {
    const FUNCTION: &str = "get_ion_image";
    if out.is_null() {
        return remember(QuantionError::internal(FUNCTION, "the output buffer is null"));
    }
    clear_buf(out);
    run_on_file(FUNCTION, handle, |file| {
        let options = GetIonImageOptions {
            tolerance,
            level,
            on_progress: None,
        };
        let image = crate::get_ion_image(file, mz, options)?;
        write_buf(out, build_image_bridge(&image).ok_or_else(|| QuantionError::internal(FUNCTION, "the image could not be packed"))?);
        Ok(())
    })
}

fn read_targets(
    rts: *const f64,
    mzs: *const f64,
    ranges: *const f64,
    ids: Vec<String>,
    count: usize,
) -> Vec<Target> {
    let (rts, mzs, ranges) = (read_numbers(rts, count), read_numbers(mzs, count), read_numbers(ranges, count));
    ids.into_iter()
        .enumerate()
        .map(|(index, id)| Target {
            id: (!id.is_empty()).then_some(id),
            mz: mzs[index],
            rt: rts[index],
            range: ranges.get(index).copied().filter(|range| !range.is_nan()),
        })
        .collect()
}

/// Find the peak of each target in its EIC. A NaN range means no range: the whole EIC is searched.
///
/// # Safety
/// `handle` must be a valid `ParsedFile`. `rts`, `mzs` and `ranges` must point to `count` values.
/// Ids follow the `read_ids` layout. `options` must be null or valid. `out` must be a valid writable `Buf`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_targets(
    handle: *mut ParsedFile,
    rts: *const f64,
    mzs: *const f64,
    ranges: *const f64,
    ids_offsets: *const u32,
    ids_lengths: *const u32,
    ids_text: *const u8,
    ids_text_len: usize,
    count: usize,
    from: f64,
    to: f64,
    ppm_tolerance: f64,
    mz_tolerance: f64,
    time_unit: u8,
    options: *const CPeakOptions,
    threads: usize,
    out: *mut Buf,
) -> c_int {
    const FUNCTION: &str = "find_targets";
    if out.is_null() {
        return remember(QuantionError::internal(FUNCTION, "the output buffer is null"));
    }
    clear_buf(out);
    run_on_file(FUNCTION, handle, |file| {
        let ids = read_ids(count, ids_offsets, ids_lengths, ids_text, ids_text_len);
        let targets = read_targets(rts, mzs, ranges, ids, count);
        let options = FindTargetsOptions {
            eic: get_eic_options(from, to, ppm_tolerance, mz_tolerance, time_unit),
            find_peaks: get_peak_options(options),
            threads: get_threads(threads),
        };
        let peaks = crate::find_targets(file, &targets, options)?;
        write_buf(out, build_peaks_bridge(&peaks).ok_or_else(|| QuantionError::internal(FUNCTION, "the peaks could not be packed"))?);
        Ok(())
    })
}

/// Plan the byte ranges `find_targets` reads from an .ion file.
///
/// # Safety
/// `handle` must be a valid `ParsedFile`. `mzs` must point to `count` values.
/// `out` must be a valid writable `Buf`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn plan_find_targets(
    handle: *mut ParsedFile,
    mzs: *const f64,
    count: usize,
    from: f64,
    to: f64,
    ppm_tolerance: f64,
    mz_tolerance: f64,
    time_unit: u8,
    out: *mut Buf,
) -> c_int {
    const FUNCTION: &str = "find_targets";
    if out.is_null() {
        return remember(QuantionError::internal(FUNCTION, "the output buffer is null"));
    }
    clear_buf(out);
    run_on_file(FUNCTION, handle, |file| {
        let eic = get_eic_options(from, to, ppm_tolerance, mz_tolerance, time_unit);
        eic.check(FUNCTION, "eic.")?;
        let window = eic.get_window();
        let targets: Vec<Target> = read_numbers(mzs, count)
            .iter()
            .map(|mz| Target {
                mz: *mz,
                ..Default::default()
            })
            .collect();
        crate::targets::check_targets(FUNCTION, &targets)?;
        if let FileSource::Lazy(ion) | FileSource::Remote(ion) = &mut file.source {
            let ranges = plan_targets_ranges(ion, &targets, window.from, window.to, eic.in_minutes())
                .map_err(|error| from_fast_error(FUNCTION, error))?;
            write_ranges(out, &ranges);
        }
        Ok(())
    })
}

/// Find the peak of each SRM transition. Needs the whole file.
///
/// # Safety
/// `handle` must be a valid `ParsedFile`. The number arrays must point to `count` values.
/// Ids follow the `read_ids` layout. `options` must be null or valid. `out` must be a valid writable `Buf`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_transitions(
    handle: *mut ParsedFile,
    precursors: *const f64,
    products: *const f64,
    rts: *const f64,
    ranges: *const f64,
    ids_offsets: *const u32,
    ids_lengths: *const u32,
    ids_text: *const u8,
    ids_text_len: usize,
    count: usize,
    options: *const CPeakOptions,
    threads: usize,
    out: *mut Buf,
) -> c_int {
    const FUNCTION: &str = "find_transitions";
    if out.is_null() {
        return remember(QuantionError::internal(FUNCTION, "the output buffer is null"));
    }
    clear_buf(out);
    run_on_file(FUNCTION, handle, |file| {
        let (precursors, products, rts, ranges) =
            (read_numbers(precursors, count), read_numbers(products, count), read_numbers(rts, count), read_numbers(ranges, count));
        let targets: Vec<Transition> = read_ids(count, ids_offsets, ids_lengths, ids_text, ids_text_len)
            .into_iter()
            .enumerate()
            .map(|(index, id)| Transition {
                id: (!id.is_empty()).then_some(id),
                precursor: precursors[index],
                product: products[index],
                rt: rts[index],
                range: ranges[index],
            })
            .collect();
        let options = FindTransitionsOptions {
            find_peaks: get_peak_options(options),
            threads: get_threads(threads),
        };
        let peaks = crate::find_transitions(file, &targets, options)?;
        write_buf(out, build_peaks_bridge(&peaks).ok_or_else(|| QuantionError::internal(FUNCTION, "the peaks could not be packed"))?);
        Ok(())
    })
}

fn get_feature_options(
    eic: CalculateEicOptions,
    grid_start: f64,
    grid_end: f64,
    grid_step_size: f64,
    options: *const CPeakOptions,
    threads: usize,
) -> FindFeaturesOptions {
    FindFeaturesOptions {
        eic,
        grid: GridOptions {
            start: grid_start,
            end: grid_end,
            step_size: grid_step_size,
        },
        find_peaks: get_peak_options(options),
        threads: get_threads(threads),
    }
}

/// Find every feature in one file.
///
/// # Safety
/// `handle` must be a valid `ParsedFile`. `options` must be null or valid. `out` must be a valid writable `Buf`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_features(
    handle: *mut ParsedFile,
    from: f64,
    to: f64,
    ppm_tolerance: f64,
    mz_tolerance: f64,
    time_unit: u8,
    grid_start: f64,
    grid_end: f64,
    grid_step_size: f64,
    options: *const CPeakOptions,
    threads: usize,
    out: *mut Buf,
) -> c_int {
    const FUNCTION: &str = "find_features";
    if out.is_null() {
        return remember(QuantionError::internal(FUNCTION, "the output buffer is null"));
    }
    clear_buf(out);
    run_on_file(FUNCTION, handle, |file| {
        let eic = get_eic_options(from, to, ppm_tolerance, mz_tolerance, time_unit);
        let options = get_feature_options(eic, grid_start, grid_end, grid_step_size, options, threads);
        let features = crate::find_features(file, options)?;
        write_buf(out, build_features_bridge(&features).ok_or_else(|| QuantionError::internal(FUNCTION, "the features could not be packed"))?);
        Ok(())
    })
}

/// Plan the byte ranges `find_features` reads from an .ion file.
///
/// # Safety
/// `handle` must be a valid `ParsedFile`. `out` must be a valid writable `Buf`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn plan_find_features(
    handle: *mut ParsedFile,
    from: f64,
    to: f64,
    ppm_tolerance: f64,
    mz_tolerance: f64,
    time_unit: u8,
    grid_start: f64,
    grid_end: f64,
    grid_step_size: f64,
    out: *mut Buf,
) -> c_int {
    const FUNCTION: &str = "find_features";
    if out.is_null() {
        return remember(QuantionError::internal(FUNCTION, "the output buffer is null"));
    }
    clear_buf(out);
    run_on_file(FUNCTION, handle, |file| {
        let eic = get_eic_options(from, to, ppm_tolerance, mz_tolerance, time_unit);
        let options = get_feature_options(eic, grid_start, grid_end, grid_step_size, core::ptr::null(), 1);
        options.eic.check(FUNCTION, "eic.")?;
        options.grid.check(FUNCTION)?;
        let mut settings = FeatureSettings::default();
        settings.final_eic_options = eic.in_minutes();
        settings.mz_scan_grid.min_mz = grid_start;
        settings.mz_scan_grid.max_mz = grid_end;
        settings.mz_scan_grid.step = grid_step_size;
        if let FileSource::Lazy(ion) | FileSource::Remote(ion) = &mut file.source {
            let ranges = plan_features_ranges(ion, eic.get_window(), &settings).map_err(|error| from_fast_error(FUNCTION, error))?;
            write_ranges(out, &ranges);
        }
        Ok(())
    })
}

fn read_feature(function: &'static str, index: usize, values: &[f64]) -> Result<crate::Feature, QuantionError> {
    let n_points = values[6];
    if !(n_points >= 0.0 && n_points.fract() == 0.0) {
        return Err(QuantionError::invalid(function, format!("features[{index}].n_points"), format!("must be a count, got {n_points}")));
    }
    Ok(crate::Feature {
        mz: values[0],
        rt: values[1],
        from: values[2],
        to: values[3],
        intensity: values[4],
        integral: values[5],
        n_points: n_points as usize,
        frequency: (!values[7].is_nan()).then_some(values[7]),
        ..Default::default()
    })
}

/// Remove 13C isotope peaks from a feature list. `features` holds 8 values per feature, in this order:
/// mz, rt, from, to, intensity, integral, n_points, frequency (NaN when there is none).
///
/// # Safety
/// `features` must point to `count * 8` readable values. `out` must be a valid writable `Buf`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn deisotope_features(features: *const f64, count: usize, rt_tolerance: f64, mz_tolerance: f64, out: *mut Buf) -> c_int {
    const FUNCTION: &str = "deisotope_features";
    const FEATURE_VALUES: usize = 8;
    if out.is_null() {
        return remember(QuantionError::internal(FUNCTION, "the output buffer is null"));
    }
    clear_buf(out);
    run(FUNCTION, || {
        let size = count.checked_mul(FEATURE_VALUES).ok_or_else(|| QuantionError::invalid(FUNCTION, "features", "has too many values"))?;
        let (rows, _) = read_numbers(features, size).as_chunks::<FEATURE_VALUES>();
        let features = rows
            .iter()
            .enumerate()
            .map(|(index, values)| read_feature(FUNCTION, index, values))
            .collect::<Result<Vec<_>, _>>()?;
        let has_frequency = features.iter().any(|feature| feature.frequency.is_some());
        let found = crate::deisotope_features(features, crate::DeisotopeFeaturesOptions { rt_tolerance, mz_tolerance })?;
        let bytes = if has_frequency { super::build_consensus_bridge(&found) } else { super::build_features_bridge(&found) }
            .ok_or_else(|| QuantionError::internal(FUNCTION, "the features are too large"))?;
        write_buf(out, bytes);
        Ok(())
    })
}

/// Find features in every file of a folder and match them.
///
/// # Safety
/// `directory` must be a valid NUL-terminated string. `options` must be null or valid.
/// `out` must be a valid writable `Buf`.
#[cfg(not(all(target_arch = "wasm32", not(target_os = "wasi"))))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn match_features(
    directory: *const core::ffi::c_char,
    from: f64,
    to: f64,
    ppm_tolerance: f64,
    mz_tolerance: f64,
    time_unit: u8,
    grid_start: f64,
    grid_end: f64,
    grid_step_size: f64,
    grouping_ppm_tolerance: f64,
    grouping_mz_tolerance: f64,
    grouping_rt_tolerance: f64,
    grouping_min_frequency: f64,
    options: *const CPeakOptions,
    threads: usize,
    out: *mut Buf,
) -> c_int {
    const FUNCTION: &str = "match_features";
    if out.is_null() {
        return remember(QuantionError::internal(FUNCTION, "the output buffer is null"));
    }
    clear_buf(out);
    run(FUNCTION, || {
        let directory = read_text(FUNCTION, "directory", directory)?.ok_or_else(|| QuantionError::missing(FUNCTION, "directory"))?;
        let eic = get_eic_options(from, to, ppm_tolerance, mz_tolerance, time_unit);
        let features = get_feature_options(eic, grid_start, grid_end, grid_step_size, options, threads);
        let options = crate::MatchFeaturesOptions {
            eic: features.eic,
            grid: features.grid,
            grouping: crate::GroupingOptions {
                ppm_tolerance: grouping_ppm_tolerance,
                mz_tolerance: grouping_mz_tolerance,
                rt_tolerance: grouping_rt_tolerance,
                min_frequency: grouping_min_frequency,
            },
            find_peaks: features.find_peaks,
            threads: features.threads,
        };
        let matched = crate::match_features(directory, options)?;
        let bytes = super::build_consensus_bridge(&matched).ok_or_else(|| QuantionError::internal(FUNCTION, "the features could not be packed"))?;
        write_buf(out, bytes);
        Ok(())
    })
}
