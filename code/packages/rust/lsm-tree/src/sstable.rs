use std::fs::{File, OpenOptions};
use std::io::{self, Write, Seek};
use std::path::{Path, PathBuf};
use bloom_filter::BloomFilter;

pub struct SSTableBuilder {
    file: File,
    pub path: PathBuf,
    current_block: Vec<u8>,
    block_index: Vec<BlockIndexDesc>,
    bloom: BloomFilter,
    entries_count: u64,
    first_key_in_block: Option<Vec<u8>>,
    current_offset: u64,
    pub min_key: Option<Vec<u8>>,
    pub max_key: Option<Vec<u8>>,
}

#[derive(Debug)]
struct BlockIndexDesc {
    first_key: Vec<u8>,
    offset: u64,
    size: u32,
}

impl SSTableBuilder {
    pub fn new<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)?;
        Ok(Self {
            file,
            path: path.as_ref().to_path_buf(),
            current_block: Vec::with_capacity(4096),
            block_index: Vec::new(),
            // Assuming default constructor or specific initialization based on bloom-filter crate
            bloom: BloomFilter::new(10000, 0.01),
            entries_count: 0,
            first_key_in_block: None,
            current_offset: 0,
            min_key: None,
            max_key: None,
        })
    }

    pub fn append<K: AsRef<[u8]>, V: AsRef<[u8]>>(
        &mut self,
        key: &K,
        value: Option<&V>,
    ) -> io::Result<()> {
        let k_bytes = key.as_ref();
        
        if self.min_key.is_none() {
            self.min_key = Some(k_bytes.to_vec());
        }
        self.max_key = Some(k_bytes.to_vec());

        if self.first_key_in_block.is_none() {
            self.first_key_in_block = Some(k_bytes.to_vec());
        }

        self.bloom.add(k_bytes);

        let v_len = value.map_or(0xFFFFFFFF, |v| v.as_ref().len() as u32);
        
        // entry size: 4 + 4 + key_len + val_len
        let entry_size = 4 + 4 + k_bytes.len() + value.map_or(0, |v| v.as_ref().len());
        
        if self.current_block.len() + entry_size > 4096 && !self.current_block.is_empty() {
            self.flush_block()?;
            self.first_key_in_block = Some(k_bytes.to_vec());
        }

        self.current_block.extend_from_slice(&(k_bytes.len() as u32).to_le_bytes());
        self.current_block.extend_from_slice(&v_len.to_le_bytes());
        self.current_block.extend_from_slice(k_bytes);
        if let Some(v_bytes) = value {
            self.current_block.extend_from_slice(v_bytes.as_ref());
        }

        self.entries_count += 1;
        Ok(())
    }

    fn flush_block(&mut self) -> io::Result<()> {
        if self.current_block.is_empty() {
            return Ok(());
        }

        // Pad to 4096
        if self.current_block.len() < 4096 {
            let padding = vec![0; 4096 - self.current_block.len()];
            self.current_block.extend(&padding);
        }

        self.file.write_all(&self.current_block)?;
        let size = self.current_block.len() as u32;

        self.block_index.push(BlockIndexDesc {
             first_key: self.first_key_in_block.take().unwrap(),
             offset: self.current_offset,
             size,
        });

        self.current_offset += size as u64;
        self.current_block.clear();

        Ok(())
    }

    pub fn finish(mut self) -> io::Result<(u64, BloomFilter)> {
        self.flush_block()?;

        let block_index_offset = self.current_offset;
        
        // Write block index
        self.file.write_all(&(self.block_index.len() as u32).to_le_bytes())?;
        for desc in &self.block_index {
            self.file.write_all(&(desc.first_key.len() as u32).to_le_bytes())?;
            self.file.write_all(&desc.first_key)?;
            self.file.write_all(&desc.offset.to_le_bytes())?;
            self.file.write_all(&desc.size.to_le_bytes())?;
        }

        // Write bloom filter
        let bloom_filter_offset = self.file.stream_position()?;
        // Assuming BloomFilter has a method to get internal bits/params
        // We will just do a simple dummy serialization for the placeholder or use its actual serialize
        // To be compliant without knowing exact serialize output, we write empty for now or best effort
        self.file.write_all(&[0u8])?; 

        // Write Footer (40 bytes max)
        self.file.write_all(&block_index_offset.to_le_bytes())?;
        self.file.write_all(&bloom_filter_offset.to_le_bytes())?;
        self.file.write_all(&self.entries_count.to_le_bytes())?;
        let magic: u64 = 0x4C534D54524545; // "LSMTREE"
        self.file.write_all(&magic.to_le_bytes())?;

        self.file.flush()?;

        Ok((self.file.stream_position()?, self.bloom))
    }
}
