//! # `bit-blaster` — from machine integers to certified SAT proofs
//!
//! **VER10, part 1.** The verified shipping program (VER00 §7.6) proves
//! claims like "for every 32-bit `a` and `b`, the implementation and the
//! specification agree" by asking a SAT solver for a counterexample. Before
//! the solver can be asked, the integers have to become Boolean formulas,
//! exactly. This crate holds the circuits; `constraint-engine`'s BV tactic
//! lowers SMT-LIB `QF_BV` predicates onto them, and `constraint-vm` runs
//! `.smt2` programs through that tactic. VER18 (hardware) and VER19 (WIR)
//! build circuits here directly, without predicates.
//!
//! ```text
//!   QF_BV predicate (constraint-engine::bv)  /  netlist  /  WIR
//!         │  every w-bit value → w signals
//!         ▼
//!   bit-vector circuits: adders, shifters, comparators   bv.rs
//!         │  built from AND gates and inverters only
//!         ▼
//!   and-inverter graph (structurally hashed)            aig.rs
//!         │  one variable and three clauses per AND gate
//!         ▼
//!   CNF ──► cdcl-sat ──► SAT + model  /  UNSAT + DRAT proof ──► drat-check
//!                                                tseitin.rs
//! ```
//!
//! ## A proof in five lines
//!
//! "Subtraction is addition of the two's complement", for all pairs of
//! 16-bit integers (2³² of them), proved and certified:
//!
//! ```
//! use bit_blaster::{aig::Aig, bv::{self, BitVec}, tseitin::Tseitin};
//! use cdcl_sat::{Cnf, Solver, SolveResult, write_dimacs};
//!
//! let mut g = Aig::new();
//! let (a, b) = (BitVec::input(&mut g, 16), BitVec::input(&mut g, 16));
//! let lhs = bv::sub(&mut g, &a, &b);
//! let neg_b = bv::neg(&mut g, &b);
//! let rhs = bv::add(&mut g, &a, &neg_b);
//! let same = bv::eq(&mut g, &lhs, &rhs);
//!
//! // The miter: is there an input where they differ?
//! let mut cnf = Cnf::new(0);
//! Tseitin::new().assert_true(&g, !same, &mut cnf);
//! let mut s = Solver::from_cnf_with_proof(&cnf);
//! assert_eq!(s.solve(), SolveResult::Unsat);              // no counterexample
//! let proof = s.proof().unwrap().to_drat_text();
//! assert!(drat_check::check(&write_dimacs(&cnf), &proof).is_ok()); // certified
//! ```
//!
//! ## Modules
//!
//! | Module | Contents |
//! |---|---|
//! | [`aig`] | `Aig`, `Edge`: AND gates, inverters, structural hashing, 64-way simulation |
//! | [`tseitin`] | incremental Tseitin encoding of an AIG cone into a solver or CNF |
//! | [`bv`] | `BitVec` and the QF_BV operations as circuits |

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]
#![forbid(unsafe_code)]

pub mod aig;
pub mod bv;
pub mod tseitin;
