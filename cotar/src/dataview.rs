use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::io::Result as IoResult;
use std::io::Seek;
use std::io::SeekFrom;

use bytes::Bytes;

#[derive(Debug)]
pub struct DataView {
    file: BufReader<std::fs::File>,
    pub size: u64,
}

impl DataView {
    pub fn open(file_name: &str) -> IoResult<Self> {
        let file = File::open(file_name)?;
        let file_size = file.metadata()?.len();
        let reader: BufReader<File> = BufReader::new(file);

        Ok(DataView {
            file: reader,
            size: file_size,
        })
    }

    pub fn read_exact(&mut self, offset: u64, len: u64) -> IoResult<Bytes> {
        let mut buf = vec![0; len as usize];

        let current_position = self.file.stream_position()?;
        if current_position != offset {
            self.file.seek(SeekFrom::Start(offset))?;
        } 
        self.file.read_exact(&mut buf)?;
        Ok(Bytes::from(buf))
    }
}
