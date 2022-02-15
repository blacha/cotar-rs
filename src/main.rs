mod cotar;
mod dataview;
mod fnv1a;

fn main() {
    let mut ct = cotar::Cotar::from_tar("/home/blacha/cotar/topographic.tar.co")
        .expect("Failed to read cotar");

    let file_name = "tiles/8/247/108.pbf.gz";
    let info = ct.info(file_name).expect("Failed to .info()");
    match info {
        None => println!("File '{}' not found ", file_name),
        Some(file_info) => println!("ct.info('{}') => {:?}", file_name, file_info),
    }

    let tile = ct.get(file_name).expect("Failed to .get()");
    match tile {
        None => println!("File '{}' not found ", file_name),
        Some(file_data) => println!("ct.get('{}') => {:?}", file_name, file_data),
    }
}
