// SPDX-License-Identifier: Apache-2.0

//! Module caching for improved performance
//!
//! Compiled WASM modules can be cached and reused across multiple
//! instance creations, significantly improving performance when the
//! same script is executed repeatedly.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use wasmtime::component::Component;
use wasmtime::Module;

/// A cache for compiled WASM modules and components
///
/// This cache stores compiled modules and components keyed by their source
/// bytes hash, allowing reuse of expensive compilation results.
///
/// # Thread Safety
///
/// `ModuleCache` is thread-safe and can be shared across threads using Arc.
///
/// # Examples
///
/// ```
/// use wacc::vm::ModuleCache;
/// use std::sync::Arc;
///
/// let cache = Arc::new(ModuleCache::new());
/// // Share cache across threads...
/// ```
pub struct ModuleCache {
    cache: RwLock<HashMap<u64, Arc<Module>>>,
    byte_cache: RwLock<HashMap<[u8; 32], Arc<Module>>>,
    component_cache: RwLock<HashMap<[u8; 32], Arc<Component>>>,
    max_entries: usize,
}

impl ModuleCache {
    /// Default maximum cache entries
    pub const DEFAULT_MAX_ENTRIES: usize = 100;

    /// Creates a new module cache with default capacity
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(Self::DEFAULT_MAX_ENTRIES)
    }

    /// Creates a new module cache with specified capacity
    #[must_use]
    pub fn with_capacity(max_entries: usize) -> Self {
        Self {
            cache: RwLock::new(HashMap::with_capacity(max_entries.min(100))),
            byte_cache: RwLock::new(HashMap::with_capacity(max_entries.min(100))),
            component_cache: RwLock::new(HashMap::with_capacity(max_entries.min(100))),
            max_entries,
        }
    }

    /// Gets a cached module if available
    ///
    /// Returns None if the module is not in cache.
    pub fn get(&self, key: u64) -> Option<Arc<Module>> {
        self.cache.read().ok()?.get(&key).cloned()
    }

    /// Inserts a compiled module into the cache
    ///
    /// If the cache is full, this is a no-op (simple eviction strategy).
    /// For production use, consider implementing LRU eviction.
    pub fn insert(&self, key: u64, module: Arc<Module>) {
        if self.len() >= self.max_entries {
            return;
        }
        if let Ok(mut cache) = self.cache.write() {
            cache.insert(key, module);
            // Deferred: Implement LRU eviction when cache is full
        }
    }

    /// Gets a module keyed by the collision-resistant digest of its WASM bytes.
    pub fn get_bytes(&self, bytes: &[u8]) -> Option<Arc<Module>> {
        self.byte_cache
            .read()
            .ok()?
            .get(&Self::digest_bytes(bytes))
            .cloned()
    }

    /// Inserts a module keyed by the collision-resistant digest of its WASM bytes.
    pub fn insert_bytes(&self, bytes: &[u8], module: Arc<Module>) {
        if self.len() >= self.max_entries {
            return;
        }
        if let Ok(mut cache) = self.byte_cache.write() {
            cache.insert(Self::digest_bytes(bytes), module);
        }
    }

    /// Gets a compiled component keyed by the collision-resistant digest of its bytes.
    pub fn get_component_bytes(&self, bytes: &[u8]) -> Option<Arc<Component>> {
        self.component_cache
            .read()
            .ok()?
            .get(&Self::digest_bytes(bytes))
            .cloned()
    }

    /// Inserts a compiled component keyed by the collision-resistant digest of
    /// its bytes.
    ///
    /// The component cache is bounded by the same `max_entries` limit
    /// independently of the module caches. If the cache is full, this is a
    /// no-op (simple eviction strategy).
    pub fn insert_component_bytes(&self, bytes: &[u8], component: Arc<Component>) {
        if self.component_len() >= self.max_entries {
            return;
        }
        if let Ok(mut cache) = self.component_cache.write() {
            cache.insert(Self::digest_bytes(bytes), component);
        }
    }

    /// Returns the number of cached components
    pub fn component_len(&self) -> usize {
        self.component_cache.read().map_or(0, |c| c.len())
    }

    /// Returns the number of cached modules
    pub fn len(&self) -> usize {
        let keyed = self.cache.read().map_or(0, |c| c.len());
        let byte_keyed = self.byte_cache.read().map_or(0, |c| c.len());
        keyed + byte_keyed
    }

    /// Returns true if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clears all cached modules and components
    pub fn clear(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
        if let Ok(mut cache) = self.byte_cache.write() {
            cache.clear();
        }
        if let Ok(mut cache) = self.component_cache.write() {
            cache.clear();
        }
    }

    /// Computes a simple hash of the WASM bytes for a caller-managed cache key.
    #[must_use]
    pub fn hash_bytes(bytes: &[u8]) -> u64 {
        let digest = Self::digest_bytes(bytes);
        u64::from_le_bytes(digest[..8].try_into().expect("digest has at least 8 bytes"))
    }

    fn digest_bytes(bytes: &[u8]) -> [u8; 32] {
        *blake3::hash(bytes).as_bytes()
    }
}

impl Default for ModuleCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasmtime::{Config, Engine};

    const SIMPLE_WASM: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, // WASM magic
        0x01, 0x00, 0x00, 0x00, // version 1
        0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7f, // type section
        0x03, 0x02, 0x01, 0x00, // function section
        0x07, 0x08, 0x01, 0x04, 0x74, 0x65, 0x73, 0x74, 0x00, 0x00, // export
        0x0a, 0x06, 0x01, 0x04, 0x00, 0x41, 0x01, 0x0b, // code
    ];

    #[test]
    fn test_cache_basic() {
        let cache = ModuleCache::new();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());

        let engine = Engine::new(&Config::default()).unwrap();
        let module = Module::new(&engine, SIMPLE_WASM).unwrap();
        let key = ModuleCache::hash_bytes(SIMPLE_WASM);

        cache.insert(key, Arc::new(module));
        assert_eq!(cache.len(), 1);
        assert!(!cache.is_empty());

        let cached = cache.get(key);
        assert!(cached.is_some());
    }

    #[test]
    fn test_cache_miss() {
        let cache = ModuleCache::new();
        assert!(cache.get(12345).is_none());
    }

    #[test]
    fn test_cache_clear() {
        let cache = ModuleCache::new();
        let engine = Engine::new(&Config::default()).unwrap();
        let module = Module::new(&engine, SIMPLE_WASM).unwrap();
        let key = ModuleCache::hash_bytes(SIMPLE_WASM);

        cache.insert(key, Arc::new(module));
        assert_eq!(cache.len(), 1);

        cache.clear();
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_cache_capacity() {
        let cache = ModuleCache::with_capacity(2);
        let engine = Engine::new(&Config::default()).unwrap();

        // Insert first module
        let module1 = Module::new(&engine, SIMPLE_WASM).unwrap();
        cache.insert(1, Arc::new(module1));
        assert_eq!(cache.len(), 1);

        // Insert second module
        let module2 = Module::new(&engine, SIMPLE_WASM).unwrap();
        cache.insert(2, Arc::new(module2));
        assert_eq!(cache.len(), 2);

        // Cache is full, third insert should be rejected
        let module3 = Module::new(&engine, SIMPLE_WASM).unwrap();
        cache.insert(3, Arc::new(module3));
        assert_eq!(cache.len(), 2); // Should still be 2
    }

    const SIMPLE_COMPONENT: &[u8] = b"(component)";

    fn component_engine() -> Engine {
        let mut config = Config::default();
        config.wasm_component_model(true);
        Engine::new(&config).unwrap()
    }

    #[test]
    fn test_component_cache_round_trip() {
        let cache = ModuleCache::new();
        assert!(cache.get_component_bytes(SIMPLE_COMPONENT).is_none());

        let engine = component_engine();
        let component = Component::new(&engine, SIMPLE_COMPONENT).unwrap();

        cache.insert_component_bytes(SIMPLE_COMPONENT, Arc::new(component));

        let cached = cache.get_component_bytes(SIMPLE_COMPONENT);
        assert!(cached.is_some());
        assert_eq!(cache.component_len(), 1);
        assert_eq!(cache.len(), 0); // modules remain counted separately

        cache.clear();
        assert_eq!(cache.component_len(), 0);
    }

    #[test]
    fn test_component_cache_capacity_independent() {
        let cache = ModuleCache::with_capacity(1);
        let engine = component_engine();
        let component = Component::new(&engine, SIMPLE_COMPONENT).unwrap();

        cache.insert_component_bytes(SIMPLE_COMPONENT, Arc::new(component));

        // the component cache is full, so another insert is rejected
        let component2 = Component::new(&engine, SIMPLE_COMPONENT).unwrap();
        cache.insert_component_bytes(SIMPLE_COMPONENT, Arc::new(component2));
        assert_eq!(cache.component_len(), 1);

        // an unrelated module still fits its own bound
        let module = Module::new(&engine, SIMPLE_WASM).unwrap();
        cache.insert(ModuleCache::hash_bytes(SIMPLE_WASM), Arc::new(module));
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.component_len(), 1);
    }
}
