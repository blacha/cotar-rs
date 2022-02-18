use std::fs::File;
use std::io::Read;
use std::io::Result as IoResult;
use std::io::Seek;
use std::io::SeekFrom;

#[derive(Debug)]
pub struct DataView {
    file: File,
    pub size: u64,
}

// TODO this is a bad way of reading files, is there a better rust crate for this
// Ideally the same as npm lib @chunkd/fs which reads chunks from remote files with a async api
impl DataView {
    pub fn open(file_name: &str) -> IoResult<Self> {
        let file = File::open(file_name)?;
        let file_size = file.metadata()?.len();
        Ok(DataView {
            file,
            size: file_size,
        })
    }

    fn read_at(&mut self, offset: u64, bytes: usize) -> IoResult<Vec<u8>> {
        let mut buf = vec![0; bytes];
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.read_exact(&mut buf)?;
        Ok(buf)
    }

    pub fn byte(&mut self, offset: u64) -> IoResult<u8> {
        let buf = self.read_at(offset, 1)?;
        // println!("read u8 {:?} @ {}", buf, offset);
        Ok(buf[0])
    }

    pub fn u32_le(&mut self, offset: u64) -> IoResult<u32> {
        let buf = self.read_at(offset, 4)?;

        // println!("read u32 {:?} @ {}", buf, offset);
        Ok(u32::from_le_bytes(
            buf.try_into().expect("Failed to read u32_le"),
        ))
    }

    pub fn u64_le(&mut self, offset: u64) -> IoResult<u64> {
        let buf = self.read_at(offset, 8)?;

        // println!("read u64 {:?} @ {}", buf, offset);
        Ok(u64::from_le_bytes(
            buf.try_into().expect("Failed to read u64_le"),
        ))
    }

    pub fn bytes(&mut self, offset: u64, len: u64) -> IoResult<Vec<u8>> {
        let buf = self.read_at(offset, len as usize)?;

        // println!("read bytes {:?} @ {}", len, offset);
        Ok(buf)
    }
}
