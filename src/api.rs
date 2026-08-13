// SPDX-License-Identifier: Apache-2.0
pub mod branch;
pub mod check_eq;
pub mod check_preimage;
pub mod check_preimage_value;
pub mod check_signature;
pub mod log;
pub mod push;
pub mod push_value;

use crate::{
    error::ApiError,
    types::{WasmPtr, WasmSize},
    Context, Error,
};
use wasmtime::{AsContextMut, Caller, Engine, Extern, Linker, Val};

pub const WASM_TRUE: Val = Val::I32(1);
pub const WASM_FALSE: Val = Val::I32(0);

/// Add the API functions to the given Linker
pub fn add_to_linker(engine: &Engine, linker: &mut Linker<Context>) -> Result<(), Error> {
    branch::add_to_linker(engine, linker)?;
    check_eq::add_to_linker(engine, linker)?;
    check_preimage::add_to_linker(engine, linker)?;
    check_preimage_value::add_to_linker(engine, linker)?;
    check_signature::add_to_linker(engine, linker)?;
    log::add_to_linker(engine, linker)?;
    push::add_to_linker(engine, linker)?;
    push_value::add_to_linker(engine, linker)?;
    Ok(())
}

/// Reads raw bytes from WASM linear memory at the given offset and length.
///
/// Similar to [`get_string`] but returns raw bytes without UTF-8 conversion.
/// Used by `_push_value` to push binary data onto the parameter stack.
pub fn get_bytes(caller: &mut Caller<'_, Context>, params: &[Val]) -> Result<Vec<u8>, Error> {
    // get the mem
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => {
            return Err(ApiError::MissingExport {
                export_name: "memory".to_string(),
                context: "get_bytes requires linear memory export".to_string(),
            }
            .into())
        }
    };

    // make sure we have enough params
    if params.len() < 2 {
        return Err(ApiError::IncorrectNumberOfParams {
            expected: 2,
            received: params.len(),
        }
        .into());
    }

    // get the memory pointer
    let ptr = match params[0].i32() {
        Some(val) => WasmPtr::from(val),
        _ => {
            return Err(ApiError::InvalidParam {
                param_index: 0,
                reason: "expected i32 pointer parameter".to_string(),
            }
            .into())
        }
    };

    // get the length
    let len = match params[1].i32() {
        Some(val) => {
            let size = WasmSize::from(val);
            // Validate size is reasonable
            match WasmSize::new_checked(size.as_u32()) {
                Some(s) => s,
                None => {
                    return Err(ApiError::InvalidParam {
                        param_index: 1,
                        reason: format!(
                            "length {} exceeds maximum allowed size {}",
                            size.as_u32(),
                            WasmSize::MAX_SIZE
                        ),
                    }
                    .into())
                }
            }
        }
        _ => {
            return Err(ApiError::InvalidParam {
                param_index: 1,
                reason: "expected i32 length parameter".to_string(),
            }
            .into())
        }
    };

    // Check for potential overflow
    if ptr.would_overflow(len) {
        return Err(ApiError::MemoryDecodeError {
            offset: ptr.as_usize(),
            length: len.as_usize(),
            reason: "pointer + length would overflow".to_string(),
        }
        .into());
    }

    // read raw bytes from memory
    let bytes = {
        let mut buf = vec![0u8; len.as_usize()];
        mem.read(&caller, ptr.as_usize(), buf.as_mut_slice())
            .map_err(|e| ApiError::MemoryDecodeError {
                offset: ptr.as_usize(),
                length: len.as_usize(),
                reason: format!("failed to read from linear memory: {e}"),
            })?;
        buf
    };

    Ok(bytes)
}

/// This function takes an offset and length and pulls the associated bytes
/// from the linear memory and returns it as a string
pub fn get_string(caller: &mut Caller<'_, Context>, params: &[Val]) -> Result<String, Error> {
    // get the mem
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => {
            return Err(ApiError::MissingExport {
                export_name: "memory".to_string(),
                context: "get_string requires linear memory export".to_string(),
            }
            .into())
        }
    };

    // make sure we have enough params
    if params.len() < 2 {
        return Err(ApiError::IncorrectNumberOfParams {
            expected: 2,
            received: params.len(),
        }
        .into());
    }

    // get the memory pointer
    let ptr = match params[0].i32() {
        Some(val) => WasmPtr::from(val),
        _ => {
            return Err(ApiError::InvalidParam {
                param_index: 0,
                reason: "expected i32 pointer parameter".to_string(),
            }
            .into())
        }
    };

    // get the length
    let len = match params[1].i32() {
        Some(val) => {
            let size = WasmSize::from(val);
            // Validate size is reasonable
            match WasmSize::new_checked(size.as_u32()) {
                Some(s) => s,
                None => {
                    return Err(ApiError::InvalidParam {
                        param_index: 1,
                        reason: format!(
                            "length {} exceeds maximum allowed size {}",
                            size.as_u32(),
                            WasmSize::MAX_SIZE
                        ),
                    }
                    .into())
                }
            }
        }
        _ => {
            return Err(ApiError::InvalidParam {
                param_index: 1,
                reason: "expected i32 length parameter".to_string(),
            }
            .into())
        }
    };

    // Check for potential overflow
    if ptr.would_overflow(len) {
        return Err(ApiError::MemoryDecodeError {
            offset: ptr.as_usize(),
            length: len.as_usize(),
            reason: "pointer + length would overflow".to_string(),
        }
        .into());
    }

    // decode the string from the memory
    let s = {
        let mut buf = vec![0u8; len.as_usize()];
        mem.read(&caller, ptr.as_usize(), buf.as_mut_slice())
            .map_err(|e| ApiError::MemoryDecodeError {
                offset: ptr.as_usize(),
                length: len.as_usize(),
                reason: format!("failed to read from linear memory: {e}"),
            })?;
        String::from_utf8(buf)?
    };

    Ok(s)
}

/// This function takes
pub fn put_string(
    caller: &mut Caller<'_, Context>,
    s: &str,
    results: &mut [Val],
) -> Result<(), Error> {
    // make sure we have enough params
    if results.len() < 2 {
        return Err(ApiError::IncorrectNumberOfResults {
            expected: 2,
            received: results.len(),
        }
        .into());
    }

    // get the mem
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => {
            return Err(ApiError::MissingExport {
                export_name: "memory".to_string(),
                context: "put_string requires linear memory export".to_string(),
            }
            .into())
        }
    };

    // get the size
    let size = mem.data_size(&caller);

    let write_idx = {
        // get the context
        let mut ctx = caller.as_context_mut();
        let context = ctx.data_mut();

        // increment the write idx
        context.write_idx += s.len();

        // calculate the linear memory write index
        size - context.write_idx - 1
    };

    let write_ptr = WasmPtr::new(write_idx as u32);
    let write_size = WasmSize::new(s.len() as u32);

    // put the offset and length on the stack
    results[0] = Val::I32(write_ptr.as_u32() as i32);
    results[1] = Val::I32(write_size.as_u32() as i32);

    // write the string into linear memory
    mem.write(caller, write_ptr.as_usize(), s.as_bytes())
        .map_err(|e| ApiError::MemoryAccess {
            offset: write_ptr.as_usize(),
            reason: format!("failed to write to linear memory: {e}"),
        })?;

    Ok(())
}
