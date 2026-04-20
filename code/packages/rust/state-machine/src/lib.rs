//! # State Machine -- formal automata from DFA to PDA.
//!
//! This crate implements the fundamental computational models from automata
//! theory, the branch of computer science that studies abstract machines:
//!
//! - **[`types`]** -- core types shared by all automata (`TransitionRecord`)
//! - **[`dfa`]** -- Deterministic Finite Automaton (the workhorse)
//! - **[`nfa`]** -- Non-deterministic Finite Automaton with epsilon transitions
//! - **[`minimize`]** -- Hopcroft's DFA minimization algorithm
//! - **[`pda`]** -- Pushdown Automaton (finite automaton + stack)
//! - **[`modal`]** -- Modal State Machine (multiple DFA sub-machines with mode switching)
//!
//! ## The Chomsky Hierarchy
//!
//! ```text
//!     Regular languages    <  Context-free languages  <  Context-sensitive  <  RE
//!     (DFA/NFA)               (PDA)                      (LBA)               (TM)
//! ```
//!
//! This crate covers the first two levels: DFA/NFA for regular languages,
//! and PDA for context-free languages. The modal state machine sits between
//! the two, providing context-sensitive tokenization (like HTML mode switching)
//! without the full power of a pushdown automaton.
//!
//! ## Connection to the coding-adventures stack
//!
//! The 2-bit branch predictor (D02) is a DFA. The CPU pipeline (D04) is a
//! linear DFA. Regex engines convert patterns to NFAs, then to DFAs via subset
//! construction. Parsers use PDAs. HTML tokenizers use modal state machines.

pub mod types;
pub mod dfa;
pub mod document;
pub mod nfa;
pub mod minimize;
pub mod pda;
pub mod modal;

pub use types::*;
pub use dfa::DFA;
pub use document::{
    MachineKind, StateDefinition, StateMachineDocument, TransitionDefinition, STATE_MACHINE_FORMAT,
};
pub use nfa::{EPSILON, NFA};
pub use minimize::minimize;
pub use pda::{PDATraceEntry, PDATransition, PushdownAutomaton};
pub use modal::{ModalStateMachine, ModeTransitionRecord};
