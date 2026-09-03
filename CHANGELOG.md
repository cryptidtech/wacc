# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[2.1.0]: https://github.com/cryptidtech/wacc/compare/v2.0.0...v2.1.0
[2.0.0]: https://github.com/cryptidtech/wacc/releases/tag/v2.0.0
[1.0.5]: https://github.com/cryptidtech/wacc/releases/tag/v1.0.5