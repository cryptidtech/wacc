// SPDX-License-Identifier: Apache-2.0
use crate::{api, error::ApiError, Context, Error};
use wasmtime::{AsContextMut, Caller, Engine, FuncType, Linker, Val, ValType::I32};

pub fn add_to_linker(engine: &Engine, linker: &mut Linker<Context>) -> Result<(), Error> {
    linker
        .func_new(
            "wacc",
            "_branch",
            FuncType::new(engine, [I32, I32], [I32, I32]),
            branch,
        )
        .map_err(|e| ApiError::RegisterApiFailed {
            function_name: "_branch".to_string(),
            reason: format!("{e}"),
        })?;
    Ok(())
}

pub fn branch(
    mut caller: Caller<'_, Context>,
    params: &[Val],
    results: &mut [Val],
) -> Result<(), wasmtime::Error> {
    // get the string parameter
    let ret = api::get_string(&mut caller, params);

    let key = {
        // get the context
        let mut ctx = caller.as_context_mut();
        let context = ctx.data_mut();

        // get the full key given the context
        match ret {
            Ok(key) => context.branch(&key),
            Err(e) => {
                context.fail(&e.to_string());
                return Ok(());
            }
        }
    };

    // write the string to linear memory and put the offset and length on the stack
    if let Err(e) = api::put_string(&mut caller, &key, results) {
        let mut ctx = caller.as_context_mut();
        let context = ctx.data_mut();
        context.fail(&e.to_string());
    }

    Ok(())
}
