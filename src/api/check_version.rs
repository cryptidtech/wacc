use crate::{
    api::{inc_check_and_fail, WASM_TRUE},
    error::ApiError,
    Context, Error,
};
use multiutil::Varuint;
use wasmtime::{AsContextMut, Caller, FuncType, Linker, Val, ValType::*};

pub(crate) fn add_to_linker<'a, V, E>(linker: &mut Linker<Context<'a, V, E>>) -> Result<(), Error>
where
    V: Default + AsRef<[u8]> + 'static,
    E: std::error::Error + 'static,
{
    linker
        .func_new(
            "wacc",
            "_check_version",
            FuncType::new([I64], [I32]),
            check_version,
        )
        .map_err(|e| ApiError::RegisterApiFailed(e.to_string()))?;
    Ok(())
}

pub(crate) fn check_version<'a, 'b, 'c, 'd, V, E>(
    mut caller: Caller<'a, Context<'b, V, E>>,
    params: &'c [Val],
    results: &'d mut [Val],
) -> Result<(), wasmtime::Error>
where
    V: Default + AsRef<[u8]>,
    E: std::error::Error + 'static,
{
    let key = "version".to_string();

    // get the context
    let mut ctx = caller.as_context_mut();
    let context = ctx.data_mut();

    // look up the version number
    let version = {
        match context.pairs.get(key.as_str()) {
            Some(v) => match Varuint::<usize>::try_from(v.as_ref()) {
                Ok(v) => v.to_inner(),
                Err(e) => return Ok(inc_check_and_fail(context, results, &e.to_string())?),
            },
            None => {
                return Ok(inc_check_and_fail(
                    context,
                    results,
                    &format!("no value associated with {}", key),
                )?)
            }
        }
    };

    // make sure we have enough params
    if params.len() < 1 {
        return Err(ApiError::IncorrectNumberOfParams(1, params.len()).into());
    }

    // get the expected version
    let expected = match params[0].i64() {
        Some(e) => e as usize,
        _ => return Err(ApiError::InvalidParam(0).into()),
    };

    // check that the hashes match
    if version != expected {
        return Ok(inc_check_and_fail(
            context,
            results,
            &format!("version mismatch {} != {}", version, expected),
        )?);
    }

    // push the SUCCESS marker with the check count
    context.stack.push(context.check_count.into());

    results[0] = WASM_TRUE;
    Ok(())
}
