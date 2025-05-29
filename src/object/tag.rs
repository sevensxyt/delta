use crate::kvlm::{kvlm_parse, kvlm_serialise};
use anyhow::Result;
use indexmap::IndexMap;

use super::ObjectFormat;

pub struct DeltaTag {
    pub data: IndexMap<String, Vec<Vec<u8>>>,
}

impl DeltaTag {
    pub fn format(&self) -> ObjectFormat {
        ObjectFormat::Tag
    }

    pub fn serialise(&self) -> Result<Vec<u8>> {
        Ok(kvlm_serialise(&self.data)?.into_bytes())
    }

    pub fn deserialise(&mut self, data: &[u8]) -> Result<()> {
        self.data = kvlm_parse(data)?;
        Ok(())
    }
}
