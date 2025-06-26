pub mod hirc;

use std::io;

use byteorder::{ReadBytesExt, WriteBytesExt, LE};

use hirc::*;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

type Result<T> = std::result::Result<T, BnkError>;

#[derive(Debug, thiserror::Error)]
pub enum BnkError {
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Binrw error: {0}")]
    Binrw(#[from] binrw::Error),

    #[error("Accessing DATA section before DIDX section.")]
    MissingDidx,
    #[error("Unknown HIRC entry type at offset {0}: {0}")]
    UnknownHircEntryType(u64, u8),
    #[error("Unknown SoundType at offset {0}: {0}")]
    UnknownSoundType(u64, u8),
    #[error("Unknown EventActionScope at offset {0}: {0}")]
    UnknownEventActionScope(u64, u8),
    #[error(
        "Incorrect data size for {name}: expected {expected}, got {got}. Section start: {start}"
    )]
    BadDataSize {
        name: String,
        expected: u64,
        got: u64,
        start: u64,
    },
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Bnk {
    pub sections: Vec<Section>,
}

impl Bnk {
    pub fn from_reader<R>(reader: &mut R) -> Result<Self>
    where
        R: io::Read + io::Seek,
    {
        let mut sections = Vec::new();
        loop {
            let mut magic = [0u8; 4];
            if let Err(e) = reader.read_exact(&mut magic) {
                if e.kind() == io::ErrorKind::UnexpectedEof {
                    break;
                }
            };
            // handle data section separately
            let section = if &magic == b"DATA" {
                let total_length = reader.read_u32::<LE>()?;
                let didx_entries = sections
                    .iter()
                    .find_map(|sec: &Section| {
                        if let SectionPayload::Didx { entries } = &sec.payload {
                            Some(entries)
                        } else {
                            None
                        }
                    })
                    .ok_or(BnkError::MissingDidx)?;
                let data_start_pos = reader.stream_position()?;
                let mut data_list = Vec::with_capacity(didx_entries.len());
                for entry in didx_entries {
                    let mut data = vec![0; entry.length as usize];
                    reader.seek(io::SeekFrom::Start(data_start_pos + entry.offset as u64))?;
                    reader.read_exact(&mut data)?;
                    data_list.push(data);
                }
                reader.seek(io::SeekFrom::Start(data_start_pos + total_length as u64))?;
                Section {
                    magic,
                    section_length: total_length,
                    payload: SectionPayload::Data { data_list },
                }
            } else {
                Section::from_reader(reader, magic)?
            };
            sections.push(section);
        }
        Ok(Bnk { sections })
    }

    pub fn write_to<W>(&mut self, writer: &mut W) -> Result<()>
    where
        W: io::Write + io::Seek,
    {
        // fix values
        self.fix_values()?;

        let mut didx_entries: Option<&[DidxEntry]> = None;

        for section in &mut self.sections {
            writer.write_all(&section.magic)?;
            // fix section length
            writer.write_u32::<LE>(0)?;
            let start_pos = writer.stream_position()?;

            match &mut section.payload {
                SectionPayload::Bkhd {
                    version,
                    id,
                    unknown,
                } => {
                    writer.write_u32::<LE>(*version)?;
                    writer.write_u32::<LE>(*id)?;
                    writer.write_all(unknown)?;
                }
                SectionPayload::Didx { entries } => {
                    didx_entries.replace(entries);
                    for entry in entries.iter() {
                        let entry_bytes: [u8; 12] = unsafe { std::mem::transmute(entry.clone()) };
                        writer.write_all(&entry_bytes)?;
                    }
                }
                SectionPayload::Hirc { entries } => {
                    writer.write_u32::<LE>(entries.len() as u32)?;
                    for entry in entries.iter_mut() {
                        entry.write_to(writer)?;
                    }
                }
                SectionPayload::Data { data_list } => {
                    let Some(didx_entries) = didx_entries else {
                        return Err(BnkError::MissingDidx);
                    };
                    let data_start_pos = writer.stream_position()?;
                    for (i, data) in data_list.iter().enumerate() {
                        let entry = &didx_entries[i];
                        writer.seek(io::SeekFrom::Start(data_start_pos + entry.offset as u64))?;
                        writer.write_all(data)?;
                        // Unimplemented feature: 16字节对齐 padding
                    }
                }
                SectionPayload::Unk { data } => {
                    writer.write_all(data)?;
                }
            }

            let end_pos = writer.stream_position()?;
            // write section length
            let length = (end_pos - start_pos) as u32;
            writer.seek(io::SeekFrom::Start(start_pos - 4))?;
            writer.write_u32::<LE>(length)?;
            writer.seek(io::SeekFrom::Start(end_pos))?;
        }
        Ok(())
    }

    fn fix_values(&mut self) -> Result<()> {
        // 查找 DIDX 和 DATA 部分
        let mut didx_section = None;
        let mut data_section = None;

        for section in &mut self.sections {
            match &mut section.payload {
                SectionPayload::Didx { entries } => {
                    didx_section = Some(entries);
                }
                SectionPayload::Data { data_list } => {
                    data_section = Some(data_list);
                }
                _ => {}
            }
        }

        // 确保找到了 DIDX 和 DATA 部分
        let (didx_entries, data_list) = match (didx_section, data_section) {
            (Some(didx), Some(data)) => (didx, data),
            _ => return Ok(()), // 如果没有 DIDX 或 DATA 部分，直接返回
        };

        // 检查 DIDX 条目数量是否与 DATA 列表数量匹配
        if didx_entries.len() != data_list.len() {
            return Err(BnkError::BadDataSize {
                name: "DIDX entries".to_string(),
                expected: data_list.len() as u64,
                got: didx_entries.len() as u64,
                start: 0,
            });
        }

        // 修复偏移和长度值
        let mut current_offset = 0u32;
        for (didx_entry, data) in didx_entries.iter_mut().zip(data_list.iter()) {
            // 更新长度
            didx_entry.length = data.len() as u32;
            // 更新偏移
            didx_entry.offset = current_offset;
            // 计算下一个偏移（当前偏移 + 当前长度）
            current_offset += didx_entry.length;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Section {
    pub magic: [u8; 4],
    pub section_length: u32,
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub payload: SectionPayload,
}

impl Section {
    fn from_reader<R>(reader: &mut R, magic: [u8; 4]) -> Result<Self>
    where
        R: io::Read + io::Seek,
    {
        let section_length = reader.read_u32::<LE>()?;
        let payload = match &magic {
            b"BKHD" => SectionPayload::Bkhd {
                version: reader.read_u32::<LE>()?,
                id: reader.read_u32::<LE>()?,
                unknown: {
                    let mut unknown = vec![0; section_length as usize - 8];
                    reader.read_exact(&mut unknown)?;
                    unknown
                },
            },
            b"DIDX" => {
                let entry_count = (section_length as usize) / size_of::<DidxEntry>();
                let mut entries = Vec::with_capacity(entry_count);
                for _ in 0..entry_count {
                    let mut buf = [0; size_of::<DidxEntry>()];
                    reader.read_exact(&mut buf)?;
                    entries.push(unsafe { std::mem::transmute::<[u8; 12], DidxEntry>(buf) });
                }
                SectionPayload::Didx { entries }
            }
            b"HIRC" => {
                let count = reader.read_u32::<LE>()?;
                let mut entries = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    let entry_type = reader.read_u8()?;
                    let entry_type = HircEntryType::from_repr(entry_type)
                        .unwrap_or(HircEntryType::Unknown(entry_type));
                    let hirc_entry = HircEntry::from_reader(reader, entry_type)?;
                    entries.push(hirc_entry);
                }
                SectionPayload::Hirc { entries }
            }
            b"DATA" => {
                unreachable!("DATA section should be handled separately.");
            }
            _ => {
                let mut data = vec![0; section_length as usize];
                reader.read_exact(&mut data)?;
                SectionPayload::Unk { data }
            }
        };

        Ok(Section {
            magic,
            section_length,
            payload,
        })
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum SectionPayload {
    Bkhd {
        version: u32,
        id: u32,
        unknown: Vec<u8>,
    },
    Didx {
        entries: Vec<DidxEntry>,
    },
    Hirc {
        entries: Vec<HircEntry>,
    },
    Data {
        data_list: Vec<Vec<u8>>,
    },
    Unk {
        data: Vec<u8>,
    },
}

#[repr(C)]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DidxEntry {
    pub id: u32,
    pub offset: u32,
    pub length: u32,
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{self, File},
        io::{self, Seek},
    };

    use crate::rwext::ReadVecExt;

    use super::*;

    const INPUT_HIRC: &str = "test_files/Wp00_Cmn.sbnk.1.X64";
    const INPUT_HIRC_2: &str = "test_files/bgm_resident_ev.sbnk.1.X64";
    const INPUT_DIDX_DATA: &str = "test_files/Wp00_Cmn_m.sbnk.1.X64";

    #[test]
    fn test_hirc() {
        let input = fs::read(INPUT_HIRC).unwrap();
        let mut reader = io::Cursor::new(&input);
        let mut bnk = Bnk::from_reader(&mut reader).unwrap();
        assert_eq!(&bnk.sections[0].magic, b"BKHD");

        let mut output = Vec::new();
        let mut writer = io::Cursor::new(&mut output);
        bnk.write_to(&mut writer).unwrap();
        assert!(input == output);
    }

    #[test]
    fn test_hirc_2() {
        let input = fs::read(INPUT_HIRC_2).unwrap();
        let mut reader = io::Cursor::new(&input);
        let mut bnk = Bnk::from_reader(&mut reader).unwrap();

        let mut output = Vec::new();
        let mut writer = io::Cursor::new(&mut output);
        bnk.write_to(&mut writer).unwrap();
        assert!(input == output);
    }

    #[test]
    fn test_didx_data() {
        let input = fs::read(INPUT_DIDX_DATA).unwrap();
        let mut reader = io::Cursor::new(&input);
        let mut bnk = Bnk::from_reader(&mut reader).unwrap();

        let mut output = Vec::new();
        let mut writer = io::Cursor::new(&mut output);
        bnk.write_to(&mut writer).unwrap();
        assert!(input == output);
    }

    #[test]
    #[ignore]
    fn test_on_all_files() {
        let wwise_path = "E:/dev/MHWs-in-json-1.0/Sound/Wwise";

        for entry in fs::read_dir(wwise_path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let path_str = path.to_str().unwrap();
            if path_str.contains(".sbnk.1.X64") {
                let file = File::open(&path).unwrap();
                let mut reader = io::BufReader::new(file);
                let magic = reader.read_vec_u8(4).unwrap();
                if magic != [b'B', b'K', b'H', b'D'] {
                    continue;
                }
                reader.seek(io::SeekFrom::Start(0)).unwrap();
                eprintln!("Testing {}", path_str);
                // exclude special files
                if ["System.sbnk.1.X64"].contains(&path.file_name().unwrap().to_str().unwrap()) {
                    eprintln!("Skipping special file.");
                    continue;
                }

                if let Err(e) = Bnk::from_reader(&mut reader) {
                    eprintln!("Error on {}", path_str);
                    panic!("{:?}", e);
                };
            }
        }
    }
}
