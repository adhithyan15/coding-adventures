use core::cmp::Ordering;
use core::fmt;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use hyperloglog::HyperLogLog;

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SortedSet {
    by_score: BTreeMap<OrderedF64, BTreeSet<Vec<u8>>>,
    by_member: BTreeMap<Vec<u8>, OrderedF64>,
}

impl SortedSet {
    pub fn new() -> Self {
        Self {
            by_score: BTreeMap::new(),
            by_member: BTreeMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.by_member.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_member.is_empty()
    }

    pub fn contains(&self, member: &[u8]) -> bool {
        self.by_member.contains_key(member)
    }

    pub fn score(&self, member: &[u8]) -> Option<f64> {
        self.by_member.get(member).map(|score| score.0)
    }

    pub fn insert(&mut self, score: f64, member: Vec<u8>) -> bool {
        let score = OrderedF64::new(score).expect("sorted set score cannot be NaN");
        let is_new = !self.by_member.contains_key(&member);
        if let Some(old_score) = self.by_member.insert(member.clone(), score) {
            if let Some(set) = self.by_score.get_mut(&old_score) {
                set.remove(&member);
                if set.is_empty() {
                    self.by_score.remove(&old_score);
                }
            }
        }
        self.by_score.entry(score).or_default().insert(member);
        is_new
    }

    pub fn remove(&mut self, member: &[u8]) -> bool {
        let Some(old_score) = self.by_member.remove(member) else {
            return false;
        };
        if let Some(set) = self.by_score.get_mut(&old_score) {
            set.remove(member);
            if set.is_empty() {
                self.by_score.remove(&old_score);
            }
        }
        true
    }

    pub fn rank(&self, member: &[u8]) -> Option<usize> {
        let mut index = 0usize;
        for members in self.by_score.values() {
            for current in members {
                if current.as_slice() == member {
                    return Some(index);
                }
                index += 1;
            }
        }
        None
    }

    pub fn ordered_entries(&self) -> Vec<(Vec<u8>, f64)> {
        let mut out = Vec::with_capacity(self.len());
        for (score, members) in &self.by_score {
            for member in members {
                out.push((member.clone(), score.0));
            }
        }
        out
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntryValue {
    String(Vec<u8>),
    Hash(BTreeMap<Vec<u8>, Vec<u8>>),
    List(VecDeque<Vec<u8>>),
    Set(BTreeSet<Vec<u8>>),
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

    pub fn hash(value: BTreeMap<Vec<u8>, Vec<u8>>, expires_at: Option<u64>) -> Self {
        Self::new(EntryValue::Hash(value), expires_at)
    }

    pub fn list(value: VecDeque<Vec<u8>>, expires_at: Option<u64>) -> Self {
        Self::new(EntryValue::List(value), expires_at)
    }

    pub fn set(value: BTreeSet<Vec<u8>>, expires_at: Option<u64>) -> Self {
        Self::new(EntryValue::Set(value), expires_at)
    }

    pub fn zset(value: SortedSet, expires_at: Option<u64>) -> Self {
        Self::new(EntryValue::ZSet(value), expires_at)
    }

    pub fn hll(value: HyperLogLog, expires_at: Option<u64>) -> Self {
        Self::new(EntryValue::Hll(value), expires_at)
    }
}
