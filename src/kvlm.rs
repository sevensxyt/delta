use std::fmt::Display;

use anyhow::{anyhow, Result};
use indexmap::IndexMap;

pub enum KvlmKey {
    Message,
    Parent,
    Tree,
    Author,
    Committer,
}

impl KvlmKey {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Message => "",
            Self::Parent => "parent",
            Self::Tree => "tree",
            Self::Author => "author",
            Self::Committer => "committer",
        }
    }
}

impl Display for KvlmKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

pub fn kvlm_parse(raw: &[u8]) -> Result<IndexMap<String, Vec<Vec<u8>>>> {
    fn parse(
        raw: &[u8],
        start: usize,
        mut hashmap: IndexMap<String, Vec<Vec<u8>>>,
    ) -> Result<IndexMap<String, Vec<Vec<u8>>>> {
        if start >= raw.len() {
            return Ok(hashmap);
        }

        let space = raw
            .iter()
            .skip(start)
            .position(|&b| b == b' ')
            .map(|i| start + i);

        let newline = raw
            .iter()
            .skip(start)
            .position(|&b| b == b'\n')
            .map(|i| start + i)
            .ok_or(anyhow!("Newline not found"))?;

        let is_message = match space {
            Some(space) => space > newline,
            None => true,
        };

        if is_message {
            if newline != start {
                return Err(anyhow!("Unexpected format: newline before key"));
            }

            let message = std::str::from_utf8(&raw[start + 1..])?.to_string();
            hashmap
                .entry(String::new())
                .or_default()
                .push(message.into_bytes());

            return Ok(hashmap);
        }

        let space = space.ok_or(anyhow!("Space should be defined at this point"))?;
        let key = std::str::from_utf8(&raw[start..space])?.to_string();
        let mut end = space;

        loop {
            end = raw[end + 1..]
                .iter()
                .position(|&b| b == b'\n')
                .map(|i| end + 1 + i)
                .ok_or(anyhow!("Unexpected end of value"))?;

            if end + 1 >= raw.len() || raw[end + 1] != b' ' {
                break;
            }
        }

        let value = std::str::from_utf8(&raw[space + 1..end])?.replace("\n ", "\n");

        hashmap.entry(key).or_default().push(value.into_bytes());

        parse(raw, end + 1, hashmap)
    }

    let hashmap = parse(raw, 0, IndexMap::new())?;
    Ok(hashmap)
}

pub fn kvlm_serialise(kvlm: &IndexMap<String, Vec<Vec<u8>>>) -> Result<String> {
    let mut out = String::new();

    for (key, values) in kvlm {
        if key.is_empty() {
            continue;
        }

        for value in values {
            let line = String::from_utf8_lossy(value).replace("\n", "\n ");
            out.push_str(key);
            out.push(' ');
            out.push_str(&line);
            out.push('\n');
        }
    }

    if let Some(message) = kvlm.get(KvlmKey::Message.as_str()) {
        out.push('\n');
        for m in message {
            out.push_str(&String::from_utf8_lossy(m));
        }
    }

    Ok(out)
}
