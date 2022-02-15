use clap::{AppSettings, Parser, Subcommand};
use cotar::{Cotar, CotarIndexEntry};
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
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
        }
        Ok(ct) => {
            println!("{} Opened ✔️\n\tFiles: {}", file_name, ct.entries,);
        }
    }
}

fn file_index_create(file_name: &String) {
    if !file_name.ends_with(".tar") {
        println!("❌ {} does not end with .tar", file_name);
        return;
    }

    let file = File::open(file_name).unwrap();
    let mut a = Archive::new(file);

    let index: HashMap<u64, CotarIndexEntry> = HashMap::new();
    let mut count = 0 as usize;
    for file in a.entries().unwrap() {
        let mut file = file.unwrap();

        let header = file.header();
        // TODO how to convert this to a &str
        let file_name = file.header().path_bytes();
        let header_offset = file.raw_header_position();

        let entry = CotarIndexEntry {
            hash: 0, // Cotar::hash(file_name), // TODO how to get this to a str
            offset: header_offset + 512,
            size: header.size().unwrap(),
        };

        // println!("{} {} {} {}", file_name, entry.offset, entry.hash, entry.size);

        // let hash:String = file.header().path().unwrap().as_os_str();
        if count % 10_000 == 0 {
            println!(
                "{} - {:?} - {:?}",
                count,
                file.header().path().unwrap(),
                header_offset
            );
        }

        count = count + 1;
    }
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Info { file_name } => file_info(file_name),
        Commands::Index { file_name } => file_index_create(file_name),
    }
}
