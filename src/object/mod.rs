use std::error::Error;

pub trait DeltaObject {
    fn serialise(&self) -> Result<Vec<u8>, Box<dyn Error>>;
    fn deserialise(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>>;
    fn format(&self) -> &'static [u8];
}

pub mod blob;
pub mod commit;
pub mod tag;
pub mod tree;

pub use blob::DeltaBlob;
pub use commit::DeltaCommit;
pub use tag::DeltaTag;
pub use tree::DeltaTree;
