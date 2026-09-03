// SPDX-License-Identifier: Apache-2.0
//! Thread-local enforcement of XMSS stateful-signature leaf-index monotonicity
//! and merkle-tree Lamport leaf reuse prevention.
//!
//! XMSS is a stateful hash-based signature scheme: each signature consumes a
//! one-time leaf `index` that must never be reused. A provenance log records the
//! consumed index in each XMSS `Multisig` proof (as `AttrId::SigIndex`). This
//! module enforces, across a single log verification pass, that for a given XMSS
//! public key the indices used by successive entries strictly increase. That
//! makes a key-state rollback (e.g. restoring a key from an old snapshot and
//! re-signing at an already-consumed index) fail verification.
//!
//! Merkle-tree Lamport signatures carry their consumed leaf index inside the
//! signature wire data (`MtSignature` encodes `[depth, index, ...]`, so the
//! index is byte 1). Merkle leaves are also consumed in order, so the same
//! strictly-increasing monotonicity rule applies per merkle public key.
//!
//! The state is thread-local because a log is verified sequentially on one
//! thread (the wacc VM runs scripts synchronously on the calling thread). The
//! provenance-log verifier installs an [`XmssEnforcement`] guard for the
//! duration of a verification pass and commits per-entry observations only after
//! an entry fully validates, so lock-script retries within one entry cannot
//! falsely trip the reuse check.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

/// Per-key stateful-signature state for one verification pass. Tracks XMSS leaf
/// index monotonicity, Lamport one-time-key single use, and merkle-tree Lamport
/// leaf monotonicity.
#[derive(Default)]
struct XmssIndexState {
    /// max leaf index committed by previously-validated entries, keyed by the
    /// XMSS public-key bytes
    committed: HashMap<Vec<u8>, u32>,
    /// indices observed while verifying the current (not-yet-committed) entry
    observed: Vec<(Vec<u8>, u32)>,
    /// Lamport public keys that a previously-validated entry has already signed
    /// with (each Lamport key is one-time)
    lamport_used: HashSet<Vec<u8>>,
    /// Lamport public keys observed while verifying the current entry
    lamport_observed: Vec<Vec<u8>>,
    /// max merkle leaf index committed by previously-validated entries, keyed
    /// by the merkle public-key bytes
    merkle_committed: HashMap<Vec<u8>, usize>,
    /// merkle leaf indices observed while verifying the current entry
    merkle_observed: Vec<(Vec<u8>, usize)>,
}

impl XmssIndexState {
    /// Check that `index` is strictly greater than any committed index for this
    /// key, and record it as observed for the current entry. Returns an error if
    /// the index reuses or rolls back a previously-committed index.
    fn check_and_observe(&mut self, pubkey: &[u8], index: u32) -> Result<(), String> {
        if let Some(&max) = self.committed.get(pubkey) {
            if index <= max {
                return Err(format!(
                    "XMSS leaf index {index} reused or rolled back (last committed index {max})"
                ));
            }
        }
        self.observed.push((pubkey.to_vec(), index));
        Ok(())
    }

    /// Check that this Lamport public key has not already signed a
    /// previously-validated entry, and record it as observed for the current
    /// entry. Returns an error if the one-time key is being reused.
    fn check_and_observe_lamport(&mut self, pubkey: &[u8]) -> Result<(), String> {
        if self.lamport_used.contains(pubkey) {
            return Err(
                "Lamport one-time key reused: this public key already signed an earlier entry"
                    .to_string(),
            );
        }
        self.lamport_observed.push(pubkey.to_vec());
        Ok(())
    }

    /// Check that `index` is strictly greater than any committed merkle leaf
    /// index for this key, and record it as observed for the current entry.
    /// Returns an error if the leaf index is reused or rolled back.
    fn check_and_observe_merkle(&mut self, pubkey: &[u8], index: usize) -> Result<(), String> {
        if let Some(&max) = self.merkle_committed.get(pubkey) {
            if index <= max {
                return Err(format!(
                    "merkle-Lamport leaf index {index} reused or rolled back (last committed \
                     index {max})"
                ));
            }
        }
        self.merkle_observed.push((pubkey.to_vec(), index));
        Ok(())
    }

    /// Commit the current entry's observations into the committed state.
    fn commit(&mut self) {
        for (pubkey, index) in self.observed.drain(..) {
            let slot = self.committed.entry(pubkey).or_insert(index);
            if index > *slot {
                *slot = index;
            }
        }
        for pubkey in self.lamport_observed.drain(..) {
            self.lamport_used.insert(pubkey);
        }
        for (pubkey, index) in self.merkle_observed.drain(..) {
            let slot = self.merkle_committed.entry(pubkey).or_insert(index);
            if index > *slot {
                *slot = index;
            }
        }
    }

    /// Discard the current entry's observations (entry did not validate).
    fn discard(&mut self) {
        self.observed.clear();
        self.lamport_observed.clear();
        self.merkle_observed.clear();
    }
}

thread_local! {
    static XMSS_STATE: RefCell<Option<XmssIndexState>> = const { RefCell::new(None) };
}

/// RAII guard that installs XMSS index enforcement for the current thread for
/// its lifetime, restoring any previously-installed state on drop (so nested
/// verifications compose correctly).
pub struct XmssEnforcement {
    prev: Option<XmssIndexState>,
}

impl XmssEnforcement {
    /// Install a fresh enforcement state, returning a guard that restores the
    /// prior state when dropped.
    #[must_use]
    pub fn install() -> Self {
        let prev = XMSS_STATE.with(|s| s.borrow_mut().replace(XmssIndexState::default()));
        Self { prev }
    }

    /// Commit the current entry's observed indices (call after an entry validates).
    pub fn commit_entry(&self) {
        XMSS_STATE.with(|s| {
            if let Some(state) = s.borrow_mut().as_mut() {
                state.commit();
            }
        });
    }

    /// Discard the current entry's observed indices (call if an entry fails).
    pub fn discard_entry(&self) {
        XMSS_STATE.with(|s| {
            if let Some(state) = s.borrow_mut().as_mut() {
                state.discard();
            }
        });
    }
}

impl Drop for XmssEnforcement {
    fn drop(&mut self) {
        XMSS_STATE.with(|s| *s.borrow_mut() = self.prev.take());
    }
}

/// Enforce leaf-index monotonicity for one verified XMSS signature.
///
/// No-op (returns `Ok`) when no enforcement guard is installed — e.g. standalone
/// wacc VM unit tests that exercise scripts without a provenance-log context.
pub(crate) fn enforce_xmss_index(pubkey: &[u8], index: u32) -> Result<(), String> {
    XMSS_STATE.with(|s| match s.borrow_mut().as_mut() {
        Some(state) => state.check_and_observe(pubkey, index),
        None => Ok(()),
    })
}

/// Enforce single use of a Lamport one-time public key across a verification
/// pass. No-op (returns `Ok`) when no enforcement guard is installed.
pub(crate) fn enforce_lamport_once(pubkey: &[u8]) -> Result<(), String> {
    XMSS_STATE.with(|s| match s.borrow_mut().as_mut() {
        Some(state) => state.check_and_observe_lamport(pubkey),
        None => Ok(()),
    })
}

/// Enforce leaf-index monotonicity for one verified merkle-tree Lamport
/// signature. `index` is the consumed leaf index from the `MtSignature` wire
/// data (byte 1). No-op (returns `Ok`) when no enforcement guard is installed.
pub(crate) fn enforce_lamport_merkle(pubkey: &[u8], index: usize) -> Result<(), String> {
    XMSS_STATE.with(|s| match s.borrow_mut().as_mut() {
        Some(state) => state.check_and_observe_merkle(pubkey, index),
        None => Ok(()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const K1: &[u8] = b"xmss-public-key-one";
    const K2: &[u8] = b"xmss-public-key-two";

    #[test]
    fn monotonic_index_per_key() {
        let mut s = XmssIndexState::default();
        // first use of a key at index 0 is allowed
        assert!(s.check_and_observe(K1, 0).is_ok());
        s.commit();
        // reusing index 0 for the same key is rejected
        assert!(s.check_and_observe(K1, 0).is_err());
        s.discard();
        // strictly greater index is allowed
        assert!(s.check_and_observe(K1, 1).is_ok());
        s.commit();
        // rolling back to a used index is rejected
        assert!(s.check_and_observe(K1, 0).is_err());
        assert!(s.check_and_observe(K1, 1).is_err());
        // a different key is tracked independently
        assert!(s.check_and_observe(K2, 0).is_ok());
    }

    #[test]
    fn lamport_one_time_per_key() {
        let mut s = XmssIndexState::default();
        // first use of a Lamport key is allowed
        assert!(s.check_and_observe_lamport(K1).is_ok());
        s.commit();
        // a second use of the same key is rejected
        assert!(s.check_and_observe_lamport(K1).is_err());
        // a different Lamport key is independent
        assert!(s.check_and_observe_lamport(K2).is_ok());
        s.commit();
        assert!(s.check_and_observe_lamport(K2).is_err());
    }

    #[test]
    fn lamport_reuse_within_entry_tolerated_until_commit() {
        let mut s = XmssIndexState::default();
        // a lock-script retry may re-observe the same key before commit
        assert!(s.check_and_observe_lamport(K1).is_ok());
        assert!(s.check_and_observe_lamport(K1).is_ok());
        // discarding a failed entry frees the key again
        s.discard();
        assert!(s.check_and_observe_lamport(K1).is_ok());
        s.commit();
        // now committed, reuse is rejected
        assert!(s.check_and_observe_lamport(K1).is_err());
    }

    #[test]
    fn observations_only_count_after_commit() {
        let mut s = XmssIndexState::default();
        // observe index 5 but do NOT commit (entry failed)
        assert!(s.check_and_observe(K1, 5).is_ok());
        s.discard();
        // index 0 is still allowed because 5 was never committed
        assert!(s.check_and_observe(K1, 0).is_ok());
    }

    #[test]
    fn same_index_within_one_entry_is_tolerated_until_commit() {
        // a lock-script retry may re-observe the same (key, index) before commit
        let mut s = XmssIndexState::default();
        assert!(s.check_and_observe(K1, 3).is_ok());
        assert!(s.check_and_observe(K1, 3).is_ok()); // retry, same index, pre-commit
        s.commit();
        // after commit, 3 cannot be used again
        assert!(s.check_and_observe(K1, 3).is_err());
    }

    #[test]
    fn enforce_is_noop_without_installed_guard() {
        // no guard installed on this thread → enforcement is a no-op
        assert!(enforce_xmss_index(K1, 0).is_ok());
        assert!(enforce_xmss_index(K1, 0).is_ok());
    }

    #[test]
    fn raii_guard_installs_and_restores() {
        assert!(enforce_xmss_index(K1, 7).is_ok()); // no guard → ok
        {
            let guard = XmssEnforcement::install();
            assert!(enforce_xmss_index(K1, 0).is_ok());
            guard.commit_entry();
            // reuse now rejected while guard is installed
            assert!(enforce_xmss_index(K1, 0).is_err());
        }
        // guard dropped → enforcement disabled again
        assert!(enforce_xmss_index(K1, 0).is_ok());
    }

    #[test]
    fn merkle_monotonic_index_per_key() {
        let mut s = XmssIndexState::default();
        // first use of a merkle key at leaf 0 is allowed
        assert!(s.check_and_observe_merkle(K1, 0).is_ok());
        s.commit();
        // reusing leaf 0 for the same key is rejected
        assert!(s.check_and_observe_merkle(K1, 0).is_err());
        s.discard();
        // the next leaf in order is allowed
        assert!(s.check_and_observe_merkle(K1, 1).is_ok());
        s.commit();
        // rolling back to a consumed leaf is rejected
        assert!(s.check_and_observe_merkle(K1, 0).is_err());
        assert!(s.check_and_observe_merkle(K1, 1).is_err());
        // a different merkle key is tracked independently
        assert!(s.check_and_observe_merkle(K2, 0).is_ok());
    }

    #[test]
    fn merkle_same_index_within_one_entry_is_tolerated_until_commit() {
        // a lock-script retry may re-observe the same (key, leaf) before commit
        let mut s = XmssIndexState::default();
        assert!(s.check_and_observe_merkle(K1, 1).is_ok());
        assert!(s.check_and_observe_merkle(K1, 1).is_ok());
        s.commit();
        assert!(s.check_and_observe_merkle(K1, 1).is_err());
    }

    #[test]
    fn merkle_observations_only_count_after_commit() {
        let mut s = XmssIndexState::default();
        // observe leaf 1 but do NOT commit (entry failed)
        assert!(s.check_and_observe_merkle(K1, 1).is_ok());
        s.discard();
        // leaf 0 is still allowed because 1 was never committed
        assert!(s.check_and_observe_merkle(K1, 0).is_ok());
    }

    #[test]
    fn merkle_enforce_is_noop_without_installed_guard() {
        assert!(enforce_lamport_merkle(K1, 0).is_ok());
        assert!(enforce_lamport_merkle(K1, 0).is_ok());
    }

    #[test]
    fn merkle_raii_guard_installs_and_restores() {
        assert!(enforce_lamport_merkle(K1, 5).is_ok()); // no guard → ok
        {
            let guard = XmssEnforcement::install();
            assert!(enforce_lamport_merkle(K1, 0).is_ok());
            guard.commit_entry();
            // reuse now rejected while guard is installed
            assert!(enforce_lamport_merkle(K1, 0).is_err());
            // strictly greater leaf is accepted
            assert!(enforce_lamport_merkle(K1, 1).is_ok());
            guard.discard_entry();
        }
        // guard dropped → enforcement disabled again
        assert!(enforce_lamport_merkle(K1, 0).is_ok());
    }
}
