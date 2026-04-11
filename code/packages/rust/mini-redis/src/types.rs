use core::cmp::Ordering;
use core::fmt;
use std::collections::VecDeque;

use hash_map::HashMap as DtHashMap;
use hash_set::HashSet as DtHashSet;
use hyperloglog::HyperLogLog;
use skip_list::SkipList;

#[derive(Clone, Copy, Debug)]
pub struct OrderedF64(pub f64);

impl OrderedF64 {
    pub fn new(value: f64) -> Option<Self> {
        if value.is_nan() {
            None
        } else {
            Some(Self(value))
        }
    }
}

impl PartialEq for OrderedF64 {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for OrderedF64 {}

impl PartialOrd for OrderedF64 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedF64 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntryType {
    String,
    Hash,
    List,
    Set,
    ZSet,
    Hll,
}

impl EntryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Hash => "hash",
            Self::List => "list",
            Self::Set => "set",
            Self::ZSet => "zset",
            Self::Hll => "hll",
        }
    }
}

impl fmt::Display for EntryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct SortedEntry {
    score: OrderedF64,
    member: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct SortedSet {
    members: DtHashMap<Vec<u8>, OrderedF64>,
    ordering: SkipList<SortedEntry, ()>,
}

impl SortedSet {
    pub fn new() -> Self {
        Self {
            members: DtHashMap::default(),
            ordering: SkipList::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.members.size()
    }

    pub fn is_empty(&self) -> bool {
        self.members.size() == 0
    }

    pub fn contains(&self, member: &[u8]) -> bool {
        self.members.has(&member.to_vec())
    }

    pub fn score(&self, member: &[u8]) -> Option<f64> {
        self.members.get(&member.to_vec()).map(|score| score.0)
    }

    pub fn insert(&mut self, score: f64, member: Vec<u8>) -> bool {
        let score = OrderedF64::new(score).expect("sorted set score cannot be NaN");
        let is_new = !self.members.has(&member);
        if let Some(old_score) = self.members.get(&member).copied() {
            let _ = self.ordering.delete(&SortedEntry {
                score: old_score,
                member: member.clone(),
            });
        }

        let members = std::mem::take(&mut self.members);
        self.members = members.set(member.clone(), score);
        self.ordering.insert(SortedEntry { score, member }, ());
        is_new
    }

    pub fn remove(&mut self, member: &[u8]) -> bool {
        let member = member.to_vec();
        let Some(old_score) = self.members.get(&member).copied() else {
            return false;
        };

        let _ = self.ordering.delete(&SortedEntry {
            score: old_score,
            member: member.clone(),
        });

        let members = std::mem::take(&mut self.members);
        self.members = members.delete(&member);
        true
    }

    pub fn rank(&self, member: &[u8]) -> Option<usize> {
        let member = member.to_vec();
        for (index, entry) in self.ordering.iter().enumerate() {
            if entry.member == member {
                return Some(index);
            }
        }
        None
    }

    pub fn ordered_entries(&self) -> Vec<(Vec<u8>, f64)> {
        self.ordering
            .iter()
            .map(|entry| (entry.member.clone(), entry.score.0))
            .collect()
    }

    pub fn range_by_index(&self, start: isize, end: isize) -> Vec<(Vec<u8>, f64)> {
        let entries = self.ordered_entries();
        if entries.is_empty() {
            return Vec::new();
        }
        let len = entries.len() as isize;
        let start = if start < 0 { len + start } else { start };
        let end = if end < 0 { len + end } else { end };
        if start < 0 || end < 0 || start >= len || start > end {
            return Vec::new();
        }
        entries[start as usize..=end as usize].to_vec()
    }

    pub fn range_by_score(&self, min: f64, max: f64) -> Vec<(Vec<u8>, f64)> {
        let min = OrderedF64::new(min).expect("sorted set score cannot be NaN");
        let max = OrderedF64::new(max).expect("sorted set score cannot be NaN");
        self.ordered_entries()
            .into_iter()
            .filter(|(_, score)| {
                let score = OrderedF64(*score);
                score >= min && score <= max
            })
            .collect()
    }
}

impl Default for SortedSet {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for SortedSet {
    fn eq(&self, other: &Self) -> bool {
        self.ordered_entries() == other.ordered_entries()
    }
}

impl Eq for SortedSet {}

#[derive(Clone, Debug)]
pub enum EntryValue {
    String(Vec<u8>),
    Hash(DtHashMap<Vec<u8>, Vec<u8>>),
    List(VecDeque<Vec<u8>>),
    Set(DtHashSet<Vec<u8>>),
    ZSet(SortedSet),
    Hll(HyperLogLog),
}

impl EntryValue {
    pub fn entry_type(&self) -> EntryType {
        match self {
            Self::String(_) => EntryType::String,
            Self::Hash(_) => EntryType::Hash,
            Self::List(_) => EntryType::List,
            Self::Set(_) => EntryType::Set,
            Self::ZSet(_) => EntryType::ZSet,
            Self::Hll(_) => EntryType::Hll,
        }
    }
}

impl PartialEq for EntryValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::String(left), Self::String(right)) => left == right,
            (Self::Hash(left), Self::Hash(right)) => hash_map_equal(left, right),
            (Self::List(left), Self::List(right)) => left == right,
            (Self::Set(left), Self::Set(right)) => hash_set_equal(left, right),
            (Self::ZSet(left), Self::ZSet(right)) => left == right,
            (Self::Hll(left), Self::Hll(right)) => left == right,
            _ => false,
        }
    }
}

impl Eq for EntryValue {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub entry_type: EntryType,
    pub value: EntryValue,
    pub expires_at: Option<u64>,
}

impl Entry {
    pub fn new(value: EntryValue, expires_at: Option<u64>) -> Self {
        let entry_type = value.entry_type();
        Self {
            entry_type,
            value,
            expires_at,
        }
    }

    pub fn string(value: impl Into<Vec<u8>>, expires_at: Option<u64>) -> Self {
        Self::new(EntryValue::String(value.into()), expires_at)
    }

    pub fn hash(value: DtHashMap<Vec<u8>, Vec<u8>>, expires_at: Option<u64>) -> Self {
        Self::new(EntryValue::Hash(value), expires_at)
    }

    pub fn list(value: VecDeque<Vec<u8>>, expires_at: Option<u64>) -> Self {
        Self::new(EntryValue::List(value), expires_at)
    }

    pub fn set(value: DtHashSet<Vec<u8>>, expires_at: Option<u64>) -> Self {
        Self::new(EntryValue::Set(value), expires_at)
    }

    pub fn zset(value: SortedSet, expires_at: Option<u64>) -> Self {
        Self::new(EntryValue::ZSet(value), expires_at)
    }

    pub fn hll(value: HyperLogLog, expires_at: Option<u64>) -> Self {
        Self::new(EntryValue::Hll(value), expires_at)
    }
}

fn hash_map_equal<K, V>(left: &DtHashMap<K, V>, right: &DtHashMap<K, V>) -> bool
where
    K: Eq + fmt::Debug + Clone,
    V: PartialEq + Clone,
{
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

fn hash_set_equal<T>(left: &DtHashSet<T>, right: &DtHashSet<T>) -> bool
where
    T: Eq + fmt::Debug + Clone,
{
    if left.size() != right.size() {
        return false;
    }
    left.to_list()
        .into_iter()
        .all(|value| right.contains(&value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    #[test]
    fn sorted_set_and_entry_helpers_cover_all_variants() {
        assert!(OrderedF64::new(f64::NAN).is_none());
        assert!(OrderedF64::new(-1.0).unwrap() < OrderedF64::new(0.0).unwrap());

        assert_eq!(EntryType::String.as_str(), "string");
        assert_eq!(EntryType::Hash.to_string(), "hash");
        assert_eq!(EntryType::List.to_string(), "list");
        assert_eq!(EntryType::Set.to_string(), "set");
        assert_eq!(EntryType::ZSet.to_string(), "zset");
        assert_eq!(EntryType::Hll.to_string(), "hll");

        let mut zset = SortedSet::new();
        assert!(zset.is_empty());
        assert_eq!(zset.len(), 0);
        assert!(!zset.contains(b"missing"));
        assert_eq!(zset.score(b"missing"), None);
        assert_eq!(zset.rank(b"missing"), None);

        assert!(zset.insert(2.0, b"b".to_vec()));
        assert!(zset.insert(1.0, b"a".to_vec()));
        assert!(!zset.insert(3.0, b"b".to_vec()));
        assert_eq!(zset.len(), 2);
        assert!(zset.contains(b"a"));
        assert_eq!(zset.score(b"b"), Some(3.0));
        assert_eq!(
            zset.ordered_entries(),
            vec![(b"a".to_vec(), 1.0), (b"b".to_vec(), 3.0)]
        );
        assert_eq!(zset.rank(b"a"), Some(0));
        assert_eq!(zset.rank(b"b"), Some(1));
        assert_eq!(
            zset.range_by_index(0, 1),
            vec![(b"a".to_vec(), 1.0), (b"b".to_vec(), 3.0)]
        );
        assert_eq!(zset.range_by_index(-1, -1), vec![(b"b".to_vec(), 3.0)]);
        assert!(zset.range_by_index(3, 4).is_empty());
        assert_eq!(
            zset.range_by_score(1.0, 3.0),
            vec![(b"a".to_vec(), 1.0), (b"b".to_vec(), 3.0)]
        );
        assert!(zset.remove(b"a"));
        assert!(!zset.remove(b"missing"));
        assert_eq!(zset.len(), 1);

        let mut hash = DtHashMap::default();
        hash = hash.set(b"k".to_vec(), b"v".to_vec());
        let mut same_hash = DtHashMap::default();
        same_hash = same_hash.set(b"k".to_vec(), b"v".to_vec());
        let mut other_hash = DtHashMap::default();
        other_hash = other_hash.set(b"other".to_vec(), b"v".to_vec());

        let mut set = DtHashSet::default();
        set = set.add(b"a".to_vec());
        let mut same_set = DtHashSet::default();
        same_set = same_set.add(b"a".to_vec());
        let mut other_set = DtHashSet::default();
        other_set = other_set.add(b"b".to_vec());

        let mut list = VecDeque::new();
        list.push_back(b"x".to_vec());

        let string_entry = Entry::new(EntryValue::String(b"payload".to_vec()), None);
        assert_eq!(string_entry.entry_type, EntryType::String);
        assert_eq!(string_entry.value.entry_type(), EntryType::String);

        let hash_entry = Entry::hash(hash.clone(), None);
        let list_entry = Entry::list(list.clone(), Some(10));
        let set_entry = Entry::set(set.clone(), None);
        let zset_entry = Entry::zset(zset.clone(), None);
        let hll_entry = Entry::hll(HyperLogLog::new(), None);

        assert_eq!(hash_entry.entry_type, EntryType::Hash);
        assert_eq!(list_entry.entry_type, EntryType::List);
        assert_eq!(set_entry.entry_type, EntryType::Set);
        assert_eq!(zset_entry.entry_type, EntryType::ZSet);
        assert_eq!(hll_entry.entry_type, EntryType::Hll);

        assert_eq!(Entry::hash(same_hash, None), hash_entry);
        assert_ne!(Entry::hash(other_hash, None), hash_entry);
        assert_eq!(Entry::set(same_set, None), set_entry);
        assert_ne!(Entry::set(other_set, None), set_entry);
        assert_eq!(Entry::list(list, Some(10)), list_entry);
    }
}
