//! AVM2 ABC Constant Pool structures.

#![deny(unsafe_code)]

/// AVM2 namespace descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbcNamespace {
    pub kind: u8,
    pub name_index: u32,
}

/// AVM2 multiname descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbcMultiname {
    QName {
        namespace_index: u32,
        name_index: u32,
    },
    RTQName {
        name_index: u32,
    },
    RTQNameL,
    Multiname {
        name_index: u32,
        ns_set_index: u32,
    },
    MultinameL {
        ns_set_index: u32,
    },
    Other(u8),
}

/// AVM2 constant pool table.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AbcConstantPool {
    pub integers: Vec<i32>,
    pub uintegers: Vec<u32>,
    pub doubles: Vec<f64>,
    pub strings: Vec<String>,
    pub namespaces: Vec<AbcNamespace>,
    pub ns_sets: Vec<Vec<u32>>,
    pub multinames: Vec<AbcMultiname>,
}

impl AbcConstantPool {
    /// Retrieve string at index, returning empty string if index is 0 or out of bounds.
    pub fn cpool_get_string(&self, idx: usize) -> &str {
        if idx == 0 || idx >= self.strings.len() {
            ""
        } else {
            &self.strings[idx]
        }
    }

    /// Retrieve integer at index.
    pub fn cpool_get_int(&self, idx: usize) -> i32 {
        if idx == 0 || idx >= self.integers.len() {
            0
        } else {
            self.integers[idx]
        }
    }

    /// Retrieve uinteger at index.
    pub fn cpool_get_uint(&self, idx: usize) -> u32 {
        if idx == 0 || idx >= self.uintegers.len() {
            0
        } else {
            self.uintegers[idx]
        }
    }

    /// Retrieve double at index.
    pub fn cpool_get_double(&self, idx: usize) -> f64 {
        if idx == 0 || idx >= self.doubles.len() {
            0.0
        } else {
            self.doubles[idx]
        }
    }

    /// Retrieve property name string from multiname index.
    pub fn cpool_get_multiname_name(&self, idx: usize) -> &str {
        if idx == 0 || idx >= self.multinames.len() {
            return "";
        }
        match &self.multinames[idx] {
            AbcMultiname::QName { name_index, .. }
            | AbcMultiname::RTQName { name_index }
            | AbcMultiname::Multiname { name_index, .. } => {
                self.cpool_get_string(*name_index as usize)
            }
            _ => "",
        }
    }
}
