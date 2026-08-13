// SPDX-License-Identifier: Apache-2.0

//! Ports (interfaces) for cryptographic operations
//!
//! These traits define the boundaries between the domain logic and
//! external cryptographic libraries, following hexagonal architecture principles.

use std::fmt;

/// Error type for cryptographic verification operations
#[derive(Debug, Clone)]
pub enum CryptoError {
    /// Invalid hash algorithm
    InvalidHashAlgorithm(String),
    /// Hash verification failed
    HashMismatch,
    /// Signature verification failed
    SignatureInvalid(String),
    /// Invalid public key format
    InvalidPublicKey(String),
    /// Invalid signature format
    InvalidSignature(String),
    /// Other cryptographic error
    Other(String),
}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHashAlgorithm(msg) => write!(f, "Invalid hash algorithm: {msg}"),
            Self::HashMismatch => write!(f, "Hash verification failed: preimage doesn't match"),
            Self::SignatureInvalid(msg) => write!(f, "Signature verification failed: {msg}"),
            Self::InvalidPublicKey(msg) => write!(f, "Invalid public key: {msg}"),
            Self::InvalidSignature(msg) => write!(f, "Invalid signature: {msg}"),
            Self::Other(msg) => write!(f, "Cryptographic error: {msg}"),
        }
    }
}

impl std::error::Error for CryptoError {}

/// Port for hash verification operations
///
/// This trait abstracts hash verification, allowing different
/// implementations (e.g., multihash, custom hash functions).
pub trait HashVerifier {
    /// Verifies that a preimage hashes to the expected hash
    ///
    /// # Arguments
    ///
    /// * `hash` - The expected hash value (multiformat encoded)
    /// * `preimage` - The preimage data to verify
    ///
    /// # Returns
    ///
    /// `Ok(())` if verification succeeds, `Err` otherwise
    fn verify_preimage(&self, hash: &[u8], preimage: &[u8]) -> Result<(), CryptoError>;

    /// Checks if a hash algorithm is allowed by security policy
    fn is_algorithm_allowed(&self, codec: u64) -> bool;

    /// Gets the name of a hash algorithm for error messages
    fn algorithm_name(&self, codec: u64) -> &str;
}

/// Port for signature verification operations
///
/// This trait abstracts signature verification, allowing different
/// implementations (e.g., multikey/multisig, custom crypto).
pub trait SignatureVerifier {
    /// Verifies a digital signature
    ///
    /// # Arguments
    ///
    /// * `public_key` - The public key (multiformat encoded)
    /// * `signature` - The signature to verify (multiformat encoded)
    /// * `message` - The message that was signed
    ///
    /// # Returns
    ///
    /// `Ok(())` if verification succeeds, `Err` otherwise
    fn verify_signature(
        &self,
        public_key: &[u8],
        signature: &[u8],
        message: &[u8],
    ) -> Result<(), CryptoError>;
}

/// Port for equality checking
///
/// This trait allows custom equality comparison logic.
pub trait EqualityChecker {
    /// Checks if two byte slices are equal
    ///
    /// Default implementation uses constant-time comparison.
    fn are_equal(&self, a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }

        // Constant-time comparison to prevent timing attacks
        let mut diff = 0u8;
        for (byte_a, byte_b) in a.iter().zip(b.iter()) {
            diff |= byte_a ^ byte_b;
        }
        diff == 0
    }
}

/// Default implementation uses constant-time comparison
impl EqualityChecker for () {}
