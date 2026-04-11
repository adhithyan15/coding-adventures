//! DT08 AVL Tree.

use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AVLNode<T> {
    pub value: T,
    pub left: Option<Box<AVLNode<T>>>,
    pub right: Option<Box<AVLNode<T>>>,
    pub height: isize,
    size: usize,
}

impl<T> AVLNode<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            left: None,
            right: None,
            height: 0,
            size: 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AVLTree<T> {
    root: Option<Box<AVLNode<T>>>,
}

impl<T> Default for AVLTree<T> {
    fn default() -> Self {
        Self { root: None }
    }
}

impl<T: Ord + Clone> AVLTree<T> {
    pub fn empty() -> Self {
        Self { root: None }
    }

    pub fn root(&self) -> Option<&AVLNode<T>> {
        self.root.as_deref()
    }

    pub fn insert(&self, value: T) -> Self {
        Self {
            root: avl_insert(self.root.clone(), value),
        }
    }

    pub fn delete(&self, value: &T) -> Self {
        Self {
            root: avl_delete(self.root.clone(), value),
        }
    }

    pub fn search(&self, value: &T) -> Option<&AVLNode<T>> {
        avl_search(&self.root, value)
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

    pub fn is_valid_bst(&self) -> bool {
        bst_is_valid(&self.root)
    }

    pub fn balance_factor(&self, node: &AVLNode<T>) -> isize {
        balance_factor(node)
    }

    pub fn is_valid_avl(&self) -> bool {
        is_valid_avl(&self.root)
    }

    pub fn height(&self) -> isize {
        avl_height(&self.root)
    }

    pub fn size(&self) -> usize {
        avl_size(&self.root)
    }
}

pub fn avl_search<'a, T: Ord>(
    root: &'a Option<Box<AVLNode<T>>>,
    value: &T,
) -> Option<&'a AVLNode<T>> {
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

pub fn avl_insert<T: Ord + Clone>(
    root: Option<Box<AVLNode<T>>>,
    value: T,
) -> Option<Box<AVLNode<T>>> {
    match root {
        None => Some(Box::new(AVLNode::new(value))),
        Some(mut node) => {
            match value.cmp(&node.value) {
                Ordering::Less => node.left = avl_insert(node.left.take(), value),
                Ordering::Greater => node.right = avl_insert(node.right.take(), value),
                Ordering::Equal => return Some(node),
            }
            update_metadata(&mut node);
            Some(rebalance(node))
        }
    }
}

pub fn avl_delete<T: Ord + Clone>(
    root: Option<Box<AVLNode<T>>>,
    value: &T,
) -> Option<Box<AVLNode<T>>> {
    let Some(mut node) = root else {
        return None;
    };

    match value.cmp(&node.value) {
        Ordering::Less => node.left = avl_delete(node.left.take(), value),
        Ordering::Greater => node.right = avl_delete(node.right.take(), value),
        Ordering::Equal => match (node.left.take(), node.right.take()) {
            (None, None) => return None,
            (Some(left), None) => return Some(left),
            (None, Some(right)) => return Some(right),
            (Some(left), Some(right)) => {
                let (new_right, successor) = extract_min(right);
                node.value = successor;
                node.left = Some(left);
                node.right = new_right;
            }
        },
    }

    update_metadata(&mut node);
    Some(rebalance(node))
}

pub fn rotate_left<T>(mut root: Box<AVLNode<T>>) -> Box<AVLNode<T>> {
    let Some(mut new_root) = root.right.take() else {
        return root;
    };
    root.right = new_root.left.take();
    update_metadata(&mut root);
    new_root.left = Some(root);
    update_metadata(&mut new_root);
    new_root
}

pub fn rotate_right<T>(mut root: Box<AVLNode<T>>) -> Box<AVLNode<T>> {
    let Some(mut new_root) = root.left.take() else {
        return root;
    };
    root.left = new_root.right.take();
    update_metadata(&mut root);
    new_root.right = Some(root);
    update_metadata(&mut new_root);
    new_root
}

pub fn rebalance<T: Ord + Clone>(mut node: Box<AVLNode<T>>) -> Box<AVLNode<T>> {
    let bf = balance_factor(&node);
    if bf > 1 {
        if let Some(left) = node.left.as_ref() {
            if balance_factor(left) < 0 {
                node.left = node.left.take().map(rotate_left);
            }
        }
        return rotate_right(node);
    }
    if bf < -1 {
        if let Some(right) = node.right.as_ref() {
            if balance_factor(right) > 0 {
                node.right = node.right.take().map(rotate_right);
            }
        }
        return rotate_left(node);
    }
    node
}

pub fn balance_factor<T>(node: &AVLNode<T>) -> isize {
    avl_height(&node.left) - avl_height(&node.right)
}

pub fn avl_height<T>(root: &Option<Box<AVLNode<T>>>) -> isize {
    root.as_deref().map(|node| node.height).unwrap_or(-1)
}

pub fn avl_size<T>(root: &Option<Box<AVLNode<T>>>) -> usize {
    root.as_deref().map(|node| node.size).unwrap_or(0)
}

pub fn is_valid_avl<T: Ord>(root: &Option<Box<AVLNode<T>>>) -> bool {
    validate(root.as_deref(), None, None).is_some()
}

fn validate<'a, T: Ord>(
    node: Option<&'a AVLNode<T>>,
    min: Option<&'a T>,
    max: Option<&'a T>,
) -> Option<(isize, usize)> {
    match node {
        None => Some((-1, 0)),
        Some(node) => {
            if min.is_some_and(|bound| node.value <= *bound) {
                return None;
            }
            if max.is_some_and(|bound| node.value >= *bound) {
                return None;
            }
            let (left_h, left_s) = validate(node.left.as_deref(), min, Some(&node.value))?;
            let (right_h, right_s) = validate(node.right.as_deref(), Some(&node.value), max)?;
            let height = 1 + left_h.max(right_h);
            if node.height != height || node.size != 1 + left_s + right_s {
                return None;
            }
            if (left_h - right_h).abs() > 1 {
                return None;
            }
            Some((height, 1 + left_s + right_s))
        }
    }
}

fn update_metadata<T>(node: &mut Box<AVLNode<T>>) {
    node.height = 1 + avl_height(&node.left).max(avl_height(&node.right));
    node.size = 1 + avl_size(&node.left) + avl_size(&node.right);
}

fn extract_min<T: Ord + Clone>(mut node: Box<AVLNode<T>>) -> (Option<Box<AVLNode<T>>>, T) {
    match node.left.take() {
        None => (node.right.take(), node.value),
        Some(left) => {
            let (new_left, min_value) = extract_min(left);
            node.left = new_left;
            update_metadata(&mut node);
            (Some(rebalance(node)), min_value)
        }
    }
}

fn bst_min_value<T: Ord>(root: &Option<Box<AVLNode<T>>>) -> Option<&T> {
    let mut current = root.as_deref();
    while let Some(node) = current {
        if node.left.is_none() {
            return Some(&node.value);
        }
        current = node.left.as_deref();
    }
    None
}

fn bst_max_value<T: Ord>(root: &Option<Box<AVLNode<T>>>) -> Option<&T> {
    let mut current = root.as_deref();
    while let Some(node) = current {
        if node.right.is_none() {
            return Some(&node.value);
        }
        current = node.right.as_deref();
    }
    None
}

fn bst_predecessor<'a, T: Ord>(
    root: &'a Option<Box<AVLNode<T>>>,
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

fn bst_successor<'a, T: Ord>(
    root: &'a Option<Box<AVLNode<T>>>,
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

fn bst_kth_smallest<'a, T: Ord>(
    root: &'a Option<Box<AVLNode<T>>>,
    k: usize,
) -> Option<&'a T> {
    if k == 0 {
        return None;
    }
    let node = root.as_deref()?;
    let left_size = avl_size(&node.left);
    if k == left_size + 1 {
        Some(&node.value)
    } else if k <= left_size {
        bst_kth_smallest(&node.left, k)
    } else {
        bst_kth_smallest(&node.right, k - left_size - 1)
    }
}

fn bst_rank<T: Ord>(root: &Option<Box<AVLNode<T>>>, value: &T) -> usize {
    match root.as_deref() {
        None => 0,
        Some(node) => match value.cmp(&node.value) {
            Ordering::Less => bst_rank(&node.left, value),
            Ordering::Equal => avl_size(&node.left),
            Ordering::Greater => avl_size(&node.left) + 1 + bst_rank(&node.right, value),
        },
    }
}

fn bst_to_sorted_array<T: Ord + Clone>(root: &Option<Box<AVLNode<T>>>) -> Vec<T> {
    let mut out = Vec::new();
    if let Some(node) = root.as_deref() {
        bst_to_sorted_array_rec(Some(node), &mut out);
    }
    out
}

fn bst_to_sorted_array_rec<T: Ord + Clone>(root: Option<&AVLNode<T>>, out: &mut Vec<T>) {
    if let Some(node) = root {
        bst_to_sorted_array_rec(node.left.as_deref(), out);
        out.push(node.value.clone());
        bst_to_sorted_array_rec(node.right.as_deref(), out);
    }
}

fn bst_is_valid<T: Ord>(root: &Option<Box<AVLNode<T>>>) -> bool {
    validate_bst(root.as_deref(), None, None)
}

fn validate_bst<T: Ord>(node: Option<&AVLNode<T>>, min: Option<&T>, max: Option<&T>) -> bool {
    match node {
        None => true,
        Some(node) => {
            if min.is_some_and(|bound| node.value <= *bound) {
                return false;
            }
            if max.is_some_and(|bound| node.value >= *bound) {
                return false;
            }
            validate_bst(node.left.as_deref(), min, Some(&node.value))
                && validate_bst(node.right.as_deref(), Some(&node.value), max)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotations_rebalance_the_tree() {
        let tree = AVLTree::empty()
            .insert(30)
            .insert(20)
            .insert(10);
        assert_eq!(tree.root().map(|node| &node.value), Some(&20));
        assert!(tree.is_valid_avl());

        let tree = AVLTree::empty()
            .insert(10)
            .insert(20)
            .insert(30);
        assert_eq!(tree.root().map(|node| &node.value), Some(&20));
        assert!(tree.is_valid_avl());
    }

    #[test]
    fn search_and_order_statistics_work() {
        let tree = AVLTree::empty()
            .insert(8)
            .insert(3)
            .insert(10)
            .insert(1)
            .insert(6)
            .insert(14)
            .insert(4)
            .insert(7);
        assert!(tree.contains(&6));
        assert_eq!(tree.min_value(), Some(&1));
        assert_eq!(tree.max_value(), Some(&14));
        assert_eq!(tree.rank(&6), 3);
        assert_eq!(tree.kth_smallest(4), Some(&6));
        assert_eq!(tree.to_sorted_array(), vec![1, 3, 4, 6, 7, 8, 10, 14]);
        assert!(tree.is_valid_bst());
    }
}
