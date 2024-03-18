use std::fmt;

/// The values that can be pushed onto the stack
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// A binary blob on the stack
    Bin(Vec<u8>),
    /// A printable string value
    Str(String),
    /// Sucess marker
    Success(usize),
    /// Failure marker
    Failure(String),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Bin(b) => write!(f, "Bin({} bytes)", b.len()),
            Value::Str(s) => write!(f, "Str(\"{}\")", s),
            Value::Success(n) => write!(f, "Success({})", n),
            Value::Failure(e) => write!(f, "Failure({})", e),
        }
    }
}

impl From<Vec<u8>> for Value {
    fn from(b: Vec<u8>) -> Self {
        Value::Bin(b)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::from(s.to_string())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::Str(s)
    }
}

impl From<usize> for Value {
    fn from(n: usize) -> Self {
        Value::Success(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_value() {
        let v: Value = "foo".into();
        assert_eq!(Value::Str("foo".to_string()), v);
    }

    #[test]
    fn test_number_value() {
        let v: Value = 1.into();
        assert_eq!(Value::Success(1), v);
    }
}
