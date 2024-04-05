// SPDX-License-Identifier: FSL-1.1
pub(crate) mod check_preimage;
pub(crate) mod check_signature;
pub(crate) mod log;
pub(crate) mod push;

use crate::{error::ApiError, Context, Error, Key};
use wasmtime::{Caller, Extern, Linker, Val};

pub const WASM_TRUE: Val = Val::I32(1);
pub const WASM_FALSE: Val = Val::I32(0);

/// Add the API functions to the given Linker
pub(crate) fn add_to_linker<'a>(linker: &mut Linker<Context<'a>>) -> Result<(), Error>
{
    check_preimage::add_to_linker(linker)?;
    check_signature::add_to_linker(linker)?;
    log::add_to_linker(linker)?;
    push::add_to_linker(linker)?;
    Ok(())
}

/// This function takes an offset and length and pulls the associated bytes
/// from the linear memory and returns it as a string
pub(crate) fn get_string<'a, 'b, 'c>(
    caller: &mut Caller<'a, Context<'b>>,
    params: &'c [Val],
) -> Result<String, Error>
{
    // get the mem
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return Err(ApiError::MissingExport("memory".to_string()).into()),
    };

    // make sure we have enough params
    if params.len() < 2 {
        return Err(ApiError::IncorrectNumberOfParams(2, params.len()).into());
    }

    // get the memory index
    let ptr = match params[0].i32() {
        Some(ptr) => ptr as u32 as usize,
        _ => return Err(ApiError::InvalidParam(0).into()),
    };

    // get the length
    let len = match params[1].i32() {
        Some(len) => len as u32 as usize,
        _ => return Err(ApiError::InvalidParam(1).into()),
    };

    // decode the string from the memory
    let s = {
        let mut buf = Vec::<u8>::with_capacity(len);
        buf.resize(len, 0u8);
        mem.read(&caller, ptr, buf.as_mut_slice())
            .map_err(|_| ApiError::MemoryDecodeError)?;
        String::from_utf8(buf)?
    };

    Ok(s)
}

/// This function gets a string and tries to construct a Key from it 
pub(crate) fn get_key<'a, 'b, 'c>(
    caller: &mut Caller<'a, Context<'b>>,
    params: &'c [Val],
) -> Result<Key, Error>
{
    match get_string(caller, params) {
        Ok(s) => Key::try_from(s),
        Err(e) => Err(e),
    }
}
