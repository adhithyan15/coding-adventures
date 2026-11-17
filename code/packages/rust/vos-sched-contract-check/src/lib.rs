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
//! ## What moved to `vos-link` (VOS-033, backlog VOS-D049)
//!
//! `vos-link gen-doc` and `check-doc` assemble and verify the real per-implementation
//! documents from the same `.part` files, and `vos-manifest` owns the part admission rules,
//! so the string-joining stand-in and the marker-discipline stand-in are gone from here.
//! What stays is the **semantic** suite: the ground evaluation of the lemmas and obligations
//! over Warrant models and the mutation tables (about a minute), which is neither a document
//! check nor something the tool's own fast tests should carry.  It retires when the
//! evidence generator (VOS-011, VOS-012) runs the same claims against compiled Rust.
