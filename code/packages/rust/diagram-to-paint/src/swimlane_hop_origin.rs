//! Exact, bounded candidate discovery for integer segments with one common
//! left support. This module never decides whether a hop is painted: selected
//! ordinals return to the unchanged directed floating-point predicate.
//!
//! A positive intersection requires opposite start/slope ordering. Subtree
//! extrema also bound the two right endpoints, rejecting intersections beyond
//! either segment. These conservative bounds can admit false positives; they
//! are not a general output-sensitive work guarantee.

use std::cmp::Ordering;

/// Numeric equality is cross-product equality, including unreduced fractions.
/// Deliberately provide no raw-pair Eq/Ord implementation.
#[derive(Clone, Copy, Debug)]
pub(super) struct Slope {
    pub(super) numerator: i64,
    pub(super) denominator: i64,
}

impl Slope {
    pub(super) fn new(numerator: i64, denominator: i64) -> Option<Self> {
        const LIMIT: i64 = 1 << 25;
        if !(-LIMIT..=LIMIT).contains(&numerator) || !(1..=LIMIT).contains(&denominator) {
            return None;
        }
        Some(Self { numerator, denominator })
    }

    pub(super) fn compare(self, other: Self) -> Ordering {
        count(Work::Compare);
        (i128::from(self.numerator) * i128::from(other.denominator))
            .cmp(&(i128::from(other.numerator) * i128::from(self.denominator)))
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Projection {
    pub(super) origin: i64,
    pub(super) start: i64,
    pub(super) end: i64,
    pub(super) slope: Slope,
}

pub(super) fn project(from: &super::Point, to: &super::Point, exchange: bool) -> Option<Projection> {
    count(Work::Certificate);
    let ([left, right], start, end) = super::swimlane_hop_integer_supports(from, to, exchange)?;
    Some(Projection { origin: left, start, end, slope: Slope::new(end - start, right - left)? })
}

/// Counters describe actual operations of this bounded index. Occurrence counts
/// also cover the shared collector. Geometry-key comparisons, cut generation,
/// and arbitrary floating fallback work are separate, still-open contracts.
#[derive(Clone, Copy, Debug)]
pub(super) enum Work {
    Certificate, Observe, Insert, Refresh, Query, Compare, Rotation, Drop,
    Candidate, Sort, Occurrence,
}

#[cfg(test)]
std::thread_local! {
    static WORK: std::cell::Cell<[usize; 11]> = const { std::cell::Cell::new([0; 11]) };
}

pub(super) fn count(kind: Work) {
    #[cfg(test)]
    {
        super::swimlane_hop_index_work();
        WORK.with(|work| {
            let mut values = work.get();
            values[kind as usize] = values[kind as usize].checked_add(1).expect("hop work counter overflow");
            work.set(values);
        });
    }
    #[cfg(not(test))]
    let _ = kind;
}

#[cfg(test)]
pub(super) fn reset_work() { WORK.with(|work| work.set([0; 11])); }

#[cfg(test)]
pub(super) fn work_snapshot() -> [usize; 11] { WORK.with(std::cell::Cell::get) }

fn compare(left: i64, right: i64) -> Ordering {
    count(Work::Compare);
    left.cmp(&right)
}

/// Own an AVL node: sorted insertions cannot make either query recursion or
/// destruction linear in the number of prior geometries. Every rotation repairs
/// *all* extrema from the entry and its children before returning its new root.
pub(super) struct Node {
    pub(super) entry: Projection,
    pub(super) ordinal: usize,
    pub(super) height: usize,
    pub(super) left: Option<Box<Node>>,
    pub(super) right: Option<Box<Node>>,
    pub(super) min_start: i64,
    pub(super) max_start: i64,
    pub(super) min_end: i64,
    pub(super) max_end: i64,
    pub(super) min_dx: i64,
    pub(super) max_dx: i64,
    pub(super) min_slope: Slope,
    pub(super) max_slope: Slope,
}

impl Drop for Node {
    fn drop(&mut self) { count(Work::Drop); }
}

impl Node {
    fn height(node: &Option<Box<Self>>) -> usize { node.as_ref().map_or(0, |node| node.height) }

    fn refresh(&mut self) {
        count(Work::Refresh);
        self.height = 1 + Self::height(&self.left).max(Self::height(&self.right));
        self.min_start = self.entry.start;
        self.max_start = self.entry.start;
        self.min_end = self.entry.end;
        self.max_end = self.entry.end;
        self.min_dx = self.entry.slope.denominator;
        self.max_dx = self.entry.slope.denominator;
        self.min_slope = self.entry.slope;
        self.max_slope = self.entry.slope;
        for child in [&self.left, &self.right].into_iter().flatten() {
            if compare(child.min_start, self.min_start).is_lt() { self.min_start = child.min_start; }
            if compare(child.max_start, self.max_start).is_gt() { self.max_start = child.max_start; }
            if compare(child.min_end, self.min_end).is_lt() { self.min_end = child.min_end; }
            if compare(child.max_end, self.max_end).is_gt() { self.max_end = child.max_end; }
            if compare(child.min_dx, self.min_dx).is_lt() { self.min_dx = child.min_dx; }
            if compare(child.max_dx, self.max_dx).is_gt() { self.max_dx = child.max_dx; }
            if child.min_slope.compare(self.min_slope).is_lt() { self.min_slope = child.min_slope; }
            if child.max_slope.compare(self.max_slope).is_gt() { self.max_slope = child.max_slope; }
        }
    }

    fn rotate_left(mut root: Box<Self>) -> Box<Self> {
        count(Work::Rotation);
        let mut next = root.right.take().expect("AVL right child");
        root.right = next.left.take();
        root.refresh();
        next.left = Some(root);
        next.refresh();
        next
    }

    fn rotate_right(mut root: Box<Self>) -> Box<Self> {
        count(Work::Rotation);
        let mut next = root.left.take().expect("AVL left child");
        root.left = next.right.take();
        root.refresh();
        next.right = Some(root);
        next.refresh();
        next
    }

    pub(super) fn insert(tree: Option<Box<Self>>, entry: Projection, ordinal: usize) -> Box<Self> {
        count(Work::Insert);
        let Some(mut root) = tree else {
            return Box::new(Self { entry, ordinal, height: 1, left: None, right: None,
                min_start: entry.start, max_start: entry.start, min_end: entry.end, max_end: entry.end,
                min_dx: entry.slope.denominator, max_dx: entry.slope.denominator,
                min_slope: entry.slope, max_slope: entry.slope });
        };
        count(Work::Compare);
        if (entry.start, ordinal) < (root.entry.start, root.ordinal) {
            root.left = Some(Self::insert(root.left.take(), entry, ordinal));
        } else {
            root.right = Some(Self::insert(root.right.take(), entry, ordinal));
        }
        root.refresh();
        let left = Self::height(&root.left);
        let right = Self::height(&root.right);
        if left > right + 1 {
            let child = root.left.as_ref().expect("AVL left child");
            if Self::height(&child.right) > Self::height(&child.left) {
                root.left = root.left.take().map(Self::rotate_left);
            }
            return Self::rotate_right(root);
        }
        if right > left + 1 {
            let child = root.right.as_ref().expect("AVL right child");
            if Self::height(&child.left) > Self::height(&child.right) {
                root.right = root.right.take().map(Self::rotate_right);
            }
            return Self::rotate_left(root);
        }
        root
    }

    fn candidates(&self, query: Projection, below: bool, output: &mut Vec<usize>) {
        count(Work::Query);
        // At the query's right support, bound start + slope*qdx.
        // At a prior right support, bound end - querySlope*priorDx.
        // Positive denominators preserve every sign; all arithmetic is i128.
        let possible = if below {
            compare(self.min_start, query.start).is_lt()
                && self.max_slope.compare(query.slope).is_gt()
                && at_query_right(self.max_start, self.max_slope, query).is_gt()
                && at_prior_right(self.max_end,
                    if query.slope.numerator >= 0 { self.min_dx } else { self.max_dx }, query).is_gt()
        } else {
            compare(self.max_start, query.start).is_gt()
                && self.min_slope.compare(query.slope).is_lt()
                && at_query_right(self.min_start, self.min_slope, query).is_lt()
                && at_prior_right(self.min_end,
                    if query.slope.numerator >= 0 { self.max_dx } else { self.min_dx }, query).is_lt()
        };
        if !possible { return; }
        if let Some(left) = &self.left { left.candidates(query, below, output); }
        let start = compare(self.entry.start, query.start);
        let slope = self.entry.slope.compare(query.slope);
        let strict_inversion = if below { start.is_lt() && slope.is_gt() } else { start.is_gt() && slope.is_lt() };
        if strict_inversion {
            let query_end = at_query_right(self.entry.start, self.entry.slope, query);
            let prior_end = at_prior_right(self.entry.end, self.entry.slope.denominator, query);
            if if below { query_end.is_gt() && prior_end.is_gt() } else { query_end.is_lt() && prior_end.is_lt() } {
                count(Work::Candidate);
                output.push(self.ordinal);
            }
        }
        if let Some(right) = &self.right { right.candidates(query, below, output); }
    }
}

fn at_query_right(start: i64, slope: Slope, query: Projection) -> Ordering {
    count(Work::Compare);
    ((i128::from(start) - i128::from(query.end)) * i128::from(slope.denominator)
        + i128::from(slope.numerator) * i128::from(query.slope.denominator)).cmp(&0)
}

fn at_prior_right(end: i64, dx: i64, query: Projection) -> Ordering {
    count(Work::Compare);
    ((i128::from(end) - i128::from(query.start)) * i128::from(query.slope.denominator)
        - i128::from(query.slope.numerator) * i128::from(dx)).cmp(&0)
}

pub(super) struct Cohort {
    origin: i64,
    tree: Option<Box<Node>>,
}

pub(super) fn observe(cohort: &mut Option<Cohort>, first: bool, entry: Option<Projection>, ordinal: usize) {
    count(Work::Observe);
    if first {
        *cohort = entry.map(|entry| Cohort { origin: entry.origin, tree: Some(Node::insert(None, entry, ordinal)) });
    } else if let Some(index) = cohort {
        if let Some(entry) = entry.filter(|entry| compare(entry.origin, index.origin).is_eq()) {
            index.tree = Some(Node::insert(index.tree.take(), entry, ordinal));
        } else {
            // Disqualification is permanent: never rebuild from the old vector.
            *cohort = None;
        }
    }
}

impl Cohort {
    pub(super) fn candidates(&self, query: Projection) -> Option<Vec<usize>> {
        if !compare(query.origin, self.origin).is_eq() { return None; }
        let mut output = Vec::new();
        if let Some(tree) = &self.tree {
            tree.candidates(query, true, &mut output);
            tree.candidates(query, false, &mut output);
        }
        // The tree is keyed by ordinate, but the legacy collector walks first
        // observed geometry ordinals. Restore that order before the predicate.
        count(Work::Sort);
        output.sort_unstable_by(|left, right| { count(Work::Compare); left.cmp(right) });
        Some(output)
    }
}
