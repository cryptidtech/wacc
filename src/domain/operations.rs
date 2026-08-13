// SPDX-License-Identifier: Apache-2.0

//! Pure domain logic for cryptographic verification operations
//!
//! This module contains the core business logic without dependencies
//! on external cryptographic libraries. All crypto operations go
//! through port traits.

use crate::ports::crypto::{CryptoError, EqualityChecker, HashVerifier, SignatureVerifier};

/// Result of a verification operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationResult {
    /// Verification succeeded
    Success,
    /// Verification failed with reason
    Failure(String),
}

impl VerificationResult {
    /// Returns true if verification succeeded
    #[must_use]
    pub const fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }

    /// Returns true if verification failed
    #[must_use]
    pub const fn is_failure(&self) -> bool {
        !self.is_success()
    }
}

/// Verifies equality of two byte arrays
///
/// Uses constant-time comparison to prevent timing attacks.
pub fn verify_equality<C: EqualityChecker>(
    checker: &C,
    expected: &[u8],
    actual: &[u8],
) -> VerificationResult {
    if checker.are_equal(expected, actual) {
        VerificationResult::Success
    } else {
        VerificationResult::Failure("values don't match".to_string())
    }
}

/// Verifies a hash preimage
///
/// Pure domain logic - delegates actual hashing to the provided verifier.
pub fn verify_preimage<H: HashVerifier>(
    verifier: &H,
    hash: &[u8],
    preimage: &[u8],
) -> VerificationResult {
    match verifier.verify_preimage(hash, preimage) {
        Ok(()) => VerificationResult::Success,
        Err(CryptoError::HashMismatch) => {
            VerificationResult::Failure("preimage doesn't match".to_string())
        }
        Err(CryptoError::InvalidHashAlgorithm(msg)) => {
            VerificationResult::Failure(format!("invalid hash algorithm: {msg}"))
        }
        Err(e) => VerificationResult::Failure(e.to_string()),
    }
}

/// Verifies a digital signature
///
/// Pure domain logic - delegates actual verification to the provided verifier.
pub fn verify_signature<S: SignatureVerifier>(
    verifier: &S,
    public_key: &[u8],
    signature: &[u8],
    message: &[u8],
) -> VerificationResult {
    match verifier.verify_signature(public_key, signature, message) {
        Ok(()) => VerificationResult::Success,
        Err(e) => VerificationResult::Failure(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct AlwaysEqualChecker;
    impl EqualityChecker for AlwaysEqualChecker {
        fn are_equal(&self, _a: &[u8], _b: &[u8]) -> bool {
            true
        }
    }

    struct NeverEqualChecker;
    impl EqualityChecker for NeverEqualChecker {
        fn are_equal(&self, _a: &[u8], _b: &[u8]) -> bool {
            false
        }
    }

    #[test]
    fn test_verify_equality_success() {
        let result = verify_equality(&AlwaysEqualChecker, b"foo", b"bar");
        assert!(result.is_success());
    }

    #[test]
    fn test_verify_equality_failure() {
        let result = verify_equality(&NeverEqualChecker, b"foo", b"foo");
        assert!(result.is_failure());
    }
}
