use crate::kvlm::{kvlm_parse, kvlm_serialise};
use crate::object::DeltaObject;
use anyhow::Result;
use indexmap::IndexMap;
use std::error::Error;

use super::ObjectFormat;

pub struct DeltaCommit {
    pub data: IndexMap<String, Vec<Vec<u8>>>,
}

impl DeltaCommit {
    pub fn format(&self) -> ObjectFormat {
        ObjectFormat::Commit
    }

    pub fn serialise(&self) -> Result<Vec<u8>> {
        Ok(kvlm_serialise(&self.data)?.into_bytes())
    }

    pub fn deserialise(&mut self, data: &[u8]) -> Result<()> {
        self.data = kvlm_parse(data)?;
        Ok(())
    }
}

impl TryFrom<DeltaObject> for DeltaCommit {
    type Error = Box<dyn Error>;

    fn try_from(value: DeltaObject) -> Result<Self, Self::Error> {
        match value {
            DeltaObject::Commit(c) => Ok(c),
            other => Err(format!("Expected commit, got {:?}", other.format()).into()),
        }
    }
}
