# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.3.0] - 2026-10-08

### Changed

- Migrated the crate to multi-hash 2.0. The three preimage-hash sites — the `check_preimage` binary and string arms in `vm/context.rs` and `MulticodecHashVerifier::verify_preimage` in `adapters/multicodec_crypto.rs` — moved from the removed `Builder::new_from_bytes` one-shot to the streaming shape (`Builder::new` plus `update` and `try_build`). Each site pins `.output_len(32)`, the digest length multi-hash 1.1 produced, so stored preimage hashes keep their bytes; blake3 is an extendable-output codec and requires the length, and fixed-output codecs ignore it. The `ALLOWED_HASH_CODECS` whitelist and its fail-closed behavior are unchanged.
- Updated dependencies: `multi-hash` 1.1 → 2.0, `multi-key` 2.1 → 2.2, `multi-cid` 0.2 → 0.3. The graph resolves a single multi-hash major: 2.0.1, which declares `rust-version = "1.99"`. The `multi-key` raise is required for type coherence: `check_preimage_value` compares `fingerprint()` results with this crate's multi-hash-2-typed `Multihash`. `Cid` is opaque in this crate, so the `multi-cid` raise is a coherence choice.
- Added two preimage regression tests: one asserts the recomputed blake3-256 multihash matches the bytes recorded under multi-hash 1.1; one asserts a truncated stored digest still fails closed. No public API changes.

## [2.2.1] - 2026-10-07

### Changed

- Migrated the `check_signature`, `check_preimage_value`, and `enforce_stateful_key_rules` host functions and `MulticodecSignatureVerifier` from `multi_key::Views` and `multi_sig::Views` to `ViewBuilder`. No public API changes.
- Updated dependencies: `multi-codec` 1.3 → 1.5, `multi-key` 1.2 → 2.1, `multi-sig` 1.3 → 1.5, `wasmtime` 48.0 → 49.0, `wasmparser` 0.254 → 0.261, `blake3` 1.8.1 → 1.8.

## [2.2.0] - 2026-10-06

### Added

- WASM Component Model support for scripts. An added `wit/wacc.wit` declares the WIT package `cryptid:wacc@1.0.0` with the typed `host` interface and the two script worlds `unlock-script` (export `for-great-justice`) and `lock-script` (export `move-every-zig`); the eight typed host functions carry the same semantics as the eight raw-ABI core-module imports, and the component path also accepts the legacy snake-case export names through the added export-name map.
- `ScriptKind` with the variants `Module` and `Component`, and `ScriptKind::detect(bytes)`: binary detection by the version byte after the `\0asm` magic and text detection by the leading WAT token. Re-exported from the crate root.
- A blake3-keyed component cache in the public `ModuleCache` (`insert_component_bytes` / `get_component_bytes`, a second bounded map) and the `Runtime::cached_component_count()` accessor; component compilation runs through the internal `Runtime::compile_component`, mirroring the internal `Runtime::compile` on the module path.
- `Builder::with_component_bytes` and `Builder::try_build_component`, producing a `vm::ComponentInstance` with a public `store`, `run`, and `log`, mirroring the module path's fuel and store-limit flow. Typed host imports are generated with `wasmtime::component::bindgen!` and registered once on the runtime's component linker for both worlds, so script components import only `cryptid:wacc/host@1.0.0`.

### Changed

- The `wasmtime` dependency gains the `component-model` feature; the dependency stays at version `48.0`.
- `Builder::try_build` now rejects detectable component bytes with `VmError::ScriptKindMismatch` instead of a generic compilation error, and `Builder::try_build_component` rejects detectable module bytes symmetrically.

### Notes

- 2.1.2 shipped without a changelog entry.

## [2.1.1] - 2026-09-02

### Changed

- Upgraded `wasmtime` from 47.0 to 48.0. No API changes.

## [2.1.0] - 2026-09-01

### Added

- Merkle-tree Lamport leaf reuse enforcement. The 11 `LamportMerkle*Sig` codecs are now stateful in the VM: `check_signature` reads the consumed leaf index from the `MtSignature` wire data (byte 1, after the depth byte), validates it against the declared tree depth, and enforces strictly-increasing leaf consumption per public key across a verification pass (`enforce_lamport_merkle` in `xmss_guard.rs`, committed per entry by the existing `XmssEnforcement` guard). Reused or rolled-back leaf indices fail verification.
- Unit tests for the merkle guard: monotonic acceptance, duplicate rejection, rollback rejection, per-entry retry tolerance, and no-guard no-op behavior.

### Changed

- Updated dependencies: `multi-codec` 1.2 → 1.3, `multi-key` 1.1 → 1.2, `multi-sig` 1.2 → 1.3.
- Raised `rust-version` from 1.95 to 1.96 (required by `multi-key` 1.2 / `lamport_signature_plus` 0.5.0) and updated the CI MSRV job to 1.96.
- Refactored `check_signature`'s stateful-key checks (XMSS, one-time Lamport, merkle Lamport) into a shared `enforce_stateful_key_rules` helper.

## [2.0.0] - 2026-08-13

### Summary

Synced from the BetterSign workspace `bs-wacc 0.7.0` crate. This is a
breaking release: the license changed from FSL-1.1 to Apache-2.0, the
`wasmtime` dependency jumped from 19.0 to 41.0, the `thiserror` dependency
jumped from 1.0 to 2.0, and the entire source tree was replaced with the
richer workspace version (adapters, domain, ports, security, types,
xmss_guard, module cache, runtime). The public API is substantially
different from 1.0.5.

### Added

- Adapters layer for multicodec cryptographic operations (hash, signature, key).
- Domain layer for plog operations.
- Ports layer for cryptographic operations (hash, signature, key).
- Security limits and algorithm allowlists.
- Module cache with Blake3-based WASM module hashing.
- XMSS leaf-index monotonicity enforcement (`XmssEnforcement`) to prevent index reuse across a verification pass.
- VM runtime module for managing Wasmtime `Engine` and `Module` lifecycle.
- Type-safe wrappers: `CheckCount`, `ContextPath`, `FuelAmount`, `WasmPtr`, `WasmSize`, `CurrentKey`, `ProposedKey`.
- Thread-safe `Module` and `Engine` sharing via `Arc`.
- Comprehensive test suite: lock, unlock, forklock, preimage, pubkeysig, branch, security, concurrency, edge cases, property tests.
- MSRV declared as 1.85.

### Changed

- **License**: changed from `Functional Source License 1.1` to `Apache-2.0`. This is a breaking change.
- **`wasmtime`**: upgraded from `19.0` to `41.0`. This is a breaking change for all callers of the wacc VM API.
- **`thiserror`**: upgraded from `1.0` to `2.0`. This is a breaking change for error type consumers.
- **Source tree**: replaced the entire `src/` directory with the BetterSign workspace version (`bs-wacc 0.7.0`). The new source includes `adapters/`, `domain/`, `ports/`, `security.rs`, `types/`, `macros.rs`, `vm/cache.rs`, `vm/runtime.rs`, and `vm/xmss_guard.rs` modules that the previous standalone version did not have.
- **Crate name references**: all `use bs_wacc::...` references in source, tests, and doc comments now use `use wacc::...`.
- **Dependencies**: repointed from git deps (`multicid`, `multihash`, `multikey`, `multisig`, `multitrait`, `multiutil`) to path deps with `package` rename (`multi-cid`, `multi-codec`, `multi-hash`, `multi-key`, `multi-sig`, `multi-trait`, `multi-util`). The `multi-codec`, `multi-hash`, `multi-key`, `multi-sig`, `multi-trait`, and `multi-util` deps point at the `bs-*` workspace path deps in `bettersign/crates/` until Phases 7-9 of the crate extraction plan publish the Lamport and XMSS support to crates.io.
- **`blake3`**: added as a direct dependency (used by the module cache for WASM module hashing).
- **Test example file paths**: updated from `../../examples/wacc/wast` (workspace-relative) to `examples/wacc/wast` (crate-root-relative).
- **CI**: updated to run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo doc`, and an MSRV check job.

### Removed

- Old FSL license files (`LICENSE-APACHE.txt`, `LICENSE.md`, `pandoc.css`).
- Old standalone `src/` tree (replaced by the workspace version).
- Old example projects (`examples/log`, `examples/signature_first`, `examples/signature_lock`, `examples/unlock` as standalone Cargo projects — now part of `examples/wacc/`).

### Notes

- The `multi-codec`, `multi-hash`, `multi-key`, `multi-sig`, `multi-trait`, and `multi-util` dependencies currently point at the `bs-*` workspace path deps in `bettersign/crates/` via `package` rename. The published crates.io versions of `multi-codec` and `multi-sig` lack the Lamport and XMSS codec variants and the `sig_index()` method that the wacc VM security module uses. When Phases 7-9 of the crate extraction plan publish these features, the path deps will switch to the crates.io versions.

## [1.0.5] - 2026-08-11

### Notes

- Previous standalone release. Used `wasmtime 19.0`, `thiserror 1.0`, and FSL-1.1 license. Git deps for `multicid`, `multihash`, `multikey`, `multisig`, `multitrait`, `multiutil`.

[2.3.0]: https://github.com/cryptidtech/wacc/compare/v2.2.1...v2.3.0
[2.2.1]: https://github.com/cryptidtech/wacc/compare/v2.2.0...v2.2.1
[2.2.0]: https://github.com/cryptidtech/wacc/compare/v2.1.2...v2.2.0
[2.1.1]: https://github.com/cryptidtech/wacc/compare/v2.1.0...v2.1.1
[2.1.0]: https://github.com/cryptidtech/wacc/compare/v2.0.0...v2.1.0
[2.0.0]: https://github.com/cryptidtech/wacc/releases/tag/v2.0.0
[1.0.5]: https://github.com/cryptidtech/wacc/releases/tag/v1.0.5