use crate::{
    api::{get_string_param, inc_check_and_fail, WASM_TRUE},
    error::ApiError,
    Context, Error, Value,
};
use multikey::{Multikey, Views};
use multisig::Multisig;
use wasmtime::{AsContextMut, Caller, FuncType, Linker, Val, ValType::*};

pub(crate) fn add_to_linker<'a, V, E>(linker: &mut Linker<Context<'a, V, E>>) -> Result<(), Error>
where
    V: Default + AsRef<[u8]> + 'static,
    E: std::error::Error + 'static,
{
    linker
        .func_new(
            "wacc",
            "_check_signature",
            FuncType::new([I32, I32], [I32]),
            check_signature,
        )
        .map_err(|e| ApiError::RegisterApiFailed(e.to_string()))?;
    Ok(())
}

pub(crate) fn check_signature<'a, 'b, 'c, 'd, V, E>(
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

    // look up the public key and try to decode it
    let pubkey = {
        match context.pairs.get(key.as_str()) {
            Some(v) => match Multikey::try_from(v.as_ref()) {
                Ok(mk) => mk,
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

    let mut stack_iter = context.stack.iter().rev();

    // pop the top item and verify that it is a Multisig
    let sig = {
        match stack_iter.next() {
            Some(Value::Str(k)) => match context.pairs.get(k.as_str()) {
                Some(s) => match Multisig::try_from(s.as_ref()) {
                    Ok(sig) => sig,
                    Err(e) => return Ok(inc_check_and_fail(context, results, &e.to_string())?),
                },
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

    // pop the top item and verify that it is a binary blob
    let msg = {
        match stack_iter.next() {
            Some(Value::Str(k)) => match context.pairs.get(k.as_str()) {
                Some(msg) => msg,
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

    // verify the signature
    let verify_view = match pubkey.verify_view() {
        Ok(v) => v,
        Err(e) => return Ok(inc_check_and_fail(context, results, &e.to_string())?),
    };

    match verify_view.verify(&sig, Some(msg.as_ref())) {
        Ok(_) => {
            // the signature verification worked so pop the two arguments off
            // of the stack before continuing
            context.stack.pop();
            context.stack.pop();
        }
        Err(e) => return Ok(inc_check_and_fail(context, results, &e.to_string())?),
    }

    // push the SUCCESS marker with the check count
    context.stack.push(context.check_count.into());

    results[0] = WASM_TRUE;
    Ok(())
}
