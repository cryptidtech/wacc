/// Trait to a key-value storage mechanism
pub trait Pairs
{
    /// get a value associated with the key
    fn get(&self, key: &str) -> Option<Vec<u8>>;

    /// add a key-value pair to the storage, returns the previous value if the
    /// key already exists in the data structure
    fn put(&mut self, key: &str, value: &dyn AsRef<[u8]>) -> Option<Vec<u8>>;
}
