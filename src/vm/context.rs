use crate::{Pairs, Value};
use wasmtime::StoreLimits;

/// Represents the application state for each instance of a WACC execution.
pub struct Context<'a>
{
    /// The key-value store
    pub pairs: &'a dyn Pairs,
    /// The stack of values
    pub stack: &'a mut Vec<Value>,
    /// The number of times a check_* operation has been executed
    pub check_count: usize,
    /// In-memory buffer to accumulate log messages from scripts
    pub log: Vec<u8>,
    /// The limiter
    pub limiter: StoreLimits,
}
