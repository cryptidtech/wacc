/// Trait to a key-value storage mechanism
pub trait Pairs<V>
where
    V: Default + AsRef<[u8]>,
{
    /// the return type for when a put fails
    type Error;

    /// get a value associated with the key
    fn get(&self, key: &str) -> Option<V>;

    /// add a key-value pair to the storage, returns the previous value if the
    /// key already exists in the data structure
    fn put(&mut self, key: &str, value: &V) -> Result<V, Self::Error>;
}
