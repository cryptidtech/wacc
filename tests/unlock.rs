#![allow(
    clippy::string_lit_as_bytes,
    clippy::manual_string_new,
    clippy::needless_pass_by_value,
    clippy::uninlined_format_args,
    clippy::needless_collect,
    clippy::cast_sign_loss
)]
// SPDX-License-Identifier: Apache-2.0
use std::{collections::BTreeMap, fs::read, path::PathBuf, sync::Arc};
use wacc::types::{CheckCount, ContextPath};
use wacc::{
    storage::{Pairs, Stack},
    vm::{Builder, Context, Instance, Value},
};
use wasmtime::{AsContextMut, StoreLimitsBuilder};

const MEMORY_LIMIT: usize = 1 << 22; /* 4MB */

fn load_wasm(file_name: &str) -> Option<Vec<u8>> {
    let mut pb = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    pb.push("target");
    pb.push(file_name);
    println!("trying to load: {:?}", pb.as_os_str());
    read(&pb).ok()
}

fn load_wast(file_name: &str) -> Vec<u8> {
    let mut pb = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    pb.push("examples/wacc/wast");
    pb.push(file_name);
    println!("trying to load: {:?}", pb.as_os_str());
    read(&pb).unwrap_or_else(|_| panic!("WAST file {} must exist", file_name))
}

fn test_example(
    script: Vec<u8>,
    expected: bool,
    current: Kvp,
    proposed: Kvp,
    pstack: Stk,
    rstack: Stk,
) -> Instance {
    // build the context
    let context = Context {
        current: Box::new(current),
        proposed: Box::new(proposed),
        pstack: Box::new(pstack),
        rstack: Box::new(rstack),
        check_count: CheckCount::zero(),
        write_idx: 0,
        context: ContextPath::new("/forks/child/"),
        log: Vec::default(),
        limiter: StoreLimitsBuilder::new()
            .memory_size(MEMORY_LIMIT)
            .instances(2)
            .memories(1)
            .build(),
    };

    // construct the instance
    let mut instance = Builder::new()
        .with_context(context)
        .with_bytes(&script)
        .try_build()
        .unwrap();

    // execute the instance
    let result = instance.run("for_great_justice").unwrap();

    assert_eq!(expected, result);
    instance
}

#[derive(Default, Clone)]
struct Kvp {
    pub pairs: BTreeMap<String, Value>,
}

impl Pairs for Kvp {
    /// get a value associated with the key
    fn get(&self, key: &str) -> Option<Value> {
        self.pairs.get(key).cloned()
    }

    /// add a key-value pair to the storage, return previous value if overwritten
    fn put(&mut self, key: &str, value: &Value) -> Option<Value> {
        self.pairs.insert(key.to_string(), value.clone())
    }
}

#[derive(Default, Clone)]
struct Stk {
    pub stack: Vec<Value>,
}

impl Stack for Stk {
    /// push a value onto the stack
    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    /// remove the last top value from the stack
    fn pop(&mut self) -> Option<Value> {
        self.stack.pop()
    }

    /// get a reference to the top value on the stack
    fn top(&self) -> Option<Value> {
        self.stack.last().cloned()
    }

    /// peek at the item at the given index
    fn peek(&self, idx: usize) -> Option<Value> {
        if idx >= self.stack.len() {
            return None;
        }
        Some(self.stack[self.stack.len() - 1 - idx].clone())
    }

    /// return the number of values on the stack
    fn len(&self) -> usize {
        self.stack.len()
    }

    /// return if the stack is empty
    fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }
}

#[test]
fn test_unlock_wast() {
    // set up the key-value pair store
    let mut kvp = Kvp::default();
    let _ = kvp.put("/entry/", &"foo".as_bytes().into());
    let _ = kvp.put("/entry/proof", &"bar".as_bytes().into());

    // load the script
    let pstack = Stk::default();
    let rstack = Stk::default();
    let script = load_wast("unlock.wast");
    let mut instance = test_example(script, true, kvp.clone(), kvp, pstack, rstack);

    // Get the context
    let mut ctx = instance.store.as_context_mut();
    let context = ctx.data_mut();
    assert_eq!(1, context.pstack.len());
    assert_eq!(
        context.pstack.top(),
        Some(Value::Bin {
            hint: "".to_string(),
            data: Arc::from(&b"bar"[..])
        })
    );
    assert_eq!(0, context.rstack.len());
}

#[test]
fn test_unlock_wasm() {
    // Skip test if WASM file not built (requires wat2wasm tool)
    let Some(script) = load_wasm("unlock.wasm") else {
        eprintln!("Skipping test_unlock_wasm: unlock.wasm not found (run 'make' in examples/wast to build)");
        return;
    };

    // set up the key-value pair store
    let mut kvp = Kvp::default();
    let _ = kvp.put("/entry/", &"foo".as_bytes().into());
    let _ = kvp.put("/entry/proof", &"bar".as_bytes().into());

    // load the script
    let pstack = Stk::default();
    let rstack = Stk::default();
    let mut instance = test_example(script, true, kvp.clone(), kvp, pstack, rstack);

    // Get the context
    let mut ctx = instance.store.as_context_mut();
    let context = ctx.data_mut();
    assert_eq!(1, context.pstack.len());
    assert_eq!(
        context.pstack.top(),
        Some(Value::Bin {
            hint: "".to_string(),
            data: Arc::from(&b"bar"[..])
        })
    );
    assert_eq!(0, context.rstack.len());
}
