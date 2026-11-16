//! AVM2 ABC bytecode processing module.

#![deny(unsafe_code)]

pub mod abc_file;
pub mod cpool;
pub mod lifter;
pub mod opcode;
pub mod parser;
pub mod verifier;

pub use abc_file::{AbcFile, AbcMethod, AbcMethodBody, AbcScript};
pub use cpool::{AbcConstantPool, AbcMultiname, AbcNamespace};
pub use lifter::Avm2Lifter;
pub use parser::AbcParser;
pub use verifier::{AbcDecodedInstr, Avm2Verifier};
