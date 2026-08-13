[![](https://img.shields.io/badge/made%20by-Cryptid%20Technologies-gold.svg?style=flat-square)](https://cryptid.tech/)
[![](https://img.shields.io/badge/project-provenance-purple.svg?style=flat-square)](https://github.com/cryptidtech/provenance-specifications/)
[![](https://img.shields.io/badge/project-multiformats-blue.svg?style=flat-square)](https://github.com/multiformats/multiformats/)

[![Build Status](https://github.com/cryptidtech/wacc/actions/workflows/rust.yml/badge.svg)](https://github.com/cryptidtech/wacc/actions)
[![License](https://img.shields.io/crates/l/wacc?style=flat-square)](LICENSE)
[![Crates.io](https://img.shields.io/crates/v/wacc?style=flat-square)](https://crates.io/crates/wacc)
[![Documentation](https://docs.rs/wacc/badge.svg?style=flat-square)](https://docs.rs/wacc)

# wacc

Web Assembly Cryptographic Constructs (WACC) VM implementation. A WASM-based virtual machine for executing cryptographic verification scripts used in provenance systems.

The wacc VM executes lock and unlock scripts that verify signatures, check preimages, compare key-path values, and branch on conditions. It uses Wasmtime as the WASM runtime with fuel-based execution limiting, memory limits, and check-count limits to prevent DoS attacks.

## Table of Contents

- [Features](#features)
- [Install](#install)
- [Usage](#usage)
- [Security](#security)
- [Testing](#testing)
- [Maintainers](#maintainers)
- [Contribute](#contribute)
- [License](#license)

## Features

- WASM-based script execution via Wasmtime.
- Stack-based API: `push`, `pop`, `peek`, `check_eq`, `check_signature`, `check_preimage`, `branch`, `log`.
- Configurable security limits: fuel, memory, check count, and algorithm allowlists.
- Module cache with Blake3-based WASM module hashing.
- XMSS leaf-index monotonicity enforcement to prevent index reuse.
- Thread-safe `Module` and `Engine` sharing via `Arc`.
- Adapters layer for multicodec cryptographic operations.

## Install

Add this to your `Cargo.toml`:

```toml
[dependencies]
wacc = "0.1"
```

MSRV: Rust 1.85.

## Usage

```rust,no_run
use wacc::{Builder, Context, types::{CheckCount, ContextPath}, storage::{Pairs, Stack}, Value};
use std::collections::BTreeMap;

// Build a VM instance with security limits
let builder = Builder::new()
    .with_bytes(&wasm_bytes)
    .with_fuel(100_000)
    .with_memory_limit(1024 * 1024);

let mut instance = builder.try_build().unwrap();

// Set up the execution context
let context = Context {
    current: &current_state,
    proposed: &proposed_state,
    pstack: &mut program_stack,
    rstack: &mut return_stack,
    check_count: 0,
    write_idx: 0,
    context: "/entry/".to_string(),
};

// Execute the script
let result = instance.execute(&context).unwrap();
```

## Security

The wacc VM applies these security limits:

- **Fuel**: Wasmtime's fuel-based execution limiting prevents infinite loops.
- **Memory**: Configurable memory limit prevents excessive memory use.
- **Check count**: Limits the number of `check_*` operations per execution.
- **Algorithm allowlist**: Restricts which hash and signature algorithms are accepted.
- **XMSS enforcement**: Prevents XMSS leaf-index reuse across a verification pass.

## Testing

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo doc --no-deps
```

The test suite includes lock/unlock script tests, fork lock tests, preimage tests, public key signature tests, branch tests, security tests, concurrency tests, edge case tests, and property-based tests.

## Maintainers

- Dave Grantham <dwg@linuxprogrammer.org>

## Contribute

Pull requests go to the [`cryptidtech/wacc`](https://github.com/cryptidtech/wacc)
repository. Sign commits with GPG. Use Conventional Commits messages.

## License

Licensed under `Apache-2.0`.

See [`LICENSE`](LICENSE) for the full text.