#![allow(
    clippy::string_lit_as_bytes,
    clippy::manual_string_new,
    clippy::needless_pass_by_value,
    clippy::uninlined_format_args,
    clippy::needless_collect,
    clippy::cast_sign_loss
)]
// SPDX-License-Identifier: Apache-2.0

//! Edge case tests for WACC VM
//!
//! These tests verify correct behavior in boundary conditions and unusual scenarios.

use std::collections::BTreeMap;
use std::sync::Arc;
use wacc::{
    storage::{Pairs, Stack},
    types::{CheckCount, ContextPath, FuelAmount, LogSize, WasmPtr, WasmSize},
    vm::{Builder, Context, Value},
};
use wasmtime::StoreLimitsBuilder;

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

#[test]
fn test_empty_stack_operations() {
    let kvp = Kvp::default();
    let pstack = Stk::default();
    let rstack = Stk::default();

    let mut context = Context {
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

    // Pop from empty stack should fail
    let result = context.pop();
    assert_eq!(result.i32(), Some(0)); // WASM_FALSE

    // Should have pushed a Failure value
    match context.rstack.top() {
        Some(Value::Failure(_)) => {} // Expected
        other => panic!("Expected Failure, got {other:?}"),
    }
}

#[test]
fn test_missing_key_access() {
    let kvp = Kvp::default();
    let pstack = Stk::default();
    let rstack = Stk::default();

    let mut context = Context {
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

    // Push with missing key should fail
    let result = context.push("nonexistent");
    assert_eq!(result.i32(), Some(0)); // WASM_FALSE
}

#[test]
fn test_check_count_wraparound() {
    let kvp = Kvp::default();
    let pstack = Stk::default();
    let rstack = Stk::default();

    let mut context = Context {
        current: Box::new(kvp.clone()),
        proposed: Box::new(kvp),
        pstack: Box::new(pstack),
        rstack: Box::new(rstack),
        check_count: CheckCount::new(CheckCount::MAX),
        write_idx: 0,
        context: ContextPath::root(),
        log: Vec::default(),
        limiter: StoreLimitsBuilder::new().build(),
    };

    // At max, check_fail should fail
    let result = context.check_fail("test");
    assert_eq!(result.i32(), Some(0)); // Should fail

    // Error should be about max count
    match context.rstack.top() {
        Some(Value::Failure(msg)) => {
            assert!(msg.contains("maximum check count exceeded"));
        }
        other => panic!("Expected Failure with max count message, got {other:?}"),
    }
}

#[test]
fn test_log_buffer_full() {
    let kvp = Kvp::default();
    let pstack = Stk::default();
    let rstack = Stk::default();

    // Start with log buffer almost full
    let mut context = Context {
        current: Box::new(kvp.clone()),
        proposed: Box::new(kvp),
        pstack: Box::new(pstack),
        rstack: Box::new(rstack),
        check_count: CheckCount::zero(),
        write_idx: 0,
        context: ContextPath::root(),
        log: vec![0u8; LogSize::MAX - 5], // Almost full
        limiter: StoreLimitsBuilder::new().build(),
    };

    // Small log should fail (5 bytes left, but need room for newline too)
    let result = context.log("test message"); // ~13 bytes
    assert_eq!(result.i32(), Some(0)); // WASM_FALSE
}

#[test]
fn test_zero_fuel() {
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

    // Very low fuel should be rejected or run out quickly
    let result = Builder::new()
        .with_fuel(FuelAmount::new(1))
        .with_context(context)
        .with_bytes([
            0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x05, 0x01, 0x60, 0x00, 0x01,
            0x7f, 0x03, 0x02, 0x01, 0x00, 0x07, 0x08, 0x01, 0x04, 0x74, 0x65, 0x73, 0x74, 0x00,
            0x00, 0x0a, 0x06, 0x01, 0x04, 0x00, 0x41, 0x01, 0x0b,
        ])
        .try_build();

    // Builder should accept low fuel
    assert!(result.is_ok());
}

#[test]
fn test_wasm_ptr_max_value() {
    let ptr = WasmPtr::new(u32::MAX);
    let size = WasmSize::new(1);

    // Should detect overflow
    assert!(ptr.would_overflow(size));
}

#[test]
fn test_wasm_size_zero() {
    let size = WasmSize::new(0);
    assert!(size.is_zero());

    let ptr = WasmPtr::new(1000);
    assert!(!ptr.would_overflow(size)); // Zero size never overflows
}

#[test]
fn test_context_path_empty_segment() {
    let path = ContextPath::root();

    // Empty segment should be rejected
    assert!(path.branch("").is_none());
}

#[test]
fn test_context_path_with_separator() {
    let path = ContextPath::root();

    // Segment with separator should be rejected
    assert!(path.branch("bad/segment").is_none());
}

#[test]
fn test_context_path_null_byte() {
    // Path with null byte should be rejected
    assert!(ContextPath::new_checked("path\0with\0null").is_none());
}

#[test]
fn test_value_empty_data() {
    let value: Value = vec![].into();
    match value {
        Value::Bin { data, .. } => assert_eq!(data.len(), 0),
        _ => panic!("Expected Bin value"),
    }
}

#[test]
fn test_value_large_data() {
    // 1 MB of data
    let large_data = vec![0u8; 1024 * 1024];
    let value: Value = large_data.into();

    match value {
        Value::Bin { data, .. } => {
            assert_eq!(data.len(), 1024 * 1024);
            // Cloning should be cheap (Arc)
            let _cloned = Value::Bin {
                hint: String::new(),
                data: Arc::clone(&data),
            };
            // Both values share the same underlying data
            assert_eq!(data.len(), 1024 * 1024);
        }
        _ => panic!("Expected Bin value"),
    }
}

#[test]
fn test_check_count_boundary() {
    // At MAX - 1, increment should succeed
    let count = CheckCount::new(CheckCount::MAX - 1);
    assert!(count.increment().is_some());

    // At MAX, increment should fail
    let count = CheckCount::new(CheckCount::MAX);
    assert!(count.increment().is_none());
}

#[test]
fn test_fuel_amount_boundary() {
    // At valid range
    assert!(FuelAmount::new_checked(1000).is_some());
    assert!(FuelAmount::new_checked(FuelAmount::MAX).is_some());

    // Beyond max
    assert!(FuelAmount::new_checked(FuelAmount::MAX + 1).is_none());

    // Zero
    assert!(FuelAmount::new_checked(0).is_none());
}

#[test]
fn test_wasm_size_max_boundary() {
    // At limit
    assert!(WasmSize::new_checked(WasmSize::MAX_SIZE).is_some());

    // Over limit
    assert!(WasmSize::new_checked(WasmSize::MAX_SIZE + 1).is_none());
}
