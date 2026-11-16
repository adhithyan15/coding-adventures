//! AVM2 ABC bytecode file structures.

#![deny(unsafe_code)]

use super::cpool::AbcConstantPool;

/// A parsed method signature in an ABC file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbcMethod {
    pub param_types: Vec<u32>,
    pub return_type: u32,
    pub name_index: u32,
    pub flags: u8,
}

/// A parsed method body in an ABC file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbcMethodBody {
    pub method_index: u32,
    pub max_stack: u32,
    pub local_count: u32,
    pub init_scope_depth: u32,
    pub max_scope_depth: u32,
    pub code: Vec<u8>,
}

/// An entry script in an ABC file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbcScript {
    pub init_method: u32,
}

/// A fully parsed ABC bytecode file.
#[derive(Debug, Clone, PartialEq)]
pub struct AbcFile {
    pub minor_version: u16,
    pub major_version: u16,
    pub constant_pool: AbcConstantPool,
    pub methods: Vec<AbcMethod>,
    pub method_bodies: Vec<AbcMethodBody>,
    pub scripts: Vec<AbcScript>,
}
