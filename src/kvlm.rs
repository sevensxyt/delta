use indexmap::IndexMap;
use std::error::Error;

pub const MESSAGE_KEY: &str = "";
pub const PARENT_KEY: &str = "parent";

pub fn kvlm_parse(raw: &[u8]) -> Result<IndexMap<String, Vec<Vec<u8>>>, Box<dyn Error>> {
    fn parse(
        raw: &[u8],
        start: usize,
        mut hashmap: IndexMap<String, Vec<Vec<u8>>>,
    ) -> Result<IndexMap<String, Vec<Vec<u8>>>, Box<dyn Error>> {
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
            .ok_or("Newline not found")?;

        let is_message = match space {
            Some(space) => space > newline,
            None => true,
        };

        if is_message {
            if newline != start {
                return Err("Unexpected format: newline before key".into());
            }

            let message = std::str::from_utf8(&raw[start + 1..])?.to_string();
            hashmap
                .entry(String::new())
                .or_insert_with(Vec::new)
                .push(message.into_bytes());

            return Ok(hashmap);
        }

        let space = space.ok_or("Space should be defined at this point")?;
        let key = std::str::from_utf8(&raw[start..space])?.to_string();
        let mut end = space;

        loop {
            end = raw[end + 1..]
                .iter()
                .position(|&b| b == b'\n')
                .map(|i| end + 1 + i)
                .ok_or("Unexpected end of value")?;

            if end + 1 >= raw.len() || raw[end + 1] != b' ' {
                break;
            }
        }

        let value = std::str::from_utf8(&raw[space + 1..end])?.replace("\n ", "\n");

        hashmap
            .entry(key)
            .or_insert_with(Vec::new)
            .push(value.into_bytes());

        parse(raw, end + 1, hashmap)
    }

    let hashmap = parse(raw, 0, IndexMap::new())?;
    Ok(hashmap)
}

pub fn kvlm_serialise(kvlm: &IndexMap<String, Vec<Vec<u8>>>) -> Result<String, Box<dyn Error>> {
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

    if let Some(message) = kvlm.get("") {
        out.push('\n');
        for m in message {
            out.push_str(&String::from_utf8_lossy(m));
        }
    }

    Ok(out)
}
