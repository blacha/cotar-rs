use std::io::Result as IoResult;
use std::io::{Error, ErrorKind};

use bytes::{Buf, Bytes};

use crate::dataview;
use crate::fnv1a;

/// V2 cotar files have a fixed header and entry size
pub const COTAR_V2_HEADER_SIZE: u64 = 8;
pub const COTAR_V2_INDEX_ENTRY_SIZE: u64 = 16;

/// "COT\x02" as a u32
pub const COTAR_V2_HEADER_MAGIC: u32 = 39079747;

#[derive(Debug)] // TODO None of these need to be 64bits
pub struct CotarIndexEntry {
    /// FNV1A Hash of the file_name
    pub hash: u64,
    /// File offset
    pub file_offset: u64,
    /// File size, TODO this isn't fully needed as the file_size is at file_offset - some value
    pub file_size: u32,
}

#[derive(Debug)]
pub struct Cotar {
    /// Cotar index version generally v2
    pub version: u8,
    /// Number of entries in the tar archive
    pub entries: u64,
    /// Offset in the view for the index
    pub index_offset: u64,
    /// View of the
    pub view: dataview::DataView,
    /// Index reference if the index is a seperate file
    pub view_index: Option<dataview::DataView>,
}

pub struct CotarHeader {
    /// Cotar magic generally "COT\x02"
    pub magic: u32,
    /// Cotar index version generally v2
    pub version: u8,
    /// Number of entries in the tar archive
    pub entries: u32,
}

impl Cotar {
    pub fn header_from_bytes(mut header_bytes: Bytes) -> IoResult<CotarHeader> {
        if header_bytes.len() < COTAR_V2_HEADER_SIZE as usize {
            return Err(Error::new(ErrorKind::Other, "Invalid header length"));
        }
        let magic = header_bytes.get_u32_le();
        if magic != COTAR_V2_HEADER_MAGIC {
            return Err(Error::new(ErrorKind::Other, "Invalid magic"));
        }

        let entries = header_bytes.get_u32_le();

        Ok(CotarHeader {
            magic,
            version: 2,
            entries,
        })
    }

    /// Load a cotar from a packed tar file
    ///
    /// The index of the tar must be the final bytes of the tar file
    pub fn from_tar(file_name: &str) -> IoResult<Self> {
        let mut view = dataview::DataView::open(file_name)?;
        let header_bytes =
            view.read_exact(view.size - COTAR_V2_HEADER_SIZE, COTAR_V2_HEADER_SIZE)?;

        let header = Cotar::header_from_bytes(header_bytes)?;

        let index_offset = view.size - 16 - (header.entries as u64) * COTAR_V2_INDEX_ENTRY_SIZE;

        Ok(Cotar {
            version: header.version,
            entries: header.entries as u64,
            index_offset,
            view,
            view_index: None,
        })
    }

    /// Load a cotar from a tar and index file
    pub fn from_tar_index(tar_file_name: &str, index_file_name: &str) -> IoResult<Self> {
        let mut view_index = dataview::DataView::open(index_file_name)?;

        let header_bytes = view_index.read_exact(0, COTAR_V2_INDEX_ENTRY_SIZE)?;
        let header = Cotar::header_from_bytes(header_bytes)?;

        let index_offset = 0;

        Ok(Cotar {
            version: header.version,
            entries: header.entries as u64,
            index_offset,
            view_index: Some(view_index),
            view: dataview::DataView::open(tar_file_name)?,
        })
    }

    /// Generate a hash of a file path
    ///
    /// Uses fnv1a by default
    pub fn hash(path: &str) -> u64 {
        return fnv1a::fnv1a_64(path.as_bytes());
    }

    /// Read the raw bytes of a file from the tar archive
    ///
    /// Returns None if the file is not found
    pub fn get(&mut self, path: &str) -> IoResult<Option<Bytes>> {
        let info = self.info(path)?;

        match info {
            None => Ok(None),
            Some(entry) => {
                let bytes = self
                    .view
                    .read_exact(entry.file_offset, entry.file_size as u64)?;
                Ok(Some(bytes))
            }
        }
    }
    /// Read the metadata entry for a file path
    ///
    /// Returns None if file is not found
    pub fn info(&mut self, path: &str) -> IoResult<Option<CotarIndexEntry>> {
        let hash = Cotar::hash(path);

        let entries = self.entries as u64;
        let start_index = hash % entries;
        let mut index = start_index;

        let view_index = self.view_index.as_mut().unwrap_or(&mut self.view);

        loop {
            let offset =
                self.index_offset + index * COTAR_V2_INDEX_ENTRY_SIZE + COTAR_V2_HEADER_SIZE;

            let mut bytes = view_index.read_exact(offset, COTAR_V2_INDEX_ENTRY_SIZE)?;
            let start_hash = bytes.get_u64_le();
            // Null entry file is missing
            if start_hash == 0 {
                return Ok(None);
            }

            if start_hash == hash {
                return Ok(Some(CotarIndexEntry {
                    hash,
                    file_offset: (bytes.get_u32_le() as u64) * 512,
                    file_size: bytes.get_u32_le(),
                }));
            }

            index += 1;
            // Loop around to the start of the hash table
            if index >= entries {
                index = 0;
            }
            // Looped full around nothing to find here
            if index == start_index {
                return Ok(None);
            }
        }
    }
}

#[test]

fn test_header() {
    let buf = Bytes::from(vec![0x43, 0x4f, 0x54, 0x02, 0x1c, 0x00, 0x00, 0x00, 0x28]);
    let header = Cotar::header_from_bytes(buf).unwrap();

    assert_eq!(header.magic, COTAR_V2_HEADER_MAGIC);
    assert_eq!(header.entries, 28);
    // assert_eq!(header.version, 2);
}

#[test]
fn test_header_invalid_version() {
    // Set version to 0x01
    let buf = Bytes::from(vec![0x43, 0x4f, 0x54, 0x01, 0x1c, 0x00, 0x00, 0x00, 0x28]);
    let header = Cotar::header_from_bytes(buf);

    assert_eq!(header.is_ok(), false)
}
#[test]
fn test_header_invalid_magic() {
    // Set magic to "AOT\x02"
    let buf = Bytes::from(vec![0x41, 0x4f, 0x54, 0x02, 0x1c, 0x00, 0x00, 0x00, 0x28]);
    let header = Cotar::header_from_bytes(buf);

    assert_eq!(header.is_ok(), false)
}
