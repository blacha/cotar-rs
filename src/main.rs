mod cotar;

fn main() {
    let mut ct = cotar::Cotar::from_tar("/home/blacha/cotar/topographic.tar.co")
        .expect("Failed to read cotar");

    let tile_name = "tiles/8/247/108.pbf.gz";
    let info = ct.info(tile_name).expect("Failed to .info()");
    match info {
        None => println!("File '{}' not found ", tile_name),
        Some(tile_info) => println!("ct.info('{}') => {:?}", tile_name, tile_info),
    }

    let tile = ct.get(tile_name).expect("Failed to .get()");
    match tile {
        None => println!("File '{}' not found ", tile_name),
        Some(tile_data) => println!("ct.get('{}') => {:?}", tile_name, tile_data),
    }
}
