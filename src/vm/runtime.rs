// SPDX-License-Identifier: Apache-2.0

//! Reusable Wasmtime compilation state.

use crate::{
    api,
    error::VmError,
    vm::component::{lock_world, unlock_world},
    Context, Error,
};
use std::sync::{Arc, OnceLock};
use wasmtime::component::{Component, HasSelf, Linker as ComponentLinker};
use wasmtime::{Config, Engine, Linker, Module};

use super::ModuleCache;

/// Shared Wasmtime state used to build isolated WACC instances efficiently.
///
/// The engine, linker definitions, and compiled modules are reusable. Each
/// [`crate::Instance`] still owns a separate store, context, and fuel budget.
///
/// One engine serves both execution models: core WASM modules and WASM
/// components. The component linker registers the typed
/// `cryptid:wacc/host@1.0.0` interface for both script worlds; the worlds
/// import the identical interface set, so either registration satisfies them.
pub struct Runtime {
    engine: Engine,
    linker: Arc<Linker<Context>>,
    component_linker: Arc<ComponentLinker<Context>>,
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
        config.wasm_component_model(true);
        let engine = Engine::new(&config).map_err(Error::from_wasmtime)?;

        let mut linker = Linker::new(&engine);
        api::add_to_linker(&engine, &mut linker)?;

        // Both script worlds import the same typed `cryptid:wacc/host` items,
        // so the second registration overwrites an identical first one;
        // `allow_shadowing` makes that deterministic re-registration legal.
        let mut component_linker = ComponentLinker::new(&engine);
        component_linker.allow_shadowing(true);
        unlock_world::UnlockScript::add_to_linker::<Context, HasSelf<Context>>(
            &mut component_linker,
            |context| context,
        )
        .map_err(Error::from_wasmtime)?;
        lock_world::LockScript::add_to_linker::<Context, HasSelf<Context>>(
            &mut component_linker,
            |context| context,
        )
        .map_err(Error::from_wasmtime)?;

        Ok(Self {
            engine,
            linker: Arc::new(linker),
            component_linker: Arc::new(component_linker),
            modules: ModuleCache::with_capacity(max_entries),
        })
    }

    /// Returns the number of compiled modules currently cached.
    pub fn cached_module_count(&self) -> usize {
        self.modules.len()
    }

    /// Returns the number of compiled components currently cached.
    pub fn cached_component_count(&self) -> usize {
        self.modules.component_len()
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

    pub(crate) fn component_linker(&self) -> ComponentLinker<Context> {
        self.component_linker.as_ref().clone()
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

    /// Compiles a WASM component for repeated instance construction.
    ///
    /// Mirrors [`Self::compile`]: components are cached by the digest of their
    /// source bytes in their own bounded map.
    pub(crate) fn compile_component(&self, bytes: &[u8]) -> Result<Arc<Component>, Error> {
        if let Some(component) = self.modules.get_component_bytes(bytes) {
            return Ok(component);
        }

        let component =
            Component::new(&self.engine, bytes).map_err(|e| VmError::CompilationError {
                message: format!("failed to compile WASM component: {e}"),
            })?;
        let component = Arc::new(component);
        self.modules
            .insert_component_bytes(bytes, Arc::clone(&component));
        Ok(component)
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

    const SIMPLE_COMPONENT: &[u8] = b"(component)";

    #[test]
    fn reuses_compiled_component() {
        let runtime = Runtime::new().unwrap();

        let first = runtime.compile_component(SIMPLE_COMPONENT).unwrap();
        let second = runtime.compile_component(SIMPLE_COMPONENT).unwrap();

        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(runtime.cached_component_count(), 1);
        // component caching leaves the module count untouched
        assert_eq!(runtime.cached_module_count(), 0);
    }

    #[test]
    fn does_not_cache_failed_component_compilation() {
        let runtime = Runtime::new().unwrap();

        assert!(runtime.compile_component(b"not wasm").is_err());
        assert_eq!(runtime.cached_component_count(), 0);
    }
}
