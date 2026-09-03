// SPDX-License-Identifier: Apache-2.0
use crate::{
    api::{WASM_FALSE, WASM_TRUE},
    security::allowed_algorithms,
    types::{CheckCount, ContextPath, LogSize},
    Pairs, Stack, Value,
};
use log::info;
use multi_codec::Codec;
use multi_hash::{mh, Multihash};
use multi_key::{Multikey, Views};
use multi_sig::{Multisig, Views as _};
use multi_util::CodecInfo;
use std::{fmt, io::Write};
use wasmtime::{StoreLimits, Val};

/// Returns true if the codec is one of the XMSS multisig codecs (which carry a
/// stateful leaf index that must not be reused).
const fn is_xmss_msig(codec: Codec) -> bool {
    matches!(
        codec,
        Codec::XmssSha210256Msig | Codec::XmssSha216256Msig | Codec::XmssSha220256Msig
    )
}

/// Returns true if the codec is one of the Lamport signature codecs (which use
/// one-time keys that must not sign more than once).
const fn is_lamport_sig(codec: Codec) -> bool {
    matches!(
        codec,
        Codec::LamportSha3256Sig
            | Codec::LamportSha3384Sig
            | Codec::LamportSha3512Sig
            | Codec::LamportSha2256Sig
            | Codec::LamportSha2384Sig
            | Codec::LamportSha2512Sig
            | Codec::LamportBlake2B512Sig
            | Codec::LamportBlake2S256Sig
            | Codec::LamportBlake3256Sig
            | Codec::LamportShake128Sig
            | Codec::LamportShake256Sig
    )
}

/// Returns true if the codec is one of the merkle-tree Lamport signature
/// codecs (stateful: each signature consumes a leaf index carried in the
/// signature wire data, byte 1 of the `MtSignature` encoding).
const fn is_lamport_merkle_sig(codec: Codec) -> bool {
    matches!(
        codec,
        Codec::LamportMerkleSha3512Sig
            | Codec::LamportMerkleSha3384Sig
            | Codec::LamportMerkleSha3256Sig
            | Codec::LamportMerkleSha2512Sig
            | Codec::LamportMerkleSha2384Sig
            | Codec::LamportMerkleSha2256Sig
            | Codec::LamportMerkleBlake2B512Sig
            | Codec::LamportMerkleBlake2S256Sig
            | Codec::LamportMerkleBlake3256Sig
            | Codec::LamportMerkleShake128Sig
            | Codec::LamportMerkleShake256Sig
    )
}

/// Represents the application state for each instance of a WACC execution.
pub struct Context {
    /// The key-value store of the current state
    pub current: Box<dyn Pairs>,
    /// The key-value store of the proposed state update
    pub proposed: Box<dyn Pairs>,
    /// The stack of values
    pub pstack: Box<dyn Stack>,
    /// The stack of return values
    pub rstack: Box<dyn Stack>,
    /// The number of times a check_* operation has been executed
    pub check_count: CheckCount,
    /// The top down stack index for writing into linear memory
    pub write_idx: usize,
    /// The context key-path for hierarchical key organization
    pub context: ContextPath,
    /// In-memory buffer to accumulate log messages from scripts
    pub log: Vec<u8>,
    /// The limiter
    pub limiter: StoreLimits,
}

impl fmt::Debug for Context {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Context {{ check_count: {}, context: {} }}",
            self.check_count.as_usize(),
            self.context
        )
    }
}

impl Context {
    /// Increment the check counter and push a FAILURE marker on the return stack
    ///
    /// This is called when a cryptographic check operation fails.
    /// The `check_count` tracks the number of check operations performed.
    pub fn check_fail(&mut self, err: &str) -> Val {
        // Increment the check_count to track this check operation
        self.check_count = match self.check_count.increment() {
            Some(count) => count,
            None => {
                // Max check count reached - fail immediately
                return self.fail("maximum check count exceeded");
            }
        };
        // Push failure marker
        self.fail(err)
    }

    /// Push a FAILURE marker onto the return stack without incrementing check count
    ///
    /// This is used for non-check failures (e.g., stack underflow, missing keys).
    pub fn fail(&mut self, err: &str) -> Val {
        // push the FAILURE onto the return stack
        self.rstack.push(Value::Failure(err.to_string()));
        // return that we failed
        WASM_FALSE
    }

    /// Push a SUCCESS marker onto the return stack with current check count
    ///
    /// This is called when a cryptographic check operation succeeds.
    /// The Success value contains the current `check_count`.
    pub fn succeed(&mut self) -> Val {
        // Enforce check count limit
        if self.check_count.at_max() {
            return self.fail("maximum check count exceeded");
        }
        // Push SUCCESS marker with the current check count
        // (This matches original behavior - only failures increment the count)
        self.rstack.push(self.check_count.as_usize().into());
        // return that we succeeded
        WASM_TRUE
    }

    /// Add a line to the log
    pub fn log(&mut self, log_line: &str) -> Val {
        // Check if adding this line would exceed log size limit
        let current_size = LogSize::new(self.log.len());
        let line_size = log_line.len() + 1; // +1 for newline

        if current_size.would_exceed(line_size) {
            return self.fail("log buffer size limit exceeded");
        }

        // add the log line to the log
        match writeln!(&mut self.log, "{log_line}") {
            Ok(()) => WASM_TRUE,
            Err(e) => self.fail(&e.to_string()),
        }
    }

    /// Push raw bytes directly onto the parameter stack
    ///
    /// Unlike [`Self::push`], this does not look up a value from the KVP store.
    /// It pushes the provided bytes directly as a `Value::Bin`.
    /// Used by the `_push_value` WASM import to push data from linear memory.
    pub fn push_value(&mut self, bytes: Vec<u8>) -> Val {
        self.pstack.push(Value::Bin {
            hint: String::new(),
            data: bytes.into(),
        });
        WASM_TRUE
    }

    /// Push the value associated with the key onto the parameter stack
    pub fn push(&mut self, key: &str) -> Val {
        // try to look up the key-value pair by key and push the result onto the stack
        match self.current.get(key) {
            Some(v) => {
                self.pstack.push(v); // pushes Value::Bin(Vec<u8>)
                WASM_TRUE
            }
            None => self.fail(&format!("kvp missing key: {key}")),
        }
    }

    /// Pop a value from the parameter stack
    pub fn pop(&mut self) -> Val {
        // make sure we have at least one parameter on the stack
        if self.pstack.is_empty() {
            return self.fail(&format!(
                "not enough parameters on the stack for pop ({})",
                self.pstack.len()
            ));
        }

        // pop the value from the stack
        let _ = self.pstack.pop();
        WASM_TRUE
    }

    /// Calculate the full key given the context by branching
    ///
    /// For backward compatibility, this simply concatenates the context and key.
    /// The context path already includes separators as needed.
    #[must_use]
    pub fn branch(&self, key: &str) -> String {
        let s = format!("{}{}", self.context.as_str(), key);
        info!("branch({key}) -> {s}");
        s
    }

    /// Verifies the top of the stack matches the value associated with the key
    pub fn check_eq(&mut self, key: &str) -> Val {
        info!("check_eq: loading from current {key}");
        // look up the value
        let value = {
            match self.current.get(key) {
                Some(v @ Value::Bin { .. }) => v,
                Some(v @ Value::Str { .. }) => v,
                Some(_) => {
                    return self.check_fail(&format!("unexpected value type associated with {key}"))
                }
                None => return self.check_fail(&format!("no value associated with {key}")),
            }
        };

        // make sure we have at least one parameter on the stack
        if self.pstack.is_empty() {
            return self.check_fail(&format!(
                "not enough parameters on the stack for check_eq ({})",
                self.pstack.len()
            ));
        }

        // peek at the top item
        info!("check_eq: loading value from stack");
        let stack_value = {
            match self.pstack.top() {
                Some(v @ Value::Bin { .. }) => v,
                Some(v @ Value::Str { .. }) => v,
                _ => return self.check_fail("no value on stack"),
            }
        };

        // check that the values
        if value == stack_value {
            info!("check_eq({key}) -> {value:?} == {stack_value:?} -> true");
            // the eq check passed so pop the argument from the stack
            let _ = self.pstack.pop();
            self.succeed()
        } else {
            info!("check_eq({key}) -> {value:?} == {stack_value:?} -> false");
            // the hashes don't match
            self.check_fail("values don't match")
        }
    }

    /// Checks the preimage proof against the hash already committed to
    ///
    /// This function validates that the hash algorithm is on the security whitelist
    /// to prevent the use of broken or weak hash functions.
    pub fn check_preimage(&mut self, key: &str) -> Val {
        // look up the hash and try to decode it
        let hash = {
            match self.current.get(key) {
                Some(Value::Bin { hint: _, data }) => match Multihash::try_from(data.as_ref()) {
                    Ok(hash) => {
                        // Security: Validate hash algorithm is allowed
                        let codec_value = hash.codec().code();
                        if !allowed_algorithms::is_hash_allowed(codec_value) {
                            return self.check_fail(&format!(
                                "hash algorithm {} is not allowed (codec: 0x{:x})",
                                allowed_algorithms::hash_name(codec_value),
                                codec_value
                            ));
                        }
                        hash
                    }
                    Err(e) => return self.check_fail(&e.to_string()),
                },
                Some(_) => {
                    return self.check_fail(&format!("unexpected value type associated with {key}"))
                }
                None => return self.check_fail(&format!("kvp missing key: {key}")),
            }
        };

        // make sure we have at least one parameter on the stack
        if self.pstack.is_empty() {
            return self.check_fail(&format!(
                "not enough parameters on the stack for check_preimage: {}",
                self.pstack.len()
            ));
        }

        // get the preimage data from the stack
        let preimage = {
            match self.pstack.top() {
                Some(Value::Bin { hint: _, data }) => {
                    match mh::Builder::new_from_bytes(hash.codec(), data.as_ref()) {
                        Ok(builder) => match builder.try_build() {
                            Ok(hash) => hash,
                            Err(e) => return self.check_fail(&e.to_string()),
                        },
                        Err(e) => return self.check_fail(&e.to_string()),
                    }
                }
                Some(Value::Str { hint: _, data }) => {
                    match mh::Builder::new_from_bytes(hash.codec(), data.as_ref().as_bytes()) {
                        Ok(builder) => match builder.try_build() {
                            Ok(hash) => hash,
                            Err(e) => return self.check_fail(&e.to_string()),
                        },
                        Err(e) => return self.check_fail(&e.to_string()),
                    }
                }
                _ => return self.check_fail("no multihash data on stack"),
            }
        };

        // check that the hashes match
        if hash == preimage {
            info!("check_preimage({key}) -> true");
            // the hash check passed so pop the argument from the stack
            let _ = self.pstack.pop();
            self.succeed()
        } else {
            info!("check_preimage({key}) -> false");
            // the hashes don't match
            self.check_fail("preimage doesn't match")
        }
    }

    /// Checks that a public key's fingerprint matches a baked-in hash.
    ///
    /// Takes hash bytes (from WASM memory) and a KVP path string.
    /// Decodes the hash bytes as a Multihash, validates the hash algorithm,
    /// reads the Multikey from `current.get(key)`, uses its `fingerprint_view`
    /// to compute the fingerprint with the same hash codec, and compares.
    /// No pstack interaction.
    pub fn check_preimage_value(&mut self, hash_bytes: &[u8], key: &str) -> Val {
        // Decode hash bytes as Multihash
        let expected = match Multihash::try_from(hash_bytes) {
            Ok(h) => {
                let codec_value = h.codec().code();
                if !allowed_algorithms::is_hash_allowed(codec_value) {
                    return self.check_fail(&format!(
                        "hash algorithm {} is not allowed (codec: 0x{:x})",
                        allowed_algorithms::hash_name(codec_value),
                        codec_value
                    ));
                }
                h
            }
            Err(e) => return self.check_fail(&e.to_string()),
        };

        // Read Multikey from current KVP
        let pubkey = match self.current.get(key) {
            Some(Value::Bin { data, .. }) => match Multikey::try_from(data.as_ref()) {
                Ok(mk) => mk,
                Err(e) => return self.check_fail(&e.to_string()),
            },
            Some(_) => return self.check_fail(&format!("unexpected value type at {key}")),
            None => return self.check_fail(&format!("kvp missing key: {key}")),
        };

        // Compute fingerprint using Multikey's fingerprint view
        let fp_view = match pubkey.fingerprint_view() {
            Ok(v) => v,
            Err(e) => return self.check_fail(&e.to_string()),
        };
        let actual = match fp_view.fingerprint(expected.codec()) {
            Ok(h) => h,
            Err(e) => return self.check_fail(&e.to_string()),
        };

        // Compare
        if expected == actual {
            info!("check_preimage_value({key}) -> true");
            self.succeed()
        } else {
            info!("check_preimage_value({key}) -> false");
            self.check_fail("key fingerprint doesn't match")
        }
    }

    /// Verifies the digital signature proof with the public key and message already committed to
    pub fn check_signature(&mut self, key: &str, msg: &str) -> Val {
        info!("check_signature: loading from current {key}");
        // look up the pubkey and try to decode it
        let pubkey = {
            match self.current.get(key) {
                Some(Value::Bin { hint: _, data }) => match Multikey::try_from(data.as_ref()) {
                    Ok(mk) => mk,
                    Err(e) => return self.check_fail(&e.to_string()),
                },
                Some(_) => {
                    return self.check_fail(&format!("unexpected value type associated with {key}"))
                }
                None => return self.check_fail(&format!("no multikey associated with {key}")),
            }
        };

        // look up the message that was signed
        info!("check_signature: loading from proposed {msg}");
        let message_value = {
            match self.proposed.get(msg) {
                Some(v @ Value::Bin { .. }) => v,
                Some(v @ Value::Str { .. }) => v,
                Some(_) => {
                    return self.check_fail(&format!("unexpected value type associated with {msg}"))
                }
                None => return self.check_fail(&format!("no message associated with {msg}")),
            }
        };

        // make sure we have at least one parameters on the stack
        if self.pstack.is_empty() {
            return self.check_fail(&format!(
                "not enough parameters ({}) on the stack for check_signature ({key}, {msg})",
                self.pstack.len()
            ));
        }

        // peek at the top item and verify that it is a Multisig
        info!("check_signature: loading sig from stack");
        let sig = {
            match self.pstack.top() {
                Some(Value::Bin { hint: _, data }) => match Multisig::try_from(data.as_ref()) {
                    Ok(sig) => sig,
                    Err(e) => return self.check_fail(&e.to_string()),
                },
                _ => return self.check_fail("no multisig on stack"),
            }
        };

        let verify_view = match pubkey.verify_view() {
            Ok(v) => v,
            Err(e) => return self.check_fail(&e.to_string()),
        };

        // verify the signature - extract message bytes based on type
        let verification_result = match &message_value {
            Value::Bin { hint: _, data } => verify_view.verify(&sig, Some(data.as_ref())),
            Value::Str { hint: _, data } => verify_view.verify(&sig, Some(data.as_bytes())),
            _ => unreachable!("message_value already validated as Bin or Str"),
        };

        match verification_result {
            Ok(()) => {
                info!("check_signature({key}, {msg}) -> true");
                if let Err(e) = self.enforce_stateful_key_rules(&sig, &pubkey) {
                    return self.check_fail(&e);
                }
                // the signature verification worked so pop the signature argument off
                // of the stack before continuing
                self.pstack.pop();
                self.succeed()
            }
            Err(e) => {
                info!("check_signature({key}, {msg}) -> false");
                self.check_fail(&e.to_string())
            }
        }
    }

    /// Enforce the stateful/one-time key rules after a successful signature
    /// verification: XMSS leaf-index monotonicity, one-time Lamport key use,
    /// and merkle-tree Lamport leaf monotonicity.
    fn enforce_stateful_key_rules(&self, sig: &Multisig, pubkey: &Multikey) -> Result<(), String> {
        let codec = sig.codec();
        // XMSS is stateful: enforce that the consumed leaf index has not
        // been reused or rolled back for this public key across the log.
        if is_xmss_msig(codec) {
            let index = match sig.sig_index() {
                Some(i) => i,
                None => return Err("XMSS multisig missing sig-index".into()),
            };
            let pubkey_bytes = match pubkey.data_view().and_then(|dv| dv.key_bytes()) {
                Ok(b) => b,
                Err(e) => return Err(e.to_string()),
            };
            return crate::vm::xmss_guard::enforce_xmss_index(pubkey_bytes.as_slice(), index);
        }
        // Lamport keys are one-time: enforce that this public key has not
        // already signed an earlier entry in the log.
        if is_lamport_sig(codec) {
            let pubkey_bytes = match pubkey.data_view().and_then(|dv| dv.key_bytes()) {
                Ok(b) => b,
                Err(e) => return Err(e.to_string()),
            };
            return crate::vm::xmss_guard::enforce_lamport_once(pubkey_bytes.as_slice());
        }
        // Merkle-tree Lamport keys are stateful: each signature embeds
        // its consumed leaf index in the MtSignature wire data (byte 1,
        // after the depth byte). Enforce monotonic consumption across
        // the log.
        if is_lamport_merkle_sig(codec) {
            let pubkey_bytes = match pubkey.data_view().and_then(|dv| dv.key_bytes()) {
                Ok(b) => b,
                Err(e) => return Err(e.to_string()),
            };
            let sig_bytes = match sig.data_view().and_then(|dv| dv.sig_bytes()) {
                Ok(b) => b,
                Err(e) => return Err(e.to_string()),
            };
            let depth = match sig_bytes.first() {
                Some(&d) if (1..=3).contains(&d) => d,
                _ => return Err("merkle-Lamport signature has invalid depth byte".into()),
            };
            let index = match sig_bytes.get(1) {
                Some(&i) if usize::from(i) < (1usize << depth) => usize::from(i),
                _ => return Err("merkle-Lamport signature has invalid leaf index".into()),
            };
            return crate::vm::xmss_guard::enforce_lamport_merkle(pubkey_bytes.as_slice(), index);
        }
        Ok(())
    }
}
