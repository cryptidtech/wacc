// SPDX-License-Identifier: Apache-2.0

/// virtual machine builder
pub mod builder;

/// module caching for performance
pub mod cache;

/// wasm code compiler
pub mod compiler;

/// virtual machine execution context
pub mod context;

/// virtual machine instance
pub mod instance;

/// reusable Wasmtime compilation state
pub mod runtime;

/// value wrapper used in the virtual machine
pub mod value;

/// XMSS stateful-signature leaf-index reuse enforcement
pub mod xmss_guard;

pub use builder::Builder;
pub use cache::ModuleCache;
pub use compiler::Compiler;
pub use context::Context;
pub use instance::Instance;
pub use runtime::{PreparedModule, Runtime};
pub use value::Value;
pub use xmss_guard::XmssEnforcement;
