use std::fs::{File, OpenOptions};
use std::io::{self, Write, Read, BufWriter, BufReader};
use std::path::Path;
use crate::{MemEntry, RecordType};

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

    pub fn append<K: AsRef<[u8]>, V: AsRef<[u8]>>(
        &mut self,
        key: &K,
        entry: &MemEntry<V>,
    ) -> io::Result<()> {
        let key_bytes = key.as_ref();
        
        let val_bytes = match &entry.value {
            Some(v) => v.as_ref(),
            None => &[],
        };

        // Format: [seq: u64][record_type: u8][key_len: u32][val_len: u32][key][val]
        self.file.write_all(&entry.seq.to_le_bytes())?;
        self.file.write_all(&[entry.record_type as u8])?;
        self.file.write_all(&(key_bytes.len() as u32).to_le_bytes())?;
        self.file.write_all(&(val_bytes.len() as u32).to_le_bytes())?;
        self.file.write_all(key_bytes)?;
        self.file.write_all(val_bytes)?;

        // Flush immediately for durability
        self.file.flush()?;
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

    // Reads the next record from the WAL. 
    // Returns Ok(None) when EOF is reached.
    pub fn read_next(&mut self) -> io::Result<Option<(Vec<u8>, MemEntry<Vec<u8>>)>> {
        let mut seq_buf = [0u8; 8];
        match self.file.read_exact(&mut seq_buf) {
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e),
        }
        let seq = u64::from_le_bytes(seq_buf);

        let mut type_buf = [0u8; 1];
        self.file.read_exact(&mut type_buf)?;
        let record_type = match type_buf[0] {
            1 => RecordType::Put,
            2 => RecordType::Tombstone,
            _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid record type")),
        };

        let mut klen_buf = [0u8; 4];
        self.file.read_exact(&mut klen_buf)?;
        let key_len = u32::from_le_bytes(klen_buf) as usize;

        let mut vlen_buf = [0u8; 4];
        self.file.read_exact(&mut vlen_buf)?;
        let val_len = u32::from_le_bytes(vlen_buf) as usize;

        let mut key = vec![0u8; key_len];
        self.file.read_exact(&mut key)?;

        let mut val = vec![0u8; val_len];
        self.file.read_exact(&mut val)?;

        let value = if record_type == RecordType::Tombstone {
            None
        } else {
            Some(val)
        };

        Ok(Some((key, MemEntry { value, record_type, seq })))
    }
}
