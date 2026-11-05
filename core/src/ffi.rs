#[cfg(not(all(target_arch = "wasm32", not(target_os = "wasi"))))]
use core::ffi::{CStr, c_char, c_int, c_void};
#[cfg(all(target_arch = "wasm32", not(target_os = "wasi")))]
use core::ffi::{c_int, c_void};
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    ptr, slice,
    sync::Arc,
};

use ionic::{
    IonReader, ReadOptions,
    mzml::parse_mzml as parse_mzml_rs,
    source::{ByteRange, CallbackSource, ReadBytes, header_ranges, merge_ranges},
};

use crate::sample_file::{FileSource, SampleFile};
use crate::utilities::{
    bridge::*,
    calculate_eic::{
        EicReader, FastError,
    },
    find_noise_level::find_noise_level as find_noise_level_rs,
    fit_peak::PeakParameters,
    ion::{ScanSource, ScanSummary},
};

pub const QUANTION_ABI_VERSION: u32 = 2;

#[unsafe(no_mangle)]
pub extern "C" fn quantion_abi_version() -> u32 {
    QUANTION_ABI_VERSION
}

#[unsafe(no_mangle)]
pub extern "C" fn quantion_sizeof_peak_options() -> usize {
    core::mem::size_of::<CPeakOptions>()
}

mod model;
mod public;
mod warp;

use public::fail;

fn check_image_request(mz: f64, tolerance: f64) -> Result<(), crate::QuantionError> {
    crate::error::check_positive("get_ion_image", "mz", mz)?;
    crate::error::check_not_negative("get_ion_image", "tolerance", tolerance)
}

const OK: c_int = 0;
const ERR_INVALID_ARGS: c_int = 1;
const ERR_PANIC: c_int = 99;
const ERR_PARSE: c_int = 6;
const ERR_ENCODE: c_int = 99;
const ERR_FAST_PATH: c_int = 6;

fn fast_error_to_code(error: FastError) -> c_int {
    match error {
        FastError::InvalidRequest => ERR_INVALID_ARGS,
        _ => ERR_FAST_PATH,
    }
}

#[repr(C)]
pub struct Buf {
    pub ptr: *mut u8,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CPeakOptions {
    pub min_integral: f64,
    pub min_intensity: f64,
    pub min_peak_width_points: c_int,
    pub shape: c_int,
    pub noise: f64,
    pub auto_noise: c_int,
    pub auto_baseline: c_int,
    pub max_iterations: c_int,
    pub allow_overlap: c_int,
    pub lambda: f64,
    pub min_snr: f64,
    pub min_r2: f64,
    pub kernel_size: c_int,
}

const _: () = assert!(
    core::mem::size_of::<CPeakOptions>() == 80,
    "CPeakOptions size drifted — bump QUANTION_ABI_VERSION and update all wrappers"
);

pub trait RangeReader {
    fn read(&self, offset: u64, len: u64, dest: &mut [u8]) -> i32;
}

pub fn read_range<R: RangeReader>(
    reader: &R,
    offset: u64,
    length: u64,
) -> ionic::IonResult<Vec<u8>> {
    use ionic::IonError;

    if length == 0 {
        return Ok(Vec::new());
    }
    let len_u32 = u32::try_from(length)
        .map_err(|_| IonError::from("range read: length exceeds transport limit"))?;
    let mut buf = vec![0u8; len_u32 as usize];
    let rc = reader.read(offset, length, &mut buf);
    if rc != 0 {
        return Err(IonError::from(format!("range read failed: {rc}")));
    }
    Ok(buf)
}

/// A function the caller supplies so the library can ask for a slice of the file.
///
/// It must write `length` bytes taken from `offset` into `dest`, and return 0.
/// Any other value means the read failed.
/// It is always called on the thread that called the entry point.
#[cfg(not(all(target_arch = "wasm32", not(target_os = "wasi"))))]
pub type ReadRangeCallback =
    unsafe extern "C" fn(context: *mut c_void, offset: u64, length: u64, dest: *mut u8) -> i32;

#[cfg(not(all(target_arch = "wasm32", not(target_os = "wasi"))))]
struct CallerRangeReader {
    read: ReadRangeCallback,
    context: *mut c_void,
}

#[cfg(not(all(target_arch = "wasm32", not(target_os = "wasi"))))]
unsafe impl Send for CallerRangeReader {}

#[cfg(not(all(target_arch = "wasm32", not(target_os = "wasi"))))]
unsafe impl Sync for CallerRangeReader {}

#[cfg(not(all(target_arch = "wasm32", not(target_os = "wasi"))))]
impl RangeReader for CallerRangeReader {
    fn read(&self, offset: u64, len: u64, dest: &mut [u8]) -> i32 {
        unsafe { (self.read)(self.context, offset, len, dest.as_mut_ptr()) }
    }
}

/// Open an ion file whose bytes the caller supplies on demand.
///
/// The library never reads a file or a socket itself: every time it needs a slice
/// it calls `read`. Use this for a file that lives somewhere the library cannot
/// reach, such as a URL.
///
/// # Safety
/// `read` must stay callable, and `context` must stay valid, until the returned
/// handle is freed with `free_mzml`.
#[cfg(not(all(target_arch = "wasm32", not(target_os = "wasi"))))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_ion_source(
    read: Option<
        unsafe extern "C" fn(context: *mut c_void, offset: u64, length: u64, dest: *mut u8) -> i32,
    >,
    context: *mut c_void,
    max_cache_size: usize,
    out: *mut *mut ParsedFile,
) -> c_int {
    let Some(read) = read else {
        return fail(ERR_INVALID_ARGS, "parse_ion");
    };
    if out.is_null() {
        return fail(ERR_INVALID_ARGS, "parse_ion");
    }
    match catch_unwind(AssertUnwindSafe(|| -> Result<(), c_int> {
        let reader = CallerRangeReader { read, context };
        let ion = IonReader::new(
            Arc::new(CallbackSource::new(move |range| {
                read_range(&reader, range.offset, range.length)
            })) as Arc<dyn ReadBytes>,
            &ReadOptions {
                max_cached_bytes: max_cache_size,
                ..Default::default()
            },
        )
        .map_err(|_| ERR_PARSE)?;

        unsafe {
            *out = Box::into_raw(Box::new(ParsedFile::new(FileSource::Remote(Box::new(ion)))))
        };
        Ok(())
    })) {
        Ok(Ok(())) => OK,
        Ok(Err(code)) => fail(code, "parse_ion"),
        Err(_) => fail(ERR_PANIC, "parse_ion"),
    }
}

#[cfg(all(target_arch = "wasm32", not(target_os = "wasi")))]
#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn range_read(
        source_id: u32,
        offset_lo: u32,
        offset_hi: u32,
        len: u32,
        dest_ptr: *mut u8,
    ) -> i32;
}

#[cfg(all(target_arch = "wasm32", not(target_os = "wasi")))]
struct WasmRangeReader {
    source_id: u32,
}

#[cfg(all(target_arch = "wasm32", not(target_os = "wasi")))]
impl RangeReader for WasmRangeReader {
    fn read(&self, offset: u64, len: u64, dest: &mut [u8]) -> i32 {
        let len_u32 = match u32::try_from(len) {
            Ok(value) => value,
            Err(_) => return -2,
        };
        unsafe {
            range_read(
                self.source_id,
                offset as u32,
                (offset >> 32) as u32,
                len_u32,
                dest.as_mut_ptr(),
            )
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn alloc(size: usize) -> *mut u8 {
    if size == 0 {
        return core::ptr::null_mut();
    }
    catch_unwind(AssertUnwindSafe(|| {
        let mut v = Vec::<u8>::with_capacity(size);
        let p = v.as_mut_ptr();
        core::mem::forget(v);
        p
    }))
    .unwrap_or(core::ptr::null_mut())
}

/// Free memory returned by `alloc`.
///
/// # Safety
/// `ptr_raw` must be a live pointer from `alloc(size)`.
/// `size` must match the `alloc` call.
/// After this call, `ptr_raw` is invalid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_(ptr_raw: *mut u8, size: usize) {
    if !ptr_raw.is_null() {
        unsafe {
            drop(Vec::<u8>::from_raw_parts(ptr_raw, 0, size));
        }
    }
}

pub struct ParsedFile {
    pub(crate) file: SampleFile,
    broken: bool,
}

impl ParsedFile {
    pub(crate) fn new(source: FileSource) -> Self {
        Self {
            file: SampleFile::new(source),
            broken: false,
        }
    }

    pub(crate) fn from_file(file: SampleFile) -> Self {
        Self { file, broken: false }
    }

    pub(crate) fn is_broken(&self) -> bool {
        self.broken
    }

    fn source_mut(&mut self) -> &mut FileSource {
        &mut self.file.source
    }
}

fn file_is_broken(h: *const ParsedFile) -> bool {
    !h.is_null() && unsafe { (*h).broken }
}

fn mark_file_broken(h: *const ParsedFile) {
    if !h.is_null() {
        unsafe { (*(h as *mut ParsedFile)).broken = true };
    }
}

impl ScanSource for ParsedFile {
    fn for_each_summary(&mut self, cb: &mut dyn FnMut(usize, ScanSummary)) {
        match &mut self.file.source {
            FileSource::Full(mzml) => mzml.for_each_summary(cb),
            FileSource::Lazy(file) => file.for_each_summary(cb),
            FileSource::Remote(ion) => ion.for_each_summary(cb),
        }
    }

    fn load_scan(&mut self, index: usize, mz: &mut Vec<f64>, intensity: &mut Vec<f64>) -> bool {
        match &mut self.file.source {
            FileSource::Full(mzml) => mzml.load_scan(index, mz, intensity),
            FileSource::Lazy(file) => file.load_scan(index, mz, intensity),
            FileSource::Remote(ion) => ion.load_scan(index, mz, intensity),
        }
    }

    fn load_array(&mut self, index: usize, accession: u32, values: &mut Vec<f64>) -> bool {
        match &mut self.file.source {
            FileSource::Full(mzml) => mzml.load_array(index, accession, values),
            FileSource::Lazy(file) => file.load_array(index, accession, values),
            FileSource::Remote(ion) => ion.load_array(index, accession, values),
        }
    }

    fn load_metadata(&mut self, index: usize) -> ionic::IonResult<Vec<crate::files::ScanParam>> {
        match &mut self.file.source {
            FileSource::Full(mzml) => mzml.load_metadata(index),
            FileSource::Lazy(file) => file.load_metadata(index),
            FileSource::Remote(ion) => ion.load_metadata(index),
        }
    }
}

/// Free a `ParsedFile` handle.
///
/// # Safety
/// `handle` must be a valid pointer returned by this library.
/// Do not free the same handle twice.
/// After this call, `handle` is invalid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_mzml(handle: *mut ParsedFile) {
    if !handle.is_null() {
        let _ = catch_unwind(AssertUnwindSafe(|| {
            drop(unsafe { Box::from_raw(handle) });
        }));
    }
}

/// Parse mzML bytes and store the result in `dest`.
///
/// # Safety
/// `data_ptr` must point to `data_len` readable bytes.
/// `dest` must be a valid writable pointer to `*mut ParsedFile`.
/// On success, `*dest` must be freed with `free_mzml`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_mzml(
    data_ptr: *const u8,
    data_len: usize,
    dest: *mut *mut ParsedFile,
) -> c_int {
    if data_ptr.is_null() || dest.is_null() {
        return fail(ERR_INVALID_ARGS, "parse_mzml");
    }
    match catch_unwind(AssertUnwindSafe(|| -> Result<(), c_int> {
        let parsed = parse_mzml_rs(unsafe { slice::from_raw_parts(data_ptr, data_len) })
            .map_err(|_| ERR_PARSE)?;
        unsafe {
            *dest = Box::into_raw(Box::new(ParsedFile::new(FileSource::Full(Box::new(
                parsed,
            )))))
        };
        Ok(())
    })) {
        Ok(Ok(())) => OK,
        Ok(Err(c)) => fail(c, "parse_mzml"),
        Err(_) => fail(ERR_PANIC, "parse_mzml"),
    }
}

#[cfg(all(target_arch = "wasm32", not(target_os = "wasi")))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_ion_url(
    source_id: u32,
    cache_bytes: usize,
    dest: *mut *mut ParsedFile,
) -> c_int {
    if dest.is_null() {
        return fail(ERR_INVALID_ARGS, "parse_ion");
    }
    match catch_unwind(AssertUnwindSafe(|| -> Result<(), c_int> {
        let reader = WasmRangeReader { source_id };
        let ion = IonReader::new(
            Arc::new(CallbackSource::new(move |range| {
                read_range(&reader, range.offset, range.length)
            })) as Arc<dyn ReadBytes>,
            &ReadOptions {
                max_cached_bytes: cache_bytes,
                ..Default::default()
            },
        )
        .map_err(|_| ERR_PARSE)?;

        unsafe {
            *dest = Box::into_raw(Box::new(ParsedFile::new(FileSource::Remote(Box::new(ion)))))
        };
        Ok(())
    })) {
        Ok(Ok(())) => OK,
        Ok(Err(c)) => fail(c, "parse_ion"),
        Err(_) => fail(ERR_PANIC, "parse_ion"),
    }
}

/// Parse binary data and store the result in `dest`.
///
/// # Safety
/// `data_ptr` must point to `data_len` readable bytes.
/// `dest` must be a valid writable pointer to `*mut ParsedFile`.
/// On success, `*dest` must be freed with `free_mzml`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_bin(
    data_ptr: *const u8,
    data_len: usize,
    max_cache_size: usize,
    dest: *mut *mut ParsedFile,
) -> c_int {
    if data_ptr.is_null() || dest.is_null() {
        return fail(ERR_INVALID_ARGS, "parse_ion");
    }
    let bytes = unsafe { std::slice::from_raw_parts(data_ptr, data_len) }.to_vec();
    open_ion(crate::IonInput::Bytes(bytes), max_cache_size, dest)
}

fn open_ion(input: crate::IonInput, max_cache_size: usize, dest: *mut *mut ParsedFile) -> c_int {
    public::run("parse_ion", || {
        let file = crate::parse_ion(input, crate::ParseIonOptions { max_cache_size })?;
        unsafe { *dest = Box::into_raw(Box::new(ParsedFile::from_file(file))) };
        Ok(())
    })
}

#[cfg(not(all(target_arch = "wasm32", not(target_os = "wasi"))))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_ion_path(
    path_ptr: *const c_char,
    max_cache_size: usize,
    dest: *mut *mut ParsedFile,
) -> c_int {
    if path_ptr.is_null() || dest.is_null() {
        return fail(ERR_INVALID_ARGS, "parse_ion");
    }
    let Ok(path) = unsafe { CStr::from_ptr(path_ptr) }.to_str() else {
        return public::remember(crate::QuantionError::invalid("parse_ion", "source", "must be valid UTF-8 text"));
    };
    open_ion(crate::IonInput::Path(std::path::PathBuf::from(path)), max_cache_size, dest)
}

const LARGEST_OPEN_GAP: u64 = 131072;

fn open_gap_for(ranges: &[ByteRange]) -> u64 {
    let total = ranges
        .iter()
        .map(|range| range.offset + range.length)
        .max()
        .unwrap_or(0);
    LARGEST_OPEN_GAP.min(total / 8)
}

/// Plan the byte ranges an open needs.
///
/// # Safety
/// `header_ptr` must point to at least `header_len` readable bytes, and `header_len` must be at
/// least 1024 so the header can be parsed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn plan_open(
    header_ptr: *const u8,
    header_len: usize,
    out: *mut Buf,
) -> c_int {
    if header_ptr.is_null() || out.is_null() {
        return fail(ERR_INVALID_ARGS, "parse_ion");
    }
    clear_buf(out);

    match catch_unwind(AssertUnwindSafe(|| -> Result<(), c_int> {
        let header = unsafe { slice::from_raw_parts(header_ptr, header_len) };
        let mut ranges = header_ranges(header).map_err(|_| ERR_FAST_PATH)?;
        let gap = open_gap_for(&ranges);
        merge_ranges(&mut ranges, gap);
        let bytes = pack_byte_ranges(ranges.iter().map(|range| (range.offset, range.length)));
        write_buf(out, bytes);
        Ok(())
    })) {
        Ok(Ok(())) => OK,
        Ok(Err(code)) => fail(code, "parse_ion"),
        Err(_) => fail(ERR_PANIC, "parse_ion"),
    }
}





/// Write the noise window width and the noise intensity for an intensity array.
///
/// # Safety
/// `y_ptr` must point to `len` readable `f32` values.
/// `out_width` and `out_intensity` must be valid writable pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_noise_level(
    y_ptr: *const f32,
    len: usize,
    out_width: *mut usize,
    out_intensity: *mut f64,
) -> c_int {
    if y_ptr.is_null() || out_width.is_null() || out_intensity.is_null() || len == 0 {
        return ERR_INVALID_ARGS;
    }
    match catch_unwind(AssertUnwindSafe(|| {
        find_noise_level_rs(unsafe { slice::from_raw_parts(y_ptr, len) })
    })) {
        Ok(noise) => {
            unsafe {
                *out_width = noise.width;
                *out_intensity = noise.intensity;
            }
            OK
        }
        Err(_) => ERR_PANIC,
    }
}



#[unsafe(no_mangle)]
pub unsafe extern "C" fn plan_image(
    h: *mut ParsedFile,
    target: f64,
    tolerance: f64,
    level: u8,
    out: *mut Buf,
) -> c_int {
    if let Err(error) = check_image_request(target, tolerance) {
        return public::remember(error);
    }
    if h.is_null() || out.is_null() {
        return fail(ERR_INVALID_ARGS, "get_ion_image");
    }
    if file_is_broken(h) {
        return fail(ERR_PANIC, "get_ion_image");
    }
    clear_buf(out);

    match catch_unwind(AssertUnwindSafe(|| -> Result<(), c_int> {
        let file = unsafe { &mut *h };
        let ranges = match file.source_mut() {
            FileSource::Lazy(ion) => crate::utilities::ion_image::plan_ion_image_ranges(
                ion.as_mut(),
                target,
                tolerance,
                level,
            ),
            FileSource::Remote(ion) => crate::utilities::ion_image::plan_ion_image_ranges(
                ion.as_mut(),
                target,
                tolerance,
                level,
            ),
            FileSource::Full(_) => Err(FastError::UnsupportedBackend),
        }
        .map_err(fast_error_to_code)?;

        let bytes = pack_byte_ranges(ranges.iter().map(|range| (range.offset, range.length)));
        write_buf(out, bytes);
        Ok(())
    })) {
        Ok(Ok(())) => OK,
        Ok(Err(code)) => fail(code, "get_ion_image"),
        Err(_) => {
            mark_file_broken(h);
            fail(ERR_PANIC, "get_ion_image")
        }
    }
}



#[unsafe(no_mangle)]
pub unsafe extern "C" fn image_begin(
    h: *mut ParsedFile,
    target: f64,
    tolerance: f64,
    level: u8,
    out_session: *mut usize,
) -> c_int {
    if h.is_null() || out_session.is_null() {
        return fail(ERR_INVALID_ARGS, "get_ion_image");
    }
    if file_is_broken(h) {
        return fail(ERR_PANIC, "get_ion_image");
    }
    if let Err(error) = check_image_request(target, tolerance) {
        return public::remember(error);
    }

    match catch_unwind(AssertUnwindSafe(|| -> Result<(), c_int> {
        let file = unsafe { &mut *h };
        let mut reader = match file.source_mut() {
            FileSource::Full(mzml) => EicReader::Mzml(mzml.as_mut()),
            FileSource::Lazy(ion) => EicReader::Ion(ion.as_mut()),
            FileSource::Remote(ion) => EicReader::Ion(ion.as_mut()),
        };
        let session =
            crate::utilities::ion_image::image_session_begin(&mut reader, target, tolerance, level);
        let pointer = Box::into_raw(Box::new(session)) as usize;
        unsafe { *out_session = pointer };
        Ok(())
    })) {
        Ok(Ok(())) => OK,
        Ok(Err(code)) => fail(code, "get_ion_image"),
        Err(_) => {
            mark_file_broken(h);
            fail(ERR_PANIC, "get_ion_image")
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn image_scan_count(
    session: *mut crate::utilities::ion_image::ImageSession,
    out_count: *mut usize,
) -> c_int {
    if session.is_null() || out_count.is_null() {
        return fail(ERR_INVALID_ARGS, "get_ion_image");
    }
    match catch_unwind(AssertUnwindSafe(|| {
        let session = unsafe { &*session };
        unsafe { *out_count = session.scan_count() };
    })) {
        Ok(()) => OK,
        Err(_) => fail(ERR_PANIC, "get_ion_image"),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn image_ranges(
    h: *mut ParsedFile,
    session: *mut crate::utilities::ion_image::ImageSession,
    from: usize,
    count: usize,
    out: *mut Buf,
) -> c_int {
    if h.is_null() || session.is_null() || out.is_null() {
        return fail(ERR_INVALID_ARGS, "get_ion_image");
    }
    if session_is_broken(session) {
        return fail(ERR_PANIC, "get_ion_image");
    }
    if file_is_broken(h) {
        return fail(ERR_PANIC, "get_ion_image");
    }
    clear_buf(out);

    match catch_unwind(AssertUnwindSafe(|| -> Result<(), c_int> {
        let file = unsafe { &mut *h };
        let session = unsafe { &*session };
        let ranges = match file.source_mut() {
            FileSource::Lazy(ion) => session.ranges(ion.as_mut(), from, count),
            FileSource::Remote(ion) => session.ranges(ion.as_mut(), from, count),
            FileSource::Full(_) => Err(FastError::UnsupportedBackend),
        }
        .map_err(fast_error_to_code)?;

        let bytes = pack_byte_ranges(ranges.iter().map(|range| (range.offset, range.length)));
        write_buf(out, bytes);
        Ok(())
    })) {
        Ok(Ok(())) => OK,
        Ok(Err(code)) => fail(code, "get_ion_image"),
        Err(_) => {
            mark_file_broken(h);
            fail(ERR_PANIC, "get_ion_image")
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn image_fold(
    h: *mut ParsedFile,
    session: *mut crate::utilities::ion_image::ImageSession,
    from: usize,
    count: usize,
) -> c_int {
    if h.is_null() || session.is_null() {
        return fail(ERR_INVALID_ARGS, "get_ion_image");
    }
    if session_is_broken(session) {
        return fail(ERR_PANIC, "get_ion_image");
    }
    if file_is_broken(h) {
        return fail(ERR_PANIC, "get_ion_image");
    }

    match catch_unwind(AssertUnwindSafe(|| -> Result<(), c_int> {
        let file = unsafe { &mut *h };
        let session = unsafe { &mut *session };
        let mut reader = match file.source_mut() {
            FileSource::Full(mzml) => EicReader::Mzml(mzml.as_mut()),
            FileSource::Lazy(ion) => EicReader::Ion(ion.as_mut()),
            FileSource::Remote(ion) => EicReader::Ion(ion.as_mut()),
        };
        session.fold(&mut reader, from, count);
        Ok(())
    })) {
        Ok(Ok(())) => OK,
        Ok(Err(code)) => fail(code, "get_ion_image"),
        Err(_) => {
            mark_file_broken(h);
            fail(ERR_PANIC, "get_ion_image")
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn image_finish(
    session: *mut crate::utilities::ion_image::ImageSession,
    out: *mut Buf,
) -> c_int {
    if session.is_null() || out.is_null() {
        return fail(ERR_INVALID_ARGS, "get_ion_image");
    }
    if session_is_broken(session) {
        return fail(ERR_PANIC, "get_ion_image");
    }
    clear_buf(out);

    match catch_unwind(AssertUnwindSafe(|| -> Result<(), c_int> {
        let session = unsafe { &*session };
        let image = session.finish();
        let bytes = build_image_bridge(&image).ok_or(ERR_ENCODE)?;
        write_buf(out, bytes);
        Ok(())
    })) {
        Ok(Ok(())) => OK,
        Ok(Err(code)) => fail(code, "get_ion_image"),
        Err(_) => {
            mark_session_broken(session);
            fail(ERR_PANIC, "get_ion_image")
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn image_free(session: *mut crate::utilities::ion_image::ImageSession) {
    if !session.is_null() {
        let _ = catch_unwind(AssertUnwindSafe(|| {
            drop(unsafe { Box::from_raw(session) });
        }));
    }
}


fn build_scan_arrays_bridge(found: Vec<crate::utilities::calculate_eic::ScanArrays>) -> Option<Box<[u8]>> {
    let scan_count = found.len() as u64;
    let array_count: usize = found.iter().map(|scan| scan.arrays.len()).sum();
    let value_count: u64 = found.iter().flat_map(|scan| &scan.arrays).map(|values| values.len() as u64).sum();
    let scans: Vec<crate::Scan> = found.iter().map(|scan| crate::Scan::from(scan.scan.clone())).collect();

    let mut builder = BridgeBuilder::new(QUANTION_PAYLOAD_SCANS, scan_count);
    builder.add_section(QUANTION_SECTION_POINT_STARTS, QUANTION_ELEMENT_U64, scan_count + 1);
    builder.add_section(QUANTION_SECTION_MZ, QUANTION_ELEMENT_F64, 0);
    builder.add_section(QUANTION_SECTION_INTENSITY, QUANTION_ELEMENT_F64, 0);
    for id in SCAN_DETAIL_SECTIONS {
        builder.add_section(id, QUANTION_ELEMENT_F64, scan_count);
    }
    builder.add_section(QUANTION_SECTION_SCAN_ARRAY_STARTS, QUANTION_ELEMENT_U64, array_count as u64 + 1);
    builder.add_section(QUANTION_SECTION_SCAN_ARRAY_VALUES, QUANTION_ELEMENT_F64, value_count);
    let mut bridge = builder.build()?;

    for (index, scan) in scans.iter().enumerate() {
        write_scan_details(&mut bridge, index as u64, scan);
    }
    bridge.write_u64_section(QUANTION_SECTION_POINT_STARTS, &vec![0u64; scans.len() + 1]);

    let mut array_starts = Vec::with_capacity(array_count + 1);
    array_starts.push(0u64);
    let mut next_value: u64 = 0;
    for values in found.iter().flat_map(|scan| &scan.arrays) {
        bridge.write_f64_at(QUANTION_SECTION_SCAN_ARRAY_VALUES, next_value, values);
        next_value += values.len() as u64;
        array_starts.push(next_value);
    }
    bridge.write_u64_section(QUANTION_SECTION_SCAN_ARRAY_STARTS, &array_starts);
    Some(bridge.into_bytes())
}

fn write_scan_details(bridge: &mut Bridge, position: u64, scan: &crate::Scan) {
    let details = [
        (QUANTION_SECTION_SCAN_INDEX, scan.index as f64),
        (QUANTION_SECTION_RT, scan.rt),
        (QUANTION_SECTION_MS_LEVEL, f64::from(scan.level)),
        (QUANTION_SECTION_SELECTED_ION_MZ, scan.precursor_mz),
        (QUANTION_SECTION_BASE_PEAK_MZ, scan.base_peak_mz),
        (QUANTION_SECTION_BASE_PEAK_INT, scan.base_peak_intensity),
        (QUANTION_SECTION_TOTAL_ION_CURRENT, scan.total_ion_current),
        (QUANTION_SECTION_POLARITY, f64::from(scan.polarity)),
        (QUANTION_SECTION_POSITION_X, f64::from(scan.x)),
        (QUANTION_SECTION_POSITION_Y, f64::from(scan.y)),
        (QUANTION_SECTION_POSITION_Z, f64::from(scan.z)),
    ];
    for (id, value) in details {
        bridge.write_f64_at(id, position, &[value]);
    }
}

fn build_scans_bridge(scans: &[crate::Scan]) -> Option<Box<[u8]>> {
    let scan_count = scans.len() as u64;
    let mut total_points: u64 = 0;
    for scan in scans {
        total_points = total_points.checked_add(scan.mz.len() as u64)?;
    }

    let mut builder = BridgeBuilder::new(QUANTION_PAYLOAD_SCANS, scan_count);
    builder.add_section(QUANTION_SECTION_POINT_STARTS, QUANTION_ELEMENT_U64, scan_count + 1);
    builder.add_section(QUANTION_SECTION_MZ, QUANTION_ELEMENT_F64, total_points);
    builder.add_section(QUANTION_SECTION_INTENSITY, QUANTION_ELEMENT_F64, total_points);
    for id in SCAN_DETAIL_SECTIONS {
        builder.add_section(id, QUANTION_ELEMENT_F64, scan_count);
    }
    let mut bridge = builder.build()?;

    let mut point_starts = Vec::with_capacity(scans.len() + 1);
    point_starts.push(0u64);
    let mut next_point: u64 = 0;
    for (index, scan) in scans.iter().enumerate() {
        let position = index as u64;
        bridge.write_f64_at(QUANTION_SECTION_MZ, next_point, &scan.mz);
        bridge.write_f64_at(QUANTION_SECTION_INTENSITY, next_point, &scan.intensity);
        write_scan_details(&mut bridge, position, scan);
        next_point += scan.mz.len() as u64;
        point_starts.push(next_point);
    }

    bridge.write_u64_section(QUANTION_SECTION_POINT_STARTS, &point_starts);
    Some(bridge.into_bytes())
}

fn build_image_bridge(image: &crate::utilities::ion_image::IonImage) -> Option<Box<[u8]>> {
    let cell_count = image.data.len() as u64;
    let mut builder = BridgeBuilder::new(QUANTION_PAYLOAD_ION_IMAGE, cell_count);
    builder.add_section(QUANTION_SECTION_IMAGE_SHAPE, QUANTION_ELEMENT_U32, 6);
    builder.add_section(
        QUANTION_SECTION_IMAGE_DATA,
        QUANTION_ELEMENT_F64,
        cell_count,
    );
    builder.add_section(
        QUANTION_SECTION_IMAGE_COUNTS,
        QUANTION_ELEMENT_U32,
        image.counts.len() as u64,
    );
    let mut bridge = builder.build()?;

    let shape = [
        image.width,
        image.height,
        image.min_x,
        image.min_y,
        image.min_z,
        image.max_z,
    ];
    bridge.write_u32_section(QUANTION_SECTION_IMAGE_SHAPE, &shape);
    bridge.write_f64_section(QUANTION_SECTION_IMAGE_DATA, &image.data);
    bridge.write_u32_section(QUANTION_SECTION_IMAGE_COUNTS, &image.counts);
    Some(bridge.into_bytes())
}


fn read_ids(
    count: usize,
    ids_off: *const u32,
    ids_len: *const u32,
    ids_buf: *const u8,
    ids_buf_len: usize,
) -> Vec<String> {
    if ids_off.is_null() || ids_len.is_null() || ids_buf.is_null() || ids_buf_len == 0 {
        return vec![String::new(); count];
    }
    let offsets = unsafe { slice::from_raw_parts(ids_off, count) };
    let lengths = unsafe { slice::from_raw_parts(ids_len, count) };
    let bytes = unsafe { slice::from_raw_parts(ids_buf, ids_buf_len) };
    (0..count)
        .map(|index| {
            let (start, length) = (offsets[index] as usize, lengths[index] as usize);
            start
                .checked_add(length)
                .filter(|&end| end <= bytes.len())
                .and_then(|end| std::str::from_utf8(&bytes[start..end]).ok())
                .unwrap_or("")
                .to_string()
        })
        .collect()
}

#[inline]
fn f64_to_u8(v: &[f64]) -> Box<[u8]> {
    let n = core::mem::size_of_val(v);
    let mut out = vec![0u8; n];
    unsafe {
        ptr::copy_nonoverlapping(v.as_ptr() as *const u8, out.as_mut_ptr(), n);
    }
    out.into_boxed_slice()
}

fn count_as_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn build_peaks_bridge(peaks: &[crate::utilities::structs::Peak]) -> Option<Box<[u8]>> {
    let from: Vec<f64> = peaks.iter().map(|peak| peak.from).collect();
    let to: Vec<f64> = peaks.iter().map(|peak| peak.to).collect();
    let rt: Vec<f64> = peaks.iter().map(|peak| peak.rt).collect();
    let integral: Vec<f64> = peaks.iter().map(|peak| peak.integral).collect();
    let intensity: Vec<f64> = peaks.iter().map(|peak| peak.intensity).collect();
    let point_count: Vec<u32> = peaks
        .iter()
        .map(|peak| count_as_u32(peak.n_points))
        .collect();

    build_record_bridge(
        QUANTION_PAYLOAD_PEAKS,
        peaks.len() as u64,
        &[
            Column::Numbers {
                id: QUANTION_SECTION_PEAK_FROM,
                values: &from,
            },
            Column::Numbers {
                id: QUANTION_SECTION_PEAK_TO,
                values: &to,
            },
            Column::Numbers {
                id: QUANTION_SECTION_PEAK_RT,
                values: &rt,
            },
            Column::Numbers {
                id: QUANTION_SECTION_PEAK_INTEGRAL,
                values: &integral,
            },
            Column::Numbers {
                id: QUANTION_SECTION_PEAK_INTENSITY,
                values: &intensity,
            },
            Column::Counts {
                id: QUANTION_SECTION_PEAK_POINT_COUNT,
                values: &point_count,
            },
        ],
    )
}

fn build_fit_bridge(params: &Option<PeakParameters>) -> Option<Box<[u8]>> {
    let (height, center, fwhm, tail, r2) = match params {
        Some(found) => (
            vec![found.height],
            vec![found.center],
            vec![found.fwhm],
            vec![found.tail],
            vec![found.r2],
        ),
        None => (vec![], vec![], vec![], vec![], vec![]),
    };

    build_record_bridge(
        QUANTION_PAYLOAD_FIT_RESULT,
        height.len() as u64,
        &[
            Column::Numbers {
                id: QUANTION_SECTION_FIT_HEIGHT,
                values: &height,
            },
            Column::Numbers {
                id: QUANTION_SECTION_FIT_CENTER,
                values: &center,
            },
            Column::Numbers {
                id: QUANTION_SECTION_FIT_FWHM,
                values: &fwhm,
            },
            Column::Numbers {
                id: QUANTION_SECTION_FIT_TAIL,
                values: &tail,
            },
            Column::Numbers {
                id: QUANTION_SECTION_FIT_R2,
                values: &r2,
            },
        ],
    )
}

fn build_features_bridge(
    features: &[crate::utilities::find_features::Feature],
) -> Option<Box<[u8]>> {
    let mz: Vec<f64> = features.iter().map(|item| item.mz).collect();
    let rt: Vec<f64> = features.iter().map(|item| item.rt).collect();
    let from: Vec<f64> = features.iter().map(|item| item.from).collect();
    let to: Vec<f64> = features.iter().map(|item| item.to).collect();
    let intensity: Vec<f64> = features.iter().map(|item| item.intensity).collect();
    let integral: Vec<f64> = features.iter().map(|item| item.integral).collect();
    let point_count: Vec<u32> = features
        .iter()
        .map(|item| count_as_u32(item.n_points))
        .collect();

    build_record_bridge(
        QUANTION_PAYLOAD_FEATURES,
        features.len() as u64,
        &[
            Column::Numbers {
                id: QUANTION_SECTION_FEATURE_MZ,
                values: &mz,
            },
            Column::Numbers {
                id: QUANTION_SECTION_FEATURE_RT,
                values: &rt,
            },
            Column::Numbers {
                id: QUANTION_SECTION_FEATURE_FROM,
                values: &from,
            },
            Column::Numbers {
                id: QUANTION_SECTION_FEATURE_TO,
                values: &to,
            },
            Column::Numbers {
                id: QUANTION_SECTION_FEATURE_INTENSITY,
                values: &intensity,
            },
            Column::Numbers {
                id: QUANTION_SECTION_FEATURE_INTEGRAL,
                values: &integral,
            },
            Column::Counts {
                id: QUANTION_SECTION_FEATURE_POINT_COUNT,
                values: &point_count,
            },
        ],
    )
}

fn build_consensus_bridge(
    features: &[crate::utilities::find_features::Feature],
) -> Option<Box<[u8]>> {
    let mz: Vec<f64> = features.iter().map(|item| item.mz).collect();
    let rt: Vec<f64> = features.iter().map(|item| item.rt).collect();
    let from: Vec<f64> = features.iter().map(|item| item.from).collect();
    let to: Vec<f64> = features.iter().map(|item| item.to).collect();
    let intensity: Vec<f64> = features.iter().map(|item| item.intensity).collect();
    let integral: Vec<f64> = features.iter().map(|item| item.integral).collect();
    let n_points: Vec<u32> = features
        .iter()
        .map(|item| count_as_u32(item.n_points))
        .collect();
    let frequency: Vec<f64> = features
        .iter()
        .map(|item| item.frequency.unwrap_or(f64::NAN))
        .collect();

    build_record_bridge(
        QUANTION_PAYLOAD_CONSENSUS_FEATURES,
        features.len() as u64,
        &[
            Column::Numbers {
                id: QUANTION_SECTION_CONSENSUS_MZ,
                values: &mz,
            },
            Column::Numbers {
                id: QUANTION_SECTION_CONSENSUS_RT,
                values: &rt,
            },
            Column::Numbers {
                id: QUANTION_SECTION_CONSENSUS_FROM,
                values: &from,
            },
            Column::Numbers {
                id: QUANTION_SECTION_CONSENSUS_TO,
                values: &to,
            },
            Column::Numbers {
                id: QUANTION_SECTION_CONSENSUS_INTENSITY,
                values: &intensity,
            },
            Column::Numbers {
                id: QUANTION_SECTION_CONSENSUS_INTEGRAL,
                values: &integral,
            },
            Column::Counts {
                id: QUANTION_SECTION_CONSENSUS_POINT_COUNT,
                values: &n_points,
            },
            Column::Numbers {
                id: QUANTION_SECTION_CONSENSUS_FREQUENCY,
                values: &frequency,
            },
        ],
    )
}

const SCAN_DETAIL_SECTIONS: [u32; 11] = [
    QUANTION_SECTION_SCAN_INDEX,
    QUANTION_SECTION_RT,
    QUANTION_SECTION_MS_LEVEL,
    QUANTION_SECTION_SELECTED_ION_MZ,
    QUANTION_SECTION_BASE_PEAK_MZ,
    QUANTION_SECTION_BASE_PEAK_INT,
    QUANTION_SECTION_TOTAL_ION_CURRENT,
    QUANTION_SECTION_POLARITY,
    QUANTION_SECTION_POSITION_X,
    QUANTION_SECTION_POSITION_Y,
    QUANTION_SECTION_POSITION_Z,
];

fn clear_buf(out: *mut Buf) {
    if out.is_null() {
        return;
    }
    unsafe {
        ptr::write_unaligned(
            out,
            Buf {
                ptr: ptr::null_mut(),
                len: 0,
            },
        )
    };
}

fn session_is_broken(session: *const crate::utilities::ion_image::ImageSession) -> bool {
    !session.is_null() && unsafe { (*session).is_broken() }
}

fn mark_session_broken(session: *mut crate::utilities::ion_image::ImageSession) {
    if !session.is_null() {
        unsafe { (*session).mark_broken() };
    }
}

fn write_buf(out: *mut Buf, bytes: Box<[u8]>) {
    let len = bytes.len();
    unsafe {
        ptr::write_unaligned(
            out,
            Buf {
                ptr: Box::into_raw(bytes) as *mut u8,
                len,
            },
        )
    };
}

fn push_u64_le(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn pack_byte_ranges(ranges: impl IntoIterator<Item = (u64, u64)>) -> Box<[u8]> {
    let mut bytes = Vec::new();
    for (offset, length) in ranges {
        push_u64_le(&mut bytes, offset);
        push_u64_le(&mut bytes, length);
    }
    bytes.into_boxed_slice()
}

#[cfg(test)]
mod tests {
    use ionic::mzml::structs::{
        BinaryDataArray, BinaryDataArrayList, CvParam, NumericArray, NumericType, Run, Scan,
        ScanList, Spectrum, SpectrumList,
    };

    use ionic::{SectionStorage, WriteOptions, format::DEFAULT_MZ_WINDOW, mzml::structs::MzML};

    use super::{public::*, *};
    use crate::options::CalculateEicOptions;
    use crate::utilities::{
        calculate_eic::{ScanQuery, calculate_eic as calculate_eic_dispatcher, TimeUnit, get_scans as get_scans_rs, plan_eic_ranges, plan_scan_ranges, plan_window_ranges},
        structs::FromTo,
        ion::write_mzml_to_ion,
    };

    const ION_BLOCK_SIZE_BYTES: usize = 1024 * 1024;
    const ERR_REMOTE_FILE: c_int = 4;

    struct FakeReader {
        data: Vec<u8>,
        call_count: std::sync::atomic::AtomicUsize,
        return_code: i32,
    }

    impl RangeReader for FakeReader {
        fn read(&self, _offset: u64, len: u64, dest: &mut [u8]) -> i32 {
            if len == 0 {
                panic!("read called with len=0");
            }
            self.call_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let copy_len = std::cmp::min(len as usize, dest.len().min(self.data.len()));
            dest[..copy_len].copy_from_slice(&self.data[..copy_len]);
            self.return_code
        }
    }

    #[test]
    fn zero_len_returns_empty_and_never_calls_read() {
        let fake = FakeReader {
            data: vec![1, 2, 3, 4, 5],
            call_count: std::sync::atomic::AtomicUsize::new(0),
            return_code: 0,
        };

        let value = read_range(&fake, 100, 0).unwrap();

        assert_eq!(value.len(), 0);
        assert_eq!(fake.call_count.load(std::sync::atomic::Ordering::SeqCst), 0);
    }

    #[test]
    fn non_zero_read_calls_reader_once() {
        let fake = FakeReader {
            data: vec![10, 20, 30, 40, 50],
            call_count: std::sync::atomic::AtomicUsize::new(0),
            return_code: 0,
        };

        let _result = read_range(&fake, 0, 3).unwrap();

        assert_eq!(fake.call_count.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[test]
    fn negative_return_code_becomes_error() {
        let fake = FakeReader {
            data: vec![10, 20, 30],
            call_count: std::sync::atomic::AtomicUsize::new(0),
            return_code: -1,
        };

        let result = read_range(&fake, 0, 2);

        assert!(result.is_err());
    }

    #[test]
    fn len_exceeding_u32_max_returns_error_without_allocation() {
        let fake = FakeReader {
            data: vec![1, 2, 3],
            call_count: std::sync::atomic::AtomicUsize::new(0),
            return_code: 0,
        };

        let result = read_range(&fake, 0, u64::from(u32::MAX) + 1);

        assert!(result.is_err());
        assert_eq!(fake.call_count.load(std::sync::atomic::Ordering::SeqCst), 0);
    }

    fn decode_byte_ranges(bytes: &[u8]) -> Vec<(u64, u64)> {
        bytes
            .chunks_exact(16)
            .map(|chunk| {
                let offset = u64::from_le_bytes(chunk[0..8].try_into().unwrap());
                let length = u64::from_le_bytes(chunk[8..16].try_into().unwrap());
                (offset, length)
            })
            .collect()
    }

    fn new_buf_out() -> *mut Buf {
        Box::into_raw(Box::new(Buf {
            ptr: std::ptr::null_mut(),
            len: 0,
        }))
    }

    fn drop_buf_out(out: *mut Buf) {
        unsafe {
            if !(*out).ptr.is_null() {
                free_((*out).ptr, (*out).len);
            }
            drop(Box::from_raw(out));
        }
    }

    fn cv_param(accession: &str, value: Option<&str>) -> CvParam {
        CvParam {
            accession: Some(accession.to_string()),
            name: accession.to_string(),
            value: value.map(str::to_string),
            ..Default::default()
        }
    }

    fn cv_param_with_unit(accession: &str, value: &str, unit_accession: &str) -> CvParam {
        CvParam {
            accession: Some(accession.to_string()),
            name: accession.to_string(),
            value: Some(value.to_string()),
            unit_accession: Some(unit_accession.to_string()),
            ..Default::default()
        }
    }

    fn binary_array(role_accession: &str, values: Vec<f64>) -> BinaryDataArray {
        BinaryDataArray {
            array_length: Some(values.len()),
            cv_params: vec![
                cv_param(role_accession, None),
                cv_param("MS:1000523", None),
                cv_param("MS:1000576", None),
            ],
            numeric_type: Some(NumericType::Float64),
            binary: Some(NumericArray::F64(values)),
            ..Default::default()
        }
    }

    fn spectrum(index: usize, rt: f64) -> Spectrum {
        Spectrum {
            id: format!("scan={}", index + 1),
            index: Some(index as u32),
            default_array_length: Some(4),
            ms_level: Some(1),
            cv_params: vec![cv_param("MS:1000511", Some("1"))],
            scan_list: Some(ScanList {
                count: Some(1),
                scans: vec![Scan {
                    cv_params: vec![cv_param_with_unit(
                        "MS:1000016",
                        &rt.to_string(),
                        "UO:0000031",
                    )],
                    ..Default::default()
                }],
                ..Default::default()
            }),
            binary_data_array_list: Some(BinaryDataArrayList {
                count: Some(2),
                binary_data_arrays: vec![
                    binary_array("MS:1000514", vec![100.0, 499.999, 500.001, 900.0]),
                    binary_array("MS:1000515", vec![10.0, 1000.0, 900.0, 20.0]),
                ],
            }),
            ..Default::default()
        }
    }

    fn sample_mzml() -> MzML {
        MzML {
            run: Run {
                id: "test".to_string(),
                spectrum_list: Some(SpectrumList {
                    count: Some(3),
                    spectra: vec![spectrum(0, 1.0), spectrum(1, 2.0), spectrum(2, 3.0)],
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn build_ion(mz_window: f64, block_size: usize) -> Vec<u8> {
        let options = WriteOptions {
            compression_level: 0,
            force_f32: false,
            block_size,
            parallel: true,
            section_storage: SectionStorage::Memory,
            mz_window,
        };
        let mut bytes = Vec::new();
        write_mzml_to_ion(&sample_mzml(), options, &mut bytes).expect("ion encode should succeed");
        bytes
    }

    fn open_ion(bytes: Vec<u8>) -> *mut ParsedFile {
        let ion = IonReader::from_bytes(&bytes, &ReadOptions::default())
            .expect("IonReader::from_bytes failed");
        Box::into_raw(Box::new(ParsedFile::new(FileSource::Lazy(Box::new(ion)))))
    }

    fn windowed_ion_bytes() -> Vec<u8> {
        build_ion(DEFAULT_MZ_WINDOW, ION_BLOCK_SIZE_BYTES)
    }

    fn open_windowed_ion() -> *mut ParsedFile {
        open_ion(windowed_ion_bytes())
    }

    static mut SERVED_BYTES: usize = 0;

    unsafe extern "C" fn read_from_slice(
        context: *mut c_void,
        offset: u64,
        length: u64,
        dest: *mut u8,
    ) -> i32 {
        let bytes = unsafe { &*(context as *const Vec<u8>) };
        let start = offset as usize;
        let end = start + length as usize;
        if end > bytes.len() {
            return -1;
        }
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr().add(start), dest, length as usize);
            SERVED_BYTES += length as usize;
        }
        0
    }

    #[test]
    fn parse_ion_source_reads_only_what_it_needs() {
        let fixture =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/api/api.ion");
        let bytes = std::fs::read(&fixture).expect("read the api fixture");
        let total = bytes.len();
        unsafe { SERVED_BYTES = 0 };

        let mut handle: *mut ParsedFile = std::ptr::null_mut();
        let code = unsafe {
            parse_ion_source(
                Some(read_from_slice),
                &bytes as *const Vec<u8> as *mut c_void,
                0,
                &mut handle,
            )
        };
        assert_eq!(code, OK, "parse_ion_source failed");
        assert!(!handle.is_null());

        let served = unsafe { SERVED_BYTES };
        assert!(
            served < total,
            "opening pulled {served} of {total} bytes: it is not reading lazily"
        );
        println!("opening read {served} of {total} bytes");

        unsafe { free_mzml(handle) };
    }

    #[test]
    fn parse_ion_source_rejects_a_missing_callback() {
        let mut handle: *mut ParsedFile = std::ptr::null_mut();
        let code = unsafe { parse_ion_source(None, std::ptr::null_mut(), 0, &mut handle) };
        assert_eq!(code, ERR_INVALID_ARGS);
    }

    #[test]
    fn plan_open_rejects_null_header_ptr() {
        let out = new_buf_out();
        let code = unsafe { plan_open(std::ptr::null(), 1024, out) };
        assert_eq!(code, ERR_INVALID_ARGS);
        drop_buf_out(out);
    }

    #[test]
    fn plan_open_rejects_null_out() {
        let code = unsafe { plan_open(b"header".as_ptr(), 6, std::ptr::null_mut()) };
        assert_eq!(code, ERR_INVALID_ARGS);
    }

    #[test]
    fn plan_open_rejects_short_header() {
        let out = new_buf_out();
        let code = unsafe { plan_open(b"x".as_ptr(), 1, out) };
        assert_eq!(code, ERR_FAST_PATH);
        drop_buf_out(out);
    }

    #[test]
    fn plan_open_returns_buffer_divisible_by_16() {
        let out = new_buf_out();
        let header = vec![0u8; 2048];
        let _code = unsafe { plan_open(header.as_ptr(), header.len(), out) };
        let len = unsafe { (*out).len };
        assert_eq!(len % 16, 0);
        drop_buf_out(out);
    }

    #[test]
    fn pack_byte_ranges_writes_little_endian_pairs() {
        let bytes = pack_byte_ranges([(100, 32), (5000, 64)]);

        assert_eq!(bytes.len(), 32);
        assert_eq!(decode_byte_ranges(&bytes), vec![(100, 32), (5000, 64)]);
    }

    #[test]
    fn plan_eic_rejects_null_handle() {
        let out = new_buf_out();
        let code = unsafe { plan_eic(std::ptr::null_mut(), 500.0, 0.0, 10.0, 20.0, 0.005, 0, out) };
        assert_eq!(code, ERR_INVALID_ARGS);
        drop_buf_out(out);
    }

    #[test]
    fn plan_eic_rejects_null_out() {
        let code = unsafe {
            plan_eic(std::ptr::null_mut(), 500.0, 0.0, 10.0, 20.0, 0.005, 0, std::ptr::null_mut())
        };
        assert_eq!(code, ERR_PANIC);
    }

    #[test]
    fn plan_eic_rejects_invalid_target_mz() {
        let handle = open_windowed_ion();
        let out = new_buf_out();
        let code = unsafe { plan_eic(handle, -1.0, 0.0, 10.0, 20.0, 0.005, 0, out) };
        assert_eq!(code, ERR_INVALID_ARGS);
        drop_buf_out(out);
        unsafe { free_mzml(handle) };
    }

    fn get_last_error() -> (String, String, String) {
        let out = new_buf_out();
        unsafe { quantion_last_error(out) };
        let bytes = unsafe { std::slice::from_raw_parts((*out).ptr, (*out).len) }.to_vec();
        drop_buf_out(out);
        let text = String::from_utf8(bytes).expect("utf-8");
        let fields: Vec<&str> = text.split('\u{1f}').collect();
        (fields[1].to_string(), fields[2].to_string(), fields[4].to_string())
    }

    #[test]
    fn plan_eic_gives_the_same_error_as_calculate_eic() {
        let handle = open_windowed_ion();
        let out = new_buf_out();
        unsafe { plan_eic(handle, -1.0, 0.0, 10.0, 20.0, 0.005, 0, out) };
        let (kind, function, message) = get_last_error();
        assert_eq!((kind.as_str(), function.as_str()), ("invalid_argument", "calculate_eic"));
        assert_eq!(message, "mz must be greater than 0, got -1");
        drop_buf_out(out);
        unsafe { free_mzml(handle) };
    }

    #[test]
    fn plan_scans_gives_the_same_error_as_get_scans() {
        let handle = open_windowed_ion();
        let out = new_buf_out();
        unsafe { plan_scans(handle, 0, 3.0, 1.0, 1, 0, out) };
        let (kind, function, message) = get_last_error();
        assert_eq!((kind.as_str(), function.as_str()), ("invalid_argument", "get_scans"));
        assert_eq!(message, "to must be at least from (3), got 1");
        drop_buf_out(out);
        unsafe { free_mzml(handle) };
    }

    #[test]
    fn plan_find_targets_gives_the_same_error_as_find_targets() {
        let handle = open_windowed_ion();
        let out = new_buf_out();
        let mzs = [500.0, -1.0];
        unsafe { plan_find_targets(handle, mzs.as_ptr(), mzs.len(), 0.0, 10.0, 20.0, 0.005, 0, out) };
        let (kind, function, message) = get_last_error();
        assert_eq!((kind.as_str(), function.as_str()), ("invalid_argument", "find_targets"));
        assert_eq!(message, "targets[1].mz must be greater than 0, got -1");
        drop_buf_out(out);
        unsafe { free_mzml(handle) };
    }

    #[test]
    fn plan_eic_rejects_zero_tolerance() {
        let handle = open_windowed_ion();
        let out = new_buf_out();
        let code = unsafe { plan_eic(handle, 500.0, 0.0, 10.0, 0.0, 0.0, 0, out) };
        assert_eq!(code, ERR_INVALID_ARGS);
        drop_buf_out(out);
        unsafe { free_mzml(handle) };
    }

    #[test]
    fn plan_eic_returns_ranges_for_windowed_ion() {
        let handle = open_windowed_ion();
        let out = new_buf_out();

        let code = unsafe { plan_eic(handle, 500.0, 0.0, 10.0, 20.0, 0.005, 0, out) };

        assert_eq!(code, OK);

        let bytes = unsafe { slice::from_raw_parts((*out).ptr, (*out).len).to_vec() };
        assert_eq!(bytes.len() % 16, 0);

        let ranges = decode_byte_ranges(&bytes);
        assert!(!ranges.is_empty());
        assert!(ranges.iter().all(|(_, length)| *length > 0));

        drop_buf_out(out);
        unsafe { free_mzml(handle) };
    }

    #[test]
    fn plan_eic_works_for_wide_window_large_block_ion() {
        let handle = open_ion(build_ion(250.0, 8 * 1024 * 1024));
        let out = new_buf_out();

        let code = unsafe { plan_eic(handle, 500.0, 0.0, 10.0, 20.0, 0.005, 0, out) };

        assert_eq!(code, OK);

        let bytes = unsafe { slice::from_raw_parts((*out).ptr, (*out).len).to_vec() };
        assert_eq!(bytes.len() % 16, 0);

        let ranges = decode_byte_ranges(&bytes);
        assert!(!ranges.is_empty());
        assert!(ranges.iter().all(|(_, length)| *length > 0));

        drop_buf_out(out);
        unsafe { free_mzml(handle) };
    }

    struct RecordingReader {
        data: Vec<u8>,
        reads: std::sync::Mutex<Vec<(u64, u64)>>,
        fail: std::sync::atomic::AtomicBool,
    }

    impl RecordingReader {
        fn new(data: Vec<u8>) -> Self {
            Self {
                data,
                reads: std::sync::Mutex::new(Vec::new()),
                fail: std::sync::atomic::AtomicBool::new(false),
            }
        }

        fn clear_reads(&self) {
            self.reads.lock().unwrap().clear();
        }

        fn recorded(&self) -> Vec<(u64, u64)> {
            self.reads.lock().unwrap().clone()
        }

        fn set_fail(&self, value: bool) {
            self.fail.store(value, std::sync::atomic::Ordering::SeqCst);
        }
    }

    impl RangeReader for RecordingReader {
        fn read(&self, offset: u64, len: u64, dest: &mut [u8]) -> i32 {
            self.reads.lock().unwrap().push((offset, len));
            if self.fail.load(std::sync::atomic::Ordering::SeqCst) {
                return -1;
            }
            let start = offset as usize;
            let end = match start.checked_add(len as usize) {
                Some(value) => value,
                None => return -1,
            };
            if end > self.data.len() || dest.len() < len as usize {
                return -1;
            }
            dest[..len as usize].copy_from_slice(&self.data[start..end]);
            0
        }
    }

    #[test]
    fn eic_compute_reads_are_within_plan() {
        let bytes = windowed_ion_bytes();
        let file_len = bytes.len() as u64;
        let reader = std::sync::Arc::new(RecordingReader::new(bytes));
        let read_source = reader.clone();
        let mut ion = IonReader::new(
            Arc::new(CallbackSource::new(move |range| {
                read_range(&*read_source, range.offset, range.length)
            })) as Arc<dyn ReadBytes>,
            &ReadOptions::default(),
        )
        .expect("open_remote failed");

        let plan = plan_eic_ranges(&mut ion, 500.0, 0.0, 10.0, 20.0, 0.005)
            .expect("plan_eic_ranges failed");
        assert!(!plan.is_empty());

        let planned_bytes: u64 = plan.iter().map(|range| range.length).sum();
        assert!(planned_bytes < file_len);

        reader.clear_reads();

        let eic = calculate_eic_dispatcher(
            &mut EicReader::Ion(&mut ion),
            500.0,
            FromTo {
                from: 0.0,
                to: 10.0,
            },
            CalculateEicOptions {
                ppm_tolerance: 20.0,
                mz_tolerance: 0.005,
                ..Default::default()
            },
        )
        .expect("calculate_eic failed");
        assert!(!eic.y.is_empty());

        let compute_reads = reader.recorded();
        assert!(
            !compute_reads.is_empty(),
            "compute recorded no reads — prefetch check would be vacuous"
        );

        for (offset, len) in &compute_reads {
            let contained = plan
                .iter()
                .any(|r| r.offset <= *offset && offset.saturating_add(*len) <= r.offset + r.length);
            assert!(
                contained,
                "compute read (offset={offset}, len={len}) is not contained in any planned range"
            );
        }
    }

    #[test]
    fn open_remote_fails_cleanly_when_range_read_fails() {
        let reader = std::sync::Arc::new(RecordingReader::new(windowed_ion_bytes()));
        reader.set_fail(true);
        let read_source = reader.clone();
        let result = IonReader::new(
            Arc::new(CallbackSource::new(move |range| {
                read_range(&*read_source, range.offset, range.length)
            })) as Arc<dyn ReadBytes>,
            &ReadOptions::default(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn eic_compute_errors_cleanly_when_data_read_fails() {
        let reader = std::sync::Arc::new(RecordingReader::new(windowed_ion_bytes()));
        let read_source = reader.clone();
        let mut ion = IonReader::new(
            Arc::new(CallbackSource::new(move |range| {
                read_range(&*read_source, range.offset, range.length)
            })) as Arc<dyn ReadBytes>,
            &ReadOptions::default(),
        )
        .expect("open_remote failed");

        plan_eic_ranges(&mut ion, 500.0, 0.0, 10.0, 20.0, 0.005).expect("plan_eic_ranges failed");

        reader.set_fail(true);

        let result = calculate_eic_dispatcher(
            &mut EicReader::Ion(&mut ion),
            500.0,
            FromTo {
                from: 0.0,
                to: 10.0,
            },
            CalculateEicOptions {
                ppm_tolerance: 20.0,
                mz_tolerance: 0.005,
                ..Default::default()
            },
        );
        assert!(result.is_err());
    }

    struct PlannedOnly {
        data: Vec<u8>,
        allowed: Vec<(u64, u64)>,
    }

    impl RangeReader for PlannedOnly {
        fn read(&self, offset: u64, len: u64, dest: &mut [u8]) -> i32 {
            let inside = self
                .allowed
                .iter()
                .any(|(start, size)| offset >= *start && offset + len <= start + size);
            if !inside {
                return -1;
            }
            let start = offset as usize;
            dest[..len as usize].copy_from_slice(&self.data[start..start + len as usize]);
            0
        }
    }

    fn open_serving_only(allowed: Vec<(u64, u64)>, data: Vec<u8>) -> bool {
        let reader = PlannedOnly { data, allowed };
        IonReader::new(
            Arc::new(CallbackSource::new(move |range| {
                read_range(&reader, range.offset, range.length)
            })) as Arc<dyn ReadBytes>,
            &ReadOptions::default(),
        )
        .is_ok()
    }

    fn planned_ranges_for(bytes: &[u8]) -> Vec<(u64, u64)> {
        let out = new_buf_out();
        let code = unsafe { plan_open(bytes.as_ptr(), 1024, out) };
        assert_eq!(code, OK);
        let packed = unsafe { slice::from_raw_parts((*out).ptr, (*out).len).to_vec() };
        drop_buf_out(out);
        decode_byte_ranges(&packed)
    }

    #[test]
    fn planned_ranges_plus_the_trailer_are_enough_to_open() {
        let bytes = windowed_ion_bytes();
        let mut planned = planned_ranges_for(&bytes);
        planned.push((bytes.len() as u64 - 8, 8));
        assert!(
            open_serving_only(planned, bytes),
            "serving the planned ranges plus the trailer still did not open, so something else is unplanned"
        );
    }

    fn spectrum_in_seconds(index: usize, rt: f64) -> Spectrum {
        let mut spectrum = spectrum(index, rt);
        spectrum.scan_list = Some(ScanList {
            count: Some(1),
            scans: vec![Scan {
                cv_params: vec![cv_param_with_unit(
                    "MS:1000016",
                    &rt.to_string(),
                    "UO:0000010",
                )],
                ..Default::default()
            }],
            ..Default::default()
        });
        spectrum
    }

    fn ion_in_seconds() -> Vec<u8> {
        let mzml = MzML {
            run: Run {
                id: "test".to_string(),
                spectrum_list: Some(SpectrumList {
                    count: Some(3),
                    spectra: vec![
                        spectrum_in_seconds(0, 60.0),
                        spectrum_in_seconds(1, 120.0),
                        spectrum_in_seconds(2, 180.0),
                    ],
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        };
        let options = WriteOptions {
            compression_level: 0,
            force_f32: false,
            block_size: ION_BLOCK_SIZE_BYTES,
            parallel: true,
            section_storage: SectionStorage::Memory,
            mz_window: DEFAULT_MZ_WINDOW,
        };
        let mut bytes = Vec::new();
        write_mzml_to_ion(&mzml, options, &mut bytes).expect("ion encode should succeed");
        bytes
    }

    fn reader_for(bytes: Vec<u8>) -> IonReader {
        IonReader::from_bytes(&bytes, &ReadOptions::default()).expect("from_bytes failed")
    }

    #[test]
    fn plan_window_ranges_is_single_range_on_new_files() {
        let mut ion = reader_for(windowed_ion_bytes());
        let plan = plan_window_ranges(&mut ion, 0.0, 10.0, 499.0, 501.0)
            .expect("plan_window_ranges failed");
        assert_eq!(
            plan.len(),
            1,
            "window-major plan should be one run: {plan:?}"
        );
    }

    #[test]
    fn plan_window_ranges_converts_rt_units() {
        let mut ion = reader_for(ion_in_seconds());
        let from_minutes =
            plan_window_ranges(&mut ion, 1.0, 3.0, 499.0, 501.0).expect("minute plan failed");
        let from_seconds = ion
            .eic_byte_ranges(
                ionic::Range {
                    from: 499.0,
                    to: 501.0,
                },
                Some(ionic::Range {
                    from: 60.0,
                    to: 180.0,
                }),
                0,
            )
            .expect("second plan failed");
        assert!(!from_minutes.is_empty());
        assert_eq!(
            from_minutes
                .iter()
                .map(|r| (r.offset, r.length))
                .collect::<Vec<_>>(),
            from_seconds
                .iter()
                .map(|r| (r.offset, r.length))
                .collect::<Vec<_>>()
        );
    }

    fn covers(planned: &[(u64, u64)], range: &ByteRange) -> bool {
        planned.iter().any(|(offset, length)| {
            *offset <= range.offset && offset + length >= range.offset + range.length
        })
    }

    #[test]
    fn plan_open_coalesces() {
        let small = windowed_ion_bytes();
        let planned = planned_ranges_for(&small);
        assert!(
            planned[0].0 == 0 && planned[0].1 > 1024,
            "the header range should merge with the sections next to it: {planned:?}"
        );
        for range in header_ranges(&small[..1024]).expect("header_ranges") {
            assert!(covers(&planned, &range), "{range:?} is not covered");
        }

        let large = std::fs::read("tests/fixtures/test.ion").expect("fixture");
        let planned = planned_ranges_for(&large);
        assert_eq!(planned.len(), 2, "expected header and tail: {planned:?}");
        assert_eq!(planned[0], (0, 1024));
        assert_eq!(planned[1].0 + planned[1].1, large.len() as u64);
        for range in header_ranges(&large[..1024]).expect("header_ranges") {
            assert!(covers(&planned, &range), "{range:?} is not covered");
        }
    }

    #[test]
    fn open_succeeds_using_only_the_ranges_plan_open_asked_for() {
        let bytes = windowed_ion_bytes();
        let planned = planned_ranges_for(&bytes);
        assert!(
            open_serving_only(planned, bytes),
            "plan_open did not list every range the open needs"
        );
    }

    fn scan_queries() -> Vec<(&'static str, ScanQuery)> {
        vec![
            (
                "rt_range",
                ScanQuery::RtRange(FromTo {
                    from: 0.0,
                    to: 10.0,
                }),
            ),
            ("closest_rt", ScanQuery::Rt(5.0)),
            (
                "mz_range",
                ScanQuery::MzRange(FromTo {
                    from: 0.0,
                    to: 1000.0,
                }),
            ),
            ("closest_mz", ScanQuery::Mz(500.0)),
        ]
    }

    fn open_recording_ion(reader: &Arc<RecordingReader>) -> IonReader {
        let read_source = reader.clone();
        IonReader::new(
            Arc::new(CallbackSource::new(move |range| {
                read_range(&*read_source, range.offset, range.length)
            })) as Arc<dyn ReadBytes>,
            &ReadOptions::default(),
        )
        .expect("IonReader::new failed")
    }

    #[test]
    fn scan_reads_are_within_plan() {
        for (name, query) in scan_queries() {
            let reader = Arc::new(RecordingReader::new(windowed_ion_bytes()));
            let mut ion = open_recording_ion(&reader);

            let plan = plan_scan_ranges(&mut ion, query, TimeUnit::MINUTES, 0, &ionic::ByteRangesOptions::default())
                .expect("plan_scan_ranges failed");

            reader.clear_reads();
            let (_, scans) = get_scans_rs(&mut ion, query, TimeUnit::MINUTES, 0);
            if scans.is_empty() {
                assert!(
                    plan.is_empty(),
                    "{name}: nothing was loaded but the plan asked for bytes"
                );
                continue;
            }
            assert!(
                !plan.is_empty(),
                "{name}: scans loaded but the plan is empty"
            );

            let compute_reads = reader.recorded();
            assert!(
                !compute_reads.is_empty(),
                "{name}: compute recorded no reads, the check would be vacuous"
            );

            for (offset, len) in &compute_reads {
                let contained = plan.iter().any(|range| {
                    range.offset <= *offset
                        && offset.saturating_add(*len) <= range.offset + range.length
                });
                assert!(
                    contained,
                    "{name}: read (offset={offset}, len={len}) is outside every planned range"
                );
            }
        }
    }

    struct PlanGate {
        data: Vec<u8>,
        allowed: std::sync::Mutex<Vec<(u64, u64)>>,
        gated: std::sync::atomic::AtomicBool,
    }

    impl PlanGate {
        fn new(data: Vec<u8>) -> Self {
            Self {
                data,
                allowed: std::sync::Mutex::new(Vec::new()),
                gated: std::sync::atomic::AtomicBool::new(false),
            }
        }

        fn allow_only(&self, ranges: Vec<(u64, u64)>) {
            *self.allowed.lock().unwrap() = ranges;
            self.gated.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }

    impl RangeReader for PlanGate {
        fn read(&self, offset: u64, len: u64, dest: &mut [u8]) -> i32 {
            if self.gated.load(std::sync::atomic::Ordering::SeqCst) {
                let allowed = self.allowed.lock().unwrap();
                let inside = allowed
                    .iter()
                    .any(|(start, size)| offset >= *start && offset + len <= start + size);
                if !inside {
                    return -1;
                }
            }
            let start = offset as usize;
            dest[..len as usize].copy_from_slice(&self.data[start..start + len as usize]);
            0
        }
    }

    #[test]
    fn scans_load_using_only_the_planned_ranges() {
        for (name, query) in scan_queries() {
            let bytes = windowed_ion_bytes();
            let reader = Arc::new(RecordingReader::new(bytes.clone()));
            let mut ion = open_recording_ion(&reader);
            let (_, expected) = get_scans_rs(&mut ion, query, TimeUnit::MINUTES, 0);
            if expected.is_empty() {
                continue;
            }

            let gate = Arc::new(PlanGate::new(bytes));
            let read_source = gate.clone();
            let mut served = IonReader::new(
                Arc::new(CallbackSource::new(move |range| {
                    read_range(&*read_source, range.offset, range.length)
                })) as Arc<dyn ReadBytes>,
                &ReadOptions::default(),
            )
            .expect("IonReader::new failed");

            let plan = plan_scan_ranges(&mut served, query, TimeUnit::MINUTES, 0, &ionic::ByteRangesOptions::default())
                .expect("plan_scan_ranges failed");
            gate.allow_only(plan.iter().map(|r| (r.offset, r.length)).collect());

            let (_, got) = get_scans_rs(&mut served, query, TimeUnit::MINUTES, 0);
            assert_eq!(
                got.len(),
                expected.len(),
                "{name}: serving only the planned ranges truncated the result"
            );
        }
    }

    #[test]
    fn plan_scans_rejects_a_bad_query() {
        let out = new_buf_out();
        let handle = open_windowed_ion();
        assert_eq!(
            unsafe { plan_scans(handle, 9, 0.0, 10.0, 0, 0, out) },
            ERR_PANIC
        );
        assert_eq!(
            unsafe { plan_scans(std::ptr::null_mut(), 0, 0.0, 10.0, 0, 0, out) },
            ERR_INVALID_ARGS
        );
        assert_eq!(
            unsafe { plan_scans(handle, 0, 0.0, 10.0, 0, 0, std::ptr::null_mut()) },
            ERR_PANIC
        );
        unsafe { free_mzml(handle) };
        drop_buf_out(out);
    }

    fn noisy_signal() -> Vec<f32> {
        let mut signal = vec![0.0f32; 200];
        for (index, value) in signal.iter_mut().enumerate() {
            *value = 10.0 + ((index % 7) as f32);
        }
        for index in 90..100 {
            signal[index] = 900.0;
        }
        signal
    }

    #[test]
    fn find_noise_level_reports_width_and_intensity() {
        let signal = noisy_signal();
        let expected = find_noise_level_rs(signal.as_slice());
        let mut width = 0usize;
        let mut intensity = 0.0f64;
        let code =
            unsafe { find_noise_level(signal.as_ptr(), signal.len(), &mut width, &mut intensity) };
        assert_eq!(code, OK);
        assert_eq!(width, expected.width);
        assert_eq!(intensity, expected.intensity);
    }

    #[test]
    fn find_noise_level_rejects_missing_outputs() {
        let signal = noisy_signal();
        let mut width = 0usize;
        let mut intensity = 0.0f64;
        assert_eq!(
            unsafe {
                find_noise_level(
                    signal.as_ptr(),
                    signal.len(),
                    std::ptr::null_mut(),
                    &mut intensity,
                )
            },
            ERR_INVALID_ARGS
        );
        assert_eq!(
            unsafe {
                find_noise_level(
                    signal.as_ptr(),
                    signal.len(),
                    &mut width,
                    std::ptr::null_mut(),
                )
            },
            ERR_INVALID_ARGS
        );
        assert_eq!(
            unsafe { find_noise_level(std::ptr::null(), signal.len(), &mut width, &mut intensity) },
            ERR_INVALID_ARGS
        );
        assert_eq!(
            unsafe { find_noise_level(signal.as_ptr(), 0, &mut width, &mut intensity) },
            ERR_INVALID_ARGS
        );
    }

    struct PlannedBytes {
        bytes: Vec<u8>,
        held: std::cell::RefCell<Vec<(u64, u64)>>,
    }

    impl PlannedBytes {
        fn hold(&self, ranges: &[(u64, u64)]) {
            let mut held = self.held.borrow_mut();
            held.extend_from_slice(ranges);
            held.sort();
            let mut merged: Vec<(u64, u64)> = Vec::new();
            for &(offset, length) in held.iter() {
                match merged.last_mut() {
                    Some(last) if offset <= last.0 + last.1 => {
                        last.1 = last.1.max(offset + length - last.0);
                    }
                    _ => merged.push((offset, length)),
                }
            }
            *held = merged;
        }

        fn holds(&self, offset: u64, length: u64) -> bool {
            self.held
                .borrow()
                .iter()
                .any(|&(start, size)| start <= offset && offset + length <= start + size)
        }
    }

    unsafe extern "C" fn read_planned(
        context: *mut c_void,
        offset: u64,
        length: u64,
        dest: *mut u8,
    ) -> i32 {
        let planned = unsafe { &*(context as *const PlannedBytes) };
        if !planned.holds(offset, length) {
            return -1;
        }
        let start = offset as usize;
        unsafe {
            std::ptr::copy_nonoverlapping(planned.bytes.as_ptr().add(start), dest, length as usize)
        };
        0
    }

    fn take_ranges(out: *mut Buf) -> Vec<(u64, u64)> {
        let packed = unsafe { std::slice::from_raw_parts((*out).ptr, (*out).len) }.to_vec();
        drop_buf_out(out);
        packed
            .chunks_exact(16)
            .map(|chunk| {
                (
                    u64::from_le_bytes(chunk[..8].try_into().unwrap()),
                    u64::from_le_bytes(chunk[8..].try_into().unwrap()),
                )
            })
            .collect()
    }

    fn take_bytes(out: *mut Buf) -> Vec<u8> {
        let bytes = if unsafe { (*out).ptr.is_null() } {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts((*out).ptr, (*out).len) }.to_vec()
        };
        drop_buf_out(out);
        bytes
    }

    const PEAK_MZ: f64 = 500.0;
    const PEAK_RT: f64 = 1.0;

    fn peak_spectrum(index: usize) -> Spectrum {
        let rt = index as f64 * 0.01;
        let mz: Vec<f64> = (0..1800).map(|step| 100.0 + step as f64 * 0.5).collect();
        let height = 100_000.0 * (-((rt - PEAK_RT) / 0.05).powi(2)).exp();
        let intensity: Vec<f64> = mz
            .iter()
            .map(|value| {
                if (value - PEAK_MZ).abs() < 1e-9 {
                    10.0 + height
                } else {
                    10.0
                }
            })
            .collect();
        let mut spectrum = spectrum(index, rt);
        spectrum.default_array_length = Some(mz.len());
        spectrum.binary_data_array_list = Some(BinaryDataArrayList {
            count: Some(2),
            binary_data_arrays: vec![
                binary_array("MS:1000514", mz),
                binary_array("MS:1000515", intensity),
            ],
        });
        spectrum
    }

    fn peak_ion() -> Vec<u8> {
        let spectra: Vec<Spectrum> = (0..200).map(peak_spectrum).collect();
        let mzml = MzML {
            run: Run {
                id: "peak".to_string(),
                spectrum_list: Some(SpectrumList {
                    count: Some(spectra.len()),
                    spectra,
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        };
        let options = WriteOptions {
            compression_level: 0,
            force_f32: false,
            block_size: 64 * 1024,
            parallel: true,
            section_storage: SectionStorage::Memory,
            mz_window: DEFAULT_MZ_WINDOW,
        };
        let mut bytes = Vec::new();
        write_mzml_to_ion(&mzml, options, &mut bytes).expect("encode the peak sample");
        bytes
    }

    fn open_planned(planned: &PlannedBytes) -> *mut ParsedFile {
        let header_length = planned.bytes.len().min(1024);
        planned.hold(&[(0, header_length as u64)]);
        let out = new_buf_out();
        let code = unsafe { plan_open(planned.bytes.as_ptr(), header_length, out) };
        assert_eq!(code, OK, "plan_open failed");
        planned.hold(&take_ranges(out));

        let mut handle: *mut ParsedFile = std::ptr::null_mut();
        let code = unsafe {
            parse_ion_source(
                Some(read_planned),
                planned as *const PlannedBytes as *mut c_void,
                0,
                &mut handle,
            )
        };
        assert_eq!(code, OK, "parse_ion_source failed");
        handle
    }

    fn planned_peak_ion() -> PlannedBytes {
        PlannedBytes {
            bytes: peak_ion(),
            held: std::cell::RefCell::new(Vec::new()),
        }
    }

    fn assert_same_as_local(
        plan: impl Fn(*mut ParsedFile, *mut Buf) -> c_int,
        call: impl Fn(*mut ParsedFile) -> (c_int, Vec<u8>),
    ) {
        let planned = planned_peak_ion();
        let remote = open_planned(&planned);
        let out = new_buf_out();
        assert_eq!(plan(remote, out), OK);
        planned.hold(&take_ranges(out));

        let local = open_ion(peak_ion());
        let (remote_code, remote_rows) = call(remote);
        let (local_code, local_rows) = call(local);
        assert_eq!((remote_code, local_code), (OK, OK));
        assert_eq!(remote_rows, local_rows);
        unsafe {
            free_mzml(remote);
            free_mzml(local);
        }
    }

    fn assert_fails_without_plan(call: impl Fn(*mut ParsedFile) -> (c_int, Vec<u8>)) {
        let planned = planned_peak_ion();
        let remote = open_planned(&planned);
        let local = open_ion(peak_ion());
        assert_ne!(call(remote), call(local));
        unsafe {
            free_mzml(remote);
            free_mzml(local);
        }
    }

    fn targets_found(handle: *mut ParsedFile) -> (c_int, Vec<u8>) {
        let (rts, mzs, ranges) = ([PEAK_RT], [PEAK_MZ], [0.2]);
        let out = new_buf_out();
        let code = unsafe {
            find_targets(handle, rts.as_ptr(), mzs.as_ptr(), ranges.as_ptr(), std::ptr::null(), std::ptr::null(), std::ptr::null(), 0, 1, 0.0, 2.0, 20.0, 0.005, 0, std::ptr::null(), 1, out)
        };
        (code, take_bytes(out))
    }

    fn features(handle: *mut ParsedFile) -> (c_int, Vec<u8>) {
        let out = new_buf_out();
        let code = unsafe {
            find_features(handle, 0.0, 2.0, 20.0, 0.005, 0, 495.0, 505.0, 0.005, std::ptr::null(), 1, out)
        };
        (code, take_bytes(out))
    }

    #[test]
    fn find_targets_reads_only_planned_bytes() {
        assert_fails_without_plan(targets_found);
        assert_same_as_local(
            |handle, out| unsafe {
                plan_find_targets(handle, [PEAK_MZ].as_ptr(), 1, 0.0, 2.0, 20.0, 0.005, 0, out)
            },
            targets_found,
        );
    }

    #[test]
    fn find_features_reads_only_planned_bytes() {
        assert_fails_without_plan(features);
        assert_same_as_local(
            |handle, out| unsafe {
                plan_find_features(handle, 0.0, 2.0, 20.0, 0.005, 0, 495.0, 505.0, 0.005, out)
            },
            features,
        );
    }

    #[test]
    fn whole_file_calls_refuse_a_remote_file() {
        let planned = planned_peak_ion();
        let remote = open_planned(&planned);
        let out = new_buf_out();
        assert_eq!(unsafe { write_mzml(remote, std::ptr::null(), 0, out) }, ERR_REMOTE_FILE);
        drop_buf_out(out);
        unsafe { free_mzml(remote) };
    }
}
