use core::ffi::c_int;
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    path::Path,
    ptr, slice,
};

use super::{
    Buf, ERR_ENCODE, ERR_INVALID_ARGS, ERR_PANIC, OK, clear_buf,
    public::{fail, remember},
    write_buf,
};
use crate::utilities::{
    bridge::{
        Column, QUANTION_PAYLOAD_MODEL_METADATA, QUANTION_SECTION_MODEL_METADATA_KEY_BYTES,
        QUANTION_SECTION_MODEL_METADATA_KEY_STARTS, QUANTION_SECTION_MODEL_METADATA_VALUE_BYTES,
        QUANTION_SECTION_MODEL_METADATA_VALUE_STARTS, build_record_bridge,
    },
    load_model::{Model, ModelInput, load_model},
};

pub struct ModelHandle {
    pub(super) model: Model,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn model_load_path(
    path_ptr: *const u8,
    path_len: usize,
    out: *mut *mut ModelHandle,
    message: *mut Buf,
) -> c_int {
    if path_ptr.is_null() || path_len == 0 || out.is_null() || message.is_null() {
        return fail(ERR_INVALID_ARGS, "load_model");
    }
    let path_bytes = unsafe { slice::from_raw_parts(path_ptr, path_len) };
    let Ok(path) = std::str::from_utf8(path_bytes) else {
        return fail(ERR_INVALID_ARGS, "load_model");
    };
    write_model(out, message, ModelInput::from(Path::new(path)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn model_load_bytes(
    model_ptr: *const u8,
    model_len: usize,
    out: *mut *mut ModelHandle,
    message: *mut Buf,
) -> c_int {
    if model_ptr.is_null() || model_len == 0 || out.is_null() || message.is_null() {
        return fail(ERR_INVALID_ARGS, "load_model");
    }
    let bytes = unsafe { slice::from_raw_parts(model_ptr, model_len) };
    write_model(out, message, ModelInput::from(bytes))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn model_free(model: *mut ModelHandle) {
    if !model.is_null() {
        drop(unsafe { Box::from_raw(model) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn model_metadata(model: *const ModelHandle, out: *mut Buf) -> c_int {
    if model.is_null() || out.is_null() {
        return fail(ERR_INVALID_ARGS, "load_model");
    }
    clear_buf(out);
    let model = unsafe { &(*model).model };
    match catch_unwind(AssertUnwindSafe(|| {
        build_metadata_bridge(model.get_keys(), model.get_values())
    })) {
        Ok(Some(bytes)) => {
            write_buf(out, bytes);
            OK
        }
        Ok(None) => ERR_ENCODE,
        Err(_) => fail(ERR_PANIC, "load_model"),
    }
}

fn build_metadata_bridge(keys: &[String], values: &[String]) -> Option<Box<[u8]>> {
    let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
    let values: Vec<&str> = values.iter().map(String::as_str).collect();
    build_record_bridge(
        QUANTION_PAYLOAD_MODEL_METADATA,
        keys.len() as u64,
        &[
            Column::Text {
                starts_id: QUANTION_SECTION_MODEL_METADATA_KEY_STARTS,
                bytes_id: QUANTION_SECTION_MODEL_METADATA_KEY_BYTES,
                values: &keys,
            },
            Column::Text {
                starts_id: QUANTION_SECTION_MODEL_METADATA_VALUE_STARTS,
                bytes_id: QUANTION_SECTION_MODEL_METADATA_VALUE_BYTES,
                values: &values,
            },
        ],
    )
}

fn write_model(out: *mut *mut ModelHandle, message: *mut Buf, input: ModelInput) -> c_int {
    unsafe { *out = ptr::null_mut() };
    clear_buf(message);
    match catch_unwind(AssertUnwindSafe(|| load_model(input))) {
        Ok(Ok(model)) => {
            unsafe { *out = Box::into_raw(Box::new(ModelHandle { model })) };
            OK
        }
        Ok(Err(error)) => {
            write_buf(message, error.to_string().into_bytes().into_boxed_slice());
            remember(crate::files::read_error("load_model", error))
        }
        Err(_) => fail(ERR_PANIC, "load_model"),
    }
}
