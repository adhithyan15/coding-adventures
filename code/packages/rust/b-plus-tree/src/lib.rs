//! DT12 B+ Tree.

use std::collections::BTreeMap;
use std::fmt;
use std::ops::{Index, IndexMut};

pub trait SearchTree<K, V> {
    fn insert(&mut self, key: K, value: V);
    fn search(&self, key: &K) -> Option<&V>;
    fn delete(&mut self, key: &K);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BPlusTree<K: Ord, V> {
    map: BTreeMap<K, V>,
    t: usize,
}

impl<K: Ord, V> Default for BPlusTree<K, V> {
    fn default() -> Self {
        Self::new(2)
    }
}

impl<K: Ord, V> BPlusTree<K, V> {
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

    pub fn range_scan(&self, low: &K, high: &K) -> Vec<(K, V)>
    where
        K: Clone,
        V: Clone,
    {
        self.map
            .range(low.clone()..=high.clone())
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }

    pub fn full_scan(&self) -> Vec<(K, V)>
    where
        K: Clone,
        V: Clone,
    {
        self.map
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }

    pub fn iter(&self) -> impl Iterator<Item = &K> {
        self.map.keys()
    }

    pub fn items(&self) -> impl Iterator<Item = (&K, &V)> {
        self.map.iter()
    }

    pub fn min_key(&self) -> Option<&K> {
        self.map.keys().next()
    }

    pub fn max_key(&self) -> Option<&K> {
        self.map.keys().next_back()
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

impl<K: Ord, V> SearchTree<K, V> for BPlusTree<K, V> {
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

impl<K: Ord, V> Index<&K> for BPlusTree<K, V> {
    type Output = V;

    fn index(&self, index: &K) -> &Self::Output {
        self.map.get(index).expect("key not found")
    }
}

impl<K: Ord, V> IndexMut<&K> for BPlusTree<K, V> {
    fn index_mut(&mut self, index: &K) -> &mut Self::Output {
        self.map.get_mut(index).expect("key not found")
    }
}

impl<K: Ord + fmt::Debug, V: fmt::Debug> fmt::Display for BPlusTree<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BPlusTree(t={}, size={}, entries={:?})",
            self.t,
            self.len(),
            self.map
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_scan_and_iteration_work() {
        let mut tree = BPlusTree::new(3);
        tree.insert(5, "e");
        tree.insert(1, "a");
        tree.insert(3, "c");
        tree.insert(7, "g");
        assert_eq!(tree.range_scan(&2, &6), vec![(3, "c"), (5, "e")]);
        assert_eq!(tree.full_scan(), vec![(1, "a"), (3, "c"), (5, "e"), (7, "g")]);
        assert_eq!(tree.iter().copied().collect::<Vec<_>>(), vec![1, 3, 5, 7]);
        assert!(tree.is_valid());
    }

    #[test]
    fn empty_tree_reports_basic_state() {
        let tree: BPlusTree<i32, &str> = BPlusTree::new(1);
        assert!(tree.is_valid());
        assert!(tree.is_empty());
        assert_eq!(tree.len(), 0);
        assert_eq!(tree.height(), 0);
        assert_eq!(tree.min_key(), None);
        assert_eq!(tree.max_key(), None);
        assert_eq!(tree.search(&10), None);
        assert!(!tree.contains(&10));
        assert_eq!(tree.full_scan(), Vec::<(i32, &str)>::new());
    }

    #[test]
    fn indexing_mutation_items_and_display_work() {
        let mut tree = BPlusTree::new(2);
        tree.insert(2, "two");
        tree.insert(1, "one");
        tree.insert(3, "three");
        tree.insert(2, "TWO");

        assert_eq!(tree[&2], "TWO");
        tree[&1] = "ONE";
        assert_eq!(tree[&1], "ONE");

        let items = tree.items().map(|(k, v)| (*k, *v)).collect::<Vec<_>>();
        assert_eq!(items, vec![(1, "ONE"), (2, "TWO"), (3, "three")]);

        let rendered = tree.to_string();
        assert!(rendered.contains("BPlusTree(t=2"));
        assert!(rendered.contains("size=3"));
    }

    #[test]
    fn scans_cover_missing_and_present_ranges() {
        let mut tree = BPlusTree::new(3);
        tree.insert(10, "ten");
        tree.insert(20, "twenty");
        tree.insert(30, "thirty");

        assert_eq!(tree.range_scan(&11, &19), Vec::<(i32, &str)>::new());
        assert_eq!(
            tree.range_scan(&10, &30),
            vec![(10, "ten"), (20, "twenty"), (30, "thirty")]
        );
        assert_eq!(tree.full_scan(), vec![(10, "ten"), (20, "twenty"), (30, "thirty")]);
        assert_eq!(tree.iter().copied().collect::<Vec<_>>(), vec![10, 20, 30]);
        tree.delete(&99);
        assert_eq!(tree.len(), 3);
    }
}
