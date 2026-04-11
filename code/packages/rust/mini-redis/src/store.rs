use core::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::types::{Entry, EntryType};

pub const DEFAULT_DB_COUNT: usize = 16;

pub fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[derive(Clone, Debug)]
pub struct Database {
    pub entries: BTreeMap<Vec<u8>, Entry>,
    pub ttl_heap: BinaryHeap<Reverse<(u64, Vec<u8>)>>,
}

impl PartialEq for Database {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}

impl Eq for Database {}

impl Database {
    pub fn empty() -> Self {
        Self {
            entries: BTreeMap::new(),
            ttl_heap: BinaryHeap::new(),
        }
    }

    pub fn get(&self, key: impl AsRef<[u8]>) -> Option<&Entry> {
        let key = key.as_ref();
        let entry = self.entries.get(key)?;
        if entry.expires_at.is_some_and(|expires_at| current_time_ms() >= expires_at) {
            return None;
        }
        Some(entry)
    }

    pub fn set(mut self, key: impl Into<Vec<u8>>, entry: Entry) -> Self {
        let key = key.into();
        if let Some(expires_at) = entry.expires_at {
            self.ttl_heap.push(Reverse((expires_at, key.clone())));
        }
        self.entries.insert(key, entry);
        self
    }

    pub fn delete(mut self, key: impl AsRef<[u8]>) -> Self {
        self.entries.remove(key.as_ref());
        self
    }

    pub fn exists(&self, key: impl AsRef<[u8]>) -> bool {
        self.get(key).is_some()
    }

    pub fn type_of(&self, key: impl AsRef<[u8]>) -> Option<EntryType> {
        self.get(key).map(|entry| entry.entry_type.clone())
    }

    pub fn keys(&self, pattern: impl AsRef<[u8]>) -> Vec<Vec<u8>> {
        let pattern = pattern.as_ref();
        self.entries
            .keys()
            .filter(|key| glob_match(pattern, key))
            .cloned()
            .collect()
    }

    pub fn dbsize(&self) -> usize {
        self.entries
            .keys()
            .filter(|key| self.get(key.as_slice()).is_some())
            .count()
    }

    pub fn expire_lazy(mut self, key: Option<impl AsRef<[u8]>>) -> Self {
        let Some(key) = key else {
            return self;
        };
        let key = key.as_ref();
        let expired = self
            .entries
            .get(key)
            .and_then(|entry| entry.expires_at)
            .is_some_and(|expires_at| current_time_ms() >= expires_at);
        if expired {
            self.entries.remove(key);
        }
        self
    }

    pub fn active_expire(mut self) -> Self {
        let now = current_time_ms();
        while let Some(Reverse((expires_at, key))) = self.ttl_heap.peek().cloned() {
            if expires_at > now {
                break;
            }
            self.ttl_heap.pop();
            let should_delete = self
                .entries
                .get(&key)
                .and_then(|entry| entry.expires_at)
                .is_some_and(|current| current == expires_at);
            if should_delete {
                self.entries.remove(&key);
            }
        }
        self
    }

    pub fn clear(mut self) -> Self {
        self.entries.clear();
        self.ttl_heap.clear();
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Store {
    pub databases: Vec<Database>,
    pub active_db: usize,
}

impl Store {
    pub fn empty() -> Self {
        Self {
            databases: std::iter::repeat_with(Database::empty)
                .take(DEFAULT_DB_COUNT)
                .collect(),
            active_db: 0,
        }
    }

    pub fn with_active_db(mut self, active_db: usize) -> Self {
        self.active_db = active_db.min(self.databases.len().saturating_sub(1));
        self
    }

    pub fn select(mut self, active_db: usize) -> Self {
        self.active_db = active_db.min(self.databases.len().saturating_sub(1));
        self
    }

    pub fn get(&self, key: impl AsRef<[u8]>) -> Option<&Entry> {
        self.current_db().get(key)
    }

    pub fn set(mut self, key: impl Into<Vec<u8>>, entry: Entry) -> Self {
        let db = self.current_db_mut();
        let updated = db.clone().set(key, entry);
        *db = updated;
        self
    }

    pub fn delete(mut self, key: impl AsRef<[u8]>) -> Self {
        let db = self.current_db_mut();
        let updated = db.clone().delete(key);
        *db = updated;
        self
    }

    pub fn exists(&self, key: impl AsRef<[u8]>) -> bool {
        self.get(key).is_some()
    }

    pub fn keys(&self, pattern: impl AsRef<[u8]>) -> Vec<Vec<u8>> {
        self.current_db().keys(pattern)
    }

    pub fn type_of(&self, key: impl AsRef<[u8]>) -> Option<EntryType> {
        self.current_db().type_of(key)
    }

    pub fn dbsize(&self) -> usize {
        self.current_db().dbsize()
    }

    pub fn expire_lazy(mut self, key: Option<impl AsRef<[u8]>>) -> Self {
        let active_db = self.active_db;
        let updated = self.databases[active_db].clone().expire_lazy(key);
        self.databases[active_db] = updated;
        self
    }

    pub fn active_expire(mut self) -> Self {
        let active_db = self.active_db;
        let updated = self.databases[active_db].clone().active_expire();
        self.databases[active_db] = updated;
        self
    }

    pub fn active_expire_all(mut self) -> Self {
        for db in &mut self.databases {
            let updated = db.clone().active_expire();
            *db = updated;
        }
        self
    }

    pub fn flushdb(mut self) -> Self {
        let active_db = self.active_db;
        self.databases[active_db] = Database::empty();
        self
    }

    pub fn flushall(mut self) -> Self {
        for db in &mut self.databases {
            *db = Database::empty();
        }
        self
    }

    pub fn current_db(&self) -> &Database {
        &self.databases[self.active_db]
    }

    pub fn current_db_mut(&mut self) -> &mut Database {
        &mut self.databases[self.active_db]
    }
}

fn glob_match(pattern: &[u8], text: &[u8]) -> bool {
    glob_match_inner(pattern, text)
}

fn glob_match_inner(pattern: &[u8], text: &[u8]) -> bool {
    if pattern.is_empty() {
        return text.is_empty();
    }

    match pattern[0] {
        b'*' => {
            glob_match_inner(&pattern[1..], text)
                || (!text.is_empty() && glob_match_inner(pattern, &text[1..]))
        }
        b'?' => !text.is_empty() && glob_match_inner(&pattern[1..], &text[1..]),
        b'[' => {
            if let Some(end) = pattern.iter().position(|byte| *byte == b']') {
                if text.is_empty() {
                    return false;
                }
                let class = &pattern[1..end];
                if class_contains(class, text[0]) {
                    glob_match_inner(&pattern[end + 1..], &text[1..])
                } else {
                    false
                }
            } else {
                !text.is_empty() && pattern[0] == text[0] && glob_match_inner(&pattern[1..], &text[1..])
            }
        }
        byte => !text.is_empty() && byte == text[0] && glob_match_inner(&pattern[1..], &text[1..]),
    }
}

fn class_contains(class: &[u8], byte: u8) -> bool {
    let mut i = 0;
    while i < class.len() {
        if i + 2 < class.len() && class[i + 1] == b'-' {
            let start = class[i];
            let end = class[i + 2];
            if start <= byte && byte <= end {
                return true;
            }
            i += 3;
        } else {
            if class[i] == byte {
                return true;
            }
            i += 1;
        }
    }
    false
}
