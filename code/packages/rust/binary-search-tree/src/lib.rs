//! DT07 Binary Search Tree.
//!
//! The tree is exposed both as an owned `BST` wrapper and as free functions on
//! `Option<Box<BSTNode<T>>>` to match the composition-oriented Rust API in the
//! specification.

use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BSTNode<T> {
    pub value: T,
    pub left: Option<Box<BSTNode<T>>>,
    pub right: Option<Box<BSTNode<T>>>,
    size: usize,
}

impl<T> BSTNode<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            left: None,
            right: None,
            size: 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BST<T> {
    root: Option<Box<BSTNode<T>>>,
}

impl<T> Default for BST<T> {
    fn default() -> Self {
        Self { root: None }
    }
}

impl<T: Ord + Clone> BST<T> {
    pub fn empty() -> Self {
        Self { root: None }
    }

    pub fn from_sorted_array(array: impl Into<Vec<T>>) -> Self {
        let values = array.into();
        Self {
            root: build_balanced(&values),
        }
    }

    pub fn root(&self) -> Option<&BSTNode<T>> {
        self.root.as_deref()
    }

    pub fn insert(&self, value: T) -> Self {
        Self {
            root: bst_insert(self.root.clone(), value),
        }
    }

    pub fn delete(&self, value: &T) -> Self {
        Self {
            root: bst_delete(self.root.clone(), value),
        }
    }

    pub fn search(&self, value: &T) -> Option<&BSTNode<T>> {
        bst_search(&self.root, value)
    }

    pub fn contains(&self, value: &T) -> bool {
        self.search(value).is_some()
    }

    pub fn min_value(&self) -> Option<&T> {
        bst_min_value(&self.root)
    }

    pub fn max_value(&self) -> Option<&T> {
        bst_max_value(&self.root)
    }

    pub fn predecessor(&self, value: &T) -> Option<&T> {
        bst_predecessor(&self.root, value)
    }

    pub fn successor(&self, value: &T) -> Option<&T> {
        bst_successor(&self.root, value)
    }

    pub fn kth_smallest(&self, k: usize) -> Option<&T> {
        bst_kth_smallest(&self.root, k)
    }

    pub fn rank(&self, value: &T) -> usize {
        bst_rank(&self.root, value)
    }

    pub fn to_sorted_array(&self) -> Vec<T> {
        bst_to_sorted_array(&self.root)
    }

    pub fn is_valid(&self) -> bool {
        bst_is_valid(&self.root)
    }

    pub fn height(&self) -> isize {
        bst_height(&self.root)
    }

    pub fn size(&self) -> usize {
        bst_size(&self.root)
    }
}

pub fn bst_search<'a, T: Ord>(
    root: &'a Option<Box<BSTNode<T>>>,
    value: &T,
) -> Option<&'a BSTNode<T>> {
    let mut current = root.as_deref();
    while let Some(node) = current {
        match value.cmp(&node.value) {
            Ordering::Less => current = node.left.as_deref(),
            Ordering::Greater => current = node.right.as_deref(),
            Ordering::Equal => return Some(node),
        }
    }
    None
}

pub fn bst_insert<T: Ord + Clone>(
    root: Option<Box<BSTNode<T>>>,
    value: T,
) -> Option<Box<BSTNode<T>>> {
    match root {
        None => Some(Box::new(BSTNode::new(value))),
        Some(mut node) => {
            match value.cmp(&node.value) {
                Ordering::Less => {
                    node.left = bst_insert(node.left.take(), value);
                }
                Ordering::Greater => {
                    node.right = bst_insert(node.right.take(), value);
                }
                Ordering::Equal => return Some(node),
            }
            update_size(&mut node);
            Some(node)
        }
    }
}

pub fn bst_delete<T: Ord + Clone>(
    root: Option<Box<BSTNode<T>>>,
    value: &T,
) -> Option<Box<BSTNode<T>>> {
    let Some(mut node) = root else {
        return None;
    };

    match value.cmp(&node.value) {
        Ordering::Less => {
            node.left = bst_delete(node.left.take(), value);
            update_size(&mut node);
            Some(node)
        }
        Ordering::Greater => {
            node.right = bst_delete(node.right.take(), value);
            update_size(&mut node);
            Some(node)
        }
        Ordering::Equal => match (node.left.take(), node.right.take()) {
            (None, None) => None,
            (Some(left), None) => Some(left),
            (None, Some(right)) => Some(right),
            (Some(left), Some(right)) => {
                let (new_right, successor) = extract_min(right);
                node.value = successor;
                node.left = Some(left);
                node.right = new_right;
                update_size(&mut node);
                Some(node)
            }
        },
    }
}

pub fn bst_min_value<T: Ord>(root: &Option<Box<BSTNode<T>>>) -> Option<&T> {
    let mut current = root.as_deref();
    while let Some(node) = current {
        if node.left.is_none() {
            return Some(&node.value);
        }
        current = node.left.as_deref();
    }
    None
}

pub fn bst_max_value<T: Ord>(root: &Option<Box<BSTNode<T>>>) -> Option<&T> {
    let mut current = root.as_deref();
    while let Some(node) = current {
        if node.right.is_none() {
            return Some(&node.value);
        }
        current = node.right.as_deref();
    }
    None
}

pub fn bst_predecessor<'a, T: Ord>(
    root: &'a Option<Box<BSTNode<T>>>,
    value: &T,
) -> Option<&'a T> {
    let mut current = root.as_deref();
    let mut best = None;
    while let Some(node) = current {
        match value.cmp(&node.value) {
            Ordering::Less | Ordering::Equal => current = node.left.as_deref(),
            Ordering::Greater => {
                best = Some(&node.value);
                current = node.right.as_deref();
            }
        }
    }
    best
}

pub fn bst_successor<'a, T: Ord>(
    root: &'a Option<Box<BSTNode<T>>>,
    value: &T,
) -> Option<&'a T> {
    let mut current = root.as_deref();
    let mut best = None;
    while let Some(node) = current {
        match value.cmp(&node.value) {
            Ordering::Greater | Ordering::Equal => current = node.right.as_deref(),
            Ordering::Less => {
                best = Some(&node.value);
                current = node.left.as_deref();
            }
        }
    }
    best
}

pub fn bst_kth_smallest<'a, T: Ord>(
    root: &'a Option<Box<BSTNode<T>>>,
    k: usize,
) -> Option<&'a T> {
    if k == 0 {
        return None;
    }
    let node = root.as_deref()?;
    let left_size = bst_size(&node.left);
    if k == left_size + 1 {
        Some(&node.value)
    } else if k <= left_size {
        bst_kth_smallest(&node.left, k)
    } else {
        bst_kth_smallest(&node.right, k - left_size - 1)
    }
}

pub fn bst_rank<T: Ord>(root: &Option<Box<BSTNode<T>>>, value: &T) -> usize {
    match root.as_deref() {
        None => 0,
        Some(node) => match value.cmp(&node.value) {
            Ordering::Less => bst_rank(&node.left, value),
            Ordering::Equal => bst_size(&node.left),
            Ordering::Greater => bst_size(&node.left) + 1 + bst_rank(&node.right, value),
        },
    }
}

pub fn bst_to_sorted_array<T: Ord + Clone>(root: &Option<Box<BSTNode<T>>>) -> Vec<T> {
    let mut out = Vec::new();
    bst_inorder(root, &mut out);
    out
}

pub fn bst_inorder<T: Ord + Clone>(root: &Option<Box<BSTNode<T>>>, out: &mut Vec<T>) {
    if let Some(node) = root.as_deref() {
        bst_inorder(&node.left, out);
        out.push(node.value.clone());
        bst_inorder(&node.right, out);
    }
}

pub fn bst_height<T>(root: &Option<Box<BSTNode<T>>>) -> isize {
    match root.as_deref() {
        None => -1,
        Some(node) => 1 + bst_height(&node.left).max(bst_height(&node.right)),
    }
}

pub fn bst_size<T>(root: &Option<Box<BSTNode<T>>>) -> usize {
    root.as_deref().map(|node| node.size).unwrap_or(0)
}

pub fn bst_is_valid<T: Ord>(root: &Option<Box<BSTNode<T>>>) -> bool {
    validate(root.as_deref(), None, None)
}

fn validate<T: Ord>(node: Option<&BSTNode<T>>, min: Option<&T>, max: Option<&T>) -> bool {
    match node {
        None => true,
        Some(node) => {
            if min.is_some_and(|bound| node.value <= *bound) {
                return false;
            }
            if max.is_some_and(|bound| node.value >= *bound) {
                return false;
            }
            validate(node.left.as_deref(), min, Some(&node.value))
                && validate(node.right.as_deref(), Some(&node.value), max)
        }
    }
}

fn update_size<T>(node: &mut Box<BSTNode<T>>) {
    node.size = 1 + bst_size(&node.left) + bst_size(&node.right);
}

fn extract_min<T: Ord + Clone>(mut node: Box<BSTNode<T>>) -> (Option<Box<BSTNode<T>>>, T) {
    match node.left.take() {
        None => (node.right.take(), node.value),
        Some(left) => {
            let (new_left, min_value) = extract_min(left);
            node.left = new_left;
            update_size(&mut node);
            (Some(node), min_value)
        }
    }
}

fn build_balanced<T: Clone>(values: &[T]) -> Option<Box<BSTNode<T>>> {
    if values.is_empty() {
        return None;
    }
    let mid = values.len() / 2;
    let mut node = Box::new(BSTNode::new(values[mid].clone()));
    node.left = build_balanced(&values[..mid]);
    node.right = build_balanced(&values[mid + 1..]);
    update_size(&mut node);
    Some(node)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_search_and_delete_work() {
        let tree = BST::empty()
            .insert(8)
            .insert(3)
            .insert(10)
            .insert(1)
            .insert(6)
            .insert(14)
            .insert(4)
            .insert(7);
        assert!(tree.contains(&4));
        assert_eq!(tree.search(&4).map(|node| node.value), Some(4));
        assert_eq!(tree.min_value(), Some(&1));
        assert_eq!(tree.max_value(), Some(&14));
        assert_eq!(tree.rank(&6), 3);
        assert_eq!(tree.kth_smallest(4), Some(&6));
        let tree = tree.delete(&3);
        assert!(!tree.contains(&3));
        assert!(tree.is_valid());
    }

    #[test]
    fn from_sorted_array_builds_balanced_tree() {
        let tree = BST::from_sorted_array(vec![1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(tree.to_sorted_array(), vec![1, 2, 3, 4, 5, 6, 7]);
        assert!(tree.height() <= 2);
    }
}
