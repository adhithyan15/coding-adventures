//! Safe, bounded parser for Flash AVM1 bytecode streams.

#![deny(unsafe_code)]

use super::action::{Avm1Action, Avm1ActionKind};
use super::opcode::*;
use super::value::Avm1Value;
use crate::error::{LifterError, MAX_BYTECODE_SIZE, MAX_CONSTANTS, MAX_INSTRUCTIONS};

/// Bounded parser for AVM1 byte streams.
pub struct Avm1Parser<'a> {
    data: &'a [u8],
    pos: usize,
    instruction_count: usize,
}

impl<'a> Avm1Parser<'a> {
    /// Create a new AVM1 parser for the given bytecode slice.
    pub fn a1p_new(data: &'a [u8]) -> Result<Self, LifterError> {
        if data.len() > MAX_BYTECODE_SIZE {
            return Err(LifterError::InvalidBytecode(
                "bytecode stream exceeds maximum allowed size".to_string(),
            ));
        }
        Ok(Self {
            data,
            pos: 0,
            instruction_count: 0,
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

    fn a1p_remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    fn a1p_read_u8(&mut self) -> Result<u8, LifterError> {
        if self.pos >= self.data.len() {
            return Err(LifterError::UnexpectedEof);
        }
        let b = self.data[self.pos];
        self.pos += 1;
        Ok(b)
    }

    fn a1p_read_u16_le(&mut self) -> Result<u16, LifterError> {
        if self.a1p_remaining() < 2 {
            return Err(LifterError::UnexpectedEof);
        }
        let b0 = self.data[self.pos] as u16;
        let b1 = self.data[self.pos + 1] as u16;
        self.pos += 2;
        Ok(b0 | (b1 << 8))
    }

    fn a1p_read_i16_le(&mut self) -> Result<i16, LifterError> {
        self.a1p_read_u16_le().map(|u| u as i16)
    }

    fn a1p_read_u32_le(&mut self) -> Result<u32, LifterError> {
        if self.a1p_remaining() < 4 {
            return Err(LifterError::UnexpectedEof);
        }
        let b0 = self.data[self.pos] as u32;
        let b1 = self.data[self.pos + 1] as u32;
        let b2 = self.data[self.pos + 2] as u32;
        let b3 = self.data[self.pos + 3] as u32;
        self.pos += 4;
        Ok(b0 | (b1 << 8) | (b2 << 16) | (b3 << 24))
    }

    fn a1p_read_f32_le(&mut self) -> Result<f32, LifterError> {
        self.a1p_read_u32_le().map(f32::from_bits)
    }

    fn a1p_read_f64_le(&mut self) -> Result<f64, LifterError> {
        if self.a1p_remaining() < 8 {
            return Err(LifterError::UnexpectedEof);
        }
        // In Flash AVM1 ActionPush, double floats are swapped 32-bit words in Flash 5/6!
        // Read 8 bytes
        let bytes: [u8; 8] = self.data[self.pos..self.pos + 8]
            .try_into()
            .map_err(|_| LifterError::UnexpectedEof)?;
        self.pos += 8;
        // Check SWF double format: lower and upper 32-bit words are swapped
        let w0 = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let w1 = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let u = (w0 as u64) | ((w1 as u64) << 32);
        Ok(f64::from_bits(u))
    }

    fn a1p_read_string_null_terminated(&mut self, max_len: usize) -> Result<String, LifterError> {
        let mut bytes = Vec::new();
        while self.pos < self.data.len() {
            let b = self.a1p_read_u8()?;
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
        // If EOF before null byte
        Ok(String::from_utf8_lossy(&bytes).to_string())
    }

    fn a1p_parse_action(&mut self) -> Result<Avm1Action, LifterError> {
        self.instruction_count += 1;
        if self.instruction_count > MAX_INSTRUCTIONS {
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
        if self.a1p_remaining() < payload_len {
            return Err(LifterError::UnexpectedEof);
        }
        let payload_end = self.pos + payload_len;

        let (kind, total_length) = match op {
            ACTION_GOTO_FRAME => {
                let frame = self.a1p_read_u16_le()?;
                (Avm1ActionKind::GotoFrame(frame), 3 + payload_len)
            }
            ACTION_STORE_REGISTER => {
                let reg = self.a1p_read_u8()?;
                (Avm1ActionKind::StoreRegister(reg), 3 + payload_len)
            }
            ACTION_CONSTANT_POOL => {
                let count = self.a1p_read_u16_le()? as usize;
                if count > MAX_CONSTANTS {
                    return Err(LifterError::ConstantPoolLimitExceeded);
                }
                let mut constants = Vec::with_capacity(count.min(1024));
                for _ in 0..count {
                    let s = self.a1p_read_string_null_terminated(65536)?;
                    constants.push(s);
                }
                (Avm1ActionKind::ConstantPool(constants), 3 + payload_len)
            }
            ACTION_PUSH => {
                let mut values = Vec::new();
                while self.pos < payload_end {
                    let ptype = self.a1p_read_u8()?;
                    let val = match ptype {
                        PUSH_TYPE_STRING => {
                            let s = self.a1p_read_string_null_terminated(65536)?;
                            Avm1Value::String(s)
                        }
                        PUSH_TYPE_FLOAT => {
                            let f = self.a1p_read_f32_le()?;
                            Avm1Value::Float(f)
                        }
                        PUSH_TYPE_NULL => Avm1Value::Null,
                        PUSH_TYPE_UNDEFINED => Avm1Value::Undefined,
                        PUSH_TYPE_REGISTER => {
                            let r = self.a1p_read_u8()?;
                            Avm1Value::Register(r)
                        }
                        PUSH_TYPE_BOOLEAN => {
                            let b = self.a1p_read_u8()? != 0;
                            Avm1Value::Boolean(b)
                        }
                        PUSH_TYPE_DOUBLE => {
                            let d = self.a1p_read_f64_le()?;
                            Avm1Value::Double(d)
                        }
                        PUSH_TYPE_INTEGER => {
                            let i = self.a1p_read_u32_le()? as i32;
                            Avm1Value::Integer(i)
                        }
                        PUSH_TYPE_CONSTANT8 => {
                            let idx = self.a1p_read_u8()?;
                            Avm1Value::Constant8(idx)
                        }
                        PUSH_TYPE_CONSTANT16 => {
                            let idx = self.a1p_read_u16_le()?;
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
                (Avm1ActionKind::Push(values), 3 + payload_len)
            }
            ACTION_BRANCH_ALWAYS => {
                let offset = self.a1p_read_i16_le()?;
                (Avm1ActionKind::BranchAlways(offset), 3 + payload_len)
            }
            ACTION_BRANCH_IF_TRUE => {
                let offset = self.a1p_read_i16_le()?;
                (Avm1ActionKind::BranchIfTrue(offset), 3 + payload_len)
            }
            ACTION_DEFINE_FUNCTION => {
                let name = self.a1p_read_string_null_terminated(256)?;
                let num_params = self.a1p_read_u16_le()? as usize;
                let mut params = Vec::with_capacity(num_params.min(256));
                for _ in 0..num_params {
                    params.push(self.a1p_read_string_null_terminated(256)?);
                }
                let body_len = self.a1p_read_u16_le()? as usize;
                // Move position to end of header payload
                self.pos = payload_end;

                // Read function body
                if self.a1p_remaining() < body_len {
                    return Err(LifterError::UnexpectedEof);
                }
                let body_slice = &self.data[self.pos..self.pos + body_len];
                let mut body_parser = Avm1Parser::a1p_new(body_slice)?;
                let body = body_parser.a1p_parse_all()?;
                self.pos += body_len;

                let total_span = 3 + payload_len + body_len;
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
                let name = self.a1p_read_string_null_terminated(256)?;
                let num_params = self.a1p_read_u16_le()? as usize;
                let register_count = self.a1p_read_u8()?;
                let flags = self.a1p_read_u16_le()?;
                let mut params = Vec::with_capacity(num_params.min(256));
                for _ in 0..num_params {
                    let reg = self.a1p_read_u8()?;
                    let pname = self.a1p_read_string_null_terminated(256)?;
                    params.push((reg, pname));
                }
                let body_len = self.a1p_read_u16_le()? as usize;
                self.pos = payload_end;

                if self.a1p_remaining() < body_len {
                    return Err(LifterError::UnexpectedEof);
                }
                let body_slice = &self.data[self.pos..self.pos + body_len];
                let mut body_parser = Avm1Parser::a1p_new(body_slice)?;
                let body = body_parser.a1p_parse_all()?;
                self.pos += body_len;

                let total_span = 3 + payload_len + body_len;
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
                // Unsupported action with payload, skip over it safely
                self.pos = payload_end;
                return Err(LifterError::UnsupportedOpcode(op));
            }
        };

        // Ensure parser position is at the end of this action
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
