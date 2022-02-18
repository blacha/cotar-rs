use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs::File;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Read;
use std::io::Result as IoResult;
use std::io::Write;
use tar::Archive;

pub struct CotarIndex {
    pub entries: HashMap<u64, crate::CotarIndexEntry>,
}

pub struct CotarIndexResult {
    pub vec: Vec<u8>,
    pub max_search: usize,
}

impl CotarIndex {
    pub fn new() -> Self {
        CotarIndex {
            entries: HashMap::new(),
        }
    }

    pub fn from_tar(file_name: &str) -> IoResult<CotarIndex> {
        let file = File::open(file_name)?;
        let mut a = Archive::new(file);

        let mut cotar_index = CotarIndex::new();
        for file in a.entries()? {
            let file = file?;

            let header = file.header();
            let header_offset = file.raw_header_position() + 512;

            if let Some(file_name) = file.header().path()?.to_str() {
                let file_size = header.size()? as u32;
                if !cotar_index.add(file_name, header_offset, file_size) {
                    return Err(Error::new(
                        ErrorKind::Other,
                        format!("Failed to insert {}", file_name),
                    ));
                    // println!("Failed insert");
                }

                if cotar_index.entries.len() % 100_000 == 0 {
                    println!("{}", cotar_index.entries.len());
                }
            }
        }
        println!("{} Done", cotar_index.entries.len());
        Ok(cotar_index)
    }

    pub fn add(&mut self, path: &str, header_offset: u64, size: u32) -> bool {
        let hash = crate::Cotar::hash(path);
        if self.entries.contains_key(&hash) {
            return false;
        }

        let entry = crate::CotarIndexEntry {
            hash,
            offset: header_offset / 512,
            size,
        };
        self.entries.insert(entry.hash, entry);
        return true;
    }

    pub fn pack(&mut self, packing_factor: f64) -> IoResult<CotarIndexResult> {
        let entry_count = self.entries.len();
        let slot_count = ((entry_count as f64) * packing_factor).ceil() as u64;
        if slot_count >= (u32::MAX as u64) {
            return Err(std::io::Error::new(
                ErrorKind::Other,
                "Too many index entries",
            ));
        }

        let buffer_size =
            crate::COTAR_V2_HEADER_SIZE * 2 + crate::COTAR_V2_INDEX_ENTRY_SIZE * slot_count;

        let mut all_values: Vec<&crate::CotarIndexEntry> = self.entries.values().collect();
        // Sort the entries into the order they should be written to the file
        all_values.sort_by(|&a, &b| {
            let mut hash_order = (a.hash % slot_count).cmp(&(b.hash % slot_count));
            if hash_order != Ordering::Equal {
                return hash_order;
            }

            hash_order = a.offset.cmp(&b.offset);
            if hash_order != Ordering::Equal {
                return hash_order;
            }

            return a.hash.cmp(&b.hash);
        });

        let output: Vec<u8> = Vec::with_capacity(buffer_size as usize);
        let mut cursor = std::io::Cursor::new(output);
        // Write the header
        cursor.write(&u32::to_le_bytes(crate::COTAR_V2_HEADER_MAGIC))?;
        cursor.write(&u32::to_le_bytes(slot_count as u32))?;

        // Write the footer
        cursor.set_position(buffer_size - crate::COTAR_V2_HEADER_SIZE);
        cursor.write(&u32::to_le_bytes(crate::COTAR_V2_HEADER_MAGIC))?;
        cursor.write(&u32::to_le_bytes(slot_count as u32))?;

        let mut max_search_count: usize = 0;
        for entry in all_values {
            let mut search_count: usize = 0;
            let mut index = (entry.hash % slot_count) as u64;
            let start_index = index;

            loop {
                // Loop back to the start if we go around the file
                if index >= slot_count {
                    index = 0;
                }
                let offset = crate::COTAR_V2_INDEX_ENTRY_SIZE * index + crate::COTAR_V2_HEADER_SIZE;

                let mut hash_buf = [0; 8];
                cursor.set_position(offset);
                cursor.read(&mut hash_buf)?;

                //  empty slot found
                if u64::from_le_bytes(hash_buf) == 0 {
                    // Seek back to where the entry should be written
                    cursor.set_position(offset);
                    break;
                }

                search_count = search_count + 1;
                index = index + 1;
                if index == start_index {
                    return Err(std::io::Error::new(ErrorKind::Other, "Hash index looped"));
                }
            }

            if search_count > max_search_count {
                max_search_count = search_count;
            }

            // println!("Write {} at {}", entry.hash, cursor.position());
            let block_offset = (entry.offset / 512) as u32;
            cursor.write(&u64::to_le_bytes(entry.hash))?;
            cursor.write(&u32::to_le_bytes(block_offset))?;
            cursor.write(&u32::to_le_bytes(entry.size))?;
        }

        return Ok(CotarIndexResult {
            vec: cursor.into_inner(),
            max_search: max_search_count,
        });
    }
}