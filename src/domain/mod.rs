// SPDX-License-Identifier: Apache-2.0

//! Domain logic for the WACC VM
//!
//! This module contains pure business logic that is independent of
//! external libraries and infrastructure concerns. Following hexagonal
//! architecture, domain logic only depends on port interfaces, never
//! on concrete implementations.
//!
//! # Principles
//!
//! 1. **No external dependencies** - Only depends on ports (traits)
//! 2. **Testable in isolation** - Can test without real crypto libraries
//! 3. **Pure functions** - Logic is deterministic and side-effect free
//! 4. **Clear intent** - Business rules are explicit

pub mod operations;

pub use operations::{verify_equality, verify_preimage, verify_signature, VerificationResult};
