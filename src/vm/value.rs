// SPDX-License-Identifier: Apache-2.0
use std::{fmt, sync::Arc};

/// The values that can be pushed onto the stack
///
/// Values use `Arc` for binary and string data to enable cheap cloning.
/// This significantly reduces allocations when values are pushed/popped
/// from stacks or retrieved from storage.
#[derive(Clone, PartialEq, Eq)]
pub enum Value {
    /// A binary blob value with debugging hint
    Bin {
        /// Arbitrary description of the data for debugging purposes (empty if unused)
        hint: String,
        /// Binary value data (reference-counted for cheap cloning)
        data: Arc<[u8]>,
    },
    /// A printable string value with debugging hint
    Str {
        /// Arbitrary description of the data for debugging purposes (empty if unused)
        hint: String,
        /// String value data (reference-counted for cheap cloning)
        data: Arc<str>,
    },
    /// Success marker
    Success(usize),
    /// Failure marker
    Failure(String),
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Bin { hint, data } => write!(f, "Bin(\"{}\": {} bytes)", hint, data.len()),
            Self::Str { hint, data } => write!(f, "Str(\"{}\": {} bytes)", hint, data.len()),
            Self::Success(n) => write!(f, "Success({n})"),
            Self::Failure(e) => write!(f, "Failure(\"{e}\")"),
        }
    }
}

impl From<&[u8]> for Value {
    fn from(b: &[u8]) -> Self {
        Self::Bin {
            hint: String::new(),
            data: Arc::from(b),
        }
    }
}

impl From<Vec<u8>> for Value {
    fn from(b: Vec<u8>) -> Self {
        Self::Bin {
            hint: String::new(),
            data: Arc::from(b.into_boxed_slice()),
        }
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Self::Str {
            hint: String::new(),
            data: Arc::from(s),
        }
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Self::Str {
            hint: String::new(),
            data: Arc::from(s.into_boxed_str()),
        }
    }
}

impl From<usize> for Value {
    fn from(n: usize) -> Self {
        Self::Success(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_value() {
        let v: Value = "foo".into();
        assert_eq!(
            Value::Str {
                hint: String::new(),
                data: Arc::from("foo"),
            },
            v
        );
    }

    #[test]
    fn test_number_value() {
        let v: Value = 1.into();
        assert_eq!(Value::Success(1), v);
    }
}
