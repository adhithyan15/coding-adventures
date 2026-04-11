//! DT05 Segment Tree.
//!
//! A segment tree supports efficient range queries and point updates over an
//! array using a monoid-like combine function.

use std::fmt;
use std::ops::Add;
use std::sync::Arc;

pub trait SegmentValue: Clone + 'static {
    fn min_identity() -> Self;
    fn max_identity() -> Self;
}

macro_rules! impl_segment_value_signed {
    ($($ty:ty),* $(,)?) => {
        $(
            impl SegmentValue for $ty {
                fn min_identity() -> Self {
                    <$ty>::MAX
                }

                fn max_identity() -> Self {
                    <$ty>::MIN
                }
            }
        )*
    };
}

macro_rules! impl_segment_value_unsigned {
    ($($ty:ty),* $(,)?) => {
        $(
            impl SegmentValue for $ty {
                fn min_identity() -> Self {
                    <$ty>::MAX
                }

                fn max_identity() -> Self {
                    0
                }
            }
        )*
    };
}

macro_rules! impl_segment_value_float {
    ($($ty:ty),* $(,)?) => {
        $(
            impl SegmentValue for $ty {
                fn min_identity() -> Self {
                    <$ty>::INFINITY
                }

                fn max_identity() -> Self {
                    <$ty>::NEG_INFINITY
                }
            }
        )*
    };
}

impl_segment_value_signed!(i8, i16, i32, i64, i128, isize);
impl_segment_value_unsigned!(u8, u16, u32, u64, u128, usize);
impl_segment_value_float!(f32, f64);

#[derive(Clone)]
pub struct SegmentTree<T> {
    tree: Vec<T>,
    n: usize,
    combine: Arc<dyn Fn(&T, &T) -> T + Send + Sync>,
    identity: T,
}

impl<T: fmt::Debug + Clone> fmt::Debug for SegmentTree<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SegmentTree")
            .field("n", &self.n)
            .field("tree", &self.tree)
            .finish()
    }
}

impl<T: Clone + 'static> SegmentTree<T> {
    pub fn new(
        array: impl Into<Vec<T>>,
        combine: impl Fn(&T, &T) -> T + Send + Sync + 'static,
        identity: T,
    ) -> Self {
        let values = array.into();
        let n = values.len();
        let mut tree = if n == 0 {
            vec![identity.clone()]
        } else {
            vec![identity.clone(); 4 * n + 4]
        };
        let combine = Arc::new(combine);
        if n > 0 {
            build(&mut tree, &values, 1, 0, n - 1, combine.as_ref());
        }
        Self {
            tree,
            n,
            combine,
            identity,
        }
    }

    pub fn query(&self, left: usize, right: usize) -> T {
        if self.n == 0 {
            return self.identity.clone();
        }
        assert!(left <= right, "left must be <= right");
        assert!(right < self.n, "right out of bounds");
        query(
            &self.tree,
            1,
            0,
            self.n - 1,
            left,
            right,
            self.combine.as_ref(),
            &self.identity,
        )
    }

    pub fn update(&mut self, index: usize, value: T) {
        if self.n == 0 {
            return;
        }
        assert!(index < self.n, "index out of bounds");
        update(
            &mut self.tree,
            1,
            0,
            self.n - 1,
            index,
            value,
            self.combine.as_ref(),
        );
    }

    pub fn len(&self) -> usize {
        self.n
    }

    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    pub fn to_array(&self) -> Vec<T> {
        (0..self.n).map(|index| self.query(index, index)).collect()
    }
}

impl<T> SegmentTree<T>
where
    T: Clone + Add<Output = T> + Default + 'static,
{
    pub fn sum_tree(array: impl Into<Vec<T>>) -> Self {
        Self::new(array, |left, right| left.clone() + right.clone(), T::default())
    }
}

impl<T> SegmentTree<T>
where
    T: SegmentValue + Ord + Clone + 'static,
{
    pub fn min_tree(array: impl Into<Vec<T>>) -> Self {
        Self::new(
            array,
            |left, right| if left <= right { left.clone() } else { right.clone() },
            T::min_identity(),
        )
    }

    pub fn max_tree(array: impl Into<Vec<T>>) -> Self {
        Self::new(
            array,
            |left, right| if left >= right { left.clone() } else { right.clone() },
            T::max_identity(),
        )
    }
}

fn build<T: Clone>(
    tree: &mut [T],
    values: &[T],
    node: usize,
    left: usize,
    right: usize,
    combine: &dyn Fn(&T, &T) -> T,
) {
    if left == right {
        tree[node] = values[left].clone();
        return;
    }

    let mid = (left + right) / 2;
    build(tree, values, node * 2, left, mid, combine);
    build(tree, values, node * 2 + 1, mid + 1, right, combine);
    tree[node] = combine(&tree[node * 2], &tree[node * 2 + 1]);
}

fn query<T: Clone>(
    tree: &[T],
    node: usize,
    left: usize,
    right: usize,
    ql: usize,
    qr: usize,
    combine: &dyn Fn(&T, &T) -> T,
    identity: &T,
) -> T {
    if right < ql || left > qr {
        return identity.clone();
    }
    if ql <= left && right <= qr {
        return tree[node].clone();
    }

    let mid = (left + right) / 2;
    let l = query(tree, node * 2, left, mid, ql, qr, combine, identity);
    let r = query(tree, node * 2 + 1, mid + 1, right, ql, qr, combine, identity);
    combine(&l, &r)
}

fn update<T: Clone>(
    tree: &mut [T],
    node: usize,
    left: usize,
    right: usize,
    index: usize,
    value: T,
    combine: &dyn Fn(&T, &T) -> T,
) {
    if left == right {
        tree[node] = value;
        return;
    }

    let mid = (left + right) / 2;
    if index <= mid {
        update(tree, node * 2, left, mid, index, value, combine);
    } else {
        update(tree, node * 2 + 1, mid + 1, right, index, value, combine);
    }
    tree[node] = combine(&tree[node * 2], &tree[node * 2 + 1]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sum_tree_queries_and_updates() {
        let mut tree = SegmentTree::sum_tree(vec![2i64, 1, 5, 3, 4]);
        assert_eq!(tree.query(1, 3), 9);
        tree.update(2, 7);
        assert_eq!(tree.query(1, 3), 11);
        assert_eq!(tree.to_array(), vec![2, 1, 7, 3, 4]);
    }

    #[test]
    fn min_and_max_trees_work() {
        let values = vec![2i64, 1, 5, 3, 4];
        let min_tree = SegmentTree::min_tree(values.clone());
        let max_tree = SegmentTree::max_tree(values);
        assert_eq!(min_tree.query(1, 4), 1);
        assert_eq!(max_tree.query(1, 4), 5);
    }
}
