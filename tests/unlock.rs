// SPDX-License-Identifier: FSL-1.1
use std::{collections::BTreeMap, fs::read, path::PathBuf};
use wacc::{storage::{Pairs, Stack}, vm::{Builder, Context, Instance, Key, Value}};
use wasmtime::{AsContextMut, StoreLimitsBuilder};

const MEMORY_LIMIT: usize = 1 << 22; /* 4MB */

fn load_wasm(file_name: &str) -> Vec<u8> {
    let mut pb = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    pb.push("target");
    pb.push(file_name);
    println!("trying to load: {:?}", pb.as_os_str());
    read(&pb).expect(&format!("Error loading file {file_name}"))
}

fn load_wast(file_name: &str) -> Vec<u8> {
    let mut pb = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    pb.push("examples");
    pb.push("wast");
    pb.push(file_name);
    println!("trying to load: {:?}", pb.as_os_str());
    read(&pb).expect(&format!("Error loading file {file_name}"))
}

fn test_example<'a>(
    script: Vec<u8>,
    expected: bool,
    pairs: &'a Kvp,
    pstack: &'a mut Stk,
    rstack: &'a mut Stk,
) -> Instance<'a> {
    // build the context
    let context = Context {
        pairs,
        pstack,
        rstack,
        check_count: 0,
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

#[derive(Default)]
struct Kvp {
    pub pairs: BTreeMap<Key, Value>,
}

impl Pairs for Kvp {
    /// get a value associated with the key
    fn get(&self, key: &Key) -> Option<Value> {
        self.pairs.get(&key).cloned()
    }

    /// add a key-value pair to the storage, return previous value if overwritten
    fn put(&mut self, key: &Key, value: &Value) -> Option<Value> {
        self.pairs.insert(key.clone(), value.clone())
    }
}

#[derive(Default)]
struct Stk {
    pub stack: Vec<Value>
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
    let _ = kvp.put(&"/entry/".try_into().unwrap(), &"foo".as_bytes().into());
    let _ = kvp.put(&"/entry/proof".try_into().unwrap(), &"bar".as_bytes().into());

    // load the script
    let mut pstack = Stk::default();
    let mut rstack = Stk::default();
    let script = load_wast("unlock.wast");
    let mut instance = test_example(script, true, &kvp, &mut pstack, &mut rstack);

    // Get the context
    let mut ctx = instance.store.as_context_mut();
    let context = ctx.data_mut();
    assert_eq!(2, context.pstack.len());
    assert_eq!(context.pstack.top(), Some(Value::Bin(b"bar".to_vec())));
    assert_eq!(context.pstack.peek(1), Some(Value::Bin(b"foo".to_vec())));
    assert_eq!(0, context.rstack.len());
}

#[test]
fn test_unlock_wasm() {
    // set up the key-value pair store
    let mut kvp = Kvp::default();
    let _ = kvp.put(&"/entry/".try_into().unwrap(), &"foo".as_bytes().into());
    let _ = kvp.put(&"/entry/proof".try_into().unwrap(), &"bar".as_bytes().into());

    // load the script
    let mut pstack = Stk::default();
    let mut rstack = Stk::default();
    let script = load_wasm("unlock.wasm");
    let mut instance = test_example(script, true, &kvp, &mut pstack, &mut rstack);

    // Get the context
    let mut ctx = instance.store.as_context_mut();
    let context = ctx.data_mut();
    assert_eq!(2, context.pstack.len());
    assert_eq!(context.pstack.top(), Some(Value::Bin(b"bar".to_vec())));
    assert_eq!(context.pstack.peek(1), Some(Value::Bin(b"foo".to_vec())));
    assert_eq!(0, context.rstack.len());
}
