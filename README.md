# Cotar

Rust based [cotar](https://github.com/linz/cotar) reader

## CLI

Commands:
- `index` Create a tar index for a tar
- `validate` Validate tar index
- `from-mbtiles` Create a tar from a mbtiles archive

Examples
Create a tar index and validate it

create `topographic.tar.index`

```
cotar index topographic.tar
```

validate the new index

```
cotar validate topographic.tar topographic.tar.index
```

Convert a mbtiles to a tar

```
cotar from-mbtiles topographic.mbtiles topographic.tar

cotar index topographic.tar # creates topographic.tar.index
```

## Library

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

### Building

```
cargo build --release
```
