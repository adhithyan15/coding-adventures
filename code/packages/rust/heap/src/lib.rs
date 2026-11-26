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

// A complete tree's next level is found by index arithmetic: the left child
// is 2*i+1 and the right child is 2*i+2. Use checked helpers rather than
// allowing those calculations to wrap for a very large zero-sized Vec.
fn sift_down<T: Ord>(data: &mut [T], mut index: usize, higher_priority: fn(&T, &T) -> bool) {
    let len = data.len();
    while let Some(left) = left_child_index(index) {
        if left >= len {
            break;
        }
        let mut best = index;

        if higher_priority(&data[left], &data[best]) {
            best = left;
        }
        if let Some(right) = right_child_index(left) {
            if right < len && higher_priority(&data[right], &data[best]) {
                best = right;
            }
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

/// A full bounded queue returns the rejected value instead of dropping it.
/// Allocation errors retain the value and expose the original reserve error.
#[derive(Debug)]
pub enum TryPushError<T> {
    /// The logical capacity was already full; the value was not inserted.
    Full { value: T },
    /// Reserving room failed before the queue was mutated.
    Allocation {
        value: T,
        source: std::collections::TryReserveError,
    },
}

impl<T> TryPushError<T> {
    /// Recover the value rejected by `try_push`.
    pub fn into_value(self) -> T {
        match self {
            Self::Full { value } | Self::Allocation { value, .. } => value,
        }
    }
}

impl<T> fmt::Display for TryPushError<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full { .. } => formatter.write_str("bounded heap is at capacity"),
            Self::Allocation { source, .. } => {
                write!(formatter, "heap allocation failed: {source}")
            }
        }
    }
}

impl<T: fmt::Debug> std::error::Error for TryPushError<T> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Full { .. } => None,
            Self::Allocation { source, .. } => Some(source),
        }
    }
}

/// A bounded-heap constructor failed before it could return a queue.
#[derive(Debug)]
pub enum TryBuildError<T> {
    /// One item beyond the logical limit was observed; the accepted prefix and
    /// the first rejected item are returned to the caller.
    CapacityExceeded { accepted: Vec<T>, rejected: T },
    /// Reserving for the next accepted item failed; no item was lost.
    Allocation {
        accepted: Vec<T>,
        rejected: T,
        source: std::collections::TryReserveError,
    },
}

impl<T> fmt::Display for TryBuildError<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityExceeded { .. } => {
                formatter.write_str("input contains more items than the heap capacity")
            }
            Self::Allocation { source, .. } => {
                write!(formatter, "heap allocation failed: {source}")
            }
        }
    }
}

impl<T: fmt::Debug> std::error::Error for TryBuildError<T> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CapacityExceeded { .. } => None,
            Self::Allocation { source, .. } => Some(source),
        }
    }
}

/// Operations shared by the checked bounded min- and max-priority queues.
///
/// `Ord` must remain a consistent total order while values are stored. A
/// comparison panic can leave a queue partially changed; callers should
/// discard that queue rather than relying on rollback or invariant recovery.
pub trait CheckedPriorityQueue<T: Ord> {
    /// Maximum number of values admitted by the queue.
    fn capacity(&self) -> usize;
    /// Number of currently queued values.
    fn len(&self) -> usize;
    /// Returns `true` when no values are queued.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Returns the next value without removing it.
    fn peek(&self) -> Option<&T>;
    /// Removes and returns the next value.
    fn pop(&mut self) -> Option<T>;
    /// Adds one value or returns ownership of it when admission fails.
    fn try_push(&mut self, value: T) -> Result<(), TryPushError<T>>;
    /// Drops all queued values while retaining the logical capacity.
    fn clear(&mut self);
}

// In the flat complete tree, every non-root item has one parent and each
// parent has at most two children. The logical capacity is separate from the
// Vec's allocation capacity, which can be unusual for zero-sized T.
#[derive(Clone, Debug, PartialEq, Eq)]
struct BoundedCore<T> {
    items: Vec<T>,
    capacity: usize,
    min_order: bool,
}

fn left_child_index(index: usize) -> Option<usize> {
    index.checked_mul(2)?.checked_add(1)
}

fn right_child_index(left: usize) -> Option<usize> {
    left.checked_add(1)
}

impl<T: Ord> BoundedCore<T> {
    fn new(capacity: usize, min_order: bool) -> Self {
        Self {
            items: Vec::new(),
            capacity,
            min_order,
        }
    }

    fn is_higher_priority(&self, first: &T, second: &T) -> bool {
        if self.min_order {
            first.cmp(second).is_lt()
        } else {
            first.cmp(second).is_gt()
        }
    }

    fn sift_up(&mut self, mut index: usize) {
        while index > 0 {
            let parent = (index - 1) / 2;
            if !self.is_higher_priority(&self.items[index], &self.items[parent]) {
                break;
            }
            self.items.swap(index, parent);
            index = parent;
        }
    }

    fn sift_down(&mut self, mut index: usize) {
        let len = self.items.len();
        while let Some(left) = left_child_index(index) {
            // A node at i has a left child at 2*i+1. Checked arithmetic makes
            // the boundary explicit even when a zero-sized T permits a very
            // large Vec length.
            if left >= len {
                break;
            }

            // Select the higher-priority child before comparing with the
            // current node, or one child edge could remain out of order.
            let mut best = left;
            if let Some(right) = right_child_index(left) {
                if right < len && self.is_higher_priority(&self.items[right], &self.items[left]) {
                    best = right;
                }
            }

            if !self.is_higher_priority(&self.items[best], &self.items[index]) {
                break;
            }
            self.items.swap(index, best);
            index = best;
        }
    }

    fn try_from_iterable<I>(
        capacity: usize,
        min_order: bool,
        items: I,
    ) -> Result<Self, TryBuildError<T>>
    where
        I: IntoIterator<Item = T>,
    {
        Self::try_from_iterable_with_reservation(
            capacity,
            min_order,
            items,
            |values, additional| values.try_reserve(additional),
        )
    }

    fn try_from_iterable_with_reservation<I, F>(
        capacity: usize,
        min_order: bool,
        items: I,
        mut reserve: F,
    ) -> Result<Self, TryBuildError<T>>
    where
        I: IntoIterator<Item = T>,
        F: FnMut(&mut Vec<T>, usize) -> Result<(), std::collections::TryReserveError>,
    {
        let mut accepted = Vec::new();
        for rejected_or_next in items {
            // Read the first excess value so it can be returned intact, then
            // stop without asking the iterator for another item.
            if accepted.len() >= capacity {
                return Err(TryBuildError::CapacityExceeded {
                    accepted,
                    rejected: rejected_or_next,
                });
            }
            if let Err(source) = reserve(&mut accepted, 1) {
                return Err(TryBuildError::Allocation {
                    accepted,
                    rejected: rejected_or_next,
                    source,
                });
            }
            accepted.push(rejected_or_next);
        }

        let mut core = Self {
            items: accepted,
            capacity,
            min_order,
        };
        for index in (0..core.items.len() / 2).rev() {
            core.sift_down(index);
        }
        Ok(core)
    }

    fn try_push(&mut self, value: T) -> Result<(), TryPushError<T>> {
        self.try_push_with_reservation(value, |items, additional| items.try_reserve(additional))
    }

    fn try_push_with_reservation<F>(&mut self, value: T, reserve: F) -> Result<(), TryPushError<T>>
    where
        F: FnOnce(&mut Vec<T>, usize) -> Result<(), std::collections::TryReserveError>,
    {
        if self.items.len() >= self.capacity {
            return Err(TryPushError::Full { value });
        }
        // Reserve before inserting: after success, appending one value cannot
        // allocate. An Ord panic during the following sift is a separate
        // unwind boundary and is intentionally not rolled back.
        if let Err(source) = reserve(&mut self.items, 1) {
            return Err(TryPushError::Allocation { value, source });
        }
        self.items.push(value);
        self.sift_up(self.items.len() - 1);
        Ok(())
    }

    fn pop(&mut self) -> Option<T> {
        let last = self.items.pop()?;
        if self.items.is_empty() {
            return Some(last);
        }
        let root = std::mem::replace(&mut self.items[0], last);
        self.sift_down(0);
        Some(root)
    }
}

macro_rules! bounded_heap {
    ($heap:ident, $min_order:expr, $description:literal) => {
        #[doc = $description]
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct $heap<T: Ord> {
            core: BoundedCore<T>,
        }

        impl<T: Ord> $heap<T> {
            /// Creates an empty queue with this logical capacity. No capacity-
            /// sized allocation is made until values are pushed.
            pub fn new(capacity: usize) -> Self {
                Self {
                    core: BoundedCore::new(capacity, $min_order),
                }
            }

            /// Builds a queue in linear time, consuming at most one value
            /// beyond the capacity before returning an error.
            pub fn try_from_iterable<I>(items: I, capacity: usize) -> Result<Self, TryBuildError<T>>
            where
                I: IntoIterator<Item = T>,
            {
                BoundedCore::try_from_iterable(capacity, $min_order, items)
                    .map(|core| Self { core })
            }

            /// Logical limit, independent of the backing `Vec` allocation.
            pub fn capacity(&self) -> usize {
                self.core.capacity
            }

            /// Number of queued values.
            pub fn len(&self) -> usize {
                self.core.items.len()
            }

            /// Whether the queue contains no values.
            pub fn is_empty(&self) -> bool {
                self.core.items.is_empty()
            }

            /// Returns the next value without removing it.
            pub fn peek(&self) -> Option<&T> {
                self.core.items.first()
            }

            /// Adds a value, returning it unchanged if full or if reservation
            /// fails before insertion.
            pub fn try_push(&mut self, value: T) -> Result<(), TryPushError<T>> {
                self.core.try_push(value)
            }

            /// Removes the highest-priority value, if one exists.
            pub fn pop(&mut self) -> Option<T> {
                self.core.pop()
            }

            /// Drops all values while retaining the configured logical limit.
            pub fn clear(&mut self) {
                self.core.items.clear();
            }

            /// Copies the backing heap order; this is not a sorted result.
            pub fn to_vec(&self) -> Vec<T>
            where
                T: Clone,
            {
                self.core.items.clone()
            }
        }

        impl<T: Ord> CheckedPriorityQueue<T> for $heap<T> {
            fn capacity(&self) -> usize {
                self.capacity()
            }

            fn len(&self) -> usize {
                self.len()
            }

            fn peek(&self) -> Option<&T> {
                self.peek()
            }

            fn pop(&mut self) -> Option<T> {
                self.pop()
            }

            fn try_push(&mut self, value: T) -> Result<(), TryPushError<T>> {
                self.try_push(value)
            }

            fn clear(&mut self) {
                self.clear();
            }
        }
    };
}

bounded_heap!(
    BoundedMinHeap,
    true,
    "A min-priority queue with a checked logical capacity."
);
bounded_heap!(
    BoundedMaxHeap,
    false,
    "A max-priority queue with a checked logical capacity."
);

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
