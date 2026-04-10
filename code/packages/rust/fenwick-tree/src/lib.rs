//! Binary Indexed Tree (Fenwick Tree) for prefix sums with point updates.

use std::error::Error;
use std::fmt;

fn lowbit(index: usize) -> usize {
    index & (!index + 1)
}

fn highest_power_of_two_at_most(n: usize) -> usize {
    if n == 0 {
        0
    } else {
        1usize << (usize::BITS as usize - n.leading_zeros() as usize - 1)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum FenwickError {
    IndexOutOfRange {
        index: usize,
        min: usize,
        max: usize,
    },
    InvalidRange {
        left: usize,
        right: usize,
    },
    EmptyTree,
    NonPositiveTarget {
        target: f64,
    },
    TargetExceedsTotal {
        target: f64,
        total: f64,
    },
}

impl fmt::Display for FenwickError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IndexOutOfRange { index, min, max } => {
                write!(f, "index {index} out of range [{min}, {max}]")
            }
            Self::InvalidRange { left, right } => {
                write!(f, "left ({left}) must be <= right ({right})")
            }
            Self::EmptyTree => write!(f, "find_kth called on empty tree"),
            Self::NonPositiveTarget { target } => {
                write!(f, "k must be positive, got {target}")
            }
            Self::TargetExceedsTotal { target, total } => {
                write!(f, "k ({target}) exceeds total sum of the tree ({total})")
            }
        }
    }
}

impl Error for FenwickError {}

#[derive(Clone, Debug, PartialEq)]
pub struct FenwickTree {
    n: usize,
    bit: Vec<f64>,
}

impl FenwickTree {
    pub fn new(n: usize) -> Self {
        Self {
            n,
            bit: vec![0.0; n + 1],
        }
    }

    pub fn from_slice(values: &[f64]) -> Self {
        let mut tree = Self::new(values.len());
        for index in 1..=tree.n {
            tree.bit[index] += values[index - 1];
            let parent = index + lowbit(index);
            if parent <= tree.n {
                tree.bit[parent] += tree.bit[index];
            }
        }
        tree
    }

    pub fn from_iterable<I>(values: I) -> Self
    where
        I: IntoIterator<Item = f64>,
    {
        let collected: Vec<f64> = values.into_iter().collect();
        Self::from_slice(&collected)
    }

    pub fn update(&mut self, index: usize, delta: f64) -> Result<(), FenwickError> {
        self.check_index(index)?;
        let mut current = index;
        while current <= self.n {
            self.bit[current] += delta;
            current += lowbit(current);
        }
        Ok(())
    }

    pub fn prefix_sum(&self, index: usize) -> Result<f64, FenwickError> {
        if index > self.n {
            return Err(FenwickError::IndexOutOfRange {
                index,
                min: 0,
                max: self.n,
            });
        }

        let mut total = 0.0;
        let mut current = index;
        while current > 0 {
            total += self.bit[current];
            current -= lowbit(current);
        }
        Ok(total)
    }

    pub fn range_sum(&self, left: usize, right: usize) -> Result<f64, FenwickError> {
        if left > right {
            return Err(FenwickError::InvalidRange { left, right });
        }

        self.check_index(left)?;
        self.check_index(right)?;
        if left == 1 {
            self.prefix_sum(right)
        } else {
            Ok(self.prefix_sum(right)? - self.prefix_sum(left - 1)?)
        }
    }

    pub fn point_query(&self, index: usize) -> Result<f64, FenwickError> {
        self.check_index(index)?;
        self.range_sum(index, index)
    }

    pub fn find_kth(&self, mut target: f64) -> Result<usize, FenwickError> {
        if self.n == 0 {
            return Err(FenwickError::EmptyTree);
        }
        if target <= 0.0 {
            return Err(FenwickError::NonPositiveTarget { target });
        }

        let total = self.prefix_sum(self.n)?;
        if target > total {
            return Err(FenwickError::TargetExceedsTotal { target, total });
        }

        let mut index = 0;
        let mut step = highest_power_of_two_at_most(self.n);
        while step > 0 {
            let next_index = index + step;
            if next_index <= self.n && self.bit[next_index] < target {
                index = next_index;
                target -= self.bit[index];
            }
            step >>= 1;
        }

        Ok(index + 1)
    }

    pub fn len(&self) -> usize {
        self.n
    }

    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    pub fn bit_array(&self) -> &[f64] {
        &self.bit[1..]
    }

    fn check_index(&self, index: usize) -> Result<(), FenwickError> {
        if (1..=self.n).contains(&index) {
            Ok(())
        } else {
            Err(FenwickError::IndexOutOfRange {
                index,
                min: 1,
                max: self.n,
            })
        }
    }
}

impl fmt::Display for FenwickTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FenwickTree(n={}, bit={:?})", self.n, self.bit_array())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "expected {expected}, got {actual}"
        );
    }

    fn brute_prefix(values: &[f64], index: usize) -> f64 {
        values[..index].iter().sum()
    }

    fn brute_range(values: &[f64], left: usize, right: usize) -> f64 {
        values[left - 1..right].iter().sum()
    }

    #[test]
    fn from_slice_builds_expected_prefix_sums() {
        let tree = FenwickTree::from_slice(&[3.0, 2.0, 1.0, 7.0, 4.0]);
        assert_close(tree.prefix_sum(1).unwrap(), 3.0);
        assert_close(tree.prefix_sum(2).unwrap(), 5.0);
        assert_close(tree.prefix_sum(3).unwrap(), 6.0);
        assert_close(tree.prefix_sum(4).unwrap(), 13.0);
        assert_close(tree.prefix_sum(5).unwrap(), 17.0);
    }

    #[test]
    fn prefix_sum_accepts_zero() {
        let tree = FenwickTree::from_slice(&[1.0, 2.0, 3.0]);
        assert_close(tree.prefix_sum(0).unwrap(), 0.0);
    }

    #[test]
    fn range_sum_and_point_query_match_expected_values() {
        let tree = FenwickTree::from_slice(&[3.0, 2.0, 1.0, 7.0, 4.0]);
        assert_close(tree.range_sum(2, 4).unwrap(), 10.0);
        assert_close(tree.point_query(4).unwrap(), 7.0);
    }

    #[test]
    fn update_changes_future_queries() {
        let mut tree = FenwickTree::from_slice(&[3.0, 2.0, 1.0, 7.0, 4.0]);
        tree.update(3, 5.0).unwrap();
        assert_close(tree.prefix_sum(3).unwrap(), 11.0);
        assert_close(tree.point_query(3).unwrap(), 6.0);
    }

    #[test]
    fn find_kth_matches_order_statistic_examples() {
        let tree = FenwickTree::from_slice(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(tree.find_kth(1.0).unwrap(), 1);
        assert_eq!(tree.find_kth(2.0).unwrap(), 2);
        assert_eq!(tree.find_kth(3.0).unwrap(), 2);
        assert_eq!(tree.find_kth(4.0).unwrap(), 3);
        assert_eq!(tree.find_kth(10.0).unwrap(), 4);
    }

    #[test]
    fn find_kth_reports_empty_or_invalid_targets() {
        let tree = FenwickTree::new(0);
        assert_eq!(tree.find_kth(1.0), Err(FenwickError::EmptyTree));

        let tree = FenwickTree::from_slice(&[1.0, 2.0, 3.0]);
        assert_eq!(
            tree.find_kth(0.0),
            Err(FenwickError::NonPositiveTarget { target: 0.0 })
        );
        assert!(matches!(
            tree.find_kth(100.0),
            Err(FenwickError::TargetExceedsTotal { .. })
        ));
    }

    #[test]
    fn invalid_indices_report_errors() {
        let tree = FenwickTree::from_slice(&[1.0, 2.0, 3.0]);
        assert!(matches!(
            tree.prefix_sum(4),
            Err(FenwickError::IndexOutOfRange { .. })
        ));
        assert!(matches!(
            tree.range_sum(0, 3),
            Err(FenwickError::IndexOutOfRange { .. })
        ));
        assert_eq!(
            tree.range_sum(3, 1),
            Err(FenwickError::InvalidRange { left: 3, right: 1 })
        );
    }

    #[test]
    fn bit_array_and_display_expose_internal_state() {
        let tree = FenwickTree::from_slice(&[1.0, 2.0]);
        assert_eq!(tree.bit_array(), &[1.0, 3.0]);
        assert!(tree.to_string().contains("FenwickTree"));
    }

    #[test]
    fn brute_force_prefix_and_range_queries() {
        let values = [5.0, -2.0, 7.0, 1.5, 4.5];
        let tree = FenwickTree::from_slice(&values);
        for index in 1..=values.len() {
            assert_close(
                tree.prefix_sum(index).unwrap(),
                brute_prefix(&values, index),
            );
        }
        for left in 1..=values.len() {
            for right in left..=values.len() {
                assert_close(
                    tree.range_sum(left, right).unwrap(),
                    brute_range(&values, left, right),
                );
            }
        }
    }
}
