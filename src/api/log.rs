use crate::{
    api::{get_string_param, WASM_FALSE, WASM_TRUE},
    error::ApiError,
    Context, Error,
};
use std::io::Write;
use wasmtime::{AsContextMut, Caller, FuncType, Linker, Val, ValType::*};

pub(crate) fn add_to_linker<'a, V, E>(linker: &mut Linker<Context<'a, V, E>>) -> Result<(), Error>
where
    V: Default + AsRef<[u8]> + 'static,
    E: std::error::Error + 'static,
{
    linker
        .func_new("wacc", "_log", FuncType::new([I32, I32], [I32]), log)
        .map_err(|e| ApiError::RegisterApiFailed(e.to_string()))?;
    Ok(())
}

pub(crate) fn log<'a, 'b, 'c, 'd, V, E>(
    mut caller: Caller<'a, Context<'b, V, E>>,
    params: &'c [Val],
    results: &'d mut [Val],
) -> Result<(), wasmtime::Error>
where
    V: Default + AsRef<[u8]>,
    E: std::error::Error + 'static,
{
    // get the string parameter
    let string = match get_string_param::<V, E>(&mut caller, params) {
        Ok(s) => s,
        _ => {
            results[0] = WASM_FALSE;
            return Ok(());
        }
    };

    // write the log entry to the execution context log buffer
    let mut ctx = caller.as_context_mut();
    let context = ctx.data_mut();
    writeln!(&mut context.log, "{string}")?;

    results[0] = WASM_TRUE;
    Ok(())
}
