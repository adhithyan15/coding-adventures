use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::Path;

pub struct WalWriter {
    file: BufWriter<File>,
}

impl WalWriter {
    pub fn new<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        Ok(Self {
            file: BufWriter::new(file),
        })
    }

    /// Append a generic byte record to the WAL.
    /// Prefixes the data with a u32 length.
    pub fn append_record(&mut self, data: &[u8]) -> io::Result<()> {
        let len = data.len() as u32;
        self.file.write_all(&len.to_le_bytes())?;
        self.file.write_all(data)?;
        self.file.flush()?; // Guarantee durability
        Ok(())
    }
}

pub struct WalReader {
    file: BufReader<File>,
}

impl WalReader {
    pub fn new<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = File::open(path)?;
        Ok(Self {
            file: BufReader::new(file),
        })
    }

    /// Read the next record from the WAL.
    /// Returns Ok(None) at End of File.
    pub fn read_next(&mut self) -> io::Result<Option<Vec<u8>>> {
        let mut len_buf = [0u8; 4];
        match self.file.read_exact(&mut len_buf) {
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e),
        }
        let len = u32::from_le_bytes(len_buf) as usize;

        let mut data = vec![0u8; len];
        self.file.read_exact(&mut data)?;

        Ok(Some(data))
    }
}
