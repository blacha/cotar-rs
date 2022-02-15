use clap::{AppSettings, Parser, Subcommand};

use cotar::Cotar;

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
}

fn info_file(file_name: String) {
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

fn main() {
    let cli = Cli::parse();

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level app
    match &cli.command {
        Commands::Info { file_name } => info_file(file_name.to_string()),
    }
}
