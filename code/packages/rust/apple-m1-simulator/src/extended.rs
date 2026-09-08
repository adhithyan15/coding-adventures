use super::{AppleM1Error, AppleM1State, MEMORY_SIZE, XZR};

const MASK32: u128 = u32::MAX as u128;
const MASK64: u128 = u64::MAX as u128;

pub(super) const fn is_extended(raw: u32) -> bool {
    ((raw >> 24) & 0x1f == 0b11110)
        || ((raw >> 24) & 0x1f == 0b01110)
        || (((raw >> 27) & 7 == 7) && ((raw >> 24) & 3 == 1) && ((raw >> 26) & 1 == 1))
}

pub(super) fn execute(state: &mut AppleM1State, raw: u32) -> Result<&'static str, AppleM1Error> {
    let mnemonic = if ((raw >> 24) & 0x1f) == 0b11110 {
        execute_scalar(state, raw)?
    } else if ((raw >> 24) & 0x1f) == 0b01110 {
        execute_neon(state, raw)?
    } else {
        execute_load_store(state, raw)?
    };
    state.pc = state.pc.wrapping_add(4);
    Ok(mnemonic)
}

fn unknown(state: &AppleM1State, raw: u32) -> AppleM1Error {
    AppleM1Error::UnknownInstruction { raw, pc: state.pc }
}

fn read_gpr(state: &AppleM1State, index: usize, sf: bool) -> u64 {
    let value = if index == XZR {
        0
    } else {
        state.registers[index]
    };
    if sf {
        value
    } else {
        value & u64::from(u32::MAX)
    }
}

fn write_gpr(state: &mut AppleM1State, index: usize, value: u64, sf: bool) {
    if index != XZR {
        state.registers[index] = if sf {
            value
        } else {
            value & u64::from(u32::MAX)
        };
    }
    state.registers[XZR] = 0;
}

fn low32(value: u128) -> u32 {
    value as u32
}

fn low64(value: u128) -> u64 {
    value as u64
}

fn write_low32(state: &mut AppleM1State, index: usize, value: u32) {
    state.vectors[index] = u128::from(value);
}

fn write_low64(state: &mut AppleM1State, index: usize, value: u64) {
    state.vectors[index] = u128::from(value);
}

fn fp_nzcv(a: f64, b: f64) -> u8 {
    if a.is_nan() || b.is_nan() {
        0b0011
    } else if a == b {
        0b0110
    } else if a < b {
        0b1000
    } else {
        0b0010
    }
}

fn div64(a: f64, b: f64) -> f64 {
    if b == 0.0 {
        if (a * b).is_sign_negative() {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        }
    } else {
        a / b
    }
}

fn div32(a: f64, b: f64) -> f32 {
    if b == 0.0 {
        if (a * b).is_sign_negative() {
            f32::NEG_INFINITY
        } else {
            f32::INFINITY
        }
    } else {
        (a / b) as f32
    }
}

fn execute_scalar(state: &mut AppleM1State, raw: u32) -> Result<&'static str, AppleM1Error> {
    let sf = raw >> 31 != 0;
    let ftype = (raw >> 22) & 3;
    if ftype > 1 || ((raw >> 21) & 1) == 0 {
        return Err(unknown(state, raw));
    }
    let double = ftype == 1;
    let field_15_10 = (raw >> 10) & 0x3f;
    let field_20_16 = (raw >> 16) & 0x1f;
    let rn = ((raw >> 5) & 0x1f) as usize;
    let rd = (raw & 0x1f) as usize;

    if field_15_10 == 0b001000 {
        let rm = field_20_16 as usize;
        let zero = (raw & 3) == 3;
        state.nzcv = if double {
            fp_nzcv(
                f64::from_bits(low64(state.vectors[rn])),
                if zero {
                    0.0
                } else {
                    f64::from_bits(low64(state.vectors[rm]))
                },
            )
        } else {
            fp_nzcv(
                f64::from(f32::from_bits(low32(state.vectors[rn]))),
                if zero {
                    0.0
                } else {
                    f64::from(f32::from_bits(low32(state.vectors[rm])))
                },
            )
        };
        return Ok("fcmp");
    }

    if field_15_10 == 0 {
        match field_20_16 {
            0b00110 => {
                if double {
                    write_gpr(state, rd, low64(state.vectors[rn]), true);
                } else {
                    write_gpr(state, rd, u64::from(low32(state.vectors[rn])), false);
                }
                return Ok("fmov");
            }
            0b00111 => {
                if double {
                    write_low64(state, rd, read_gpr(state, rn, true));
                } else {
                    write_low32(state, rd, read_gpr(state, rn, false) as u32);
                }
                return Ok("fmov");
            }
            0b11000 => {
                let value = if double {
                    f64::from_bits(low64(state.vectors[rn]))
                } else {
                    f64::from(f32::from_bits(low32(state.vectors[rn])))
                };
                let result = if sf {
                    if value.is_nan() {
                        0
                    } else if value >= i64::MAX as f64 {
                        i64::MAX as u64
                    } else if value <= i64::MIN as f64 {
                        i64::MIN as u64
                    } else {
                        (value.trunc() as i64) as u64
                    }
                } else {
                    let signed = if value.is_nan() {
                        0
                    } else if value >= f64::from(i32::MAX) {
                        i32::MAX
                    } else if value <= f64::from(i32::MIN) {
                        i32::MIN
                    } else {
                        value.trunc() as i32
                    };
                    signed as u32 as u64
                };
                write_gpr(state, rd, result, sf);
                return Ok("fcvtzs");
            }
            0b00010 | 0b00011 => {
                let signed = field_20_16 == 0b00010;
                let value = if signed {
                    if sf {
                        read_gpr(state, rn, true) as i64 as f64
                    } else {
                        f64::from(read_gpr(state, rn, false) as u32 as i32)
                    }
                } else if sf {
                    read_gpr(state, rn, true) as f64
                } else {
                    f64::from(read_gpr(state, rn, false) as u32)
                };
                if double {
                    write_low64(state, rd, value.to_bits());
                } else {
                    write_low32(state, rd, (value as f32).to_bits());
                }
                return Ok(if signed { "scvtf" } else { "ucvtf" });
            }
            _ => return Err(unknown(state, raw)),
        }
    }

    if ((raw >> 10) & 3) == 2 {
        let opcode = (raw >> 12) & 0xf;
        let rm = ((raw >> 16) & 0x1f) as usize;
        if opcode > 3 {
            return Err(unknown(state, raw));
        }
        let mnemonic = ["fmul", "fdiv", "fadd", "fsub"][opcode as usize];
        if double {
            let a = f64::from_bits(low64(state.vectors[rn]));
            let b = f64::from_bits(low64(state.vectors[rm]));
            let result = match opcode {
                0 => a * b,
                1 => div64(a, b),
                2 => a + b,
                _ => a - b,
            };
            write_low64(state, rd, result.to_bits());
        } else {
            let a = f64::from(f32::from_bits(low32(state.vectors[rn])));
            let b = f64::from(f32::from_bits(low32(state.vectors[rm])));
            let result = match opcode {
                0 => (a * b) as f32,
                1 => div32(a, b),
                2 => (a + b) as f32,
                _ => (a - b) as f32,
            };
            write_low32(state, rd, result.to_bits());
        }
        return Ok(mnemonic);
    }

    if ((raw >> 10) & 0x1f) == 0b10000 {
        let opcode = (raw >> 15) & 0x3f;
        if opcode > 4 {
            return Err(unknown(state, raw));
        }
        let mnemonic = ["fmov", "fabs", "fneg", "fsqrt", "fcvt"][opcode as usize];
        if opcode == 4 {
            if double {
                let converted = (f64::from_bits(low64(state.vectors[rn])) as f32).to_bits();
                write_low32(state, rd, converted);
            } else {
                write_low64(
                    state,
                    rd,
                    f64::from(f32::from_bits(low32(state.vectors[rn]))).to_bits(),
                );
            }
        } else if double {
            let value = f64::from_bits(low64(state.vectors[rn]));
            let result = match opcode {
                0 => value,
                1 => value.abs(),
                2 => -value,
                _ => value.abs().sqrt(),
            };
            write_low64(state, rd, result.to_bits());
        } else {
            let value = f64::from(f32::from_bits(low32(state.vectors[rn])));
            let result = match opcode {
                0 => value as f32,
                1 => value.abs() as f32,
                2 => (-value) as f32,
                _ => value.abs().sqrt() as f32,
            };
            write_low32(state, rd, result.to_bits());
        }
        return Ok(mnemonic);
    }
    Err(unknown(state, raw))
}

fn execute_load_store(state: &mut AppleM1State, raw: u32) -> Result<&'static str, AppleM1Error> {
    let size = (raw >> 30) & 3;
    let opc = (raw >> 22) & 3;
    if size < 2 || opc > 1 {
        return Err(unknown(state, raw));
    }
    let width = 1usize << size;
    let rn = ((raw >> 5) & 0x1f) as usize;
    let rt = (raw & 0x1f) as usize;
    let base = if rn == XZR {
        state.sp
    } else {
        read_gpr(state, rn, true)
    };
    let address = base.wrapping_add(u64::from((raw >> 10) & 0xfff) * width as u64);
    check_data(address, width)?;
    let start = address as usize;
    if opc == 0 {
        let bytes = state.vectors[rt].to_be_bytes();
        state.memory[start..start + width].copy_from_slice(&bytes[16 - width..]);
        Ok(if width == 4 { "str_s" } else { "str_d" })
    } else {
        let mut bytes = [0_u8; 16];
        bytes[16 - width..].copy_from_slice(&state.memory[start..start + width]);
        state.vectors[rt] = u128::from_be_bytes(bytes);
        Ok(if width == 4 { "ldr_s" } else { "ldr_d" })
    }
}

fn check_data(address: u64, width: usize) -> Result<(), AppleM1Error> {
    if !address.is_multiple_of(width as u64) {
        return Err(AppleM1Error::MisalignedData { address, width });
    }
    if address
        .checked_add(width as u64)
        .is_none_or(|end| end > MEMORY_SIZE as u64)
    {
        return Err(AppleM1Error::DataOutOfRange { address, width });
    }
    Ok(())
}

fn execute_neon(state: &mut AppleM1State, raw: u32) -> Result<&'static str, AppleM1Error> {
    let q = ((raw >> 30) & 1) != 0;
    let unsigned = ((raw >> 29) & 1) != 0;
    let size = (raw >> 22) & 3;
    let bit21 = (raw >> 21) & 1;
    let rm = ((raw >> 16) & 0x1f) as usize;
    let opcode = (raw >> 11) & 0x1f;
    let rn = ((raw >> 5) & 0x1f) as usize;
    let rd = (raw & 0x1f) as usize;

    if ((raw >> 14) & 0x1f) == 1
        && ((raw >> 13) & 1) == 1
        && ((raw >> 11) & 3) == 0
        && ((raw >> 10) & 1) == 1
        && !unsigned
    {
        let imm5 = (raw >> 19) & 0x1f;
        let elem = if imm5 & 1 != 0 {
            8
        } else if imm5 & 2 != 0 {
            16
        } else if imm5 & 4 != 0 || imm5 & 8 != 0 {
            32
        } else if imm5 & 16 != 0 {
            64
        } else {
            return Err(unknown(state, raw));
        };
        let total = if q { 128 } else { 64 };
        let mask = if elem == 64 {
            u128::from(u64::MAX)
        } else {
            (1_u128 << elem) - 1
        };
        let lane = u128::from(read_gpr(state, rn, true)) & mask;
        let mut result = 0;
        for shift in (0..total).step_by(elem) {
            result |= lane << shift;
        }
        state.vectors[rd] = result;
        return Ok("dup");
    }
    if bit21 != 1 || ((raw >> 10) & 1) == 0 {
        return Err(unknown(state, raw));
    }
    let total = if q { 128 } else { 64 };
    if opcode == 0x10 || opcode == 0x13 {
        if opcode == 0x13 && size == 3 {
            return Err(unknown(state, raw));
        }
        let elem = 8usize << size;
        let mask = if elem == 64 {
            MASK64
        } else {
            (1_u128 << elem) - 1
        };
        let mut result = 0;
        for shift in (0..total).step_by(elem) {
            let a = (state.vectors[rn] >> shift) & mask;
            let b = (state.vectors[rm] >> shift) & mask;
            let lane = if opcode == 0x13 {
                a.wrapping_mul(b)
            } else if unsigned {
                a.wrapping_sub(b)
            } else {
                a.wrapping_add(b)
            };
            result |= (lane & mask) << shift;
        }
        state.vectors[rd] = result;
        return Ok(if opcode == 0x13 {
            "vmul"
        } else if unsigned {
            "vsub"
        } else {
            "vadd"
        });
    }
    if (raw >> 23) & 1 != 0 || !matches!(opcode, 0x19..=0x1b) {
        return Err(unknown(state, raw));
    }
    let double = ((raw >> 22) & 1) != 0;
    let elem = if double { 64 } else { 32 };
    let mask = if double { MASK64 } else { MASK32 };
    let mut result = 0;
    for shift in (0..total).step_by(elem) {
        let n = (state.vectors[rn] >> shift) & mask;
        let m = (state.vectors[rm] >> shift) & mask;
        let d = (state.vectors[rd] >> shift) & mask;
        let lane = if double {
            let n = f64::from_bits(n as u64);
            let m = f64::from_bits(m as u64);
            let value = match opcode {
                0x19 => f64::from_bits(d as u64) + n * m,
                0x1a if unsigned => n - m,
                0x1a => n + m,
                _ => n * m,
            };
            u128::from(value.to_bits())
        } else {
            let n = f64::from(f32::from_bits(n as u32));
            let m = f64::from(f32::from_bits(m as u32));
            let value = match opcode {
                0x19 => f64::from(f32::from_bits(d as u32)) + n * m,
                0x1a if unsigned => n - m,
                0x1a => n + m,
                _ => n * m,
            };
            u128::from((value as f32).to_bits())
        };
        result |= (lane & mask) << shift;
    }
    state.vectors[rd] = result;
    Ok(match opcode {
        0x19 => "fmla",
        0x1a if unsigned => "vfsub",
        0x1a => "vfadd",
        _ => "vfmul",
    })
}
