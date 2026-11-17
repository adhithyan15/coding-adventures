//! # `vos-sched-contract-check`
//!
//! A test-only companion of `vos-sched-contract` (VOS-010). This library is
//! intentionally empty: the work is in `tests/warrant_contract.rs`, which parses,
//! type-checks and evaluates the contract's Warrant text with the repository's
//! own Warrant tooling.
//!
//! ## Why a separate crate
//!
//! The contract crate is a class K component crate, and the shape check
//! (VOS03, rule SH12) allows such a crate no dependency but `core`, contract
//! crates and the two dev-dependencies of the generated evidence harness. The
//! Warrant parser and evaluator are none of those, so the checks that need them
//! live here, in a crate with the host class (`vos.toml`, class H).
//!
//! ## When it goes away
//!
//! `vos-link gen-doc` and `check-doc` (VOS-033) assemble the real
//! per-implementation documents from the same `.part` files, and the evidence
//! generator (VOS-011, VOS-012) runs their claims against compiled Rust. The
//! model-level lemmas and the obligation-text checks here are then folded into
//! that tool's own tests (backlog VOS-D049).
