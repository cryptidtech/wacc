// SPDX-License-Identifier: Apache-2.0

//! Adapter for cryptographic operations using multicodec libraries
//!
//! This adapter implements the crypto ports using the multikey,
//! multisig, and multihash crates from the bettersign workspace.

use crate::{
    ports::crypto::{CryptoError, HashVerifier, SignatureVerifier},
    security::allowed_algorithms,
};
use multi_hash::{mh, Multihash};
use multi_key::{Multikey, Views};
use multi_sig::Multisig;
use multi_util::CodecInfo;

/// Adapter for hash verification using multihash
pub struct MulticodecHashVerifier;

impl HashVerifier for MulticodecHashVerifier {
    fn verify_preimage(&self, hash_bytes: &[u8], preimage: &[u8]) -> Result<(), CryptoError> {
        // Decode the hash
        let hash = Multihash::try_from(hash_bytes)
            .map_err(|e| CryptoError::Other(format!("Failed to decode multihash: {e}")))?;

        // Check if algorithm is allowed
        let codec = hash.codec().code();
        if !self.is_algorithm_allowed(codec) {
            return Err(CryptoError::InvalidHashAlgorithm(format!(
                "{} (codec: 0x{:x})",
                self.algorithm_name(codec),
                codec
            )));
        }

        // Compute hash of preimage using same algorithm
        let computed = mh::Builder::new_from_bytes(hash.codec(), preimage)
            .map_err(|e| CryptoError::Other(format!("Failed to hash preimage: {e}")))?
            .try_build()
            .map_err(|e| CryptoError::Other(format!("Failed to build hash: {e}")))?;

        // Compare hashes
        if hash == computed {
            Ok(())
        } else {
            Err(CryptoError::HashMismatch)
        }
    }

    fn is_algorithm_allowed(&self, codec: u64) -> bool {
        allowed_algorithms::is_hash_allowed(codec)
    }

    fn algorithm_name(&self, codec: u64) -> &str {
        allowed_algorithms::hash_name(codec)
    }
}

/// Adapter for signature verification using multikey/multisig
pub struct MulticodecSignatureVerifier;

impl SignatureVerifier for MulticodecSignatureVerifier {
    fn verify_signature(
        &self,
        public_key_bytes: &[u8],
        signature_bytes: &[u8],
        message: &[u8],
    ) -> Result<(), CryptoError> {
        // Decode public key
        let public_key = Multikey::try_from(public_key_bytes).map_err(|e| {
            CryptoError::InvalidPublicKey(format!("Failed to decode multikey: {e}"))
        })?;

        // Decode signature
        let signature = Multisig::try_from(signature_bytes).map_err(|e| {
            CryptoError::InvalidSignature(format!("Failed to decode multisig: {e}"))
        })?;

        // Get verification view
        let verify_view = public_key.verify_view().map_err(|e| {
            CryptoError::InvalidPublicKey(format!("Failed to get verify view: {e}"))
        })?;

        // Verify signature
        verify_view
            .verify(&signature, Some(message))
            .map_err(|e| CryptoError::SignatureInvalid(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_verifier_algorithm_validation() {
        let verifier = MulticodecHashVerifier;

        // Allowed algorithms
        assert!(verifier.is_algorithm_allowed(0x12)); // SHA-256
        assert!(verifier.is_algorithm_allowed(0x16)); // SHA3-256

        // Disallowed algorithms
        assert!(!verifier.is_algorithm_allowed(0x11)); // SHA-1
        assert!(!verifier.is_algorithm_allowed(0xd5)); // MD5
    }

    #[test]
    fn test_hash_verifier_algorithm_names() {
        let verifier = MulticodecHashVerifier;

        assert_eq!(verifier.algorithm_name(0x12), "SHA2-256");
        assert_eq!(verifier.algorithm_name(0x11), "SHA-1 (deprecated)");
        assert_eq!(verifier.algorithm_name(0xd5), "MD5 (broken)");
    }
}
