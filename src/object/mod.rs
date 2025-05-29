use anyhow::Result;
use std::{error::Error, fmt, fmt::Display};

pub enum DeltaObject {
    Blob(DeltaBlob),
    Commit(DeltaCommit),
    Tag(DeltaTag),
    Tree(DeltaTree),
}

impl DeltaObject {
    pub fn serialise(&self) -> Result<Vec<u8>> {
        match self {
            Self::Blob(obj) => obj.serialise(),
            Self::Commit(obj) => obj.serialise(),
            Self::Tag(obj) => obj.serialise(),
            Self::Tree(obj) => obj.serialise(),
        }
    }

    pub fn deserialise(&mut self, data: &[u8]) -> Result<()> {
        match self {
            Self::Blob(obj) => obj.deserialise(data),
            Self::Commit(obj) => obj.deserialise(data),
            Self::Tag(obj) => obj.deserialise(data),
            Self::Tree(obj) => obj.deserialise(data),
        }
    }

    pub fn format(&self) -> ObjectFormat {
        match self {
            Self::Blob(obj) => obj.format(),
            Self::Commit(obj) => obj.format(),
            Self::Tag(obj) => obj.format(),
            Self::Tree(obj) => obj.format(),
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

impl Display for ObjectFormat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            ObjectFormat::Blob => "blob",
            ObjectFormat::Commit => "commit",
            ObjectFormat::Tag => "tag",
            ObjectFormat::Tree => "tree",
        };

        write!(f, "{}", s)
    }
}

pub mod blob;
pub mod commit;
pub mod tag;
pub mod tree;

pub use blob::DeltaBlob;
pub use commit::DeltaCommit;
pub use tag::DeltaTag;
pub use tree::DeltaTree;
