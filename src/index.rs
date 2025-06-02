use hex::FromHex;
use std::{
    fmt::Display,
    fs::{self, File},
    io::Write,
    iter::repeat_n,
};

use anyhow::{anyhow, Context, Result};

use crate::repo::DeltaRepository;

const ASSUME_VALID_FLAG: u16 = 0x8000;
const STAGE_FLAG_MASK: u16 = 0x3000;
const NAME_LENGTH_MASK: u16 = 0x0FFF;

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
    pub assume_valid_flag: bool,
    pub stage_flag: u8,

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
    fn from_bytes(byte: u8) -> Result<Self> {
        let mode_type = match byte {
            0b1000 => Self::Regular,
            0b1010 => Self::Symlink,
            0b1110 => Self::Deltalink,
            b => return Err(anyhow!("Invalid mode type found {}", b)),
        };

        Ok(mode_type)
    }

    fn to_bytes(&self) -> u8 {
        match self {
            Self::Regular => 0b1000,
            Self::Symlink => 0b1010,
            Self::Deltalink => 0b1110,
        }
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
        let count = u32::from_be_bytes(count.try_into()?) as usize;

        if version != 2 {
            return Err(anyhow!(
                "Only version 2 is supported, version {} found instead",
                version
            ));
        }

        let mut entries = vec![];
        let content = &raw[12..];

        let mut index = 0;
        while index < count {
            let parse = |x, y| -> Result<u128> {
                let data = &content[index + x..index + y];
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
            let mode_type = ModeType::from_bytes((mode >> 12) as u8)?;
            let mode_perms = mode & 0b0000000111111111;

            let uid = parse(28, 32)? as u32;
            let gid = parse(32, 36)? as u32;
            let fsize = parse(36, 40)? as u32;

            let sha = format!("{:040x}", parse(40, 60)?);
            let flags = parse(60, 62)? as u16;

            let assume_valid_flag = flags & ASSUME_VALID_FLAG != 0;
            let extended_flag = flags & 0b0100000000000000 != 0;
            let stage_flag = ((flags & STAGE_FLAG_MASK) >> 12) as u8;
            let name_length = (flags & NAME_LENGTH_MASK) as usize;

            if extended_flag {
                return Err(anyhow!("Flag should not be extended"));
            }

            index += 62;

            let raw_name = if name_length < 0xFFF {
                let pos = index + name_length;
                if let Some(&b) = content.get(pos) {
                    if b != 0x00 {
                        return Err(anyhow!("Name should end at this point"));
                    }
                } else {
                    return Err(anyhow!(
                        "Position {} exceeds content length of {}",
                        pos,
                        content.len()
                    ));
                }

                index += 1;
                &content[index..index + name_length]
            } else {
                let null_index = content
                    .iter()
                    .skip(index)
                    .position(|&b| b == 0x00)
                    .ok_or(anyhow!("Name is not terminated"))?;
                index += null_index;
                &content[index..null_index]
            };

            let name = String::from_utf8_lossy(raw_name).to_string();
            index += (8 - (index % 8)) % 8;

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
                assume_valid_flag,
                stage_flag,
                name,
            };

            entries.push(entry);
        }

        Ok(DeltaIndex { version, entries })
    }

    pub fn write_index(&self, repo: &DeltaRepository) -> Result<()> {
        let path = repo.repo_file(&["index"], true)?;
        let mut file = File::create(path)?;
        let mut content = Vec::<u8>::new();

        content.extend_from_slice(b"DIRC");
        content.extend_from_slice(&self.version.to_be_bytes());
        content.extend_from_slice(&self.entries.len().to_be_bytes());

        let mut index = 0;
        for e in &self.entries {
            content.extend_from_slice(&e.ctime.0.to_be_bytes());
            content.extend_from_slice(&e.ctime.1.to_be_bytes());

            content.extend_from_slice(&e.mtime.0.to_be_bytes());
            content.extend_from_slice(&e.mtime.1.to_be_bytes());

            content.extend_from_slice(&e.device_id.to_be_bytes());
            content.extend_from_slice(&e.inode.to_be_bytes());

            let mode = ((e.mode_type.to_bytes() as u16) << 12) | e.mode_perms;
            content.extend_from_slice(&mode.to_be_bytes());

            content.extend_from_slice(&e.uid.to_be_bytes());
            content.extend_from_slice(&e.gid.to_be_bytes());

            content.extend_from_slice(&e.fsize.to_be_bytes());

            let sha = <[u8; 20]>::from_hex(&e.sha)?;
            content.extend_from_slice(&sha);

            let assume_valid_flag = if e.assume_valid_flag { 0x1 << 15 } else { 0 };
            let stage_flag = (e.stage_flag as u16) << 12;

            let name_bytes = &e.name.as_bytes();
            let name_len = name_bytes.len().min(0xFFF);

            let data = assume_valid_flag | stage_flag | name_len as u16;
            content.extend_from_slice(&data.to_be_bytes());

            content.extend_from_slice(name_bytes);
            content.push(0);

            index += 62 + name_bytes.len() + 1;
            let padding = (8 - (index % 8)) % 8;
            content.extend(repeat_n(0, padding));
            index += padding;
        }

        file.write_all(&content)?;

        Ok(())
    }
}
