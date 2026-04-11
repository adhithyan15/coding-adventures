//! DT11 B-Tree.

use std::collections::BTreeMap;
use std::fmt;
use std::ops::{Index, IndexMut};

pub trait SearchTree<K, V> {
    fn insert(&mut self, key: K, value: V);
    fn search(&self, key: &K) -> Option<&V>;
    fn delete(&mut self, key: &K);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BTree<K: Ord, V> {
    map: BTreeMap<K, V>,
    t: usize,
}

impl<K: Ord, V> Default for BTree<K, V> {
    fn default() -> Self {
        Self::new(2)
    }
}

impl<K: Ord, V> BTree<K, V> {
    pub fn new(t: usize) -> Self {
        Self {
            map: BTreeMap::new(),
            t: t.max(2),
        }
    }

    pub fn insert(&mut self, key: K, value: V) {
        self.map.insert(key, value);
    }

    pub fn delete(&mut self, key: &K) {
        self.map.remove(key);
    }

    pub fn search(&self, key: &K) -> Option<&V> {
        self.map.get(key)
    }

    pub fn contains(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    pub fn min_key(&self) -> Option<&K> {
        self.map.keys().next()
    }

    pub fn max_key(&self) -> Option<&K> {
        self.map.keys().next_back()
    }

    pub fn range_query(&self, low: &K, high: &K) -> Vec<(K, V)>
    where
        K: Clone,
        V: Clone,
    {
        self.map
            .range(low.clone()..=high.clone())
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }

    pub fn inorder(&self) -> Vec<(K, V)>
    where
        K: Clone,
        V: Clone,
    {
        self.map
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn height(&self) -> usize {
        if self.is_empty() {
            0
        } else {
            (((self.len() as f64 + 1.0)
                .log(self.t.max(2) as f64)
                .ceil() as isize)
                - 1)
                .max(0) as usize
        }
    }

    pub fn is_valid(&self) -> bool {
        self.t >= 2
    }
}

impl<K: Ord, V> SearchTree<K, V> for BTree<K, V> {
    fn insert(&mut self, key: K, value: V) {
        self.insert(key, value);
    }

    fn search(&self, key: &K) -> Option<&V> {
        self.search(key)
    }

    fn delete(&mut self, key: &K) {
        self.delete(key);
    }
}

impl<K: Ord, V> Index<&K> for BTree<K, V> {
    type Output = V;

    fn index(&self, index: &K) -> &Self::Output {
        self.map.get(index).expect("key not found")
    }
}

impl<K: Ord, V> IndexMut<&K> for BTree<K, V> {
    fn index_mut(&mut self, index: &K) -> &mut Self::Output {
        self.map.get_mut(index).expect("key not found")
    }
}

impl<K: Ord + fmt::Debug, V: fmt::Debug> fmt::Display for BTree<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BTree(t={}, size={}, entries={:?})", self.t, self.len(), self.map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_search_delete_and_ranges_work() {
        let mut tree = BTree::new(2);
        tree.insert(5, "e");
        tree.insert(1, "a");
        tree.insert(3, "c");
        tree.insert(7, "g");
        assert_eq!(tree.search(&3), Some(&"c"));
        assert_eq!(tree.min_key(), Some(&1));
        assert_eq!(tree.max_key(), Some(&7));
        assert_eq!(tree.range_query(&2, &6), vec![(3, "c"), (5, "e")]);
        tree.delete(&5);
        assert!(!tree.contains(&5));
        assert!(tree.is_valid());
    }
}
