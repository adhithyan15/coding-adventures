//! # `vos-manifest`
//!
//! The **pure** half of the verified-OS document tooling (backlog VOS-033, spec
//! `code/specs/VOS02-interface-contracts-and-profile-linking.md` sections 2, 6.8, 10
//! and 10.1).  Crate class P (parser, VOS01 section 1.6): bytes in, values out, no
//! filesystem, no clock, no environment.  The program `vos-link` (class H) owns every
//! file read and write and calls this library.
//!
//! ## What is in here
//!
//! ```text
//!   contract.vos bytes ──► contract::parse_contract ──► Contract ──► emit ──► canonical text, digest
//!   *.part bytes       ──► part::admit_part         ──► Part       (no marker, no control byte, caps)
//!   Part blocks        ──► document::assemble       ──► the generated document text
//!   document bytes     ──► document::parse_layout   ──► Layout     (markers, blocks, tiers, trailer)
//!   document text      ──► warrant::analyze_document──► Analysis   (claims + claim_sha256, trailer, build cone)
//! ```
//!
//! ## No hand-written lexers or parsers
//!
//! This repository forbids them.  Both text formats here are described by a token
//! grammar and a parser grammar in `code/grammars/vos_contract` and
//! `code/grammars/vos_document`, and read by the generic `GrammarLexer` and
//! `GrammarParser`.  Warrant text is read only by `warrant-grammar`.  What a
//! context-free grammar cannot say (the class of a name depends on its row, a number
//! must fit `u64`, tiers must be sorted) is a small **post-parse check** on already
//! tokenised values, named in the error it raises.
//!
//! ## Fail closed
//!
//! Every function is total: no panic, no unbounded allocation.  Byte caps (VOS02
//! section 6.8) are checked before a byte is lexed.  Anything outside the grammar,
//! or any file that is not already in canonical form, is an error and never a
//! repair.

pub mod caps;
pub mod contract;
pub mod digest;
pub mod document;
pub mod grammar;
pub mod part;
pub mod warrant;

pub use contract::{Contract, ContractError, Tier};
pub use document::{DocError, Layout};
pub use part::{Part, PartError};
