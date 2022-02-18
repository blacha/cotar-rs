use clap::{AppSettings, Parser, Subcommand};
use cotar::{Cotar, CotarIndex};
use std::fs::File;
use std::io::Write;
use std::process;
use std::time::Instant;

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
    Info {
        file_name: String,
    },
    Index {
        /// Tar file to index
        file_name: String,

        /// Worst case number of records that need to be searched to find a specific file
        ///
        /// - 25 slots: "-m 25"
        /// - 5 slots: "-m 5"
        ///
        /// Default: 100
        #[clap(short = 'm')]
        max_search: Option<usize>,
    },
}

fn file_info(file_name: &String) {
    println!("Reading cotar {}", file_name);

    let ct_open = Cotar::from_tar(file_name.as_str());

    match ct_open {
        Err(e) => {
            println!("{} Failed ❌\n {:?}", file_name, e);
            process::exit(1);
        }
        Ok(mut ct) => {
            println!(
                "{} Opened ✔️ COT: v{}\nHash Slots: {}",
                file_name, ct.version, ct.entries
            );
            let mut max_search = 0;
            let mut total_search = 0;
            let mut valid_entries: u64 = 0;
            for i in 0..ct.entries {
                let hash = ct.view.u64_le(i * 16 + 8).unwrap();
                if hash != 0 {
                    // Where should this hash actually be located
                    valid_entries = valid_entries + 1;
                    let search_count = i - (hash % ct.entries);
                    total_search = total_search + search_count;
                    if search_count > max_search {
                        max_search = search_count;
                    }
                }
            }

            // What is the worst case
            println!(
                "Hash Stats Max: {} Avg: {:.2},",
                max_search,
                (total_search as f64) / (valid_entries as f64),
            );
            println!("Entries: {}", valid_entries);
        }
    }
}

const MAX_SEARCH: usize = 100;

fn file_index_create(file_name: &String, max_search: usize) {
    if !file_name.ends_with(".tar") {
        println!("❌ {} does not end with .tar", file_name);
        process::exit(1);
    }

    println!("Reading tar {} max_search:{}", file_name, max_search);
    let mut cotar_index = CotarIndex::from_tar(file_name).unwrap();

    // TODO handle file exists
    let mut output_file = File::create("output.tar.index").expect("Failed to create output file");

    println!("Packing index..");
    let mut packing_factor = 1.0;
    loop {
        packing_factor = packing_factor + 0.01;
        let packing_time = Instant::now();

        let mut output = cotar_index.pack(packing_factor).unwrap();
        println!(
            "Index packed! current_factor:{:.2}% search:{} duration:{}ms ",
            packing_factor * 100.0,
            output.max_search,
            packing_time.elapsed().as_millis()
        );

        if output.max_search > max_search {
            continue;
        }
        output_file.write(&mut output.vec).unwrap();
        output_file.flush().unwrap();
        break;
    }

    println!("Index packed\n Files: {}", cotar_index.entries.len(),);
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Info { file_name } => file_info(file_name),
        Commands::Index {
            file_name,
            max_search,
        } => {
            file_index_create(file_name, max_search.unwrap_or(MAX_SEARCH));
        }
    }
}
