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
//! - **[`transducer`]** -- ordered effectful state machines for tokenizers
//!
//! ## The Chomsky Hierarchy
//!
//! ```text
//!     Regular languages    <  Context-free languages  <  Context-sensitive  <  RE
//!     (DFA/NFA)               (PDA)                      (LBA)               (TM)
//! ```
//!
//! This crate covers the first two levels: DFA/NFA for regular languages,
//! and PDA for context-free languages.  It also exposes wider primitives such
//! as modal machines and effectful transducers so tokenizer-style runtimes can
//! share the same state-machine foundation instead of forcing every problem
//! into a recognition-only automaton.
//!
//! ## Connection to the coding-adventures stack
//!
//! The 2-bit branch predictor (D02) is a DFA. The CPU pipeline (D04) is a
//! linear DFA. Regex engines convert patterns to NFAs, then to DFAs via subset
//! construction. Parsers use PDAs. HTML tokenizers use modal state machines.

pub mod definitions;
pub mod dfa;
pub mod minimize;
pub mod modal;
pub mod nfa;
pub mod pda;
pub mod transducer;
pub mod types;

pub use definitions::{
    FixtureDefinition, GuardDefinition, InputDefinition, MachineKind, MatcherDefinition,
    RegisterDefinition, StateDefinition, StateMachineDefinition, TokenDefinition,
    TransitionDefinition,
};
pub use dfa::DFA;
pub use minimize::minimize;
pub use modal::{ModalStateMachine, ModeTransitionRecord};
pub use nfa::{EPSILON, NFA};
pub use pda::{PDATraceEntry, PDATransition, PushdownAutomaton};
pub use transducer::{
    EffectfulInput, EffectfulMatcher, EffectfulStateMachine, EffectfulStep, EffectfulTransition,
    ANY_INPUT, END_INPUT,
};
pub use types::*;
