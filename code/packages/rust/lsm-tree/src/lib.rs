pub mod sstable;

use bloom_filter::BloomFilter;
use skip_list::SkipList;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::io;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordType {
    Put = 1,
    Tombstone = 2,
}

#[derive(Debug, Clone)]
pub struct MemEntry<V> {
    pub value: Option<V>, // None if this is a tombstone
    pub record_type: RecordType,
    pub seq: u64,         // sequence number; higher = newer
}

pub struct BlockIndexEntry<K> {
    pub first_key: K,
    pub offset: u64,
    pub size: u32,
}

pub struct SSTableMeta<K> {
    pub file_id: u64,
    pub level: usize,
    pub path: PathBuf,
    pub min_key: K,
    pub max_key: K,
    pub size_bytes: u64,
    pub bloom: BloomFilter,
}

pub struct LSMTree<K: Ord + Clone + AsRef<[u8]> + TryFrom<Vec<u8>>, V: Clone + AsRef<[u8]> + TryFrom<Vec<u8>>> {
    pub memtable: SkipList<K, MemEntry<V>>,
    pub immutable_memtable: Option<SkipList<K, MemEntry<V>>>,
    pub levels: Vec<Vec<SSTableMeta<K>>>,
    pub wal_path: PathBuf,
    wal_writer: write_ahead_log::WalWriter,
    pub seq: u64,
    pub snapshot_seqs: HashSet<u64>,
    pub data_dir: PathBuf,
}

fn serialize_entry<K: AsRef<[u8]>, V: AsRef<[u8]>>(seq: u64, record_type: RecordType, key: &K, value: Option<&V>) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&seq.to_le_bytes());
    buf.push(record_type as u8);
    let k_bytes = key.as_ref();
    buf.extend_from_slice(&(k_bytes.len() as u32).to_le_bytes());
    
    let v_bytes = value.map_or(&[][..], |v| v.as_ref());
    buf.extend_from_slice(&(v_bytes.len() as u32).to_le_bytes());
    
    buf.extend_from_slice(k_bytes);
    buf.extend_from_slice(v_bytes);
    buf
}

fn deserialize_entry(bytes: &[u8]) -> Option<(Vec<u8>, MemEntry<Vec<u8>>)> {
    if bytes.len() < 17 { return None; }
    
    let mut seq_buf = [0u8; 8];
    seq_buf.copy_from_slice(&bytes[0..8]);
    let seq = u64::from_le_bytes(seq_buf);
    
    let record_type = match bytes[8] {
        1 => RecordType::Put,
        2 => RecordType::Tombstone,
        _ => return None,
    };
    
    let mut klen_buf = [0u8; 4];
    klen_buf.copy_from_slice(&bytes[9..13]);
    let key_len = u32::from_le_bytes(klen_buf) as usize;
    
    let mut vlen_buf = [0u8; 4];
    vlen_buf.copy_from_slice(&bytes[13..17]);
    let val_len = u32::from_le_bytes(vlen_buf) as usize;
    
    if bytes.len() < 17 + key_len + val_len { return None; }
    
    let key = bytes[17..17+key_len].to_vec();
    let val = if record_type == RecordType::Tombstone {
        None
    } else {
        Some(bytes[17+key_len..17+key_len+val_len].to_vec())
    };
    
    Some((key, MemEntry { value: val, record_type, seq }))
}

impl<K: Ord + Clone + AsRef<[u8]> + TryFrom<Vec<u8>>, V: Clone + AsRef<[u8]> + TryFrom<Vec<u8>>> LSMTree<K, V> {
    /// Open (or create) an LSM tree rooted at data_dir.
    /// If data_dir contains an existing tree, recover its state.
    pub fn new<P: AsRef<Path>>(data_dir: P) -> io::Result<Self> {
        let wal_path = data_dir.as_ref().join("current.wal");
        
        let mut memtable = SkipList::new();
        let mut seq = 0;

        // Crash Recovery
        if wal_path.exists() {
            if let Ok(mut reader) = write_ahead_log::WalReader::new(&wal_path) {
                while let Ok(Some(bytes)) = reader.read_next() {
                    if let Some((key_bytes, entry)) = deserialize_entry(&bytes) {
                        let key_res = K::try_from(key_bytes);
                        let val_opt = match entry.value {
                            Some(v_bytes) => V::try_from(v_bytes).ok(),
                            None => None,
                        };
                        
                        if let Ok(key) = key_res {
                            let mem_entry = MemEntry {
                                value: val_opt,
                                record_type: entry.record_type,
                                seq: entry.seq,
                            };
                            memtable.insert(key, mem_entry);
                            if entry.seq > seq {
                                seq = entry.seq;
                            }
                        }
                    }
                }
            }
        }

        let wal_writer = write_ahead_log::WalWriter::new(&wal_path)?;

        Ok(Self {
            memtable,
            immutable_memtable: None,
            levels: vec![Vec::new()], // L0
            wal_path,
            wal_writer,
            seq,
            snapshot_seqs: HashSet::new(),
            data_dir: data_dir.as_ref().to_path_buf(),
        })
    }

    pub fn put(&mut self, key: K, value: V) -> io::Result<()> {
        self.seq += 1;
        let entry = MemEntry {
            value: Some(value.clone()),
            record_type: RecordType::Put,
            seq: self.seq,
        };

        // Append to WAL
        let data = serialize_entry(self.seq, RecordType::Put, &key, Some(&value));
        self.wal_writer.append_record(&data)?;

        self.memtable.insert(key, entry);

        // TODO: Check if memtable is full and trigger flush

        Ok(())
    }

    pub fn delete(&mut self, key: K) -> io::Result<()> {
        self.seq += 1;
        let entry = MemEntry {
            value: None,
            record_type: RecordType::Tombstone,
            seq: self.seq,
        };

        // Append to WAL
        let data = serialize_entry::<K, V>(self.seq, RecordType::Tombstone, &key, None);
        self.wal_writer.append_record(&data)?;

        self.memtable.insert(key, entry);
        Ok(())
    }

    /// Read most recent value for key, or None if not present.
    pub fn get(&self, key: &K) -> Option<V> {
        let target_seq = self.seq;

        // 1. Check memtable
        // Iterate through entries to find the one with the highest sequence number <= target_seq
        if let Some(entry) = self.memtable.search(key) {
            if entry.seq <= target_seq {
                if entry.record_type == RecordType::Tombstone {
                    return None;
                }
                return entry.value;
            }
        }

        // 2. Check immutable memtable
        if let Some(immutable) = &self.immutable_memtable {
            if let Some(entry) = immutable.search(key) {
                if entry.seq <= target_seq {
                    if entry.record_type == RecordType::Tombstone {
                        return None;
                    }
                    return entry.value;
                }
            }
        }

        // 3. Check L0 SSTables (newest first)
        // TODO: Implement SSTable bloom filter check and disk read

        // 4. Check L1, L2, ...
        // TODO: Implement L1+ level search

        None
    }
}
