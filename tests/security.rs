#![allow(
    clippy::string_lit_as_bytes,
    clippy::manual_string_new,
    clippy::needless_pass_by_value,
    clippy::uninlined_format_args,
    clippy::needless_collect,
    clippy::cast_sign_loss
)]
// SPDX-License-Identifier: Apache-2.0

//! Security tests for WACC VM resource limits and protections

use std::collections::BTreeMap;
use wacc::{
    security::{allowed_algorithms, SecurityLimits},
    storage::{Pairs, Stack},
    types::{CheckCount, ContextPath, FuelAmount},
    vm::{Builder, Context, Value},
};
use wasmtime::StoreLimitsBuilder;

const MEMORY_LIMIT: usize = 1 << 22; /* 4MB */

#[derive(Default, Clone)]
struct Kvp {
    pub pairs: BTreeMap<String, Value>,
}

impl Pairs for Kvp {
    fn get(&self, key: &str) -> Option<Value> {
        self.pairs.get(key).cloned()
    }

    fn put(&mut self, key: &str, value: &Value) -> Option<Value> {
        self.pairs.insert(key.to_string(), value.clone())
    }
}

#[derive(Default, Clone)]
struct Stk {
    pub stack: Vec<Value>,
}

impl Stack for Stk {
    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    fn pop(&mut self) -> Option<Value> {
        self.stack.pop()
    }

    fn top(&self) -> Option<Value> {
        self.stack.last().cloned()
    }

    fn peek(&self, idx: usize) -> Option<Value> {
        if idx >= self.stack.len() {
            return None;
        }
        Some(self.stack[self.stack.len() - 1 - idx].clone())
    }

    fn len(&self) -> usize {
        self.stack.len()
    }

    fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }
}

/// Simple WASM module that returns success (for testing limits)
const SIMPLE_WASM: &[u8] = &[
    0x00, 0x61, 0x73, 0x6d, // WASM magic
    0x01, 0x00, 0x00, 0x00, // version 1
    0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7f, // type section: func () -> i32
    0x03, 0x02, 0x01, 0x00, // function section
    0x07, 0x0a, 0x01, 0x06, 0x74, 0x65, 0x73, 0x74, 0x65, 0x64, // export section: "tested"
    0x00, 0x00, // export func 0
    0x0a, 0x06, 0x01, 0x04, 0x00, 0x41, 0x01, 0x0b, // code section: return 1
];

#[test]
fn test_default_fuel_enforced() {
    // Verify that Builder::new() sets default fuel
    let kvp = Kvp::default();
    let pstack = Stk::default();
    let rstack = Stk::default();

    let context = Context {
        current: Box::new(kvp.clone()),
        proposed: Box::new(kvp),
        pstack: Box::new(pstack),
        rstack: Box::new(rstack),
        check_count: CheckCount::zero(),
        write_idx: 0,
        context: ContextPath::root(),
        log: Vec::default(),
        limiter: StoreLimitsBuilder::new().memory_size(MEMORY_LIMIT).build(),
    };

    // Builder::new() should automatically set default fuel
    let result = Builder::new()
        .with_context(context)
        .with_bytes(SIMPLE_WASM)
        .try_build();

    assert!(result.is_ok(), "Builder should succeed with default fuel");
}

#[test]
fn test_strict_limits() {
    // Verify that new_strict() uses more restrictive limits
    let limits = SecurityLimits::STRICT;
    assert!(limits.max_fuel.as_u64() < SecurityLimits::PRODUCTION.max_fuel.as_u64());
    assert!(limits.max_checks.as_usize() < SecurityLimits::PRODUCTION.max_checks.as_usize());
    assert!(limits.max_memory_size < SecurityLimits::PRODUCTION.max_memory_size);
}

#[test]
fn test_check_count_limit_enforced() {
    // Test that exceeding check count limit is prevented
    let pstack = Stk::default();
    let rstack = Stk::default();
    let kvp = Kvp::default();

    let mut context = Context {
        current: Box::new(kvp.clone()),
        proposed: Box::new(kvp),
        pstack: Box::new(pstack),
        rstack: Box::new(rstack),
        check_count: CheckCount::new(CheckCount::MAX - 1),
        write_idx: 0,
        context: ContextPath::root(),
        log: Vec::default(),
        limiter: StoreLimitsBuilder::new().build(),
    };

    // One more increment should work
    let result = context.check_fail("test");
    assert_eq!(result.i32(), Some(0)); // Should be WASM_FALSE

    // At max, succeed should fail
    let result = context.succeed();
    assert_eq!(result.i32(), Some(0)); // Should fail with max exceeded
}

#[test]
fn test_log_size_limit_enforced() {
    let pstack = Stk::default();
    let rstack = Stk::default();
    let kvp = Kvp::default();

    let mut context = Context {
        current: Box::new(kvp.clone()),
        proposed: Box::new(kvp),
        pstack: Box::new(pstack),
        rstack: Box::new(rstack),
        check_count: CheckCount::zero(),
        write_idx: 0,
        context: ContextPath::root(),
        log: vec![0u8; 64 * 1024 - 10], // Almost at limit
        limiter: StoreLimitsBuilder::new().build(),
    };

    // Small log should succeed
    let result = context.log("test");
    assert_eq!(result.i32(), Some(1)); // WASM_TRUE

    // Large log should fail (exceeds limit)
    let large_message = "x".repeat(100);
    let result = context.log(&large_message);
    assert_eq!(result.i32(), Some(0)); // WASM_FALSE - limit exceeded
}

#[test]
fn test_hash_algorithm_whitelist() {
    // Secure algorithms should be allowed
    assert!(allowed_algorithms::is_hash_allowed(0x12)); // sha2-256
    assert!(allowed_algorithms::is_hash_allowed(0x13)); // sha2-512
    assert!(allowed_algorithms::is_hash_allowed(0x16)); // sha3-256
    assert!(allowed_algorithms::is_hash_allowed(0x1e)); // blake3
    assert!(allowed_algorithms::is_hash_allowed(0xb220)); // blake2b-256

    // Broken/weak algorithms should be rejected
    assert!(!allowed_algorithms::is_hash_allowed(0x11)); // SHA-1 (broken)
    assert!(!allowed_algorithms::is_hash_allowed(0xd5)); // MD5 (broken)
    assert!(!allowed_algorithms::is_hash_allowed(0xd4)); // MD4 (broken)
}

#[test]
fn test_signature_algorithm_whitelist() {
    // Modern secure algorithms
    assert!(allowed_algorithms::is_sig_allowed(0xed)); // ed25519-pub
    assert!(allowed_algorithms::is_sig_allowed(0xe7)); // secp256k1-pub
    assert!(allowed_algorithms::is_sig_allowed(0x1200)); // p256-pub
    assert!(allowed_algorithms::is_sig_allowed(0xea)); // bls12_381-g1-pub

    // Unknown/untrusted algorithms
    assert!(!allowed_algorithms::is_sig_allowed(0x9999));
}

#[test]
fn test_excessive_fuel_rejected() {
    let kvp = Kvp::default();
    let pstack = Stk::default();
    let rstack = Stk::default();

    let context = Context {
        current: Box::new(kvp.clone()),
        proposed: Box::new(kvp),
        pstack: Box::new(pstack),
        rstack: Box::new(rstack),
        check_count: CheckCount::zero(),
        write_idx: 0,
        context: ContextPath::root(),
        log: Vec::default(),
        limiter: StoreLimitsBuilder::new().build(),
    };

    // Trying to set fuel beyond security limit should fail
    let excessive_fuel = FuelAmount::new(FuelAmount::MAX);
    let result = Builder::new()
        .with_context(context)
        .with_bytes(SIMPLE_WASM)
        .with_fuel(excessive_fuel)
        .try_build();

    // Should fail because excessive_fuel > PRODUCTION limit
    assert!(result.is_err(), "Excessive fuel should be rejected");
}

#[test]
fn test_context_path_validation() {
    // Valid paths
    assert!(ContextPath::new_checked("/valid/path").is_some());
    assert!(ContextPath::new_checked("").is_some()); // Root is valid

    // Invalid paths
    assert!(ContextPath::new_checked("x".repeat(2000)).is_none()); // Too long
    assert!(ContextPath::new_checked("path\0with\0nulls").is_none()); // Null bytes

    // Depth limit
    let mut path = ContextPath::root();
    for i in 0..ContextPath::MAX_DEPTH {
        path = path.branch(&format!("level{i}")).unwrap();
    }
    // Next branch should fail
    assert!(
        path.branch("overflow").is_none(),
        "Should enforce depth limit"
    );
}

#[test]
fn test_memory_overflow_protection() {
    use wacc::types::{WasmPtr, WasmSize};

    // Overflow should be detected
    let ptr = WasmPtr::new(u32::MAX - 10);
    let size = WasmSize::new(20);
    assert!(ptr.would_overflow(size));

    // Valid access should not overflow
    let ptr = WasmPtr::new(1000);
    let size = WasmSize::new(100);
    assert!(!ptr.would_overflow(size));
}

#[test]
fn test_memory_size_validation() {
    use wacc::types::WasmSize;

    // Valid sizes
    assert!(WasmSize::new_checked(1024).is_some());
    assert!(WasmSize::new_checked(WasmSize::MAX_SIZE).is_some());

    // Invalid sizes (too large)
    assert!(WasmSize::new_checked(WasmSize::MAX_SIZE + 1).is_none());
    assert!(WasmSize::new_checked(u32::MAX).is_none());
}
