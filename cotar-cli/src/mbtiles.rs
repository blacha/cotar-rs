use rusqlite::Connection;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Error, ErrorKind, Result as IoResult};
use std::time::SystemTime;
use tar::{Builder, EntryType, Header};

use crate::file_index_create;

/// Check if the buffer looks like a gziped buffer
fn is_gzip(buf: &[u8]) -> bool {
    return buf.len() > 2 && buf[0] == 31 && buf[1] == 139;
}

#[derive(Debug)]
struct Tile {
    x: u32,
    y: u32,
    z: u32,
    data: Vec<u8>,
}

struct SqliteMetadata {
    name: String,
    value: String,
}

fn open_mbtiles(file_name: &str) -> IoResult<Connection> {
    let conn = Connection::open(file_name);
    match conn {
        Err(_e) => Err(Error::new(ErrorKind::Other, "Failed to open ")),
        Ok(c) => Ok(c),
    }
}

struct TileHashTree {
    data: HashMap<usize, HashMap<u64, String>>,
}

impl TileHashTree {
    pub fn new() -> Self {
        TileHashTree {
            data: HashMap::with_capacity(1 << 16),
        }
    }

    pub fn insert(&mut self, file_size: usize, hash: u64, path: &str) -> Option<String> {
        let hm = self.data.entry(file_size).or_insert_with(HashMap::new);
        let entry = hm.entry(hash);
        match entry {
            Entry::Vacant(e) => {
                e.insert(path.to_string());
                None
            }
            Entry::Occupied(e) => Some(e.get().to_owned()),
        }
    }

    pub fn len(&mut self) -> usize {
        let mut count = 0;
        for map in self.data.values() {
            count += map.len();
        }
        count
    }
}

pub fn to_tar(
    file_name: &str,
    output_file: &str,
    deduplicate: bool,
    drop_duplicates: bool,
    create_index: bool,
) -> IoResult<()> {
    if !file_name.ends_with(".mbtiles") {
        return Err(Error::new(
            ErrorKind::Other,
            format!("\"{}\" does not end with .mbtiles", file_name),
        ));
    }

    if !output_file.ends_with(".tar") {
        return Err(Error::new(
            ErrorKind::Other,
            format!("\"{}\" does not end with .tar", output_file),
        ));
    }

    let conn = open_mbtiles(file_name)?;
    let mut tht = TileHashTree::new();

    // TODO this is really slow for huge archives
    let tile_count: f64 = conn
        .query_row("SELECT count(*) from tiles", [], |row| row.get(0))
        .expect("Failed to query tile_count");

    // How often to report progress
    let mut progress_count = 50_000;
    if tile_count < 1_000_000.0 {
        progress_count = (tile_count / 20.0).round() as usize
    }

    println!(
        "MBtiles opened, tiles:{:.0} deduplicate:{} drop_duplicates:{}",
        tile_count, deduplicate, drop_duplicates
    );

    if deduplicate == false && drop_duplicates {
        return Err(Error::new(
            ErrorKind::Other,
            "Cannot use deduplicate:false and drop_duplicates:true",
        ));
    }

    // TODO would be good to create the index at the same time, It doesn't seem easy to get where the header was written too though :(
    let file_writer = File::create(output_file)?;
    let mut tb = Builder::new(file_writer);

    let mut stmt = conn
        .prepare("SELECT zoom_level,tile_column,tile_row,tile_data from tiles")
        .expect("Failed to query tiles from sqlite");

    let mut metadata = conn
        .prepare("SELECT name,value from metadata")
        .expect("Failed to query metedata from sqlite");

    let metadata_rows = metadata
        .query_map([], |row| {
            Ok(SqliteMetadata {
                name: row.get(0)?,
                value: row.get(1)?,
            })
        })
        .expect("Failed to extract metadata");

    let mut format = String::from("pbf");
    for meta_row in metadata_rows {
        let row = meta_row.unwrap();
        if row.name == "format" {
            format = row.value.to_string();
            println!("Metadata format: {}", format);
        }
    }

    let tile_iter = stmt
        .query_map([], |row| {
            Ok(Tile {
                z: row.get(0)?,
                x: row.get(1)?,
                y: row.get(2)?,
                data: row.get(3)?,
            })
        })
        .expect("Failed to extract tile");

    let mut count: usize = 0;
    let mut duplicates: usize = 0;
    let start = SystemTime::now();
    let mut current: SystemTime = start;

    for tile_r in tile_iter {
        let tile = tile_r.unwrap();
        // Flip Y coordinate because MBTiles files are stored against a tile matrix set.
        let y = (1 << tile.z) - 1 - tile.y;

        // Tar archives have 100 bytes for a path_name so this needs to be < 100 bytes long
        let mut file_name = format!("tiles/{}/{}/{}.", tile.z, tile.x, y);
        file_name.push_str(format.as_str());
        if is_gzip(&tile.data) {
            file_name.push_str(".gz")
        }

        // Hash the files and de-duplicate them in the tar using links
        if deduplicate {
            let file_hash = cotar::fnv1a_64(&tile.data);
            // does this file hash already exist in the output tar
            match tht.insert(tile.data.len(), file_hash, &file_name) {
                None => {
                    let mut header = Header::new_gnu();
                    header.set_size(tile.data.len() as u64);
                    tb.append_data(&mut header, &file_name, tile.data.as_slice())
                        .expect("Failed to write file");
                }
                Some(p) => {
                    if drop_duplicates {
                        // Drop them from the output tar
                        // TODO this should be recorded somewhere
                        duplicates = duplicates + 1;
                    } else {
                        // Duplicate entry store it as a link to the previous entry
                        let mut header = Header::new_gnu();
                        header.set_size(0);
                        header.set_entry_type(EntryType::Link);
                        tb.append_link(&mut header, &file_name, p)
                            .expect("Failed to insert");
                    }
                }
            }
        } else {
            let mut header = Header::new_gnu();
            header.set_size(tile.data.len() as u64);
            tb.append_data(&mut header, &file_name, tile.data.as_slice())
                .expect("Failed to write file");
        }

        if count == 0 {
            println!(
                "{:>10} {:>6.2}% {:>8} {:>32} {}",
                "count", "", "unique_files", "last_path", "duration"
            );
        }
        count += 1;

        if count % progress_count == 0 {
            let uniques = match tht.len() {
                0 => count,
                len => len,
            };

            let now = SystemTime::now();
            println!(
                "{:>10} {:>6.1}% {:>8} {:>32} {:.2?}",
                count,
                (count as f64 / tile_count) * 100.0,
                uniques,
                file_name,
                now.duration_since(current).unwrap(),
            );
            current = SystemTime::now();
        }
    }

    tb.finish().expect("Failed to write tar");

    println!(
        "\n✔️ Tar created: {} from mbtiles entries:{} unique_files:{}\n",
        output_file,
        count,
        tht.len(),
    );

    if create_index {
        file_index_create(output_file, true, 50);
        println!("✔️ Tar index created: {}.index", output_file);
    }

    Ok(())
}
