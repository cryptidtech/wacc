// SPDX-License-Identifier: Apache-2.0

//! Adapters that implement port interfaces
//!
//! Adapters connect the domain logic to external dependencies.
//! They implement the port traits using concrete libraries.
//!
//! # Available Adapters
//!
//! - `multicodec_crypto` - Cryptographic operations using multikey/multisig/multihash

pub mod multicodec_crypto;

pub use multicodec_crypto::{MulticodecHashVerifier, MulticodecSignatureVerifier};
