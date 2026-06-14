//! The dependency directed-acyclic-graph that drives recalc.
//!
//! Tracks "cell X depends on cells Y..." (out-edges) plus the reverse
//! "cell Y is a dependency of cells X..." (in-edges). Both directions
//! get indexed because both queries are hot during recalc.
//!
//! Cycles are detected via Tarjan's strongly-connected-components
//! algorithm. Any SCC of size > 1 is a cycle; cells in a cycle
//! evaluate to `#REF!` by default (the Phase-1 policy; Phase 2 will
//! add Excel-style iterative calculation).

use std::collections::{HashMap, HashSet};

use crate::address::{CellAddress, SheetId};

/// A fully-qualified cell address: `(sheet, address)`.
pub type Node = (SheetId, CellAddress);

/// The dependency graph.
#[derive(Debug, Clone, Default)]
pub struct DependencyGraph {
    /// `node → cells it depends on` (out-edges).
    out_edges: HashMap<Node, Vec<Node>>,
    /// `node → cells that depend on it` (in-edges, dependents).
    in_edges: HashMap<Node, Vec<Node>>,
}

impl DependencyGraph {
    /// Create an empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of nodes that have at least one out-edge.
    pub fn len(&self) -> usize {
        self.out_edges.len()
    }

    /// `true` iff no edges.
    pub fn is_empty(&self) -> bool {
        self.out_edges.is_empty()
    }

    /// Set the out-edges for `node`, replacing any existing set. The
    /// reverse `in_edges` for the previous set is cleaned up and the
    /// new set is wired in.
    pub fn set_dependencies(&mut self, node: Node, deps: Vec<Node>) {
        // Drop existing reverse edges first.
        if let Some(old) = self.out_edges.remove(&node) {
            for dep in old {
                if let Some(list) = self.in_edges.get_mut(&dep) {
                    list.retain(|n| *n != node);
                    if list.is_empty() {
                        self.in_edges.remove(&dep);
                    }
                }
            }
        }
        // Install the new edges.
        for dep in &deps {
            self.in_edges.entry(*dep).or_default().push(node);
        }
        if !deps.is_empty() {
            self.out_edges.insert(node, deps);
        }
    }

    /// Remove all edges for `node` (used when a cell becomes empty
    /// or a literal).
    pub fn remove(&mut self, node: Node) {
        self.set_dependencies(node, Vec::new());
    }

    /// Cells `node` depends on.
    pub fn dependencies(&self, node: Node) -> &[Node] {
        self.out_edges
            .get(&node)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Cells that depend on `node` (transitive consumers come via
    /// `transitive_dependents`).
    pub fn direct_dependents(&self, node: Node) -> &[Node] {
        self.in_edges
            .get(&node)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Walk the in-edges transitively from `start`. Used to compute
    /// the dirty set when a cell changes.
    pub fn transitive_dependents(&self, start: Node) -> HashSet<Node> {
        let mut out = HashSet::new();
        let mut stack = vec![start];
        while let Some(n) = stack.pop() {
            for &dep in self.direct_dependents(n) {
                if out.insert(dep) {
                    stack.push(dep);
                }
            }
        }
        out
    }

    /// Topological order of a set of nodes. Tarjan's algorithm —
    /// also reports any SCCs with size > 1 (cycles).
    ///
    /// Returns `(topo_order, cycles)`:
    /// - `topo_order` is a linear order where every cell appears
    ///   AFTER its dependencies (so iterating evaluates correctly).
    /// - `cycles` is the set of nodes that belong to an SCC of size
    ///   > 1; those should not be evaluated normally.
    pub fn topological_order(&self, nodes: &HashSet<Node>) -> (Vec<Node>, HashSet<Node>) {
        let mut state = TarjanState::new(self, nodes);
        for &n in nodes {
            if !state.indexes.contains_key(&n) {
                state.strongconnect(n);
            }
        }
        // Tarjan emits SCCs in REVERSE topological order with
        // respect to the graph's edges. Since our out-edges represent
        // "X depends on Y," a sink (a leaf with no out-edges) is a
        // cell with no dependencies. That's the cell we want to
        // evaluate FIRST. Tarjan emits leaves first, so we walk the
        // sccs vector in order — NOT reversed.
        let mut order: Vec<Node> = Vec::with_capacity(nodes.len());
        let mut cycles: HashSet<Node> = HashSet::new();
        for scc in state.sccs.iter() {
            if scc.len() > 1 {
                for n in scc {
                    cycles.insert(*n);
                }
                continue;
            }
            order.extend(scc.iter().copied());
        }
        (order, cycles)
    }
}

// ---------------------------------------------------------------------------
// Tarjan's strongly-connected-components.
// ---------------------------------------------------------------------------

struct TarjanState<'a> {
    graph: &'a DependencyGraph,
    nodes: &'a HashSet<Node>,
    indexes: HashMap<Node, usize>,
    lowlinks: HashMap<Node, usize>,
    on_stack: HashSet<Node>,
    stack: Vec<Node>,
    next_index: usize,
    sccs: Vec<Vec<Node>>,
}

impl<'a> TarjanState<'a> {
    fn new(graph: &'a DependencyGraph, nodes: &'a HashSet<Node>) -> Self {
        Self {
            graph,
            nodes,
            indexes: HashMap::new(),
            lowlinks: HashMap::new(),
            on_stack: HashSet::new(),
            stack: Vec::new(),
            next_index: 0,
            sccs: Vec::new(),
        }
    }

    /// Tarjan's SCC search, written iteratively with an explicit work
    /// stack instead of native recursion.
    ///
    /// A recursive port would descend one stack frame per node along a
    /// dependency chain, so a user could build `A1=A2`, `A2=A3`, …
    /// thousands of cells deep (each a perfectly valid formula) and the
    /// next recalc would overflow the native stack — an uncatchable
    /// abort. The explicit `work` stack moves that depth onto the heap.
    ///
    /// Each `Frame` mirrors one recursive activation: the node `v`, its
    /// in-scope successors, and `i`, how many of them we've visited.
    /// Re-entering a frame after `i` advances is the "next iteration of
    /// the for-loop"; pushing a frame is "descend into `strongconnect`";
    /// popping one and folding its lowlink into the parent is the
    /// `lowlink[v] = min(lowlink[v], lowlink[w])` the recursive caller
    /// ran *after* the recursive call returned.
    fn strongconnect(&mut self, root: Node) {
        struct Frame {
            v: Node,
            succ: Vec<Node>,
            i: usize,
        }

        self.enter(root);
        let mut work: Vec<Frame> = vec![Frame {
            v: root,
            succ: self.in_scope_deps(root),
            i: 0,
        }];

        while !work.is_empty() {
            let top = work.len() - 1;
            let v = work[top].v;

            if work[top].i < work[top].succ.len() {
                let w = work[top].succ[work[top].i];
                work[top].i += 1;

                if !self.indexes.contains_key(&w) {
                    // Unvisited successor → descend (recursive call).
                    self.enter(w);
                    let succ = self.in_scope_deps(w);
                    work.push(Frame { v: w, succ, i: 0 });
                } else if self.on_stack.contains(&w) {
                    let lv = self.lowlinks[&v];
                    let iw = self.indexes[&w];
                    self.lowlinks.insert(v, lv.min(iw));
                }
                continue;
            }

            // Every successor of v has been processed — this is the
            // point just after the recursive for-loop.
            if self.lowlinks[&v] == self.indexes[&v] {
                // v is an SCC root: pop the component off the stack.
                let mut scc = Vec::new();
                loop {
                    let w = self.stack.pop().unwrap();
                    self.on_stack.remove(&w);
                    scc.push(w);
                    if w == v {
                        break;
                    }
                }
                self.sccs.push(scc);
            }

            work.pop();
            // Fold v's lowlink into its parent (the post-return `min`).
            if let Some(parent) = work.last() {
                let p = parent.v;
                let lw = self.lowlinks[&v];
                let lp = self.lowlinks[&p];
                self.lowlinks.insert(p, lp.min(lw));
            }
        }
    }

    /// Begin visiting `v`: assign it the next index/lowlink and push it
    /// onto the SCC stack. (The prologue of the recursive call.)
    fn enter(&mut self, v: Node) {
        self.indexes.insert(v, self.next_index);
        self.lowlinks.insert(v, self.next_index);
        self.next_index += 1;
        self.stack.push(v);
        self.on_stack.insert(v);
    }

    /// `v`'s out-edges ("v depends on …"), filtered to the node set
    /// under consideration. Pre-filtering here is equivalent to the
    /// recursive version's `if !self.nodes.contains(&w) { continue; }`.
    fn in_scope_deps(&self, v: Node) -> Vec<Node> {
        self.graph
            .dependencies(v)
            .iter()
            .copied()
            .filter(|w| self.nodes.contains(w))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn node(r: u32, c: u32) -> Node {
        (SheetId(0), CellAddress::new(r, c))
    }

    #[test]
    fn empty_graph_has_no_edges() {
        let g = DependencyGraph::new();
        assert!(g.is_empty());
        assert_eq!(g.dependencies(node(1, 1)).len(), 0);
        assert_eq!(g.direct_dependents(node(1, 1)).len(), 0);
    }

    #[test]
    fn set_dependencies_indexes_both_directions() {
        let mut g = DependencyGraph::new();
        // A1 depends on B1 and B2.
        g.set_dependencies(node(1, 1), vec![node(1, 2), node(2, 2)]);
        assert_eq!(g.dependencies(node(1, 1)).len(), 2);
        assert_eq!(g.direct_dependents(node(1, 2)), &[node(1, 1)]);
        assert_eq!(g.direct_dependents(node(2, 2)), &[node(1, 1)]);
    }

    #[test]
    fn updating_dependencies_cleans_old() {
        let mut g = DependencyGraph::new();
        g.set_dependencies(node(1, 1), vec![node(1, 2)]);
        // Now A1 depends only on C1.
        g.set_dependencies(node(1, 1), vec![node(1, 3)]);
        assert!(g.direct_dependents(node(1, 2)).is_empty());
        assert_eq!(g.direct_dependents(node(1, 3)), &[node(1, 1)]);
    }

    #[test]
    fn transitive_dependents_walks_the_graph() {
        let mut g = DependencyGraph::new();
        // A1 -> B1 -> C1 (depends-on direction).
        // So the dependents of C1 transitively are B1 and A1.
        g.set_dependencies(node(1, 1), vec![node(2, 1)]);
        g.set_dependencies(node(2, 1), vec![node(3, 1)]);
        let deps = g.transitive_dependents(node(3, 1));
        assert!(deps.contains(&node(2, 1)));
        assert!(deps.contains(&node(1, 1)));
        assert_eq!(deps.len(), 2);
    }

    #[test]
    fn topological_order_acyclic() {
        let mut g = DependencyGraph::new();
        // A1 -> B1 -> C1.
        // Topo order: C1 before B1 before A1.
        g.set_dependencies(node(1, 1), vec![node(2, 1)]);
        g.set_dependencies(node(2, 1), vec![node(3, 1)]);
        let set: HashSet<Node> =
            [node(1, 1), node(2, 1), node(3, 1)].into_iter().collect();
        let (order, cycles) = g.topological_order(&set);
        assert!(cycles.is_empty());
        assert_eq!(order.len(), 3);
        let pos_a = order.iter().position(|&n| n == node(1, 1)).unwrap();
        let pos_b = order.iter().position(|&n| n == node(2, 1)).unwrap();
        let pos_c = order.iter().position(|&n| n == node(3, 1)).unwrap();
        assert!(pos_c < pos_b);
        assert!(pos_b < pos_a);
    }

    #[test]
    fn topological_order_detects_cycle() {
        let mut g = DependencyGraph::new();
        // A1 <-> B1.
        g.set_dependencies(node(1, 1), vec![node(2, 1)]);
        g.set_dependencies(node(2, 1), vec![node(1, 1)]);
        let set: HashSet<Node> = [node(1, 1), node(2, 1)].into_iter().collect();
        let (order, cycles) = g.topological_order(&set);
        assert!(order.is_empty());
        assert!(cycles.contains(&node(1, 1)));
        assert!(cycles.contains(&node(2, 1)));
    }

    #[test]
    fn remove_clears_edges() {
        let mut g = DependencyGraph::new();
        g.set_dependencies(node(1, 1), vec![node(1, 2)]);
        g.remove(node(1, 1));
        assert!(g.dependencies(node(1, 1)).is_empty());
        assert!(g.direct_dependents(node(1, 2)).is_empty());
    }

    #[test]
    fn long_dependency_chain_does_not_overflow_the_stack() {
        // Build A1 → A2 → … → A_N (each cell depends on the next), a
        // chain far deeper than the native stack could hold one frame
        // per node. The iterative Tarjan must order all N nodes with no
        // spurious cycle — a recursive implementation would abort here.
        const N: u32 = 100_000;
        let mut g = DependencyGraph::new();
        for r in 1..N {
            g.set_dependencies(node(r, 1), vec![node(r + 1, 1)]);
        }
        let all: HashSet<Node> = (1..=N).map(|r| node(r, 1)).collect();
        let (order, cycles) = g.topological_order(&all);
        assert!(cycles.is_empty(), "linear chain must be acyclic");
        assert_eq!(order.len() as u32, N);
        // The deepest cell (no dependencies) must be evaluated first.
        assert_eq!(order[0], node(N, 1));
        assert_eq!(*order.last().unwrap(), node(1, 1));
    }

    #[test]
    fn long_cycle_is_detected_without_overflow() {
        // A long chain that loops back on itself is one big SCC; cycle
        // detection must report every node, again without recursing.
        const N: u32 = 50_000;
        let mut g = DependencyGraph::new();
        for r in 1..N {
            g.set_dependencies(node(r, 1), vec![node(r + 1, 1)]);
        }
        g.set_dependencies(node(N, 1), vec![node(1, 1)]); // close the loop
        let all: HashSet<Node> = (1..=N).map(|r| node(r, 1)).collect();
        let (order, cycles) = g.topological_order(&all);
        assert!(order.is_empty());
        assert_eq!(cycles.len() as u32, N);
    }
}
