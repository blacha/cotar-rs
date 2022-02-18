const FNV1A_PRIME_64: u64 = 0x00000100000001b3;
const FNV1A_OFFSET_64: u64 = 0xcbf29ce484222325;

pub fn fnv1a_64(buf: &[u8]) -> u64 {
    let mut hash = FNV1A_OFFSET_64;
    for ch in buf {
        hash ^=  *ch as u64;
        hash = hash.wrapping_mul(FNV1A_PRIME_64);
    }
    hash
}

#[test]
fn test_ascii() {
    assert_eq!(0xa430d84680aabd0b, fnv1a_64("hello".as_bytes()));
    assert_eq!(0x8c0ec8d1fb9e6e32, fnv1a_64("Hello World!".as_bytes()));
}

#[test]
fn test_utf8() {
    assert_eq!(0x0ac1e907b717cfd7, fnv1a_64("ß".as_bytes()));
    assert_eq!(0xa243ed17175ca587, fnv1a_64("🦄🌈".as_bytes()));
    assert_eq!(0xff9c5f3875888db2, fnv1a_64("🦄".as_bytes()));
    assert_eq!(0xff430738753bcf54, fnv1a_64("🌈".as_bytes()));
    assert_eq!(0x1366018ddd32b3cc, fnv1a_64("🦄🌈🌈🌈🦄".as_bytes()));
}
