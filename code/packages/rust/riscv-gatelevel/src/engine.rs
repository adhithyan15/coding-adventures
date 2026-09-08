//! Independent RV32I + machine-CSR decode and gate execution engine.

use arithmetic::adders::ripple_carry_adder_with_carry;
use logic_gates::combinational::mux2;
use logic_gates::gates::{and_gate, not_gate, or_gate, xnor_gate, xor_gate};
use riscv_simulator::csr::{
    CAUSE_ECALL_M_MODE, CSR_MCAUSE, CSR_MEPC, CSR_MSCRATCH, CSR_MSTATUS, CSR_MTVEC, MIE,
};
use riscv_simulator::{Rv32IError, Rv32IState, RV32I_MEMORY_SIZE};

use crate::state::{bits32, word32};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operation {
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
    Lb,
    Lh,
    Lw,
    Lbu,
    Lhu,
    Sb,
    Sh,
    Sw,
    Beq,
    Bne,
    Blt,
    Bge,
    Bltu,
    Bgeu,
    Jal,
    Jalr,
    Lui,
    Auipc,
    Ecall,
    Mret,
    Csrrw,
    Csrrs,
    Csrrc,
}

impl Operation {
    fn mnemonic(self) -> &'static str {
        match self {
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
            Self::Lb => "lb",
            Self::Lh => "lh",
            Self::Lw => "lw",
            Self::Lbu => "lbu",
            Self::Lhu => "lhu",
            Self::Sb => "sb",
            Self::Sh => "sh",
            Self::Sw => "sw",
            Self::Beq => "beq",
            Self::Bne => "bne",
            Self::Blt => "blt",
            Self::Bge => "bge",
            Self::Bltu => "bltu",
            Self::Bgeu => "bgeu",
            Self::Jal => "jal",
            Self::Jalr => "jalr",
            Self::Lui => "lui",
            Self::Auipc => "auipc",
            Self::Ecall => "ecall",
            Self::Mret => "mret",
            Self::Csrrw => "csrrw",
            Self::Csrrs => "csrrs",
            Self::Csrrc => "csrrc",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Decoded {
    operation: Operation,
    rd: usize,
    rs1: usize,
    rs2: usize,
    immediate: u32,
    csr: u32,
}

impl Decoded {
    fn new(operation: Operation) -> Self {
        Self {
            operation,
            rd: 0,
            rs1: 0,
            rs2: 0,
            immediate: 0,
            csr: 0,
        }
    }
}

pub(crate) struct WorkingCpu {
    pub(crate) state: Rv32IState,
}

impl WorkingCpu {
    pub(crate) fn step_inner(&mut self) -> Result<(u32, String), Rv32IError> {
        let pc = self.state.pc;
        if pc & 3 != 0 {
            return Err(Rv32IError::MisalignedFetch { pc });
        }
        self.check_fetch(pc)?;
        let raw = self.read_u32(pc, false)?;
        let decoded = decode(raw, pc)?;
        self.execute(decoded, pc)?;
        self.state.registers[0] = 0;
        Ok((raw, decoded.operation.mnemonic().into()))
    }

    fn check_fetch(&self, pc: u32) -> Result<(), Rv32IError> {
        let start = self.state.loaded_origin as usize;
        let end = start + self.state.loaded_len;
        let address = pc as usize;
        if address < start || address.checked_add(4).is_none_or(|last| last > end) {
            Err(Rv32IError::FetchOutsideProgram { pc })
        } else {
            Ok(())
        }
    }

    fn execute(&mut self, decoded: Decoded, pc: u32) -> Result<(), Rv32IError> {
        let a = self.state.registers[decoded.rs1];
        let b = self.state.registers[decoded.rs2];
        self.state.pc = gate_add(pc, 4);
        match decoded.operation {
            Operation::Addi => self.write_rd(decoded.rd, gate_add(a, decoded.immediate)),
            Operation::Slti => {
                self.write_rd(
                    decoded.rd,
                    u32::from(gate_signed_less(a, decoded.immediate)),
                );
            }
            Operation::Sltiu => {
                self.write_rd(
                    decoded.rd,
                    u32::from(gate_unsigned_less(a, decoded.immediate)),
                );
            }
            Operation::Xori => {
                self.write_rd(decoded.rd, gate_boolean(a, decoded.immediate, xor_gate));
            }
            Operation::Ori => {
                self.write_rd(decoded.rd, gate_boolean(a, decoded.immediate, or_gate));
            }
            Operation::Andi => {
                self.write_rd(decoded.rd, gate_boolean(a, decoded.immediate, and_gate));
            }
            Operation::Slli => {
                self.write_rd(
                    decoded.rd,
                    gate_barrel(a, ShiftKind::Left, decoded.immediate),
                );
            }
            Operation::Srli => {
                self.write_rd(
                    decoded.rd,
                    gate_barrel(a, ShiftKind::Right, decoded.immediate),
                );
            }
            Operation::Srai => {
                self.write_rd(
                    decoded.rd,
                    gate_barrel(a, ShiftKind::ArithmeticRight, decoded.immediate),
                );
            }
            Operation::Add => self.write_rd(decoded.rd, gate_add(a, b)),
            Operation::Sub => self.write_rd(decoded.rd, gate_subtract(a, b)),
            Operation::Sll => self.write_rd(decoded.rd, gate_barrel(a, ShiftKind::Left, b)),
            Operation::Slt => self.write_rd(decoded.rd, u32::from(gate_signed_less(a, b))),
            Operation::Sltu => self.write_rd(decoded.rd, u32::from(gate_unsigned_less(a, b))),
            Operation::Xor => self.write_rd(decoded.rd, gate_boolean(a, b, xor_gate)),
            Operation::Srl => self.write_rd(decoded.rd, gate_barrel(a, ShiftKind::Right, b)),
            Operation::Sra => {
                self.write_rd(decoded.rd, gate_barrel(a, ShiftKind::ArithmeticRight, b));
            }
            Operation::Or => self.write_rd(decoded.rd, gate_boolean(a, b, or_gate)),
            Operation::And => self.write_rd(decoded.rd, gate_boolean(a, b, and_gate)),
            Operation::Lb => {
                let value = self.read_u8(gate_add(a, decoded.immediate))?;
                self.write_rd(decoded.rd, sign_extend(u32::from(value), 8));
            }
            Operation::Lbu => {
                let value = self.read_u8(gate_add(a, decoded.immediate))?;
                self.write_rd(decoded.rd, u32::from(value));
            }
            Operation::Lh => {
                let value = self.read_u16(gate_add(a, decoded.immediate))?;
                self.write_rd(decoded.rd, sign_extend(u32::from(value), 16));
            }
            Operation::Lhu => {
                let value = self.read_u16(gate_add(a, decoded.immediate))?;
                self.write_rd(decoded.rd, u32::from(value));
            }
            Operation::Lw => {
                let value = self.read_u32(gate_add(a, decoded.immediate), true)?;
                self.write_rd(decoded.rd, value);
            }
            Operation::Sb => self.write_u8(gate_add(a, decoded.immediate), b as u8)?,
            Operation::Sh => self.write_u16(gate_add(a, decoded.immediate), b as u16)?,
            Operation::Sw => self.write_u32(gate_add(a, decoded.immediate), b)?,
            Operation::Beq => self.branch_if(gate_equal(a, b, 32), pc, decoded.immediate),
            Operation::Bne => self.branch_if(!gate_equal(a, b, 32), pc, decoded.immediate),
            Operation::Blt => self.branch_if(gate_signed_less(a, b), pc, decoded.immediate),
            Operation::Bge => self.branch_if(!gate_signed_less(a, b), pc, decoded.immediate),
            Operation::Bltu => self.branch_if(gate_unsigned_less(a, b), pc, decoded.immediate),
            Operation::Bgeu => self.branch_if(!gate_unsigned_less(a, b), pc, decoded.immediate),
            Operation::Jal => {
                self.write_rd(decoded.rd, gate_add(pc, 4));
                self.state.pc = gate_add(pc, decoded.immediate);
            }
            Operation::Jalr => {
                let target = gate_boolean(gate_add(a, decoded.immediate), !1, and_gate);
                self.write_rd(decoded.rd, gate_add(pc, 4));
                self.state.pc = target;
            }
            Operation::Lui => self.write_rd(decoded.rd, decoded.immediate),
            Operation::Auipc => self.write_rd(decoded.rd, gate_add(pc, decoded.immediate)),
            Operation::Ecall => self.execute_ecall(pc),
            Operation::Mret => {
                self.state.csr_mstatus = gate_boolean(self.state.csr_mstatus, MIE, or_gate);
                self.state.pc = self.state.csr_mepc;
            }
            Operation::Csrrw | Operation::Csrrs | Operation::Csrrc => {
                self.execute_csr(decoded, pc)?;
            }
        }
        Ok(())
    }

    fn write_rd(&mut self, rd: usize, value: u32) {
        if rd != 0 {
            self.state.registers[rd] = value;
        }
    }

    fn branch_if(&mut self, condition: bool, pc: u32, immediate: u32) {
        if condition {
            self.state.pc = gate_add(pc, immediate);
        }
    }

    fn execute_ecall(&mut self, pc: u32) {
        if gate_equal(self.state.csr_mtvec, 0, 32) {
            self.state.pc = pc;
            self.state.halted = true;
        } else {
            self.state.csr_mepc = pc;
            self.state.csr_mcause = CAUSE_ECALL_M_MODE;
            self.state.csr_mstatus = gate_boolean(self.state.csr_mstatus, gate_not(MIE), and_gate);
            self.state.pc = self.state.csr_mtvec;
        }
    }

    fn execute_csr(&mut self, decoded: Decoded, pc: u32) -> Result<(), Rv32IError> {
        let old = self.read_csr(decoded.csr, pc)?;
        let source = self.state.registers[decoded.rs1];
        let value = match decoded.operation {
            Operation::Csrrw => source,
            Operation::Csrrs => gate_boolean(old, source, or_gate),
            Operation::Csrrc => gate_boolean(old, gate_not(source), and_gate),
            _ => unreachable!("CSR helper receives a CSR operation"),
        };
        self.write_csr(decoded.csr, value, pc)?;
        self.write_rd(decoded.rd, old);
        Ok(())
    }

    fn read_csr(&self, address: u32, pc: u32) -> Result<u32, Rv32IError> {
        match address {
            CSR_MSTATUS => Ok(self.state.csr_mstatus),
            CSR_MTVEC => Ok(self.state.csr_mtvec),
            CSR_MSCRATCH => Ok(self.state.csr_mscratch),
            CSR_MEPC => Ok(self.state.csr_mepc),
            CSR_MCAUSE => Ok(self.state.csr_mcause),
            _ => Err(Rv32IError::UnsupportedCsr { address, pc }),
        }
    }

    fn write_csr(&mut self, address: u32, value: u32, pc: u32) -> Result<(), Rv32IError> {
        match address {
            CSR_MSTATUS => self.state.csr_mstatus = value,
            CSR_MTVEC => self.state.csr_mtvec = value,
            CSR_MSCRATCH => self.state.csr_mscratch = value,
            CSR_MEPC => self.state.csr_mepc = value,
            CSR_MCAUSE => self.state.csr_mcause = value,
            _ => return Err(Rv32IError::UnsupportedCsr { address, pc }),
        }
        Ok(())
    }

    fn checked_range(address: u32, width: usize, alignment: usize) -> Result<usize, Rv32IError> {
        if !(address as usize).is_multiple_of(alignment) {
            return Err(Rv32IError::MisalignedData { address, width });
        }
        let start = address as usize;
        if start
            .checked_add(width)
            .is_none_or(|end| end > RV32I_MEMORY_SIZE)
        {
            return Err(Rv32IError::DataOutOfRange { address, width });
        }
        Ok(start)
    }

    fn read_u8(&self, address: u32) -> Result<u8, Rv32IError> {
        let start = Self::checked_range(address, 1, 1)?;
        Ok(self.state.memory[start])
    }

    fn read_u16(&self, address: u32) -> Result<u16, Rv32IError> {
        let start = Self::checked_range(address, 2, 2)?;
        Ok(u16::from_le_bytes([
            self.state.memory[start],
            self.state.memory[start + 1],
        ]))
    }

    fn read_u32(&self, address: u32, data: bool) -> Result<u32, Rv32IError> {
        let start = if data {
            Self::checked_range(address, 4, 4)?
        } else {
            address as usize
        };
        Ok(u32::from_le_bytes([
            self.state.memory[start],
            self.state.memory[start + 1],
            self.state.memory[start + 2],
            self.state.memory[start + 3],
        ]))
    }

    fn write_u8(&mut self, address: u32, value: u8) -> Result<(), Rv32IError> {
        let start = Self::checked_range(address, 1, 1)?;
        self.state.memory[start] = value;
        Ok(())
    }

    fn write_u16(&mut self, address: u32, value: u16) -> Result<(), Rv32IError> {
        let start = Self::checked_range(address, 2, 2)?;
        self.state.memory[start..start + 2].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn write_u32(&mut self, address: u32, value: u32) -> Result<(), Rv32IError> {
        let start = Self::checked_range(address, 4, 4)?;
        self.state.memory[start..start + 4].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }
}

fn decode(raw: u32, pc: u32) -> Result<Decoded, Rv32IError> {
    let opcode = raw & 0x7f;
    let rd = ((raw >> 7) & 0x1f) as usize;
    let funct3 = (raw >> 12) & 7;
    let rs1 = ((raw >> 15) & 0x1f) as usize;
    let rs2 = ((raw >> 20) & 0x1f) as usize;
    let funct7 = raw >> 25;
    let mut decoded = match opcode {
        0x13 => {
            let operation = match funct3 {
                0 => Operation::Addi,
                2 => Operation::Slti,
                3 => Operation::Sltiu,
                4 => Operation::Xori,
                6 => Operation::Ori,
                7 => Operation::Andi,
                1 if funct7 == 0 => Operation::Slli,
                5 if funct7 == 0 => Operation::Srli,
                5 if funct7 == 0x20 => Operation::Srai,
                _ => return unknown(raw, pc),
            };
            let mut value = Decoded::new(operation);
            value.rd = rd;
            value.rs1 = rs1;
            value.immediate = if matches!(
                operation,
                Operation::Slli | Operation::Srli | Operation::Srai
            ) {
                (raw >> 20) & 31
            } else {
                sign_extend(raw >> 20, 12)
            };
            value
        }
        0x33 => {
            let operation = match (funct3, funct7) {
                (0, 0) => Operation::Add,
                (0, 0x20) => Operation::Sub,
                (1, 0) => Operation::Sll,
                (2, 0) => Operation::Slt,
                (3, 0) => Operation::Sltu,
                (4, 0) => Operation::Xor,
                (5, 0) => Operation::Srl,
                (5, 0x20) => Operation::Sra,
                (6, 0) => Operation::Or,
                (7, 0) => Operation::And,
                _ => return unknown(raw, pc),
            };
            let mut value = Decoded::new(operation);
            value.rd = rd;
            value.rs1 = rs1;
            value.rs2 = rs2;
            value
        }
        0x03 => {
            let operation = match funct3 {
                0 => Operation::Lb,
                1 => Operation::Lh,
                2 => Operation::Lw,
                4 => Operation::Lbu,
                5 => Operation::Lhu,
                _ => return unknown(raw, pc),
            };
            let mut value = Decoded::new(operation);
            value.rd = rd;
            value.rs1 = rs1;
            value.immediate = sign_extend(raw >> 20, 12);
            value
        }
        0x23 => {
            let operation = match funct3 {
                0 => Operation::Sb,
                1 => Operation::Sh,
                2 => Operation::Sw,
                _ => return unknown(raw, pc),
            };
            let mut value = Decoded::new(operation);
            value.rs1 = rs1;
            value.rs2 = rs2;
            value.immediate = sign_extend(((raw >> 25) << 5) | ((raw >> 7) & 0x1f), 12);
            value
        }
        0x63 => {
            let operation = match funct3 {
                0 => Operation::Beq,
                1 => Operation::Bne,
                4 => Operation::Blt,
                5 => Operation::Bge,
                6 => Operation::Bltu,
                7 => Operation::Bgeu,
                _ => return unknown(raw, pc),
            };
            let mut value = Decoded::new(operation);
            value.rs1 = rs1;
            value.rs2 = rs2;
            value.immediate = sign_extend(
                ((raw >> 31) << 12)
                    | (((raw >> 7) & 1) << 11)
                    | (((raw >> 25) & 0x3f) << 5)
                    | (((raw >> 8) & 0xf) << 1),
                13,
            );
            value
        }
        0x6f => {
            let mut value = Decoded::new(Operation::Jal);
            value.rd = rd;
            value.immediate = sign_extend(
                ((raw >> 31) << 20)
                    | (((raw >> 12) & 0xff) << 12)
                    | (((raw >> 20) & 1) << 11)
                    | (((raw >> 21) & 0x3ff) << 1),
                21,
            );
            value
        }
        0x67 if funct3 == 0 => {
            let mut value = Decoded::new(Operation::Jalr);
            value.rd = rd;
            value.rs1 = rs1;
            value.immediate = sign_extend(raw >> 20, 12);
            value
        }
        0x37 => {
            let mut value = Decoded::new(Operation::Lui);
            value.rd = rd;
            value.immediate = raw & 0xffff_f000;
            value
        }
        0x17 => {
            let mut value = Decoded::new(Operation::Auipc);
            value.rd = rd;
            value.immediate = raw & 0xffff_f000;
            value
        }
        0x73 if raw == 0x0000_0073 => Decoded::new(Operation::Ecall),
        0x73 if raw == 0x3020_0073 => Decoded::new(Operation::Mret),
        0x73 if matches!(funct3, 1..=3) => {
            let operation = match funct3 {
                1 => Operation::Csrrw,
                2 => Operation::Csrrs,
                3 => Operation::Csrrc,
                _ => unreachable!(),
            };
            let mut value = Decoded::new(operation);
            value.rd = rd;
            value.rs1 = rs1;
            value.csr = (raw >> 20) & 0xfff;
            value
        }
        _ => return unknown(raw, pc),
    };
    decoded.rs1 &= 31;
    decoded.rs2 &= 31;
    Ok(decoded)
}

fn unknown<T>(raw: u32, pc: u32) -> Result<T, Rv32IError> {
    Err(Rv32IError::UnknownInstruction { raw, pc })
}

fn sign_extend(value: u32, width: u32) -> u32 {
    let shift = 32 - width;
    ((value << shift) as i32 >> shift) as u32
}

fn gate_equal(left: u32, right: u32, width: usize) -> bool {
    let left_bits = bits32(left);
    let right_bits = bits32(right);
    left_bits[..width]
        .iter()
        .zip(&right_bits[..width])
        .map(|(&a, &b)| xnor_gate(a, b))
        .fold(1, and_gate)
        != 0
}

fn gate_add_carry(left: u32, right: u32, carry: bool) -> (u32, bool, bool) {
    let left_bits = bits32(left);
    let right_bits = bits32(right);
    let output = ripple_carry_adder_with_carry(&left_bits, &right_bits, u8::from(carry));
    let sum: [u8; 32] = output.sum.try_into().expect("32-bit ripple result");
    let overflow = and_gate(
        not_gate(xor_gate(left_bits[31], right_bits[31])),
        xor_gate(left_bits[31], sum[31]),
    ) != 0;
    (word32(&sum), output.carry_out != 0, overflow)
}

fn gate_add(left: u32, right: u32) -> u32 {
    gate_add_carry(left, right, false).0
}

fn gate_subtract_full(left: u32, right: u32) -> (u32, bool, bool) {
    let left_bits = bits32(left);
    let right_bits = bits32(right);
    let inverted: [u8; 32] = std::array::from_fn(|bit| not_gate(right_bits[bit]));
    let output = ripple_carry_adder_with_carry(&left_bits, &inverted, 1);
    let difference: [u8; 32] = output.sum.try_into().expect("32-bit ripple result");
    let overflow = and_gate(
        xor_gate(left_bits[31], right_bits[31]),
        xor_gate(left_bits[31], difference[31]),
    ) != 0;
    (word32(&difference), output.carry_out != 0, overflow)
}

fn gate_subtract(left: u32, right: u32) -> u32 {
    gate_subtract_full(left, right).0
}

fn gate_unsigned_less(left: u32, right: u32) -> bool {
    !gate_subtract_full(left, right).1
}

fn gate_signed_less(left: u32, right: u32) -> bool {
    let (difference, _, overflow) = gate_subtract_full(left, right);
    (bits32(difference)[31] != 0) ^ overflow
}

fn gate_boolean(left: u32, right: u32, gate: fn(u8, u8) -> u8) -> u32 {
    let left_bits = bits32(left);
    let right_bits = bits32(right);
    word32(&std::array::from_fn(|bit| {
        gate(left_bits[bit], right_bits[bit])
    }))
}

fn gate_not(value: u32) -> u32 {
    let input = bits32(value);
    word32(&std::array::from_fn(|bit| not_gate(input[bit])))
}

#[derive(Debug, Clone, Copy)]
enum ShiftKind {
    Left,
    Right,
    ArithmeticRight,
}

fn gate_barrel(value: u32, kind: ShiftKind, amount: u32) -> u32 {
    let selects = bits32(amount);
    let mut current = bits32(value);
    for (stage, select) in selects[..5].iter().copied().enumerate() {
        let distance = 1usize << stage;
        let shifted: [u8; 32] = std::array::from_fn(|bit| match kind {
            ShiftKind::Left => bit
                .checked_sub(distance)
                .map_or(0, |source| current[source]),
            ShiftKind::Right => current.get(bit + distance).copied().unwrap_or(0),
            ShiftKind::ArithmeticRight => {
                current.get(bit + distance).copied().unwrap_or(current[31])
            }
        });
        current = std::array::from_fn(|bit| mux2(current[bit], shifted[bit], select));
    }
    word32(&current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_network_edges() {
        assert_eq!(gate_add(u32::MAX, 1), 0);
        assert_eq!(gate_subtract(0, 1), u32::MAX);
        assert!(gate_unsigned_less(0, u32::MAX));
        assert!(gate_signed_less(0x8000_0000, 0));
        assert_eq!(gate_boolean(0xf0f0, 0x0ff0, and_gate), 0x00f0);
        assert_eq!(gate_not(0), u32::MAX);
        assert_eq!(gate_barrel(1, ShiftKind::Left, 31), 0x8000_0000);
        assert_eq!(
            gate_barrel(0x8000_0000, ShiftKind::ArithmeticRight, 31),
            u32::MAX
        );
    }

    #[test]
    fn decoder_rejects_non_rv32i_encodings() {
        assert!(matches!(
            decode(0x0200_0033, 0x100),
            Err(Rv32IError::UnknownInstruction { .. })
        ));
        assert!(matches!(
            decode(0xffff_ffff, 0x100),
            Err(Rv32IError::UnknownInstruction { .. })
        ));
    }
}
