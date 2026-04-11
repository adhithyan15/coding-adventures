//! DT10 Treap.

use std::cmp::Ordering;
use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};

static PRIORITY_SEED: AtomicU32 = AtomicU32::new(0x9E37_79B9);

#[derive(Clone, Debug, PartialEq)]
pub struct TreapNode<K> {
    pub key: K,
    pub priority: f64,
    pub left: Option<Box<TreapNode<K>>>,
    pub right: Option<Box<TreapNode<K>>>,
    size: usize,
}

impl<K> TreapNode<K> {
    pub fn new(key: K, priority: f64) -> Self {
        Self {
            key,
            priority,
            left: None,
            right: None,
            size: 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Treap<K> {
    root: Option<Box<TreapNode<K>>>,
}

impl<K> Default for Treap<K> {
    fn default() -> Self {
        Self { root: None }
    }
}

impl<K: Ord + Clone> Treap<K> {
    pub fn empty() -> Self {
        Self { root: None }
    }

    pub fn root(&self) -> Option<&TreapNode<K>> {
        self.root.as_deref()
    }

    pub fn insert(&self, key: K, priority: Option<f64>) -> Self {
        Self {
            root: treap_insert(self.root.clone(), key, priority),
        }
    }

    pub fn delete(&self, key: &K) -> Self {
        Self {
            root: treap_delete(self.root.clone(), key),
        }
    }

    pub fn search(&self, key: &K) -> Option<&TreapNode<K>> {
        treap_search(&self.root, key)
    }

    pub fn contains(&self, key: &K) -> bool {
        self.search(key).is_some()
    }

    pub fn split(&self, key: &K) -> (Self, Self) {
        let (left, right) = treap_split(self.root.clone(), key);
        (Self { root: left }, Self { root: right })
    }

    pub fn merge(left: Self, right: Self) -> Self {
        Self {
            root: treap_merge(left.root, right.root),
        }
    }

    pub fn min_key(&self) -> Option<&K> {
        treap_min_key(&self.root)
    }

    pub fn max_key(&self) -> Option<&K> {
        treap_max_key(&self.root)
    }

    pub fn predecessor(&self, key: &K) -> Option<&K> {
        bst_predecessor(&self.root, key)
    }

    pub fn successor(&self, key: &K) -> Option<&K> {
        bst_successor(&self.root, key)
    }

    pub fn kth_smallest(&self, k: usize) -> Option<&K> {
        bst_kth_smallest(&self.root, k)
    }

    pub fn to_sorted_array(&self) -> Vec<K> {
        bst_to_sorted_array(&self.root)
    }

    pub fn is_valid_treap(&self) -> bool {
        is_valid_treap(&self.root)
    }

    pub fn height(&self) -> isize {
        treap_height(&self.root)
    }

    pub fn size(&self) -> usize {
        treap_size(&self.root)
    }
}

pub fn treap_search<'a, K: Ord>(
    root: &'a Option<Box<TreapNode<K>>>,
    key: &K,
) -> Option<&'a TreapNode<K>> {
    let mut current = root.as_deref();
    while let Some(node) = current {
        match key.cmp(&node.key) {
            Ordering::Less => current = node.left.as_deref(),
            Ordering::Greater => current = node.right.as_deref(),
            Ordering::Equal => return Some(node),
        }
    }
    None
}

pub fn treap_insert<K: Ord + Clone>(
    root: Option<Box<TreapNode<K>>>,
    key: K,
    priority: Option<f64>,
) -> Option<Box<TreapNode<K>>> {
    let priority = priority.unwrap_or_else(next_priority);
    insert_rec(root, key, priority)
}

pub fn treap_delete<K: Ord + Clone>(
    root: Option<Box<TreapNode<K>>>,
    key: &K,
) -> Option<Box<TreapNode<K>>> {
    delete_rec(root, key)
}

pub fn treap_split<K: Ord + Clone>(
    root: Option<Box<TreapNode<K>>>,
    key: &K,
) -> (Option<Box<TreapNode<K>>>, Option<Box<TreapNode<K>>>) {
    match root {
        None => (None, None),
        Some(mut node) => match key.cmp(&node.key) {
            Ordering::Less => {
                let (left, right) = treap_split(node.left.take(), key);
                node.left = right;
                update_metadata(&mut node);
                (left, Some(node))
            }
            Ordering::Equal | Ordering::Greater => {
                let (left, right) = treap_split(node.right.take(), key);
                node.right = left;
                update_metadata(&mut node);
                (Some(node), right)
            }
        },
    }
}

pub fn treap_merge<K: Ord + Clone>(
    left: Option<Box<TreapNode<K>>>,
    right: Option<Box<TreapNode<K>>>,
) -> Option<Box<TreapNode<K>>> {
    match (left, right) {
        (None, other) => other,
        (other, None) => other,
        (Some(mut left), Some(mut right)) => {
            if left.priority >= right.priority {
                left.right = treap_merge(left.right.take(), Some(right));
                update_metadata(&mut left);
                Some(left)
            } else {
                right.left = treap_merge(Some(left), right.left.take());
                update_metadata(&mut right);
                Some(right)
            }
        }
    }
}

pub fn treap_min_key<K: Ord>(root: &Option<Box<TreapNode<K>>>) -> Option<&K> {
    let mut current = root.as_deref();
    while let Some(node) = current {
        if node.left.is_none() {
            return Some(&node.key);
        }
        current = node.left.as_deref();
    }
    None
}

pub fn treap_max_key<K: Ord>(root: &Option<Box<TreapNode<K>>>) -> Option<&K> {
    let mut current = root.as_deref();
    while let Some(node) = current {
        if node.right.is_none() {
            return Some(&node.key);
        }
        current = node.right.as_deref();
    }
    None
}

pub fn treap_height<K>(root: &Option<Box<TreapNode<K>>>) -> isize {
    match root.as_deref() {
        None => -1,
        Some(node) => 1 + treap_height(&node.left).max(treap_height(&node.right)),
    }
}

pub fn treap_size<K>(root: &Option<Box<TreapNode<K>>>) -> usize {
    root.as_deref().map(|node| node.size).unwrap_or(0)
}

pub fn is_valid_treap<K: Ord>(root: &Option<Box<TreapNode<K>>>) -> bool {
    validate(root.as_deref(), None, None, None).is_some()
}

fn validate<'a, K: Ord>(
    node: Option<&'a TreapNode<K>>,
    min: Option<&'a K>,
    max: Option<&'a K>,
    parent_priority: Option<f64>,
) -> Option<usize> {
    match node {
        None => Some(0),
        Some(node) => {
            if min.is_some_and(|bound| node.key <= *bound) {
                return None;
            }
            if max.is_some_and(|bound| node.key > *bound) {
                return None;
            }
            if parent_priority.is_some_and(|priority| node.priority > priority) {
                return None;
            }
            let left = validate(
                node.left.as_deref(),
                min,
                Some(&node.key),
                Some(node.priority),
            )?;
            let right = validate(
                node.right.as_deref(),
                Some(&node.key),
                max,
                Some(node.priority),
            )?;
            if node.size != 1 + left + right {
                return None;
            }
            Some(1 + left + right)
        }
    }
}

fn insert_rec<K: Ord + Clone>(
    root: Option<Box<TreapNode<K>>>,
    key: K,
    priority: f64,
) -> Option<Box<TreapNode<K>>> {
    match root {
        None => Some(Box::new(TreapNode::new(key, priority))),
        Some(mut node) => {
            match key.cmp(&node.key) {
                Ordering::Less => {
                    node.left = insert_rec(node.left.take(), key, priority);
                    if node
                        .left
                        .as_ref()
                        .is_some_and(|left| left.priority > node.priority)
                    {
                        node = rotate_right(node);
                    }
                }
                Ordering::Greater => {
                    node.right = insert_rec(node.right.take(), key, priority);
                    if node
                        .right
                        .as_ref()
                        .is_some_and(|right| right.priority > node.priority)
                    {
                        node = rotate_left(node);
                    }
                }
                Ordering::Equal => return Some(node),
            }
            update_metadata(&mut node);
            Some(node)
        }
    }
}

fn delete_rec<K: Ord + Clone>(
    root: Option<Box<TreapNode<K>>>,
    key: &K,
) -> Option<Box<TreapNode<K>>> {
    let Some(mut node) = root else {
        return None;
    };

    match key.cmp(&node.key) {
        Ordering::Less => {
            node.left = delete_rec(node.left.take(), key);
            update_metadata(&mut node);
            Some(node)
        }
        Ordering::Greater => {
            node.right = delete_rec(node.right.take(), key);
            update_metadata(&mut node);
            Some(node)
        }
        Ordering::Equal => treap_merge(node.left.take(), node.right.take()),
    }
}

fn rotate_left<K>(mut root: Box<TreapNode<K>>) -> Box<TreapNode<K>> {
    let Some(mut new_root) = root.right.take() else {
        return root;
    };
    root.right = new_root.left.take();
    update_metadata(&mut root);
    new_root.left = Some(root);
    update_metadata(&mut new_root);
    new_root
}

fn rotate_right<K>(mut root: Box<TreapNode<K>>) -> Box<TreapNode<K>> {
    let Some(mut new_root) = root.left.take() else {
        return root;
    };
    root.left = new_root.right.take();
    update_metadata(&mut root);
    new_root.right = Some(root);
    update_metadata(&mut new_root);
    new_root
}

fn update_metadata<K>(node: &mut Box<TreapNode<K>>) {
    node.size = 1 + treap_size(&node.left) + treap_size(&node.right);
}

fn next_priority() -> f64 {
    let mut state = PRIORITY_SEED.fetch_add(0x9E37_79B9, AtomicOrdering::Relaxed);
    state ^= state >> 13;
    state ^= state << 17;
    state ^= state >> 5;
    let mixed = state.wrapping_mul(0x85EB_CA6B);
    (mixed as f64) / (u32::MAX as f64)
}

fn bst_predecessor<'a, K: Ord>(root: &'a Option<Box<TreapNode<K>>>, key: &K) -> Option<&'a K> {
    let mut current = root.as_deref();
    let mut best = None;
    while let Some(node) = current {
        match key.cmp(&node.key) {
            Ordering::Less | Ordering::Equal => current = node.left.as_deref(),
            Ordering::Greater => {
                best = Some(&node.key);
                current = node.right.as_deref();
            }
        }
    }
    best
}

fn bst_successor<'a, K: Ord>(root: &'a Option<Box<TreapNode<K>>>, key: &K) -> Option<&'a K> {
    let mut current = root.as_deref();
    let mut best = None;
    while let Some(node) = current {
        match key.cmp(&node.key) {
            Ordering::Greater | Ordering::Equal => current = node.right.as_deref(),
            Ordering::Less => {
                best = Some(&node.key);
                current = node.left.as_deref();
            }
        }
    }
    best
}

fn bst_kth_smallest<'a, K: Ord>(root: &'a Option<Box<TreapNode<K>>>, k: usize) -> Option<&'a K> {
    if k == 0 {
        return None;
    }
    let node = root.as_deref()?;
    let left_size = treap_size(&node.left);
    if k == left_size + 1 {
        Some(&node.key)
    } else if k <= left_size {
        bst_kth_smallest(&node.left, k)
    } else {
        bst_kth_smallest(&node.right, k - left_size - 1)
    }
}

fn bst_to_sorted_array<K: Ord + Clone>(root: &Option<Box<TreapNode<K>>>) -> Vec<K> {
    let mut out = Vec::new();
    if let Some(node) = root.as_deref() {
        bst_inorder(Some(node), &mut out);
    }
    out
}

fn bst_inorder<K: Ord + Clone>(root: Option<&TreapNode<K>>, out: &mut Vec<K>) {
    if let Some(node) = root {
        bst_inorder(node.left.as_deref(), out);
        out.push(node.key.clone());
        bst_inorder(node.right.as_deref(), out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_merge_and_search_work() {
        let treap = Treap::empty()
            .insert(8, Some(0.8))
            .insert(3, Some(0.7))
            .insert(10, Some(0.6))
            .insert(1, Some(0.9))
            .insert(6, Some(0.5));
        assert!(treap.contains(&6));
        let (left, right) = treap.split(&6);
        assert!(left.to_sorted_array().iter().all(|k| *k <= 6));
        assert!(right.to_sorted_array().iter().all(|k| *k > 6));
        let merged = Treap::merge(left, right);
        assert!(merged.is_valid_treap());
        assert_eq!(merged.to_sorted_array(), vec![1, 3, 6, 8, 10]);
    }

    #[test]
    fn delete_and_order_statistics_work() {
        let treap = Treap::empty()
            .insert(8, Some(0.8))
            .insert(3, Some(0.7))
            .insert(10, Some(0.6))
            .insert(1, Some(0.9))
            .insert(6, Some(0.5))
            .insert(14, Some(0.4))
            .insert(4, Some(0.3))
            .insert(7, Some(0.2));
        assert_eq!(treap.min_key(), Some(&1));
        assert_eq!(treap.max_key(), Some(&14));
        assert_eq!(treap.kth_smallest(4), Some(&6));
        assert!(treap.is_valid_treap());
        let treap = treap.delete(&3);
        assert!(!treap.contains(&3));
        assert!(treap.is_valid_treap());
    }
}
