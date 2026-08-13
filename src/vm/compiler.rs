// SPDX-License-Identifier: Apache-2.0
use crate::Error;
use wasmtime::{Config, Engine};

/// Compiler type for compiling wasm scripts
#[derive(Default)]
pub struct Compiler {
    bytes: Vec<u8>,
}

impl Compiler {
    /// create a new builder
    #[must_use]
    pub fn new() -> Self {
        Self {
            bytes: Vec::default(),
        }
    }

    /// Initializes the [`Compiler`] with the bytes to execute
    pub fn with_bytes(mut self, bytes: impl AsRef<[u8]>) -> Self {
        self.bytes = bytes.as_ref().to_vec();
        self
    }

    /// Tries to compile the WASM module to precompiled bytecode
    pub fn try_compile(self) -> Result<Vec<u8>, Error> {
        // configure the engine
        let config = Config::default();
        let engine = Engine::new(&config).map_err(Error::from_wasmtime)?;

        // try to compile the script
        engine
            .precompile_module(&self.bytes)
            .map_err(Error::from_wasmtime)
    }
}
