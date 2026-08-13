// SPDX-License-Identifier: Apache-2.0
use crate::{api, error::ApiError, Context, Error};
use wasmtime::{AsContextMut, Caller, Engine, FuncType, Linker, Val, ValType::I32};

pub fn add_to_linker(engine: &Engine, linker: &mut Linker<Context>) -> Result<(), Error> {
    linker
        .func_new(
            "wacc",
            "_log",
            FuncType::new(engine, [I32, I32], [I32]),
            log,
        )
        .map_err(|e| ApiError::RegisterApiFailed {
            function_name: "_log".to_string(),
            reason: format!("{e}"),
        })?;
    Ok(())
}

pub fn log(
    mut caller: Caller<'_, Context>,
    params: &[Val],
    results: &mut [Val],
) -> Result<(), wasmtime::Error> {
    // get the string parameter
    let ret = api::get_string(&mut caller, params);

    // get the context
    let mut ctx = caller.as_context_mut();
    let context = ctx.data_mut();

    // add the log line
    results[0] = match ret {
        Ok(log_line) => context.log(&log_line),
        Err(e) => context.fail(&e.to_string()),
    };

    Ok(())
}
