use anyhow::Result;
use std::{collections::HashMap, env};

use crate::{
    reference::{ref_list, RefEntry},
    repo::DeltaRepository,
};

pub fn show_ref() -> Result<()> {
    let cwd = env::current_dir().expect("Error getting cwd");
    let repo = DeltaRepository::repo_find(cwd)?;
    let refs = ref_list(&repo, None)?;
    display_ref(&repo, &refs, true, "refs")?;
    Ok(())
}

fn display_ref(
    repo: &DeltaRepository,
    refs: &HashMap<String, RefEntry>,
    with_hash: bool,
    prefix: &str,
) -> Result<()> {
    let mut prefix = prefix.to_string();
    if prefix != "" {
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
                display_ref(repo, refs, with_hash, &prefix)?;
            }
        }
    }

    Ok(())
}
