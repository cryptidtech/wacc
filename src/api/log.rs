use crate::{
    api,
    error::ApiError,
    Context, Error,
};
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
