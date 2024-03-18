use crate::{
    api::{get_string_param, WASM_FALSE, WASM_TRUE},
    error::ApiError,
    Context, Error,
};
use wasmtime::{AsContextMut, Caller, FuncType, Linker, Val, ValType::*};

pub(crate) fn add_to_linker<'a, V, E>(linker: &mut Linker<Context<'a, V, E>>) -> Result<(), Error>
where
    V: Default + AsRef<[u8]> + 'static,
    E: std::error::Error + 'static,
{
    linker
        .func_new("wacc", "_push", FuncType::new([I32, I32], [I32]), push)
        .map_err(|e| ApiError::RegisterApiFailed(e.to_string()))?;
    Ok(())
}

pub(crate) fn push<'a, 'b, 'c, 'd, V, E>(
    mut caller: Caller<'a, Context<'b, V, E>>,
    params: &'c [Val],
    results: &'d mut [Val],
) -> Result<(), wasmtime::Error>
where
    V: Default + AsRef<[u8]>,
    E: std::error::Error + 'static,
{
    // get the key parameter
    let key = match get_string_param::<V, E>(&mut caller, params) {
        Ok(s) => s,
        _ => {
            results[0] = WASM_FALSE;
            return Ok(());
        }
    };

    // Get the context
    let mut ctx = caller.as_context_mut();
    let context = ctx.data_mut();

    // try to look up the key-value pair by key
    if context.pairs.get(key.as_str()).is_some() {
        // if it exists in the kvp store, push the key onto the stack
        context.stack.push(key.into());
        results[0] = WASM_TRUE;
    } else {
        // if it doesn't exist, return false
        results[0] = WASM_FALSE;
    }

    Ok(())
}
