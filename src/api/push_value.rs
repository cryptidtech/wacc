// SPDX-License-Identifier: Apache-2.0
use crate::{api, error::ApiError, Context, Error};
use wasmtime::{AsContextMut, Caller, Engine, FuncType, Linker, Val, ValType::I32};

pub fn add_to_linker(engine: &Engine, linker: &mut Linker<Context>) -> Result<(), Error> {
    linker
        .func_new(
            "wacc",
            "_push_value",
            FuncType::new(engine, [I32, I32], [I32]),
            push_value,
        )
        .map_err(|e| ApiError::RegisterApiFailed {
            function_name: "_push_value".to_string(),
            reason: format!("{e}"),
        })?;
    Ok(())
}

pub fn push_value(
    mut caller: Caller<'_, Context>,
    params: &[Val],
    results: &mut [Val],
) -> Result<(), wasmtime::Error> {
    // read raw bytes from linear memory
    let ret = api::get_bytes(&mut caller, params);

    // get the context
    let mut ctx = caller.as_context_mut();
    let context = ctx.data_mut();

    // push the raw bytes onto the parameter stack
    results[0] = match ret {
        Ok(bytes) => context.push_value(bytes),
        Err(e) => context.fail(&e.to_string()),
    };

    Ok(())
}
