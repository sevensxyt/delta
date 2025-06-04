use anyhow::{anyhow, Result};

use crate::{index::DeltaIndex, repo::DeltaRepository};
use chrono::DateTime;
use users::{get_group_by_gid, get_user_by_uid};

pub fn ls_files(verbose: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let repo = DeltaRepository::find_repo(cwd)?;
    let DeltaIndex { version, entries } = DeltaIndex::read_index(&repo)?;

    if verbose {
        println!(
            "Index file format v{}, containing {} entries",
            version,
            entries.len()
        );
    }

    for e in entries {
        println!("{}", e.name);

        if verbose {
            println!("{} with perms: {:o}", e.mode_type, e.mode_perms);
            println!("\ton blob: {}", e.sha);

            let created = DateTime::from_timestamp(e.ctime.0.into(), e.ctime.1)
                .ok_or(anyhow!("Invalid created time {:?}", e.ctime))?;
            let modified = DateTime::from_timestamp(e.mtime.0.into(), e.mtime.1)
                .ok_or(anyhow!("Invalid modified time {:?}", e.mtime))?;

            println!("\tcreated: {}, modified: {}", created, modified);
            println!("\tdevice: {}, inode: {}", e.device_id, e.inode);

            let user =
                get_user_by_uid(e.uid).ok_or(anyhow!("User with uid {} not found", e.uid))?;
            let group =
                get_group_by_gid(e.gid).ok_or(anyhow!("Group with gid {} not found", e.gid))?;

            println!(
                "\tuser: {} ({}), group: {} ({})",
                user.name().to_string_lossy(),
                e.uid,
                group.name().to_string_lossy(),
                e.gid
            );
            println!(
                "\tflags: stage={}, assume_valid={}",
                e.stage_flag, e.assume_valid_flag
            );
        }
    }

    Ok(())
}
