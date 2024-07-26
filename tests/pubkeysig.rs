// SPDX-License-Identifier: FSL-1.1
use std::{collections::BTreeMap, fs::read, path::PathBuf};
use wacc::{storage::{Pairs, Stack}, vm::{Builder, Context, Instance, Value}};
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
    func: &str,
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
        write_idx: 0,
        context: "/forks/child/".to_string(),
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
    let result = instance.run(func).unwrap();

    assert_eq!(expected, result);
    instance
}

#[derive(Default)]
struct Kvp {
    pub pairs: BTreeMap<String, Value>,
}

impl Pairs for Kvp {
    /// get a value associated with the key
    fn get(&self, key: &str) -> Option<Value> {
        self.pairs.get(&key.to_string()).cloned()
    }

    /// add a key-value pair to the storage, return previous value if overwritten
    fn put(&mut self, key: &str, value: &Value) -> Option<Value> {
        self.pairs.insert(key.to_string(), value.clone())
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
fn test_pubkeysig_wast() {
    // create the stack to use
    let mut pstack = Stk::default();
    let mut rstack = Stk::default();

    { // unlock
        // set up the key-value pair store with the message and signature data
        let mut kvp_unlock = Kvp::default();

        // NOTE: this is an example of a signed string message
        let _ = kvp_unlock.put("/entry/", &"for great justice, move every zig!".to_string().into());
        let _ = kvp_unlock.put("/entry/proof", &hex::decode("3983a6c0060001004076fee92ca796162b5e37a84b4150da685d636491b43c1e2a1fab392a7337553502588a609075b56c46b5c033b260d8d314b584e396fc2221c55f54843679ee08").unwrap().into());

        // load the unlock script
        let script = load_wast("pubkeysig_unlock.wast");

        // run the unlock script to set up the stack
        let mut instance = test_example(script, "for_great_justice", true, &kvp_unlock, &mut pstack, &mut rstack);

        // check that the stack is what we expect
        let mut ctx = instance.store.as_context_mut();
        let context = ctx.data_mut();
        assert_eq!(2, context.pstack.len());
        assert_eq!(context.pstack.top(), Some(Value::Bin { hint: "".to_string(), data: hex::decode("3983a6c0060001004076fee92ca796162b5e37a84b4150da685d636491b43c1e2a1fab392a7337553502588a609075b56c46b5c033b260d8d314b584e396fc2221c55f54843679ee08").unwrap() }));
        assert_eq!(context.pstack.peek(1), Some(Value::Str { hint: "".to_string(), data: "for great justice, move every zig!".to_string() }))
    }

    { // lock
        // set up the key-value pair store with the encoded Multikey
        let mut kvp_lock = Kvp::default();
        let _ = kvp_lock.put("/pubkey", &hex::decode("3aed010874657374206b657901012084d515ef051e07d597f3c14ac09e5a9d5012c659c196d96db5c6b98ea552f603").unwrap().into());

        // load the lock script
        let script = load_wast("pubkeysig_lock.wast");

        // run the lock script to check the proof
        let mut instance = test_example(script, "move_every_zig", true, &kvp_lock, &mut pstack, &mut rstack);

        // check that the stack is what we expect
        let mut ctx = instance.store.as_context_mut();
        let context = ctx.data_mut();
        assert_eq!(1, context.rstack.len());
        assert_eq!(context.rstack.top(), Some(Value::Success(0)));
    }
}

#[test]
fn test_pubkeysig_wasm() {
    // create the stack to use
    let mut pstack = Stk::default();
    let mut rstack = Stk::default();

    { // unlock
        // set up the key-value pair store with the message and signature data
        let mut kvp_unlock = Kvp::default();

        // NOTE: this is an example of a signed binary message
        let _ = kvp_unlock.put("/entry/", &"for great justice, move every zig!".as_bytes().into());
        let _ = kvp_unlock.put("/entry/proof", &hex::decode("3983a6c0060001004076fee92ca796162b5e37a84b4150da685d636491b43c1e2a1fab392a7337553502588a609075b56c46b5c033b260d8d314b584e396fc2221c55f54843679ee08").unwrap().into());

        // load the unlock script
        let script = load_wasm("pubkeysig_unlock.wasm");

        // run the unlock script to set up the stack
        let mut instance = test_example(script, "for_great_justice", true, &kvp_unlock, &mut pstack, &mut rstack);

        // check that the stack is what we expect
        let mut ctx = instance.store.as_context_mut();
        let context = ctx.data_mut();
        assert_eq!(2, context.pstack.len());
        assert_eq!(context.pstack.top(), Some(Value::Bin { hint: "".to_string(), data: hex::decode("3983a6c0060001004076fee92ca796162b5e37a84b4150da685d636491b43c1e2a1fab392a7337553502588a609075b56c46b5c033b260d8d314b584e396fc2221c55f54843679ee08").unwrap() }));
        assert_eq!(context.pstack.peek(1), Some(Value::Bin { hint: "".to_string(), data: b"for great justice, move every zig!".to_vec() }));
    }

    { // lock
        // set up the key-value pair store with the encoded Multikey
        let mut kvp_lock = Kvp::default();
        let _ = kvp_lock.put("/pubkey", &hex::decode("3aed010874657374206b657901012084d515ef051e07d597f3c14ac09e5a9d5012c659c196d96db5c6b98ea552f603").unwrap().into());

        // load the lock script
        let script = load_wasm("pubkeysig_lock.wasm");

        // run the lock script to check the proof
        let mut instance = test_example(script, "move_every_zig", true, &kvp_lock, &mut pstack, &mut rstack);

        // check that the stack is what we expect
        let mut ctx = instance.store.as_context_mut();
        let context = ctx.data_mut();
        assert_eq!(1, context.rstack.len());
        assert_eq!(context.rstack.top(), Some(Value::Success(0)));
    }
}
