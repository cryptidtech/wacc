pub(crate) mod check_preimage;
pub(crate) mod check_signature;
pub(crate) mod check_version;
pub(crate) mod log;
pub(crate) mod pop;
pub(crate) mod push;

use crate::{error::ApiError, Context, Error, Value};
use wasmtime::{Caller, Extern, Linker, Val};

pub const WASM_TRUE: Val = Val::I32(1);
pub const WASM_FALSE: Val = Val::I32(0);

/// Add the API functions to the given Linker
pub(crate) fn add_to_linker<'a, V, E>(linker: &mut Linker<Context<'a, V, E>>) -> Result<(), Error>
where
    V: Default + AsRef<[u8]> + 'static,
    E: std::error::Error + 'static,
{
    check_preimage::add_to_linker::<V, E>(linker)?;
    check_signature::add_to_linker::<V, E>(linker)?;
    check_version::add_to_linker::<V, E>(linker)?;
    log::add_to_linker::<V, E>(linker)?;
    pop::add_to_linker::<V, E>(linker)?;
    push::add_to_linker::<V, E>(linker)?;
    Ok(())
}

/// This function takes an offset and length and pulls the associated bytes
/// from the linear memory and returns it as a string
pub(crate) fn get_string_param<'a, 'b, 'c, V, E>(
    caller: &mut Caller<'a, Context<'b, V, E>>,
    params: &'c [Val],
) -> Result<String, Error>
where
    V: Default + AsRef<[u8]>,
    E: std::error::Error + 'static,
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
        Some(ptr) => ptr,
        _ => return Err(ApiError::InvalidParam(0).into()),
    };

    // get the length
    let len = match params[1].i32() {
        Some(len) => len,
        _ => return Err(ApiError::InvalidParam(1).into()),
    };

    // decode the string from the memory
    let string = {
        let data = match mem
            .data(&caller)
            .get(ptr as u32 as usize..)
            .and_then(|arr| arr.get(..len as u32 as usize))
        {
            Some(d) => d.to_vec(),
            _ => return Err(ApiError::MemoryDecodeError.into()),
        };
        String::from_utf8(data)?
    };

    Ok(string)
}

pub(crate) fn inc_check_and_fail<'a, 'b, V, E>(
    context: &mut Context<'a, V, E>,
    results: &'b mut [Val],
    err: &str,
) -> Result<(), Error>
where
    V: Default + AsRef<[u8]>,
    E: std::error::Error + 'static,
{
    context.check_count += 1;
    context.stack.push(Value::Failure(err.to_string()));
    results[0] = WASM_FALSE;
    Ok(())
}
