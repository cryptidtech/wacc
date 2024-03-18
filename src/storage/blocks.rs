use multicid::Cid;

/// Block storage trait for getting and putting content address data
pub trait Blocks {
    /// Error type returned when a block put fails
    type Error;

    /// Try to get a block from it's content address
    fn get(&self, cid: &Cid) -> Option<Vec<u8>>;

    /// Try to put a block and get back its content address
    fn put(&mut self, data: &impl AsRef<[u8]>) -> Result<Cid, Self::Error>;
}
