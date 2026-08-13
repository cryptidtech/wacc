// SPDX-License-Identifier: Apache-2.0

//! Security configuration and limits for the WACC VM
//!
//! This module defines security-critical constants and configuration
//! that protect against resource exhaustion and other attacks.

use crate::types::{CheckCount, FuelAmount, LogSize};

/// Security limits for WACC VM execution
///
/// These limits protect against various attack vectors including:
/// - Resource exhaustion (`DoS`)
/// - Memory exhaustion
/// - Infinite loops
/// - Log spam
#[derive(Debug, Clone, Copy)]
pub struct SecurityLimits {
    /// Maximum fuel for execution (prevents infinite loops)
    pub max_fuel: FuelAmount,

    /// Maximum number of cryptographic check operations
    pub max_checks: CheckCount,

    /// Maximum log buffer size
    pub max_log_size: LogSize,

    /// Maximum WASM linear memory size (in bytes)
    pub max_memory_size: usize,

    /// Maximum number of WASM instances
    pub max_instances: usize,

    /// Maximum number of WASM memories
    pub max_memories: usize,

    /// Maximum number of WASM tables
    pub max_tables: usize,
}

impl SecurityLimits {
    /// Default production limits (conservative, secure)
    ///
    /// These limits are appropriate for production use where security
    /// is the primary concern.
    pub const PRODUCTION: Self = Self {
        max_fuel: FuelAmount::new(FuelAmount::DEFAULT),
        max_checks: CheckCount::new(CheckCount::MAX),
        max_log_size: LogSize::new(LogSize::MAX),
        max_memory_size: 4 * 1024 * 1024, // 4 MB
        max_instances: 1,
        max_memories: 1,
        max_tables: 0,
    };

    /// Development limits (more permissive for testing)
    ///
    /// These limits are appropriate for development and testing
    /// where debugging information is more important.
    pub const DEVELOPMENT: Self = Self {
        max_fuel: FuelAmount::new(10 * FuelAmount::DEFAULT),
        max_checks: CheckCount::new(CheckCount::MAX),
        max_log_size: LogSize::new(LogSize::MAX),
        max_memory_size: 16 * 1024 * 1024, // 16 MB
        max_instances: 2,
        max_memories: 2,
        max_tables: 1,
    };

    /// Strict limits for untrusted code
    ///
    /// Use these limits when executing code from untrusted sources.
    pub const STRICT: Self = Self {
        max_fuel: FuelAmount::new(100_000),
        max_checks: CheckCount::new(16),      // Very limited checks
        max_log_size: LogSize::new(4 * 1024), // 4 KB
        max_memory_size: 1024 * 1024,         // 1 MB
        max_instances: 1,
        max_memories: 1,
        max_tables: 0,
    };
}

impl Default for SecurityLimits {
    fn default() -> Self {
        Self::PRODUCTION
    }
}

/// Allowed cryptographic hash algorithms
///
/// This whitelist prevents the use of weak or broken hash functions.
/// Only well-vetted, cryptographically secure hash algorithms are permitted.
pub mod allowed_algorithms {
    /// Allowed hash algorithm multicodec codes
    ///
    /// See: <https://github.com/multiformats/multicodec/blob/master/table.csv>
    /// Codes are taken from bettersign/crates/multicodec/table.csv
    pub const ALLOWED_HASH_CODECS: &[u64] = &[
        // SHA-2 family (NIST FIPS 180-4)
        0x12, // sha2-256
        0x13, // sha2-512
        0x20, // sha2-384
        // SHA-3 family (NIST FIPS 202)
        0x14, // sha3-512
        0x15, // sha3-384
        0x16, // sha3-256
        0x17, // sha3-224
        // Keccak family (used in Ethereum)
        0x1a, // keccak-224
        0x1b, // keccak-256
        0x1c, // keccak-384
        0x1d, // keccak-512
        // Blake family (modern, fast, secure)
        0x1e,   // blake3
        0xb220, // blake2b-256
        0xb240, // blake2b-512
        0xb260, // blake2s-256
    ];

    /// Checks if a hash algorithm is allowed
    ///
    /// # Examples
    ///
    /// ```
    /// # use wacc::security::allowed_algorithms::is_hash_allowed;
    /// assert!(is_hash_allowed(0x12)); // SHA-256
    /// assert!(is_hash_allowed(0x1e)); // Blake2b-256
    /// assert!(!is_hash_allowed(0x11)); // SHA-1 (broken)
    /// assert!(!is_hash_allowed(0xd5)); // MD5 (broken)
    /// ```
    #[must_use]
    pub fn is_hash_allowed(codec: u64) -> bool {
        ALLOWED_HASH_CODECS.contains(&codec)
    }

    /// Gets the name of a hash algorithm (for error messages)
    #[must_use]
    pub const fn hash_name(codec: u64) -> &'static str {
        match codec {
            // Broken/deprecated
            0x11 => "SHA-1 (deprecated)",
            0xd4 => "MD4 (broken)",
            0xd5 => "MD5 (broken)",

            // SHA-2 family
            0x12 => "SHA2-256",
            0x13 => "SHA2-512",
            0x1013 => "SHA2-224",
            0x20 => "SHA2-384",

            // SHA-3 family
            0x14 => "SHA3-512",
            0x15 => "SHA3-384",
            0x16 => "SHA3-256",
            0x17 => "SHA3-224",

            // Keccak family
            0x1a => "Keccak-224",
            0x1b => "Keccak-256",
            0x1c => "Keccak-384",
            0x1d => "Keccak-512",

            // Blake family
            0x1e => "Blake3",
            0xb220 => "Blake2b-256",
            0xb240 => "Blake2b-512",
            0xb260 => "Blake2s-256",

            _ => "unknown",
        }
    }

    /// Allowed signature algorithm multicodec codes
    ///
    /// Only modern, secure signature algorithms are permitted.
    /// Codes are taken from bettersign/crates/multicodec/table.csv
    pub const ALLOWED_SIG_CODECS: &[u64] = &[
        0xed,   // ed25519-pub (widely used, secure)
        0xe7,   // secp256k1-pub (Bitcoin, Ethereum)
        0x1200, // p256-pub (NIST P-256)
        0xea,   // bls12_381-g1-pub (modern, supports aggregation)
    ];

    /// Checks if a signature algorithm is allowed
    #[must_use]
    pub fn is_sig_allowed(codec: u64) -> bool {
        ALLOWED_SIG_CODECS.contains(&codec)
    }
}

/// Stack depth limits
pub mod stack_limits {
    /// Maximum parameter stack depth
    pub const MAX_PSTACK_DEPTH: usize = 256;

    /// Maximum return stack depth
    pub const MAX_RSTACK_DEPTH: usize = 256;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_limits_default() {
        let limits = SecurityLimits::default();
        assert_eq!(limits.max_fuel.as_u64(), FuelAmount::DEFAULT);
        assert_eq!(limits.max_checks.as_usize(), CheckCount::MAX);
    }

    #[test]
    fn test_production_vs_strict() {
        let prod = SecurityLimits::PRODUCTION;
        let strict = SecurityLimits::STRICT;

        assert!(prod.max_fuel.as_u64() > strict.max_fuel.as_u64());
        assert!(prod.max_checks.as_usize() > strict.max_checks.as_usize());
        assert!(prod.max_memory_size > strict.max_memory_size);
    }

    #[test]
    fn test_hash_whitelist() {
        use allowed_algorithms::is_hash_allowed;

        // Allowed
        assert!(is_hash_allowed(0x12)); // SHA-256
        assert!(is_hash_allowed(0x13)); // SHA-512
        assert!(is_hash_allowed(0x1e)); // Blake2b-256
        assert!(is_hash_allowed(0xb220)); // Blake3

        // Not allowed (broken/weak)
        assert!(!is_hash_allowed(0x11)); // SHA-1
        assert!(!is_hash_allowed(0xd5)); // MD5
        assert!(!is_hash_allowed(0x9999)); // Unknown
    }

    #[test]
    fn test_sig_whitelist() {
        use allowed_algorithms::is_sig_allowed;

        // Allowed
        assert!(is_sig_allowed(0xed)); // Ed25519
        assert!(is_sig_allowed(0x1200)); // secp256k1
        assert!(is_sig_allowed(0xea)); // BLS12-381

        // Not allowed
        assert!(!is_sig_allowed(0x9999)); // Unknown
    }
}
