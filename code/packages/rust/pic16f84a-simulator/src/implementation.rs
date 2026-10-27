use core::fmt;

pub const PROGRAM_WORDS: usize = 1024;
pub const GPR_BYTES: usize = 68;
pub const MAX_RETAINED_STEPS: usize = 64;
const PC_MASK: u16 = 0x1fff;
const WORD_MASK: u16 = 0x3fff;
const STATUS_C: u8 = 1 << 0;
const STATUS_DC: u8 = 1 << 1;
const STATUS_Z: u8 = 1 << 2;
const STATUS_PD: u8 = 1 << 3;
const STATUS_TO: u8 = 1 << 4;
const STATUS_RP0: u8 = 1 << 5;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pic16f84aState {
    pub pc: u16,
    pub w: u8,
    pub status: u8,
    pub fsr: u8,
    pub pclath: u8,
    pub intcon: u8,
    pub gpr: [u8; GPR_BYTES],
    pub return_stack: [u16; 8],
    pub stack_pointer: u8,
    pub program: [u16; PROGRAM_WORDS],
    pub origin: Option<u16>,
    pub length: u16,
    pub sleeping: bool,
    pub halted: bool,
}

impl Default for Pic16f84aState {
    fn default() -> Self {
        Self {
            pc: 0,
            w: 0,
            status: STATUS_TO | STATUS_PD,
            fsr: 0,
            pclath: 0,
            intcon: 0,
            gpr: [0; GPR_BYTES],
            return_stack: [0; 8],
            stack_pointer: 0,
            program: [0; PROGRAM_WORDS],
            origin: None,
            length: 0,
            sleeping: false,
            halted: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Instruction {
    Addwf,
    Andwf,
    Comf,
    Decf,
    Decfsz,
    Incf,
    Incfsz,
    Iorwf,
    Movf,
    Rlf,
    Rrf,
    Subwf,
    Swapf,
    Xorwf,
    Clrf,
    Clrw,
    Movwf,
    Nop,
    Bcf,
    Bsf,
    Btfsc,
    Btfss,
    Addlw,
    Andlw,
    Iorlw,
    Movlw,
    Sublw,
    Xorlw,
    Retlw,
    Call,
    Goto,
    Return,
    Retfie,
    Sleep,
    Clrwdt,
    Halt,
}

impl Instruction {
    #[must_use]
    pub const fn mnemonic(self) -> &'static str {
        match self {
            Self::Addwf => "ADDWF",
            Self::Andwf => "ANDWF",
            Self::Comf => "COMF",
            Self::Decf => "DECF",
            Self::Decfsz => "DECFSZ",
            Self::Incf => "INCF",
            Self::Incfsz => "INCFSZ",
            Self::Iorwf => "IORWF",
            Self::Movf => "MOVF",
            Self::Rlf => "RLF",
            Self::Rrf => "RRF",
            Self::Subwf => "SUBWF",
            Self::Swapf => "SWAPF",
            Self::Xorwf => "XORWF",
            Self::Clrf => "CLRF",
            Self::Clrw => "CLRW",
            Self::Movwf => "MOVWF",
            Self::Nop => "NOP",
            Self::Bcf => "BCF",
            Self::Bsf => "BSF",
            Self::Btfsc => "BTFSC",
            Self::Btfss => "BTFSS",
            Self::Addlw => "ADDLW",
            Self::Andlw => "ANDLW",
            Self::Iorlw => "IORLW",
            Self::Movlw => "MOVLW",
            Self::Sublw => "SUBLW",
            Self::Xorlw => "XORLW",
            Self::Retlw => "RETLW",
            Self::Call => "CALL",
            Self::Goto => "GOTO",
            Self::Return => "RETURN",
            Self::Retfie => "RETFIE",
            Self::Sleep => "SLEEP",
            Self::Clrwdt => "CLRWDT",
            Self::Halt => "HALT",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Pic16f84aError {
    EmptyProgram,
    ProgramWordOutOfRange {
        index: usize,
        word: u16,
    },
    ImageOutOfRange {
        origin: u16,
        length: usize,
    },
    FetchOutsideProgram {
        pc: u16,
        physical: u16,
    },
    UnknownInstruction {
        raw: u16,
        pc: u16,
    },
    UnsupportedLegacyInstruction {
        raw: u16,
        pc: u16,
    },
    UnsupportedPeripheralRegister {
        address: u8,
    },
    InvalidState {
        reason: &'static str,
    },
    Sleeping,
    Halted,
    TraceLimitExceeded {
        requested: usize,
        limit: usize,
    },
    StepLimitExceeded {
        limit: usize,
    },
    AllocationFailed,
    ExecutionFailed {
        step: usize,
        pc: u16,
        source: Box<Pic16f84aError>,
    },
}

impl fmt::Display for Pic16f84aError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProgram => f.write_str("program is empty"),
            Self::ProgramWordOutOfRange { index, word } => {
                write!(f, "program word {index} is not 14-bit: {word:#06x}")
            }
            Self::ImageOutOfRange { origin, length } => {
                write!(
                    f,
                    "{length}-word image at {origin:#05x} exceeds program memory"
                )
            }
            Self::FetchOutsideProgram { pc, physical } => {
                write!(f, "PC {pc:#06x} aliases uninstalled word {physical:#05x}")
            }
            Self::UnknownInstruction { raw, pc } => {
                write!(f, "unknown instruction {raw:#06x} at {pc:#06x}")
            }
            Self::UnsupportedLegacyInstruction { raw, pc } => {
                write!(
                    f,
                    "legacy instruction {raw:#06x} at {pc:#06x} is unsupported"
                )
            }
            Self::UnsupportedPeripheralRegister { address } => {
                write!(f, "file register {address:#04x} belongs to a peripheral")
            }
            Self::InvalidState { reason } => write!(f, "invalid PIC16F84A state: {reason}"),
            Self::Sleeping => f.write_str("processor is sleeping"),
            Self::Halted => f.write_str("processor is halted"),
            Self::TraceLimitExceeded { requested, limit } => {
                write!(f, "requested {requested} traces exceeds limit {limit}")
            }
            Self::StepLimitExceeded { limit } => write!(f, "step limit {limit} exhausted"),
            Self::AllocationFailed => f.write_str("trace allocation failed"),
            Self::ExecutionFailed { step, pc, source } => {
                write!(f, "execution failed at step {step}, PC {pc:#06x}: {source}")
            }
        }
    }
}

impl std::error::Error for Pic16f84aError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ExecutionFailed { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pic16f84aStepTrace {
    pub pc_before: u16,
    pub pc_after: u16,
    pub fetch_index: u16,
    pub raw_word: u16,
    pub mnemonic: &'static str,
    pub cycles: u8,
    pub pipeline_discarded: bool,
    pub skip_taken: bool,
    pub branch_taken: bool,
    pub file_address: Option<u8>,
    pub indirect: bool,
    pub post_state: Pic16f84aState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pic16f84aExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub traces: Vec<Pic16f84aStepTrace>,
    pub final_state: Pic16f84aState,
}

#[derive(Clone, Copy, Debug, Default)]
struct Effects {
    cycles: u8,
    discarded: bool,
    skipped: bool,
    branched: bool,
    file: Option<u8>,
    indirect: bool,
}

impl Effects {
    const fn ordinary() -> Self {
        Self {
            cycles: 1,
            discarded: false,
            skipped: false,
            branched: false,
            file: None,
            indirect: false,
        }
    }

    fn flush(&mut self) {
        self.cycles = 2;
        self.discarded = true;
        self.branched = true;
    }
}

#[derive(Debug, Default)]
pub struct Pic16f84aSimulator {
    state: Pic16f84aState,
}

impl Pic16f84aSimulator {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.state = Pic16f84aState::default();
    }

    #[must_use]
    pub fn get_state(&self) -> Pic16f84aState {
        self.state.clone()
    }

    /// Return an owned snapshot of every architectural and transport field.
    #[must_use]
    pub fn snapshot(&self) -> Pic16f84aState {
        self.state.clone()
    }

    pub fn restore(&mut self, state: Pic16f84aState) -> Result<(), Pic16f84aError> {
        validate_state(&state)?;
        self.state = state;
        Ok(())
    }

    pub fn load(&mut self, words: &[u16], origin: u16) -> Result<(), Pic16f84aError> {
        if words.is_empty() {
            return Err(Pic16f84aError::EmptyProgram);
        }
        for (index, &word) in words.iter().enumerate() {
            if word > WORD_MASK {
                return Err(Pic16f84aError::ProgramWordOutOfRange { index, word });
            }
        }
        let end = usize::from(origin)
            .checked_add(words.len())
            .filter(|&end| end <= PROGRAM_WORDS)
            .ok_or(Pic16f84aError::ImageOutOfRange {
                origin,
                length: words.len(),
            })?;
        let mut state = Pic16f84aState::default();
        state.program[usize::from(origin)..end].copy_from_slice(words);
        state.pc = origin;
        state.origin = Some(origin);
        state.length = words.len() as u16;
        self.state = state;
        Ok(())
    }

    pub fn step(&mut self) -> Result<Pic16f84aStepTrace, Pic16f84aError> {
        if self.state.halted {
            return Err(Pic16f84aError::Halted);
        }
        if self.state.sleeping {
            return Err(Pic16f84aError::Sleeping);
        }
        let pc_before = self.state.pc;
        let fetch_index = pc_before & 0x03ff;
        let raw_word = fetch(&self.state, pc_before)?;
        let instruction = classify_at(raw_word, pc_before)?;
        let mut candidate = self.state.clone();
        let fallthrough = pc_before.wrapping_add(1) & PC_MASK;
        candidate.pc = fallthrough;
        let mut effects = Effects::ordinary();
        execute_instruction(
            &mut candidate,
            instruction,
            raw_word,
            fallthrough,
            &mut effects,
        )?;
        validate_state(&candidate)?;
        let trace = Pic16f84aStepTrace {
            pc_before,
            pc_after: candidate.pc,
            fetch_index,
            raw_word,
            mnemonic: instruction.mnemonic(),
            cycles: effects.cycles,
            pipeline_discarded: effects.discarded,
            skip_taken: effects.skipped,
            branch_taken: effects.branched,
            file_address: effects.file,
            indirect: effects.indirect,
            post_state: candidate.clone(),
        };
        self.state = candidate;
        Ok(trace)
    }

    pub fn run(&mut self, max_steps: usize) -> Result<Pic16f84aExecutionResult, Pic16f84aError> {
        if max_steps > MAX_RETAINED_STEPS {
            return Err(Pic16f84aError::TraceLimitExceeded {
                requested: max_steps,
                limit: MAX_RETAINED_STEPS,
            });
        }
        let original = self.state.clone();
        let mut traces = Vec::new();
        if traces.try_reserve_exact(max_steps).is_err() {
            return Err(Pic16f84aError::AllocationFailed);
        }
        for step in 0..max_steps {
            let pc = self.state.pc;
            match self.step() {
                Ok(trace) => {
                    let halted = trace.post_state.halted;
                    traces.push(trace);
                    if halted {
                        return Ok(Pic16f84aExecutionResult {
                            halted: true,
                            steps: traces.len(),
                            traces,
                            final_state: self.state.clone(),
                        });
                    }
                }
                Err(source) => {
                    self.state = original;
                    return Err(Pic16f84aError::ExecutionFailed {
                        step,
                        pc,
                        source: Box::new(source),
                    });
                }
            }
        }
        let pc = self.state.pc;
        self.state = original;
        Err(Pic16f84aError::ExecutionFailed {
            step: max_steps,
            pc,
            source: Box::new(Pic16f84aError::StepLimitExceeded { limit: max_steps }),
        })
    }

    pub fn execute(
        &mut self,
        words: &[u16],
        origin: u16,
        max_steps: usize,
    ) -> Result<Pic16f84aExecutionResult, Pic16f84aError> {
        let original = self.state.clone();
        self.load(words, origin)?;
        match self.run(max_steps) {
            Ok(result) => Ok(result),
            Err(error) => {
                self.state = original;
                Err(error)
            }
        }
    }
}

pub fn classify(raw: u16) -> Result<Instruction, Pic16f84aError> {
    classify_at(raw, 0)
}

fn classify_at(raw: u16, pc: u16) -> Result<Instruction, Pic16f84aError> {
    if raw > WORD_MASK {
        return Err(Pic16f84aError::UnknownInstruction { raw, pc });
    }
    let exact = match raw {
        0x0001 => Some(Instruction::Halt),
        0x0008 => Some(Instruction::Return),
        0x0009 => Some(Instruction::Retfie),
        0x0063 => Some(Instruction::Sleep),
        0x0064 => Some(Instruction::Clrwdt),
        0x0062 | 0x0065..=0x0067 => {
            return Err(Pic16f84aError::UnsupportedLegacyInstruction { raw, pc })
        }
        _ => None,
    };
    if let Some(instruction) = exact {
        return Ok(instruction);
    }
    let byte = match raw & 0x3f00 {
        0x0700 => Some(Instruction::Addwf),
        0x0500 => Some(Instruction::Andwf),
        0x0900 => Some(Instruction::Comf),
        0x0300 => Some(Instruction::Decf),
        0x0b00 => Some(Instruction::Decfsz),
        0x0a00 => Some(Instruction::Incf),
        0x0f00 => Some(Instruction::Incfsz),
        0x0400 => Some(Instruction::Iorwf),
        0x0800 => Some(Instruction::Movf),
        0x0d00 => Some(Instruction::Rlf),
        0x0c00 => Some(Instruction::Rrf),
        0x0200 => Some(Instruction::Subwf),
        0x0e00 => Some(Instruction::Swapf),
        0x0600 => Some(Instruction::Xorwf),
        _ => None,
    };
    if let Some(instruction) = byte {
        return Ok(instruction);
    }
    if raw & 0x3f80 == 0x0180 {
        return Ok(Instruction::Clrf);
    }
    if raw & 0x3f80 == 0x0100 {
        return Ok(Instruction::Clrw);
    }
    if raw & 0x3f80 == 0x0080 {
        return Ok(Instruction::Movwf);
    }
    if raw & 0x3f9f == 0 {
        return Ok(Instruction::Nop);
    }
    let bit = match raw & 0x3c00 {
        0x1000 => Some(Instruction::Bcf),
        0x1400 => Some(Instruction::Bsf),
        0x1800 => Some(Instruction::Btfsc),
        0x1c00 => Some(Instruction::Btfss),
        _ => None,
    };
    if let Some(instruction) = bit {
        return Ok(instruction);
    }
    if raw & 0x3e00 == 0x3e00 {
        return Ok(Instruction::Addlw);
    }
    if raw & 0x3e00 == 0x3c00 {
        return Ok(Instruction::Sublw);
    }
    match raw & 0x3f00 {
        0x3900 => return Ok(Instruction::Andlw),
        0x3800 => return Ok(Instruction::Iorlw),
        0x3a00 => return Ok(Instruction::Xorlw),
        _ => {}
    }
    match raw & 0x3c00 {
        0x3000 => return Ok(Instruction::Movlw),
        0x3400 => return Ok(Instruction::Retlw),
        _ => {}
    }
    match raw & 0x3800 {
        0x2000 => return Ok(Instruction::Call),
        0x2800 => return Ok(Instruction::Goto),
        _ => {}
    }
    Err(Pic16f84aError::UnknownInstruction { raw, pc })
}

fn validate_state(state: &Pic16f84aState) -> Result<(), Pic16f84aError> {
    if state.pc > PC_MASK {
        return Err(Pic16f84aError::InvalidState {
            reason: "PC exceeds 13 bits",
        });
    }
    if state.status & 0xc0 != 0 {
        return Err(Pic16f84aError::InvalidState {
            reason: "STATUS has unimplemented bits",
        });
    }
    if state.pclath & 0xe0 != 0 {
        return Err(Pic16f84aError::InvalidState {
            reason: "PCLATH has unimplemented bits",
        });
    }
    if state.stack_pointer > 7 {
        return Err(Pic16f84aError::InvalidState {
            reason: "stack pointer exceeds seven",
        });
    }
    if state.return_stack.iter().any(|&word| word > PC_MASK) {
        return Err(Pic16f84aError::InvalidState {
            reason: "stack entry exceeds 13 bits",
        });
    }
    if state.program.iter().any(|&word| word > WORD_MASK) {
        return Err(Pic16f84aError::InvalidState {
            reason: "program word exceeds 14 bits",
        });
    }
    match state.origin {
        None if state.length != 0 => {
            return Err(Pic16f84aError::InvalidState {
                reason: "image length without origin",
            })
        }
        Some(origin)
            if origin > 0x03ff
                || usize::from(origin) + usize::from(state.length) > PROGRAM_WORDS
                || state.length == 0 =>
        {
            return Err(Pic16f84aError::InvalidState {
                reason: "invalid installed image",
            })
        }
        _ => {}
    }
    if state.sleeping && state.halted {
        return Err(Pic16f84aError::InvalidState {
            reason: "sleep and halt both set",
        });
    }
    Ok(())
}

fn fetch(state: &Pic16f84aState, pc: u16) -> Result<u16, Pic16f84aError> {
    let physical = pc & 0x03ff;
    let Some(origin) = state.origin else {
        return Err(Pic16f84aError::FetchOutsideProgram { pc, physical });
    };
    let end = origin + state.length;
    if physical < origin || physical >= end {
        return Err(Pic16f84aError::FetchOutsideProgram { pc, physical });
    }
    Ok(state.program[usize::from(physical)])
}

fn resolve_file(state: &Pic16f84aState, f: u8) -> (u8, bool) {
    let direct = ((state.status & STATUS_RP0) << 2) | (f & 0x7f);
    if direct == 0 || direct == 0x80 {
        (state.fsr, true)
    } else {
        (direct, false)
    }
}

fn is_peripheral(address: u8) -> bool {
    matches!(
        address,
        0x01 | 0x05 | 0x06 | 0x08 | 0x09 | 0x81 | 0x85 | 0x86 | 0x88 | 0x89
    )
}

fn gpr_index(address: u8) -> Option<usize> {
    let low = address & 0x7f;
    (0x0c..=0x4f)
        .contains(&low)
        .then(|| usize::from(low - 0x0c))
}

fn read_file(state: &Pic16f84aState, address: u8, fallthrough: u16) -> Result<u8, Pic16f84aError> {
    if is_peripheral(address) {
        return Err(Pic16f84aError::UnsupportedPeripheralRegister { address });
    }
    if address == 0 || address == 0x80 {
        return Ok(0);
    }
    if let Some(index) = gpr_index(address) {
        return Ok(state.gpr[index]);
    }
    Ok(match address & 0x7f {
        0x02 => fallthrough as u8,
        0x03 => state.status,
        0x04 => state.fsr,
        0x0a => state.pclath,
        0x0b => state.intcon,
        _ => 0,
    })
}

fn merge_flags(state: &mut Pic16f84aState, mask: u8, values: u8) {
    state.status = (state.status & !mask) | (values & mask);
    state.status &= 0x3f;
}

fn write_file(
    state: &mut Pic16f84aState,
    address: u8,
    value: u8,
    flag_mask: u8,
    flag_values: u8,
) -> Result<bool, Pic16f84aError> {
    if is_peripheral(address) {
        return Err(Pic16f84aError::UnsupportedPeripheralRegister { address });
    }
    let mut pc_write = false;
    if address == 0 || address == 0x80 {
        // INDF through an INDF FSR is a synthetic zero/no-op portal.
    } else if let Some(index) = gpr_index(address) {
        state.gpr[index] = value;
    } else {
        match address & 0x7f {
            0x02 => {
                state.pc = ((u16::from(state.pclath & 0x1f) << 8) | u16::from(value)) & PC_MASK;
                pc_write = true;
            }
            0x03 => {
                let preserved = state.status & (STATUS_TO | STATUS_PD);
                state.status = preserved | (value & (STATUS_RP0 | STATUS_Z | STATUS_DC | STATUS_C));
            }
            0x04 => state.fsr = value,
            0x0a => state.pclath = value & 0x1f,
            0x0b => state.intcon = value,
            _ => {}
        }
    }
    merge_flags(state, flag_mask, flag_values);
    Ok(pc_write)
}

fn add_flags(left: u8, right: u8, result: u8) -> u8 {
    let carry = u8::from(u16::from(left) + u16::from(right) > 0xff);
    let digit = u8::from((left & 0x0f) + (right & 0x0f) > 0x0f);
    (carry * STATUS_C) | (digit * STATUS_DC) | (u8::from(result == 0) * STATUS_Z)
}

fn sub_flags(left: u8, right: u8, result: u8) -> u8 {
    let no_borrow = u8::from(left >= right);
    let no_digit_borrow = u8::from((left & 0x0f) >= (right & 0x0f));
    (no_borrow * STATUS_C) | (no_digit_borrow * STATUS_DC) | (u8::from(result == 0) * STATUS_Z)
}

fn push(state: &mut Pic16f84aState, value: u16) {
    state.return_stack[usize::from(state.stack_pointer)] = value & PC_MASK;
    state.stack_pointer = (state.stack_pointer + 1) & 7;
}

fn pop(state: &mut Pic16f84aState) -> u16 {
    state.stack_pointer = state.stack_pointer.wrapping_sub(1) & 7;
    state.return_stack[usize::from(state.stack_pointer)]
}

fn resolve_operand(
    state: &Pic16f84aState,
    raw: u16,
    fallthrough: u16,
    effects: &mut Effects,
) -> Result<(u8, u8), Pic16f84aError> {
    let (address, indirect) = resolve_file(state, raw as u8 & 0x7f);
    effects.file = Some(address);
    effects.indirect = indirect;
    Ok((address, read_file(state, address, fallthrough)?))
}

fn write_destination(
    state: &mut Pic16f84aState,
    raw: u16,
    address: u8,
    value: u8,
    flag_mask: u8,
    flag_values: u8,
    effects: &mut Effects,
) -> Result<bool, Pic16f84aError> {
    if raw & 0x0080 == 0 {
        state.w = value;
        merge_flags(state, flag_mask, flag_values);
        Ok(false)
    } else {
        let wrote_pc = write_file(state, address, value, flag_mask, flag_values)?;
        if wrote_pc {
            effects.flush();
        }
        Ok(wrote_pc)
    }
}

fn execute_instruction(
    state: &mut Pic16f84aState,
    instruction: Instruction,
    raw: u16,
    fallthrough: u16,
    effects: &mut Effects,
) -> Result<(), Pic16f84aError> {
    use Instruction::*;
    match instruction {
        Addwf | Andwf | Comf | Decf | Decfsz | Incf | Incfsz | Iorwf | Movf | Rlf | Rrf | Subwf
        | Swapf | Xorwf => {
            let (address, source) = resolve_operand(state, raw, fallthrough, effects)?;
            let old_c = u8::from(state.status & STATUS_C != 0);
            let (result, mask, flags) = match instruction {
                Addwf => {
                    let result = source.wrapping_add(state.w);
                    (
                        result,
                        STATUS_C | STATUS_DC | STATUS_Z,
                        add_flags(source, state.w, result),
                    )
                }
                Subwf => {
                    let result = source.wrapping_sub(state.w);
                    (
                        result,
                        STATUS_C | STATUS_DC | STATUS_Z,
                        sub_flags(source, state.w, result),
                    )
                }
                Andwf => (
                    source & state.w,
                    STATUS_Z,
                    u8::from(source & state.w == 0) * STATUS_Z,
                ),
                Iorwf => (
                    source | state.w,
                    STATUS_Z,
                    u8::from(source | state.w == 0) * STATUS_Z,
                ),
                Xorwf => (
                    source ^ state.w,
                    STATUS_Z,
                    u8::from(source ^ state.w == 0) * STATUS_Z,
                ),
                Comf => (!source, STATUS_Z, u8::from(source == 0xff) * STATUS_Z),
                Decf => {
                    let result = source.wrapping_sub(1);
                    (result, STATUS_Z, u8::from(result == 0) * STATUS_Z)
                }
                Incf => {
                    let result = source.wrapping_add(1);
                    (result, STATUS_Z, u8::from(result == 0) * STATUS_Z)
                }
                Decfsz => (source.wrapping_sub(1), 0, 0),
                Incfsz => (source.wrapping_add(1), 0, 0),
                Movf => (source, STATUS_Z, u8::from(source == 0) * STATUS_Z),
                Rlf => (
                    (source << 1) | old_c,
                    STATUS_C,
                    u8::from(source & 0x80 != 0) * STATUS_C,
                ),
                Rrf => (
                    (source >> 1) | (old_c << 7),
                    STATUS_C,
                    (source & 1) * STATUS_C,
                ),
                Swapf => (source.rotate_left(4), 0, 0),
                _ => unreachable!(),
            };
            let wrote_pc = write_destination(state, raw, address, result, mask, flags, effects)?;
            if matches!(instruction, Decfsz | Incfsz) && result == 0 {
                effects.skipped = true;
                effects.cycles = 2;
                effects.discarded = true;
                if !wrote_pc {
                    state.pc = state.pc.wrapping_add(1) & PC_MASK;
                }
            }
        }
        Clrf => {
            let (address, indirect) = resolve_file(state, raw as u8 & 0x7f);
            effects.file = Some(address);
            effects.indirect = indirect;
            if write_file(state, address, 0, STATUS_Z, STATUS_Z)? {
                effects.flush();
            }
        }
        Clrw => {
            state.w = 0;
            merge_flags(state, STATUS_Z, STATUS_Z);
        }
        Movwf => {
            let (address, indirect) = resolve_file(state, raw as u8 & 0x7f);
            effects.file = Some(address);
            effects.indirect = indirect;
            if write_file(state, address, state.w, 0, 0)? {
                effects.flush();
            }
        }
        Nop => {}
        Bcf | Bsf | Btfsc | Btfss => {
            let (address, value) = resolve_operand(state, raw, fallthrough, effects)?;
            let bit = ((raw >> 7) & 7) as u8;
            let selected = value & (1 << bit) != 0;
            match instruction {
                Bcf | Bsf => {
                    let result = if instruction == Bsf {
                        value | (1 << bit)
                    } else {
                        value & !(1 << bit)
                    };
                    if write_file(state, address, result, 0, 0)? {
                        effects.flush();
                    }
                }
                Btfsc | Btfss => {
                    let skip = if instruction == Btfsc {
                        !selected
                    } else {
                        selected
                    };
                    if skip {
                        state.pc = state.pc.wrapping_add(1) & PC_MASK;
                        effects.skipped = true;
                        effects.cycles = 2;
                        effects.discarded = true;
                    }
                }
                _ => unreachable!(),
            }
        }
        Addlw | Andlw | Iorlw | Movlw | Sublw | Xorlw => {
            let literal = raw as u8;
            let (result, mask, flags) = match instruction {
                Addlw => {
                    let result = state.w.wrapping_add(literal);
                    (
                        result,
                        STATUS_C | STATUS_DC | STATUS_Z,
                        add_flags(state.w, literal, result),
                    )
                }
                Sublw => {
                    let result = literal.wrapping_sub(state.w);
                    (
                        result,
                        STATUS_C | STATUS_DC | STATUS_Z,
                        sub_flags(literal, state.w, result),
                    )
                }
                Andlw => (
                    state.w & literal,
                    STATUS_Z,
                    u8::from(state.w & literal == 0) * STATUS_Z,
                ),
                Iorlw => (
                    state.w | literal,
                    STATUS_Z,
                    u8::from(state.w | literal == 0) * STATUS_Z,
                ),
                Xorlw => (
                    state.w ^ literal,
                    STATUS_Z,
                    u8::from(state.w ^ literal == 0) * STATUS_Z,
                ),
                Movlw => (literal, 0, 0),
                _ => unreachable!(),
            };
            state.w = result;
            merge_flags(state, mask, flags);
        }
        Call | Goto => {
            if instruction == Call {
                push(state, fallthrough);
            }
            state.pc = ((u16::from(state.pclath & 0x18) << 8) | (raw & 0x07ff)) & PC_MASK;
            effects.flush();
        }
        Return | Retfie | Retlw => {
            if instruction == Retlw {
                state.w = raw as u8;
            }
            state.pc = pop(state);
            if instruction == Retfie {
                state.intcon |= 0x80;
            }
            effects.flush();
        }
        Sleep => {
            state.status |= STATUS_TO;
            state.status &= !STATUS_PD;
            state.sleeping = true;
        }
        Clrwdt => state.status |= STATUS_TO | STATUS_PD,
        Halt => state.halted = true,
    }
    Ok(())
}

pub mod encoding {
    #[must_use]
    pub const fn movlw(value: u8) -> u16 {
        0x3000 | value as u16
    }

    #[must_use]
    pub const fn movwf(file: u8) -> u16 {
        0x0080 | (file as u16 & 0x7f)
    }

    #[must_use]
    pub const fn addwf(file: u8, destination_file: bool) -> u16 {
        0x0700 | ((destination_file as u16) << 7) | (file as u16 & 0x7f)
    }

    #[must_use]
    pub const fn call(address: u16) -> u16 {
        0x2000 | (address & 0x07ff)
    }

    #[must_use]
    pub const fn goto(address: u16) -> u16 {
        0x2800 | (address & 0x07ff)
    }

    #[must_use]
    pub const fn halt() -> u16 {
        0x0001
    }
}
