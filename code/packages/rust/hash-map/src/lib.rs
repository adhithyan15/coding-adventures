//! DT18 hash map implemented from scratch in Rust.
//!
//! The crate exposes a generic [`HashMap`] with two collision strategies:
//! separate chaining and open addressing with tombstones. Public helpers are
//! provided as both inherent methods and free functions so the API can be used
//! in a more functional style or in a more direct Rust style.

use std::fmt::Debug;

use coding_adventures_hash_functions::{
    djb2, fnv1a_32, murmur3_32, siphash_2_4,
};

const DEFAULT_CAPACITY: usize = 16;
const CHAINING_RESIZE_THRESHOLD: f64 = 1.0;
const OPEN_ADDRESSING_RESIZE_THRESHOLD: f64 = 0.75;
const SIPHASH_KEY: [u8; 16] = *b"codex-dt18-key!!";

fn serialize_key<K: Debug + ?Sized>(key: &K) -> Vec<u8> {
    format!("{:?}", key).into_bytes()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollisionStrategy {
    Chaining,
    OpenAddressing,
}

impl CollisionStrategy {
    fn from_name(name: &str) -> Self {
        match name {
            "chaining" => Self::Chaining,
            "open_addressing" | "open-addressing" | "open" => Self::OpenAddressing,
            other => panic!("unknown collision strategy: {other}"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HashAlgorithm {
    SipHash24,
    Fnv1a32,
    Murmur3_32,
    Djb2,
}

impl HashAlgorithm {
    fn from_name(name: &str) -> Self {
        match name {
            "siphash" | "siphash_2_4" => Self::SipHash24,
            "fnv1a" | "fnv1a_32" => Self::Fnv1a32,
            "murmur3" | "murmur3_32" => Self::Murmur3_32,
            "djb2" => Self::Djb2,
            other => panic!("unknown hash function: {other}"),
        }
    }

    fn hash(self, data: &[u8]) -> u64 {
        match self {
            Self::SipHash24 => siphash_2_4(data, &SIPHASH_KEY),
            Self::Fnv1a32 => fnv1a_32(data) as u64,
            Self::Murmur3_32 => murmur3_32(data) as u64,
            Self::Djb2 => djb2(data),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Slot<K, V> {
    Empty,
    Tombstone,
    Occupied(K, V),
}

#[derive(Clone, Debug)]
pub struct HashMap<K, V> {
    buckets: Vec<Vec<(K, V)>>,
    slots: Vec<Slot<K, V>>,
    strategy: CollisionStrategy,
    size: usize,
    capacity: usize,
    hash_fn: HashAlgorithm,
}

impl<K, V> Default for HashMap<K, V> {
    fn default() -> Self {
        Self::new(DEFAULT_CAPACITY, CollisionStrategy::Chaining)
    }
}

impl<K, V> HashMap<K, V> {
    pub fn new(capacity: usize, strategy: CollisionStrategy) -> Self {
        Self::with_hash_fn(capacity, strategy, HashAlgorithm::SipHash24)
    }

    pub fn with_hash_fn(
        capacity: usize,
        strategy: CollisionStrategy,
        hash_fn: HashAlgorithm,
    ) -> Self {
        let capacity = capacity.max(1);
        let buckets = if matches!(strategy, CollisionStrategy::Chaining) {
            std::iter::repeat_with(Vec::new).take(capacity).collect()
        } else {
            Vec::new()
        };
        let slots = if matches!(strategy, CollisionStrategy::OpenAddressing) {
            std::iter::repeat_with(|| Slot::Empty)
                .take(capacity)
                .collect()
        } else {
            Vec::new()
        };
        Self {
            buckets,
            slots,
            strategy,
            size: 0,
            capacity,
            hash_fn,
        }
    }

    pub fn with_options(
        capacity: usize,
        strategy: impl AsRef<str>,
        hash_fn: impl AsRef<str>,
    ) -> Self {
        Self::with_hash_fn(
            capacity,
            CollisionStrategy::from_name(strategy.as_ref()),
            HashAlgorithm::from_name(hash_fn.as_ref()),
        )
    }

    pub fn strategy(&self) -> CollisionStrategy {
        self.strategy
    }

    pub fn hash_algorithm(&self) -> HashAlgorithm {
        self.hash_fn
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn load_factor(&self) -> f64 {
        self.size as f64 / self.capacity as f64
    }

    pub fn needs_resize(&self) -> bool {
        match self.strategy {
            CollisionStrategy::Chaining => self.load_factor() > CHAINING_RESIZE_THRESHOLD,
            CollisionStrategy::OpenAddressing => {
                self.load_factor() > OPEN_ADDRESSING_RESIZE_THRESHOLD
            }
        }
    }
}

impl<K: Eq + Debug, V> HashMap<K, V> {
    pub fn get(&self, key: &K) -> Option<&V> {
        match self.strategy {
            CollisionStrategy::Chaining => {
                let idx = self.bucket_index(key);
                self.buckets[idx]
                    .iter()
                    .find(|(existing_key, _)| existing_key == key)
                    .map(|(_, value)| value)
            }
            CollisionStrategy::OpenAddressing => {
                let start = self.bucket_index(key);
                for probe in 0..self.capacity {
                    let idx = (start + probe) % self.capacity;
                    match &self.slots[idx] {
                        Slot::Empty => return None,
                        Slot::Tombstone => {}
                        Slot::Occupied(existing_key, value) if existing_key == key => {
                            return Some(value);
                        }
                        Slot::Occupied(_, _) => {}
                    }
                }
                None
            }
        }
    }

    pub fn has(&self, key: &K) -> bool {
        self.get(key).is_some()
    }

    pub fn set(mut self, key: K, value: V) -> Self {
        self.insert_without_resize(key, value);
        if self.needs_resize() {
            self.resize(self.capacity.saturating_mul(2));
        }
        self
    }

    pub fn delete(mut self, key: &K) -> Self {
        let removed = match self.strategy {
            CollisionStrategy::Chaining => self.delete_chaining(key),
            CollisionStrategy::OpenAddressing => self.delete_open_addressing(key),
        };
        if removed {
            // Tombstones are compacted on resize; deletion itself does not shrink.
        }
        self
    }

    pub fn entries(&self) -> Vec<(K, V)>
    where
        K: Clone,
        V: Clone,
    {
        match self.strategy {
            CollisionStrategy::Chaining => self
                .buckets
                .iter()
                .flat_map(|bucket| bucket.iter().cloned())
                .collect(),
            CollisionStrategy::OpenAddressing => self
                .slots
                .iter()
                .filter_map(|slot| match slot {
                    Slot::Occupied(key, value) => Some((key.clone(), value.clone())),
                    _ => None,
                })
                .collect(),
        }
    }

    pub fn keys(&self) -> Vec<K>
    where
        K: Clone,
        V: Clone,
    {
        self.entries().into_iter().map(|(key, _)| key).collect()
    }

    pub fn values(&self) -> Vec<V>
    where
        K: Clone,
        V: Clone,
    {
        self.entries().into_iter().map(|(_, value)| value).collect()
    }

    pub fn into_entries(self) -> Vec<(K, V)> {
        match self.strategy {
            CollisionStrategy::Chaining => self
                .buckets
                .into_iter()
                .flat_map(|bucket| bucket.into_iter())
                .collect(),
            CollisionStrategy::OpenAddressing => self
                .slots
                .into_iter()
                .filter_map(|slot| match slot {
                    Slot::Occupied(key, value) => Some((key, value)),
                    _ => None,
                })
                .collect(),
        }
    }

    pub fn from_entries<I>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
    {
        Self::from_entries_with_options(pairs, DEFAULT_CAPACITY, "chaining", "siphash")
    }

    pub fn from_entries_with_options<I>(
        pairs: I,
        capacity: usize,
        strategy: impl AsRef<str>,
        hash_fn: impl AsRef<str>,
    ) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
    {
        let mut map = Self::with_options(capacity, strategy, hash_fn);
        for (key, value) in pairs {
            map = map.set(key, value);
        }
        map
    }

    fn bucket_index(&self, key: &K) -> usize {
        let data = serialize_key(key);
        (self.hash_fn.hash(&data) as usize) % self.capacity
    }

    fn insert_without_resize(&mut self, key: K, value: V) {
        match self.strategy {
            CollisionStrategy::Chaining => self.insert_chaining(key, value),
            CollisionStrategy::OpenAddressing => self.insert_open_addressing(key, value),
        }
    }

    fn insert_chaining(&mut self, key: K, value: V) {
        let idx = self.bucket_index(&key);
        let bucket = &mut self.buckets[idx];
        if let Some((_, existing_value)) = bucket
            .iter_mut()
            .find(|(existing_key, _)| existing_key == &key)
        {
            *existing_value = value;
            return;
        }
        bucket.push((key, value));
        self.size += 1;
    }

    fn insert_open_addressing(&mut self, key: K, value: V) {
        let start = self.bucket_index(&key);
        let mut first_tombstone: Option<usize> = None;

        for probe in 0..self.capacity {
            let idx = (start + probe) % self.capacity;
            match &self.slots[idx] {
                Slot::Empty => {
                    let insert_at = first_tombstone.unwrap_or(idx);
                    self.slots[insert_at] = Slot::Occupied(key, value);
                    self.size += 1;
                    return;
                }
                Slot::Tombstone => {
                    if first_tombstone.is_none() {
                        first_tombstone = Some(idx);
                    }
                }
                Slot::Occupied(existing_key, _) if existing_key == &key => {
                    if let Slot::Occupied(_, existing_value) = &mut self.slots[idx] {
                        *existing_value = value;
                    }
                    return;
                }
                Slot::Occupied(_, _) => {}
            }
        }

        if let Some(insert_at) = first_tombstone {
            self.slots[insert_at] = Slot::Occupied(key, value);
            self.size += 1;
            return;
        }

        panic!("hash map is full; resize should have happened earlier");
    }

    fn delete_chaining(&mut self, key: &K) -> bool {
        let idx = self.bucket_index(key);
        let bucket = &mut self.buckets[idx];
        let before = bucket.len();
        bucket.retain(|(existing_key, _)| existing_key != key);
        let removed = bucket.len() != before;
        if removed {
            self.size -= 1;
        }
        removed
    }

    fn delete_open_addressing(&mut self, key: &K) -> bool {
        let start = self.bucket_index(key);
        for probe in 0..self.capacity {
            let idx = (start + probe) % self.capacity;
            if matches!(self.slots[idx], Slot::Empty) {
                return false;
            }
            if matches!(&self.slots[idx], Slot::Occupied(existing_key, _) if existing_key == key) {
                self.slots[idx] = Slot::Tombstone;
                self.size -= 1;
                return true;
            }
        }
        false
    }

    fn resize(&mut self, new_capacity: usize) {
        let new_capacity = new_capacity.max(1);
        let entries = self.drain_entries();
        self.capacity = new_capacity;
        self.size = 0;
        self.buckets = if matches!(self.strategy, CollisionStrategy::Chaining) {
            std::iter::repeat_with(Vec::new).take(new_capacity).collect()
        } else {
            Vec::new()
        };
        self.slots = if matches!(self.strategy, CollisionStrategy::OpenAddressing) {
            std::iter::repeat_with(|| Slot::Empty)
                .take(new_capacity)
                .collect()
        } else {
            Vec::new()
        };

        for (key, value) in entries {
            self.insert_without_resize(key, value);
        }
    }

    fn drain_entries(&mut self) -> Vec<(K, V)> {
        match self.strategy {
            CollisionStrategy::Chaining => std::mem::take(&mut self.buckets)
                .into_iter()
                .flat_map(|bucket| bucket.into_iter())
                .collect(),
            CollisionStrategy::OpenAddressing => std::mem::take(&mut self.slots)
                .into_iter()
                .filter_map(|slot| match slot {
                    Slot::Occupied(key, value) => Some((key, value)),
                    _ => None,
                })
                .collect(),
        }
    }
}

pub fn new_map<K, V>(capacity: usize, strategy: impl AsRef<str>, hash_fn: impl AsRef<str>) -> HashMap<K, V> {
    HashMap::with_options(capacity, strategy, hash_fn)
}

pub fn set<K: Eq + Debug, V>(map: HashMap<K, V>, key: K, value: V) -> HashMap<K, V> {
    map.set(key, value)
}

pub fn delete<K: Eq + Debug, V>(map: HashMap<K, V>, key: &K) -> HashMap<K, V> {
    map.delete(key)
}

pub fn get<'a, K: Eq + Debug, V>(map: &'a HashMap<K, V>, key: &K) -> Option<&'a V> {
    map.get(key)
}

pub fn has<K: Eq + Debug, V>(map: &HashMap<K, V>, key: &K) -> bool {
    map.has(key)
}

pub fn keys<K, V>(map: &HashMap<K, V>) -> Vec<K>
where
    K: Eq + Debug + Clone,
    V: Clone,
{
    map.keys()
}

pub fn values<K, V>(map: &HashMap<K, V>) -> Vec<V>
where
    K: Eq + Debug + Clone,
    V: Clone,
{
    map.values()
}

pub fn entries<K, V>(map: &HashMap<K, V>) -> Vec<(K, V)>
where
    K: Eq + Debug + Clone,
    V: Clone,
{
    map.entries()
}

pub fn size<K, V>(map: &HashMap<K, V>) -> usize {
    map.size()
}

pub fn load_factor<K, V>(map: &HashMap<K, V>) -> f64 {
    map.load_factor()
}

pub fn capacity<K, V>(map: &HashMap<K, V>) -> usize {
    map.capacity()
}

pub fn from_entries<K, V, I>(pairs: I) -> HashMap<K, V>
where
    K: Eq + Debug,
    I: IntoIterator<Item = (K, V)>,
{
    HashMap::from_entries(pairs)
}

pub fn merge<K: Eq + Debug, V>(m1: HashMap<K, V>, m2: HashMap<K, V>) -> HashMap<K, V> {
    let strategy = m1.strategy;
    let hash_fn = m1.hash_fn;
    let mut result = HashMap::with_hash_fn(m1.capacity.max(m2.capacity), strategy, hash_fn);
    for (key, value) in m1.into_entries() {
        result = result.set(key, value);
    }
    for (key, value) in m2.into_entries() {
        result = result.set(key, value);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_strategies() -> [CollisionStrategy; 2] {
        [CollisionStrategy::Chaining, CollisionStrategy::OpenAddressing]
    }

    #[test]
    fn default_map_uses_chaining() {
        let map: HashMap<i32, i32> = HashMap::default();
        assert_eq!(map.capacity(), DEFAULT_CAPACITY);
        assert_eq!(map.strategy(), CollisionStrategy::Chaining);
        assert_eq!(map.size(), 0);
    }

    #[test]
    fn set_and_get_work_for_both_strategies() {
        for strategy in all_strategies() {
            let map = HashMap::new(8, strategy).set("hello", 42).set("world", 7);
            assert_eq!(map.get(&"hello"), Some(&42));
            assert_eq!(map.get(&"world"), Some(&7));
            assert!(map.has(&"hello"));
            assert_eq!(map.size(), 2);
        }
    }

    #[test]
    fn set_overwrites_existing_values() {
        for strategy in all_strategies() {
            let map = HashMap::new(4, strategy)
                .set("hello", 42)
                .set("hello", 99);
            assert_eq!(map.get(&"hello"), Some(&99));
            assert_eq!(map.size(), 1);
        }
    }

    #[test]
    fn delete_removes_entries() {
        for strategy in all_strategies() {
            let map = HashMap::new(8, strategy).set("hello", 42).delete(&"hello");
            assert_eq!(map.get(&"hello"), None);
            assert_eq!(map.size(), 0);
        }
    }

    #[test]
    fn collisions_are_handled() {
        let map = HashMap::new(1, CollisionStrategy::OpenAddressing)
            .set("cat", 1)
            .set("car", 2)
            .set("cab", 3);
        assert_eq!(map.size(), 3);
        assert_eq!(map.get(&"cat"), Some(&1));
        assert_eq!(map.get(&"car"), Some(&2));
        assert_eq!(map.get(&"cab"), Some(&3));
    }

    #[test]
    fn resize_happens_at_expected_thresholds() {
        let chaining = (0..5).fold(HashMap::new(4, CollisionStrategy::Chaining), |map, i| {
            map.set(format!("k{i}"), i)
        });
        assert_eq!(chaining.capacity(), 8);
        assert_eq!(chaining.size(), 5);

        let open = (0..4).fold(HashMap::new(4, CollisionStrategy::OpenAddressing), |map, i| {
            map.set(format!("k{i}"), i)
        });
        assert_eq!(open.capacity(), 8);
        assert_eq!(open.size(), 4);
    }

    #[test]
    fn keys_values_and_entries_are_available() {
        let map = HashMap::new(8, CollisionStrategy::Chaining)
            .set("x", 1)
            .set("y", 2)
            .set("z", 3);
        let mut keys = map.keys();
        keys.sort();
        assert_eq!(keys, vec!["x", "y", "z"]);
        let mut values = map.values();
        values.sort();
        assert_eq!(values, vec![1, 2, 3]);
        assert_eq!(map.entries().len(), 3);
    }

    #[test]
    fn from_entries_last_value_wins() {
        let map = HashMap::from_entries([("a", 1), ("a", 2), ("b", 3)]);
        assert_eq!(map.get(&"a"), Some(&2));
        assert_eq!(map.get(&"b"), Some(&3));
    }

    #[test]
    fn merge_prefers_right_hand_values() {
        let left = HashMap::from_entries([("a", 1), ("b", 2)]);
        let right = HashMap::from_entries([("b", 99), ("c", 3)]);
        let merged = merge(left, right);
        assert_eq!(merged.get(&"a"), Some(&1));
        assert_eq!(merged.get(&"b"), Some(&99));
        assert_eq!(merged.get(&"c"), Some(&3));
    }

    #[test]
    fn open_addressing_tombstones_preserve_probe_chains() {
        let map = HashMap::new(8, CollisionStrategy::OpenAddressing)
            .set("cat", 1)
            .set("car", 2)
            .delete(&"cat");
        assert_eq!(map.get(&"car"), Some(&2));
    }

    #[test]
    fn with_options_accepts_aliases_and_hashes() {
        let strategy_cases = [
            ("chaining", CollisionStrategy::Chaining),
            ("open_addressing", CollisionStrategy::OpenAddressing),
            ("open-addressing", CollisionStrategy::OpenAddressing),
            ("open", CollisionStrategy::OpenAddressing),
        ];
        let hash_cases = [
            ("siphash", HashAlgorithm::SipHash24),
            ("siphash_2_4", HashAlgorithm::SipHash24),
            ("fnv1a", HashAlgorithm::Fnv1a32),
            ("fnv1a_32", HashAlgorithm::Fnv1a32),
            ("murmur3", HashAlgorithm::Murmur3_32),
            ("murmur3_32", HashAlgorithm::Murmur3_32),
            ("djb2", HashAlgorithm::Djb2),
        ];

        for (strategy_name, expected_strategy) in strategy_cases {
            for (hash_name, expected_hash) in hash_cases {
                let map = HashMap::<&str, i32>::with_options(2, strategy_name, hash_name)
                    .set("hello", 10);
                assert_eq!(map.strategy(), expected_strategy);
                assert_eq!(map.hash_algorithm(), expected_hash);
                assert_eq!(map.get(&"hello"), Some(&10));
            }
        }
    }

    #[test]
    fn with_options_rejects_unknown_names() {
        assert!(
            std::panic::catch_unwind(|| HashMap::<i32, i32>::with_options(4, "wat", "siphash"))
                .is_err()
        );
        assert!(
            std::panic::catch_unwind(|| HashMap::<i32, i32>::with_options(4, "chaining", "wat"))
                .is_err()
        );
    }

    #[test]
    fn free_function_wrappers_cover_common_paths() {
        let map = new_map::<_, _>(0, "open_addressing", "fnv1a");
        assert_eq!(capacity(&map), 1);
        assert_eq!(load_factor(&map), 0.0);

        let map = set(map, "a", 1);
        let map = set(map, "b", 2);
        assert_eq!(get(&map, &"a"), Some(&1));
        assert!(has(&map, &"b"));
        assert_eq!(size(&map), 2);

        let mut keys = keys(&map);
        keys.sort();
        assert_eq!(keys, vec!["a", "b"]);

        let mut values = values(&map);
        values.sort();
        assert_eq!(values, vec![1, 2]);

        let mut entries = entries(&map);
        entries.sort();
        assert_eq!(entries, vec![("a", 1), ("b", 2)]);

        let map = delete(map, &"a");
        assert!(!has(&map, &"a"));
        assert_eq!(size(&map), 1);

        let left = from_entries([("x", 1), ("y", 2)]);
        let right = from_entries([("y", 99), ("z", 3)]);
        let merged = merge(left, right);
        let mut merged_entries = merged.entries();
        merged_entries.sort();
        assert_eq!(merged_entries, vec![("x", 1), ("y", 99), ("z", 3)]);
    }

    #[test]
    fn into_entries_and_resize_paths_work_for_open_addressing() {
        let map = HashMap::with_hash_fn(2, CollisionStrategy::OpenAddressing, HashAlgorithm::Djb2)
            .set("cat", 1)
            .set("car", 2)
            .set("cab", 3);
        assert!(map.capacity() >= 2);

        let mut entries = map.clone().into_entries();
        entries.sort();
        assert_eq!(entries.len(), 3);
        assert_eq!(map.keys().len(), 3);

        let map = HashMap::with_hash_fn(2, CollisionStrategy::OpenAddressing, HashAlgorithm::Djb2)
            .set("cat", 1)
            .delete(&"cat")
            .set("cab", 3);
        assert_eq!(map.get(&"cab"), Some(&3));
        assert_eq!(map.size(), 1);
    }

    #[test]
    fn load_factor_tracks_size() {
        let map = HashMap::with_hash_fn(4, CollisionStrategy::Chaining, HashAlgorithm::SipHash24)
            .set(1, 10)
            .set(2, 20);
        assert_eq!(map.capacity(), 4);
        assert!((map.load_factor() - 0.5).abs() < f64::EPSILON);
    }
}
