# Cotar


**Work in progress** 

Rust based [cotar](https://github.com/linz/cotar) reader


```rust
use cotar;

let mut ct = cotar::Cotar::from_tar("example.tar.co").expect("Failed to ::from_tar()");
let file_name = "tiles/8/247/108.pbf.gz";

let file_info = ct.info(file_name).expect("Failed to .info()");
println!("ct.info('{}') {:?}", file_name, file_info);
// CotarIndexEntry { hash: 121498119488259422, offset: 7080448, size: 68 }

let file_data = ct.get(file_name).expect("Failed to .get()");
println!("ct.get('{}') {:?}", file_name, file_data);
// [26, 66, 120 ... ]
```