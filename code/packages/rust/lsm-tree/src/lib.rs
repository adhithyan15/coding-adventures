pub mod wal;
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

pub struct LSMTree<K: Ord + Clone + AsRef<[u8]>, V: Clone + AsRef<[u8]>> {
    pub memtable: SkipList<K, MemEntry<V>>,
    pub immutable_memtable: Option<SkipList<K, MemEntry<V>>>,
    pub levels: Vec<Vec<SSTableMeta<K>>>,
    pub wal_path: PathBuf,
    wal_writer: wal::WalWriter,
    pub seq: u64,
    pub snapshot_seqs: HashSet<u64>,
    pub data_dir: PathBuf,
}

impl<K: Ord + Clone + AsRef<[u8]>, V: Clone + AsRef<[u8]>> LSMTree<K, V> {
    /// Open (or create) an LSM tree rooted at data_dir.
    /// If data_dir contains an existing tree, recover its state.
    pub fn new<P: AsRef<Path>>(data_dir: P) -> io::Result<Self> {
        let wal_path = data_dir.as_ref().join("current.wal");
        let wal_writer = wal::WalWriter::new(&wal_path)?;

        Ok(Self {
            memtable: SkipList::new(),
            immutable_memtable: None,
            levels: vec![Vec::new()], // L0
            wal_path,
            wal_writer,
            seq: 0,
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
        self.wal_writer.append(&key, &entry)?;

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
        self.wal_writer.append(&key, &entry)?;

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
