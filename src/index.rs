use hex::{encode, FromHex};
use std::{
    fmt::Display,
    fs::{self, File},
    io::{Cursor, Read, Write},
    iter::repeat_n,
};

use anyhow::{anyhow, Context, Result};

use crate::repo::DeltaRepository;

const STAGE_FLAG_MASK: u16 = 0x3000;
const NAME_LENGTH_MASK: u16 = 0x0FFF;
const ASSUME_VALID_FLAG: u16 = 0x8000;
const EXTENDED_FLAG_MASK: u16 = 0b0000000111111111;
const MODE_PERMS_MASK: u16 = 0b0000000111111111;

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

        let mut cursor = Cursor::new(&raw);
        let mut buf4 = [0u8; 4];

        cursor.read_exact(&mut buf4)?;
        if &buf4 != b"DIRC" {
            return Err(anyhow!(
                "Signature should be 'DIRC', found {}",
                std::str::from_utf8(&buf4)?
            ));
        }

        cursor.read_exact(&mut buf4)?;
        let version = u32::from_be_bytes(buf4);
        if version != 2 {
            return Err(anyhow!(
                "Only version 2 is supported, version {} found instead",
                version
            ));
        }

        cursor.read_exact(&mut buf4)?;
        let count = u32::from_be_bytes(buf4) as usize;

        let mut entries = vec![];

        for _ in 0..count {
            cursor.read_exact(&mut buf4)?;
            let ctime_s = u32::from_be_bytes(buf4);

            cursor.read_exact(&mut buf4)?;
            let ctime_ns = u32::from_be_bytes(buf4);

            let ctime = (ctime_s, ctime_ns);

            cursor.read_exact(&mut buf4)?;
            let mtime_s = u32::from_be_bytes(buf4);

            cursor.read_exact(&mut buf4)?;
            let mtime_ns = u32::from_be_bytes(buf4);

            let mtime = (mtime_s, mtime_ns);

            cursor.read_exact(&mut buf4)?;
            let device_id = u32::from_be_bytes(buf4);

            cursor.read_exact(&mut buf4)?;
            let inode_number = u32::from_be_bytes(buf4);

            let mut buf2 = [0u8; 2];
            cursor.read_exact(&mut buf2)?;
            let unused = u16::from_be_bytes(buf2);
            if unused != 0 {
                return Err(anyhow!("Field should be unused, found {}", unused));
            }

            cursor.read_exact(&mut buf2)?;
            let mode = u16::from_be_bytes(buf2);
            let mode_type = ModeType::from_bytes((mode >> 12) as u8)?;
            let mode_perms = mode & MODE_PERMS_MASK;

            cursor.read_exact(&mut buf4)?;
            let uid = u32::from_be_bytes(buf4);

            cursor.read_exact(&mut buf4)?;
            let gid = u32::from_be_bytes(buf4);

            cursor.read_exact(&mut buf4)?;
            let fsize = u32::from_be_bytes(buf4);

            let mut sha = [0u8; 20];
            cursor.read_exact(&mut sha)?;
            let sha = encode(sha);

            cursor.read_exact(&mut buf2)?;
            let flags = u16::from_be_bytes(buf2);

            let assume_valid_flag = flags & ASSUME_VALID_FLAG != 0;
            let extended_flag = (flags & EXTENDED_FLAG_MASK) != 0;
            let stage_flag = ((flags & STAGE_FLAG_MASK) >> 12) as u8;
            let name_length = (flags & NAME_LENGTH_MASK) as usize;

            if extended_flag {
                return Err(anyhow!("Extended flag should be disabled"));
            }

            let mut name_bytes = vec![0u8; name_length];
            cursor.read_exact(&mut name_bytes)?;

            let mut null_byte = [0u8; 1];
            cursor.read_exact(&mut null_byte)?;
            if null_byte[0] != 0x00 {
                return Err(anyhow!("Name is not terminated properly"));
            }

            let name = String::from_utf8_lossy(&name_bytes).to_string();

            let pos = cursor.position();
            let padding = (8 - (pos % 8)) % 8;
            cursor.set_position(pos + padding);

            entries.push(DeltaIndexEntry {
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
            });
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
