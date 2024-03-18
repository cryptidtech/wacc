use crate::{vm::Context, Error};
use wasmtime::{Linker, Module, Store};

/// Represents an instance of a WACC containing the options, code, as well as
/// the application state and Wac execution context.
pub struct Instance<'a, V, E>
where
    V: Default + AsRef<[u8]>,
    E: std::error::Error,
{
    /// Virtual machine linker
    pub linker: Linker<Context<'a, V, E>>,

    /// Virtual machine module to execute
    pub module: Module,

    /// Virtual machine store for state
    pub store: Store<Context<'a, V, E>>,
}

impl<'a, V, E> Instance<'a, V, E>
where
    V: Default + AsRef<[u8]>,
    E: std::error::Error,
{
    /// Executes the instance to completion
    pub fn run(&mut self, fname: &str) -> Result<bool, Error> {
        let instance = self
            .linker
            .instantiate(&mut self.store, &self.module)
            .map_err(|e| Error::Wasmtime(e.to_string()))?;
        let func = instance
            .get_typed_func::<(), i32>(&mut self.store, fname)
            .map_err(|e| Error::Wasmtime(e.to_string()))?;
        Ok(func
            .call(&mut self.store, ())
            .map_err(|e| Error::Wasmtime(e.to_string()))?
            != 0)
    }

    /// Gets the accumulated log data from the context
    pub fn log(&self) -> Vec<u8> {
        self.store.data().log.clone()
    }
}
