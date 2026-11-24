//! # vos-capacity-probe
//!
//! The solver and enumeration capacity probe of the verified-OS program (VOS-028b, VOS30
//! section 9).  See the README and the spec for the design; each module opens with its own
//! part of the story.

// The probe's own code has no `unsafe`.  The one exception is a generated file it includes
// (enumx): the real scheduler's evidence harness installs its allocation observer as the global
// allocator, which is an `unsafe impl`.
#![deny(unsafe_code)]

pub mod cli;
pub mod cost;
pub mod cpu;
pub mod enumx;
pub mod exp;
pub mod inst;
pub mod join;
pub mod meta;
pub mod prog;
pub mod results;
pub mod row;
pub mod runner;
pub mod sem;
pub mod shapes;
pub mod solve;
pub mod spot;
pub mod sweep;
pub mod table;
