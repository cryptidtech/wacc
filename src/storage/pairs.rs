// SPDX-License-Identifier: FSL-1.1
use crate::{Key, Value};

/// Trait to a key-value storage mechanism
pub trait Pairs
{
    /// get a value associated with the key
    fn get(&self, key: &Key) -> Option<&Value>;

    /// add a key-value pair to the storage, returns the previous value if the
    /// key already exists in the data structure
    fn put(&mut self, key: &Key, value: &Value) -> Option<Value>;
}
