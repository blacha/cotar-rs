use std::fs::File;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;

#[derive(Debug)]
pub struct DataView {
    file: File,
    pub size: u64,
}

// TODO this is a bad way of reading files, is there a better rust crate for this
// Ideally the same as npm lib @chunkd/fs read chunks from remote files with a async api
impl DataView {
    pub fn open(file_name: &str) -> Result<Self, std::io::Error> {
        let file = File::open(file_name)?;
        let file_size = file.metadata().unwrap().len();
        return Ok(DataView {
            file,
            size: file_size,
        });
    }

    pub fn byte(&mut self, offset: u64) -> Result<u8, std::io::Error> {
        let mut buf = vec![0; 1];
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.read(&mut buf)?;
        // println!("read u8 {:?} @ {}", buf, offset);

        return Ok(buf[0]);
    }

    pub fn u32_le(&mut self, offset: u64) -> Result<u32, std::io::Error> {
        let mut buf = vec![0; 4];
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.read(&mut buf)?;
        // println!("read u32 {:?} @ {}", buf, offset);
        return Ok(((buf[0] as u32) << 0)
            + ((buf[1] as u32) << 8)
            + ((buf[2] as u32) << 16)
            + ((buf[3] as u32) << 24));
    }

    pub fn u64_le(&mut self, offset: u64) -> Result<u64, std::io::Error> {
        let mut buf = vec![0; 8];
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.read(&mut buf)?;
        // println!("read u64 {:?} @ {}", buf, offset);
        return Ok(((buf[0] as u64) << 0)
            + ((buf[1] as u64) << 8)
            + ((buf[2] as u64) << 16)
            + ((buf[3] as u64) << 24)
            + ((buf[4] as u64) << 32)
            + ((buf[5] as u64) << 40)
            + ((buf[6] as u64) << 48)
            + ((buf[7] as u64) << 56));
    }

    pub fn bytes(&mut self, offset: u64, len: u64) -> Result<Vec<u8>, std::io::Error> {
        let mut buf: Vec<u8> = vec![0; len as usize];
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.read(&mut buf)?;
        // println!("read bytes {:?} @ {}", len, offset);
        return Ok(buf);
    }
}
