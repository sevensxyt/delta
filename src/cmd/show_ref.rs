use anyhow::{Context, Result};
use std::{collections::HashMap, env};

use crate::{
    reference::{ref_list, RefEntry},
    repo::DeltaRepository,
};

pub fn show_ref() -> Result<()> {
    let cwd = env::current_dir().context("Error getting cwd")?;
    let repo = DeltaRepository::find_repo(cwd)?;
    let refs = ref_list(&repo, None)?;
    display_ref(&refs, true, "refs")?;
    Ok(())
}

fn display_ref(refs: &HashMap<String, RefEntry>, with_hash: bool, prefix: &str) -> Result<()> {
    let mut prefix = prefix.to_string();
    if !prefix.is_empty() {
        prefix.push('/');
    }

    for (key, value) in refs.iter() {
        match value {
            RefEntry::Direct(v) => {
                if with_hash {
                    println!("{} {}{}", v, prefix, key);
                } else {
                    println!("{}{}", prefix, key);
                }
            }
            RefEntry::Indirect(refs) => {
                let prefix = format!("{}{}", prefix, key);
                display_ref(refs, with_hash, &prefix)?;
            }
        }
    }

    Ok(())
}
