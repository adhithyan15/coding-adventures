//! # The VSIDS decision heap
//!
//! VSIDS ("variable state independent decaying sum") picks the next decision
//! variable as the unassigned variable with the highest **activity**. A
//! variable's activity goes up every time it takes part in a conflict, and
//! all activities decay over time, so the solver keeps working on the
//! variables that are causing trouble *now*.
//!
//! To find the most active unassigned variable quickly, variables sit in a
//! binary **max-heap** keyed on activity:
//!
//! ```text
//!                 x4 (9.0)              parent of slot i:  (i - 1) / 2
//!               /          \            children of slot i: 2i + 1, 2i + 2
//!          x1 (7.5)      x0 (3.0)
//!          /     \
//!     x2 (1.0)  x3 (0.5)
//! ```
//!
//! The heap needs one operation a plain `BinaryHeap` does not have:
//! **increase a key in place** when a variable's activity is bumped. So it
//! also keeps `position[v]`, the slot each variable occupies (or `ABSENT`),
//! which lets a bumped variable sift up from where it already is.
//!
//! Activities live outside the heap (the solver owns them); every operation
//! that compares keys takes the activity slice as an argument.
//!
//! Assigned variables are not removed eagerly. The solver pops variables
//! until it finds an unassigned one, and puts variables back when
//! backtracking unassigns them. That is the MiniSat design: it keeps
//! backtracking cheap.

/// Marker for "this variable is not in the heap".
const ABSENT: usize = usize::MAX;

/// A binary max-heap of variable indices ordered by an external activity
/// array, with in-place key increase.
#[derive(Debug, Default, Clone)]
pub(crate) struct VarHeap {
    /// The heap array: `slots[0]` is the most active variable.
    slots: Vec<usize>,
    /// `position[v]` is `v`'s slot in `slots`, or `ABSENT`.
    position: Vec<usize>,
}

impl VarHeap {
    /// Make room for variables `0..n` (none of them inserted yet).
    pub(crate) fn grow_to(&mut self, n: usize) {
        if self.position.len() < n {
            self.position.resize(n, ABSENT);
        }
    }

    /// Whether variable `v` is currently in the heap.
    pub(crate) fn contains(&self, v: usize) -> bool {
        v < self.position.len() && self.position[v] != ABSENT
    }

    /// Whether the heap holds no variables.
    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Insert variable `v` (no-op if already present).
    pub(crate) fn insert(&mut self, v: usize, activity: &[f64]) {
        self.grow_to(v + 1);
        if self.contains(v) {
            return;
        }
        self.position[v] = self.slots.len();
        self.slots.push(v);
        self.sift_up(self.slots.len() - 1, activity);
    }

    /// Restore heap order after `v`'s activity went **up**.
    pub(crate) fn increased(&mut self, v: usize, activity: &[f64]) {
        if self.contains(v) {
            self.sift_up(self.position[v], activity);
        }
    }

    /// Remove and return the most active variable.
    pub(crate) fn pop_max(&mut self, activity: &[f64]) -> Option<usize> {
        let top = *self.slots.first()?;
        let last = self.slots.pop().expect("non-empty heap has a last slot");
        self.position[top] = ABSENT;
        if !self.slots.is_empty() {
            // Move the last element into the hole at the root, then let it
            // sink to where it belongs.
            self.slots[0] = last;
            self.position[last] = 0;
            self.sift_down(0, activity);
        }
        Some(top)
    }

    /// Move the element at `slot` up while it is more active than its parent.
    fn sift_up(&mut self, mut slot: usize, activity: &[f64]) {
        let v = self.slots[slot];
        while slot > 0 {
            let parent = (slot - 1) / 2;
            let p = self.slots[parent];
            if activity[v] <= activity[p] {
                break;
            }
            self.slots[slot] = p;
            self.position[p] = slot;
            slot = parent;
        }
        self.slots[slot] = v;
        self.position[v] = slot;
    }

    /// Move the element at `slot` down while a child is more active.
    fn sift_down(&mut self, mut slot: usize, activity: &[f64]) {
        let v = self.slots[slot];
        let n = self.slots.len();
        loop {
            let left = 2 * slot + 1;
            if left >= n {
                break;
            }
            let right = left + 1;
            let child = if right < n && activity[self.slots[right]] > activity[self.slots[left]] {
                right
            } else {
                left
            };
            if activity[self.slots[child]] <= activity[v] {
                break;
            }
            let c = self.slots[child];
            self.slots[slot] = c;
            self.position[c] = slot;
            slot = child;
        }
        self.slots[slot] = v;
        self.position[v] = slot;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pops_in_activity_order() {
        let activity = vec![3.0, 7.5, 1.0, 0.5, 9.0];
        let mut h = VarHeap::default();
        for v in 0..5 {
            h.insert(v, &activity);
        }
        h.insert(2, &activity); // duplicate insert is a no-op
        let mut order = Vec::new();
        while let Some(v) = h.pop_max(&activity) {
            order.push(v);
        }
        assert_eq!(order, vec![4, 1, 0, 2, 3]);
        assert!(h.is_empty());
        assert_eq!(h.pop_max(&activity), None);
    }

    #[test]
    fn increase_key_moves_a_variable_up() {
        let mut activity = vec![1.0, 2.0, 3.0, 4.0];
        let mut h = VarHeap::default();
        for v in 0..4 {
            h.insert(v, &activity);
        }
        activity[0] = 10.0;
        h.increased(0, &activity);
        h.increased(99, &activity); // absent variable: ignored
        assert_eq!(h.pop_max(&activity), Some(0));
        assert!(!h.contains(0));
        assert!(h.contains(3));
        assert_eq!(h.pop_max(&activity), Some(3));
    }

    #[test]
    fn many_random_operations_keep_heap_order() {
        // A small deterministic generator so the test needs no crates.
        let mut state = 0x1234_5678_u64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let n = 200;
        let mut activity: Vec<f64> = (0..n).map(|_| (next() % 1000) as f64).collect();
        let mut h = VarHeap::default();
        h.grow_to(n);
        for v in 0..n {
            h.insert(v, &activity);
        }
        for _ in 0..500 {
            let v = (next() % n as u64) as usize;
            activity[v] += (next() % 50) as f64;
            h.increased(v, &activity);
        }
        let mut last = f64::INFINITY;
        while let Some(v) = h.pop_max(&activity) {
            assert!(activity[v] <= last);
            last = activity[v];
        }
    }
}
