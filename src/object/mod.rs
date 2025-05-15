use std::error::Error;

pub enum DeltaObjectEnum {
    Blob(DeltaBlob),
    Commit(DeltaCommit),
    Tag(DeltaTag),
    Tree(DeltaTree),
}

impl DeltaObject for DeltaObjectEnum {
    fn serialise(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        match self {
            DeltaObjectEnum::Blob(obj) => obj.serialise(),
            DeltaObjectEnum::Commit(obj) => obj.serialise(),
            DeltaObjectEnum::Tag(obj) => obj.serialise(),
            DeltaObjectEnum::Tree(obj) => obj.serialise(),
        }
    }

    fn deserialise(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        match self {
            DeltaObjectEnum::Blob(obj) => obj.deserialise(data),
            DeltaObjectEnum::Commit(obj) => obj.deserialise(data),
            DeltaObjectEnum::Tag(obj) => obj.deserialise(data),
            DeltaObjectEnum::Tree(obj) => obj.deserialise(data),
        }
    }

    fn format(&self) -> ObjectFormat {
        match self {
            DeltaObjectEnum::Blob(obj) => obj.format(),
            DeltaObjectEnum::Commit(obj) => obj.format(),
            DeltaObjectEnum::Tag(obj) => obj.format(),
            DeltaObjectEnum::Tree(obj) => obj.format(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectFormat {
    Blob,
    Commit,
    Tag,
    Tree,
}

impl ObjectFormat {
    pub fn as_bytes(&self) -> &'static [u8] {
        match self {
            Self::Blob => b"blob",
            Self::Commit => b"commit",
            Self::Tag => b"tag",
            Self::Tree => b"tree",
        }
    }

    pub fn from_bytes(format: &[u8]) -> Result<ObjectFormat, Box<dyn Error>> {
        match format {
            b"blob" => Ok(Self::Blob),
            b"commit" => Ok(Self::Commit),
            b"tag" => Ok(Self::Tag),
            b"tree" => Ok(Self::Tree),
            _ => Err(format!("Unknown format {:?}", format).into()),
        }
    }
}

pub trait DeltaObject {
    fn serialise(&self) -> Result<Vec<u8>, Box<dyn Error>>;
    fn deserialise(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>>;
    fn format(&self) -> ObjectFormat;
}

pub mod blob;
pub mod commit;
pub mod tag;
pub mod tree;

pub use blob::DeltaBlob;
pub use commit::DeltaCommit;
pub use tag::DeltaTag;
pub use tree::DeltaTree;
