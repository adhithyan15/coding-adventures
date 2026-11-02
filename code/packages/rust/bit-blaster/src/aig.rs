//! # And-inverter graphs
//!
//! Every Boolean function can be built from two-input AND gates and NOT.
//! An **and-inverter graph** (AIG) is a circuit made of exactly those: a
//! directed acyclic graph whose internal nodes are ANDs and whose edges may
//! carry an inverter. Hardware verification tools (ABC, AIGER, every
//! equivalence checker) use AIGs because they are tiny, uniform and easy to
//! simplify. VER18's gate-level CPUs and VER19's bit-blasted integers both
//! become AIGs before they become clauses.
//!
//! ```text
//!        out = ¬(a ∧ b) ∧ c                     node 0: FALSE
//!                                               node 1: input a
//!         c ───────────┐                        node 2: input b
//!                      AND ── out               node 3: input c
//!   a ──┐              │                        node 4: AND(a, b)
//!       AND ──o────────┘   (o = inverter)       node 5: AND(¬node4, c)
//!   b ──┘
//! ```
//!
//! ## Edges are literals
//!
//! An [`Edge`] is a node index with a complement bit, packed as
//! `2·node + complement`, the same packing a SAT literal uses (VER09 §3.1).
//! Node 0 is the constant `false`, so edge 0 is `FALSE`, edge 1 is `TRUE`,
//! and inverting an edge flips its low bit.
//!
//! ## Never build the same gate twice
//!
//! [`Aig::and`] simplifies before it builds (VER10 §2.2): constants fold,
//! `a ∧ a = a`, `a ∧ ¬a = false`, and an AND of two operands that already
//! has a node returns that node (**structural hashing**). The result is that
//! two syntactically identical sub-circuits are one sub-circuit, which
//! makes miters of near-identical designs collapse before a solver ever
//! sees them.
//!
//! ## Nodes are in topological order
//!
//! A node is created only after its operands, so a node's index is greater
//! than its operands' indices. Evaluating nodes in index order is therefore
//! always valid, with no recursion. [`Aig::simulate`] does exactly that,
//! on 64 input patterns at once.

use std::collections::HashMap;
use std::ops::Not;

/// A possibly-inverted reference to an AIG node: `2·node + complement`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Edge(u32);

impl Edge {
    /// The constant `false`.
    pub const FALSE: Edge = Edge(0);
    /// The constant `true`.
    pub const TRUE: Edge = Edge(1);

    /// The edge for a Boolean constant.
    pub fn constant(value: bool) -> Edge {
        if value {
            Edge::TRUE
        } else {
            Edge::FALSE
        }
    }

    /// The node this edge points at.
    pub fn node(self) -> usize {
        (self.0 >> 1) as usize
    }

    /// Whether the edge carries an inverter.
    pub fn is_complemented(self) -> bool {
        self.0 & 1 == 1
    }

    /// Whether the edge is one of the two constants.
    pub fn is_constant(self) -> bool {
        self.node() == 0
    }

    fn new(node: usize, complemented: bool) -> Edge {
        assert!(node < (1 << 31), "AIG exceeds 2^31 nodes");
        Edge(((node as u32) << 1) | complemented as u32)
    }
}

impl Not for Edge {
    type Output = Edge;

    /// Add (or remove) an inverter.
    fn not(self) -> Edge {
        Edge(self.0 ^ 1)
    }
}

/// What a node is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Node {
    /// Node 0, the constant `false`.
    False,
    /// The `i`-th input.
    Input(usize),
    /// The AND of two edges, in canonical order (first ≤ second).
    And(Edge, Edge),
}

/// An and-inverter graph.
#[derive(Debug, Clone)]
pub struct Aig {
    nodes: Vec<Node>,
    /// `input_nodes[i]` is the node of input `i`.
    input_nodes: Vec<usize>,
    /// Structural hash: operands → existing AND node.
    strash: HashMap<(Edge, Edge), usize>,
    /// How many times [`Aig::and`] has been called, including calls that
    /// simplified away or hit the structural hash. Node count measures the
    /// circuit; this measures the *work* of building it, which a caller
    /// facing hostile input can bound.
    and_calls: u64,
}

impl Default for Aig {
    fn default() -> Self {
        Aig::new()
    }
}

impl Aig {
    /// An AIG holding only the constant node.
    pub fn new() -> Aig {
        Aig { nodes: vec![Node::False], input_nodes: Vec::new(), strash: HashMap::new(), and_calls: 0 }
    }

    /// A fresh input. Inputs are numbered `0, 1, 2, …` in creation order.
    pub fn input(&mut self) -> Edge {
        let node = self.nodes.len();
        self.nodes.push(Node::Input(self.input_nodes.len()));
        self.input_nodes.push(node);
        Edge::new(node, false)
    }

    /// Number of inputs.
    pub fn num_inputs(&self) -> usize {
        self.input_nodes.len()
    }

    /// The (positive) edge of input `i`, if it exists.
    pub fn input_edge(&self, i: usize) -> Option<Edge> {
        self.input_nodes.get(i).map(|&n| Edge::new(n, false))
    }

    /// Number of nodes (the constant, inputs and ANDs).
    pub fn num_nodes(&self) -> usize {
        self.nodes.len()
    }

    /// Number of AND gates.
    pub fn num_ands(&self) -> usize {
        self.nodes.len() - 1 - self.input_nodes.len()
    }

    /// Number of [`Aig::and`] calls so far (the work of construction).
    pub fn and_calls(&self) -> u64 {
        self.and_calls
    }

    /// What node `index` is.
    pub fn node(&self, index: usize) -> Node {
        self.nodes[index]
    }

    /// `a ∧ b`, simplified and structurally hashed (VER10 §2.2).
    pub fn and(&mut self, a: Edge, b: Edge) -> Edge {
        self.and_calls += 1;
        let (a, b) = if a <= b { (a, b) } else { (b, a) };
        // Smallest edge first, so constants (node 0) come first.
        if a == Edge::FALSE {
            return Edge::FALSE;
        }
        if a == Edge::TRUE {
            return b;
        }
        if a == b {
            return a;
        }
        if a == !b {
            return Edge::FALSE;
        }
        if let Some(&node) = self.strash.get(&(a, b)) {
            return Edge::new(node, false);
        }
        let node = self.nodes.len();
        self.nodes.push(Node::And(a, b));
        self.strash.insert((a, b), node);
        Edge::new(node, false)
    }

    /// `a ∨ b = ¬(¬a ∧ ¬b)` (De Morgan).
    pub fn or(&mut self, a: Edge, b: Edge) -> Edge {
        !self.and(!a, !b)
    }

    /// `a ⊕ b = (a ∨ b) ∧ ¬(a ∧ b)`: three AND gates.
    pub fn xor(&mut self, a: Edge, b: Edge) -> Edge {
        let either = self.or(a, b);
        let both = self.and(a, b);
        self.and(either, !both)
    }

    /// `a ⇔ b`, the complement of XOR.
    pub fn xnor(&mut self, a: Edge, b: Edge) -> Edge {
        !self.xor(a, b)
    }

    /// `a ⇒ b = ¬a ∨ b`.
    pub fn implies(&mut self, a: Edge, b: Edge) -> Edge {
        self.or(!a, b)
    }

    /// `if s then t else e = (s ∧ t) ∨ (¬s ∧ e)`.
    pub fn mux(&mut self, s: Edge, t: Edge, e: Edge) -> Edge {
        if t == e {
            return t;
        }
        let when_true = self.and(s, t);
        let when_false = self.and(!s, e);
        self.or(when_true, when_false)
    }

    /// The AND of many edges (`TRUE` for none).
    pub fn and_all(&mut self, edges: &[Edge]) -> Edge {
        edges.iter().fold(Edge::TRUE, |acc, &e| self.and(acc, e))
    }

    /// The OR of many edges (`FALSE` for none).
    pub fn or_all(&mut self, edges: &[Edge]) -> Edge {
        edges.iter().fold(Edge::FALSE, |acc, &e| self.or(acc, e))
    }

    /// Evaluate every node on 64 input patterns at once.
    ///
    /// `inputs[i]` holds input `i`'s value in each of 64 patterns, one per
    /// bit. The result holds every node's 64 values, indexed by node; read
    /// an edge's values with [`edge_value`]. Nodes are evaluated in index
    /// order, which is topological (module docs).
    ///
    /// # Panics
    ///
    /// Panics if `inputs` is shorter than [`Aig::num_inputs`].
    pub fn simulate(&self, inputs: &[u64]) -> Vec<u64> {
        assert!(inputs.len() >= self.num_inputs(), "one input word per input is required");
        let mut values = Vec::with_capacity(self.nodes.len());
        for node in &self.nodes {
            let v = match *node {
                Node::False => 0,
                Node::Input(i) => inputs[i],
                Node::And(a, b) => edge_value(&values, a) & edge_value(&values, b),
            };
            values.push(v);
        }
        values
    }

    /// Evaluate one edge on one input assignment.
    pub fn eval(&self, edge: Edge, inputs: &[bool]) -> bool {
        let words: Vec<u64> = inputs.iter().map(|&b| b as u64).collect();
        let values = self.simulate(&words);
        edge_value(&values, edge) & 1 == 1
    }
}

/// The 64 simulated values of `edge`, given every node's values.
pub fn edge_value(values: &[u64], edge: Edge) -> u64 {
    let v = values[edge.node()];
    if edge.is_complemented() {
        !v
    } else {
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_and_packing() {
        assert_eq!(!Edge::FALSE, Edge::TRUE);
        assert!(Edge::TRUE.is_constant() && Edge::TRUE.is_complemented());
        assert_eq!(Edge::constant(true), Edge::TRUE);
        assert_eq!(Edge::constant(false), Edge::FALSE);
        let mut g = Aig::default();
        let a = g.input();
        assert_eq!(a.node(), 1);
        assert!(!a.is_constant());
        assert_eq!(g.node(1), Node::Input(0));
        assert_eq!(g.node(0), Node::False);
        assert_eq!(g.input_edge(0), Some(a));
        assert_eq!(g.input_edge(1), None);
    }

    #[test]
    fn and_simplifications() {
        let mut g = Aig::new();
        let a = g.input();
        let b = g.input();
        assert_eq!(g.and(a, Edge::FALSE), Edge::FALSE);
        assert_eq!(g.and(Edge::TRUE, a), a);
        assert_eq!(g.and(a, a), a);
        assert_eq!(g.and(a, !a), Edge::FALSE);
        assert_eq!(g.num_ands(), 0);
        let ab = g.and(a, b);
        assert_eq!(g.and(b, a), ab); // structural hashing, either order
        assert_eq!(g.num_ands(), 1);
        assert_eq!(g.and_calls(), 6, "every call counts, hashed or simplified");
        assert_eq!(g.node(ab.node()), Node::And(a, b));
        assert_eq!(g.num_nodes(), 4);
        assert_eq!(g.num_inputs(), 2);
    }

    #[test]
    fn derived_gates_truth_tables() {
        let mut g = Aig::new();
        let a = g.input();
        let b = g.input();
        let s = g.input();
        let gates = [g.or(a, b), g.xor(a, b), g.xnor(a, b), g.implies(a, b), g.mux(s, a, b)];
        assert_eq!(g.mux(s, a, a), a);
        for bits in 0..8u8 {
            let (va, vb, vs) = (bits & 1 == 1, bits & 2 == 2, bits & 4 == 4);
            let ins = [va, vb, vs];
            assert_eq!(g.eval(gates[0], &ins), va || vb);
            assert_eq!(g.eval(gates[1], &ins), va ^ vb);
            assert_eq!(g.eval(gates[2], &ins), va == vb);
            assert_eq!(g.eval(gates[3], &ins), !va || vb);
            assert_eq!(g.eval(gates[4], &ins), if vs { va } else { vb });
        }
    }

    #[test]
    fn many_input_gates() {
        let mut g = Aig::new();
        let xs: Vec<Edge> = (0..4).map(|_| g.input()).collect();
        let all = g.and_all(&xs);
        let any = g.or_all(&xs);
        assert_eq!(g.and_all(&[]), Edge::TRUE);
        assert_eq!(g.or_all(&[]), Edge::FALSE);
        for bits in 0..16u32 {
            let ins: Vec<bool> = (0..4).map(|i| bits >> i & 1 == 1).collect();
            assert_eq!(g.eval(all, &ins), bits == 15);
            assert_eq!(g.eval(any, &ins), bits != 0);
        }
    }

    #[test]
    fn simulation_is_bit_parallel() {
        let mut g = Aig::new();
        let a = g.input();
        let b = g.input();
        let x = g.xor(a, b);
        let values = g.simulate(&[0b1100, 0b1010]);
        assert_eq!(edge_value(&values, x) & 0xF, 0b0110);
        assert_eq!(edge_value(&values, !x) & 0xF, 0b1001);
    }

    #[test]
    #[should_panic(expected = "one input word per input")]
    fn simulate_needs_every_input() {
        let mut g = Aig::new();
        g.input();
        g.simulate(&[]);
    }
}
