use anyhow::Result;

use super::ObjectFormat;

pub struct DeltaBlob {
    pub data: Vec<u8>,
}

impl DeltaBlob {
    pub fn format(&self) -> ObjectFormat {
        ObjectFormat::Blob
    }

    pub fn serialise(&self) -> Result<Vec<u8>> {
        Ok(self.data.clone())
    }

    pub fn deserialise(&mut self, data: &[u8]) -> Result<()> {
        self.data = data.to_vec();
        Ok(())
    }
}
