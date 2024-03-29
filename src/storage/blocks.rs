use multicid::Cid;

/// Block storage trait for getting and putting content address data
pub trait Blocks {
    /// Try to get a block from it's content address
    fn get(&self, cid: &Cid) -> Option<Vec<u8>>;

    /// Try to put a block and get back its content address
    fn put(&mut self, data: &dyn AsRef<[u8]>) -> Option<Cid>;
}
