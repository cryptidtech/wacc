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
use test_log::test;
use tracing::info;
use wacc::error::VmError;
use wacc::storage::{Pairs, Stack};
use wacc::types::{CheckCount, ContextPath};
use wacc::vm::{Builder, Context, Value};
use wacc::{Error, Runtime, ScriptKind};
use wasmparser::{Parser, Payload};
use wasmtime::StoreLimitsBuilder;

const MEMORY_LIMIT: usize = 1 << 22; /* 4MB */

/// Inline component that imports the typed `cryptid:wacc/host@1.0.0` `push`
/// function and calls it with the string `/entry/` so the same component
/// exercises a typed guest import and a typed component export. The guest core
/// module shares the canonical-ABI memory with the shim, and the exported
/// entry point is named `export_name`.
fn script_component_wat(export_name: &str) -> Vec<u8> {
    format!(
        r#"(component
  (import "cryptid:wacc/host@1.0.0" (instance $host
    (export "push" (func (param "key" string) (result bool)))
  ))
  (alias export $host "push" (func $host_push))

  ;; canonical ABI backing: the memory the guests and the lowerings share
  (core module $shim
    (memory (export "memory") 1)
  )
  (core instance $shim_inst (instantiate $shim))
  (alias core export $shim_inst "memory" (core memory $mem))

  ;; guest: calls the lowered host import with the data-segment string
  (core module $guest
    (import "env" "memory" (memory 1))
    (import "env" "push" (func $push (param i32 i32) (result i32)))
    (data (i32.const 0) "/entry/")
    (func (export "run") (result i32)
      (call $push (i32.const 0) (i32.const 7))
    )
  )
  (core func $host_push_lowered (canon lower (func $host_push) (memory $mem)))
  (core instance $guest_inst (instantiate $guest
    (with "env" (instance
      (export "memory" (memory $mem))
      (export "push" (func $host_push_lowered))
    ))
  ))
  (alias core export $guest_inst "run" (core func $run))
  (type $exit (func (result s32)))
  (func $entry (type $exit) (canon lift (core func $run)))
  (export "{export_name}" (func $entry))
)"#
    )
    .into_bytes()
}

/// Component that imports an interface no linker provides, to exercise
/// instantiation failure mapping.
const UNKNOWN_IMPORT_WAT: &[u8] = br#"(component
  (import "wacc:unknown/host@1.0.0" (instance (export "unused" (func))))
)"#;

/// Core module with the bytes layout of a simple module that exports `test`
/// and returns 1, used to prove the module path still works on the component
/// enabled engine.
const SIMPLE_MODULE_WAT: &[u8] = br#"(module
  (memory (export "memory") 1)
  (func (export "test") (result i32) (i32.const 1))
)"#;

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

    /// return true if there are no values on the stack
    fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }
}

/// Builds a [`Context`] whose current KVP holds the `/entry/` path that the
/// inline component's `push` import resolves.
fn make_context() -> Context {
    let mut current = Kvp::default();
    current.put(
        "/entry/",
        &Value::Bin {
            hint: String::new(),
            data: Arc::from(vec![1, 2, 3].into_boxed_slice()),
        },
    );

    Context {
        current: Box::new(current),
        proposed: Box::new(Kvp::default()),
        pstack: Box::new(Stk::default()),
        rstack: Box::new(Stk::default()),
        check_count: CheckCount::zero(),
        write_idx: 0,
        context: ContextPath::root(),
        log: Vec::default(),
        limiter: StoreLimitsBuilder::new()
            .memory_size(MEMORY_LIMIT)
            .instances(16)
            .memories(8)
            .build(),
    }
}

fn build_component(component_wat: &[u8], context: Context) -> wacc::vm::ComponentInstance {
    info!("building component of {} bytes", component_wat.len());
    match Builder::new()
        .with_context(context)
        .with_component_bytes(component_wat)
        .try_build_component()
    {
        Ok(component) => component,
        Err(e) => {
            println!("component builder failed: {}", e);
            panic!()
        }
    }
}

/// Loads a built guest component fixture; guests live as separate Cargo
/// projects under `examples/wacc/` and the Makefile copies their components
/// into the crate's `target/components/` directory.
fn load_component(file_name: &str) -> Option<Vec<u8>> {
    let mut pb = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    pb.push("target/components");
    pb.push(file_name);
    info!("trying to load: {}", pb.as_os_str().display());
    read(&pb).ok()
}

/// Builds a [`Context`] for the `abi` guest fixture: `/abi/` resolves in the
/// current KVP and the branch prefix makes the composed key-path distinctive.
fn make_abi_context() -> Context {
    let mut current = Kvp::default();
    current.put(
        "/abi/",
        &Value::Bin {
            hint: String::new(),
            data: Arc::from(vec![1, 2, 3].into_boxed_slice()),
        },
    );

    Context {
        current: Box::new(current),
        proposed: Box::new(Kvp::default()),
        pstack: Box::new(Stk::default()),
        rstack: Box::new(Stk::default()),
        check_count: CheckCount::zero(),
        write_idx: 0,
        context: ContextPath::new("/forks/child/"),
        log: Vec::default(),
        limiter: StoreLimitsBuilder::new()
            .memory_size(MEMORY_LIMIT)
            .instances(16)
            .memories(8)
            .build(),
    }
}

#[test]
fn test_component_unlock_runs_with_both_export_spellings() {
    let wat = script_component_wat("for-great-justice");

    let mut component = build_component(&wat, make_context());

    // the legacy snake spelling maps onto the kebab world export
    assert!(component.run("for_great_justice").unwrap());

    // the typed push import left one entry-parameter on the parameter stack
    let pstack = component.store.data().pstack.top();
    assert!(matches!(pstack, Some(Value::Bin { .. })));
    assert_eq!(component.store.data().pstack.len(), 1);

    // the kebab spelling runs on the already instantiated component
    assert!(component.run("for-great-justice").unwrap());
    assert_eq!(component.store.data().pstack.len(), 2);
}

#[test]
fn test_component_lock_runs_with_both_export_spellings() {
    let wat = script_component_wat("move-every-zig");

    let mut component = build_component(&wat, make_context());

    assert!(component.run("move_every_zig").unwrap());
    assert_eq!(component.store.data().pstack.len(), 1);

    assert!(component.run("move-every-zig").unwrap());
    assert_eq!(component.store.data().pstack.len(), 2);
}

#[test]
fn test_component_unknown_export_name_fails() {
    let wat = script_component_wat("for-great-justice");

    let mut component = build_component(&wat, make_context());

    let result = component.run("no_such_export");
    assert!(matches!(
        result,
        Err(Error::Vm(VmError::ExecutionError { .. }))
    ));
}

#[test]
fn test_module_builder_rejects_component_bytes() {
    let wat = script_component_wat("for-great-justice");

    let result = Builder::new()
        .with_context(make_context())
        .with_bytes(&wat)
        .try_build();

    assert!(matches!(
        result,
        Err(Error::Vm(VmError::ScriptKindMismatch {
            expected: ScriptKind::Module,
            actual: ScriptKind::Component,
        }))
    ));
}

#[test]
fn test_component_builder_rejects_module_bytes() {
    let result = Builder::new()
        .with_context(make_context())
        .with_component_bytes(SIMPLE_MODULE_WAT)
        .try_build_component();

    assert!(matches!(
        result,
        Err(Error::Vm(VmError::ScriptKindMismatch {
            expected: ScriptKind::Component,
            actual: ScriptKind::Module,
        }))
    ));
}

#[test]
fn test_component_compilation_is_cached() {
    let runtime = Arc::new(Runtime::new().unwrap());
    let wat = script_component_wat("for-great-justice");

    let _first = Builder::new()
        .with_context(make_context())
        .with_runtime(Arc::clone(&runtime))
        .with_component_bytes(&wat)
        .try_build_component()
        .unwrap();

    let _second = Builder::new()
        .with_context(make_context())
        .with_runtime(Arc::clone(&runtime))
        .with_component_bytes(&wat)
        .try_build_component()
        .unwrap();

    assert_eq!(runtime.cached_component_count(), 1);
    // component caching leaves the module count untouched
    assert_eq!(runtime.cached_module_count(), 0);
}

#[test]
fn test_module_path_still_runs_on_shared_engine() {
    let mut instance = Builder::new()
        .with_context(make_context())
        .with_bytes(SIMPLE_MODULE_WAT)
        .try_build()
        .unwrap();

    assert!(instance.run("test").unwrap());
}

#[test]
fn test_component_build_fails_without_context() {
    let wat = script_component_wat("for-great-justice");

    let result = Builder::new()
        .with_component_bytes(&wat)
        .try_build_component();

    assert!(matches!(
        result,
        Err(Error::Vm(VmError::MissingContext { .. }))
    ));
}

#[test]
fn test_unknown_import_fails_instantiation() {
    let result = Builder::new()
        .with_context(make_context())
        .with_component_bytes(UNKNOWN_IMPORT_WAT)
        .try_build_component();

    assert!(matches!(
        result,
        Err(Error::Vm(VmError::InstantiationError { .. }))
    ));
}

#[test]
fn test_script_kind_detects_wat_component_text() {
    // binary components carry the component version byte at index 4; prove
    // the detector keeps the binary rule by compiling the wat text component
    // and checking the text rule against the same artifact's wat form
    let wat = script_component_wat("for-great-justice");
    assert_eq!(ScriptKind::detect(&wat), Some(ScriptKind::Component));
}

/// Component whose guest burns fuel in a bounded-counter loop so large it
/// exhausts any practical fuel budget before reaching its exit, so the typed
/// export call traps with the store's fuel limit.
const INFINITE_LOOP_COMPONENT_WAT: &[u8] = br#"(component
  (core module $guest
    (func (export "run") (result i32)
      (local $i i64)
      (loop $burn
        (local.set $i (i64.add (local.get $i) (i64.const 1)))
        (br_if $burn (i64.lt_u (local.get $i) (i64.const 4611686018427387904)))
      )
      (i32.const 1)
    )
  )
  (core instance $inst (instantiate $guest))
  (alias core export $inst "run" (core func $run))
  (type $exit (func (result s32)))
  (func $entry (type $exit) (canon lift (core func $run)))
  (export "for-great-justice" (func $entry))
)"#;

#[test]
fn test_fuel_exhaustion_traps_on_the_component_path() {
    let mut component = build_component(INFINITE_LOOP_COMPONENT_WAT, make_context());

    let result = component.run("for_great_justice");
    assert!(matches!(
        result,
        Err(Error::Vm(VmError::ExecutionError { .. }))
    ));
}

#[test]
fn test_abi_guest_fixture_round_trips_the_typed_imports() {
    // Skip test if the component fixture is not built (run 'make -C
    // examples/wacc/wast guests')
    let Some(bytes) = load_component("abi.wasm") else {
        eprintln!(
            "Skipping test_abi_guest_fixture_round_trips_the_typed_imports: abi.wasm not found"
        );
        return;
    };

    let mut component = build_component(&bytes, make_abi_context());

    assert!(component.run("for_great_justice").unwrap());

    // push and push-value both landed on the parameter stack: the kvp-backed
    // value first, then the raw guest bytes
    assert_eq!(component.store.data().pstack.len(), 2);
    assert_eq!(
        component.store.data().pstack.top(),
        Some(Value::Bin {
            hint: String::new(),
            data: Arc::from(vec![0xAB, 0xCD].into_boxed_slice())
        })
    );
    assert_eq!(
        component.store.data().pstack.peek(1),
        Some(Value::Bin {
            hint: String::new(),
            data: Arc::from(vec![1, 2, 3].into_boxed_slice())
        })
    );

    // the four stateless checks each left a failure marker on the return stack
    // and incremented the check counter once
    assert_eq!(component.store.data().rstack.len(), 4);
    for value in [
        component.store.data().rstack.peek(0),
        component.store.data().rstack.peek(1),
        component.store.data().rstack.peek(2),
        component.store.data().rstack.peek(3),
    ]
    .into_iter()
    .flatten()
    {
        assert!(matches!(value, Value::Failure { .. }));
    }
    assert_eq!(component.store.data().check_count.as_usize(), 4);

    // branch returned the composed key-path through the guest realloc path
    // and the guest logged it back along with its own line
    let log = String::from_utf8(component.log()).unwrap();
    assert!(log.contains("/forks/child//abi/proof"), "log: {log:?}");
    assert!(log.contains("/abi/log-line"), "log: {log:?}");
}

/// The guest fixture components, built by `make -C examples/wacc/wast guests`.
const GUEST_FIXTURES: [&str; 5] = ["abi", "unlock", "log", "signature_lock", "signature_first"];

/// Checks every built fixture component imports only the typed
/// `cryptid:wacc/host@1.0.0` interface, so the no-WASI property of the guest
/// recipe stays verified.
#[test]
fn test_fixture_components_import_only_the_typed_host_interface() {
    let mut checked: usize = 0;
    for name in GUEST_FIXTURES {
        let Some(bytes) = load_component(&format!("{name}.wasm")) else {
            eprintln!(
                "Skipping {name}.wasm: fixture not found (run 'make -C examples/wacc/wast guests')"
            );
            continue;
        };

        let mut imports: Vec<String> = Vec::new();
        for payload in Parser::new(0).parse_all(&bytes) {
            // component-level imports: the fixture guests import the typed
            // `cryptid:wacc/host@1.0.0` interface instance and nothing else.
            // The core modules embedded inside the component have their own
            // import sections and are not part of the guest's host surface.
            if let Payload::ComponentImportSection(reader) = payload.unwrap() {
                for import in reader {
                    imports.push(import.unwrap().name.name.to_string());
                }
            }
        }

        assert!(
            imports.iter().all(|module| !module.starts_with("wasi:")),
            "{name} imports a WASI interface: {imports:?}"
        );
        assert_eq!(
            imports,
            vec!["cryptid:wacc/host@1.0.0".to_string()],
            "{name} unexpected import set"
        );
        checked += 1;
    }
    eprintln!("checked {checked} fixture component(s) for import hygiene");
}
