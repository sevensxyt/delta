use crate::object::DeltaObject;
use std::error::Error;

use super::ObjectFormat;

pub struct DeltaBlob {
    pub data: Vec<u8>,
}

impl DeltaObject for DeltaBlob {
    fn format(&self) -> ObjectFormat {
        ObjectFormat::Blob
    }

    fn serialise(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        Ok(self.data.clone())
    }

    fn deserialise(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        self.data = data.to_vec();
        Ok(())
    }
}
