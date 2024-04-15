// SPDX-License-Identifier: FSL-1.1
use crate::{
    api,
    error::ApiError,
    Context, Error,
};
use wasmtime::{AsContextMut, Caller, Engine, FuncType, Linker, Val, ValType::*};

pub(crate) fn add_to_linker<'a>(engine: &Engine, linker: &mut Linker<Context<'a>>) -> Result<(), Error>
{
    linker
        .func_new(
            "wacc",
            "_check_signature",
            FuncType::new(engine, [I32, I32], [I32]),
            check_signature,
        )
        .map_err(|e| ApiError::RegisterApiFailed(e.to_string()))?;
    Ok(())
}

pub(crate) fn check_signature<'a, 'b, 'c>(
    mut caller: Caller<'a, Context<'b>>,
    params: &'c [Val],
    results: &mut [Val],
) -> Result<(), wasmtime::Error>
{
    // get the string parameter
    let ret = api::get_string(&mut caller, params);

    // get the context
    let mut ctx = caller.as_context_mut();
    let context = ctx.data_mut();

    // check the preimage
    results[0] = match ret {
        Ok(key) => context.check_signature(&key),
        Err(e) => context.fail(&e.to_string()),
    };

    Ok(())
}
