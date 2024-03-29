use crate::{
    api::{get_string, inc_check_and_fail, WASM_FALSE, WASM_TRUE},
    error::ApiError,
    Context, Error,
};
use wasmtime::{AsContextMut, Caller, FuncType, Linker, Val, ValType::*};

pub(crate) fn add_to_linker<'a>(linker: &mut Linker<Context<'a>>) -> Result<(), Error>
{
    linker
        .func_new("wacc", "_push", FuncType::new([I32, I32], [I32]), push)
        .map_err(|e| ApiError::RegisterApiFailed(e.to_string()))?;
    Ok(())
}

pub(crate) fn push<'a, 'b, 'c>(
    mut caller: Caller<'a, Context<'b>>,
    params: &'c [Val],
    results: &mut [Val],
) -> Result<(), wasmtime::Error>
{
    // any early exit, the result is WASM_FALSE
    results[0] = WASM_FALSE;

    // get the key parameter
    let key = match get_string(&mut caller, params) {
        Ok(s) => s,
        Err(e) => return Ok(inc_check_and_fail(&mut caller, results, &e.to_string())?)
    };

    // Get the context
    let mut ctx = caller.as_context_mut();
    let context = ctx.data_mut();

    // try to look up the key-value pair by key and push the result onto the stack
    match context.pairs.get(&key) {
        Some(v) => context.stack.push(v.into()), // pushes Value::Bin(Vec<u8>)
        None => return Ok(inc_check_and_fail(&mut caller, results, &format!("missing key: {key}"))?)
    }

    // we succeeded
    results[0] = WASM_TRUE;
    Ok(())
}
