use std::{fmt::Display, fs};

use anyhow::{anyhow, Context, Result};

use crate::repo::DeltaRepository;

pub struct DeltaIndexEntry {
    pub ctime: (u32, u32),
    pub mtime: (u32, u32),
    pub device_id: u32,
    pub inode: u32,

    pub mode_type: ModeType,
    pub mode_perms: u16,

    // user id
    pub uid: u32,
    // group id
    pub gid: u32,

    // size of project (bytes)
    pub fsize: u32,

    pub sha: String,
    pub flag_assume_valid: bool,
    pub flag_stage: bool,

    pub name: String,
}

pub struct DeltaIndex {
    pub version: u32,
    pub entries: Vec<DeltaIndexEntry>,
}

impl Default for DeltaIndex {
    fn default() -> Self {
        DeltaIndex {
            version: 2,
            entries: Vec::new(),
        }
    }
}

pub enum ModeType {
    Regular,
    Symlink,
    Deltalink,
}

impl ModeType {
    fn from_byte(byte: u8) -> Result<Self> {
        let mode_type = match byte {
            0b1000 => Self::Regular,
            0b1010 => Self::Symlink,
            0b1110 => Self::Deltalink,
            b => return Err(anyhow!("Invalid mode type found {}", b)),
        };

        Ok(mode_type)
    }
}

impl Display for ModeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Regular => write!(f, "regular file"),
            Self::Symlink => write!(f, "symlink"),
            Self::Deltalink => write!(f, "delta link"),
        }
    }
}

impl DeltaIndex {
    pub fn read_index(repo: &DeltaRepository) -> Result<DeltaIndex> {
        let index_file = repo.repo_file(&["index"], false)?;

        if !index_file.exists() {
            return Ok(DeltaIndex {
                ..Default::default()
            });
        }

        let raw = fs::read(&index_file)
            .context(anyhow!("Error reading index file {}", index_file.display()))?;

        let header = &raw[..12];
        let (signature, rest) = header.split_at(4);

        if signature != "DIRC".as_bytes() {
            return Err(anyhow!(
                "Signature should be 'DIRC', found {}",
                std::str::from_utf8(signature)?
            ));
        }

        let (version, count) = rest.split_at(4);
        let version = u32::from_be_bytes(version.try_into()?);
        let count = usize::from_be_bytes(count.try_into()?);

        if version != 2 {
            return Err(anyhow!(
                "Only version 2 is supported, version {} found instead",
                version
            ));
        }

        let mut entries = vec![];
        let content = &raw[12..];

        let mut i = 0;
        while i < count {
            let parse = |x, y| -> Result<u128> {
                let data = &content[i + x..y + y];
                let byte_count = y - x;

                let res = match byte_count {
                    20 => u128::from_be_bytes(data.try_into()?),
                    4 => u32::from_be_bytes(data.try_into()?) as u128,
                    2 => u32::from_be_bytes(data.try_into()?) as u128,
                    c => return Err(anyhow!("Invalid byte count of {}", c)),
                };

                Ok(res)
            };

            let ctime_s = parse(0, 4)? as u32;
            let ctime_ns = parse(4, 8)? as u32;
            let ctime = (ctime_s, ctime_ns);

            let mtime_s = parse(8, 12)? as u32;
            let mtime_ns = parse(12, 16)? as u32;
            let mtime = (mtime_s, mtime_ns);

            let device_id = parse(16, 20)? as u32;
            let inode_number = parse(20, 24)? as u32;

            if parse(24, 26)? != 0 {
                return Err(anyhow!("Bytes 24 to 27 should be unused"));
            };

            let mode = parse(26, 28)? as u16;
            let mode_type = ModeType::from_byte((mode >> 12) as u8)?;
            let mode_perms = mode & 0b0000000111111111;

            let uid = parse(28, 32)? as u32;
            let gid = parse(32, 36)? as u32;
            let fsize = parse(36, 40)? as u32;

            let sha = format!("{:040x}", parse(40, 60)?);
            let flags = parse(60, 62)? as u16;

            let flag_assume_valid = flags & 0b1000000000000000 != 0;
            let flag_extended = flags & 0b0100000000000000 != 0;
            let flag_stage = flags & 0b0011000000000000 != 0;
            let name_length = (flags & 0b0000111111111111) as usize;

            if flag_extended {
                return Err(anyhow!("Flag should not be extended"));
            }

            i += 62;

            let raw_name = if name_length < 0xFFF {
                if content[i + name_length] != 0x00 {
                    return Err(anyhow!("Name should end at this point"));
                }

                i += 1;
                &content[i..i + name_length]
            } else {
                let null_index = content
                    .iter()
                    .position(|&b| b == 0x00)
                    .ok_or(anyhow!("Name is not terminated"))?;
                i += null_index;
                &content[i..null_index]
            };

            let name = String::from_utf8_lossy(raw_name).to_string();
            i += 8 - (i % 8);

            let entry = DeltaIndexEntry {
                ctime,
                mtime,
                device_id,
                inode: inode_number,
                mode_type,
                mode_perms,
                uid,
                gid,
                fsize,
                sha,
                flag_assume_valid,
                flag_stage,
                name,
            };

            entries.push(entry);
        }

        Ok(DeltaIndex { version, entries })
    }
}
