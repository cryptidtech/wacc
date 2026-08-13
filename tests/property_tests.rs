#![allow(
    clippy::string_lit_as_bytes,
    clippy::manual_string_new,
    clippy::needless_pass_by_value,
    clippy::uninlined_format_args,
    clippy::needless_collect,
    clippy::cast_sign_loss
)]
// SPDX-License-Identifier: Apache-2.0

//! Property-based tests for WACC VM
//!
//! These tests use proptest to verify invariants hold across
//! a wide range of randomly generated inputs.

use proptest::prelude::*;
use wacc::types::{CheckCount, ContextPath, FuelAmount, WasmPtr, WasmSize};

// Property: CheckCount increment is monotonic
proptest! {
    #[test]
    fn prop_check_count_monotonic(count in 0usize..CheckCount::MAX) {
        let c1 = CheckCount::new(count);
        if let Some(c2) = c1.increment() {
            prop_assert!(c2.as_usize() > c1.as_usize());
            prop_assert_eq!(c2.as_usize(), c1.as_usize() + 1);
        }
    }
}

// Property: CheckCount at max cannot increment
proptest! {
    #[test]
    fn prop_check_count_max_cannot_increment(extra in 0usize..1000) {
        let count = CheckCount::new(CheckCount::MAX + extra);
        prop_assert!(count.at_max());
        prop_assert!(count.increment().is_none());
    }
}

// Property: FuelAmount subtraction is correct
proptest! {
    #[test]
    fn prop_fuel_subtraction(
        initial in 1u64..FuelAmount::MAX,
        consumed in 0u64..1000u64
    ) {
        let fuel = FuelAmount::new(initial);

        if let Some(remaining) = fuel.checked_sub(consumed) {
            prop_assert_eq!(remaining.as_u64(), initial - consumed);
            prop_assert!(remaining.as_u64() < fuel.as_u64() || consumed == 0);
        } else {
            // Underflow - consumed must be > initial
            prop_assert!(consumed > initial);
        }
    }
}

// Property: WasmPtr + WasmSize overflow detection is correct
proptest! {
    #[test]
    fn prop_wasm_overflow_detection(
        ptr in 0u32..u32::MAX,
        size in 0u32..1000u32
    ) {
        let p = WasmPtr::new(ptr);
        let s = WasmSize::new(size);

        let would_overflow = p.would_overflow(s);
        let actual_overflow = u64::from(ptr) + u64::from(size) > u64::from(u32::MAX);

        prop_assert_eq!(would_overflow, actual_overflow);
    }
}

// Property: ContextPath branching maintains depth invariant
proptest! {
    #[test]
    fn prop_context_path_depth(
        segments in proptest::collection::vec("[a-z]{1,10}", 1..10)
    ) {
        let mut path = ContextPath::root();
        let mut depth = 0;

        for segment in segments {
            if let Some(new_path) = path.branch(&segment) {
                depth += 1;
                prop_assert_eq!(new_path.depth(), depth);
                path = new_path;
            } else {
                // Branching failed (e.g., max depth reached)
                break;
            }
        }

        // Depth should never exceed MAX_DEPTH
        prop_assert!(depth <= ContextPath::MAX_DEPTH);
    }
}

// Property: Valid paths can be created up to MAX_LENGTH
proptest! {
    #[test]
    fn prop_context_path_length(
        segment_len in 1usize..100
    ) {
        let segment = "x".repeat(segment_len);
        let path = ContextPath::root();

        if let Some(branched) = path.branch(&segment) {
            // If branching succeeded, length should be within limits
            prop_assert!(branched.as_str().len() <= ContextPath::MAX_LENGTH);
        }
    }
}

// Property: WasmSize validation is correct
proptest! {
    #[test]
    fn prop_wasm_size_validation(size in 0u32..u32::MAX) {
        let checked = WasmSize::new_checked(size);

        if size <= WasmSize::MAX_SIZE {
            prop_assert!(checked.is_some());
            prop_assert_eq!(checked.unwrap().as_u32(), size);
        } else {
            prop_assert!(checked.is_none());
        }
    }
}

// Property: FuelAmount subtraction doesn't exceed original
proptest! {
    #[test]
    fn prop_fuel_never_exceeds_original(
        initial in 1u64..10000u64,
        consumed in 0u64..10000u64
    ) {
        let fuel = FuelAmount::new(initial);
        if consumed <= initial {
            if let Some(remaining) = fuel.checked_sub(consumed) {
                prop_assert!(remaining.as_u64() <= fuel.as_u64());
            }
        }
    }
}

// Property: CheckCount zero is_zero
proptest! {
    #[test]
    fn prop_check_count_zero(_dummy in 0..1) {
        let count = CheckCount::zero();
        prop_assert_eq!(count.as_usize(), 0);
        prop_assert!(count.is_zero());
    }
}

// Property: WasmPtr checked_add is commutative with addition
proptest! {
    #[test]
    fn prop_wasm_ptr_checked_add(
        ptr in 0u32..1000u32,
        offset in 0u32..1000u32
    ) {
        let p = WasmPtr::new(ptr);

        if let Some(result) = p.checked_add(offset) {
            prop_assert_eq!(result.as_u32(), ptr + offset);
        } else {
            // Overflow occurred
            prop_assert!(u64::from(ptr) + u64::from(offset) > u64::from(u32::MAX));
        }
    }
}
