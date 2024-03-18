use crate::{api::WASM_TRUE, error::ApiError, Context, Error};
use wasmtime::{AsContextMut, Caller, FuncType, Linker, Val, ValType::*};

pub(crate) fn add_to_linker<'a, V, E>(linker: &mut Linker<Context<'a, V, E>>) -> Result<(), Error>
where
    V: Default + AsRef<[u8]> + 'static,
    E: std::error::Error + 'static,
{
    linker
        .func_new("wacc", "_pop", FuncType::new([], [I32]), pop)
        .map_err(|e| ApiError::RegisterApiFailed(e.to_string()))?;
    Ok(())
}

pub(crate) fn pop<'a, 'b, 'c, 'd, V, E>(
    mut caller: Caller<'a, Context<'b, V, E>>,
    _params: &'c [Val],
    results: &'d mut [Val],
) -> Result<(), wasmtime::Error>
where
    V: Default + AsRef<[u8]>,
    E: std::error::Error + 'static,
{
    // Get the context
    let mut ctx = caller.as_context_mut();
    let context = ctx.data_mut();

    // pop the top off of the stack
    context.stack.pop();

    results[0] = WASM_TRUE;
    Ok(())
}
