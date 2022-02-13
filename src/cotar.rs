#[path = "dataview.rs"] // TODO why do these files need a named import
mod dataview;
#[path = "fnv1a.rs"]
mod fnv1a;

use std::io::{Error, ErrorKind};

const COTAR_HEADER_SIZE: u64 = 8;
const COTAR_INDEX_ENTRY_SIZE: u64 = 24; // TODO shrink the index entry size as its mostly wasted space

#[derive(Debug)] // TODO None of these need to be 64bits
pub struct CotarIndexEntry {
    pub hash: u64,
    pub offset: u64,
    pub size: u64,
}

#[derive(Debug)]
pub struct Cotar {
    pub version: u8,
    pub entries: u64,
    pub index_offset: u64,
    pub view: dataview::DataView,
}

impl Cotar {
    pub fn from_tar(file_name: &str) -> Result<Self, std::io::Error> {
        let mut view = dataview::DataView::open(file_name)?;

        let magic = view.u32_le(view.size - 8)?;
        // "COT\x01" as a u32
        if magic != 22302531 {
            return Err(Error::new(ErrorKind::Other, "oh no!"));
        }

        let version = view.byte(view.size - 5)?;
        let entries = view.u32_le(view.size - 4)? as u64;

        let index_offset = view.size - 16 - entries * COTAR_INDEX_ENTRY_SIZE;

        Ok(Cotar {
            version,
            entries,
            index_offset,
            view,
        })
    }

    pub fn hash(path: &str) -> u64 {
        return fnv1a::fnv1a_64(path.as_bytes());
    }

    pub fn get(&mut self, path: &str) -> Result<Option<Vec<u8>>, std::io::Error> {
        let info = self.info(path)?;

        match info {
            None => return Ok(None),
            Some(entry) => {
                let bytes = self.view.bytes(entry.offset, entry.size)?;
                return Ok(Some(bytes));
            }
        }
    }

    pub fn info(&mut self, path: &str) -> Result<Option<CotarIndexEntry>, std::io::Error> {
        let hash = Cotar::hash(path);

        let start_index = hash % self.entries;
        let mut index = start_index;

        loop {
            let offset = self.index_offset + index * COTAR_INDEX_ENTRY_SIZE + COTAR_HEADER_SIZE;

            let start_hash = self.view.u64_le(offset)?;
            // Null entry file is missing
            if start_hash == 0 {
                return Ok(None);
            }

            if start_hash == hash {
                return Ok(Some(CotarIndexEntry {
                    hash,
                    offset: self.view.u64_le(offset + 8)?,
                    size: self.view.u64_le(offset + 16)?,
                }));
            }

            index = index + 1;
            // Loop around to the start of the hash table
            if index >= self.entries {
                index = 0;
            }
            // Looped full around nothing to find here
            if index == start_index {
                return Ok(None);
            }
        }
    }
}
