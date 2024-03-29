use crate::{
    api::{get_string, inc_check_and_fail, WASM_TRUE},
    error::ApiError,
    Context, Error,
};
use std::io::Write;
use wasmtime::{AsContextMut, Caller, FuncType, Linker, Val, ValType::*};

pub(crate) fn add_to_linker<'a>(linker: &mut Linker<Context<'a>>) -> Result<(), Error>
{
    linker
        .func_new("wacc", "_log", FuncType::new([I32, I32], [I32]), log)
        .map_err(|e| ApiError::RegisterApiFailed(e.to_string()))?;
    Ok(())
}

pub(crate) fn log<'a, 'b, 'c>(
    mut caller: Caller<'a, Context<'b>>,
    params: &'c [Val],
    results: &mut [Val],
) -> Result<(), wasmtime::Error>
{
    // get the string parameter
    let log_line = match get_string(&mut caller, params) {
        Ok(s) => s,
        Err(e) => return Ok(inc_check_and_fail(&mut caller, results, &e.to_string())?)
    };

    // write the log entry to the execution context log buffer
    {
        // get the context
        let mut ctx = caller.as_context_mut();
        let context = ctx.data_mut();

        // add the log line
        writeln!(&mut context.log, "{log_line}")?;
    }

    results[0] = WASM_TRUE;
    Ok(())
}
