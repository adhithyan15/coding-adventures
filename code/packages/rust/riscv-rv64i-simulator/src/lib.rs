//! Checked functional RISC-V RV64I plus M simulator for Spec 07y.

use std::fmt;

pub mod encoding;

/// Exact architectural memory size.
pub const MEMORY_SIZE: usize = 65_536;
/// Number of integer registers.
pub const REGISTER_COUNT: usize = 32;
/// Reset stack pointer convention inherited from the Python oracle.
pub const RESET_STACK_POINTER: u64 = 0xfff8;

/// Complete architectural and installation state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rv64IState {
    pub registers: [u64; REGISTER_COUNT],
    pub pc: u64,
    pub memory: Vec<u8>,
    pub halted: bool,
    pub loaded_origin: u64,
    pub loaded_len: usize,
}

/// Typed lifecycle, fetch, decode, and data failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rv64IError {
    InvalidState(String),
    InvalidRegister { index: usize },
    MisalignedProgram { origin: u64 },
    ProgramOutOfRange { origin: u64, length: usize },
    MisalignedFetch { pc: u64 },
    FetchOutsideProgram { pc: u64 },
    MisalignedData { address: u64, width: usize },
    DataOutOfRange { address: u64, width: usize },
    UnknownInstruction { raw: u32, pc: u64 },
    Halted,
    StepLimitExceeded { max_steps: usize },
}

impl fmt::Display for Rv64IError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for Rv64IError {}

/// One complete instruction-boundary transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rv64IStepTrace {
    pub pc_before: u64,
    pub pc_after: u64,
    pub raw: u32,
    pub mnemonic: &'static str,
    pub state_before: Rv64IState,
    pub state_after: Rv64IState,
}

/// Successful bounded execution result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rv64IExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub traces: Vec<Rv64IStepTrace>,
    pub final_state: Rv64IState,
}

#[derive(Debug, Clone, Copy)]
enum Operation {
    Halt(&'static str),
    Lui,
    Auipc,
    Jal,
    Jalr,
    Branch(Branch),
    Load(Load),
    Store(Store),
    AluImm(Alu),
    AluReg(Alu),
    AluImmWord(WordAlu),
    AluRegWord(WordAlu),
    Mul(Mul),
    MulWord(MulWord),
    Fence(&'static str),
}

#[derive(Debug, Clone, Copy)]
enum Branch {
    Eq,
    Ne,
    Lt,
    Ge,
    Ltu,
    Geu,
}

#[derive(Debug, Clone, Copy)]
enum Load {
    Byte,
    Half,
    Word,
    Double,
    ByteU,
    HalfU,
    WordU,
}

#[derive(Debug, Clone, Copy)]
enum Store {
    Byte,
    Half,
    Word,
    Double,
}

#[derive(Debug, Clone, Copy)]
enum Alu {
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
}

#[derive(Debug, Clone, Copy)]
enum WordAlu {
    Add,
    Sub,
    Sll,
    Srl,
    Sra,
}

#[derive(Debug, Clone, Copy)]
enum Mul {
    Low,
    Mulh,
    Mulhsu,
    Mulhu,
    Div,
    Divu,
    Rem,
    Remu,
}

#[derive(Debug, Clone, Copy)]
enum MulWord {
    Mul,
    Div,
    Divu,
    Rem,
    Remu,
}

#[derive(Debug, Clone, Copy)]
struct Decoded {
    operation: Operation,
    rd: usize,
    rs1: usize,
    rs2: usize,
    immediate: i64,
}

fn sign_extend(value: u32, bits: u32) -> i64 {
    let shift = 64 - bits;
    ((u64::from(value) << shift) as i64) >> shift
}

fn i_imm(raw: u32) -> i64 {
    sign_extend(raw >> 20, 12)
}

fn s_imm(raw: u32) -> i64 {
    sign_extend(((raw >> 25) << 5) | ((raw >> 7) & 0x1f), 12)
}

fn b_imm(raw: u32) -> i64 {
    let value = ((raw >> 31) << 12)
        | (((raw >> 7) & 1) << 11)
        | (((raw >> 25) & 0x3f) << 5)
        | (((raw >> 8) & 0xf) << 1);
    sign_extend(value, 13)
}

fn u_imm(raw: u32) -> i64 {
    i64::from(raw as i32 & !0xfff)
}

fn j_imm(raw: u32) -> i64 {
    let value = ((raw >> 31) << 20)
        | (((raw >> 12) & 0xff) << 12)
        | (((raw >> 20) & 1) << 11)
        | (((raw >> 21) & 0x3ff) << 1);
    sign_extend(value, 21)
}

fn decode(raw: u32, pc: u64) -> Result<Decoded, Rv64IError> {
    let opcode = raw & 0x7f;
    let rd = ((raw >> 7) & 0x1f) as usize;
    let funct3 = (raw >> 12) & 7;
    let rs1 = ((raw >> 15) & 0x1f) as usize;
    let rs2 = ((raw >> 20) & 0x1f) as usize;
    let funct7 = raw >> 25;
    let unknown = || Rv64IError::UnknownInstruction { raw, pc };
    let (operation, immediate) = match opcode {
        0x37 => (Operation::Lui, u_imm(raw)),
        0x17 => (Operation::Auipc, u_imm(raw)),
        0x6f => (Operation::Jal, j_imm(raw)),
        0x67 if funct3 == 0 => (Operation::Jalr, i_imm(raw)),
        0x63 => {
            let branch = match funct3 {
                0 => Branch::Eq,
                1 => Branch::Ne,
                4 => Branch::Lt,
                5 => Branch::Ge,
                6 => Branch::Ltu,
                7 => Branch::Geu,
                _ => return Err(unknown()),
            };
            (Operation::Branch(branch), b_imm(raw))
        }
        0x03 => {
            let load = match funct3 {
                0 => Load::Byte,
                1 => Load::Half,
                2 => Load::Word,
                3 => Load::Double,
                4 => Load::ByteU,
                5 => Load::HalfU,
                6 => Load::WordU,
                _ => return Err(unknown()),
            };
            (Operation::Load(load), i_imm(raw))
        }
        0x23 => {
            let store = match funct3 {
                0 => Store::Byte,
                1 => Store::Half,
                2 => Store::Word,
                3 => Store::Double,
                _ => return Err(unknown()),
            };
            (Operation::Store(store), s_imm(raw))
        }
        0x13 => {
            let alu = match funct3 {
                0 => Alu::Add,
                2 => Alu::Slt,
                3 => Alu::Sltu,
                4 => Alu::Xor,
                6 => Alu::Or,
                7 => Alu::And,
                1 if raw >> 26 == 0 => Alu::Sll,
                5 if raw >> 26 == 0 => Alu::Srl,
                5 if raw >> 26 == 0x10 => Alu::Sra,
                _ => return Err(unknown()),
            };
            (Operation::AluImm(alu), i_imm(raw))
        }
        0x33 => {
            if funct7 == 1 {
                let mul = match funct3 {
                    0 => Mul::Low,
                    1 => Mul::Mulh,
                    2 => Mul::Mulhsu,
                    3 => Mul::Mulhu,
                    4 => Mul::Div,
                    5 => Mul::Divu,
                    6 => Mul::Rem,
                    7 => Mul::Remu,
                    _ => unreachable!(),
                };
                (Operation::Mul(mul), 0)
            } else {
                let alu = match (funct7, funct3) {
                    (0, 0) => Alu::Add,
                    (0x20, 0) => Alu::Sub,
                    (0, 1) => Alu::Sll,
                    (0, 2) => Alu::Slt,
                    (0, 3) => Alu::Sltu,
                    (0, 4) => Alu::Xor,
                    (0, 5) => Alu::Srl,
                    (0x20, 5) => Alu::Sra,
                    (0, 6) => Alu::Or,
                    (0, 7) => Alu::And,
                    _ => return Err(unknown()),
                };
                (Operation::AluReg(alu), 0)
            }
        }
        0x1b => {
            let alu = match (funct7, funct3) {
                (_, 0) => WordAlu::Add,
                (0, 1) => WordAlu::Sll,
                (0, 5) => WordAlu::Srl,
                (0x20, 5) => WordAlu::Sra,
                _ => return Err(unknown()),
            };
            (Operation::AluImmWord(alu), i_imm(raw))
        }
        0x3b => {
            if funct7 == 1 {
                let mul = match funct3 {
                    0 => MulWord::Mul,
                    4 => MulWord::Div,
                    5 => MulWord::Divu,
                    6 => MulWord::Rem,
                    7 => MulWord::Remu,
                    _ => return Err(unknown()),
                };
                (Operation::MulWord(mul), 0)
            } else {
                let alu = match (funct7, funct3) {
                    (0, 0) => WordAlu::Add,
                    (0x20, 0) => WordAlu::Sub,
                    (0, 1) => WordAlu::Sll,
                    (0, 5) => WordAlu::Srl,
                    (0x20, 5) => WordAlu::Sra,
                    _ => return Err(unknown()),
                };
                (Operation::AluRegWord(alu), 0)
            }
        }
        0x0f if funct3 == 0 && rs1 == 0 && rd == 0 => (Operation::Fence("fence"), 0),
        0x0f if funct3 == 1 && raw >> 20 == 0 && rs1 == 0 && rd == 0 => {
            (Operation::Fence("fence.i"), 0)
        }
        0x73 if raw == 0x0000_0073 => (Operation::Halt("ecall"), 0),
        0x73 if raw == 0x0010_0073 => (Operation::Halt("ebreak"), 0),
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

fn word_sign_extend(value: u32) -> u64 {
    (value as i32 as i64) as u64
}

fn operation_name(operation: Operation) -> &'static str {
    match operation {
        Operation::Halt(name) | Operation::Fence(name) => name,
        Operation::Lui => "lui",
        Operation::Auipc => "auipc",
        Operation::Jal => "jal",
        Operation::Jalr => "jalr",
        Operation::Branch(branch) => match branch {
            Branch::Eq => "beq",
            Branch::Ne => "bne",
            Branch::Lt => "blt",
            Branch::Ge => "bge",
            Branch::Ltu => "bltu",
            Branch::Geu => "bgeu",
        },
        Operation::Load(load) => match load {
            Load::Byte => "lb",
            Load::Half => "lh",
            Load::Word => "lw",
            Load::Double => "ld",
            Load::ByteU => "lbu",
            Load::HalfU => "lhu",
            Load::WordU => "lwu",
        },
        Operation::Store(store) => match store {
            Store::Byte => "sb",
            Store::Half => "sh",
            Store::Word => "sw",
            Store::Double => "sd",
        },
        Operation::AluImm(alu) => match alu {
            Alu::Add => "addi",
            Alu::Sll => "slli",
            Alu::Slt => "slti",
            Alu::Sltu => "sltiu",
            Alu::Xor => "xori",
            Alu::Srl => "srli",
            Alu::Sra => "srai",
            Alu::Or => "ori",
            Alu::And => "andi",
            Alu::Sub => unreachable!(),
        },
        Operation::AluReg(alu) => match alu {
            Alu::Add => "add",
            Alu::Sub => "sub",
            Alu::Sll => "sll",
            Alu::Slt => "slt",
            Alu::Sltu => "sltu",
            Alu::Xor => "xor",
            Alu::Srl => "srl",
            Alu::Sra => "sra",
            Alu::Or => "or",
            Alu::And => "and",
        },
        Operation::AluImmWord(alu) => match alu {
            WordAlu::Add => "addiw",
            WordAlu::Sll => "slliw",
            WordAlu::Srl => "srliw",
            WordAlu::Sra => "sraiw",
            WordAlu::Sub => unreachable!(),
        },
        Operation::AluRegWord(alu) => match alu {
            WordAlu::Add => "addw",
            WordAlu::Sub => "subw",
            WordAlu::Sll => "sllw",
            WordAlu::Srl => "srlw",
            WordAlu::Sra => "sraw",
        },
        Operation::Mul(mul) => match mul {
            Mul::Low => "mul",
            Mul::Mulh => "mulh",
            Mul::Mulhsu => "mulhsu",
            Mul::Mulhu => "mulhu",
            Mul::Div => "div",
            Mul::Divu => "divu",
            Mul::Rem => "rem",
            Mul::Remu => "remu",
        },
        Operation::MulWord(mul) => match mul {
            MulWord::Mul => "mulw",
            MulWord::Div => "divw",
            MulWord::Divu => "divuw",
            MulWord::Rem => "remw",
            MulWord::Remu => "remuw",
        },
    }
}

/// Exact checked RV64I+M functional machine.
#[derive(Debug, Clone)]
pub struct Rv64ISimulator {
    state: Rv64IState,
}

impl Default for Rv64ISimulator {
    fn default() -> Self {
        Self::new()
    }
}

impl Rv64ISimulator {
    #[must_use]
    pub fn new() -> Self {
        let mut registers = [0; REGISTER_COUNT];
        registers[2] = RESET_STACK_POINTER;
        Self {
            state: Rv64IState {
                registers,
                pc: 0,
                memory: vec![0; MEMORY_SIZE],
                halted: false,
                loaded_origin: 0,
                loaded_len: 0,
            },
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    #[must_use]
    pub fn get_state(&self) -> Rv64IState {
        self.state.clone()
    }

    pub fn restore(&mut self, snapshot: &Rv64IState) -> Result<(), Rv64IError> {
        Self::validate_state(snapshot)?;
        self.state = snapshot.clone();
        Ok(())
    }

    fn validate_state(state: &Rv64IState) -> Result<(), Rv64IError> {
        if state.memory.len() != MEMORY_SIZE {
            return Err(Rv64IError::InvalidState(
                "memory must contain exactly 65,536 bytes".into(),
            ));
        }
        if state.registers[0] != 0 {
            return Err(Rv64IError::InvalidState("x0 must be zero".into()));
        }
        if state.pc & 3 != 0 {
            return Err(Rv64IError::InvalidState("PC must be word aligned".into()));
        }
        let end = state
            .loaded_origin
            .checked_add(state.loaded_len as u64)
            .ok_or_else(|| Rv64IError::InvalidState("installed range overflow".into()))?;
        if state.loaded_origin & 3 != 0 || end > MEMORY_SIZE as u64 {
            return Err(Rv64IError::InvalidState(
                "installed range is invalid".into(),
            ));
        }
        Ok(())
    }

    pub fn load_checked(&mut self, program: &[u8]) -> Result<(), Rv64IError> {
        self.load_at_checked(program, 0)
    }

    pub fn load_at_checked(&mut self, program: &[u8], origin: u64) -> Result<(), Rv64IError> {
        if origin & 3 != 0 {
            return Err(Rv64IError::MisalignedProgram { origin });
        }
        let end =
            origin
                .checked_add(program.len() as u64)
                .ok_or(Rv64IError::ProgramOutOfRange {
                    origin,
                    length: program.len(),
                })?;
        if end > MEMORY_SIZE as u64 {
            return Err(Rv64IError::ProgramOutOfRange {
                origin,
                length: program.len(),
            });
        }
        self.reset();
        self.state.memory[origin as usize..end as usize].copy_from_slice(program);
        self.state.pc = origin;
        self.state.loaded_origin = origin;
        self.state.loaded_len = program.len();
        Ok(())
    }

    pub fn read_register(&self, index: usize) -> Result<u64, Rv64IError> {
        self.state
            .registers
            .get(index)
            .copied()
            .ok_or(Rv64IError::InvalidRegister { index })
    }

    pub fn write_register(&mut self, index: usize, value: u64) -> Result<(), Rv64IError> {
        let register = self
            .state
            .registers
            .get_mut(index)
            .ok_or(Rv64IError::InvalidRegister { index })?;
        if index != 0 {
            *register = value;
        }
        Ok(())
    }

    pub fn read_byte(&self, address: u64) -> Result<u8, Rv64IError> {
        self.state
            .memory
            .get(address as usize)
            .copied()
            .ok_or(Rv64IError::DataOutOfRange { address, width: 1 })
    }

    pub fn write_byte(&mut self, address: u64, value: u8) -> Result<(), Rv64IError> {
        let byte = self
            .state
            .memory
            .get_mut(address as usize)
            .ok_or(Rv64IError::DataOutOfRange { address, width: 1 })?;
        *byte = value;
        Ok(())
    }

    fn fetch(&self) -> Result<u32, Rv64IError> {
        let pc = self.state.pc;
        if pc & 3 != 0 {
            return Err(Rv64IError::MisalignedFetch { pc });
        }
        let installed_end = self.state.loaded_origin + self.state.loaded_len as u64;
        if pc < self.state.loaded_origin || pc.checked_add(4).is_none_or(|end| end > installed_end)
        {
            return Err(Rv64IError::FetchOutsideProgram { pc });
        }
        let start = pc as usize;
        Ok(u32::from_le_bytes(
            self.state.memory[start..start + 4]
                .try_into()
                .expect("checked fetch"),
        ))
    }

    fn checked_address(address: u64, width: usize) -> Result<usize, Rv64IError> {
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

    fn load_value(&self, address: u64, width: usize) -> Result<u64, Rv64IError> {
        let start = Self::checked_address(address, width)?;
        let mut bytes = [0u8; 8];
        bytes[..width].copy_from_slice(&self.state.memory[start..start + width]);
        Ok(u64::from_le_bytes(bytes))
    }

    fn store_value(&mut self, address: u64, width: usize, value: u64) -> Result<(), Rv64IError> {
        let start = Self::checked_address(address, width)?;
        self.state.memory[start..start + width].copy_from_slice(&value.to_le_bytes()[..width]);
        Ok(())
    }

    fn set_pc(&mut self, target: u64) -> Result<(), Rv64IError> {
        if target & 3 != 0 {
            return Err(Rv64IError::MisalignedFetch { pc: target });
        }
        self.state.pc = target;
        Ok(())
    }

    fn write_result(&mut self, rd: usize, value: u64) {
        if rd != 0 {
            self.state.registers[rd] = value;
        }
    }

    fn execute_decoded(&mut self, decoded: Decoded, pc: u64) -> Result<(), Rv64IError> {
        let a = self.state.registers[decoded.rs1];
        let b = self.state.registers[decoded.rs2];
        let immediate = decoded.immediate as u64;
        match decoded.operation {
            Operation::Halt(_) => self.state.halted = true,
            Operation::Fence(_) => {}
            Operation::Lui => self.write_result(decoded.rd, immediate),
            Operation::Auipc => self.write_result(decoded.rd, pc.wrapping_add(immediate)),
            Operation::Jal => {
                self.write_result(decoded.rd, pc.wrapping_add(4));
                self.set_pc(pc.wrapping_add(immediate))?;
            }
            Operation::Jalr => {
                self.write_result(decoded.rd, pc.wrapping_add(4));
                self.set_pc(a.wrapping_add(immediate) & !1)?;
            }
            Operation::Branch(branch) => {
                let taken = match branch {
                    Branch::Eq => a == b,
                    Branch::Ne => a != b,
                    Branch::Lt => (a as i64) < (b as i64),
                    Branch::Ge => (a as i64) >= (b as i64),
                    Branch::Ltu => a < b,
                    Branch::Geu => a >= b,
                };
                if taken {
                    self.set_pc(pc.wrapping_add(immediate))?;
                }
            }
            Operation::Load(load) => {
                let address = a.wrapping_add(immediate);
                let (width, signed) = match load {
                    Load::Byte => (1, true),
                    Load::Half => (2, true),
                    Load::Word => (4, true),
                    Load::Double => (8, false),
                    Load::ByteU => (1, false),
                    Load::HalfU => (2, false),
                    Load::WordU => (4, false),
                };
                let value = self.load_value(address, width)?;
                let value = if signed {
                    match width {
                        1 => (value as u8 as i8 as i64) as u64,
                        2 => (value as u16 as i16 as i64) as u64,
                        4 => (value as u32 as i32 as i64) as u64,
                        _ => unreachable!(),
                    }
                } else {
                    value
                };
                self.write_result(decoded.rd, value);
            }
            Operation::Store(store) => {
                let width = match store {
                    Store::Byte => 1,
                    Store::Half => 2,
                    Store::Word => 4,
                    Store::Double => 8,
                };
                self.store_value(a.wrapping_add(immediate), width, b)?;
            }
            Operation::AluImm(alu) | Operation::AluReg(alu) => {
                let operand = if matches!(decoded.operation, Operation::AluImm(_)) {
                    immediate
                } else {
                    b
                };
                let value = match alu {
                    Alu::Add => a.wrapping_add(operand),
                    Alu::Sub => a.wrapping_sub(operand),
                    Alu::Sll => a.wrapping_shl((operand & 63) as u32),
                    Alu::Slt => u64::from((a as i64) < (operand as i64)),
                    Alu::Sltu => u64::from(a < operand),
                    Alu::Xor => a ^ operand,
                    Alu::Srl => a.wrapping_shr((operand & 63) as u32),
                    Alu::Sra => ((a as i64) >> (operand & 63)) as u64,
                    Alu::Or => a | operand,
                    Alu::And => a & operand,
                };
                self.write_result(decoded.rd, value);
            }
            Operation::AluImmWord(alu) | Operation::AluRegWord(alu) => {
                let operand = if matches!(decoded.operation, Operation::AluImmWord(_)) {
                    immediate
                } else {
                    b
                };
                let a32 = a as u32;
                let b32 = operand as u32;
                let value = match alu {
                    WordAlu::Add => a32.wrapping_add(b32),
                    WordAlu::Sub => a32.wrapping_sub(b32),
                    WordAlu::Sll => a32.wrapping_shl(b32 & 31),
                    WordAlu::Srl => a32.wrapping_shr(b32 & 31),
                    WordAlu::Sra => ((a32 as i32) >> (b32 & 31)) as u32,
                };
                self.write_result(decoded.rd, word_sign_extend(value));
            }
            Operation::Mul(mul) => {
                let value = match mul {
                    Mul::Low => a.wrapping_mul(b),
                    Mul::Mulh => (((a as i64 as i128) * (b as i64 as i128)) >> 64) as u64,
                    Mul::Mulhsu => (((a as i64 as i128) * (b as u128 as i128)) >> 64) as u64,
                    Mul::Mulhu => (((a as u128) * (b as u128)) >> 64) as u64,
                    Mul::Div if b == 0 => u64::MAX,
                    Mul::Div if a == 1 << 63 && b == u64::MAX => a,
                    Mul::Div => (a as i64 / b as i64) as u64,
                    Mul::Divu if b == 0 => u64::MAX,
                    Mul::Divu => a / b,
                    Mul::Rem if b == 0 => a,
                    Mul::Rem if a == 1 << 63 && b == u64::MAX => 0,
                    Mul::Rem => (a as i64 % b as i64) as u64,
                    Mul::Remu if b == 0 => a,
                    Mul::Remu => a % b,
                };
                self.write_result(decoded.rd, value);
            }
            Operation::MulWord(mul) => {
                let a32 = a as u32;
                let b32 = b as u32;
                let value = match mul {
                    MulWord::Mul => a32.wrapping_mul(b32),
                    MulWord::Div if b32 == 0 => u32::MAX,
                    MulWord::Div if a32 == 1 << 31 && b32 == u32::MAX => a32,
                    MulWord::Div => (a32 as i32 / b32 as i32) as u32,
                    MulWord::Divu if b32 == 0 => u32::MAX,
                    MulWord::Divu => a32 / b32,
                    MulWord::Rem if b32 == 0 => a32,
                    MulWord::Rem if a32 == 1 << 31 && b32 == u32::MAX => 0,
                    MulWord::Rem => (a32 as i32 % b32 as i32) as u32,
                    MulWord::Remu if b32 == 0 => a32,
                    MulWord::Remu => a32 % b32,
                };
                self.write_result(decoded.rd, word_sign_extend(value));
            }
        }
        self.state.registers[0] = 0;
        Ok(())
    }

    pub fn step_checked(&mut self) -> Result<Rv64IStepTrace, Rv64IError> {
        if self.state.halted {
            return Err(Rv64IError::Halted);
        }
        let before = self.get_state();
        let pc = before.pc;
        let raw = self.fetch()?;
        let decoded = if raw == 0 {
            Decoded {
                operation: Operation::Halt("zero-halt"),
                rd: 0,
                rs1: 0,
                rs2: 0,
                immediate: 0,
            }
        } else {
            decode(raw, pc)?
        };
        self.state.pc = pc.wrapping_add(4);
        if let Err(error) = self.execute_decoded(decoded, pc) {
            self.state = before;
            return Err(error);
        }
        Ok(Rv64IStepTrace {
            pc_before: pc,
            pc_after: self.state.pc,
            raw,
            mnemonic: operation_name(decoded.operation),
            state_before: before,
            state_after: self.get_state(),
        })
    }

    pub fn run_checked(&mut self, max_steps: usize) -> Result<Rv64IExecutionResult, Rv64IError> {
        let before = self.get_state();
        let mut traces = Vec::new();
        if self.state.halted {
            return Ok(Rv64IExecutionResult {
                halted: true,
                steps: 0,
                traces,
                final_state: before,
            });
        }
        for _ in 0..max_steps {
            match self.step_checked() {
                Ok(trace) => traces.push(trace),
                Err(error) => {
                    self.state = before;
                    return Err(error);
                }
            }
            if self.state.halted {
                return Ok(Rv64IExecutionResult {
                    halted: true,
                    steps: traces.len(),
                    traces,
                    final_state: self.get_state(),
                });
            }
        }
        self.state = before;
        Err(Rv64IError::StepLimitExceeded { max_steps })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_i(opcode: u32, funct3: u32, rd: u32, rs1: u32, immediate: i32) -> u32 {
        ((immediate as u32 & 0xfff) << 20)
            | ((rs1 & 31) << 15)
            | ((funct3 & 7) << 12)
            | ((rd & 31) << 7)
            | opcode
    }

    #[test]
    fn reset_and_checked_lifecycle_are_exact() {
        let mut cpu = Rv64ISimulator::new();
        assert_eq!(cpu.get_state().memory.len(), MEMORY_SIZE);
        assert_eq!(cpu.read_register(2), Ok(RESET_STACK_POINTER));
        cpu.write_register(1, 99).unwrap();
        cpu.write_register(0, 99).unwrap();
        assert_eq!(cpu.read_register(0), Ok(0));
        assert_eq!(
            cpu.read_register(32),
            Err(Rv64IError::InvalidRegister { index: 32 })
        );
    }

    #[test]
    fn addi_then_distinct_halt_trace() {
        let words = [encode_i(0x13, 0, 10, 0, 42), 0x0000_0073];
        let program: Vec<u8> = words.into_iter().flat_map(u32::to_le_bytes).collect();
        let mut cpu = Rv64ISimulator::new();
        cpu.load_checked(&program).unwrap();
        let result = cpu.run_checked(3).unwrap();
        assert_eq!(result.final_state.registers[10], 42);
        assert_eq!(result.traces[0].mnemonic, "addi");
        assert_eq!(result.traces[1].mnemonic, "ecall");
    }

    #[test]
    fn decode_and_data_faults_are_atomic() {
        let mut cpu = Rv64ISimulator::new();
        cpu.load_checked(&u32::MAX.to_le_bytes()).unwrap();
        let before = cpu.get_state();
        assert!(matches!(
            cpu.step_checked(),
            Err(Rv64IError::UnknownInstruction { .. })
        ));
        assert_eq!(cpu.get_state(), before);

        cpu.load_checked(&encode_i(0x03, 2, 1, 0, 2).to_le_bytes())
            .unwrap();
        let before = cpu.get_state();
        assert_eq!(
            cpu.step_checked(),
            Err(Rv64IError::MisalignedData {
                address: 2,
                width: 4
            })
        );
        assert_eq!(cpu.get_state(), before);
    }
}
