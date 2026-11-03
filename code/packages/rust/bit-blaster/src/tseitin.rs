//! # Tseitin encoding: circuits to clauses, in linear size
//!
//! A SAT solver wants CNF: an AND of ORs. The textbook way to turn a
//! formula into CNF (distribute ∨ over ∧) can blow up exponentially;
//! `(a₁ ∧ b₁) ∨ … ∨ (aₙ ∧ bₙ)` has 2ⁿ clauses. Tseitin's 1968 trick avoids
//! that: give **every gate its own variable** and write down only what each
//! gate does. The result is not *equivalent* to the circuit (it has extra
//! variables) but it is **equisatisfiable**, and better: in every model, each
//! gate variable holds exactly the value the gate computes.
//!
//! For an AND node `n = a ∧ b`:
//!
//! ```text
//!   (¬n ∨ a)        if n then a
//!   (¬n ∨ b)        if n then b
//!   (n ∨ ¬a ∨ ¬b)   if a and b then n
//! ```
//!
//! Three clauses per gate, so the CNF is linear in the circuit. Inverted
//! edges cost nothing: the literal of `¬e` is the negation of the literal of
//! `e`, which is why AIG edges and SAT literals share one packing.
//!
//! ## Only the cone, only once
//!
//! [`Tseitin::lit`] encodes the **cone** of an edge: the nodes it depends on,
//! and nothing else. It remembers what it has encoded, so asking for a
//! second edge only adds the nodes not already encoded. That is what
//! incremental bounded model checking needs (VER18): unroll one more step,
//! encode only the new part, keep everything the solver learned.
//!
//! The walk is an explicit stack, not recursion, so a deep circuit (a 4096-bit
//! ripple-carry adder is a chain 4096 gates long) cannot overflow the
//! thread's stack.

use cdcl_sat::{Cnf, Lit, Solver, Var};

use crate::aig::{Aig, Edge, Node};

/// Something clauses can be written into: a solver or a CNF formula.
pub trait ClauseSink {
    /// A fresh variable.
    fn fresh_var(&mut self) -> Var;
    /// Add a clause.
    fn add(&mut self, clause: &[Lit]);
}

impl ClauseSink for Solver {
    fn fresh_var(&mut self) -> Var {
        self.new_var()
    }
    fn add(&mut self, clause: &[Lit]) {
        self.add_clause(clause);
    }
}

impl ClauseSink for Cnf {
    fn fresh_var(&mut self) -> Var {
        self.num_vars += 1;
        Var::from_index(self.num_vars - 1)
    }
    fn add(&mut self, clause: &[Lit]) {
        self.add_clause(clause);
    }
}

/// The incremental Tseitin encoder for one AIG.
#[derive(Debug, Clone, Default)]
pub struct Tseitin {
    /// The SAT variable of each AIG node, once encoded.
    var_of: Vec<Option<Var>>,
}

impl Tseitin {
    /// An encoder that has encoded nothing yet.
    pub fn new() -> Tseitin {
        Tseitin::default()
    }

    /// The SAT variable of AIG node `node`, if it has been encoded.
    pub fn node_var(&self, node: usize) -> Option<Var> {
        self.var_of.get(node).copied().flatten()
    }

    /// The SAT literal of `edge`, encoding its cone into `sink` first.
    pub fn lit(&mut self, aig: &Aig, edge: Edge, sink: &mut impl ClauseSink) -> Lit {
        if self.var_of.len() < aig.num_nodes() {
            self.var_of.resize(aig.num_nodes(), None);
        }
        // Post-order walk: a node is encoded once both operands are.
        let mut stack = vec![edge.node()];
        while let Some(&n) = stack.last() {
            if self.var_of[n].is_some() {
                stack.pop();
                continue;
            }
            match aig.node(n) {
                Node::False => {
                    let v = sink.fresh_var();
                    sink.add(&[v.negative()]); // the constant node is false
                    self.var_of[n] = Some(v);
                    stack.pop();
                }
                Node::Input(_) => {
                    self.var_of[n] = Some(sink.fresh_var());
                    stack.pop();
                }
                Node::And(a, b) => {
                    let pending: Vec<usize> =
                        [a.node(), b.node()].into_iter().filter(|&m| self.var_of[m].is_none()).collect();
                    if !pending.is_empty() {
                        stack.extend(pending);
                        continue;
                    }
                    let out = sink.fresh_var();
                    let la = self.edge_lit(a);
                    let lb = self.edge_lit(b);
                    sink.add(&[out.negative(), la]);
                    sink.add(&[out.negative(), lb]);
                    sink.add(&[out.positive(), !la, !lb]);
                    self.var_of[n] = Some(out);
                    stack.pop();
                }
            }
        }
        self.edge_lit(edge)
    }

    /// Assert that `edge` is true: encode its cone and add it as a unit
    /// clause.
    pub fn assert_true(&mut self, aig: &Aig, edge: Edge, sink: &mut impl ClauseSink) {
        let l = self.lit(aig, edge, sink);
        sink.add(&[l]);
    }

    /// The literal of an edge whose node is already encoded.
    fn edge_lit(&self, edge: Edge) -> Lit {
        let v = self.var_of[edge.node()].expect("operand encoded before its gate");
        v.lit(edge.is_complemented())
    }

    /// Read the value of AIG input `input` out of a solver model, if that
    /// input was encoded. (An input outside every asserted cone is
    /// unconstrained; callers usually treat it as `false`.)
    pub fn input_value(&self, aig: &Aig, input: usize, model: &[bool]) -> Option<bool> {
        let v = self.node_var(aig.input_edge(input)?.node())?;
        model.get(v.index()).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cdcl_sat::SolveResult;

    #[test]
    fn encodes_an_and_gate_exactly() {
        let mut g = Aig::new();
        let a = g.input();
        let b = g.input();
        let ab = g.and(a, b);
        let mut cnf = Cnf::new(0);
        let mut t = Tseitin::new();
        let l = t.lit(&g, ab, &mut cnf);
        assert_eq!(cnf.num_vars, 3);
        assert_eq!(cnf.clauses.len(), 3);
        // In every model of the clauses, the gate variable is a ∧ b.
        for bits in 0..8u32 {
            let m: Vec<bool> = (0..3).map(|i| bits >> i & 1 == 1).collect();
            if cnf.is_satisfied_by(&m) {
                let va = m[t.node_var(a.node()).unwrap().index()];
                let vb = m[t.node_var(b.node()).unwrap().index()];
                assert_eq!(m[l.var().index()], va && vb);
            }
        }
        // Asking again adds nothing.
        let again = t.lit(&g, !ab, &mut cnf);
        assert_eq!(again, !l);
        assert_eq!(cnf.clauses.len(), 3);
    }

    #[test]
    fn constants_are_pinned() {
        let g = Aig::new();
        let mut s = Solver::new();
        let mut t = Tseitin::new();
        t.assert_true(&g, Edge::TRUE, &mut s);
        assert_eq!(s.solve(), SolveResult::Sat);
        t.assert_true(&g, Edge::FALSE, &mut s);
        assert_eq!(s.solve(), SolveResult::Unsat);
    }

    #[test]
    fn input_values_come_back_from_the_model() {
        let mut g = Aig::new();
        let a = g.input();
        let b = g.input();
        let unused = g.input();
        let f = g.and(a, !b);
        let mut s = Solver::new();
        let mut t = Tseitin::new();
        t.assert_true(&g, f, &mut s);
        assert_eq!(s.solve(), SolveResult::Sat);
        assert_eq!(t.input_value(&g, 0, s.model()), Some(true));
        assert_eq!(t.input_value(&g, 1, s.model()), Some(false));
        assert_eq!(t.input_value(&g, 2, s.model()), None, "never encoded");
        assert_eq!(t.input_value(&g, 7, s.model()), None, "no such input");
        assert!(unused.node() > 0);
    }

    #[test]
    fn deep_chains_do_not_recurse() {
        // A 200 000-gate chain would overflow a recursive encoder's stack.
        let mut g = Aig::new();
        let mut acc = g.input();
        for _ in 0..200_000 {
            let x = g.input();
            acc = g.and(acc, x);
        }
        let mut cnf = Cnf::new(0);
        let mut t = Tseitin::new();
        t.lit(&g, acc, &mut cnf);
        assert_eq!(cnf.clauses.len(), 3 * 200_000);
    }
}
