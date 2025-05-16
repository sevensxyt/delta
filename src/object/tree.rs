use crate::object::DeltaObject;
use std::{error::Error, path::PathBuf};

use super::ObjectFormat;

struct DeltaTreeLeaf {
    pub mode: [u8; 6],
    pub path: PathBuf,
    pub sha: String,
}

impl DeltaTreeLeaf {
    fn parse_tree(raw: &[u8]) -> Result<Vec<DeltaTreeLeaf>, Box<dyn Error>> {
        let mut offset = 0;
        let mut leaves = vec![];

        while offset < raw.len() {
            let (new_pos, data) = DeltaTreeLeaf::parse_leaf(raw, offset)?;
            offset = new_pos;
            leaves.push(data);
        }

        Ok(leaves)
    }

    fn parse_leaf(raw: &[u8], start: usize) -> Result<(usize, DeltaTreeLeaf), Box<dyn Error>> {
        let space_index = raw
            .iter()
            .skip(start)
            .position(|b| *b == b' ')
            .map(|i| i + start)
            .ok_or("Invalid format: Could not find mode")?;
        let mode: [u8; 6] = raw[start..space_index]
            .try_into()
            .map_err(|_| "Mode must be 6 bytes")?;

        let null_index = raw
            .iter()
            .skip(start)
            .position(|b| *b == b'\x00')
            .map(|i| i + start)
            .ok_or("Invalid format: Could not find path")?;
        let path = std::str::from_utf8(&raw[space_index + 1..null_index])?;

        let end = null_index + 21;
        let sha = &raw[null_index + 1..end]
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();

        Ok((
            end,
            DeltaTreeLeaf {
                mode,
                path: path.into(),
                sha: sha.to_string(),
            },
        ))
    }
}

pub struct DeltaTree {
    pub data: Vec<u8>,
}

impl DeltaObject for DeltaTree {
    fn format(&self) -> ObjectFormat {
        ObjectFormat::Tree
    }

    fn serialise(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        Ok(self.data.clone())
    }

    fn deserialise(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        self.data = data.to_vec();
        Ok(())
    }
}
