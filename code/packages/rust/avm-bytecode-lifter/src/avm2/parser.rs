//! Safe, bounded parser for Flash AVM2 ActionScript Bytecode (`DoABC`).

#![deny(unsafe_code)]

use super::abc_file::{AbcFile, AbcMethod, AbcMethodBody, AbcScript};
use super::cpool::{AbcConstantPool, AbcMultiname, AbcNamespace};
use crate::error::{LifterError, MAX_BYTECODE_SIZE, MAX_CONSTANTS};

/// Bounded parser for ABC binary bytecode files.
pub struct AbcParser<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> AbcParser<'a> {
    /// Create a new ABC parser over the given slice.
    pub fn a2p_new(data: &'a [u8]) -> Result<Self, LifterError> {
        if data.len() > MAX_BYTECODE_SIZE {
            return Err(LifterError::InvalidBytecode(
                "ABC bytecode exceeds maximum size".to_string(),
            ));
        }
        Ok(Self { data, pos: 0 })
    }

    fn a2p_remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    fn a2p_read_u8(&mut self) -> Result<u8, LifterError> {
        if self.pos >= self.data.len() {
            return Err(LifterError::UnexpectedEof);
        }
        let b = self.data[self.pos];
        self.pos += 1;
        Ok(b)
    }

    fn a2p_read_u16_le(&mut self) -> Result<u16, LifterError> {
        if self.a2p_remaining() < 2 {
            return Err(LifterError::UnexpectedEof);
        }
        let b0 = self.data[self.pos] as u16;
        let b1 = self.data[self.pos + 1] as u16;
        self.pos += 2;
        Ok(b0 | (b1 << 8))
    }

    /// Read an unsigned variable-length 30-bit integer (u30 / LEB128).
    pub fn a2p_read_u30(&mut self) -> Result<u32, LifterError> {
        let mut result: u32 = 0;
        let mut shift = 0;
        for _ in 0..5 {
            let b = self.a2p_read_u8()?;
            result |= ((b & 0x7F) as u32) << shift;
            if (b & 0x80) == 0 {
                return Ok(result);
            }
            shift += 7;
        }
        Err(LifterError::InvalidBytecode(
            "u30 LEB128 exceeded 5 bytes".to_string(),
        ))
    }

    /// Read a signed variable-length 32-bit integer (s32).
    pub fn a2p_read_s32(&mut self) -> Result<i32, LifterError> {
        let mut result: u32 = 0;
        let mut shift = 0;
        for i in 0..5 {
            let b = self.a2p_read_u8()?;
            result |= ((b & 0x7F) as u32) << shift;
            shift += 7;
            if (b & 0x80) == 0 {
                if shift < 32 && (b & 0x40) != 0 {
                    result |= !0 << shift;
                }
                return Ok(result as i32);
            }
            if i == 4 {
                return Ok(result as i32);
            }
        }
        Err(LifterError::InvalidBytecode(
            "s32 LEB128 exceeded 5 bytes".to_string(),
        ))
    }

    fn a2p_read_f64_le(&mut self) -> Result<f64, LifterError> {
        if self.a2p_remaining() < 8 {
            return Err(LifterError::UnexpectedEof);
        }
        let bytes: [u8; 8] = self.data[self.pos..self.pos + 8]
            .try_into()
            .map_err(|_| LifterError::UnexpectedEof)?;
        self.pos += 8;
        Ok(f64::from_le_bytes(bytes))
    }

    /// Parse complete ABC file.
    pub fn a2p_parse(&mut self) -> Result<AbcFile, LifterError> {
        let minor_version = self.a2p_read_u16_le()?;
        let major_version = self.a2p_read_u16_le()?;

        let constant_pool = self.a2p_parse_cpool()?;

        // Methods
        let method_count = self.a2p_read_u30()? as usize;
        let mut methods = Vec::with_capacity(method_count.min(1024));
        for _ in 0..method_count {
            let param_count = self.a2p_read_u30()? as usize;
            let return_type = self.a2p_read_u30()?;
            let mut param_types = Vec::with_capacity(param_count.min(64));
            for _ in 0..param_count {
                param_types.push(self.a2p_read_u30()?);
            }
            let name_index = self.a2p_read_u30()?;
            let flags = self.a2p_read_u8()?;
            // Check optional parameters
            if (flags & 0x08) != 0 {
                let opt_count = self.a2p_read_u30()? as usize;
                for _ in 0..opt_count {
                    let _val = self.a2p_read_u30()?;
                    let _kind = self.a2p_read_u8()?;
                }
            }
            // Check param names
            if (flags & 0x80) != 0 {
                for _ in 0..param_count {
                    let _pname = self.a2p_read_u30()?;
                }
            }
            methods.push(AbcMethod {
                param_types,
                return_type,
                name_index,
                flags,
            });
        }

        // Metadata
        let metadata_count = self.a2p_read_u30()? as usize;
        for _ in 0..metadata_count {
            let _name = self.a2p_read_u30()?;
            let item_count = self.a2p_read_u30()? as usize;
            for _ in 0..item_count {
                let _key = self.a2p_read_u30()?;
                let _val = self.a2p_read_u30()?;
            }
        }

        // Instances & Classes
        let class_count = self.a2p_read_u30()? as usize;
        for _ in 0..class_count {
            let _name = self.a2p_read_u30()?;
            let _super_name = self.a2p_read_u30()?;
            let flags = self.a2p_read_u8()?;
            if (flags & 0x08) != 0 {
                let _protected_ns = self.a2p_read_u30()?;
            }
            let interface_count = self.a2p_read_u30()? as usize;
            for _ in 0..interface_count {
                let _iface = self.a2p_read_u30()?;
            }
            let _iinit = self.a2p_read_u30()?;
            self.a2p_skip_traits()?;
        }
        for _ in 0..class_count {
            let _cinit = self.a2p_read_u30()?;
            self.a2p_skip_traits()?;
        }

        // Scripts
        let script_count = self.a2p_read_u30()? as usize;
        let mut scripts = Vec::with_capacity(script_count.min(16));
        for _ in 0..script_count {
            let init_method = self.a2p_read_u30()?;
            self.a2p_skip_traits()?;
            scripts.push(AbcScript { init_method });
        }

        // Method bodies
        let body_count = self.a2p_read_u30()? as usize;
        let mut method_bodies = Vec::with_capacity(body_count.min(1024));
        for _ in 0..body_count {
            let method_index = self.a2p_read_u30()?;
            let max_stack = self.a2p_read_u30()?;
            let local_count = self.a2p_read_u30()?;
            let init_scope_depth = self.a2p_read_u30()?;
            let max_scope_depth = self.a2p_read_u30()?;
            let code_length = self.a2p_read_u30()? as usize;

            if self.a2p_remaining() < code_length {
                return Err(LifterError::UnexpectedEof);
            }
            let code = self.data[self.pos..self.pos + code_length].to_vec();
            self.pos += code_length;

            let exception_count = self.a2p_read_u30()? as usize;
            for _ in 0..exception_count {
                let _from = self.a2p_read_u30()?;
                let _to = self.a2p_read_u30()?;
                let _target = self.a2p_read_u30()?;
                let _exc_type = self.a2p_read_u30()?;
                let _var_name = self.a2p_read_u30()?;
            }
            self.a2p_skip_traits()?;

            method_bodies.push(AbcMethodBody {
                method_index,
                max_stack,
                local_count,
                init_scope_depth,
                max_scope_depth,
                code,
            });
        }

        Ok(AbcFile {
            minor_version,
            major_version,
            constant_pool,
            methods,
            method_bodies,
            scripts,
        })
    }

    fn a2p_skip_traits(&mut self) -> Result<(), LifterError> {
        let trait_count = self.a2p_read_u30()? as usize;
        for _ in 0..trait_count {
            let _name = self.a2p_read_u30()?;
            let kind_byte = self.a2p_read_u8()?;
            let kind = kind_byte & 0x0F;
            match kind {
                0 | 6 => {
                    // Slot or Const
                    let _slot_id = self.a2p_read_u30()?;
                    let _type_name = self.a2p_read_u30()?;
                    let vindex = self.a2p_read_u30()?;
                    if vindex != 0 {
                        let _vkind = self.a2p_read_u8()?;
                    }
                }
                1..=3 => {
                    // Method, Getter, Setter
                    let _disp_id = self.a2p_read_u30()?;
                    let _method = self.a2p_read_u30()?;
                }
                4 => {
                    // Class
                    let _slot_id = self.a2p_read_u30()?;
                    let _class = self.a2p_read_u30()?;
                }
                5 => {
                    // Function
                    let _slot_id = self.a2p_read_u30()?;
                    let _function = self.a2p_read_u30()?;
                }
                _ => {}
            }
            if ((kind_byte >> 4) & 0x04) != 0 {
                // ATTR_Metadata
                let meta_count = self.a2p_read_u30()? as usize;
                for _ in 0..meta_count {
                    let _m = self.a2p_read_u30()?;
                }
            }
        }
        Ok(())
    }

    fn a2p_parse_cpool(&mut self) -> Result<AbcConstantPool, LifterError> {
        let mut cpool = AbcConstantPool::default();

        // Integers
        let int_count = self.a2p_read_u30()? as usize;
        if int_count > 0 {
            cpool.integers.push(0); // index 0 dummy
            for _ in 1..int_count {
                cpool.integers.push(self.a2p_read_s32()?);
            }
        }

        // Uintegers
        let uint_count = self.a2p_read_u30()? as usize;
        if uint_count > 0 {
            cpool.uintegers.push(0);
            for _ in 1..uint_count {
                cpool.uintegers.push(self.a2p_read_u30()?);
            }
        }

        // Doubles
        let double_count = self.a2p_read_u30()? as usize;
        if double_count > 0 {
            cpool.doubles.push(0.0);
            for _ in 1..double_count {
                cpool.doubles.push(self.a2p_read_f64_le()?);
            }
        }

        // Strings
        let string_count = self.a2p_read_u30()? as usize;
        if string_count > MAX_CONSTANTS {
            return Err(LifterError::ConstantPoolLimitExceeded);
        }
        if string_count > 0 {
            cpool.strings.push(String::new());
            for _ in 1..string_count {
                let len = self.a2p_read_u30()? as usize;
                if self.a2p_remaining() < len {
                    return Err(LifterError::UnexpectedEof);
                }
                let s_bytes = &self.data[self.pos..self.pos + len];
                self.pos += len;
                cpool
                    .strings
                    .push(String::from_utf8_lossy(s_bytes).to_string());
            }
        }

        // Namespaces
        let ns_count = self.a2p_read_u30()? as usize;
        if ns_count > 0 {
            cpool.namespaces.push(AbcNamespace {
                kind: 0,
                name_index: 0,
            });
            for _ in 1..ns_count {
                let kind = self.a2p_read_u8()?;
                let name_index = self.a2p_read_u30()?;
                cpool.namespaces.push(AbcNamespace { kind, name_index });
            }
        }

        // NsSets
        let ns_set_count = self.a2p_read_u30()? as usize;
        if ns_set_count > 0 {
            cpool.ns_sets.push(Vec::new());
            for _ in 1..ns_set_count {
                let count = self.a2p_read_u30()? as usize;
                let mut set = Vec::with_capacity(count.min(64));
                for _ in 0..count {
                    set.push(self.a2p_read_u30()?);
                }
                cpool.ns_sets.push(set);
            }
        }

        // Multinames
        let mn_count = self.a2p_read_u30()? as usize;
        if mn_count > 0 {
            cpool.multinames.push(AbcMultiname::Other(0));
            for _ in 1..mn_count {
                let kind = self.a2p_read_u8()?;
                let mn = match kind {
                    0x07 | 0x0D => {
                        let namespace_index = self.a2p_read_u30()?;
                        let name_index = self.a2p_read_u30()?;
                        AbcMultiname::QName {
                            namespace_index,
                            name_index,
                        }
                    }
                    0x0F | 0x10 => {
                        let name_index = self.a2p_read_u30()?;
                        AbcMultiname::RTQName { name_index }
                    }
                    0x11 | 0x12 => AbcMultiname::RTQNameL,
                    0x09 | 0x0E => {
                        let name_index = self.a2p_read_u30()?;
                        let ns_set_index = self.a2p_read_u30()?;
                        AbcMultiname::Multiname {
                            name_index,
                            ns_set_index,
                        }
                    }
                    0x1B | 0x1C => {
                        let ns_set_index = self.a2p_read_u30()?;
                        AbcMultiname::MultinameL { ns_set_index }
                    }
                    other => AbcMultiname::Other(other),
                };
                cpool.multinames.push(mn);
            }
        }

        Ok(cpool)
    }
}
