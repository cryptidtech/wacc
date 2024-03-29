use std::{collections::HashMap, convert::AsRef, fs::read, path::PathBuf};
use wacc::{storage::Pairs, vm};
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
    stack: &'a mut Vec<vm::Value>,
) -> vm::Instance<'a> {
    // build the context
    let context = vm::Context {
        pairs,
        stack,
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
    let result = instance.run("for_great_justice").unwrap();

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

#[test]
fn test_unlock_wast() {
    // set up the key-value pair store
    let mut kvp = Kvp::default();
    let _ = kvp.put("/entry/", &b"foo".to_vec());
    let _ = kvp.put("/entry/proof", &b"bar".to_vec());

    // load the script
    let mut stack = Vec::default();
    let script = load_wast("unlock.wast");
    let mut instance = test_example(script, true, &kvp, &mut stack);

    // Get the context
    let mut ctx = instance.store.as_context_mut();
    let context = ctx.data_mut();
    assert_eq!(2, context.stack.len());
    assert_eq!(context.stack[0], vm::Value::Bin(b"foo".to_vec()));
    assert_eq!(context.stack[1], vm::Value::Bin(b"bar".to_vec()));
}

#[test]
fn test_unlock_wasm() {
    // set up the key-value pair store
    let mut kvp = Kvp::default();
    let _ = kvp.put("/entry/", &b"foo".to_vec());
    let _ = kvp.put("/entry/proof", &b"bar".to_vec());

    // load the script
    let mut stack = Vec::default();
    let script = load_wasm("unlock.wasm");
    let mut instance = test_example(script, true, &kvp, &mut stack);

    // Get the context
    let mut ctx = instance.store.as_context_mut();
    let context = ctx.data_mut();
    assert_eq!(2, context.stack.len());
    assert_eq!(context.stack[0], vm::Value::Bin(b"foo".to_vec()));
    assert_eq!(context.stack[1], vm::Value::Bin(b"bar".to_vec()));
}
