use cotar::Cotar;
use std::fs::File;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Result as IoResult;
use tar::{Archive, EntryType};

pub fn create_cotar(tar_file: &str, tar_index: &Option<String>) -> IoResult<Cotar> {
    match tar_index {
        Some(tar_index_name) => Cotar::from_tar_index(tar_file, &tar_index_name.to_string()),
        None => Cotar::from_tar(tar_file),
    }
}

pub fn validate(tar_file: &str, tar_index: &Option<String>) -> IoResult<()> {
    let mut ct = create_cotar(tar_file, tar_index).expect("Failed to open cotar");
    println!("Cotar v{} Opened, entries: {}", ct.version, ct.entries);

    let file = File::open(tar_file)?;
    let mut a = Archive::new(file);

    let mut count = 0;

    for file in a.entries()? {
        let file = file?;

        let header = file.header();
        let file_path = header.path()?;
        let file_name = file_path.to_str().expect("Failed to extract path");

        match header.entry_type() {
            EntryType::Regular => {
                // offset to the file is at end of the header
                let file_offset = file.raw_header_position() + 512;
                // let info = ct.info(file_name).expect("Failed to find file");
                let file_size = header.size()? as u32;

                if let Some(info) = ct.info(file_name).expect("Failed to find file") {
                    assert_eq!(info.file_offset, file_offset);
                    assert_eq!(info.file_size, file_size);

                    // println!("Ok {} - {} {} {:?}", file_name, file_offset, file_size, info);
                } else {
                    println!("Missing Lookup {}", file_name);
                    return Err(Error::new(
                        ErrorKind::Other,
                        format!("Missing file: {}", file_name),
                    ));
                }
            }
            EntryType::Link => {
                // let link_path = header.link_name()?.expect("No link path found??");
                // let link_name = link_path.to_str().expect("Failed to extract link_path");
                // cotar_index.link(file_name, link_name)?;
            }
            e => {
                // Folders/other files??
                println!("Unknown entry_type: {:?}", e)
            }
        }

        count += 1;
        // If a report is requested dump how far through the file we are.
        if count % 25_000 == 0 {
            println!("{}", count);
        }
    }

    println!("✔️ Cotar Validated unique_files:{}", count);

    Ok(())
}
