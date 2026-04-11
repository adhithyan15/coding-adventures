//! DT09 Red-Black Tree.

use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    Red,
    Black,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RBNode<T> {
    pub value: T,
    pub color: Color,
    pub left: Option<Box<RBNode<T>>>,
    pub right: Option<Box<RBNode<T>>>,
    size: usize,
}

impl<T> RBNode<T> {
    pub fn new(value: T, color: Color) -> Self {
        Self {
            value,
            color,
            left: None,
            right: None,
            size: 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RBTree<T> {
    root: Option<Box<RBNode<T>>>,
}

impl<T> Default for RBTree<T> {
    fn default() -> Self {
        Self { root: None }
    }
}

impl<T: Ord + Clone> RBTree<T> {
    pub fn empty() -> Self {
        Self { root: None }
    }

    pub fn root(&self) -> Option<&RBNode<T>> {
        self.root.as_deref()
    }

    pub fn insert(&self, value: T) -> Self {
        Self {
            root: rb_insert(self.root.clone(), value),
        }
    }

    pub fn delete(&self, value: &T) -> Self {
        Self {
            root: rb_delete(self.root.clone(), value),
        }
    }

    pub fn search(&self, value: &T) -> Option<&RBNode<T>> {
        rb_search(&self.root, value)
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

    pub fn to_sorted_array(&self) -> Vec<T> {
        bst_to_sorted_array(&self.root)
    }

    pub fn is_valid_rb(&self) -> bool {
        is_valid_rb(&self.root)
    }

    pub fn black_height(&self) -> usize {
        black_height(&self.root)
    }
}

pub fn rb_search<'a, T: Ord>(root: &'a Option<Box<RBNode<T>>>, value: &T) -> Option<&'a RBNode<T>> {
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

pub fn rb_insert<T: Ord + Clone>(
    root: Option<Box<RBNode<T>>>,
    value: T,
) -> Option<Box<RBNode<T>>> {
    let mut root = insert_rec(root, value)?;
    root.color = Color::Black;
    Some(root)
}

pub fn rb_delete<T: Ord + Clone>(
    root: Option<Box<RBNode<T>>>,
    value: &T,
) -> Option<Box<RBNode<T>>> {
    let mut root = delete_rec(root, value)?;
    root.color = Color::Black;
    Some(root)
}

pub fn rb_fix_insert<T: Ord + Clone>(root: Option<Box<RBNode<T>>>) -> Option<Box<RBNode<T>>> {
    root.map(fix_up)
}

pub fn rb_fix_delete<T: Ord + Clone>(root: Option<Box<RBNode<T>>>) -> Option<Box<RBNode<T>>> {
    root.map(fix_up)
}

pub fn rotate_left<T>(mut root: Box<RBNode<T>>) -> Box<RBNode<T>> {
    let Some(mut new_root) = root.right.take() else {
        return root;
    };
    let root_color = root.color;
    root.right = new_root.left.take();
    root.color = Color::Red;
    update_size(&mut root);
    new_root.left = Some(root);
    new_root.color = root_color;
    update_size(&mut new_root);
    new_root
}

pub fn rotate_right<T>(mut root: Box<RBNode<T>>) -> Box<RBNode<T>> {
    let Some(mut new_root) = root.left.take() else {
        return root;
    };
    let root_color = root.color;
    root.left = new_root.right.take();
    root.color = Color::Red;
    update_size(&mut root);
    new_root.right = Some(root);
    new_root.color = root_color;
    update_size(&mut new_root);
    new_root
}

fn insert_rec<T: Ord + Clone>(
    root: Option<Box<RBNode<T>>>,
    value: T,
) -> Option<Box<RBNode<T>>> {
    match root {
        None => Some(Box::new(RBNode::new(value, Color::Red))),
        Some(mut node) => {
            match value.cmp(&node.value) {
                Ordering::Less => node.left = insert_rec(node.left.take(), value),
                Ordering::Greater => node.right = insert_rec(node.right.take(), value),
                Ordering::Equal => return Some(node),
            }
            Some(fix_up(node))
        }
    }
}

fn delete_rec<T: Ord + Clone>(
    root: Option<Box<RBNode<T>>>,
    value: &T,
) -> Option<Box<RBNode<T>>> {
    let Some(mut node) = root else {
        return None;
    };

    if value < &node.value {
        if !is_red(&node.left) && !is_red_left(&node.left) {
            node = move_red_left(node);
        }
        node.left = delete_rec(node.left.take(), value);
    } else {
        if is_red(&node.left) {
            node = rotate_right(node);
        }
        if value == &node.value && node.right.is_none() {
            return None;
        }
        if !is_red(&node.right) && !is_red_left(&node.right) {
            node = move_red_right(node);
        }
        if value == &node.value {
            let right = node.right.take().expect("right child exists");
            let (successor, new_right) = delete_min(right);
            node.value = successor;
            node.right = new_right;
        } else {
            node.right = delete_rec(node.right.take(), value);
        }
    }

    Some(fix_up(node))
}

fn delete_min<T: Ord + Clone>(
    mut node: Box<RBNode<T>>,
) -> (T, Option<Box<RBNode<T>>>) {
    if node.left.is_none() {
        return (node.value, node.right.take());
    }

    if !is_red(&node.left) && !is_red_left(&node.left) {
        node = move_red_left(node);
    }
    let left = node.left.take().expect("left child exists");
    let (min_value, new_left) = delete_min(left);
    node.left = new_left;
    let node = fix_up(node);
    (min_value, Some(node))
}

fn move_red_left<T: Ord + Clone>(mut node: Box<RBNode<T>>) -> Box<RBNode<T>> {
    flip_colors(&mut node);
    if is_red_left(&node.right) {
        if let Some(right) = node.right.take() {
            node.right = Some(rotate_right(right));
        }
        node = rotate_left(node);
        flip_colors(&mut node);
    }
    node
}

fn move_red_right<T: Ord + Clone>(mut node: Box<RBNode<T>>) -> Box<RBNode<T>> {
    flip_colors(&mut node);
    if is_red_left(&node.left) {
        node = rotate_right(node);
        flip_colors(&mut node);
    }
    node
}

fn fix_up<T: Ord + Clone>(mut node: Box<RBNode<T>>) -> Box<RBNode<T>> {
    if is_red(&node.right) && !is_red(&node.left) {
        node = rotate_left(node);
    }
    if is_red(&node.left) && is_red_left(&node.left) {
        node = rotate_right(node);
    }
    if is_red(&node.left) && is_red(&node.right) {
        flip_colors(&mut node);
    }
    update_size(&mut node);
    node
}

fn flip_colors<T>(node: &mut Box<RBNode<T>>) {
    node.color = match node.color {
        Color::Red => Color::Black,
        Color::Black => Color::Red,
    };
    if let Some(left) = node.left.as_mut() {
        left.color = match left.color {
            Color::Red => Color::Black,
            Color::Black => Color::Red,
        };
    }
    if let Some(right) = node.right.as_mut() {
        right.color = match right.color {
            Color::Red => Color::Black,
            Color::Black => Color::Red,
        };
    }
}

fn is_red<T>(node: &Option<Box<RBNode<T>>>) -> bool {
    matches!(node.as_deref().map(|node| node.color), Some(Color::Red))
}

fn is_red_left<T>(node: &Option<Box<RBNode<T>>>) -> bool {
    node.as_deref()
        .and_then(|node| node.left.as_deref())
        .map(|node| node.color == Color::Red)
        .unwrap_or(false)
}

fn update_size<T>(node: &mut Box<RBNode<T>>) {
    node.size = 1 + rb_size(&node.left) + rb_size(&node.right);
}

fn rb_size<T>(root: &Option<Box<RBNode<T>>>) -> usize {
    root.as_deref().map(|node| node.size).unwrap_or(0)
}

pub fn black_height<T>(root: &Option<Box<RBNode<T>>>) -> usize {
    let mut height = 0;
    let mut current = root.as_deref();
    while let Some(node) = current {
        if node.color == Color::Black {
            height += 1;
        }
        current = node.left.as_deref();
    }
    height
}

pub fn is_valid_rb<T: Ord>(root: &Option<Box<RBNode<T>>>) -> bool {
    if let Some(node) = root.as_deref() {
        if node.color != Color::Black {
            return false;
        }
    }
    validate(root.as_deref(), None, None).is_some()
}

fn validate<'a, T: Ord>(
    node: Option<&'a RBNode<T>>,
    min: Option<&'a T>,
    max: Option<&'a T>,
) -> Option<usize> {
    match node {
        None => Some(1),
        Some(node) => {
            if min.is_some_and(|bound| node.value <= *bound) {
                return None;
            }
            if max.is_some_and(|bound| node.value >= *bound) {
                return None;
            }
            if node.color == Color::Red {
                if is_red(&node.left) || is_red(&node.right) {
                    return None;
                }
            }
            let left = validate(node.left.as_deref(), min, Some(&node.value))?;
            let right = validate(node.right.as_deref(), Some(&node.value), max)?;
            if left != right {
                return None;
            }
            let expected_size = 1 + rb_size(&node.left) + rb_size(&node.right);
            if expected_size != node.size {
                return None;
            }
            Some(left + usize::from(node.color == Color::Black))
        }
    }
}

fn bst_min_value<T: Ord>(root: &Option<Box<RBNode<T>>>) -> Option<&T> {
    let mut current = root.as_deref();
    while let Some(node) = current {
        if node.left.is_none() {
            return Some(&node.value);
        }
        current = node.left.as_deref();
    }
    None
}

fn bst_max_value<T: Ord>(root: &Option<Box<RBNode<T>>>) -> Option<&T> {
    let mut current = root.as_deref();
    while let Some(node) = current {
        if node.right.is_none() {
            return Some(&node.value);
        }
        current = node.right.as_deref();
    }
    None
}

fn bst_predecessor<'a, T: Ord>(root: &'a Option<Box<RBNode<T>>>, value: &T) -> Option<&'a T> {
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

fn bst_successor<'a, T: Ord>(root: &'a Option<Box<RBNode<T>>>, value: &T) -> Option<&'a T> {
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
    root: &'a Option<Box<RBNode<T>>>,
    k: usize,
) -> Option<&'a T> {
    if k == 0 {
        return None;
    }
    let node = root.as_deref()?;
    let left_size = rb_size(&node.left);
    if k == left_size + 1 {
        Some(&node.value)
    } else if k <= left_size {
        bst_kth_smallest(&node.left, k)
    } else {
        bst_kth_smallest(&node.right, k - left_size - 1)
    }
}

fn bst_to_sorted_array<T: Ord + Clone>(root: &Option<Box<RBNode<T>>>) -> Vec<T> {
    let mut out = Vec::new();
    bst_inorder(root, &mut out);
    out
}

fn bst_inorder<T: Ord + Clone>(root: &Option<Box<RBNode<T>>>, out: &mut Vec<T>) {
    if let Some(node) = root.as_deref() {
        bst_inorder(&node.left, out);
        out.push(node.value.clone());
        bst_inorder(&node.right, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_search_and_delete_work() {
        let tree = RBTree::empty()
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
        assert_eq!(tree.kth_smallest(4), Some(&6));
        assert!(tree.is_valid_rb());
        let tree = tree.delete(&3);
        assert!(!tree.contains(&3));
        assert!(tree.is_valid_rb());
    }

    #[test]
    fn black_height_and_sorted_output_work() {
        let tree = RBTree::empty().insert(2).insert(1).insert(3);
        assert!(tree.black_height() >= 1);
        assert_eq!(tree.to_sorted_array(), vec![1, 2, 3]);
    }
}
