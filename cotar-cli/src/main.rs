use clap::{Parser, Subcommand};
use cotar::CotarIndex;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process;
use std::time::Instant;

mod mbtiles;
mod validate;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a cotar from a tar file
    Create {
        /// Tar file to index
        file_name: String,

        /// Overwrite any existing files
        ///
        /// Default: false
        #[clap(long, short = 'f')]
        force: Option<bool>,

        /// Worst case number of records that need to be searched to find a specific file
        ///
        /// - 25 slots: "-m 25"
        /// - 5 slots: "-m 5"
        ///
        /// Default: 100
        #[clap(short = 'm', long)]
        max_search: Option<usize>,
    },

    /// Create a tar index for a tar
    Index {
        /// Tar file to index
        file_name: String,

        /// Overwrite any existing files
        ///
        /// Default: false
        #[clap(long, short = 'f')]
        force: Option<bool>,

        /// Worst case number of records that need to be searched to find a specific file
        ///
        /// - 25 slots: "-m 25"
        /// - 5 slots: "-m 5"
        ///
        /// Default: 100
        #[clap(short = 'm', long)]
        max_search: Option<usize>,
    },

    /// Validate tar index
    Validate {
        /// Tar file name
        file_name: String,

        /// Optional external index file name
        index_file_name: Option<String>,
    },

    /// Create a tar from a mbtiles archive
    FromMbtiles {
        /// Source mbtiles file
        mbtiles_file_name: String,

        // Location to write the cotar file
        output_file: String,

        /// Hash file contents and deduplicate files with the same hash
        #[clap(short = 'e', long)]
        deduplicate: Option<bool>,

        /// Drop duplicate files from the archive
        #[clap(short = 'd', long)]
        drop_duplicates: Option<bool>,

        /// Create a tar index after the tar is create
        #[clap(long, action)]
        create_index: Option<bool>
    },
}

const MAX_SEARCH: usize = 100;

fn create(file_name: &str, force: bool, max_search: usize) {
    file_index_create(file_name, force, max_search);
}

fn file_index_create(file_name: &str, force: bool, max_search: usize) {
    if !file_name.ends_with(".tar") {
        println!("❌ {} does not end with .tar", file_name);
        process::exit(1);
    }

    let index_file_name = format!("{}.index", file_name);
    if !force && Path::new(index_file_name.as_str()).exists() {
        println!("❌ {} already exists", index_file_name);
        process::exit(1);
    }

    println!("Creating tar index from:{} max_search:{}", file_name, max_search);
    let mut cotar_index = CotarIndex::from_tar(file_name, 100_000).unwrap();
    println!("Tar read done.. files: {}", cotar_index.entries.len());

    // TODO handle file exists
    let mut output_file = File::create(index_file_name).expect("Failed to create output file");

    println!("Packing index..");
    let mut packing_factor = 1.0;
    loop {
        packing_factor += 0.0223;
        let packing_time = Instant::now();

        let output = cotar_index.pack(packing_factor).unwrap();
        println!(
            "Index packed! current_factor:{:.2}% search_max:{} search_avg: {:.2} duration:{}ms ",
            packing_factor * 100.0,
            output.search_max,
            output.search_avg,
            packing_time.elapsed().as_millis()
        );

        if output.search_max > max_search {
            continue;
        }
        output_file.write_all(&output.vec).unwrap();
        output_file.flush().unwrap();
        break;
    }

    println!("Index packed\n Files: {}", cotar_index.entries.len(),);
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Index {
            file_name,
            force,
            max_search,
        } => {
            file_index_create(
                file_name,
                force.unwrap_or(false),
                max_search.unwrap_or(MAX_SEARCH),
            );
        }
        Commands::Create {
            file_name,
            force,
            max_search,
        } => {
            create(
                file_name,
                force.unwrap_or(false),
                max_search.unwrap_or(MAX_SEARCH),
            );
        }
        Commands::FromMbtiles {
            mbtiles_file_name,
            output_file,
            deduplicate,
            drop_duplicates,
            create_index
        } => {
            crate::mbtiles::to_tar(
                mbtiles_file_name,
                output_file,
                deduplicate.unwrap_or(true),
                drop_duplicates.unwrap_or(false),
                create_index.unwrap_or(false)
            )
            .unwrap();
        }
        Commands::Validate {
            file_name,
            index_file_name,
        } => crate::validate::validate(file_name, index_file_name).expect("❌ Failed to validate"),
    }
}
