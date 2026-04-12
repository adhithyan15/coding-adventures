//! DT ordered set built on top of the balanced tree crates.

use std::cmp::Ordering;
use std::iter::FromIterator;
use std::marker::PhantomData;

use avl_tree::AVLTree;
use red_black_tree::RBTree;

pub trait OrderedTreeBackend<T>: Clone + Default
where
    T: Ord + Clone,
{
    fn insert(&self, value: T) -> Self;
    fn delete(&self, value: &T) -> Self;
    fn contains(&self, value: &T) -> bool;
    fn min_value(&self) -> Option<&T>;
    fn max_value(&self) -> Option<&T>;
    fn predecessor(&self, value: &T) -> Option<&T>;
    fn successor(&self, value: &T) -> Option<&T>;
    fn kth_smallest(&self, k: usize) -> Option<&T>;
    fn to_sorted_array(&self) -> Vec<T>;

    fn size(&self) -> usize {
        self.to_sorted_array().len()
    }

    fn rank(&self, value: &T) -> usize {
        self.to_sorted_array()
            .partition_point(|candidate| candidate < value)
    }
}

impl<T: Ord + Clone> OrderedTreeBackend<T> for AVLTree<T> {
    fn insert(&self, value: T) -> Self {
        AVLTree::insert(self, value)
    }

    fn delete(&self, value: &T) -> Self {
        AVLTree::delete(self, value)
    }

    fn contains(&self, value: &T) -> bool {
        AVLTree::contains(self, value)
    }

    fn min_value(&self) -> Option<&T> {
        AVLTree::min_value(self)
    }

    fn max_value(&self) -> Option<&T> {
        AVLTree::max_value(self)
    }

    fn predecessor(&self, value: &T) -> Option<&T> {
        AVLTree::predecessor(self, value)
    }

    fn successor(&self, value: &T) -> Option<&T> {
        AVLTree::successor(self, value)
    }

    fn kth_smallest(&self, k: usize) -> Option<&T> {
        AVLTree::kth_smallest(self, k)
    }

    fn to_sorted_array(&self) -> Vec<T> {
        AVLTree::to_sorted_array(self)
    }

    fn size(&self) -> usize {
        AVLTree::size(self)
    }

    fn rank(&self, value: &T) -> usize {
        AVLTree::rank(self, value)
    }
}

impl<T: Ord + Clone> OrderedTreeBackend<T> for RBTree<T> {
    fn insert(&self, value: T) -> Self {
        RBTree::insert(self, value)
    }

    fn delete(&self, value: &T) -> Self {
        RBTree::delete(self, value)
    }

    fn contains(&self, value: &T) -> bool {
        RBTree::contains(self, value)
    }

    fn min_value(&self) -> Option<&T> {
        RBTree::min_value(self)
    }

    fn max_value(&self) -> Option<&T> {
        RBTree::max_value(self)
    }

    fn predecessor(&self, value: &T) -> Option<&T> {
        RBTree::predecessor(self, value)
    }

    fn successor(&self, value: &T) -> Option<&T> {
        RBTree::successor(self, value)
    }

    fn kth_smallest(&self, k: usize) -> Option<&T> {
        RBTree::kth_smallest(self, k)
    }

    fn to_sorted_array(&self) -> Vec<T> {
        RBTree::to_sorted_array(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeSet<T, B = AVLTree<T>>
where
    T: Ord + Clone,
    B: OrderedTreeBackend<T>,
{
    backend: B,
    marker: PhantomData<T>,
}

pub type AvlTreeSet<T> = TreeSet<T, AVLTree<T>>;
pub type RedBlackTreeSet<T> = TreeSet<T, RBTree<T>>;

impl<T, B> Default for TreeSet<T, B>
where
    T: Ord + Clone,
    B: OrderedTreeBackend<T>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T, B> TreeSet<T, B>
where
    T: Ord + Clone,
    B: OrderedTreeBackend<T>,
{
    pub fn with_backend(backend: B) -> Self {
        Self {
            backend,
            marker: PhantomData,
        }
    }

    pub fn into_backend(self) -> B {
        self.backend
    }

    pub fn backend(&self) -> &B {
        &self.backend
    }

    pub fn new() -> Self {
        Self::with_backend(B::default())
    }

    pub fn empty() -> Self {
        Self::new()
    }

    pub fn from_list(values: impl IntoIterator<Item = T>) -> Self {
        let mut set = Self::new();
        for value in values {
            set = set.insert(value);
        }
        set
    }

    pub fn from_sorted_array(values: impl IntoIterator<Item = T>) -> Self {
        Self::from_list(values)
    }

    pub fn len(&self) -> usize {
        self.backend.size()
    }

    pub fn size(&self) -> usize {
        self.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn contains(&self, value: &T) -> bool {
        self.backend.contains(value)
    }

    pub fn min_value(&self) -> Option<&T> {
        self.backend.min_value()
    }

    pub fn max_value(&self) -> Option<&T> {
        self.backend.max_value()
    }

    pub fn first(&self) -> Option<&T> {
        self.min_value()
    }

    pub fn last(&self) -> Option<&T> {
        self.max_value()
    }

    pub fn predecessor(&self, value: &T) -> Option<&T> {
        self.backend.predecessor(value)
    }

    pub fn successor(&self, value: &T) -> Option<&T> {
        self.backend.successor(value)
    }

    pub fn kth_smallest(&self, k: usize) -> Option<&T> {
        self.backend.kth_smallest(k)
    }

    pub fn rank(&self, value: &T) -> usize {
        self.backend.rank(value)
    }

    pub fn to_sorted_array(&self) -> Vec<T> {
        self.backend.to_sorted_array()
    }

    pub fn to_list(&self) -> Vec<T> {
        self.to_sorted_array()
    }

    pub fn iter(&self) -> std::vec::IntoIter<T> {
        self.to_sorted_array().into_iter()
    }

    pub fn insert(&self, value: T) -> Self {
        Self::with_backend(self.backend.insert(value))
    }

    pub fn remove(&self, value: &T) -> Self {
        Self::with_backend(self.backend.delete(value))
    }

    pub fn delete(&self, value: &T) -> Self {
        self.remove(value)
    }

    pub fn range(&self, min: &T, max: &T, inclusive: bool) -> Vec<T> {
        if min > max {
            return Vec::new();
        }
        let values = self.to_sorted_array();
        let slice = values.as_slice();
        let start = if inclusive {
            slice.partition_point(|value| value < min)
        } else {
            slice.partition_point(|value| value <= min)
        };
        let end = if inclusive {
            slice.partition_point(|value| value <= max)
        } else {
            slice.partition_point(|value| value < max)
        };
        if start >= end {
            return Vec::new();
        }
        values[start..end].to_vec()
    }

    pub fn union(&self, other: &Self) -> Self {
        Self::from_list(merge_sorted_unique(&self.to_sorted_array(), &other.to_sorted_array()))
    }

    pub fn intersection(&self, other: &Self) -> Self {
        Self::from_list(intersection_sorted(&self.to_sorted_array(), &other.to_sorted_array()))
    }

    pub fn difference(&self, other: &Self) -> Self {
        Self::from_list(difference_sorted(&self.to_sorted_array(), &other.to_sorted_array()))
    }

    pub fn symmetric_difference(&self, other: &Self) -> Self {
        Self::from_list(symmetric_difference_sorted(
            &self.to_sorted_array(),
            &other.to_sorted_array(),
        ))
    }

    pub fn is_subset(&self, other: &Self) -> bool {
        is_subset_sorted(&self.to_sorted_array(), &other.to_sorted_array())
    }

    pub fn is_superset(&self, other: &Self) -> bool {
        other.is_subset(self)
    }

    pub fn is_disjoint(&self, other: &Self) -> bool {
        is_disjoint_sorted(&self.to_sorted_array(), &other.to_sorted_array())
    }

    pub fn equals(&self, other: &Self) -> bool {
        self.to_sorted_array() == other.to_sorted_array()
    }
}

impl<T, B> FromIterator<T> for TreeSet<T, B>
where
    T: Ord + Clone,
    B: OrderedTreeBackend<T>,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::from_list(iter)
    }
}

fn merge_sorted_unique<T: Ord + Clone>(left: &[T], right: &[T]) -> Vec<T> {
    let mut result = Vec::with_capacity(left.len() + right.len());
    let mut left_index = 0;
    let mut right_index = 0;

    while left_index < left.len() && right_index < right.len() {
        match left[left_index].cmp(&right[right_index]) {
            Ordering::Less => {
                result.push(left[left_index].clone());
                left_index += 1;
            }
            Ordering::Greater => {
                result.push(right[right_index].clone());
                right_index += 1;
            }
            Ordering::Equal => {
                result.push(left[left_index].clone());
                left_index += 1;
                right_index += 1;
            }
        }
    }

    result.extend(left[left_index..].iter().cloned());
    result.extend(right[right_index..].iter().cloned());
    result
}

fn intersection_sorted<T: Ord + Clone>(left: &[T], right: &[T]) -> Vec<T> {
    let mut result = Vec::new();
    let mut left_index = 0;
    let mut right_index = 0;

    while left_index < left.len() && right_index < right.len() {
        match left[left_index].cmp(&right[right_index]) {
            Ordering::Less => left_index += 1,
            Ordering::Greater => right_index += 1,
            Ordering::Equal => {
                result.push(left[left_index].clone());
                left_index += 1;
                right_index += 1;
            }
        }
    }

    result
}

fn difference_sorted<T: Ord + Clone>(left: &[T], right: &[T]) -> Vec<T> {
    let mut result = Vec::new();
    let mut left_index = 0;
    let mut right_index = 0;

    while left_index < left.len() && right_index < right.len() {
        match left[left_index].cmp(&right[right_index]) {
            Ordering::Less => {
                result.push(left[left_index].clone());
                left_index += 1;
            }
            Ordering::Greater => right_index += 1,
            Ordering::Equal => {
                left_index += 1;
                right_index += 1;
            }
        }
    }

    result.extend(left[left_index..].iter().cloned());
    result
}

fn symmetric_difference_sorted<T: Ord + Clone>(left: &[T], right: &[T]) -> Vec<T> {
    let mut result = Vec::new();
    let mut left_index = 0;
    let mut right_index = 0;

    while left_index < left.len() && right_index < right.len() {
        match left[left_index].cmp(&right[right_index]) {
            Ordering::Less => {
                result.push(left[left_index].clone());
                left_index += 1;
            }
            Ordering::Greater => {
                result.push(right[right_index].clone());
                right_index += 1;
            }
            Ordering::Equal => {
                left_index += 1;
                right_index += 1;
            }
        }
    }

    result.extend(left[left_index..].iter().cloned());
    result.extend(right[right_index..].iter().cloned());
    result
}

fn is_subset_sorted<T: Ord + Clone>(left: &[T], right: &[T]) -> bool {
    let mut left_index = 0;
    let mut right_index = 0;

    while left_index < left.len() && right_index < right.len() {
        match left[left_index].cmp(&right[right_index]) {
            Ordering::Less => return false,
            Ordering::Greater => right_index += 1,
            Ordering::Equal => {
                left_index += 1;
                right_index += 1;
            }
        }
    }

    left_index == left.len()
}

fn is_disjoint_sorted<T: Ord + Clone>(left: &[T], right: &[T]) -> bool {
    let mut left_index = 0;
    let mut right_index = 0;

    while left_index < left.len() && right_index < right.len() {
        match left[left_index].cmp(&right[right_index]) {
            Ordering::Less => left_index += 1,
            Ordering::Greater => right_index += 1,
            Ordering::Equal => return false,
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avl_backend_supports_ordered_set_operations() {
        let set = TreeSet::<i32>::from_list([7, 3, 9, 1, 5, 3]);
        assert_eq!(set.to_sorted_array(), vec![1, 3, 5, 7, 9]);
        assert_eq!(set.len(), 5);
        assert_eq!(set.min_value(), Some(&1));
        assert_eq!(set.max_value(), Some(&9));
        assert_eq!(set.rank(&7), 3);
        assert_eq!(set.kth_smallest(3), Some(&5));
        assert_eq!(set.range(&3, &7, true), vec![3, 5, 7]);
        assert_eq!(set.range(&3, &7, false), vec![5]);
        assert!(set.backend.is_valid_avl());

        let removed = set.remove(&5);
        assert_eq!(removed.to_sorted_array(), vec![1, 3, 7, 9]);
        assert_eq!(set.to_sorted_array(), vec![1, 3, 5, 7, 9]);
    }

    #[test]
    fn avl_backend_set_algebra_works() {
        let left = TreeSet::<i32>::from_list([1, 2, 3, 5]);
        let right = TreeSet::<i32>::from_list([3, 4, 5, 6]);

        assert_eq!(left.union(&right).to_sorted_array(), vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(left.intersection(&right).to_sorted_array(), vec![3, 5]);
        assert_eq!(left.difference(&right).to_sorted_array(), vec![1, 2]);
        assert_eq!(left.symmetric_difference(&right).to_sorted_array(), vec![1, 2, 4, 6]);
        assert!(left.is_subset(&left.union(&right)));
        assert!(left.is_superset(&left.intersection(&right).union(&TreeSet::from_list([1, 2]))));
        assert!(left.is_disjoint(&TreeSet::<i32>::from_list([8, 9])));
        assert!(left.equals(&TreeSet::<i32>::from_list([1, 2, 3, 5])));
    }

    #[test]
    fn red_black_backend_supports_the_same_api() {
        let set = TreeSet::<i32, RBTree<i32>>::from_list([10, 4, 14, 2, 8, 12, 16]);
        assert_eq!(set.to_sorted_array(), vec![2, 4, 8, 10, 12, 14, 16]);
        assert!(set.backend.is_valid_rb());
        assert_eq!(set.predecessor(&10), Some(&8));
        assert_eq!(set.successor(&10), Some(&12));
        assert!(set.contains(&14));
        assert_eq!(set.delete(&8).to_sorted_array(), vec![2, 4, 10, 12, 14, 16]);
    }
}
