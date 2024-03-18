use crate::{
    api::{get_string_param, inc_check_and_fail, WASM_TRUE},
    error::ApiError,
    Context, Error, Value,
};
use multihash::{mh, Multihash};
use multiutil::CodecInfo;
use wasmtime::{AsContextMut, Caller, FuncType, Linker, Val, ValType::*};

pub(crate) fn add_to_linker<'a, V, E>(linker: &mut Linker<Context<'a, V, E>>) -> Result<(), Error>
where
    V: Default + AsRef<[u8]> + 'static,
    E: std::error::Error + 'static,
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

pub(crate) fn check_preimage<'a, 'b, 'c, 'd, V, E>(
    mut caller: Caller<'a, Context<'b, V, E>>,
    params: &'c [Val],
    results: &'d mut [Val],
) -> Result<(), wasmtime::Error>
where
    V: Default + AsRef<[u8]>,
    E: std::error::Error + 'static,
{
    // get the key parameter
    let key = match get_string_param::<V, E>(&mut caller, params) {
        Ok(s) => s,
        Err(e) => {
            let mut ctx = caller.as_context_mut();
            let context = ctx.data_mut();
            return Ok(inc_check_and_fail(context, results, &e.to_string())?);
        }
    };

    // get the context
    let mut ctx = caller.as_context_mut();
    let context = ctx.data_mut();

    // look up the hash and try to decode it
    let hash = {
        match context.pairs.get(key.as_str()) {
            Some(v) => match Multihash::try_from(v.as_ref()) {
                Ok(hash) => hash,
                Err(e) => return Ok(inc_check_and_fail(context, results, &e.to_string())?),
            },
            None => {
                return Ok(inc_check_and_fail(
                    context,
                    results,
                    &format!("no value associated with {}", key.as_str()),
                )?)
            }
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

    // pop the top item and look it up...this is the preimage
    let preimage = {
        match context.stack.last() {
            Some(Value::Str(k)) => match context.pairs.get(k.as_str()) {
                Some(pi) => pi,
                None => {
                    return Ok(inc_check_and_fail(
                        context,
                        results,
                        &format!("no value associated with {}", key.as_str()),
                    )?)
                }
            },
            _ => {
                return Ok(inc_check_and_fail(
                    context,
                    results,
                    "no key name on stack",
                )?)
            }
        }
    };

    // build a hash from the preimage data using the same codec
    let builder = match mh::Builder::new_from_bytes(hash.codec(), preimage) {
        Ok(b) => b,
        Err(e) => return Ok(inc_check_and_fail(context, results, &e.to_string())?),
    };

    let phash = match builder.try_build() {
        Ok(ph) => ph,
        Err(e) => return Ok(inc_check_and_fail(context, results, &e.to_string())?),
    };

    // check that the hashes match
    if hash != phash {
        return Ok(inc_check_and_fail(
            context,
            results,
            "preimage doesn't match",
        )?);
    }

    // the hash check passed so pop the argument from the stack
    context.stack.pop();

    // push the SUCCESS marker with the check count
    context.stack.push(context.check_count.into());

    results[0] = WASM_TRUE;
    Ok(())
}
