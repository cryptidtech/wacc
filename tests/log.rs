use std::{collections::HashMap, convert::AsRef, fs::read, path::PathBuf};
use wacc::{storage::Pairs, vm};
use wasmtime::StoreLimitsBuilder;

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
    let result = instance.run("move_every_zig").unwrap();

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
fn test_log_wast() {
    let kvp = Kvp::default();
    let mut stack = Vec::default();
    let script = load_wast("log.wast");
    let instance = test_example(script, true, &kvp, &mut stack);
    assert_eq!(b"Hello World!\n".to_vec(), instance.log());
}

#[test]
fn test_invalid_utf8_wast() {
    let kvp = Kvp::default();
    let mut stack = Vec::default();
    let script = load_wast("invalid_utf8.wast");
    test_example(script, false, &kvp, &mut stack);
}

#[test]
fn test_log_wasm() {
    let kvp = Kvp::default();
    let mut stack = Vec::default();
    let script = load_wasm("log.wasm");
    let instance = test_example(script, true, &kvp, &mut stack);
    assert_eq!(b"Hello World!\n".to_vec(), instance.log());
}
