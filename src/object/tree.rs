use super::ObjectFormat;
use crate::object::DeltaObject;
use hex;
use std::{error::Error, path::PathBuf};

pub struct DeltaTreeLeaf {
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

    fn tree_leaf_sort_key(leaf: &DeltaTreeLeaf) -> String {
        let mut key = leaf.path.to_str().unwrap_or("invalid utf8").to_owned();
        if &leaf.mode[..2] != b"10" {
            key.push('/');
        }
        key
    }
}

pub struct DeltaTree {
    pub data: Vec<u8>,
}

impl DeltaTree {
    fn serialise(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut leaves = DeltaTreeLeaf::parse_tree(&self.data)?;
        leaves.sort_by_key(DeltaTreeLeaf::tree_leaf_sort_key);
        let mut res = vec![];

        for leaf in leaves {
            res.extend(&leaf.mode);

            res.push(b' ');

            let path = leaf.path.to_str().ok_or("Invalid UTF-8 path")?.as_bytes();
            res.extend(path);

            res.push(b'\x00');

            let sha: [u8; 20] = hex::decode(leaf.sha)?
                .try_into()
                .map_err(|_| "SHA must be 20 bytes")?;
            res.extend(&sha);
        }

        Ok(res)
    }

    pub fn items(&self) -> Result<Vec<DeltaTreeLeaf>, Box<dyn Error>> {
        Ok(DeltaTreeLeaf::parse_tree(&self.data)?)
    }
}

impl DeltaObject for DeltaTree {
    fn format(&self) -> ObjectFormat {
        ObjectFormat::Tree
    }

    fn serialise(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        Ok(self.serialise()?)
    }

    fn deserialise(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        self.data = data.to_vec();
        Ok(())
    }
}
