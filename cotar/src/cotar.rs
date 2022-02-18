use std::io::Result as IoResult;
use std::io::{Error, ErrorKind};

use crate::dataview;
use crate::fnv1a;

/// V1 cotar files have a fixed header and entry size
pub const COTAR_V2_HEADER_SIZE: u64 = 8;
pub const COTAR_V2_INDEX_ENTRY_SIZE: u64 = 16;

/// "COT\x02" as a u32
pub const COTAR_V2_HEADER_MAGIC: u32 = 39079747;

#[derive(Debug)] // TODO None of these need to be 64bits
pub struct CotarIndexEntry {
    pub hash: u64,
    pub offset: u64,
    pub size: u32,
}

#[derive(Debug)]
pub struct Cotar {
    pub version: u8,
    pub entries: u64,
    pub index_offset: u64,
    pub view: dataview::DataView,
}

impl Cotar {
    pub fn from_tar(file_name: &str) -> IoResult<Self> {
        let mut view = dataview::DataView::open(file_name)?;

        let magic = view.u32_le(view.size - 8)?;
        // "COT\x02" as a u32
        if magic != COTAR_V2_HEADER_MAGIC {
            return Err(Error::new(ErrorKind::Other, "Invalid magic"));
        }

        let version = view.byte(view.size - 5)?;
        let entries = view.u32_le(view.size - 4)? as u64;

        let index_offset = view.size - 16 - entries * COTAR_V2_INDEX_ENTRY_SIZE;

        Ok(Cotar {
            version,
            entries,
            index_offset,
            view,
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
    pub fn get(&mut self, path: &str) -> IoResult<Option<Vec<u8>>> {
        let info = self.info(path)?;

        match info {
            None => return Ok(None),
            Some(entry) => {
                let bytes = self.view.bytes(entry.offset, entry.size as u64)?;
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

        loop {
            let offset =
                self.index_offset + index * COTAR_V2_INDEX_ENTRY_SIZE + COTAR_V2_HEADER_SIZE;

            let start_hash = self.view.u64_le(offset)?;
            // Null entry file is missing
            if start_hash == 0 {
                return Ok(None);
            }

            if start_hash == hash {
                return Ok(Some(CotarIndexEntry {
                    hash,
                    offset: (self.view.u32_le(offset + 8)? as u64) * 512,
                    size: self.view.u32_le(offset + 16)?,
                }));
            }

            index = index + 1;
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
