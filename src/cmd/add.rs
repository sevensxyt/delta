use std::{collections::HashSet, fs, os::unix::fs::MetadataExt, path::PathBuf};

use anyhow::{anyhow, Result};

use crate::{
    index::{DeltaIndex, DeltaIndexEntry, ModeType},
    object::ObjectFormat,
    repo::DeltaRepository,
};

use super::rm::remove;

pub fn add(path: Vec<PathBuf>) -> Result<()> {
    let repo = DeltaRepository::find_repo(std::env::current_dir()?)?;

    add_to_index(&repo, &path, false, true)
}

fn add_to_index(
    repo: &DeltaRepository,
    paths: &Vec<PathBuf>,
    delete: bool,
    skip_missing: bool,
) -> Result<()> {
    remove(repo, paths, delete, skip_missing)?;
    let worktree = &repo.worktree;
    let mut clear_paths = HashSet::<(PathBuf, PathBuf)>::new();

    for path in paths {
        let absolute_path = fs::canonicalize(path)?;
        if !absolute_path.starts_with(worktree) && absolute_path.is_file() {
            return Err(anyhow!(
                "Not a file, or outside of worktree {}",
                absolute_path.display()
            ));
        }

        let relative_path = absolute_path.strip_prefix(worktree)?.to_path_buf();
        clear_paths.insert((absolute_path, relative_path));
    }

    let mut index = DeltaIndex::read_index(repo)?;

    for (absolute_path, relative_path) in clear_paths {
        let data = fs::read(&absolute_path)?;
        let sha = DeltaRepository::hash_object(data, ObjectFormat::Blob, true)?;

        let metadata = fs::metadata(&absolute_path)?;
        let ctime_s = metadata.ctime() as u32;
        let ctime_ns = (metadata.ctime_nsec() % u32::pow(10, 9) as i64) as u32;
        let ctime = (ctime_s, ctime_ns);

        let mtime_s = metadata.mtime() as u32;
        let mtime_ns = (metadata.mtime_nsec() % u32::pow(10, 9) as i64) as u32;
        let mtime = (mtime_s, mtime_ns);

        let entry = DeltaIndexEntry {
            ctime,
            mtime,
            device_id: metadata.rdev().try_into()?,
            inode: metadata.ino().try_into()?,
            mode_type: ModeType::Regular,
            uid: metadata.uid(),
            gid: metadata.gid(),
            mode_perms: 0o644,
            fsize: metadata.size().try_into()?,
            sha,
            assume_valid_flag: false,
            stage_flag: 0,
            name: relative_path.display().to_string(),
        };

        index.entries.push(entry);
    }

    Ok(())
}
