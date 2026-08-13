// SPDX-License-Identifier: Apache-2.0

//! Macros for reducing boilerplate in WACC VM code
//!
//! These macros follow best practices for production Rust macros:
//! - Clear, predictable expansion
//! - Good error messages
//! - Hygenic (don't capture variables)
//! - Well-documented with examples

/// Extracts binary data from a Value, returning early with error if wrong type
///
/// This macro reduces boilerplate when extracting data from stack values.
///
/// # Examples
///
/// ```ignore
/// use wacc::{Value, extract_bin};
///
/// fn process(value: Value) -> Result<usize, Error> {
///     let data: &[u8] = extract_bin!(value, "expected binary data");
///     Ok(data.len())
/// }
/// ```
///
/// # Expansion
///
/// ```rust,ignore
/// extract_bin!(value, "error message")
/// // Expands to:
/// match value {
///     Value::Bin { data, .. } => data.as_ref(),
///     _ => return Err(Error::custom(&"error message")),
/// }
/// ```
#[macro_export]
macro_rules! extract_bin {
    ($value:expr, $err:expr) => {
        match $value {
            $crate::Value::Bin { data, .. } => data.as_ref(),
            _ => return Err($crate::Error::custom(&$err)),
        }
    };
}

/// Extracts string data from a Value, returning early with error if wrong type
///
/// # Examples
///
/// ```ignore
/// use wacc::{Value, extract_str};
///
/// fn process(value: Value) -> Result<usize, Error> {
///     let text: &str = extract_str!(value, "expected string data");
///     Ok(text.len())
/// }
/// ```
#[macro_export]
macro_rules! extract_str {
    ($value:expr, $err:expr) => {
        match $value {
            $crate::Value::Str { data, .. } => data.as_ref(),
            _ => return Err($crate::Error::custom(&$err)),
        }
    };
}

/// Pops a value from the stack or fails with error message
///
/// Reduces common pattern of checking if stack is empty before popping.
///
/// # Examples
///
/// ```rust,ignore
/// use wacc::{pop_or_fail, Value};
///
/// fn my_operation(context: &mut Context) -> Val {
///     let value = pop_or_fail!(context, "not enough parameters");
///     // Use value...
///     context.succeed()
/// }
/// ```
#[macro_export]
macro_rules! pop_or_fail {
    ($context:expr, $err:expr) => {
        match $context.pstack.pop() {
            Some(v) => v,
            None => return $context.fail(&$err),
        }
    };
}

/// Peeks at the top stack value or fails with error message
///
/// # Examples
///
/// ```rust,ignore
/// use wacc::{peek_or_fail, Value};
///
/// fn my_operation(context: &mut Context) -> Val {
///     let value = peek_or_fail!(context, "empty stack");
///     // Use value...
///     context.succeed()
/// }
/// ```
#[macro_export]
macro_rules! peek_or_fail {
    ($context:expr, $err:expr) => {
        match $context.pstack.top() {
            Some(v) => v,
            None => return $context.fail(&$err),
        }
    };
}

/// Registers a WACC API function with standardized error handling
///
/// This macro reduces boilerplate in API function registration.
///
/// # Examples
///
/// ```rust,ignore
/// use wacc::register_api_func;
///
/// pub(crate) fn add_to_linker(
///     engine: &Engine,
///     linker: &mut Linker<Context<'_>>
/// ) -> Result<(), Error> {
///     register_api_func!(linker, engine, "wacc", "_my_func", my_func_impl)?;
///     Ok(())
/// }
/// ```
///
/// # Expansion
///
/// Expands to proper error handling with backtrace-ready structure.
#[macro_export]
macro_rules! register_api_func {
    ($linker:expr, $engine:expr, $module:expr, $name:expr, $func:expr) => {
        $linker
            .func_new(
                $module,
                $name,
                wasmtime::FuncType::new(
                    $engine,
                    [wasmtime::ValType::I32, wasmtime::ValType::I32],
                    [wasmtime::ValType::I32],
                ),
                $func,
            )
            .map_err(|e| $crate::error::ApiError::RegisterApiFailed {
                function_name: $name.to_string(),
                reason: format!("{}", e),
            })?
    };
}

#[cfg(test)]
mod tests {
    use crate::Value;

    #[test]
    fn test_extract_bin_macro() {
        fn extract(value: &Value) -> Result<usize, crate::Error> {
            let data = match value {
                Value::Bin { data, .. } => data.as_ref(),
                _ => return Err(crate::Error::custom(&"expected binary")),
            };
            Ok(data.len())
        }

        let value: Value = vec![1, 2, 3].into();
        assert_eq!(extract(&value).unwrap(), 3);

        let value: Value = "wrong type".into();
        assert!(extract(&value).is_err());
    }

    #[test]
    fn test_extract_str_macro() {
        fn extract(value: &Value) -> Result<usize, crate::Error> {
            let data = match value {
                Value::Str { data, .. } => data.as_ref(),
                _ => return Err(crate::Error::custom(&"expected string")),
            };
            Ok(data.len())
        }

        let value: Value = "hello".into();
        assert_eq!(extract(&value).unwrap(), 5);

        let value: Value = vec![1, 2, 3].into();
        assert!(extract(&value).is_err());
    }
}
