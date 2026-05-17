//! Dependency graph for formula cells.
//!
//! A spreadsheet is fundamentally a directed graph: cell B1 *depends on* A1
//! if B1's formula references A1.  When A1 changes, B1 must be recalculated.
//! And if C1 depends on B1, it must be recalculated after B1, in the right
//! order.
//!
//! # Topological sort
//!
//! Computing cells in the right order is called *topological sorting*.
//! Imagine listing all the tasks where some tasks must be done before others —
//! a topological sort gives you a valid completion order.
//!
//! We use **Kahn's algorithm** (1962):
//!
//! 1. Count the number of *incoming edges* (dependencies) for every node.
//! 2. Put all nodes with zero incoming edges in a queue (they have no deps
//!    to wait for).
//! 3. Repeatedly: pick a node from the queue, "process" it, then reduce the
//!    in-degree of all nodes it points to.  When a node reaches in-degree 0,
//!    add it to the queue.
//! 4. If the queue empties before all nodes are processed, a cycle exists.
//!
//! Kahn's algorithm runs in O(V + E) time where V = cells and E = dependency
//! edges.
//!
//! # Cycle detection
//!
//! If the topological sort completes without visiting every node, the
//! remaining unvisited nodes form cycle(s).  We return them as the cycle set
//! and the caller marks them with `#CIRC`.

use std::collections::{HashMap, HashSet, VecDeque};
use crate::addr::CellAddr;

/// The dependency graph.
///
/// `deps[A] = {B, C}` means "cell A's formula references cells B and C".
/// In graph terms, A has outgoing edges to B and C, and B and C have an
/// incoming edge from A.
///
/// We also store the *reverse* map (`rdeps[B] = {A}`) so that when B
/// changes, we can quickly find all cells that need to be re-evaluated.
#[derive(Debug, Default)]
pub struct DependencyGraph {
    /// Forward edges: cell → set of cells it depends on.
    deps: HashMap<CellAddr, HashSet<CellAddr>>,
    /// Reverse edges: cell → set of cells that depend on it.
    rdeps: HashMap<CellAddr, HashSet<CellAddr>>,
}

impl DependencyGraph {
    /// Create an empty dependency graph.
    pub fn new() -> Self {
        DependencyGraph::default()
    }

    /// Update the dependencies for `cell`.
    ///
    /// `new_deps` is the complete set of cells that `cell`'s formula
    /// references.  Any previous dependencies are removed first.
    pub fn set_deps(&mut self, cell: &CellAddr, new_deps: &[CellAddr]) {
        // Remove old reverse edges.
        if let Some(old) = self.deps.get(cell).cloned() {
            for dep in &old {
                if let Some(rev) = self.rdeps.get_mut(dep) {
                    rev.remove(cell);
                }
            }
        }

        // Insert new forward edges.
        let new_set: HashSet<CellAddr> = new_deps.iter().cloned().collect();
        for dep in &new_set {
            self.rdeps.entry(dep.clone()).or_default().insert(cell.clone());
        }
        self.deps.insert(cell.clone(), new_set);
    }

    /// Remove all dependencies for a cell (called when the cell becomes a
    /// literal rather than a formula).
    pub fn clear_deps(&mut self, cell: &CellAddr) {
        self.set_deps(cell, &[]);
        self.deps.remove(cell);
    }

    /// Return all cells that directly or indirectly depend on `cell`.
    ///
    /// Used to find the full dirty set when a cell value changes.
    pub fn transitive_dependents(&self, cell: &CellAddr) -> HashSet<CellAddr> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(cell.clone());
        while let Some(current) = queue.pop_front() {
            if let Some(rev) = self.rdeps.get(&current) {
                for dep in rev {
                    if visited.insert(dep.clone()) {
                        queue.push_back(dep.clone());
                    }
                }
            }
        }
        visited
    }

    /// Topologically sort `dirty_cells` so that each cell appears *after* all
    /// cells it depends on.
    ///
    /// Returns `(ordered, cycle_members)` where:
    /// - `ordered` — cells that can be safely evaluated in this order.
    /// - `cycle_members` — cells that are part of a cycle and cannot be
    ///   evaluated.
    ///
    /// If `cycle_members` is non-empty, those cells should be assigned
    /// `CellValue::Error(FormulaError::Circ)`.
    pub fn topological_sort(
        &self,
        dirty_cells: &HashSet<CellAddr>,
    ) -> (Vec<CellAddr>, HashSet<CellAddr>) {
        // We only care about dirty cells — other cells don't need to change.
        //
        // Build a sub-graph restricted to dirty cells.  In-degree counts
        // how many dependencies of a dirty cell are also dirty.

        // in_degree[C] = number of dirty cells that C depends on.
        let mut in_degree: HashMap<CellAddr, usize> = HashMap::new();
        for cell in dirty_cells {
            in_degree.entry(cell.clone()).or_insert(0);
            if let Some(cell_deps) = self.deps.get(cell) {
                for dep in cell_deps {
                    if dirty_cells.contains(dep) {
                        *in_degree.entry(cell.clone()).or_insert(0) += 1;
                    }
                }
            }
        }

        // Start with all dirty cells that have zero in-degree.
        let mut queue: VecDeque<CellAddr> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(cell, _)| cell.clone())
            .collect();

        // Sort the initial queue for deterministic output.
        let mut q_vec: Vec<CellAddr> = queue.drain(..).collect();
        q_vec.sort_by_key(|a| (a.col, a.row));
        queue.extend(q_vec);

        let mut ordered = Vec::new();

        while let Some(cell) = queue.pop_front() {
            ordered.push(cell.clone());

            // Find dirty cells that depend on `cell` (reverse edges restricted
            // to the dirty set).
            if let Some(rev) = self.rdeps.get(&cell) {
                let mut next: Vec<CellAddr> = rev
                    .iter()
                    .filter(|dep| dirty_cells.contains(*dep))
                    .cloned()
                    .collect();
                // Sort for determinism.
                next.sort_by_key(|a| (a.col, a.row));
                for dep in next {
                    let deg = in_degree.entry(dep.clone()).or_insert(0);
                    if *deg > 0 {
                        *deg -= 1;
                    }
                    if *deg == 0 {
                        queue.push_back(dep);
                    }
                }
            }
        }

        // Any dirty cell not in `ordered` is part of a cycle.
        let cycle_members: HashSet<CellAddr> = dirty_cells
            .iter()
            .filter(|cell| !ordered.contains(cell))
            .cloned()
            .collect();

        (ordered, cycle_members)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(s: &str) -> CellAddr {
        CellAddr::parse(s).unwrap()
    }

    #[test]
    fn test_topological_sort_simple() {
        // A1 → B1 → C1  (A1 must be evaluated first)
        let mut graph = DependencyGraph::new();
        graph.set_deps(&addr("B1"), &[addr("A1")]);
        graph.set_deps(&addr("C1"), &[addr("B1")]);

        let dirty: HashSet<CellAddr> = [addr("A1"), addr("B1"), addr("C1")].into();
        let (ordered, cycles) = graph.topological_sort(&dirty);
        assert!(cycles.is_empty());
        // A1 before B1, B1 before C1.
        let pos: HashMap<_, _> = ordered.iter().enumerate().map(|(i, a)| (a.clone(), i)).collect();
        assert!(pos[&addr("A1")] < pos[&addr("B1")]);
        assert!(pos[&addr("B1")] < pos[&addr("C1")]);
    }

    #[test]
    fn test_cycle_detection() {
        // A1 depends on B1, B1 depends on A1 — a 2-cycle.
        let mut graph = DependencyGraph::new();
        graph.set_deps(&addr("A1"), &[addr("B1")]);
        graph.set_deps(&addr("B1"), &[addr("A1")]);

        let dirty: HashSet<CellAddr> = [addr("A1"), addr("B1")].into();
        let (ordered, cycles) = graph.topological_sort(&dirty);
        assert!(ordered.is_empty());
        assert!(cycles.contains(&addr("A1")));
        assert!(cycles.contains(&addr("B1")));
    }

    #[test]
    fn test_transitive_dependents() {
        // A1 ← B1 ← C1  (C1 depends on B1 which depends on A1)
        // When A1 changes, both B1 and C1 are affected.
        let mut graph = DependencyGraph::new();
        graph.set_deps(&addr("B1"), &[addr("A1")]);
        graph.set_deps(&addr("C1"), &[addr("B1")]);

        let affected = graph.transitive_dependents(&addr("A1"));
        assert!(affected.contains(&addr("B1")));
        assert!(affected.contains(&addr("C1")));
        // A1 itself is NOT in the dependents set.
        assert!(!affected.contains(&addr("A1")));
    }

    #[test]
    fn test_update_deps_removes_old_edges() {
        let mut graph = DependencyGraph::new();
        // B1 depends on A1.
        graph.set_deps(&addr("B1"), &[addr("A1")]);
        // Change B1 to depend on C1 instead.
        graph.set_deps(&addr("B1"), &[addr("C1")]);

        // A1's rdeps should no longer contain B1.
        let affected_by_a1 = graph.transitive_dependents(&addr("A1"));
        assert!(!affected_by_a1.contains(&addr("B1")));

        // C1's rdeps should now contain B1.
        let affected_by_c1 = graph.transitive_dependents(&addr("C1"));
        assert!(affected_by_c1.contains(&addr("B1")));
    }
}
