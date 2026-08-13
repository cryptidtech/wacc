// SPDX-License-Identifier: Apache-2.0

//! Type-safe wrappers for WACC VM values
//!
//! This module provides newtype wrappers that enforce type safety and validation
//! at compile time, following Rust best practices for the newtype pattern.
//!
//! # Design Principles
//!
//! 1. **Compile-Time Safety**: Different kinds of values use different types,
//!    preventing mixing up keys, pointers, and sizes.
//!
//! 2. **Runtime Validation**: Constructors validate inputs, preventing invalid
//!    values from being created.
//!
//! 3. **Zero-Cost Abstraction**: Newtypes have no runtime overhead compared to
//!    raw primitives.
//!
//! 4. **Clear Intent**: Type signatures make it obvious what kind of value is
//!    expected, improving code readability.
//!
//! # Module Organization
//!
//! - `keys` - Key and path newtypes for the key-value store
//! - `limits` - Resource limit newtypes (fuel, check count, log size)
//! - `memory` - WASM memory address and size newtypes

/// Key and path newtypes for key-value store operations
pub mod keys;
pub use keys::{ContextPath, CurrentKey, ProposedKey};

/// Resource limit newtypes
pub mod limits;
pub use limits::{CheckCount, FuelAmount, LogSize};

/// WASM memory newtypes
pub mod memory;
pub use memory::{WasmPtr, WasmSize};
