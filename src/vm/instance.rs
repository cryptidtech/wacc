// SPDX-License-Identifier: Apache-2.0
use crate::{error::VmError, vm::Context, Error};
use wasmtime::{Linker, Module, Store};

/// Represents an instance of a WACC containing the options, code, as well as
/// the application state and Wac execution context.
pub struct Instance {
    /// Virtual machine linker
    pub linker: Linker<Context>,

    /// Virtual machine module to execute
    pub module: Module,

    /// Virtual machine store for state
    pub store: Store<Context>,
}

impl Instance {
    /// Executes the instance to completion
    pub fn run(&mut self, fname: &str) -> Result<bool, Error> {
        let instance = self
            .linker
            .instantiate(&mut self.store, &self.module)
            .map_err(|e| VmError::InstantiationError {
                message: format!("failed to instantiate WASM module: {e}"),
            })?;

        let func = instance
            .get_typed_func::<(), i32>(&mut self.store, fname)
            .map_err(|e| VmError::ExecutionError {
                function: fname.to_string(),
                message: format!("failed to get typed function '{fname}': {e}"),
            })?;

        let result = func
            .call(&mut self.store, ())
            .map_err(|e| VmError::ExecutionError {
                function: fname.to_string(),
                message: format!("function '{fname}' execution failed: {e}"),
            })?;

        Ok(result != 0)
    }

    /// Gets the accumulated log data from the context
    #[must_use]
    pub fn log(&self) -> Vec<u8> {
        self.store.data().log.clone()
    }
}
