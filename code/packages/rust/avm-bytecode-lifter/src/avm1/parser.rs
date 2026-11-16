//! Safe, bounded parser for Flash AVM1 bytecode streams.

#![deny(unsafe_code)]

use super::action::{Avm1Action, Avm1ActionKind};
use super::opcode::*;
use super::value::Avm1Value;
use crate::error::{
    LifterError, MAX_ACTIONS, MAX_BYTECODE_SIZE, MAX_CONSTANTS, MAX_FUNCTION_DEPTH,
};

/// Bounded reader for reading within a single action's payload sub-slice.
struct Avm1PayloadReader<'p> {
    slice: &'p [u8],
    pos: usize,
}

impl<'p> Avm1PayloadReader<'p> {
    fn a1pr_new(slice: &'p [u8]) -> Self {
        Self { slice, pos: 0 }
    }

    fn a1pr_remaining(&self) -> usize {
        self.slice.len().saturating_sub(self.pos)
    }

    fn a1pr_read_u8(&mut self) -> Result<u8, LifterError> {
        let b = *self.slice.get(self.pos).ok_or(LifterError::UnexpectedEof)?;
        self.pos = self
            .pos
            .checked_add(1)
            .ok_or_else(|| LifterError::InvalidBytecode("offset overflow".to_string()))?;
        Ok(b)
    }

    fn a1pr_read_slice(&mut self, len: usize) -> Result<&'p [u8], LifterError> {
        let s = self
            .slice
            .get(self.pos..)
            .and_then(|tail| tail.get(..len))
            .ok_or(LifterError::UnexpectedEof)?;
        self.pos = self
            .pos
            .checked_add(len)
            .ok_or_else(|| LifterError::InvalidBytecode("offset overflow".to_string()))?;
        Ok(s)
    }

    fn a1pr_read_u16_le(&mut self) -> Result<u16, LifterError> {
        let s = self.a1pr_read_slice(2)?;
        Ok((s[0] as u16) | ((s[1] as u16) << 8))
    }

    fn a1pr_read_i16_le(&mut self) -> Result<i16, LifterError> {
        self.a1pr_read_u16_le().map(|u| u as i16)
    }

    fn a1pr_read_u32_le(&mut self) -> Result<u32, LifterError> {
        let s = self.a1pr_read_slice(4)?;
        Ok((s[0] as u32) | ((s[1] as u32) << 8) | ((s[2] as u32) << 16) | ((s[3] as u32) << 24))
    }

    fn a1pr_read_f32_le(&mut self) -> Result<f32, LifterError> {
        self.a1pr_read_u32_le().map(f32::from_bits)
    }

    fn a1pr_read_f64_le(&mut self) -> Result<f64, LifterError> {
        let s = self.a1pr_read_slice(8)?;
        let bytes: [u8; 8] = s.try_into().map_err(|_| LifterError::UnexpectedEof)?;
        let w0 = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let w1 = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let u = (w0 as u64) | ((w1 as u64) << 32);
        Ok(f64::from_bits(u))
    }

    fn a1pr_read_string_null_terminated(&mut self, max_len: usize) -> Result<String, LifterError> {
        let mut bytes = Vec::new();
        while self.pos < self.slice.len() {
            let b = self.a1pr_read_u8()?;
            if b == 0 {
                return Ok(String::from_utf8_lossy(&bytes).to_string());
            }
            bytes.push(b);
            if bytes.len() > max_len {
                return Err(LifterError::InvalidBytecode(
                    "null-terminated string exceeded maximum length".to_string(),
                ));
            }
        }
        Ok(String::from_utf8_lossy(&bytes).to_string())
    }
}

/// Bounded parser for AVM1 byte streams.
pub struct Avm1Parser<'a> {
    data: &'a [u8],
    pos: usize,
    instruction_count: usize,
    depth: usize,
}

impl<'a> Avm1Parser<'a> {
    /// Create a new AVM1 parser for the given bytecode slice.
    pub fn a1p_new(data: &'a [u8]) -> Result<Self, LifterError> {
        Self::a1p_new_with_depth(data, 0, 0)
    }

    /// Create a new AVM1 parser with specified function nesting depth and instruction budget.
    pub fn a1p_new_with_depth(
        data: &'a [u8],
        depth: usize,
        instruction_count: usize,
    ) -> Result<Self, LifterError> {
        if depth > MAX_FUNCTION_DEPTH {
            return Err(LifterError::FunctionLimitExceeded);
        }
        if data.len() > MAX_BYTECODE_SIZE {
            return Err(LifterError::InvalidBytecode(
                "bytecode stream exceeds maximum allowed size".to_string(),
            ));
        }
        Ok(Self {
            data,
            pos: 0,
            instruction_count,
            depth,
        })
    }

    /// Parse all AVM1 actions in the bytecode buffer.
    pub fn a1p_parse_all(&mut self) -> Result<Vec<Avm1Action>, LifterError> {
        let mut actions = Vec::new();
        while self.pos < self.data.len() {
            let start_offset = self.pos;
            let action = self.a1p_parse_action()?;
            actions.push(action);
            if self.pos <= start_offset {
                // Prevent infinite loop on zero-byte progression
                return Err(LifterError::InvalidBytecode(
                    "parser made no forward progress".to_string(),
                ));
            }
        }
        Ok(actions)
    }

    fn a1p_read_u8(&mut self) -> Result<u8, LifterError> {
        let b = *self.data.get(self.pos).ok_or(LifterError::UnexpectedEof)?;
        self.pos = self
            .pos
            .checked_add(1)
            .ok_or_else(|| LifterError::InvalidBytecode("offset overflow".to_string()))?;
        Ok(b)
    }

    fn a1p_read_u16_le(&mut self) -> Result<u16, LifterError> {
        let slice = self
            .data
            .get(self.pos..)
            .and_then(|tail| tail.get(..2))
            .ok_or(LifterError::UnexpectedEof)?;
        self.pos = self
            .pos
            .checked_add(2)
            .ok_or_else(|| LifterError::InvalidBytecode("offset overflow".to_string()))?;
        Ok((slice[0] as u16) | ((slice[1] as u16) << 8))
    }

    fn a1p_parse_action(&mut self) -> Result<Avm1Action, LifterError> {
        self.instruction_count = self
            .instruction_count
            .checked_add(1)
            .ok_or(LifterError::InstructionLimitExceeded)?;
        if self.instruction_count > MAX_ACTIONS {
            return Err(LifterError::InstructionLimitExceeded);
        }

        let start_offset = self.pos;
        let op = self.a1p_read_u8()?;

        if op == ACTION_END {
            return Ok(Avm1Action {
                offset: start_offset,
                length: 1,
                kind: Avm1ActionKind::End,
            });
        }

        if op < 0x80 {
            let kind = match op {
                ACTION_NEXT_FRAME => Avm1ActionKind::NextFrame,
                ACTION_PREVIOUS_FRAME => Avm1ActionKind::PreviousFrame,
                ACTION_PLAY => Avm1ActionKind::Play,
                ACTION_STOP => Avm1ActionKind::Stop,
                ACTION_TOGGLE_QUALITY => Avm1ActionKind::ToggleQuality,
                ACTION_STOP_SOUNDS => Avm1ActionKind::StopSounds,
                ACTION_ADD => Avm1ActionKind::Add,
                ACTION_SUBTRACT => Avm1ActionKind::Subtract,
                ACTION_MULTIPLY => Avm1ActionKind::Multiply,
                ACTION_DIVIDE => Avm1ActionKind::Divide,
                ACTION_EQUALS => Avm1ActionKind::Equals,
                ACTION_LESS => Avm1ActionKind::Less,
                ACTION_AND => Avm1ActionKind::And,
                ACTION_OR => Avm1ActionKind::Or,
                ACTION_NOT => Avm1ActionKind::Not,
                ACTION_STRING_EQUALS => Avm1ActionKind::StringEquals,
                ACTION_STRING_LENGTH => Avm1ActionKind::StringLength,
                ACTION_STRING_EXTRACT => Avm1ActionKind::StringExtract,
                ACTION_POP => Avm1ActionKind::Pop,
                ACTION_TO_INTEGER => Avm1ActionKind::ToInteger,
                ACTION_GET_VARIABLE => Avm1ActionKind::GetVariable,
                ACTION_SET_VARIABLE => Avm1ActionKind::SetVariable,
                ACTION_STRING_ADD => Avm1ActionKind::StringAdd,
                ACTION_TRACE => Avm1ActionKind::Trace,
                ACTION_CHAR_TO_ASCII => Avm1ActionKind::CharToAscii,
                ACTION_ASCII_TO_CHAR => Avm1ActionKind::AsciiToChar,
                ACTION_GET_TIME => Avm1ActionKind::GetTime,
                ACTION_CALL_FUNCTION => Avm1ActionKind::CallFunction,
                ACTION_RETURN => Avm1ActionKind::Return,
                ACTION_NEW_OBJECT => Avm1ActionKind::NewObject,
                ACTION_ADD2 => Avm1ActionKind::Add2,
                ACTION_LESS2 => Avm1ActionKind::Less2,
                ACTION_EQUALS2 => Avm1ActionKind::Equals2,
                ACTION_TO_NUMBER => Avm1ActionKind::ToNumber,
                ACTION_TO_STRING => Avm1ActionKind::ToString,
                ACTION_GET_MEMBER => Avm1ActionKind::GetMember,
                ACTION_SET_MEMBER => Avm1ActionKind::SetMember,
                ACTION_CALL_METHOD => Avm1ActionKind::CallMethod,
                _ => return Err(LifterError::UnsupportedOpcode(op)),
            };
            return Ok(Avm1Action {
                offset: start_offset,
                length: 1,
                kind,
            });
        }

        // Action has payload: op >= 0x80
        let payload_len = self.a1p_read_u16_le()? as usize;
        let payload_start = self.pos;
        let payload_end = payload_start
            .checked_add(payload_len)
            .ok_or_else(|| LifterError::InvalidBytecode("offset overflow".to_string()))?;
        if payload_end > self.data.len() {
            return Err(LifterError::UnexpectedEof);
        }
        let payload_slice = &self.data[payload_start..payload_end];
        let mut reader = Avm1PayloadReader::a1pr_new(payload_slice);

        let (kind, total_length) = match op {
            ACTION_GOTO_FRAME => {
                let frame = reader.a1pr_read_u16_le()?;
                self.pos = payload_end;
                let span = 3usize
                    .checked_add(payload_len)
                    .ok_or_else(|| LifterError::InvalidBytecode("overflow".to_string()))?;
                (Avm1ActionKind::GotoFrame(frame), span)
            }
            ACTION_STORE_REGISTER => {
                let reg = reader.a1pr_read_u8()?;
                self.pos = payload_end;
                let span = 3usize
                    .checked_add(payload_len)
                    .ok_or_else(|| LifterError::InvalidBytecode("overflow".to_string()))?;
                (Avm1ActionKind::StoreRegister(reg), span)
            }
            ACTION_CONSTANT_POOL => {
                let count = reader.a1pr_read_u16_le()? as usize;
                if count > MAX_CONSTANTS {
                    return Err(LifterError::ConstantPoolLimitExceeded);
                }
                let mut constants = Vec::with_capacity(count.min(reader.a1pr_remaining() + 1));
                for _ in 0..count {
                    let s = reader.a1pr_read_string_null_terminated(65536)?;
                    constants.push(s);
                }
                self.pos = payload_end;
                let span = 3usize
                    .checked_add(payload_len)
                    .ok_or_else(|| LifterError::InvalidBytecode("overflow".to_string()))?;
                (Avm1ActionKind::ConstantPool(constants), span)
            }
            ACTION_PUSH => {
                let mut values = Vec::new();
                while reader.pos < reader.slice.len() {
                    let ptype = reader.a1pr_read_u8()?;
                    let val = match ptype {
                        PUSH_TYPE_STRING => {
                            let s = reader.a1pr_read_string_null_terminated(65536)?;
                            Avm1Value::String(s)
                        }
                        PUSH_TYPE_FLOAT => {
                            let f = reader.a1pr_read_f32_le()?;
                            Avm1Value::Float(f)
                        }
                        PUSH_TYPE_NULL => Avm1Value::Null,
                        PUSH_TYPE_UNDEFINED => Avm1Value::Undefined,
                        PUSH_TYPE_REGISTER => {
                            let r = reader.a1pr_read_u8()?;
                            Avm1Value::Register(r)
                        }
                        PUSH_TYPE_BOOLEAN => {
                            let b = reader.a1pr_read_u8()? != 0;
                            Avm1Value::Boolean(b)
                        }
                        PUSH_TYPE_DOUBLE => {
                            let d = reader.a1pr_read_f64_le()?;
                            Avm1Value::Double(d)
                        }
                        PUSH_TYPE_INTEGER => {
                            let i = reader.a1pr_read_u32_le()? as i32;
                            Avm1Value::Integer(i)
                        }
                        PUSH_TYPE_CONSTANT8 => {
                            let idx = reader.a1pr_read_u8()?;
                            Avm1Value::Constant8(idx)
                        }
                        PUSH_TYPE_CONSTANT16 => {
                            let idx = reader.a1pr_read_u16_le()?;
                            Avm1Value::Constant16(idx)
                        }
                        other => {
                            return Err(LifterError::InvalidBytecode(format!(
                                "unknown ActionPush value type: {}",
                                other
                            )));
                        }
                    };
                    values.push(val);
                }
                self.pos = payload_end;
                let span = 3usize
                    .checked_add(payload_len)
                    .ok_or_else(|| LifterError::InvalidBytecode("overflow".to_string()))?;
                (Avm1ActionKind::Push(values), span)
            }
            ACTION_BRANCH_ALWAYS => {
                let offset = reader.a1pr_read_i16_le()?;
                self.pos = payload_end;
                let span = 3usize
                    .checked_add(payload_len)
                    .ok_or_else(|| LifterError::InvalidBytecode("overflow".to_string()))?;
                (Avm1ActionKind::BranchAlways(offset), span)
            }
            ACTION_BRANCH_IF_TRUE => {
                let offset = reader.a1pr_read_i16_le()?;
                self.pos = payload_end;
                let span = 3usize
                    .checked_add(payload_len)
                    .ok_or_else(|| LifterError::InvalidBytecode("overflow".to_string()))?;
                (Avm1ActionKind::BranchIfTrue(offset), span)
            }
            ACTION_DEFINE_FUNCTION => {
                let name = reader.a1pr_read_string_null_terminated(256)?;
                let num_params = reader.a1pr_read_u16_le()? as usize;
                let mut params = Vec::with_capacity(num_params.min(256));
                for _ in 0..num_params {
                    params.push(reader.a1pr_read_string_null_terminated(256)?);
                }
                let body_len = reader.a1pr_read_u16_le()? as usize;
                self.pos = payload_end;

                let body_end = payload_end
                    .checked_add(body_len)
                    .ok_or_else(|| LifterError::InvalidBytecode("offset overflow".to_string()))?;
                if body_end > self.data.len() {
                    return Err(LifterError::UnexpectedEof);
                }
                let body_slice = &self.data[payload_end..body_end];

                if self.depth >= MAX_FUNCTION_DEPTH {
                    return Err(LifterError::FunctionLimitExceeded);
                }
                let mut body_parser = Avm1Parser::a1p_new_with_depth(
                    body_slice,
                    self.depth + 1,
                    self.instruction_count,
                )?;
                let body = body_parser.a1p_parse_all()?;
                self.instruction_count = body_parser.instruction_count;
                self.pos = body_end;

                let total_span = 3usize
                    .checked_add(payload_len)
                    .and_then(|t| t.checked_add(body_len))
                    .ok_or_else(|| {
                        LifterError::InvalidBytecode("action length overflow".to_string())
                    })?;
                (
                    Avm1ActionKind::DefineFunction {
                        name,
                        params,
                        body_len,
                        body,
                    },
                    total_span,
                )
            }
            ACTION_DEFINE_FUNCTION2 => {
                let name = reader.a1pr_read_string_null_terminated(256)?;
                let num_params = reader.a1pr_read_u16_le()? as usize;
                let register_count = reader.a1pr_read_u8()?;
                let flags = reader.a1pr_read_u16_le()?;
                let mut params = Vec::with_capacity(num_params.min(256));
                for _ in 0..num_params {
                    let reg = reader.a1pr_read_u8()?;
                    let pname = reader.a1pr_read_string_null_terminated(256)?;
                    params.push((reg, pname));
                }
                let body_len = reader.a1pr_read_u16_le()? as usize;
                self.pos = payload_end;

                let body_end = payload_end
                    .checked_add(body_len)
                    .ok_or_else(|| LifterError::InvalidBytecode("offset overflow".to_string()))?;
                if body_end > self.data.len() {
                    return Err(LifterError::UnexpectedEof);
                }
                let body_slice = &self.data[payload_end..body_end];

                if self.depth >= MAX_FUNCTION_DEPTH {
                    return Err(LifterError::FunctionLimitExceeded);
                }
                let mut body_parser = Avm1Parser::a1p_new_with_depth(
                    body_slice,
                    self.depth + 1,
                    self.instruction_count,
                )?;
                let body = body_parser.a1p_parse_all()?;
                self.instruction_count = body_parser.instruction_count;
                self.pos = body_end;

                let total_span = 3usize
                    .checked_add(payload_len)
                    .and_then(|t| t.checked_add(body_len))
                    .ok_or_else(|| {
                        LifterError::InvalidBytecode("action length overflow".to_string())
                    })?;
                (
                    Avm1ActionKind::DefineFunction2 {
                        name,
                        register_count,
                        flags,
                        params,
                        body_len,
                        body,
                    },
                    total_span,
                )
            }
            _ => {
                self.pos = payload_end;
                return Err(LifterError::UnsupportedOpcode(op));
            }
        };

        if self.pos < payload_end {
            self.pos = payload_end;
        }

        Ok(Avm1Action {
            offset: start_offset,
            length: total_length,
            kind,
        })
    }
}
