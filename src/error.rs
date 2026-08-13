// SPDX-License-Identifier: Apache-2.0

/// Errors created by this library
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Api error
    #[error(transparent)]
    Api(#[from] ApiError),

    /// Vm error
    #[error(transparent)]
    Vm(#[from] VmError),

    /// Wasmtime runtime error
    #[error("Wasmtime error: {message}")]
    Wasmtime {
        /// Error message (may include source error details)
        message: String,
    },

    /// UTF-8 conversion error
    #[error(transparent)]
    Utf8(#[from] std::string::FromUtf8Error),

    /// Custom error message with context
    #[error("{message}")]
    Custom {
        /// Error message
        message: String,
    },
}

impl Error {
    /// Creates a custom error from a string with backtrace
    pub fn custom(s: &impl ToString) -> Self {
        Self::Custom {
            message: s.to_string(),
        }
    }

    /// Creates a Wasmtime error with backtrace
    pub fn wasmtime(message: impl Into<String>) -> Self {
        Self::Wasmtime {
            message: message.into(),
        }
    }

    /// Creates a Wasmtime error from a `wasmtime::Error` with backtrace
    #[must_use]
    pub fn from_wasmtime(error: wasmtime::Error) -> Self {
        Self::Wasmtime {
            message: error.to_string(),
        }
    }

    /// Adds context to the error
    pub fn context(self, ctx: impl Into<String>) -> Self {
        match self {
            Self::Custom { message } => Self::Custom {
                message: format!("{}: {}", ctx.into(), message),
            },
            other => other,
        }
    }
}

/// Api errors created by this library
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ApiError {
    /// Missing export from WASM module
    #[error("missing vm export '{export_name}': {context}")]
    MissingExport {
        /// Name of the missing export
        export_name: String,
        /// Additional context
        context: String,
    },

    /// Invalid function parameter
    #[error("invalid vm function parameter at index {param_index}: {reason}")]
    InvalidParam {
        /// Parameter index that was invalid
        param_index: usize,
        /// Reason for invalidity
        reason: String,
    },

    /// Incorrect number of parameters
    #[error("incorrect number of vm function params; expected {expected}, received {received}")]
    IncorrectNumberOfParams {
        /// Expected number of parameters
        expected: usize,
        /// Received number of parameters
        received: usize,
    },

    /// Incorrect number of results
    #[error("incorrect number of vm function results; expected {expected}, received {received}")]
    IncorrectNumberOfResults {
        /// Expected number of results
        expected: usize,
        /// Received number of results
        received: usize,
    },

    /// Failed to decode memory value
    #[error("failed to get memory value at offset {offset}, length {length}: {reason}")]
    MemoryDecodeError {
        /// Memory offset
        offset: usize,
        /// Length to read
        length: usize,
        /// Reason for failure
        reason: String,
    },

    /// Failed to register API function
    #[error("failed to register API function '{function_name}': {reason}")]
    RegisterApiFailed {
        /// Function name
        function_name: String,
        /// Reason for failure
        reason: String,
    },

    /// Missing key-value pair
    #[error("no value associated with key '{key}' in {store}")]
    NoValue {
        /// The key that was not found
        key: String,
        /// Which store (current/proposed)
        store: String,
    },

    /// Memory access error
    #[error("memory access error at offset {offset}: {reason}")]
    MemoryAccess {
        /// Memory offset
        offset: usize,
        /// Reason for error
        reason: String,
    },
}

/// Vm errors created by this library
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum VmError {
    /// Missing VM context during operation
    #[error("missing VM context: {context}")]
    MissingContext {
        /// Context about where the error occurred
        context: String,
    },

    /// Invalid key-path for the key-value store
    #[error("invalid key-path '{path}': {reason}")]
    InvalidKeyPath {
        /// The invalid key-path
        path: String,
        /// Reason why it's invalid
        reason: String,
    },

    /// Compilation error
    #[error("compilation error: {message}")]
    CompilationError {
        /// Error message (includes source error details)
        message: String,
    },

    /// Instantiation error
    #[error("instantiation error: {message}")]
    InstantiationError {
        /// Error message (includes source error details)
        message: String,
    },

    /// Execution error
    #[error("execution error in function '{function}': {message}")]
    ExecutionError {
        /// Function name
        function: String,
        /// Error message (includes source error details)
        message: String,
    },
}
