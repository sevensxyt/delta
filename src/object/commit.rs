use crate::kvlm::{kvlm_parse, kvlm_serialise};
use crate::object::{DeltaObject, DeltaObjectEnum};
use indexmap::IndexMap;
use std::error::Error;

use super::ObjectFormat;

pub struct DeltaCommit {
    pub data: IndexMap<String, Vec<Vec<u8>>>,
}

impl DeltaObject for DeltaCommit {
    fn format(&self) -> ObjectFormat {
        ObjectFormat::Commit
    }

    fn serialise(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        Ok(kvlm_serialise(&self.data)?.into_bytes())
    }

    fn deserialise(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        self.data = kvlm_parse(data)?;
        Ok(())
    }
}

impl TryFrom<DeltaObjectEnum> for DeltaCommit {
    type Error = Box<dyn Error>;

    fn try_from(value: DeltaObjectEnum) -> Result<Self, Self::Error> {
        match value {
            DeltaObjectEnum::Commit(c) => Ok(c),
            other => Err(format!("Expected commit, got {:?}", other.format()).into()),
        }
    }
}
