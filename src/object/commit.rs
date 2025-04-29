use crate::kvlm::{kvlm_parse, kvlm_serialise};
use crate::object::DeltaObject;
use indexmap::IndexMap;
use std::error::Error;

pub struct DeltaCommit {
    pub data: IndexMap<String, Vec<Vec<u8>>>,
}

impl DeltaObject for DeltaCommit {
    fn format(&self) -> &'static [u8] {
        return b"commit";
    }

    fn serialise(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        Ok(kvlm_serialise(&self.data)?.into_bytes())
    }

    fn deserialise(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        self.data = kvlm_parse(data)?;
        Ok(())
    }
}
