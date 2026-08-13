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
    read(&pb).unwrap_or_else(|_| panic!("WAST file {file_name} must exist"))
}

fn test_example(
    script: Vec<u8>,
    func: &str,
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
    let result = instance.run(func).unwrap();

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
fn test_pubkeysig_wast() {
    // the key-value pair store with the message and signature data
    let mut kvp_unlock = Kvp::default();
    // the key-value pair store with the encoded Multikey
    let mut kvp_lock = Kvp::default();

    // Values to transfer from unlock to lock phase
    let mut pstack_values: Vec<Value>;

    {
        // unlock
        // create the stack to use for unlock
        let pstack = Stk::default();
        let rstack = Stk::default();

        // set up the key-value pair store with the message and signature data
        // NOTE: this is an example of a signed string message
        let _ = kvp_unlock.put(
            "/entry/",
            &"for great justice, move every zig!".to_string().into(),
        );
        let _ = kvp_unlock.put("/entry/proof", &hex::decode("b92483a6c0060001004076fee92ca796162b5e37a84b4150da685d636491b43c1e2a1fab392a7337553502588a609075b56c46b5c033b260d8d314b584e396fc2221c55f54843679ee08").unwrap().into());

        // load the unlock script
        let script = load_wast("pubkeysig_unlock.wast");

        // run the unlock script to set up the stack
        let mut instance = test_example(
            script,
            "for_great_justice",
            true,
            kvp_unlock.clone(),
            kvp_unlock.clone(),
            pstack,
            rstack,
        );

        // check that the stack is what we expect and save values
        let mut ctx = instance.store.as_context_mut();
        let context = ctx.data_mut();
        assert_eq!(1, context.pstack.len());
        assert_eq!(context.pstack.top(), Some(Value::Bin { hint: String::new(), data: Arc::from(hex::decode("b92483a6c0060001004076fee92ca796162b5e37a84b4150da685d636491b43c1e2a1fab392a7337553502588a609075b56c46b5c033b260d8d314b584e396fc2221c55f54843679ee08").unwrap().into_boxed_slice()) }));
        // Save the pstack values for the lock phase by extracting them
        pstack_values = vec![];
        for i in 0..context.pstack.len() {
            if let Some(val) = context.pstack.peek(context.pstack.len() - 1 - i) {
                pstack_values.push(val);
            }
        }
    }

    {
        // lock
        // create the stack to use for lock and populate it with values from unlock phase
        let pstack = Stk {
            stack: pstack_values,
        };
        let rstack = Stk::default();

        // set up the key-value pair store with the encoded Multikey
        let _ = kvp_lock.put("/keys/primary", &hex::decode("ba24ed010874657374206b657901012084d515ef051e07d597f3c14ac09e5a9d5012c659c196d96db5c6b98ea552f603").unwrap().into());

        // load the lock script
        let script = load_wast("pubkeysig_lock.wast");

        // run the lock script to check the proof
        let mut instance = test_example(
            script,
            "move_every_zig",
            true,
            kvp_lock,
            kvp_unlock,
            pstack,
            rstack,
        );

        // check that the stack is what we expect
        let mut ctx = instance.store.as_context_mut();
        let context = ctx.data_mut();
        assert_eq!(1, context.rstack.len());
        assert_eq!(context.rstack.top(), Some(Value::Success(0)));
    }
}

#[test]
fn test_pubkeysig_wasm() {
    // Skip test if WASM file not built (requires wat2wasm tool)
    let Some(unlock_script) = load_wasm("pubkeysig_unlock.wasm") else {
        eprintln!("Skipping test_pubkeysig_wasm: pubkeysig_unlock.wasm not found (run 'make' in examples/wast to build)");
        return;
    };
    let Some(lock_script) = load_wasm("pubkeysig_lock.wasm") else {
        eprintln!("Skipping test_pubkeysig_wasm: pubkeysig_lock.wasm not found (run 'make' in examples/wast to build)");
        return;
    };

    // the key-value pair store with the message and signature data
    let mut kvp_unlock = Kvp::default();
    // the key-value pair store with the encoded Multikey
    let mut kvp_lock = Kvp::default();

    // Values to transfer from unlock to lock phase
    let mut pstack_values: Vec<Value>;

    {
        // unlock
        // create the stack to use for unlock
        let pstack = Stk::default();
        let rstack = Stk::default();

        // set up the key-value pair store with the message and signature data
        // NOTE: this is an example of a signed binary message
        let _ = kvp_unlock.put(
            "/entry/",
            &"for great justice, move every zig!".as_bytes().into(),
        );
        let _ = kvp_unlock.put("/entry/proof", &hex::decode("b92483a6c0060001004076fee92ca796162b5e37a84b4150da685d636491b43c1e2a1fab392a7337553502588a609075b56c46b5c033b260d8d314b584e396fc2221c55f54843679ee08").unwrap().into());

        // run the unlock script to set up the stack
        let mut instance = test_example(
            unlock_script,
            "for_great_justice",
            true,
            kvp_unlock.clone(),
            kvp_unlock.clone(),
            pstack,
            rstack,
        );

        // check that the stack is what we expect and save values
        let mut ctx = instance.store.as_context_mut();
        let context = ctx.data_mut();
        assert_eq!(1, context.pstack.len());
        assert_eq!(context.pstack.top(), Some(Value::Bin { hint: String::new(), data: Arc::from(hex::decode("b92483a6c0060001004076fee92ca796162b5e37a84b4150da685d636491b43c1e2a1fab392a7337553502588a609075b56c46b5c033b260d8d314b584e396fc2221c55f54843679ee08").unwrap().into_boxed_slice()) }));
        // Save the pstack values for the lock phase by extracting them
        pstack_values = vec![];
        for i in 0..context.pstack.len() {
            if let Some(val) = context.pstack.peek(context.pstack.len() - 1 - i) {
                pstack_values.push(val);
            }
        }
    }

    {
        // lock
        // create the stack to use for lock and populate it with values from unlock phase
        let pstack = Stk {
            stack: pstack_values,
        };
        let rstack = Stk::default();

        // set up the key-value pair store with the encoded Multikey
        let _ = kvp_lock.put("/keys/primary", &hex::decode("ba24ed010874657374206b657901012084d515ef051e07d597f3c14ac09e5a9d5012c659c196d96db5c6b98ea552f603").unwrap().into());

        // run the lock script to check the proof
        let mut instance = test_example(
            lock_script,
            "move_every_zig",
            true,
            kvp_lock,
            kvp_unlock,
            pstack,
            rstack,
        );

        // check that the stack is what we expect
        let mut ctx = instance.store.as_context_mut();
        let context = ctx.data_mut();
        assert_eq!(1, context.rstack.len());
        assert_eq!(context.rstack.top(), Some(Value::Success(0)));
    }
}
