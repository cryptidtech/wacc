/// Errors created by this library
#[derive(Clone, Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Api error
    #[error(transparent)]
    Api(#[from] ApiError),
    /// Vm error
    #[error(transparent)]
    Vm(#[from] VmError),

    /// Failed to get WASM Memory handle
    #[error("Wasmtime error: {0}")]
    Wasmtime(String),
    /// Utf8 error
    #[error(transparent)]
    Utf8(#[from] std::string::FromUtf8Error),
    /// Custom error message
    #[error("{0}")]
    Custom(String),
}

impl Error {
    /// creates a custom error from a string
    pub fn custom(s: &impl ToString) -> Error {
        Error::Custom(s.to_string())
    }
}

/// Api errors created by this library
#[derive(Clone, Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ApiError {
    /// Missing export
    #[error("missing vm export: {0}")]
    MissingExport(String),
    /// Missing param
    #[error("missing vm function param: {0}")]
    InvalidParam(usize),
    /// Incorrect number of params
    #[error("incorrect number of vm function params; expected {0}, received {1}")]
    IncorrectNumberOfParams(usize, usize),
    /// Couldn't get the data from the Memory
    #[error("failed to get memory value")]
    MemoryDecodeError,
    /// Failed to register a function
    #[error("failed register API function")]
    RegisterApiFailed(String),
}

/// Vm errors created by this library
#[derive(Clone, Debug, thiserror::Error)]
#[non_exhaustive]
pub enum VmError {
    /// Missing VM context
    #[error("Missing VM context")]
    MissingContext,
}
