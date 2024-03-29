use crate::{
    api::{get_string, inc_check_and_fail, WASM_FALSE, WASM_TRUE},
    error::ApiError,
    Context, Error, Value,
};
use multihash::{mh, Multihash};
use multiutil::CodecInfo;
use wasmtime::{AsContextMut, Caller, FuncType, Linker, Val, ValType::*};

pub(crate) fn add_to_linker<'a>(linker: &mut Linker<Context<'a>>) -> Result<(), Error>
{
    linker
        .func_new(
            "wacc",
            "_check_preimage",
            FuncType::new([I32, I32], [I32]),
            check_preimage,
        )
        .map_err(|e| ApiError::RegisterApiFailed(e.to_string()))?;
    Ok(())
}

pub(crate) fn check_preimage<'a, 'b, 'c>(
    mut caller: Caller<'a, Context<'b>>,
    params: &'c [Val],
    results: &mut [Val],
) -> Result<(), wasmtime::Error>
{
    // any early exit, the result is WASM_FALSE
    results[0] = WASM_FALSE;

    // get the key parameter
    let key = match get_string(&mut caller, params) {
        Ok(s) => s,
        Err(e) => return Ok(inc_check_and_fail(&mut caller, results, &e.to_string())?)
    };

    // get the context
    let mut ctx = caller.as_context_mut();
    let context = ctx.data_mut();

    // look up the hash and try to decode it
    let hash = {
        match context.pairs.get(&key) {
            Some(v) => match Multihash::try_from(v.as_ref()) {
                Ok(hash) => hash,
                Err(e) => return Ok(inc_check_and_fail(&mut caller, results, &e.to_string())?),
            },
            None => return Ok(inc_check_and_fail(&mut caller, results, &format!("no value associated with {key}"))?)
        }
    };

    'stack: loop {
        match context.stack.last() {
            Some(&Value::Success(_)) => {
                // consume success markers
                let _ = context.stack.pop();
            }
            Some(&Value::Failure(_)) => {
                // consume failure marker as well
                let _ = context.stack.pop();
            }
            None | Some(_) => break 'stack,
        }
    }

    // get the preimage data from the stack
    let preimage = {
        match context.stack.last() {
            Some(Value::Bin(v)) => match mh::Builder::new_from_bytes(hash.codec(), v) {
                Ok(builder) => match builder.try_build() {
                    Ok(hash) => hash,
                    Err(e) => return Ok(inc_check_and_fail(&mut caller, results, &e.to_string())?),
                }
                Err(e) => return Ok(inc_check_and_fail(&mut caller, results, &e.to_string())?),
            },
            _ => return Ok(inc_check_and_fail(&mut caller, results, "no multihash data on stack")?)
        }
    };

    // check that the hashes match
    if hash != preimage {
        return Ok(inc_check_and_fail(&mut caller, results, "preimage doesn't match")?);
    }

    // the hash check passed so pop the argument from the stack
    context.stack.pop();

    // push the SUCCESS marker with the check count
    context.stack.push(context.check_count.into());

    // we succeeded
    results[0] = WASM_TRUE;
    Ok(())
}
