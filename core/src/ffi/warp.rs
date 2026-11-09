use core::ffi::c_int;
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    ptr,
};

use super::{
    Buf, ERR_ENCODE, ERR_INVALID_ARGS, ERR_PANIC, FileSource, OK, ParsedFile, clear_buf, f64_to_u8,
    file_is_broken, mark_file_broken,
    model::ModelHandle,
    pack_byte_ranges,
    public::{fail, read_targets_at, read_values, remember},
    write_buf,
};
use crate::utilities::{
    bridge::{
        Column, QUANTION_PAYLOAD_WARP_TARGETS, QUANTION_SECTION_WARP_TARGET_ID_BYTES,
        QUANTION_SECTION_WARP_TARGET_ID_STARTS, QUANTION_SECTION_WARP_TARGET_MZ,
        QUANTION_SECTION_WARP_TARGET_RT, build_record_bridge,
    },
    calculate_eic::{EicReader, FastError},
    structs::Target,
    warp_model::{
        WarpError, WarpModel, check_target_lists, get_median_targets, load_warp,
        plan_predict_rts_ranges,
    },
};

pub struct WarpModelHandle {
    warp: WarpModel,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn warp_model_load(
    model: *const ModelHandle,
    out: *mut *mut WarpModelHandle,
    message: *mut Buf,
) -> c_int {
    if model.is_null() || out.is_null() || message.is_null() {
        return fail(ERR_INVALID_ARGS, "load_warp");
    }
    unsafe { *out = ptr::null_mut() };
    clear_buf(message);
    let model = unsafe { &(*model).model };
    match catch_unwind(AssertUnwindSafe(|| load_warp(model))) {
        Ok(Ok(warp)) => {
            unsafe { *out = Box::into_raw(Box::new(WarpModelHandle { warp })) };
            OK
        }
        Ok(Err(error)) => write_error("load_warp", message, error),
        Err(_) => fail(ERR_PANIC, "load_warp"),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn warp_model_free(warp: *mut WarpModelHandle) {
    if !warp.is_null() {
        drop(unsafe { Box::from_raw(warp) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn predict_rts_from_file(
    warp: *const WarpModelHandle,
    h: *const ParsedFile,
    mzs_ptr: *const f64,
    rts_ptr: *const f64,
    n: usize,
    out: *mut Buf,
) -> c_int {
    if warp.is_null() || h.is_null() || mzs_ptr.is_null() || rts_ptr.is_null() || out.is_null() {
        return fail(ERR_INVALID_ARGS, "predict_rts");
    }
    if file_is_broken(h) {
        return fail(ERR_PANIC, "predict_rts");
    }
    clear_buf(out);

    match catch_unwind(AssertUnwindSafe(|| -> Result<(), c_int> {
        let file = unsafe { &mut *(h as *mut ParsedFile) };
        let targets = read_targets_at(mzs_ptr, rts_ptr, n);
        let mut reader = match file.source_mut() {
            FileSource::Full(mzml) => EicReader::Mzml(mzml.as_mut()),
            FileSource::Lazy(ion) => EicReader::Ion(ion.as_mut()),
            FileSource::Remote(ion) => EicReader::Ion(ion.as_mut()),
        };

        let predicted = unsafe { &(*warp).warp }
            .predict_rts_from_reader(&mut reader, &targets)
            .map_err(|error| write_error("predict_rts", out, error))?;
        let values: Vec<f64> = predicted
            .iter()
            .flat_map(|prediction| [prediction.rt, prediction.uncertainty])
            .collect();
        write_buf(out, f64_to_u8(&values));
        Ok(())
    })) {
        Ok(Ok(())) => OK,
        Ok(Err(code)) => code,
        Err(_) => {
            mark_file_broken(h);
            fail(ERR_PANIC, "predict_rts")
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn plan_predict_rts(
    warp: *const WarpModelHandle,
    h: *mut ParsedFile,
    mzs_ptr: *const f64,
    rts_ptr: *const f64,
    n: usize,
    out: *mut Buf,
) -> c_int {
    if warp.is_null() || h.is_null() || mzs_ptr.is_null() || rts_ptr.is_null() || out.is_null() {
        return fail(ERR_INVALID_ARGS, "predict_rts");
    }
    if file_is_broken(h) {
        return fail(ERR_PANIC, "predict_rts");
    }
    clear_buf(out);

    match catch_unwind(AssertUnwindSafe(|| -> Result<(), c_int> {
        let file = unsafe { &mut *h };
        let targets = read_targets_at(mzs_ptr, rts_ptr, n);
        let warp = unsafe { &(*warp).warp };
        let ranges = match file.source_mut() {
            FileSource::Lazy(ion) => plan_predict_rts_ranges(warp, ion.as_mut(), &targets),
            FileSource::Remote(ion) => plan_predict_rts_ranges(warp, ion.as_mut(), &targets),
            FileSource::Full(_) => Err(WarpError::FileNotRead(FastError::UnsupportedBackend)),
        }
        .map_err(|error| write_error("predict_rts", out, error))?;
        write_buf(
            out,
            pack_byte_ranges(ranges.iter().map(|range| (range.offset, range.length))),
        );
        Ok(())
    })) {
        Ok(Ok(())) => OK,
        Ok(Err(code)) => code,
        Err(_) => {
            mark_file_broken(h);
            fail(ERR_PANIC, "predict_rts")
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn calibrate_rts(
    warp: *const WarpModelHandle,
    files: *const *const ParsedFile,
    file_count: usize,
    list_lengths: *const usize,
    list_count: usize,
    ids_off: *const u32,
    ids_len: *const u32,
    ids_buf: *const u8,
    ids_buf_len: usize,
    mzs_ptr: *const f64,
    rts_ptr: *const f64,
    target_count: usize,
    out: *mut Buf,
) -> c_int {
    if warp.is_null() || out.is_null() {
        return fail(ERR_INVALID_ARGS, "calibrate");
    }
    if (file_count > 0 && files.is_null()) || (list_count > 0 && list_lengths.is_null()) {
        return fail(ERR_INVALID_ARGS, "calibrate");
    }
    if target_count > 0
        && (ids_off.is_null() || ids_len.is_null() || mzs_ptr.is_null() || rts_ptr.is_null())
    {
        return fail(ERR_INVALID_ARGS, "calibrate");
    }
    clear_buf(out);
    let files = read_values(files, file_count);
    let lengths = read_values(list_lengths, list_count);
    if lengths.iter().sum::<usize>() != target_count {
        return fail(ERR_INVALID_ARGS, "calibrate");
    }
    if files
        .iter()
        .any(|file| file.is_null() || file_is_broken(*file))
    {
        return remember(crate::QuantionError::invalid(
            "calibrate",
            "anchor_samples",
            "must all be open; one was disposed or never opened",
        ));
    }

    let warp = unsafe { &(*warp).warp };
    let mut targets = read_targets_at(mzs_ptr, rts_ptr, target_count);
    match get_ids(ids_off, ids_len, ids_buf, ids_buf_len, target_count) {
        Some(ids) => targets
            .iter_mut()
            .zip(ids)
            .for_each(|(target, id)| target.id = (!id.is_empty()).then_some(id)),
        None => return fail(ERR_INVALID_ARGS, "calibrate"),
    }
    let target_lists = split_targets(targets, lengths);
    if let Err(error) = check_target_lists(files.len(), &target_lists) {
        return write_error("calibrate", out, error);
    }

    let mut calibrated_lists = Vec::with_capacity(files.len());
    for (file, targets) in files.iter().zip(&target_lists) {
        match catch_unwind(AssertUnwindSafe(|| {
            let parsed = unsafe { &mut *(*file as *mut ParsedFile) };
            let mut reader = match parsed.source_mut() {
                FileSource::Full(mzml) => EicReader::Mzml(mzml.as_mut()),
                FileSource::Lazy(ion) => EicReader::Ion(ion.as_mut()),
                FileSource::Remote(ion) => EicReader::Ion(ion.as_mut()),
            };
            warp.calibrate_sample(&mut reader, targets)
        })) {
            Ok(Ok(calibrated)) => calibrated_lists.push(calibrated),
            Ok(Err(error)) => return write_error("calibrate", out, error),
            Err(_) => {
                mark_file_broken(*file);
                return fail(ERR_PANIC, "calibrate");
            }
        }
    }

    match build_targets_bridge(&get_median_targets(&calibrated_lists)) {
        Some(bytes) => {
            write_buf(out, bytes);
            OK
        }
        None => ERR_ENCODE,
    }
}

fn split_targets(targets: Vec<Target>, lengths: &[usize]) -> Vec<Vec<Target>> {
    let mut rest = targets.into_iter();
    lengths
        .iter()
        .map(|length| rest.by_ref().take(*length).collect())
        .collect()
}

fn build_targets_bridge(targets: &[Target]) -> Option<Box<[u8]>> {
    let ids: Vec<&str> = targets.iter().map(|target| target.get_id()).collect();
    let mz: Vec<f64> = targets.iter().map(|target| target.mz).collect();
    let rt: Vec<f64> = targets.iter().map(|target| target.rt).collect();
    build_record_bridge(
        QUANTION_PAYLOAD_WARP_TARGETS,
        targets.len() as u64,
        &[
            Column::Text {
                starts_id: QUANTION_SECTION_WARP_TARGET_ID_STARTS,
                bytes_id: QUANTION_SECTION_WARP_TARGET_ID_BYTES,
                values: &ids,
            },
            Column::Numbers {
                id: QUANTION_SECTION_WARP_TARGET_MZ,
                values: &mz,
            },
            Column::Numbers {
                id: QUANTION_SECTION_WARP_TARGET_RT,
                values: &rt,
            },
        ],
    )
}

fn get_ids(
    ids_off: *const u32,
    ids_len: *const u32,
    ids_buf: *const u8,
    ids_buf_len: usize,
    n: usize,
) -> Option<Vec<String>> {
    let offsets = read_values(ids_off, n);
    let lengths = read_values(ids_len, n);
    let bytes = if ids_buf.is_null() {
        &[][..]
    } else {
        read_values(ids_buf, ids_buf_len)
    };
    offsets
        .iter()
        .zip(lengths)
        .map(|(offset, length)| {
            let start = *offset as usize;
            let id = bytes.get(start..start.checked_add(*length as usize)?)?;
            std::str::from_utf8(id).ok().map(str::to_string)
        })
        .collect()
}

fn write_error(function: &'static str, message: *mut Buf, error: WarpError) -> c_int {
    write_buf(message, error.to_string().into_bytes().into_boxed_slice());
    remember(crate::models::from_warp_error(function, error))
}
