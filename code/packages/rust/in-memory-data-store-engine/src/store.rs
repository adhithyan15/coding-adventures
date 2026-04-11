use hash_map::HashMap as DtHashMap;
use heap::MinHeap;
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
    pub entries: DtHashMap<Vec<u8>, Entry>,
    pub ttl_heap: MinHeap<(u64, Vec<u8>)>,
}

impl PartialEq for Database {
    fn eq(&self, other: &Self) -> bool {
        database_entries_equal(&self.entries, &other.entries)
    }
}

impl Eq for Database {}

impl Database {
    pub fn empty() -> Self {
        Self {
            entries: DtHashMap::default(),
            ttl_heap: MinHeap::new(),
        }
    }

    pub fn get(&self, key: impl AsRef<[u8]>) -> Option<&Entry> {
        let key = key.as_ref().to_vec();
        let entry = self.entries.get(&key)?;
        if entry
            .expires_at
            .is_some_and(|expires_at| current_time_ms() >= expires_at)
        {
            return None;
        }
        Some(entry)
    }

    pub fn set(mut self, key: impl Into<Vec<u8>>, entry: Entry) -> Self {
        let key = key.into();
        if let Some(expires_at) = entry.expires_at {
            self.ttl_heap.push((expires_at, key.clone()));
        }
        self.entries = self.entries.set(key, entry);
        self
    }

    pub fn delete(mut self, key: impl AsRef<[u8]>) -> Self {
        let key = key.as_ref().to_vec();
        self.entries = self.entries.delete(&key);
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
        let mut keys = self
            .entries
            .keys()
            .into_iter()
            .filter(|key| glob_match(pattern, key))
            .collect::<Vec<_>>();
        keys.sort();
        keys
    }

    pub fn dbsize(&self) -> usize {
        self.entries
            .keys()
            .into_iter()
            .filter(|key| self.get(key.as_slice()).is_some())
            .count()
    }

    pub fn expire_lazy(mut self, key: Option<impl AsRef<[u8]>>) -> Self {
        let Some(key) = key else {
            return self;
        };
        let key = key.as_ref().to_vec();
        let expired = self
            .entries
            .get(&key)
            .and_then(|entry| entry.expires_at)
            .is_some_and(|expires_at| current_time_ms() >= expires_at);
        if expired {
            self.entries = self.entries.delete(&key);
        }
        self
    }

    pub fn active_expire(mut self) -> Self {
        let now = current_time_ms();
        while let Some((expires_at, key)) = self.ttl_heap.peek().cloned() {
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
                self.entries = self.entries.delete(&key);
            }
        }
        self
    }

    pub fn clear(mut self) -> Self {
        self.entries = DtHashMap::default();
        self.ttl_heap = MinHeap::new();
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

fn database_entries_equal(
    left: &DtHashMap<Vec<u8>, Entry>,
    right: &DtHashMap<Vec<u8>, Entry>,
) -> bool {
    if left.size() != right.size() {
        return false;
    }
    for (key, value) in left.entries() {
        if right.get(&key) != Some(&value) {
            return false;
        }
    }
    true
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
                !text.is_empty()
                    && pattern[0] == text[0]
                    && glob_match_inner(&pattern[1..], &text[1..])
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_helpers_cover_glob_matching_and_expiration_paths() {
        assert!(glob_match(b"", b""));
        assert!(!glob_match(b"", b"x"));
        assert!(glob_match(b"*", b"abc"));
        assert!(glob_match(b"a?c", b"abc"));
        assert!(glob_match(b"user:[1-2]", b"user:1"));
        assert!(glob_match(b"user:[1-2]", b"user:2"));
        assert!(!glob_match(b"user:[1-2]", b"user:3"));
        assert!(glob_match(b"[", b"["));
        assert!(class_contains(b"a-cx", b'b'));
        assert!(class_contains(b"a-cx", b'x'));
        assert!(!class_contains(b"a-cx", b'z'));

        let now = current_time_ms();
        let expired = now.saturating_sub(1);
        let future = now.saturating_add(60_000);

        let mut db = Database::empty();
        db = db.set(b"alive", Entry::string("value", None));
        db = db.set(b"temp", Entry::string("gone", Some(expired)));
        db = db.set(b"later", Entry::string("stay", Some(future)));

        let mut same = Database::empty();
        same = same.set(b"later", Entry::string("stay", Some(future)));
        same = same.set(b"temp", Entry::string("gone", Some(expired)));
        same = same.set(b"alive", Entry::string("value", None));
        assert_eq!(db, same);

        assert!(db.exists(b"alive"));
        assert_eq!(db.type_of(b"alive"), Some(EntryType::String));
        assert_eq!(db.dbsize(), 2);
        assert_eq!(db.keys(b"l*"), vec![b"later".to_vec()]);
        assert!(db.get(b"temp").is_none());

        let lazy = db.clone().expire_lazy(Some(b"temp"));
        assert!(lazy.entries.get(&b"temp".to_vec()).is_none());

        let active = db.active_expire();
        assert!(active.entries.get(&b"temp".to_vec()).is_none());
        assert!(active.entries.get(&b"later".to_vec()).is_some());

        let cleared = active.clear();
        assert_eq!(cleared.entries.size(), 0);
        assert!(cleared.ttl_heap.is_empty());
    }

    #[test]
    fn store_selection_and_flush_paths_cover_database_switching() {
        let mut store = Store::empty().with_active_db(99);
        assert_eq!(store.active_db, DEFAULT_DB_COUNT - 1);

        store = store.set(b"tail", Entry::string("last", None));
        store = store.select(0);
        store = store.set(b"head", Entry::string("first", None));

        {
            let db = store.current_db_mut();
            db.entries = db.entries.clone().set(b"manual".to_vec(), Entry::string("ok", None));
        }

        assert_eq!(store.current_db().dbsize(), 2);
        assert_eq!(store.keys(b"h*"), vec![b"head".to_vec()]);
        assert_eq!(store.type_of(b"head"), Some(EntryType::String));
        assert!(store.exists(b"manual"));
        assert_eq!(store.dbsize(), 2);

        store = store.delete(b"manual");
        assert!(!store.exists(b"manual"));

        let mut expired_db = Database::empty();
        expired_db = expired_db.set(
            b"gone",
            Entry::string("bye", Some(current_time_ms().saturating_sub(1))),
        );
        store.databases[1] = expired_db;
        store = store.expire_lazy(None::<&[u8]>);
        store = store.active_expire_all();
        assert!(store.databases[1].entries.get(&b"gone".to_vec()).is_none());

        store = store.flushdb();
        assert_eq!(store.dbsize(), 0);
        assert_eq!(store.current_db().entries.size(), 0);

        store.databases[1] = Database::empty().set(b"other", Entry::string("x", None));
        store = store.flushall();
        assert!(store.databases.iter().all(|db| db.entries.size() == 0));
    }
}
