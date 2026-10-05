// SPDX-License-Identifier: Apache-2.0

//! WASM Component Model script support.
//!
//! Components run on the same [`Runtime`](crate::Runtime) engine and share the
//! same [`Context`] state as core modules. A script component imports the
//! typed `cryptid:wacc/host@1.0.0` interface and exports one of the two script
//! worlds: `unlock-script` exports `for-great-justice` and `lock-script`
//! exports `move-every-zig`. The typed interface carries the same semantics as
//! the raw-ABI imports used by the module path.

use crate::{error::VmError, vm::Context, Error};
use std::fmt;
use wasmtime::Store;

/// Bindings for components that implement the `lock-script` world.
pub(crate) mod lock_world {
    wasmtime::component::bindgen!({
        world: "lock-script",
    });
}

/// Bindings for components that implement the `unlock-script` world.
pub(crate) mod unlock_world {
    wasmtime::component::bindgen!({
        world: "unlock-script",
    });
}

/// The WASM execution model that a script artifact targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScriptKind {
    /// A core WebAssembly module: binary version byte `0x01` after the `\0asm`
    /// magic, or a `module` token at the start of WAT text.
    Module,
    /// A WASM component: binary version byte `0x0d` after the `\0asm` magic,
    /// or a `component` token at the start of WAT text.
    Component,
}

/// Version byte of a binary core WebAssembly module, following the `\0asm`
/// magic.
const MODULE_VERSION_BYTE: u8 = 0x01;

/// Version byte of a binary WASM component, following the `\0asm` magic.
const COMPONENT_VERSION_BYTE: u8 = 0x0d;

/// Magic bytes that begin every binary WASM artifact.
const WASM_MAGIC: [u8; 4] = [0x00, b'a', b's', b'm'];

impl ScriptKind {
    /// Detects the WASM execution model that an encoded script artifact
    /// targets.
    ///
    /// Binary artifacts are recognized by the `\0asm` magic followed by the
    /// version byte: `0x01` selects [`ScriptKind::Module`] and `0x0d` selects
    /// [`ScriptKind::Component`]; any other version byte yields [`None`].
    ///
    /// Text artifacts are recognized by skipping leading whitespace, `;;` line
    /// comments, and nested `(; ... ;)` block comments, then taking the leading
    /// token: `component` selects [`ScriptKind::Component`] and `module`
    /// selects [`ScriptKind::Module`]. Both the bare `module` spelling and the
    /// parenthesized `(module ...)` form are accepted.
    ///
    /// Undetectable input yields [`None`], which leaves the kind up to the
    /// caller. The documented builders fail loudly only on detectable content.
    #[must_use]
    pub fn detect(bytes: &[u8]) -> Option<Self> {
        if bytes.len() >= 5 && bytes[..4] == WASM_MAGIC {
            return match bytes[4] {
                MODULE_VERSION_BYTE => Some(Self::Module),
                COMPONENT_VERSION_BYTE => Some(Self::Component),
                _ => None,
            };
        }
        Self::detect_text(bytes)
    }

    /// Applies the text-artifact rule to non-binary input.
    fn detect_text(bytes: &[u8]) -> Option<Self> {
        let text = std::str::from_utf8(bytes).ok()?;
        // strip a byte-order mark so BOM-prefixed WAT text stays detectable
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let mut rest = text.trim_start();

        // skip leading whitespace and comments until real content is visible
        loop {
            if rest.is_empty() {
                return None;
            }
            if rest.starts_with(";;") {
                // a line comment runs through its newline
                let idx = rest.find('\n')?;
                rest = rest[idx + 1..].trim_start();
                continue;
            }
            if rest.starts_with("(;") {
                rest = skip_block_comment(rest)?.trim_start();
                continue;
            }
            break;
        }

        // accept both the bare `module`/`component` spelling and the
        // parenthesized `(module ...)`/`(component ...)` form
        let token = rest.strip_prefix('(').unwrap_or(rest);

        if starts_token(token, "component") {
            return Some(Self::Component);
        }
        if starts_token(token, "module") {
            return Some(Self::Module);
        }
        None
    }
}

/// Skips a `(; ... ;)` WAT block comment, honoring nested comment blocks.
///
/// Returns the text that follows the comment, or [`None`] when the comment is
/// unterminated.
fn skip_block_comment(rest: &str) -> Option<&str> {
    let bytes = rest.as_bytes();
    let mut depth: usize = 0;
    let mut idx: usize = 0;
    while idx + 1 < bytes.len() {
        if bytes[idx] == b'(' && bytes[idx + 1] == b';' {
            depth += 1;
            idx += 2;
            continue;
        }
        if bytes[idx] == b';' && bytes[idx + 1] == b')' {
            depth -= 1;
            idx += 2;
            if depth == 0 {
                return rest.get(idx..);
            }
            continue;
        }
        idx += 1;
    }
    None
}

/// Returns true when `text` starts with `token` followed by a token boundary,
/// so `components` or `module_name` do not match the header tokens.
fn starts_token(text: &str, token: &str) -> bool {
    match text.strip_prefix(token) {
        Some("") => true,
        Some(after) => !after
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'),
        None => false,
    }
}

impl fmt::Display for ScriptKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Module => "module",
            Self::Component => "component",
        })
    }
}

/// An instantiated WASM component script bound to a [`Context`].
///
/// Mirrors [`crate::Instance`] for the component path: the component instance
/// stays internal while the store is public so callers read stacks and
/// accumulated log lines exactly like they do for modules. Unlike the module
/// path, a component is instantiated once at build time, so no linker is
/// retained after instantiation.
pub struct ComponentInstance {
    instance: wasmtime::component::Instance,

    /// Virtual machine store for state
    pub store: Store<Context>,
}

impl ComponentInstance {
    pub(crate) const fn new(
        instance: wasmtime::component::Instance,
        store: Store<Context>,
    ) -> Self {
        Self { instance, store }
    }

    /// Executes the component's exported script entry point.
    ///
    /// The name maps to the script worlds: `for_great_justice` or
    /// `for-great-justice` calls the `unlock-script` export, and
    /// `move_every_zig` or `move-every-zig` calls the `lock-script` export.
    /// The legacy snake-case spellings are accepted for compatibility with the
    /// module path. A non-zero export result means success, matching
    /// [`crate::Instance::run`].
    pub fn run(&mut self, fname: &str) -> Result<bool, Error> {
        match fname {
            "for_great_justice" | "for-great-justice" => {
                let world = unlock_world::UnlockScript::new(&mut self.store, &self.instance)
                    .map_err(|e| VmError::ExecutionError {
                        function: fname.to_string(),
                        message: format!("failed to bind export '{fname}': {e}"),
                    })?;
                let result = world.call_for_great_justice(&mut self.store).map_err(|e| {
                    VmError::ExecutionError {
                        function: fname.to_string(),
                        message: format!("function '{fname}' execution failed: {e}"),
                    }
                })?;
                Ok(result != 0)
            }
            "move_every_zig" | "move-every-zig" => {
                let world =
                    lock_world::LockScript::new(&mut self.store, &self.instance).map_err(|e| {
                        VmError::ExecutionError {
                            function: fname.to_string(),
                            message: format!("failed to bind export '{fname}': {e}"),
                        }
                    })?;
                let result = world.call_move_every_zig(&mut self.store).map_err(|e| {
                    VmError::ExecutionError {
                        function: fname.to_string(),
                        message: format!("function '{fname}' execution failed: {e}"),
                    }
                })?;
                Ok(result != 0)
            }
            _ => Err(VmError::ExecutionError {
                function: fname.to_string(),
                message: format!(
                    "'{fname}' is not a script export; expected 'for_great_justice' or \
                     'move_every_zig' (kebab spellings accepted)"
                ),
            }
            .into()),
        }
    }

    /// Gets the accumulated log data from the context
    #[must_use]
    pub fn log(&self) -> Vec<u8> {
        self.store.data().log.clone()
    }
}

/// Converts a raw-ABI `Val` result into the `bool` result that the typed host
/// interface declares, using the same non-zero-means-success rule as
/// [`crate::Instance::run`].
fn as_bool(value: wasmtime::Val) -> bool {
    value.i32().is_some_and(|raw| raw != 0)
}

impl unlock_world::cryptid::wacc::host::Host for Context {
    fn push(&mut self, key: String) -> bool {
        as_bool(Self::push(self, &key))
    }

    fn push_value(&mut self, data: Vec<u8>) -> bool {
        as_bool(Self::push_value(self, data))
    }

    fn branch(&mut self, key: String) -> String {
        Self::branch(self, &key)
    }

    fn check_eq(&mut self, key: String) -> bool {
        as_bool(Self::check_eq(self, &key))
    }

    fn check_preimage(&mut self, key: String) -> bool {
        as_bool(Self::check_preimage(self, &key))
    }

    fn check_preimage_value(&mut self, hash: Vec<u8>, key: String) -> bool {
        as_bool(Self::check_preimage_value(self, &hash, &key))
    }

    fn check_signature(&mut self, key: String, msg: String) -> bool {
        as_bool(Self::check_signature(self, &key, &msg))
    }

    fn log(&mut self, text: String) -> bool {
        as_bool(Self::log(self, &text))
    }
}

impl lock_world::cryptid::wacc::host::Host for Context {
    fn push(&mut self, key: String) -> bool {
        as_bool(Self::push(self, &key))
    }

    fn push_value(&mut self, data: Vec<u8>) -> bool {
        as_bool(Self::push_value(self, data))
    }

    fn branch(&mut self, key: String) -> String {
        Self::branch(self, &key)
    }

    fn check_eq(&mut self, key: String) -> bool {
        as_bool(Self::check_eq(self, &key))
    }

    fn check_preimage(&mut self, key: String) -> bool {
        as_bool(Self::check_preimage(self, &key))
    }

    fn check_preimage_value(&mut self, hash: Vec<u8>, key: String) -> bool {
        as_bool(Self::check_preimage_value(self, &hash, &key))
    }

    fn check_signature(&mut self, key: String, msg: String) -> bool {
        as_bool(Self::check_signature(self, &key, &msg))
    }

    fn log(&mut self, text: String) -> bool {
        as_bool(Self::log(self, &text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_accepts_binary_kinds() {
        let module = [0x00, 0x61, 0x73, 0x6d, 0x01];
        assert_eq!(ScriptKind::detect(&module), Some(ScriptKind::Module));
        let component = [0x00, 0x61, 0x73, 0x6d, 0x0d];
        assert_eq!(ScriptKind::detect(&component), Some(ScriptKind::Component));
    }

    #[test]
    fn detect_rejects_unknown_binary_versions_and_garbage() {
        let unknown_version = [0x00, 0x61, 0x73, 0x6d, 0x02];
        assert_eq!(ScriptKind::detect(&unknown_version), None);
        assert_eq!(ScriptKind::detect(b"not wasm bytes"), None);
        assert_eq!(ScriptKind::detect(&[]), None);
        assert_eq!(ScriptKind::detect(&[0x00, 0x61]), None);
    }

    #[test]
    fn detect_accepts_text_kinds() {
        assert_eq!(ScriptKind::detect(b"(module)"), Some(ScriptKind::Module));
        assert_eq!(
            ScriptKind::detect(b"(component)"),
            Some(ScriptKind::Component)
        );
        assert_eq!(ScriptKind::detect(b"module $m"), Some(ScriptKind::Module));
        assert_eq!(
            ScriptKind::detect(b"component $c"),
            Some(ScriptKind::Component)
        );
        assert_eq!(
            ScriptKind::detect(b"  \n (module "),
            Some(ScriptKind::Module)
        );
    }

    #[test]
    fn detect_text_skips_leading_comments() {
        assert_eq!(
            ScriptKind::detect(b";; header\n(component)"),
            Some(ScriptKind::Component)
        );
        assert_eq!(
            ScriptKind::detect(b"(; hi ;) component"),
            Some(ScriptKind::Component)
        );
        assert_eq!(
            ScriptKind::detect(b"(;(;nested;) ;) module $m"),
            Some(ScriptKind::Module)
        );
        assert_eq!(ScriptKind::detect(b";; unterminated"), None);
        assert_eq!(ScriptKind::detect(b"(; unterminated"), None);
        assert_eq!(ScriptKind::detect(b";; c\n"), None);
    }

    #[test]
    fn detect_text_tolerates_a_byte_order_mark() {
        let bom_prefixed = "\u{feff}(component)".as_bytes();
        assert_eq!(
            ScriptKind::detect(bom_prefixed),
            Some(ScriptKind::Component)
        );
    }

    #[test]
    fn detect_text_enforces_token_boundaries() {
        assert_eq!(ScriptKind::detect(b"modulet"), None);
        assert_eq!(ScriptKind::detect(b"component-thing"), None);
        assert_eq!(ScriptKind::detect(b"components"), None);
        assert_eq!(ScriptKind::detect(b"module_name"), None);
        assert_eq!(ScriptKind::detect(b""), None);
    }

    #[test]
    fn kind_display_matches_detection_names() {
        assert_eq!(ScriptKind::Module.to_string(), "module");
        assert_eq!(ScriptKind::Component.to_string(), "component");
    }
}
