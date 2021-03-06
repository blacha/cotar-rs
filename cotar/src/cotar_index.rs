use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs::File;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Read;
use std::io::Result as IoResult;
use std::io::Write;
use tar::Archive;
use tar::EntryType;

pub struct CotarIndex {
    pub entries: HashMap<u64, crate::CotarIndexEntry>,
}

pub struct CotarIndexResult {
    /// Packed buffer
    pub vec: Vec<u8>,
    /// Total entries packed
    pub entries: usize,
    /// Max number of records to search to find a record
    pub search_max: usize,
    /// Average amount of records needed to search to find a record in the index
    pub search_avg: f64,
}

impl Default for CotarIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl CotarIndex {
    pub fn new() -> Self {
        CotarIndex {
            entries: HashMap::new(),
        }
    }

    /// Create a CotarIndex from a tar file
    pub fn from_tar(file_name: &str, report_at: usize) -> IoResult<CotarIndex> {
        let file = File::open(file_name)?;
        let mut a = Archive::new(file);

        let mut cotar_index = CotarIndex::new();
        for file in a.entries()? {
            let file = file?;

            let header = file.header();
            let file_path = header.path()?;
            let file_name = file_path.to_str().expect("Failed to extract file_name");

            match header.entry_type() {
                EntryType::Regular => {
                    // offset to the file is at end of the header
                    let file_offset = file.raw_header_position() + 512;
                    // println!("load_entry {} {:?} {:?}", file_offset, header, header.entry_type());
                    let file_size = header.size()? as u32;
                    cotar_index.add(file_name, file_offset, file_size)?;
                }
                EntryType::Link => {
                    let link_path = header.link_name()?.expect("No link_path found?");
                    let link_name = link_path.to_str().expect("Failed to extract link_path");
                    cotar_index.link(file_name, link_name)?;
                }
                _e => {
                    // TODO what to do with these types
                    // Folders/other files, ignore for now??
                    // println!("Unknown entry_type: {:?}", e)
                }
            }

            // If a report is requested dump how far through the file we are.
            if report_at > 0 && cotar_index.entries.len() % report_at == 0 {
                println!("{}", cotar_index.entries.len());
            }
        }
        Ok(cotar_index)
    }

    pub fn add(&mut self, path: &str, file_offset: u64, file_size: u32) -> IoResult<()> {
        let hash = crate::Cotar::hash(path);
        if self.entries.contains_key(&hash) {
            return Err(Error::new(
                ErrorKind::Other,
                format!("Duplicate hash key : {}", path),
            ));
        }

        let entry = crate::CotarIndexEntry {
            hash,
            file_offset,
            file_size,
        };
        self.entries.insert(entry.hash, entry);
        Ok(())
    }

    /// If a file is the exact same as another file in the archive, create a link
    /// rather than storing the file twice
    pub fn link(&mut self, source: &str, target: &str) -> IoResult<()> {
        let hash_target = crate::Cotar::hash(target);
        let entry = self.entries.get(&hash_target);

        match entry {
            None => Err(Error::new(ErrorKind::Other, "Missing link target")),
            Some(e) => {
                let file_size = e.file_size;
                let file_offset = e.file_offset;
                self.add(source, file_offset, file_size)
            }
        }
    }

    /// Pack the COTAR index into a buffer with the specified amount of excess slots
    pub fn pack(&mut self, packing_factor: f64) -> IoResult<CotarIndexResult> {
        let entry_count = self.entries.len();
        // Cannot pack into less than 100% size...
        if packing_factor < 1.0 {
            return Err(std::io::Error::new(
                ErrorKind::Other,
                "Packing factor too low",
            ));
        }
        // Slot count is limited to uint32
        let slot_count = ((entry_count as f64) * packing_factor).floor() as u64;
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

            hash_order = a.file_offset.cmp(&b.file_offset);
            if hash_order != Ordering::Equal {
                return hash_order;
            }

            a.hash.cmp(&b.hash)
        });

        let output: Vec<u8> = Vec::with_capacity(buffer_size as usize);
        let mut cursor = std::io::Cursor::new(output);
        // Write the header
        cursor.write_all(&u32::to_le_bytes(crate::COTAR_V2_HEADER_MAGIC))?;
        cursor.write_all(&u32::to_le_bytes(slot_count as u32))?;

        // Write the footer
        cursor.set_position(buffer_size - crate::COTAR_V2_HEADER_SIZE);
        cursor.write_all(&u32::to_le_bytes(crate::COTAR_V2_HEADER_MAGIC))?;
        cursor.write_all(&u32::to_le_bytes(slot_count as u32))?;

        let mut max_search_count: usize = 0;
        let mut total_search_count: usize = 0;
        for entry in all_values {
            let mut search_count: usize = 0;
            let mut index = (entry.hash % slot_count) as u64;
            let start_index = index;

            loop {
                // Loop back to the start if we go past the end of the file
                if index >= slot_count {
                    index = 0;
                }
                let offset = crate::COTAR_V2_INDEX_ENTRY_SIZE * index + crate::COTAR_V2_HEADER_SIZE;

                let mut hash_buf = [0; 8];
                cursor.set_position(offset);
                cursor.read_exact(&mut hash_buf)?;

                //  empty slot found
                if u64::from_le_bytes(hash_buf) == 0 {
                    // Seek back to where the entry should be written
                    cursor.set_position(offset);
                    break;
                }

                search_count += 1;
                index += 1;
                // If the index loops all the way around to the start something horrible has happened
                if index == start_index {
                    return Err(std::io::Error::new(ErrorKind::Other, "Hash index looped"));
                }
            }

            if search_count > max_search_count {
                max_search_count = search_count;
            }
            total_search_count += search_count;

            // Tar files are aligned to 512 byte blocks store the block offset not the file offset
            let file_block_offset = (entry.file_offset / 512) as u32;
            cursor.write_all(&u64::to_le_bytes(entry.hash))?;
            cursor.write_all(&u32::to_le_bytes(file_block_offset))?;
            cursor.write_all(&u32::to_le_bytes(entry.file_size))?;
        }

        Ok(CotarIndexResult {
            vec: cursor.into_inner(),
            entries: entry_count,
            search_max: max_search_count,
            search_avg: (total_search_count as f64) / (entry_count as f64),
        })
    }
}
