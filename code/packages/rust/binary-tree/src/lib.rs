//! DT03 Binary Tree.
//!
//! This crate provides a generic binary tree plus traversal and shape
//! predicates that are reused by the search-tree family.

use std::collections::VecDeque;
use std::fmt::{Debug, Write};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BinaryTreeNode<T> {
    pub value: T,
    pub left: Option<Box<BinaryTreeNode<T>>>,
    pub right: Option<Box<BinaryTreeNode<T>>>,
}

impl<T> BinaryTreeNode<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            left: None,
            right: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BinaryTree<T> {
    root: Option<Box<BinaryTreeNode<T>>>,
}

impl<T> Default for BinaryTree<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> BinaryTree<T> {
    pub fn new() -> Self {
        Self { root: None }
    }

    pub fn with_root(root: Option<BinaryTreeNode<T>>) -> Self {
        Self {
            root: root.map(Box::new),
        }
    }

    pub fn from_level_order(values: impl IntoIterator<Item = Option<T>>) -> Self
    where
        T: Clone,
    {
        let values: Vec<Option<T>> = values.into_iter().collect();
        Self {
            root: build_from_level_order(&values, 0),
        }
    }

    pub fn root(&self) -> Option<&BinaryTreeNode<T>> {
        self.root.as_deref()
    }

    pub fn is_full(&self) -> bool {
        is_full(self.root())
    }

    pub fn is_complete(&self) -> bool {
        is_complete(self.root())
    }

    pub fn is_perfect(&self) -> bool {
        is_perfect(self.root())
    }

    pub fn height(&self) -> isize {
        height(self.root())
    }

    pub fn size(&self) -> usize {
        size(self.root())
    }

    pub fn left_child(&self, value: &T) -> Option<&BinaryTreeNode<T>>
    where
        T: PartialEq,
    {
        left_child(&self.root, value)
    }

    pub fn right_child(&self, value: &T) -> Option<&BinaryTreeNode<T>>
    where
        T: PartialEq,
    {
        right_child(&self.root, value)
    }

    pub fn inorder(&self) -> Vec<T>
    where
        T: Clone,
    {
        inorder(self.root())
    }

    pub fn preorder(&self) -> Vec<T>
    where
        T: Clone,
    {
        preorder(self.root())
    }

    pub fn postorder(&self) -> Vec<T>
    where
        T: Clone,
    {
        postorder(self.root())
    }

    pub fn level_order(&self) -> Vec<T>
    where
        T: Clone,
    {
        level_order(self.root())
    }

    pub fn to_array(&self) -> Vec<Option<T>>
    where
        T: Clone,
    {
        to_array(self.root())
    }

    pub fn to_ascii(&self) -> String
    where
        T: Debug,
    {
        to_ascii(self.root())
    }
}

pub fn find<'a, T: PartialEq>(
    root: &'a Option<Box<BinaryTreeNode<T>>>,
    value: &T,
) -> Option<&'a BinaryTreeNode<T>> {
    match root.as_deref() {
        None => None,
        Some(node) if &node.value == value => Some(node),
        Some(node) => find(&node.left, value).or_else(|| find(&node.right, value)),
    }
}

pub fn left_child<'a, T: PartialEq>(
    root: &'a Option<Box<BinaryTreeNode<T>>>,
    value: &T,
) -> Option<&'a BinaryTreeNode<T>> {
    find(root, value).and_then(|node| node.left.as_deref())
}

pub fn right_child<'a, T: PartialEq>(
    root: &'a Option<Box<BinaryTreeNode<T>>>,
    value: &T,
) -> Option<&'a BinaryTreeNode<T>> {
    find(root, value).and_then(|node| node.right.as_deref())
}

pub fn inorder<T: Clone>(root: Option<&BinaryTreeNode<T>>) -> Vec<T> {
    let mut out = Vec::new();
    inorder_recursive(root, &mut out);
    out
}

pub fn preorder<T: Clone>(root: Option<&BinaryTreeNode<T>>) -> Vec<T> {
    let mut out = Vec::new();
    preorder_recursive(root, &mut out);
    out
}

pub fn postorder<T: Clone>(root: Option<&BinaryTreeNode<T>>) -> Vec<T> {
    let mut out = Vec::new();
    postorder_recursive(root, &mut out);
    out
}

pub fn level_order<T: Clone>(root: Option<&BinaryTreeNode<T>>) -> Vec<T> {
    let mut out = Vec::new();
    let mut queue = VecDeque::new();
    queue.push_back(root);
    while let Some(node) = queue.pop_front() {
        if let Some(node) = node {
            out.push(node.value.clone());
            queue.push_back(node.left.as_deref());
            queue.push_back(node.right.as_deref());
        }
    }
    out
}

pub fn height<T>(root: Option<&BinaryTreeNode<T>>) -> isize {
    match root {
        None => -1,
        Some(node) => 1 + height(node.left.as_deref()).max(height(node.right.as_deref())),
    }
}

pub fn size<T>(root: Option<&BinaryTreeNode<T>>) -> usize {
    match root {
        None => 0,
        Some(node) => 1 + size(node.left.as_deref()) + size(node.right.as_deref()),
    }
}

pub fn is_full<T>(root: Option<&BinaryTreeNode<T>>) -> bool {
    match root {
        None => true,
        Some(node) => match (node.left.as_deref(), node.right.as_deref()) {
            (None, None) => true,
            (Some(left), Some(right)) => is_full(Some(left)) && is_full(Some(right)),
            _ => false,
        },
    }
}

pub fn is_complete<T>(root: Option<&BinaryTreeNode<T>>) -> bool {
    let mut queue = VecDeque::new();
    queue.push_back(root);
    let mut seen_none = false;

    while let Some(item) = queue.pop_front() {
        match item {
            None => seen_none = true,
            Some(node) => {
                if seen_none {
                    return false;
                }
                queue.push_back(node.left.as_deref());
                queue.push_back(node.right.as_deref());
            }
        }
    }

    true
}

pub fn is_perfect<T>(root: Option<&BinaryTreeNode<T>>) -> bool {
    let h = height(root);
    let n = size(root);
    if h < 0 {
        return n == 0;
    }
    n == ((1usize << ((h as usize) + 1)) - 1)
}

pub fn to_array<T: Clone>(root: Option<&BinaryTreeNode<T>>) -> Vec<Option<T>> {
    let h = height(root);
    if h < 0 {
        return Vec::new();
    }

    let len = (1usize << ((h as usize) + 1)) - 1;
    let mut out = vec![None; len];
    fill_array(root, 0, &mut out);
    out
}

pub fn to_ascii<T: Debug>(root: Option<&BinaryTreeNode<T>>) -> String {
    let mut out = String::new();
    if let Some(node) = root {
        render_ascii(node, "", true, &mut out);
    }
    out
}

fn build_from_level_order<T: Clone>(
    values: &[Option<T>],
    index: usize,
) -> Option<Box<BinaryTreeNode<T>>> {
    let Some(Some(value)) = values.get(index) else {
        return None;
    };
    Some(Box::new(BinaryTreeNode {
        value: value.clone(),
        left: build_from_level_order(values, 2 * index + 1),
        right: build_from_level_order(values, 2 * index + 2),
    }))
}

fn inorder_recursive<T: Clone>(root: Option<&BinaryTreeNode<T>>, out: &mut Vec<T>) {
    if let Some(node) = root {
        inorder_recursive(node.left.as_deref(), out);
        out.push(node.value.clone());
        inorder_recursive(node.right.as_deref(), out);
    }
}

fn preorder_recursive<T: Clone>(root: Option<&BinaryTreeNode<T>>, out: &mut Vec<T>) {
    if let Some(node) = root {
        out.push(node.value.clone());
        preorder_recursive(node.left.as_deref(), out);
        preorder_recursive(node.right.as_deref(), out);
    }
}

fn postorder_recursive<T: Clone>(root: Option<&BinaryTreeNode<T>>, out: &mut Vec<T>) {
    if let Some(node) = root {
        postorder_recursive(node.left.as_deref(), out);
        postorder_recursive(node.right.as_deref(), out);
        out.push(node.value.clone());
    }
}

fn fill_array<T: Clone>(root: Option<&BinaryTreeNode<T>>, index: usize, out: &mut [Option<T>]) {
    if let Some(node) = root {
        if index >= out.len() {
            return;
        }
        out[index] = Some(node.value.clone());
        fill_array(node.left.as_deref(), 2 * index + 1, out);
        fill_array(node.right.as_deref(), 2 * index + 2, out);
    }
}

fn render_ascii<T: Debug>(node: &BinaryTreeNode<T>, prefix: &str, is_tail: bool, out: &mut String) {
    let _ = writeln!(
        out,
        "{}{}{:?}",
        prefix,
        if is_tail { "`-- " } else { "|-- " },
        node.value
    );

    let mut children = Vec::new();
    if let Some(left) = node.left.as_deref() {
        children.push(left);
    }
    if let Some(right) = node.right.as_deref() {
        children.push(right);
    }

    let next_prefix = format!("{}{}", prefix, if is_tail { "    " } else { "|   " });
    for (index, child) in children.iter().enumerate() {
        let last = index + 1 == children.len();
        render_ascii(child, &next_prefix, last, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_order_round_trip() {
        let tree = BinaryTree::from_level_order(vec![
            Some(1),
            Some(2),
            Some(3),
            Some(4),
            Some(5),
            Some(6),
            Some(7),
        ]);
        assert_eq!(
            tree.to_array(),
            vec![
                Some(1),
                Some(2),
                Some(3),
                Some(4),
                Some(5),
                Some(6),
                Some(7),
            ]
        );
        assert_eq!(tree.level_order(), vec![1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn shape_queries_work() {
        let tree = BinaryTree::from_level_order(vec![Some(1), Some(2), None]);
        assert!(!tree.is_full());
        assert!(tree.is_complete());
        assert!(!tree.is_perfect());
        assert_eq!(tree.height(), 1);
        assert_eq!(tree.size(), 2);
    }

    #[test]
    fn traversals_work() {
        let tree = BinaryTree::from_level_order(vec![
            Some(1),
            Some(2),
            Some(3),
            Some(4),
            None,
            Some(5),
            None,
        ]);
        assert_eq!(tree.preorder(), vec![1, 2, 4, 3, 5]);
        assert_eq!(tree.inorder(), vec![4, 2, 1, 5, 3]);
        assert_eq!(tree.postorder(), vec![4, 2, 5, 3, 1]);
        assert_eq!(tree.level_order(), vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn ascii_render_contains_values() {
        let tree = BinaryTree::from_level_order(vec![Some("root"), Some("left"), Some("right")]);
        let ascii = tree.to_ascii();
        assert!(ascii.contains("root"));
        assert!(ascii.contains("left"));
        assert!(ascii.contains("right"));
    }
}
