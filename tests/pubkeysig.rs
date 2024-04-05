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
fn test_pubkeysig_wast() {
    // create the stack to use
    let mut pstack = Stk::default();
    let mut rstack = Stk::default();

    { // unlock
        // set up the key-value pair store with the message and signature data
        let mut kvp_unlock = Kvp::default();

        // NOTE: this is an example of a signed string message
        let _ = kvp_unlock.put(&"/entry/".try_into().unwrap(), &"for great justice, move every zig!".to_string().into());
        let _ = kvp_unlock.put(&"/entry/proof".try_into().unwrap(), &hex::decode("39eda1030001004033a3f4e64b31ef09956fe411ba9ab6de20a31f3384ce1599dec24eb3eb0a7d4fa83d1def1294828b5bdebdb871f8a55a4eaf1983a4f48cfe51fa15a1ecadf006").unwrap().into());

        // load the unlock script
        let script = load_wast("pubkeysig_unlock.wast");

        // run the unlock script to set up the stack
        let mut instance = test_example(script, "for_great_justice", true, &kvp_unlock, &mut pstack, &mut rstack);

        // check that the stack is what we expect
        let mut ctx = instance.store.as_context_mut();
        let context = ctx.data_mut();
        assert_eq!(2, context.pstack.len());
        assert_eq!(context.pstack.top(), Some(Value::Bin(hex::decode("39eda1030001004033a3f4e64b31ef09956fe411ba9ab6de20a31f3384ce1599dec24eb3eb0a7d4fa83d1def1294828b5bdebdb871f8a55a4eaf1983a4f48cfe51fa15a1ecadf006").unwrap())));
        assert_eq!(context.pstack.peek(1), Some(Value::Str("for great justice, move every zig!".to_string())))
    }

    { // lock
        // set up the key-value pair store with the encoded Multikey
        let mut kvp_lock = Kvp::default();
        let _ = kvp_lock.put(&"/pubkey".try_into().unwrap(), &hex::decode("3aed010874657374206b6579010120fee578e370fc6a13bb4965e2be36edbba62e9300a53fca73b01ab373cffd6eba").unwrap().into());

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
        let _ = kvp_unlock.put(&"/entry/".try_into().unwrap(), &"for great justice, move every zig!".as_bytes().into());
        let _ = kvp_unlock.put(&"/entry/proof".try_into().unwrap(), &hex::decode("39eda10300010040d31e5f6f57e01e638b8f6f0b3b560b808dea0700435044077c2a88b95e733490dd53f1b64ca68595795685541ca7b455c5b480c281ea5e35a0d3fc8645e08a07").unwrap().into());

        // load the unlock script
        let script = load_wasm("pubkeysig_unlock.wasm");

        // run the unlock script to set up the stack
        let mut instance = test_example(script, "for_great_justice", true, &kvp_unlock, &mut pstack, &mut rstack);

        // check that the stack is what we expect
        let mut ctx = instance.store.as_context_mut();
        let context = ctx.data_mut();
        assert_eq!(2, context.pstack.len());
        assert_eq!(context.pstack.top(), Some(Value::Bin(hex::decode("39eda10300010040d31e5f6f57e01e638b8f6f0b3b560b808dea0700435044077c2a88b95e733490dd53f1b64ca68595795685541ca7b455c5b480c281ea5e35a0d3fc8645e08a07").unwrap())));
        assert_eq!(context.pstack.peek(1), Some(Value::Bin(b"for great justice, move every zig!".to_vec())));
    }

    { // lock
        // set up the key-value pair store with the encoded Multikey
        let mut kvp_lock = Kvp::default();
        let _ = kvp_lock.put(&"/pubkey".try_into().unwrap(), &hex::decode("3aed010874657374206b6579010120de972f8ef7b4056d1f4e55b500945cf0ce04407d391bfa5b62459d90e0e00edb").unwrap().into());

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
