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

impl Cotar {
    /// Load a cotar from a packed tar file
    ///
    /// The index of the tar must be the final bytes of the tar file
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
            view_index: None,
        })
    }

    /// Load a cotar from a tar and index file
    pub fn from_tar_index(tar_file_name: &str, index_file_name: &str) -> IoResult<Self> {
        let mut view_index = dataview::DataView::open(index_file_name)?;

        let magic = view_index.u32_le(0)?;
        // "COT\x02" as a u32
        if magic != COTAR_V2_HEADER_MAGIC {
            return Err(Error::new(ErrorKind::Other, "Invalid magic"));
        }

        let version = view_index.byte(3)?;
        let entries = view_index.u32_le(4)? as u64;

        let index_offset = 0;

        Ok(Cotar {
            version,
            entries,
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
    pub fn get(&mut self, path: &str) -> IoResult<Option<Vec<u8>>> {
        let info = self.info(path)?;

        match info {
            None => Ok(None),
            Some(entry) => {
                let bytes = self.view.bytes(entry.file_offset, entry.file_size as u64)?;
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

            let start_hash = view_index.u64_le(offset)?;
            // Null entry file is missing
            if start_hash == 0 {
                return Ok(None);
            }

            if start_hash == hash {
                return Ok(Some(CotarIndexEntry {
                    hash,
                    file_offset: (view_index.u32_le(offset + 8)? as u64) * 512,
                    file_size: view_index.u32_le(offset + 12)?,
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
