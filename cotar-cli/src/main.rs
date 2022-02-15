use clap::{AppSettings, Parser, Subcommand};
use cotar::{Cotar, CotarIndexEntry};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::process;
use tar::Archive;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
#[clap(global_setting(AppSettings::PropagateVersion))]
#[clap(global_setting(AppSettings::UseLongFormatForHelpSubcommand))]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Info { file_name: String },
    Index { file_name: String },
}

fn file_info(file_name: &String) {
    println!("Reading cotar {}", file_name);

    let ct_open = Cotar::from_tar(file_name.as_str());

    match ct_open {
        Err(e) => {
            println!("{} Failed ❌\n {:?}", file_name, e);
            process::exit(1);
        }
        Ok(ct) => {
            println!("{} Opened ✔️\n\tFiles: {}", file_name, ct.entries,);
        }
    }
}

fn file_index_create(file_name: &String) {
    if !file_name.ends_with(".tar") {
        println!("❌ {} does not end with .tar", file_name);
        process::exit(1);
    }

    let file = File::open(file_name).unwrap();
    let mut a = Archive::new(file);

    let mut index: HashMap<u64, CotarIndexEntry> = HashMap::new();
    let mut count = 0 as usize;
    for file in a.entries().unwrap() {
        count = count + 1;
        let file = file.unwrap();

        let header = file.header();
        let header_offset = file.raw_header_position() + 512;

        if let Some(file_name) = file.header().path().unwrap().to_str() {
            let entry = CotarIndexEntry {
                hash: Cotar::hash(file_name),
                offset: header_offset,
                size: header.size().unwrap(),
            };

            if count % 100_000 == 0 {
                let current = count / 100_000;
                println!(
                    "{}00,000 - {} - {:?} - {:?} - {:?}",
                    current, file_name, entry.offset, entry.size, entry.hash,
                );
            }
            if index.contains_key(&entry.hash) {
                println!("❌ Hash Collision : {}", entry.hash);
                process::exit(1);
            }
            index.insert(entry.hash, entry);
        }
    }

    let slot_count = ((index.len() as f64) * 1.25).ceil() as u64;
    if slot_count >= (u32::MAX as u64) {
        println!("❌ too many entries : {}", slot_count);
        process::exit(1);
    }

    let mut all_values: Vec<&CotarIndexEntry> = index.values().collect();

    // TODO handle file exists
    let mut output_file = File::create("output.tar.index").expect("Failed to create output file");

    let mut header_buf = [0 as u8; 8];
    let mut buf = header_buf.as_mut();
    // "COT\x01" as u32
    buf.write(&u32::to_le_bytes(cotar::COTAR_V1_HEADER_MAGIC))
        .unwrap();
    buf.write(&u32::to_le_bytes(slot_count as u32)).unwrap();
    output_file.write(&mut header_buf).unwrap();

    // Sort the entries into the order they should be written to the file
    all_values.sort_by(|&a, &b| {
        let hash_order = (a.hash % slot_count).cmp(&(b.hash % slot_count));
        match hash_order {
            Ordering::Equal => a.offset.cmp(&b.offset),
            _ => hash_order,
        }
    });

    let mut current_slot = 0 as u64;
    let mut search_count = 0 as u64;
    let mut max_search_count = 0 as u64;
    for entry in all_values {
        let mut empty_entry_buf: [u8; 24] = [0; 24];

        let expected_slot = entry.hash % slot_count;
        search_count = search_count + 1;
        if search_count > max_search_count {
            max_search_count = search_count;
        }
        if expected_slot == current_slot {
            search_count = 0;
        }
        while expected_slot > current_slot {
            search_count = 0;
            output_file.write(&mut empty_entry_buf).unwrap();
            current_slot = current_slot + 1;
        }

        let mut as_mut = empty_entry_buf.as_mut();
        as_mut.write(&u64::to_le_bytes(entry.hash)).unwrap();
        as_mut.write(&u64::to_le_bytes(entry.offset)).unwrap();
        as_mut.write(&u64::to_le_bytes(entry.size)).unwrap();

        output_file.write(&mut empty_entry_buf).unwrap();
        current_slot = current_slot + 1;

        if current_slot % 100_000 == 0 {
            println!("Slot: {} {} ", current_slot, expected_slot);
        }
        if current_slot > slot_count {
            println!("❌ Too Many slots {} {}", current_slot, expected_slot);
            process::exit(1);
            // FIXME handle :this:
        }
    }

    output_file.write(&mut header_buf).unwrap();
    output_file.flush().unwrap();

    println!(
        "Index created\n Files: {}\n Max Search: {}",
        index.len(),
        max_search_count
    );
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Info { file_name } => file_info(file_name),
        Commands::Index { file_name } => file_index_create(file_name),
    }
}
