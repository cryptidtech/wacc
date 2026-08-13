// SPDX-License-Identifier: Apache-2.0
#![warn(missing_docs)]
#![deny(
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_qualifications
)]
// Pedantic/nursery/cargo lints are enabled in `[lints.clippy]` in Cargo.toml.
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::unnecessary_wraps,
    clippy::unused_self,
    clippy::option_if_let_else,
    clippy::match_same_arms,
    clippy::needless_pass_by_value,
    clippy::too_long_first_doc_paragraph,
    clippy::manual_let_else,
    clippy::return_self_not_must_use
)]

//! WACC - Web Assembly Cryptographic Constructs VM
//!
//! This crate provides a WASM-based virtual machine for executing
//! cryptographic verification scripts used in provenance systems.
//!
//! # Thread Safety
//!
//! The WACC VM is designed for safe concurrent execution:
//!
//! - **`Instance`** - `!Send` - One per thread, not shareable
//! - **`Module`** - `Send + Sync` - Share via `Arc<Module>`
//! - **`Engine`** - `Send + Sync` - Share via `Arc<Engine>` (from wasmtime)
//! - **`ModuleCache`** - `Send + Sync` - Thread-safe caching
//! - **`Value`** - `Send + Sync` - Arc-based, cheap to clone
//!
//! ## Example: Parallel Execution
//!
//! ```rust,no_run
//! use wacc::{Builder, Context, types::{CheckCount, ContextPath}};
//! use std::thread;
//! # use wacc::storage::{Pairs, Stack};
//! # use wacc::Value;
//! # use std::collections::BTreeMap;
//! # struct DummyPairs;
//! # impl Pairs for DummyPairs {
//! #     fn get(&self, _: &str) -> Option<Value> { None }
//! #     fn put(&mut self, _: &str, _: &Value) -> Option<Value> { None }
//! # }
//! # struct DummyStack;
//! # impl Stack for DummyStack {
//! #     fn push(&mut self, _: Value) {}
//! #     fn pop(&mut self) -> Option<Value> { None }
//! #     fn top(&self) -> Option<Value> { None }
//! #     fn peek(&self, _: usize) -> Option<Value> { None }
//! #     fn len(&self) -> usize { 0 }
//! #     fn is_empty(&self) -> bool { true }
//! # }
//! # fn create_storage() -> DummyPairs { DummyPairs }
//! # fn create_stack() -> DummyStack { DummyStack }
//! # fn create_limiter() -> wasmtime::StoreLimits { wasmtime::StoreLimitsBuilder::new().build() }
//! let scripts = vec![vec![0u8; 100], vec![1u8; 100]];
//!
//! let handles: Vec<_> = scripts
//!     .into_iter()
//!     .map(|script_bytes| {
//!         thread::spawn(move || {
//!             // Each thread gets its own instance
//!             let context = Context {
//!                 current: Box::new(create_storage()),
//!                 proposed: Box::new(create_storage()),
//!                 pstack: Box::new(create_stack()),
//!                 rstack: Box::new(create_stack()),
//!                 check_count: CheckCount::zero(),
//!                 write_idx: 0,
//!                 context: ContextPath::root(),
//!                 log: Vec::default(),
//!                 limiter: create_limiter(),
//!             };
//!
//!             let mut instance = Builder::new()
//!                 .with_context(context)
//!                 .with_bytes(script_bytes)
//!                 .try_build()
//!                 .unwrap();
//!
//!             instance.run("main").unwrap_or(false)
//!         })
//!     })
//!     .collect();
//!
//! let results: Vec<bool> = handles
//!     .into_iter()
//!     .map(|h| h.join().unwrap())
//!     .collect();
//! ```
//!
//! See [`CONCURRENCY.md`](https://github.com/cryptidtech/bettersign/blob/main/docs/wacc/CONCURRENCY.md)
//! for detailed concurrency patterns and best practices.

/// Adapters that implement port interfaces
pub mod adapters;

/// WACC API function implementations
pub(crate) mod api;

/// Domain logic (pure business logic)
pub mod domain;

/// Errors produced by this library
pub mod error;
pub use error::Error;

/// Macros for reducing boilerplate
#[macro_use]
pub mod macros;

/// Ports (trait interfaces) for external dependencies
pub mod ports;

/// Security configuration and limits
pub mod security;
pub use security::SecurityLimits;

/// Storage traits
pub mod storage;
pub use storage::{Pairs, Stack};

/// Type-safe wrappers for VM values
pub mod types;

/// The virtual machine for executing WACC code
pub mod vm;
pub use vm::{Builder, Context, Instance, PreparedModule, Runtime, Value};

/// ...and in the darkness bind them
pub mod prelude {
    pub use super::*;
    // re-exports
    pub use wasmtime::StoreLimitsBuilder;
}
