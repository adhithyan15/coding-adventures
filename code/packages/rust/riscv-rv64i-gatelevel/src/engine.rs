//! Independent RV64I+M decode and repository-gate execution engine.

use arithmetic::adders::ripple_carry_adder_with_carry;
use logic_gates::combinational::mux2;
use logic_gates::gates::{and_gate, not_gate, or_gate, xnor_gate, xor_gate};
use riscv_rv64i_simulator::{Rv64IError, Rv64IState, MEMORY_SIZE};

use crate::state::{bits64, word64};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operation {
    ZeroHalt,
    Ecall,
    Ebreak,
    Lui,
    Auipc,
    Jal,
    Jalr,
    Beq,
    Bne,
    Blt,
    Bge,
    Bltu,
    Bgeu,
    Lb,
    Lh,
    Lw,
    Ld,
    Lbu,
    Lhu,
    Lwu,
    Sb,
    Sh,
    Sw,
    Sd,
    Addi,
    Slti,
    Sltiu,
    Xori,
    Ori,
    Andi,
    Slli,
    Srli,
    Srai,
    Add,
    Sub,
    Sll,
    Slt,
    Sltu,
    Xor,
    Srl,
    Sra,
    Or,
    And,
    Addiw,
    Slliw,
    Srliw,
    Sraiw,
    Addw,
    Subw,
    Sllw,
    Srlw,
    Sraw,
    Mul,
    Mulh,
    Mulhsu,
    Mulhu,
    Div,
    Divu,
    Rem,
    Remu,
    Mulw,
    Divw,
    Divuw,
    Remw,
    Remuw,
    Fence,
    FenceI,
}

impl Operation {
    fn mnemonic(self) -> &'static str {
        match self {
            Self::ZeroHalt => "zero-halt",
            Self::Ecall => "ecall",
            Self::Ebreak => "ebreak",
            Self::Lui => "lui",
            Self::Auipc => "auipc",
            Self::Jal => "jal",
            Self::Jalr => "jalr",
            Self::Beq => "beq",
            Self::Bne => "bne",
            Self::Blt => "blt",
            Self::Bge => "bge",
            Self::Bltu => "bltu",
            Self::Bgeu => "bgeu",
            Self::Lb => "lb",
            Self::Lh => "lh",
            Self::Lw => "lw",
            Self::Ld => "ld",
            Self::Lbu => "lbu",
            Self::Lhu => "lhu",
            Self::Lwu => "lwu",
            Self::Sb => "sb",
            Self::Sh => "sh",
            Self::Sw => "sw",
            Self::Sd => "sd",
            Self::Addi => "addi",
            Self::Slti => "slti",
            Self::Sltiu => "sltiu",
            Self::Xori => "xori",
            Self::Ori => "ori",
            Self::Andi => "andi",
            Self::Slli => "slli",
            Self::Srli => "srli",
            Self::Srai => "srai",
            Self::Add => "add",
            Self::Sub => "sub",
            Self::Sll => "sll",
            Self::Slt => "slt",
            Self::Sltu => "sltu",
            Self::Xor => "xor",
            Self::Srl => "srl",
            Self::Sra => "sra",
            Self::Or => "or",
            Self::And => "and",
            Self::Addiw => "addiw",
            Self::Slliw => "slliw",
            Self::Srliw => "srliw",
            Self::Sraiw => "sraiw",
            Self::Addw => "addw",
            Self::Subw => "subw",
            Self::Sllw => "sllw",
            Self::Srlw => "srlw",
            Self::Sraw => "sraw",
            Self::Mul => "mul",
            Self::Mulh => "mulh",
            Self::Mulhsu => "mulhsu",
            Self::Mulhu => "mulhu",
            Self::Div => "div",
            Self::Divu => "divu",
            Self::Rem => "rem",
            Self::Remu => "remu",
            Self::Mulw => "mulw",
            Self::Divw => "divw",
            Self::Divuw => "divuw",
            Self::Remw => "remw",
            Self::Remuw => "remuw",
            Self::Fence => "fence",
            Self::FenceI => "fence.i",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Decoded {
    operation: Operation,
    rd: usize,
    rs1: usize,
    rs2: usize,
    immediate: u64,
}

fn sign_extend(value: u32, width: u32) -> u64 {
    let shift = 64 - width;
    ((u64::from(value) << shift) as i64 >> shift) as u64
}

fn decode(raw: u32, pc: u64) -> Result<Decoded, Rv64IError> {
    let opcode = raw & 0x7f;
    let rd = ((raw >> 7) & 0x1f) as usize;
    let funct3 = (raw >> 12) & 7;
    let rs1 = ((raw >> 15) & 0x1f) as usize;
    let rs2 = ((raw >> 20) & 0x1f) as usize;
    let funct7 = raw >> 25;
    let unknown = || Rv64IError::UnknownInstruction { raw, pc };
    let mut immediate = 0;
    let operation = match opcode {
        0x37 => {
            immediate = sign_extend(raw & 0xffff_f000, 32);
            Operation::Lui
        }
        0x17 => {
            immediate = sign_extend(raw & 0xffff_f000, 32);
            Operation::Auipc
        }
        0x6f => {
            immediate = sign_extend(
                ((raw >> 31) << 20)
                    | (((raw >> 12) & 0xff) << 12)
                    | (((raw >> 20) & 1) << 11)
                    | (((raw >> 21) & 0x3ff) << 1),
                21,
            );
            Operation::Jal
        }
        0x67 if funct3 == 0 => {
            immediate = sign_extend(raw >> 20, 12);
            Operation::Jalr
        }
        0x63 => {
            immediate = sign_extend(
                ((raw >> 31) << 12)
                    | (((raw >> 7) & 1) << 11)
                    | (((raw >> 25) & 0x3f) << 5)
                    | (((raw >> 8) & 0xf) << 1),
                13,
            );
            match funct3 {
                0 => Operation::Beq,
                1 => Operation::Bne,
                4 => Operation::Blt,
                5 => Operation::Bge,
                6 => Operation::Bltu,
                7 => Operation::Bgeu,
                _ => return Err(unknown()),
            }
        }
        0x03 => {
            immediate = sign_extend(raw >> 20, 12);
            match funct3 {
                0 => Operation::Lb,
                1 => Operation::Lh,
                2 => Operation::Lw,
                3 => Operation::Ld,
                4 => Operation::Lbu,
                5 => Operation::Lhu,
                6 => Operation::Lwu,
                _ => return Err(unknown()),
            }
        }
        0x23 => {
            immediate = sign_extend(((raw >> 25) << 5) | ((raw >> 7) & 0x1f), 12);
            match funct3 {
                0 => Operation::Sb,
                1 => Operation::Sh,
                2 => Operation::Sw,
                3 => Operation::Sd,
                _ => return Err(unknown()),
            }
        }
        0x13 => {
            immediate = sign_extend(raw >> 20, 12);
            match funct3 {
                0 => Operation::Addi,
                2 => Operation::Slti,
                3 => Operation::Sltiu,
                4 => Operation::Xori,
                6 => Operation::Ori,
                7 => Operation::Andi,
                1 if raw >> 26 == 0 => Operation::Slli,
                5 if raw >> 26 == 0 => Operation::Srli,
                5 if raw >> 26 == 0x10 => Operation::Srai,
                _ => return Err(unknown()),
            }
        }
        0x33 if funct7 == 1 => match funct3 {
            0 => Operation::Mul,
            1 => Operation::Mulh,
            2 => Operation::Mulhsu,
            3 => Operation::Mulhu,
            4 => Operation::Div,
            5 => Operation::Divu,
            6 => Operation::Rem,
            7 => Operation::Remu,
            _ => unreachable!(),
        },
        0x33 => match (funct7, funct3) {
            (0, 0) => Operation::Add,
            (0x20, 0) => Operation::Sub,
            (0, 1) => Operation::Sll,
            (0, 2) => Operation::Slt,
            (0, 3) => Operation::Sltu,
            (0, 4) => Operation::Xor,
            (0, 5) => Operation::Srl,
            (0x20, 5) => Operation::Sra,
            (0, 6) => Operation::Or,
            (0, 7) => Operation::And,
            _ => return Err(unknown()),
        },
        0x1b => {
            immediate = sign_extend(raw >> 20, 12);
            match (funct7, funct3) {
                (_, 0) => Operation::Addiw,
                (0, 1) => Operation::Slliw,
                (0, 5) => Operation::Srliw,
                (0x20, 5) => Operation::Sraiw,
                _ => return Err(unknown()),
            }
        }
        0x3b if funct7 == 1 => match funct3 {
            0 => Operation::Mulw,
            4 => Operation::Divw,
            5 => Operation::Divuw,
            6 => Operation::Remw,
            7 => Operation::Remuw,
            _ => return Err(unknown()),
        },
        0x3b => match (funct7, funct3) {
            (0, 0) => Operation::Addw,
            (0x20, 0) => Operation::Subw,
            (0, 1) => Operation::Sllw,
            (0, 5) => Operation::Srlw,
            (0x20, 5) => Operation::Sraw,
            _ => return Err(unknown()),
        },
        0x0f if funct3 == 0 && rs1 == 0 && rd == 0 => Operation::Fence,
        0x0f if funct3 == 1 && raw >> 20 == 0 && rs1 == 0 && rd == 0 => Operation::FenceI,
        0x73 if raw == 0x0000_0073 => Operation::Ecall,
        0x73 if raw == 0x0010_0073 => Operation::Ebreak,
        _ => return Err(unknown()),
    };
    Ok(Decoded {
        operation,
        rd,
        rs1,
        rs2,
        immediate,
    })
}

fn gate_boolean(left: u64, right: u64, gate: fn(u8, u8) -> u8) -> u64 {
    let left = bits64(left);
    let right = bits64(right);
    word64(&std::array::from_fn(|bit| gate(left[bit], right[bit])))
}

fn gate_add(left: u64, right: u64) -> u64 {
    let output = ripple_carry_adder_with_carry(&bits64(left), &bits64(right), 0);
    let sum: [u8; 64] = output.sum.try_into().expect("64-bit ripple result");
    word64(&sum)
}

fn gate_subtract_full(left: u64, right: u64) -> (u64, bool, bool) {
    let left_bits = bits64(left);
    let right_bits = bits64(right);
    let inverted: [u8; 64] = std::array::from_fn(|bit| not_gate(right_bits[bit]));
    let output = ripple_carry_adder_with_carry(&left_bits, &inverted, 1);
    let difference: [u8; 64] = output.sum.try_into().expect("64-bit ripple result");
    let overflow = and_gate(
        xor_gate(left_bits[63], right_bits[63]),
        xor_gate(left_bits[63], difference[63]),
    ) != 0;
    (word64(&difference), output.carry_out != 0, overflow)
}

fn gate_subtract(left: u64, right: u64) -> u64 {
    gate_subtract_full(left, right).0
}

fn gate_equal(left: u64, right: u64, width: usize) -> bool {
    let left = bits64(left);
    let right = bits64(right);
    left[..width]
        .iter()
        .zip(&right[..width])
        .map(|(&a, &b)| xnor_gate(a, b))
        .fold(1, and_gate)
        != 0
}

fn gate_unsigned_less(left: u64, right: u64) -> bool {
    !gate_subtract_full(left, right).1
}

fn gate_signed_less(left: u64, right: u64) -> bool {
    let (difference, _, overflow) = gate_subtract_full(left, right);
    (bits64(difference)[63] != 0) ^ overflow
}

#[derive(Debug, Clone, Copy)]
enum ShiftKind {
    Left,
    Right,
    ArithmeticRight,
}

fn gate_barrel(value: u64, kind: ShiftKind, amount: u64, width: usize) -> u64 {
    let selects = bits64(amount);
    let input = bits64(value);
    let mut current = input[..width].to_vec();
    let stages = width.ilog2() as usize;
    for (stage, select) in selects[..stages].iter().copied().enumerate() {
        let distance = 1usize << stage;
        let shifted: Vec<u8> = (0..width)
            .map(|bit| match kind {
                ShiftKind::Left => bit
                    .checked_sub(distance)
                    .map_or(0, |source| current[source]),
                ShiftKind::Right => current.get(bit + distance).copied().unwrap_or(0),
                ShiftKind::ArithmeticRight => current
                    .get(bit + distance)
                    .copied()
                    .unwrap_or(current[width - 1]),
            })
            .collect();
        current = current
            .iter()
            .zip(shifted)
            .map(|(&old, shifted)| mux2(old, shifted, select))
            .collect();
    }
    current
        .iter()
        .enumerate()
        .fold(0, |word, (bit, q)| word | (u64::from(*q) << bit))
}

fn gate_negate(value: u64) -> u64 {
    gate_add(gate_boolean(value, u64::MAX, xor_gate), 1)
}

fn gate_sign_extend(value: u64, width: usize) -> u64 {
    let bits = bits64(value);
    word64(&std::array::from_fn(|bit| {
        if bit < width {
            bits[bit]
        } else {
            bits[width - 1]
        }
    }))
}

fn bits128(value: u128) -> [u8; 128] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

fn word128(bits: &[u8; 128]) -> u128 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, q)| value | (u128::from(*q) << bit))
}

fn gate_negate_wide(value: u128) -> u128 {
    let bits = bits128(value);
    let inverted: [u8; 128] = std::array::from_fn(|bit| not_gate(bits[bit]));
    let one: [u8; 128] = std::array::from_fn(|bit| u8::from(bit == 0));
    let output = ripple_carry_adder_with_carry(&inverted, &one, 0);
    let result: [u8; 128] = output.sum.try_into().expect("128-bit ripple result");
    word128(&result)
}

fn gate_mul_wide(left: u64, right: u64) -> u128 {
    let left = bits64(left);
    let right = bits64(right);
    let mut accumulator = [0; 128];
    for (shift, select) in right.iter().copied().enumerate() {
        let addend: [u8; 128] = std::array::from_fn(|bit| {
            bit.checked_sub(shift)
                .filter(|source| *source < 64)
                .map_or(0, |source| and_gate(left[source], select))
        });
        let output = ripple_carry_adder_with_carry(&accumulator, &addend, 0);
        accumulator = output.sum.try_into().expect("128-bit multiply stage");
    }
    word128(&accumulator)
}

fn gate_mul_signed(left: u64, right: u64, right_signed: bool) -> u128 {
    let left_negative = bits64(left)[63] != 0;
    let right_negative = right_signed && bits64(right)[63] != 0;
    let left_magnitude = if left_negative {
        gate_negate(left)
    } else {
        left
    };
    let right_magnitude = if right_negative {
        gate_negate(right)
    } else {
        right
    };
    let product = gate_mul_wide(left_magnitude, right_magnitude);
    if left_negative ^ right_negative {
        gate_negate_wide(product)
    } else {
        product
    }
}

fn gate_divrem_unsigned(dividend: u64, divisor: u64, width: usize) -> (u64, u64) {
    if gate_equal(divisor, 0, width) {
        return (u64::MAX, dividend);
    }
    let dividend_bits = bits64(dividend);
    let divisor_bits = bits64(divisor);
    let mut remainder = vec![0; width + 1];
    let mut quotient = vec![0; width];
    let mut extended_divisor = divisor_bits[..width].to_vec();
    extended_divisor.push(0);
    for bit in (0..width).rev() {
        for position in (1..=width).rev() {
            remainder[position] = remainder[position - 1];
        }
        remainder[0] = dividend_bits[bit];
        let inverted: Vec<_> = extended_divisor.iter().copied().map(not_gate).collect();
        let trial = ripple_carry_adder_with_carry(&remainder, &inverted, 1);
        let select = trial.carry_out;
        remainder = remainder
            .iter()
            .zip(trial.sum)
            .map(|(&old, difference)| mux2(old, difference, select))
            .collect();
        quotient[bit] = select;
    }
    let pack = |bits: &[u8]| {
        bits.iter()
            .take(width)
            .enumerate()
            .fold(0, |value, (bit, q)| value | (u64::from(*q) << bit))
    };
    (pack(&quotient), pack(&remainder))
}

fn gate_divrem_signed(dividend: u64, divisor: u64, width: usize) -> (u64, u64) {
    let sign_bit = width - 1;
    let negative_dividend = bits64(dividend)[sign_bit] != 0;
    let negative_divisor = bits64(divisor)[sign_bit] != 0;
    let mask = if width == 64 {
        u64::MAX
    } else {
        (1_u64 << width) - 1
    };
    let narrow_negate = |value| gate_negate(value) & mask;
    let dividend_magnitude = if negative_dividend {
        narrow_negate(dividend)
    } else {
        dividend & mask
    };
    let divisor_magnitude = if negative_divisor {
        narrow_negate(divisor)
    } else {
        divisor & mask
    };
    let (mut quotient, mut remainder) =
        gate_divrem_unsigned(dividend_magnitude, divisor_magnitude, width);
    if negative_dividend ^ negative_divisor {
        quotient = narrow_negate(quotient);
    }
    if negative_dividend {
        remainder = narrow_negate(remainder);
    }
    (quotient, remainder)
}

pub(crate) struct WorkingCpu {
    pub(crate) state: Rv64IState,
}

impl WorkingCpu {
    pub(crate) fn step_inner(&mut self) -> Result<(u32, &'static str), Rv64IError> {
        let pc = self.state.pc;
        if pc & 3 != 0 {
            return Err(Rv64IError::MisalignedFetch { pc });
        }
        self.check_fetch(pc)?;
        let raw = self.read_u32(pc, false)?;
        let decoded = if raw == 0 {
            Decoded {
                operation: Operation::ZeroHalt,
                rd: 0,
                rs1: 0,
                rs2: 0,
                immediate: 0,
            }
        } else {
            decode(raw, pc)?
        };
        self.execute(decoded, pc)?;
        self.state.registers[0] = 0;
        Ok((raw, decoded.operation.mnemonic()))
    }

    fn check_fetch(&self, pc: u64) -> Result<(), Rv64IError> {
        let end = self.state.loaded_origin + self.state.loaded_len as u64;
        if pc < self.state.loaded_origin || pc.checked_add(4).is_none_or(|next| next > end) {
            Err(Rv64IError::FetchOutsideProgram { pc })
        } else {
            Ok(())
        }
    }

    fn execute(&mut self, decoded: Decoded, pc: u64) -> Result<(), Rv64IError> {
        let a = self.state.registers[decoded.rs1];
        let b = self.state.registers[decoded.rs2];
        self.state.pc = gate_add(pc, 4);
        match decoded.operation {
            Operation::ZeroHalt | Operation::Ecall | Operation::Ebreak => {
                self.state.halted = true;
            }
            Operation::Fence | Operation::FenceI => {}
            Operation::Lui => self.write_rd(decoded.rd, decoded.immediate),
            Operation::Auipc => self.write_rd(decoded.rd, gate_add(pc, decoded.immediate)),
            Operation::Jal => {
                self.write_rd(decoded.rd, gate_add(pc, 4));
                self.set_pc(gate_add(pc, decoded.immediate))?;
            }
            Operation::Jalr => {
                self.write_rd(decoded.rd, gate_add(pc, 4));
                let target = gate_boolean(gate_add(a, decoded.immediate), !1, and_gate);
                self.set_pc(target)?;
            }
            Operation::Beq => self.branch_if(gate_equal(a, b, 64), pc, decoded.immediate)?,
            Operation::Bne => self.branch_if(!gate_equal(a, b, 64), pc, decoded.immediate)?,
            Operation::Blt => self.branch_if(gate_signed_less(a, b), pc, decoded.immediate)?,
            Operation::Bge => self.branch_if(!gate_signed_less(a, b), pc, decoded.immediate)?,
            Operation::Bltu => self.branch_if(gate_unsigned_less(a, b), pc, decoded.immediate)?,
            Operation::Bgeu => self.branch_if(!gate_unsigned_less(a, b), pc, decoded.immediate)?,
            Operation::Lb | Operation::Lbu => {
                let value = u64::from(self.read_u8(gate_add(a, decoded.immediate))?);
                self.write_rd(
                    decoded.rd,
                    if decoded.operation == Operation::Lb {
                        gate_sign_extend(value, 8)
                    } else {
                        value
                    },
                );
            }
            Operation::Lh | Operation::Lhu => {
                let value = u64::from(self.read_u16(gate_add(a, decoded.immediate))?);
                self.write_rd(
                    decoded.rd,
                    if decoded.operation == Operation::Lh {
                        gate_sign_extend(value, 16)
                    } else {
                        value
                    },
                );
            }
            Operation::Lw | Operation::Lwu => {
                let value = u64::from(self.read_u32(gate_add(a, decoded.immediate), true)?);
                self.write_rd(
                    decoded.rd,
                    if decoded.operation == Operation::Lw {
                        gate_sign_extend(value, 32)
                    } else {
                        value
                    },
                );
            }
            Operation::Ld => {
                let value = self.read_u64(gate_add(a, decoded.immediate))?;
                self.write_rd(decoded.rd, value);
            }
            Operation::Sb => self.write_u8(gate_add(a, decoded.immediate), b as u8)?,
            Operation::Sh => self.write_u16(gate_add(a, decoded.immediate), b as u16)?,
            Operation::Sw => self.write_u32(gate_add(a, decoded.immediate), b as u32)?,
            Operation::Sd => self.write_u64(gate_add(a, decoded.immediate), b)?,
            Operation::Addi | Operation::Add => {
                let operand = if decoded.operation == Operation::Addi {
                    decoded.immediate
                } else {
                    b
                };
                self.write_rd(decoded.rd, gate_add(a, operand));
            }
            Operation::Sub => self.write_rd(decoded.rd, gate_subtract(a, b)),
            Operation::Slti | Operation::Slt => {
                let operand = if decoded.operation == Operation::Slti {
                    decoded.immediate
                } else {
                    b
                };
                self.write_rd(decoded.rd, u64::from(gate_signed_less(a, operand)));
            }
            Operation::Sltiu | Operation::Sltu => {
                let operand = if decoded.operation == Operation::Sltiu {
                    decoded.immediate
                } else {
                    b
                };
                self.write_rd(decoded.rd, u64::from(gate_unsigned_less(a, operand)));
            }
            Operation::Xori | Operation::Xor => {
                let operand = if decoded.operation == Operation::Xori {
                    decoded.immediate
                } else {
                    b
                };
                self.write_rd(decoded.rd, gate_boolean(a, operand, xor_gate));
            }
            Operation::Ori | Operation::Or => {
                let operand = if decoded.operation == Operation::Ori {
                    decoded.immediate
                } else {
                    b
                };
                self.write_rd(decoded.rd, gate_boolean(a, operand, or_gate));
            }
            Operation::Andi | Operation::And => {
                let operand = if decoded.operation == Operation::Andi {
                    decoded.immediate
                } else {
                    b
                };
                self.write_rd(decoded.rd, gate_boolean(a, operand, and_gate));
            }
            Operation::Slli | Operation::Sll => {
                let amount = if decoded.operation == Operation::Slli {
                    decoded.immediate
                } else {
                    b
                };
                self.write_rd(decoded.rd, gate_barrel(a, ShiftKind::Left, amount, 64));
            }
            Operation::Srli | Operation::Srl => {
                let amount = if decoded.operation == Operation::Srli {
                    decoded.immediate
                } else {
                    b
                };
                self.write_rd(decoded.rd, gate_barrel(a, ShiftKind::Right, amount, 64));
            }
            Operation::Srai | Operation::Sra => {
                let amount = if decoded.operation == Operation::Srai {
                    decoded.immediate
                } else {
                    b
                };
                self.write_rd(
                    decoded.rd,
                    gate_barrel(a, ShiftKind::ArithmeticRight, amount, 64),
                );
            }
            Operation::Addiw | Operation::Addw => {
                let operand = if decoded.operation == Operation::Addiw {
                    decoded.immediate
                } else {
                    b
                };
                self.write_rd(decoded.rd, gate_sign_extend(gate_add(a, operand), 32));
            }
            Operation::Subw => {
                self.write_rd(decoded.rd, gate_sign_extend(gate_subtract(a, b), 32));
            }
            Operation::Slliw | Operation::Sllw => {
                let amount = if decoded.operation == Operation::Slliw {
                    decoded.immediate
                } else {
                    b
                };
                self.write_rd(
                    decoded.rd,
                    gate_sign_extend(gate_barrel(a, ShiftKind::Left, amount, 32), 32),
                );
            }
            Operation::Srliw | Operation::Srlw => {
                let amount = if decoded.operation == Operation::Srliw {
                    decoded.immediate
                } else {
                    b
                };
                self.write_rd(
                    decoded.rd,
                    gate_sign_extend(gate_barrel(a, ShiftKind::Right, amount, 32), 32),
                );
            }
            Operation::Sraiw | Operation::Sraw => {
                let amount = if decoded.operation == Operation::Sraiw {
                    decoded.immediate
                } else {
                    b
                };
                self.write_rd(
                    decoded.rd,
                    gate_sign_extend(gate_barrel(a, ShiftKind::ArithmeticRight, amount, 32), 32),
                );
            }
            Operation::Mul | Operation::Mulhu | Operation::Mulh | Operation::Mulhsu => {
                let product = match decoded.operation {
                    Operation::Mul | Operation::Mulhu => gate_mul_wide(a, b),
                    Operation::Mulh => gate_mul_signed(a, b, true),
                    Operation::Mulhsu => gate_mul_signed(a, b, false),
                    _ => unreachable!(),
                };
                let value = if decoded.operation == Operation::Mul {
                    product as u64
                } else {
                    (product >> 64) as u64
                };
                self.write_rd(decoded.rd, value);
            }
            Operation::Div | Operation::Rem => {
                let (quotient, remainder) = gate_divrem_signed(a, b, 64);
                self.write_rd(
                    decoded.rd,
                    if decoded.operation == Operation::Div {
                        quotient
                    } else {
                        remainder
                    },
                );
            }
            Operation::Divu | Operation::Remu => {
                let (quotient, remainder) = gate_divrem_unsigned(a, b, 64);
                self.write_rd(
                    decoded.rd,
                    if decoded.operation == Operation::Divu {
                        quotient
                    } else {
                        remainder
                    },
                );
            }
            Operation::Mulw => {
                let product = gate_mul_wide(a as u32 as u64, b as u32 as u64) as u64;
                self.write_rd(decoded.rd, gate_sign_extend(product, 32));
            }
            Operation::Divw | Operation::Remw => {
                let (quotient, remainder) = gate_divrem_signed(a, b, 32);
                self.write_rd(
                    decoded.rd,
                    gate_sign_extend(
                        if decoded.operation == Operation::Divw {
                            quotient
                        } else {
                            remainder
                        },
                        32,
                    ),
                );
            }
            Operation::Divuw | Operation::Remuw => {
                let (quotient, remainder) = gate_divrem_unsigned(a, b, 32);
                self.write_rd(
                    decoded.rd,
                    gate_sign_extend(
                        if decoded.operation == Operation::Divuw {
                            quotient
                        } else {
                            remainder
                        },
                        32,
                    ),
                );
            }
        }
        Ok(())
    }

    fn write_rd(&mut self, rd: usize, value: u64) {
        if rd != 0 {
            self.state.registers[rd] = value;
        }
    }

    fn set_pc(&mut self, target: u64) -> Result<(), Rv64IError> {
        if target & 3 != 0 {
            return Err(Rv64IError::MisalignedFetch { pc: target });
        }
        self.state.pc = target;
        Ok(())
    }

    fn branch_if(&mut self, condition: bool, pc: u64, immediate: u64) -> Result<(), Rv64IError> {
        if condition {
            self.set_pc(gate_add(pc, immediate))?;
        }
        Ok(())
    }

    fn checked_range(address: u64, width: usize) -> Result<usize, Rv64IError> {
        if width > 1 && !address.is_multiple_of(width as u64) {
            return Err(Rv64IError::MisalignedData { address, width });
        }
        let end = address
            .checked_add(width as u64)
            .ok_or(Rv64IError::DataOutOfRange { address, width })?;
        if end > MEMORY_SIZE as u64 {
            return Err(Rv64IError::DataOutOfRange { address, width });
        }
        Ok(address as usize)
    }

    fn read_u8(&self, address: u64) -> Result<u8, Rv64IError> {
        Ok(self.state.memory[Self::checked_range(address, 1)?])
    }

    fn read_u16(&self, address: u64) -> Result<u16, Rv64IError> {
        let start = Self::checked_range(address, 2)?;
        Ok(u16::from_le_bytes(
            self.state.memory[start..start + 2]
                .try_into()
                .expect("checked halfword"),
        ))
    }

    fn read_u32(&self, address: u64, data: bool) -> Result<u32, Rv64IError> {
        let start = if data {
            Self::checked_range(address, 4)?
        } else {
            address as usize
        };
        Ok(u32::from_le_bytes(
            self.state.memory[start..start + 4]
                .try_into()
                .expect("checked word"),
        ))
    }

    fn read_u64(&self, address: u64) -> Result<u64, Rv64IError> {
        let start = Self::checked_range(address, 8)?;
        Ok(u64::from_le_bytes(
            self.state.memory[start..start + 8]
                .try_into()
                .expect("checked doubleword"),
        ))
    }

    fn write_u8(&mut self, address: u64, value: u8) -> Result<(), Rv64IError> {
        let start = Self::checked_range(address, 1)?;
        self.state.memory[start] = value;
        Ok(())
    }

    fn write_u16(&mut self, address: u64, value: u16) -> Result<(), Rv64IError> {
        let start = Self::checked_range(address, 2)?;
        self.state.memory[start..start + 2].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn write_u32(&mut self, address: u64, value: u32) -> Result<(), Rv64IError> {
        let start = Self::checked_range(address, 4)?;
        self.state.memory[start..start + 4].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn write_u64(&mut self, address: u64, value: u64) -> Result<(), Rv64IError> {
        let start = Self::checked_range(address, 8)?;
        self.state.memory[start..start + 8].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_arithmetic_shift_multiply_and_divide_edges() {
        assert_eq!(gate_add(u64::MAX, 1), 0);
        assert_eq!(gate_subtract(0, 1), u64::MAX);
        assert!(gate_unsigned_less(1, 2));
        assert!(gate_signed_less(u64::MAX, 0));
        assert_eq!(gate_barrel(1, ShiftKind::Left, 63, 64), 1 << 63);
        assert_eq!(
            gate_barrel(u64::MAX - 7, ShiftKind::ArithmeticRight, 1, 64),
            u64::MAX - 3
        );
        assert_eq!(gate_mul_wide(u64::MAX, 2), (u64::MAX as u128) * 2);
        assert_eq!(gate_divrem_unsigned(43, 5, 64), (8, 3));
        assert_eq!(
            gate_divrem_signed((-43_i64) as u64, 5, 64),
            ((-8_i64) as u64, (-3_i64) as u64)
        );
        assert_eq!(gate_divrem_signed(1 << 63, u64::MAX, 64), (1 << 63, 0));
    }
}
