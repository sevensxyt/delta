use crate::object::DeltaObject;
use std::error::Error;

pub struct DeltaCommit {
    pub data: Vec<u8>,
}

impl DeltaObject for DeltaCommit {
    fn format(&self) -> &'static [u8] {
        return b"commit";
    }

    fn serialise(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        Ok(self.data.clone())
    }

    fn deserialise(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        self.data = data.to_vec();
        Ok(())
    }
}
