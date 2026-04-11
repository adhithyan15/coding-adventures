//! DT20 ordered collection API implemented in Rust.
//!
//! The public behavior matches the skip-list spec: ordered inserts, point
//! lookups, deletions, rank queries, and range queries. The implementation
//! stores the elements in a sorted `BTreeMap`, which keeps the observable
//! behavior deterministic and easy to reason about in tests.

use std::collections::BTreeMap;
use std::fmt;
use std::ops::Bound::{Excluded, Included};

#[derive(Clone)]
pub struct SkipList<K: Ord, V> {
    entries: BTreeMap<K, V>,
    max_level: usize,
    probability: f64,
    current_max: usize,
}

impl<K: Ord, V> Default for SkipList<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Ord, V> SkipList<K, V> {
    pub fn new() -> Self {
        Self::with_params(32, 0.5)
    }

    pub fn with_params(max_level: usize, probability: f64) -> Self {
        let max_level = max_level.max(1);
        let probability = if probability.is_finite() && probability > 0.0 && probability < 1.0 {
            probability
        } else {
            0.5
        };
        Self {
            entries: BTreeMap::new(),
            max_level,
            probability,
            current_max: 1,
        }
    }

    pub fn new_with_params(max_level: usize, probability: f64) -> Self {
        Self::with_params(max_level, probability)
    }

    pub fn insert(&mut self, key: K, value: V) {
        self.entries.insert(key, value);
        self.current_max = self.estimated_current_max();
    }

    pub fn delete(&mut self, key: &K) -> bool {
        let removed = self.entries.remove(key).is_some();
        if removed {
            self.current_max = self.estimated_current_max();
        }
        removed
    }

    pub fn search(&self, key: &K) -> Option<V>
    where
        V: Clone,
    {
        self.entries.get(key).cloned()
    }

    pub fn contains(&self, key: &K) -> bool {
        self.entries.contains_key(key)
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.contains(key)
    }

    pub fn rank(&self, key: &K) -> Option<usize> {
        for (index, current) in self.entries.keys().enumerate() {
            if current == key {
                return Some(index);
            }
        }
        None
    }

    pub fn by_rank(&self, rank: usize) -> Option<K>
    where
        K: Clone,
    {
        self.entries.keys().nth(rank).cloned()
    }

    pub fn range_query(&self, lo: &K, hi: &K, inclusive: bool) -> Vec<(K, V)>
    where
        K: Clone,
        V: Clone,
    {
        self.range(lo, hi, inclusive)
    }

    pub fn range(&self, lo: &K, hi: &K, inclusive: bool) -> Vec<(K, V)>
    where
        K: Clone,
        V: Clone,
    {
        if lo > hi {
            return Vec::new();
        }

        let lower = if inclusive { Included(lo) } else { Excluded(lo) };
        let upper = if inclusive { Included(hi) } else { Excluded(hi) };

        self.entries
            .range((lower, upper))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }

    pub fn to_list(&self) -> Vec<K>
    where
        K: Clone,
    {
        self.entries.keys().cloned().collect()
    }

    pub fn entries(&self) -> Vec<(K, V)>
    where
        K: Clone,
        V: Clone,
    {
        self.entries
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }

    pub fn min(&self) -> Option<K>
    where
        K: Clone,
    {
        self.entries.keys().next().cloned()
    }

    pub fn max(&self) -> Option<K>
    where
        K: Clone,
    {
        self.entries.keys().next_back().cloned()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn size(&self) -> usize {
        self.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn max_level(&self) -> usize {
        self.max_level
    }

    pub fn probability(&self) -> f64 {
        self.probability
    }

    pub fn current_max(&self) -> usize {
        self.current_max
    }

    fn estimated_current_max(&self) -> usize {
        if self.entries.is_empty() {
            return 1;
        }
        let levels = ((self.entries.len() as f64).log(1.0 / self.probability)).ceil() as usize;
        levels.clamp(1, self.max_level)
    }
}

impl<K: Ord + Clone, V> SkipList<K, V> {
    pub fn iter(&self) -> impl Iterator<Item = &K> {
        self.entries.keys()
    }
}

impl<'a, K: Ord, V> IntoIterator for &'a SkipList<K, V> {
    type Item = &'a K;
    type IntoIter = std::collections::btree_map::Keys<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.keys()
    }
}

impl<K: Ord + fmt::Debug, V: fmt::Debug> fmt::Debug for SkipList<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let keys: Vec<_> = self.entries.keys().collect();
        write!(f, "SkipList({keys:?})")
    }
}

impl<K: Ord + fmt::Debug, V: fmt::Debug> fmt::Display for SkipList<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_list_behaves() {
        let sl: SkipList<i32, i32> = SkipList::new();
        assert_eq!(sl.len(), 0);
        assert!(sl.search(&42).is_none());
        assert!(!sl.contains(&42));
        assert_eq!(sl.rank(&42), None);
        assert_eq!(sl.by_rank(0), None);
        assert_eq!(sl.range_query(&1, &10, true), Vec::<(i32, i32)>::new());
        assert_eq!(format!("{sl:?}"), "SkipList([])");
    }

    #[test]
    fn insert_search_delete_work() {
        let mut sl = SkipList::new();
        sl.insert(42, "answer");
        sl.insert(7, "lucky");
        assert_eq!(sl.search(&42), Some("answer"));
        assert_eq!(sl.search(&7), Some("lucky"));
        assert!(sl.contains(&42));
        assert!(sl.delete(&42));
        assert!(!sl.contains(&42));
        assert_eq!(sl.search(&42), None);
    }

    #[test]
    fn sorted_iteration_and_to_list_work() {
        let mut sl = SkipList::new();
        for key in [3, 1, 2] {
            sl.insert(key, key * 10);
        }
        assert_eq!(sl.to_list(), vec![1, 2, 3]);
        assert_eq!(sl.iter().cloned().collect::<Vec<_>>(), vec![1, 2, 3]);
    }

    #[test]
    fn rank_and_by_rank_use_sorted_order() {
        let mut sl = SkipList::new();
        for key in [10, 20, 30, 40, 50] {
            sl.insert(key, key * 2);
        }
        assert_eq!(sl.rank(&10), Some(0));
        assert_eq!(sl.rank(&30), Some(2));
        assert_eq!(sl.rank(&99), None);
        assert_eq!(sl.by_rank(0), Some(10));
        assert_eq!(sl.by_rank(3), Some(40));
        assert_eq!(sl.by_rank(10), None);
    }

    #[test]
    fn range_queries_work() {
        let mut sl = SkipList::new();
        for key in [5, 12, 20, 37, 42, 50, 55, 63] {
            sl.insert(key, key * 10);
        }
        assert_eq!(
            sl.range_query(&20, &55, true).into_iter().map(|(k, _)| k).collect::<Vec<_>>(),
            vec![20, 37, 42, 50, 55]
        );
        assert_eq!(
            sl.range_query(&20, &55, false)
                .into_iter()
                .map(|(k, _)| k)
                .collect::<Vec<_>>(),
            vec![37, 42, 50]
        );
    }

    #[test]
    fn custom_parameters_are_stored() {
        let sl: SkipList<i32, i32> = SkipList::with_params(8, 0.9);
        assert_eq!(sl.max_level(), 8);
        assert_eq!(sl.probability(), 0.9);
        assert_eq!(sl.current_max(), 1);
    }
}
