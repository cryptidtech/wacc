use std::{collections::HashMap, convert::AsRef, fs::read, path::PathBuf};
use wacc::{storage::{Pairs, Stack}, vm};
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
) -> vm::Instance<'a> {
    // build the context
    let context = vm::Context {
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
    let mut instance = vm::Builder::new()
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
    pub pairs: HashMap<String, Vec<u8>>,
}

impl Pairs for Kvp {
    /// get a value associated with the key
    fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.pairs.get(&key.to_string()).cloned()
    }

    /// add a key-value pair to the storage, return previous value if overwritten
    fn put(&mut self, key: &str, value: &dyn AsRef<[u8]>) -> Option<Vec<u8>> {
        self.pairs.insert(key.to_string(), value.as_ref().to_vec())
    }
}

#[derive(Default)]
struct Stk {
    pub stack: Vec<vm::Value>
}

impl Stack for Stk {
    /// push a value onto the stack
    fn push(&mut self, value: vm::Value) {
        self.stack.push(value);
    }

    /// remove the last top value from the stack
    fn pop(&mut self) -> Option<vm::Value> {
        self.stack.pop()
    }

    /// get a reference to the top value on the stack 
    fn top(&self) -> Option<&vm::Value> {
        self.stack.last()
    }

    /// peek at the item at the given index
    fn peek(&self, idx: usize) -> Option<&vm::Value> {
        if idx >= self.stack.len() {
            return None;
        }
        Some(&self.stack[self.stack.len() - 1 - idx])
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
fn test_preimage_wast() {
    // create the stack to use
    let mut pstack = Stk::default();
    let mut rstack = Stk::default();

    { // unlock
        // set up the key-value pair store with the preimage data
        let mut kvp_unlock = Kvp::default();
        let _ = kvp_unlock.put("/entry/proof", &b"for great justice, move every zig!".to_vec());

        // load the unlock script
        let script = load_wast("preimage_unlock.wast");

        // run the unlock script to set up the stack
        let mut instance = test_example(script, "for_great_justice", true, &kvp_unlock, &mut pstack, &mut rstack);

        // check that the stack is what we expect
        let mut ctx = instance.store.as_context_mut();
        let context = ctx.data_mut();
        assert_eq!(1, context.pstack.len());
        assert_eq!(context.pstack.top(), Some(&vm::Value::Bin(b"for great justice, move every zig!".to_vec())));
    }

    { // lock
        // set up the key-value pair store with the sha3 256 hash of the preimage (as serialized
        // Multihash)
        let mut kvp_lock = Kvp::default();
        let _ = kvp_lock.put("/hash", &hex::decode("16206b761d3b2e7675e088e337a82207b55711d3957efdb877a3d261b0ca2c38e201").unwrap());

        // load the lock script
        let script = load_wast("preimage_lock.wast");

        // run the lock script to check the proof
        let mut instance = test_example(script, "move_every_zig", true, &kvp_lock, &mut pstack, &mut rstack);

        // check that the stack is what we expect
        let mut ctx = instance.store.as_context_mut();
        let context = ctx.data_mut();
        assert_eq!(1, context.rstack.len());
        assert_eq!(context.rstack.top(), Some(&vm::Value::Success(0)));
    }
}

#[test]
fn test_preimage_wasm() {
    // create the stack to use
    let mut pstack = Stk::default();
    let mut rstack = Stk::default();

    { // unlock
       // set up the key-value pair store with the preimage data
        let mut kvp_unlock = Kvp::default();
        let _ = kvp_unlock.put("/entry/proof", &b"for great justice, move every zig!".to_vec());

        // load the unlock script
        let script = load_wasm("preimage_unlock.wasm");

        // run the unlock script to set up the stack
        let mut instance = test_example(script, "for_great_justice", true, &kvp_unlock, &mut pstack, &mut rstack);

        // check that the stack is what we expect
        let mut ctx = instance.store.as_context_mut();
        let context = ctx.data_mut();
        assert_eq!(1, context.pstack.len());
        assert_eq!(context.pstack.top(), Some(&vm::Value::Bin(b"for great justice, move every zig!".to_vec())));
    }

    { // lock
        // set up the key-value pair store with the sha3 256 hash of the preimage (as serialized
        // Multihash)
        let mut kvp_lock = Kvp::default();
        let _ = kvp_lock.put("/hash", &hex::decode("16206b761d3b2e7675e088e337a82207b55711d3957efdb877a3d261b0ca2c38e201").unwrap());

        // load the lock script
        let script = load_wasm("preimage_lock.wasm");

        // run the lock script to check the proof
        let mut instance = test_example(script, "move_every_zig", true, &kvp_lock, &mut pstack, &mut rstack);

        // check that the stack is what we expect
        let mut ctx = instance.store.as_context_mut();
        let context = ctx.data_mut();
        assert_eq!(1, context.rstack.len());
        assert_eq!(context.rstack.top(), Some(&vm::Value::Success(0)));
    }
}
