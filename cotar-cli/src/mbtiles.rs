use rusqlite::Connection;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Error, ErrorKind, Result as IoResult};
use std::time::SystemTime;
use tar::{Builder, EntryType, Header};

#[derive(Debug)]
struct Tile {
    x: u32,
    y: u32,
    z: u32,
    data: Vec<u8>,
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

pub fn to_tar(file_name: &str, drop_duplicates: bool) -> IoResult<()> {
    if !file_name.ends_with(".mbtiles") {
        return Err(Error::new(
            ErrorKind::Other,
            format!("\"{}\" does not end with .mbtiles", file_name),
        ));
    }

    let conn = open_mbtiles(file_name)?;
    let mut tht = TileHashTree::new();

    // TODO this is really slow for huge archives
    let tile_count: f64 = conn
        .query_row("SELECT count(*) from tiles", [], |row| row.get(0))
        .expect("Failed to query tile_count");

    println!(
        "MBtiles opened, tiles:{:.0} drop_duplicates:{}",
        tile_count, drop_duplicates
    );

    // TODO would be good to create the index at the same time, It doesnt seem easy to get where the header was written too though :(
    let file_writer = File::create(format!("{}.tar", file_name))?;
    let mut tb = Builder::new(file_writer);

    let mut stmt = conn
        .prepare("SELECT * from tiles")
        .expect("Failed to query tiles from sqlite");

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
    let start = SystemTime::now();
    let mut current = start;

    for tile_r in tile_iter {
        let tile = tile_r.unwrap();
        // Flip Y coordinate because MBTiles files are stored against a tile matrix set.
        let y = (1 << tile.z) - 1 - tile.y;

        // TODO is storing these as quadkeys the best option? could just use z/x/y.pbf
        let qk = quadkey::tile_to_str(
            tile.x as usize,
            y.try_into().expect("Failed to convert y into u32"),
            tile.z as usize,
        );

        // Tar archives have 100 bytes for a path_name so this needs to be < 100 bytes long
        let file_name = format!("tiles/{}/{}.gz", tile.z, qk);
        // TODO should these files be uncompressed?

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
                } else {
                    // Duplicate entry store it as a link to the previous entry
                    let mut header = Header::new_gnu();
                    header.set_size(0);
                    header.set_entry_type(EntryType::Link);
                    tb.append_link(&mut header, file_name, p)
                        .expect("Failed to insert");
                }
            }
        }
        count += 1;

        if count % 250_000 == 0 {
            let now = SystemTime::now();
            println!(
                "{:>10} {:>5.2}% {:>8}  {:>32} {:.2?}",
                count,
                (count as f64 / tile_count) * 100.0,
                tht.len(),
                qk,
                now.duration_since(current).unwrap(),
            );
            current = SystemTime::now();
        }
    }

    tb.finish().expect("Failed to write tar");

    println!(
        "Mbtiles converted entries:{} unique_files:{}",
        count,
        tht.len(),
    );

    Ok(())
}
