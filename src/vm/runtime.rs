// SPDX-License-Identifier: Apache-2.0

//! Reusable Wasmtime compilation state.

use crate::{api, error::VmError, Context, Error};
use std::sync::{Arc, OnceLock};
use wasmtime::{Config, Engine, Linker, Module};

use super::ModuleCache;

/// Shared Wasmtime state used to build isolated WACC instances efficiently.
///
/// The engine, linker definitions, and compiled modules are reusable. Each
/// [`crate::Instance`] still owns a separate store, context, and fuel budget.
pub struct Runtime {
    engine: Engine,
    linker: Arc<Linker<Context>>,
    modules: ModuleCache,
}

/// A compiled WACC module bound to the runtime that created it.
#[derive(Clone)]
pub struct PreparedModule {
    pub(crate) runtime: Arc<Runtime>,
    pub(crate) module: Arc<Module>,
}

impl Runtime {
    /// Creates a runtime with the default compiled-module cache capacity.
    pub fn new() -> Result<Self, Error> {
        Self::with_cache_capacity(ModuleCache::DEFAULT_MAX_ENTRIES)
    }

    /// Creates a runtime with a specified compiled-module cache capacity.
    pub fn with_cache_capacity(max_entries: usize) -> Result<Self, Error> {
        let mut config = Config::default();
        config.consume_fuel(true);
        let engine = Engine::new(&config).map_err(Error::from_wasmtime)?;

        let mut linker = Linker::new(&engine);
        api::add_to_linker(&engine, &mut linker)?;

        Ok(Self {
            engine,
            linker: Arc::new(linker),
            modules: ModuleCache::with_capacity(max_entries),
        })
    }

    /// Returns the number of compiled modules currently cached.
    pub fn cached_module_count(&self) -> usize {
        self.modules.len()
    }

    /// Compiles and prepares bytecode for repeated instance construction.
    pub fn prepare(self: &Arc<Self>, bytes: &[u8]) -> Result<PreparedModule, Error> {
        Ok(PreparedModule {
            runtime: Arc::clone(self),
            module: self.compile(bytes)?,
        })
    }

    pub(crate) const fn engine(&self) -> &Engine {
        &self.engine
    }

    pub(crate) fn linker(&self) -> Linker<Context> {
        self.linker.as_ref().clone()
    }

    pub(crate) fn compile(&self, bytes: &[u8]) -> Result<Arc<Module>, Error> {
        if let Some(module) = self.modules.get_bytes(bytes) {
            return Ok(module);
        }

        let module = Module::new(&self.engine, bytes).map_err(|e| VmError::CompilationError {
            message: format!("failed to compile WASM module: {e}"),
        })?;
        let module = Arc::new(module);
        self.modules.insert_bytes(bytes, Arc::clone(&module));
        Ok(module)
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new().expect("default Wasmtime configuration must initialize")
    }
}

pub(crate) fn default_runtime() -> Result<Arc<Runtime>, Error> {
    static RUNTIME: OnceLock<Arc<Runtime>> = OnceLock::new();

    if let Some(runtime) = RUNTIME.get() {
        return Ok(Arc::clone(runtime));
    }

    let runtime = Arc::new(Runtime::new()?);
    if RUNTIME.set(Arc::clone(&runtime)).is_ok() {
        Ok(runtime)
    } else {
        Ok(Arc::clone(
            RUNTIME
                .get()
                .expect("another thread initialized the default runtime"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE_WASM: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7f,
        0x03, 0x02, 0x01, 0x00, 0x07, 0x08, 0x01, 0x04, 0x74, 0x65, 0x73, 0x74, 0x00, 0x00, 0x0a,
        0x06, 0x01, 0x04, 0x00, 0x41, 0x01, 0x0b,
    ];

    #[test]
    fn reuses_compiled_module() {
        let runtime = Runtime::new().unwrap();

        let first = runtime.compile(SIMPLE_WASM).unwrap();
        let second = runtime.compile(SIMPLE_WASM).unwrap();

        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(runtime.cached_module_count(), 1);
    }

    #[test]
    fn does_not_cache_failed_compilation() {
        let runtime = Runtime::new().unwrap();

        assert!(runtime.compile(b"not wasm").is_err());
        assert_eq!(runtime.cached_module_count(), 0);
    }

    #[test]
    fn prepared_module_keeps_runtime_and_compilation() {
        let runtime = Arc::new(Runtime::new().unwrap());

        let prepared = runtime.prepare(SIMPLE_WASM).unwrap();

        assert!(Arc::ptr_eq(&runtime, &prepared.runtime));
        assert_eq!(runtime.cached_module_count(), 1);
    }
}
