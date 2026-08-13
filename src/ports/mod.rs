// SPDX-License-Identifier: Apache-2.0

//! Ports (interfaces) for the WACC VM
//!
//! Following hexagonal architecture, ports define the boundaries between
//! the domain logic and external dependencies. All external interactions
//! go through these trait interfaces.
//!
//! # Organization
//!
//! - `crypto` - Cryptographic operation interfaces (hashing, signatures)
//!
//! # Hexagonal Architecture
//!
//! ```text
//!     External World
//!          |
//!          v
//!     [Adapters]  ← Implement ports
//!          |
//!          v
//!      [Ports]    ← Trait interfaces
//!          |
//!          v
//!     [Domain]    ← Pure business logic
//! ```
//!
//! Benefits:
//! - Domain logic is testable without external dependencies
//! - Easy to swap implementations (e.g., different crypto libraries)
//! - Clear dependency direction (domain doesn't depend on infrastructure)
//! - Easier to understand and maintain

pub mod crypto;

pub use crypto::{CryptoError, EqualityChecker, HashVerifier, SignatureVerifier};
