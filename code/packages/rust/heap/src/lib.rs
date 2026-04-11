//! Array-backed binary heaps and companion algorithms.

use std::fmt;

fn min_priority<T: Ord>(a: &T, b: &T) -> bool {
    a < b
}

fn max_priority<T: Ord>(a: &T, b: &T) -> bool {
    a > b
}

fn sift_up<T: Ord>(data: &mut [T], mut index: usize, higher_priority: fn(&T, &T) -> bool) {
    while index > 0 {
        let parent = (index - 1) / 2;
        if higher_priority(&data[index], &data[parent]) {
            data.swap(index, parent);
            index = parent;
        } else {
            break;
        }
    }
}

fn sift_down<T: Ord>(data: &mut [T], mut index: usize, higher_priority: fn(&T, &T) -> bool) {
    let len = data.len();
    loop {
        let left = 2 * index + 1;
        let right = 2 * index + 2;
        let mut best = index;

        if left < len && higher_priority(&data[left], &data[best]) {
            best = left;
        }
        if right < len && higher_priority(&data[right], &data[best]) {
            best = right;
        }
        if best == index {
            break;
        }

        data.swap(index, best);
        index = best;
    }
}

fn build_heap<T: Ord>(data: &mut [T], higher_priority: fn(&T, &T) -> bool) {
    if data.len() < 2 {
        return;
    }

    for index in (0..=((data.len() - 2) / 2)).rev() {
        sift_down(data, index, higher_priority);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MinHeap<T: Ord> {
    data: Vec<T>,
}

impl<T: Ord> Default for MinHeap<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Ord> MinHeap<T> {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn from_iterable<I>(items: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        let mut data: Vec<T> = items.into_iter().collect();
        build_heap(&mut data, min_priority::<T>);
        Self { data }
    }

    pub fn push(&mut self, value: T) {
        self.data.push(value);
        let last_index = self.data.len() - 1;
        sift_up(&mut self.data, last_index, min_priority::<T>);
    }

    pub fn pop(&mut self) -> Option<T> {
        let last = self.data.pop()?;
        if self.data.is_empty() {
            return Some(last);
        }

        let root = std::mem::replace(&mut self.data[0], last);
        sift_down(&mut self.data, 0, min_priority::<T>);
        Some(root)
    }

    pub fn peek(&self) -> Option<&T> {
        self.data.first()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn to_vec(&self) -> Vec<T>
    where
        T: Clone,
    {
        self.data.clone()
    }
}

impl<T: Ord + fmt::Display> fmt::Display for MinHeap<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.peek() {
            Some(root) => write!(f, "MinHeap(size={}, root={root})", self.len()),
            None => write!(f, "MinHeap(size=0, root=empty)"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaxHeap<T: Ord> {
    data: Vec<T>,
}

impl<T: Ord> Default for MaxHeap<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Ord> MaxHeap<T> {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn from_iterable<I>(items: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        let mut data: Vec<T> = items.into_iter().collect();
        build_heap(&mut data, max_priority::<T>);
        Self { data }
    }

    pub fn push(&mut self, value: T) {
        self.data.push(value);
        let last_index = self.data.len() - 1;
        sift_up(&mut self.data, last_index, max_priority::<T>);
    }

    pub fn pop(&mut self) -> Option<T> {
        let last = self.data.pop()?;
        if self.data.is_empty() {
            return Some(last);
        }

        let root = std::mem::replace(&mut self.data[0], last);
        sift_down(&mut self.data, 0, max_priority::<T>);
        Some(root)
    }

    pub fn peek(&self) -> Option<&T> {
        self.data.first()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn to_vec(&self) -> Vec<T>
    where
        T: Clone,
    {
        self.data.clone()
    }
}

impl<T: Ord + fmt::Display> fmt::Display for MaxHeap<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.peek() {
            Some(root) => write!(f, "MaxHeap(size={}, root={root})", self.len()),
            None => write!(f, "MaxHeap(size=0, root=empty)"),
        }
    }
}

pub fn heapify<T, I>(items: I) -> Vec<T>
where
    T: Ord + Clone,
    I: IntoIterator<Item = T>,
{
    MinHeap::from_iterable(items).to_vec()
}

pub fn heap_sort<T, I>(items: I) -> Vec<T>
where
    T: Ord,
    I: IntoIterator<Item = T>,
{
    let mut heap = MinHeap::from_iterable(items);
    let mut result = Vec::with_capacity(heap.len());
    while let Some(value) = heap.pop() {
        result.push(value);
    }
    result
}

pub fn nlargest<T, I>(iterable: I, n: usize) -> Vec<T>
where
    T: Ord,
    I: IntoIterator<Item = T>,
{
    if n == 0 {
        return Vec::new();
    }

    let mut items: Vec<T> = iterable.into_iter().collect();
    if n >= items.len() {
        items.sort_by(|a, b| b.cmp(a));
        return items;
    }

    let tail = items.split_off(n);
    let mut heap = MinHeap::from_iterable(items);
    for value in tail {
        let should_insert = heap
            .peek()
            .map(|peek| value.cmp(peek).is_gt())
            .unwrap_or(true);
        if should_insert {
            let _ = heap.pop();
            heap.push(value);
        }
    }

    let mut result = Vec::with_capacity(heap.len());
    while let Some(value) = heap.pop() {
        result.push(value);
    }
    result.reverse();
    result
}

pub fn nsmallest<T, I>(iterable: I, n: usize) -> Vec<T>
where
    T: Ord,
    I: IntoIterator<Item = T>,
{
    if n == 0 {
        return Vec::new();
    }

    let mut items: Vec<T> = iterable.into_iter().collect();
    if n >= items.len() {
        items.sort();
        return items;
    }

    let tail = items.split_off(n);
    let mut heap = MaxHeap::from_iterable(items);
    for value in tail {
        let should_insert = heap
            .peek()
            .map(|peek| value.cmp(peek).is_lt())
            .unwrap_or(true);
        if should_insert {
            let _ = heap.pop();
            heap.push(value);
        }
    }

    let mut result = Vec::with_capacity(heap.len());
    while let Some(value) = heap.pop() {
        result.push(value);
    }
    result.reverse();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_valid_min_heap<T: Ord>(values: &[T]) -> bool {
        for index in 0..values.len() {
            let left = 2 * index + 1;
            let right = 2 * index + 2;
            if left < values.len() && values[index] > values[left] {
                return false;
            }
            if right < values.len() && values[index] > values[right] {
                return false;
            }
        }
        true
    }

    fn is_valid_max_heap<T: Ord>(values: &[T]) -> bool {
        for index in 0..values.len() {
            let left = 2 * index + 1;
            let right = 2 * index + 2;
            if left < values.len() && values[index] < values[left] {
                return false;
            }
            if right < values.len() && values[index] < values[right] {
                return false;
            }
        }
        true
    }

    #[test]
    fn min_heap_push_peek_pop_order() {
        let mut heap = MinHeap::new();
        for value in [5, 3, 8, 1, 4] {
            heap.push(value);
        }
        assert_eq!(heap.peek(), Some(&1));
        assert_eq!(heap.pop(), Some(1));
        assert_eq!(heap.pop(), Some(3));
        assert_eq!(heap.pop(), Some(4));
        assert_eq!(heap.pop(), Some(5));
        assert_eq!(heap.pop(), Some(8));
        assert_eq!(heap.pop(), None);
    }

    #[test]
    fn min_heap_property_holds_after_each_push() {
        let mut heap = MinHeap::new();
        for value in [5, 3, 8, 1, 4, 2, 7] {
            heap.push(value);
            assert!(is_valid_min_heap(&heap.to_vec()));
        }
    }

    #[test]
    fn min_heap_property_holds_after_each_pop() {
        let mut heap = MinHeap::from_iterable([5, 3, 8, 1, 4, 2, 7]);
        while heap.pop().is_some() {
            assert!(is_valid_min_heap(&heap.to_vec()));
        }
    }

    #[test]
    fn min_heap_supports_generic_ord_values() {
        let mut heap = MinHeap::from_iterable([
            String::from("delta"),
            String::from("alpha"),
            String::from("charlie"),
        ]);
        assert_eq!(heap.pop(), Some(String::from("alpha")));
    }

    #[test]
    fn max_heap_push_peek_pop_order() {
        let mut heap = MaxHeap::new();
        for value in [5, 3, 8, 1, 4] {
            heap.push(value);
        }
        assert_eq!(heap.peek(), Some(&8));
        assert_eq!(heap.pop(), Some(8));
        assert_eq!(heap.pop(), Some(5));
        assert_eq!(heap.pop(), Some(4));
        assert_eq!(heap.pop(), Some(3));
        assert_eq!(heap.pop(), Some(1));
        assert_eq!(heap.pop(), None);
    }

    #[test]
    fn max_heap_property_holds_after_each_pop() {
        let mut heap = MaxHeap::from_iterable([5, 3, 8, 1, 4]);
        while heap.pop().is_some() {
            assert!(is_valid_max_heap(&heap.to_vec()));
        }
    }

    #[test]
    fn heapify_preserves_elements_and_invariant() {
        let values = vec![3, 1, 4, 1, 5, 9, 2, 6];
        let heap = heapify(values.clone());
        let mut sorted_original = values;
        let mut sorted_heap = heap.clone();
        sorted_original.sort();
        sorted_heap.sort();
        assert_eq!(sorted_original, sorted_heap);
        assert!(is_valid_min_heap(&heap));
    }

    #[test]
    fn heap_sort_sorts_ascending() {
        assert_eq!(
            heap_sort([3, 1, 4, 1, 5, 9, 2, 6]),
            vec![1, 1, 2, 3, 4, 5, 6, 9]
        );
    }

    #[test]
    fn heap_sort_handles_empty_and_single_element_inputs() {
        assert_eq!(heap_sort(Vec::<i32>::new()), Vec::<i32>::new());
        assert_eq!(heap_sort([42]), vec![42]);
    }

    #[test]
    fn nlargest_returns_descending_values() {
        assert_eq!(nlargest([3, 1, 4, 1, 5, 9, 2, 6], 3), vec![9, 6, 5]);
    }

    #[test]
    fn nsmallest_returns_ascending_values() {
        assert_eq!(nsmallest([3, 1, 4, 1, 5, 9, 2, 6], 3), vec![1, 1, 2]);
    }

    #[test]
    fn top_k_handles_zero_and_large_n() {
        assert_eq!(nlargest([3, 1, 4], 0), Vec::<i32>::new());
        assert_eq!(nsmallest([3, 1, 4], 10), vec![1, 3, 4]);
    }

    #[test]
    fn display_mentions_heap_type_and_root() {
        let heap = MinHeap::from_iterable([7, 3, 9]);
        let rendered = heap.to_string();
        assert!(rendered.contains("MinHeap"));
        assert!(rendered.contains("3"));
    }
}
