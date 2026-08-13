// SPDX-License-Identifier: Apache-2.0
use crate::{api, error::ApiError, Context, Error};
use log::info;
use wasmtime::{AsContextMut, Caller, Engine, FuncType, Linker, Val, ValType::I32};

pub fn add_to_linker(engine: &Engine, linker: &mut Linker<Context>) -> Result<(), Error> {
    linker
        .func_new(
            "wacc",
            "_check_preimage_value",
            FuncType::new(engine, [I32, I32, I32, I32], [I32]),
            check_preimage_value,
        )
        .map_err(|e| ApiError::RegisterApiFailed {
            function_name: "_check_preimage_value".to_string(),
            reason: format!("{e}"),
        })?;
    Ok(())
}

pub fn check_preimage_value(
    mut caller: Caller<'_, Context>,
    params: &[Val],
    results: &mut [Val],
) -> Result<(), wasmtime::Error> {
    // check preconditions
    if params.len() != 4 {
        let mut ctx = caller.as_context_mut();
        let context = ctx.data_mut();
        results[0] = context.fail("check_preimage_value requires hash and key-path parameters");
        return Ok(());
    }

    // get the hash bytes and key-path string pairs from WASM memory
    let (h, k) = params.split_at(2);
    info!("check_preimage_value: {h:?}, {k:?}");

    // read hash bytes from WASM memory
    let hash_bytes = match api::get_bytes(&mut caller, h) {
        Ok(b) => b,
        Err(e) => {
            let mut ctx = caller.as_context_mut();
            let context = ctx.data_mut();
            results[0] = context.fail(&e.to_string());
            return Ok(());
        }
    };

    // read KVP path string from WASM memory
    let key = match api::get_string(&mut caller, k) {
        Ok(s) => s,
        Err(e) => {
            let mut ctx = caller.as_context_mut();
            let context = ctx.data_mut();
            results[0] = context.fail(&e.to_string());
            return Ok(());
        }
    };

    // call context method
    let mut ctx = caller.as_context_mut();
    let context = ctx.data_mut();
    results[0] = context.check_preimage_value(&hash_bytes, &key);

    Ok(())
}
