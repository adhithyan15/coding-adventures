//! Checked Armv7E-M functional model of one Cortex-M4 core.
//!
//! Cortex-M4 is an extension of the Cortex-M3 programmer's model, so this
//! package deliberately reuses the already checked Cortex-M3 oracle for the
//! inherited instruction surface.  The code below owns only the architectural
//! delta: DSP flags/instructions and the optional FPv4-SP register file.  A
//! trace nevertheless contains the whole machine, which keeps the boundary
//! useful as a future gate-level and SoC oracle.

use core::fmt;

#[cfg(test)]
use core::cell::Cell;

pub use cortex_m3_simulator::{
    encoding as core_encoding, CortexM3Error, CortexM3State, LR, MAX_RETAINED_BYTES,
    MAX_RETAINED_STEPS, MEMORY_SIZE, PC, SP,
};
use cortex_m3_simulator::{CortexM3Simulator, InstructionClass as CortexM3InstructionClass};

const APSR_N: u32 = 1 << 31;
const APSR_Z: u32 = 1 << 30;
const APSR_C: u32 = 1 << 29;
const APSR_V: u32 = 1 << 28;
const APSR_Q: u32 = 1 << 27;
const APSR_GE_SHIFT: u32 = 16;
const APSR_GE_MASK: u32 = 0xf << APSR_GE_SHIFT;
const FPSCR_N: u32 = 1 << 31;
const FPSCR_Z: u32 = 1 << 30;
const FPSCR_C: u32 = 1 << 29;
const FPSCR_V: u32 = 1 << 28;
const FPSCR_IDC: u32 = 1 << 7;
const FPSCR_IXC: u32 = 1 << 4;
const FPSCR_UFC: u32 = 1 << 3;
const FPSCR_OFC: u32 = 1 << 2;
const FPSCR_DZC: u32 = 1 << 1;
const FPSCR_IOC: u32 = 1;
const FPSCR_MASK: u32 = FPSCR_N
    | FPSCR_Z
    | FPSCR_C
    | FPSCR_V
    | FPSCR_IDC
    | FPSCR_IXC
    | FPSCR_UFC
    | FPSCR_OFC
    | FPSCR_DZC
    | FPSCR_IOC;
const CANONICAL_NAN: u32 = 0x7fc0_0000;

#[derive(Clone, Copy)]
struct FloatStatus {
    inexact: bool,
    exact_nonzero: bool,
    invalid: bool,
    divide_by_zero: bool,
    overflow: bool,
}

/// Broad decode family, including the inherited Cortex-M3 classes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstructionClass {
    Core(CortexM3InstructionClass),
    Saturating,
    ParallelBytes,
    DspMultiply,
    FloatingPointBinary,
    FloatingPointUnary,
    FloatingPointCompare,
    FloatingPointStatus,
}

/// Complete restorable Cortex-M4 state at an instruction boundary.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CortexM4State {
    /// Complete inherited Armv7-M integer, privilege, memory, and lifecycle state.
    pub core: CortexM3State,
    /// APSR.GE\[3:0\], one bit for each byte lane.
    pub ge: u8,
    /// Whether the composing platform exposes and enables FPv4-SP.
    pub fpu_enabled: bool,
    /// Raw IEEE-754 binary32 bits for S0-S31.
    pub s_registers: [u32; 32],
    /// Bounded FPSCR NZCV and cumulative exception bits.
    pub fpscr: u32,
}

/// Typed execution and lifecycle failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CortexM4Error {
    Core(CortexM3Error),
    InvalidState { reason: &'static str },
    FloatingPointUnavailable { pc: u32 },
    InvalidSRegister { index: u8 },
    UnsupportedFpscr { value: u32 },
    UnknownInstruction32 { first: u16, second: u16, pc: u32 },
}

impl From<CortexM3Error> for CortexM4Error {
    fn from(error: CortexM3Error) -> Self {
        Self::Core(error)
    }
}

impl fmt::Display for CortexM4Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Core(error) => error.fmt(formatter),
            Self::InvalidState { reason } => write!(formatter, "invalid state: {reason}"),
            Self::FloatingPointUnavailable { pc } => {
                write!(
                    formatter,
                    "floating-point unit is unavailable at {pc:#010x}"
                )
            }
            Self::InvalidSRegister { index } => write!(formatter, "invalid S register s{index}"),
            Self::UnsupportedFpscr { value } => {
                write!(formatter, "unsupported FPSCR value {value:#010x}")
            }
            Self::UnknownInstruction32 { first, second, pc } => write!(
                formatter,
                "unknown Cortex-M4 instruction {first:#06x} {second:#06x} at {pc:#010x}"
            ),
        }
    }
}

impl std::error::Error for CortexM4Error {}

/// One committed complete-state transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepTrace {
    pub pc_before: u32,
    pub pc_after: u32,
    pub raw: [u8; 4],
    pub width: usize,
    pub mnemonic: &'static str,
    pub state_before: CortexM4State,
    pub state_after: CortexM4State,
}

/// Successful bounded execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionResult {
    pub stopped: bool,
    pub steps: usize,
    pub traces: Vec<StepTrace>,
    pub final_state: CortexM4State,
}

/// Deterministic single-core Cortex-M4 functional machine.
#[derive(Clone, Debug, Default)]
pub struct CortexM4Simulator {
    state: CortexM4State,
}

impl CortexM4Simulator {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.state = CortexM4State::default();
    }

    #[must_use]
    pub fn get_state(&self) -> CortexM4State {
        self.state.clone()
    }

    pub fn restore(&mut self, state: CortexM4State) -> Result<(), CortexM4Error> {
        validate_extension_state(&state)?;
        let mut core = CortexM3Simulator::new();
        core.restore(try_clone_core(&state.core)?)?;
        self.state = state;
        Ok(())
    }

    pub fn load(&mut self, program: &[u8], origin: u32) -> Result<(), CortexM4Error> {
        let mut core = CortexM3Simulator::new();
        core.load(program, origin)?;
        let fpu_enabled = self.state.fpu_enabled;
        self.state = CortexM4State {
            core: core.get_state(),
            fpu_enabled,
            ..CortexM4State::default()
        };
        Ok(())
    }

    /// Configure the optional FPv4-SP boundary supplied by the surrounding SoC.
    pub fn set_fpu_enabled(&mut self, enabled: bool) {
        self.state.fpu_enabled = enabled;
    }

    #[must_use]
    pub const fn fpu_enabled(&self) -> bool {
        self.state.fpu_enabled
    }

    pub fn read_register(&self, index: u8) -> Result<u32, CortexM4Error> {
        read_core_register(&self.state.core, index)
    }

    pub fn write_register(&mut self, index: u8, value: u32) -> Result<(), CortexM4Error> {
        write_core_register(&mut self.state.core, index, value)
    }

    pub fn read_s_register(&self, index: u8) -> Result<u32, CortexM4Error> {
        self.state
            .s_registers
            .get(usize::from(index))
            .copied()
            .ok_or(CortexM4Error::InvalidSRegister { index })
    }

    pub fn write_s_register(&mut self, index: u8, bits: u32) -> Result<(), CortexM4Error> {
        let Some(register) = self.state.s_registers.get_mut(usize::from(index)) else {
            return Err(CortexM4Error::InvalidSRegister { index });
        };
        *register = bits;
        Ok(())
    }

    pub fn read_memory(&self, address: u32, width: usize) -> Result<u32, CortexM4Error> {
        read_core_memory(&self.state.core, address, width)
    }

    pub fn write_memory(
        &mut self,
        address: u32,
        width: usize,
        value: u32,
    ) -> Result<(), CortexM4Error> {
        write_core_memory(&mut self.state.core, address, width, value)
    }

    pub fn signal_event(&mut self) {
        if self.state.core.sleeping && self.state.core.waiting_for_event {
            self.state.core.sleeping = false;
            self.state.core.waiting_for_event = false;
            self.state.core.event_registered = false;
        } else {
            self.state.core.event_registered = true;
        }
    }

    pub fn signal_interrupt(&mut self) {
        self.state.core.sleeping = false;
        self.state.core.waiting_for_event = false;
    }

    pub fn step(&mut self) -> Result<StepTrace, CortexM4Error> {
        if self.state.core.halted {
            return Err(CortexM3Error::Halted.into());
        }
        if self.state.core.sleeping {
            return Err(CortexM3Error::Sleeping.into());
        }
        let before = try_clone_state(&self.state)?;
        let pc = before.core.pc;
        let first = fetch16(&before.core, pc)?;
        let width = instruction_width(first);
        let second = if width == 4 {
            Some(
                fetch16(&before.core, pc.wrapping_add(2)).map_err(|error| match error {
                    CortexM4Error::Core(CortexM3Error::FetchOutsideProgram { .. }) => {
                        CortexM4Error::Core(CortexM3Error::FetchOutsideProgram { pc, width: 4 })
                    }
                    other => other,
                })?,
            )
        } else {
            None
        };

        if let Some(class) = classify_extension(first, second) {
            let second = second.expect("extension instructions are 32-bit");
            validate_extension_operands(first, second, class, pc)?;
            let mut next = try_clone_state(&before)?;
            next.core.pc = pc.wrapping_add(4);
            let mnemonic = if before.core.itstate != 0
                && !condition_holds(&before.core, before.core.itstate >> 4)
            {
                next.core.itstate = advance_itstate(before.core.itstate);
                "it-skip"
            } else {
                let mnemonic = execute_extension(&mut next, first, second, class, pc)?;
                if before.core.itstate != 0 {
                    next.core.itstate = advance_itstate(before.core.itstate);
                }
                mnemonic
            };
            let trace = make_trace(before, &next, first, second, mnemonic)?;
            self.state = next;
            return Ok(trace);
        }

        // The inherited oracle already enforces IT legality, privilege,
        // lifecycle, and atomicity.  Execute it on a candidate so an outer
        // allocation failure can never partially mutate this machine.
        let mut core = CortexM3Simulator::new();
        core.restore(try_clone_core(&before.core)?)?;
        let core_trace = core.step()?;
        let mut next = try_clone_state(&before)?;
        next.core = core_trace.state_after;
        integrate_inherited_ge(&before, &mut next, first, second, core_trace.mnemonic);
        let trace = StepTrace {
            pc_before: core_trace.pc_before,
            pc_after: core_trace.pc_after,
            raw: core_trace.raw,
            width: core_trace.width,
            mnemonic: core_trace.mnemonic,
            state_before: before,
            state_after: try_clone_state(&next)?,
        };
        self.state = next;
        Ok(trace)
    }

    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, CortexM4Error> {
        check_retention(max_steps)?;
        let before = try_clone_state(&self.state)?;
        let mut traces = Vec::new();
        traces
            .try_reserve_exact(max_steps)
            .map_err(|_| CortexM4Error::Core(CortexM3Error::AllocationFailed))?;
        for _ in 0..max_steps {
            match self.step() {
                Ok(trace) => traces.push(trace),
                Err(error) => {
                    self.state = before;
                    return Err(error);
                }
            }
            if self.state.core.halted || self.state.core.sleeping {
                let final_state = match snapshot_final_state(&self.state) {
                    Ok(state) => state,
                    Err(error) => {
                        self.state = before;
                        return Err(error);
                    }
                };
                return Ok(ExecutionResult {
                    stopped: true,
                    steps: traces.len(),
                    traces,
                    final_state,
                });
            }
        }
        self.state = before;
        Err(CortexM3Error::StepLimitExceeded { limit: max_steps }.into())
    }

    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u32,
        max_steps: usize,
    ) -> Result<ExecutionResult, CortexM4Error> {
        check_retention(max_steps)?;
        let before = try_clone_state(&self.state)?;
        self.load(program, origin)?;
        match self.run(max_steps) {
            Ok(result) => Ok(result),
            Err(error) => {
                self.state = before;
                Err(error)
            }
        }
    }
}

/// Classify an instruction without executing it.
pub fn classify(first: u16, second: Option<u16>) -> Result<InstructionClass, CortexM4Error> {
    if let Some(class) = classify_extension(first, second) {
        return Ok(class);
    }
    cortex_m3_simulator::classify(first, second)
        .map(InstructionClass::Core)
        .map_err(CortexM4Error::Core)
}

fn classify_extension(first: u16, second: Option<u16>) -> Option<InstructionClass> {
    let second = second?;
    if first & 0xfff0 == 0xfa80 {
        return match second & 0xf0f0 {
            0xf080 | 0xf0a0 => Some(InstructionClass::Saturating),
            0xf000 | 0xf040 => Some(InstructionClass::ParallelBytes),
            _ => None,
        };
    }
    if matches!(first & 0xffd0, 0xf300 | 0xf380) && second & 0x8020 == 0 {
        return Some(InstructionClass::Saturating);
    }
    if first & 0xfff0 == 0xfb20 && second & 0x00f0 == 0 {
        return Some(InstructionClass::DspMultiply);
    }
    let binary = matches!(
        (first & 0xffb0, second & 0x0f50),
        (0xee30, 0x0a00 | 0x0a40) | (0xee20 | 0xee80, 0x0a00)
    );
    if binary {
        return Some(InstructionClass::FloatingPointBinary);
    }
    if matches!(first & 0xffbf, 0xeeb0 | 0xeeb1) && matches!(second & 0x0fd0, 0x0a40 | 0x0ac0) {
        return Some(InstructionClass::FloatingPointUnary);
    }
    if first & 0xffbf == 0xeeb4 && second & 0x0fd0 == 0x0a40 {
        return Some(InstructionClass::FloatingPointCompare);
    }
    if matches!(first, 0xeef1 | 0xeee1) && second & 0x0fff == 0x0a10 {
        return Some(InstructionClass::FloatingPointStatus);
    }
    None
}

fn execute_extension(
    state: &mut CortexM4State,
    first: u16,
    second: u16,
    class: InstructionClass,
    pc: u32,
) -> Result<&'static str, CortexM4Error> {
    match class {
        InstructionClass::Saturating => execute_saturating(state, first, second, pc),
        InstructionClass::ParallelBytes => execute_parallel(state, first, second, pc),
        InstructionClass::DspMultiply => execute_dsp_multiply(state, first, second, pc),
        InstructionClass::FloatingPointBinary => execute_fp_binary(state, first, second, pc),
        InstructionClass::FloatingPointUnary => execute_fp_unary(state, first, second, pc),
        InstructionClass::FloatingPointCompare => execute_fp_compare(state, first, second, pc),
        InstructionClass::FloatingPointStatus => execute_fp_status(state, first, second, pc),
        InstructionClass::Core(_) => unreachable!("core classes use the Cortex-M3 oracle"),
    }
}

fn validate_extension_operands(
    first: u16,
    second: u16,
    class: InstructionClass,
    pc: u32,
) -> Result<(), CortexM4Error> {
    match class {
        InstructionClass::Saturating if first & 0xfff0 == 0xfa80 => {
            let rd = ((second >> 8) & 0xf) as u8;
            let rn = (first & 0xf) as u8;
            let rm = (second & 0xf) as u8;
            require_general_registers(first, second, pc, &[rd, rn, rm])
        }
        InstructionClass::Saturating => {
            let rd = ((second >> 8) & 0xf) as u8;
            let rn = (first & 0xf) as u8;
            require_general_registers(first, second, pc, &[rd, rn])
        }
        InstructionClass::ParallelBytes => {
            let rd = ((second >> 8) & 0xf) as u8;
            let rn = (first & 0xf) as u8;
            let rm = (second & 0xf) as u8;
            require_general_registers(first, second, pc, &[rd, rn, rm])
        }
        InstructionClass::DspMultiply => {
            let rd = ((second >> 8) & 0xf) as u8;
            let rn = (first & 0xf) as u8;
            let rm = (second & 0xf) as u8;
            let ra = ((second >> 12) & 0xf) as u8;
            require_general_registers(first, second, pc, &[rd, rn, rm])?;
            if ra == PC {
                Ok(())
            } else {
                require_general_registers(first, second, pc, &[ra])
            }
        }
        InstructionClass::FloatingPointStatus => {
            let rt = ((second >> 12) & 0xf) as u8;
            require_general_registers(first, second, pc, &[rt])
        }
        InstructionClass::FloatingPointBinary
        | InstructionClass::FloatingPointUnary
        | InstructionClass::FloatingPointCompare => Ok(()),
        InstructionClass::Core(_) => unreachable!("core classes use the Cortex-M3 oracle"),
    }
}

fn integrate_inherited_ge(
    before: &CortexM4State,
    next: &mut CortexM4State,
    first: u16,
    second: Option<u16>,
    mnemonic: &str,
) {
    let Some(second) = second else {
        return;
    };
    let sysm = second as u8;
    if sysm > 3 {
        return;
    }
    if mnemonic == "mrs" && first == 0xf3ef {
        let rd = usize::from(((second >> 8) & 0xf) as u8);
        next.core.registers[rd] =
            (next.core.registers[rd] & !APSR_GE_MASK) | (u32::from(before.ge) << APSR_GE_SHIFT);
    } else if mnemonic == "msr" && first & 0xfff0 == 0xf380 {
        let rn = usize::from((first & 0xf) as u8);
        next.ge = ((before.core.registers[rn] & APSR_GE_MASK) >> APSR_GE_SHIFT) as u8;
    }
}

fn execute_saturating(
    state: &mut CortexM4State,
    first: u16,
    second: u16,
    pc: u32,
) -> Result<&'static str, CortexM4Error> {
    if first & 0xfff0 == 0xfa80 {
        let rd = ((second >> 8) & 0xf) as u8;
        let rn = (first & 0xf) as u8;
        let rm = (second & 0xf) as u8;
        require_general_registers(first, second, pc, &[rd, rn, rm])?;
        let left = i64::from(read_core_register(&state.core, rm)? as i32);
        let right = i64::from(read_core_register(&state.core, rn)? as i32);
        let (value, saturated, mnemonic) = match second & 0x00f0 {
            0x0080 => {
                let (value, saturated) = saturate_i32(left + right);
                (value, saturated, "qadd")
            }
            0x00a0 => {
                let (value, saturated) = saturate_i32(left - right);
                (value, saturated, "qsub")
            }
            _ => return Err(unknown(first, second, pc)),
        };
        write_core_register(&mut state.core, rd, value as u32)?;
        set_q(state, saturated);
        return Ok(mnemonic);
    }

    let rd = ((second >> 8) & 0xf) as u8;
    let rn = (first & 0xf) as u8;
    require_general_registers(first, second, pc, &[rd, rn])?;
    let shift = (((second >> 12) & 7) << 2 | ((second >> 6) & 3)) as u32;
    let arithmetic = first & 0x20 != 0;
    let source = read_core_register(&state.core, rn)?;
    let shifted = if arithmetic {
        (source as i32) >> (if shift == 0 { 31 } else { shift })
    } else {
        source.wrapping_shl(shift) as i32
    };
    if first & 0x0080 == 0 {
        let bits = (second & 0x1f) as u32 + 1;
        let (value, saturated) = signed_saturate(i64::from(shifted), bits);
        write_core_register(&mut state.core, rd, value as u32)?;
        set_q(state, saturated);
        Ok("ssat")
    } else {
        let bits = (second & 0x1f) as u32;
        let (value, saturated) = unsigned_saturate(i64::from(shifted), bits);
        write_core_register(&mut state.core, rd, value)?;
        set_q(state, saturated);
        Ok("usat")
    }
}

fn execute_parallel(
    state: &mut CortexM4State,
    first: u16,
    second: u16,
    pc: u32,
) -> Result<&'static str, CortexM4Error> {
    let rd = ((second >> 8) & 0xf) as u8;
    let rn = (first & 0xf) as u8;
    let rm = (second & 0xf) as u8;
    require_general_registers(first, second, pc, &[rd, rn, rm])?;
    let left = read_core_register(&state.core, rn)?;
    let right = read_core_register(&state.core, rm)?;
    let unsigned = second & 0x0040 != 0;
    let mut result = 0_u32;
    let mut ge = 0_u8;
    for lane in 0..4 {
        let shift = lane * 8;
        let left_byte = ((left >> shift) & 0xff) as u8;
        let right_byte = ((right >> shift) & 0xff) as u8;
        if unsigned {
            let sum = u16::from(left_byte) + u16::from(right_byte);
            result |= u32::from(sum as u8) << shift;
            ge |= u8::from(sum > 0xff) << lane;
        } else {
            let sum = i16::from(left_byte as i8) + i16::from(right_byte as i8);
            result |= u32::from(sum as u8) << shift;
            ge |= u8::from(sum >= 0) << lane;
        }
    }
    state.ge = ge;
    write_core_register(&mut state.core, rd, result)?;
    Ok(if unsigned { "uadd8" } else { "sadd8" })
}

fn execute_dsp_multiply(
    state: &mut CortexM4State,
    first: u16,
    second: u16,
    pc: u32,
) -> Result<&'static str, CortexM4Error> {
    let rd = ((second >> 8) & 0xf) as u8;
    let rn = (first & 0xf) as u8;
    let rm = (second & 0xf) as u8;
    let ra = ((second >> 12) & 0xf) as u8;
    require_general_registers(first, second, pc, &[rd, rn, rm])?;
    if ra != 15 {
        require_general_registers(first, second, pc, &[ra])?;
    }
    let left = read_core_register(&state.core, rn)?;
    let right = read_core_register(&state.core, rm)?;
    let products = i64::from((left as u16 as i16) as i32) * i64::from((right as u16 as i16) as i32)
        + i64::from(((left >> 16) as u16 as i16) as i32)
            * i64::from(((right >> 16) as u16 as i16) as i32);
    let accumulated = if ra == 15 {
        products
    } else {
        products + i64::from(read_core_register(&state.core, ra)? as i32)
    };
    let overflow = accumulated < i64::from(i32::MIN) || accumulated > i64::from(i32::MAX);
    write_core_register(&mut state.core, rd, accumulated as u32)?;
    set_q(state, overflow);
    Ok(if ra == 15 { "smuad" } else { "smlad" })
}

fn execute_fp_binary(
    state: &mut CortexM4State,
    first: u16,
    second: u16,
    pc: u32,
) -> Result<&'static str, CortexM4Error> {
    require_fpu(state, pc)?;
    let sd = fp_sd(first, second);
    let sn = fp_sn(first, second);
    let sm = fp_sm(second);
    let left_bits = state.s_registers[usize::from(sn)];
    let right_bits = state.s_registers[usize::from(sm)];
    record_input_denormal(state, left_bits);
    record_input_denormal(state, right_bits);
    let left = f32::from_bits(left_bits);
    let right = f32::from_bits(right_bits);
    let (result, inexact, exact_nonzero, invalid, divide_by_zero, mnemonic) =
        match (first & 0xffb0, second & 0x0040) {
            (0xee30, 0) => (
                left + right,
                finite_inexact_sum(left_bits, right_bits, (left + right).to_bits(), false),
                left + right != 0.0,
                left.is_infinite()
                    && right.is_infinite()
                    && left.is_sign_negative() != right.is_sign_negative(),
                false,
                "vadd.f32",
            ),
            (0xee30, 0x0040) => (
                left - right,
                finite_inexact_sum(left_bits, right_bits, (left - right).to_bits(), true),
                left - right != 0.0,
                left.is_infinite()
                    && right.is_infinite()
                    && left.is_sign_negative() == right.is_sign_negative(),
                false,
                "vsub.f32",
            ),
            (0xee20, 0) => (
                left * right,
                finite_inexact_product(left_bits, right_bits, (left * right).to_bits()),
                left.is_finite() && right.is_finite() && left != 0.0 && right != 0.0,
                (left == 0.0 && right.is_infinite()) || (right == 0.0 && left.is_infinite()),
                false,
                "vmul.f32",
            ),
            (0xee80, 0) => (
                left / right,
                finite_inexact_quotient(left_bits, right_bits, (left / right).to_bits()),
                left.is_finite() && right.is_finite() && left != 0.0 && right != 0.0,
                (left == 0.0 && right == 0.0) || (left.is_infinite() && right.is_infinite()),
                right == 0.0 && left.is_finite() && left != 0.0,
                "vdiv.f32",
            ),
            _ => return Err(unknown(first, second, pc)),
        };
    let invalid = invalid || signaling_nan(left_bits) || signaling_nan(right_bits);
    let status = FloatStatus {
        inexact,
        exact_nonzero,
        invalid,
        divide_by_zero,
        overflow: result.is_infinite()
            && left.is_finite()
            && right.is_finite()
            && !divide_by_zero
            && !invalid,
    };
    let bits = finish_float(state, result, status);
    state.s_registers[usize::from(sd)] = bits;
    Ok(mnemonic)
}

fn execute_fp_unary(
    state: &mut CortexM4State,
    first: u16,
    second: u16,
    pc: u32,
) -> Result<&'static str, CortexM4Error> {
    require_fpu(state, pc)?;
    let sd = fp_sd(first, second);
    let sm = fp_sm(second);
    let bits = state.s_registers[usize::from(sm)];
    let value = f32::from_bits(bits);
    let (result_bits, mnemonic) = match (first & 0x000f, second & 0x0080) {
        (0, 0) => (bits, "vmov.f32"),
        (0, 0x0080) => (bits & 0x7fff_ffff, "vabs.f32"),
        (1, 0) => (bits ^ 0x8000_0000, "vneg.f32"),
        (1, 0x0080) => {
            record_input_denormal(state, bits);
            let invalid = value.is_sign_negative() && value != 0.0 || signaling_nan(bits);
            let result = value.sqrt();
            let inexact = value.is_finite()
                && result.is_finite()
                && value >= 0.0
                && !exact_product_equals(result.to_bits(), result.to_bits(), bits);
            (
                finish_float(
                    state,
                    result,
                    FloatStatus {
                        inexact,
                        exact_nonzero: value.is_finite() && value != 0.0,
                        invalid,
                        divide_by_zero: false,
                        overflow: false,
                    },
                ),
                "vsqrt.f32",
            )
        }
        _ => return Err(unknown(first, second, pc)),
    };
    state.s_registers[usize::from(sd)] = result_bits;
    Ok(mnemonic)
}

fn execute_fp_compare(
    state: &mut CortexM4State,
    first: u16,
    second: u16,
    pc: u32,
) -> Result<&'static str, CortexM4Error> {
    require_fpu(state, pc)?;
    let sn = fp_sd(first, second);
    let sm = fp_sm(second);
    let left_bits = state.s_registers[usize::from(sn)];
    let right_bits = state.s_registers[usize::from(sm)];
    record_input_denormal(state, left_bits);
    record_input_denormal(state, right_bits);
    let left = f32::from_bits(left_bits);
    let right = f32::from_bits(right_bits);
    state.fpscr &= !(FPSCR_N | FPSCR_Z | FPSCR_C | FPSCR_V);
    if left.is_nan() || right.is_nan() {
        state.fpscr |= FPSCR_C | FPSCR_V;
        if signaling_nan(left_bits) || signaling_nan(right_bits) {
            state.fpscr |= FPSCR_IOC;
        }
    } else if left == right {
        state.fpscr |= FPSCR_Z | FPSCR_C;
    } else if left < right {
        state.fpscr |= FPSCR_N;
    } else {
        state.fpscr |= FPSCR_C;
    }
    Ok("vcmp.f32")
}

fn execute_fp_status(
    state: &mut CortexM4State,
    first: u16,
    second: u16,
    pc: u32,
) -> Result<&'static str, CortexM4Error> {
    require_fpu(state, pc)?;
    let rt = ((second >> 12) & 0xf) as u8;
    require_general_registers(first, second, pc, &[rt])?;
    if first == 0xeef1 {
        write_core_register(&mut state.core, rt, state.fpscr)?;
        Ok("vmrs")
    } else {
        let value = read_core_register(&state.core, rt)?;
        if value & !FPSCR_MASK != 0 {
            return Err(CortexM4Error::UnsupportedFpscr { value });
        }
        state.fpscr = value;
        Ok("vmsr")
    }
}

fn finish_float(state: &mut CortexM4State, result: f32, status: FloatStatus) -> u32 {
    if status.invalid {
        state.fpscr |= FPSCR_IOC;
    }
    if status.divide_by_zero {
        state.fpscr |= FPSCR_DZC;
    }
    if status.overflow {
        state.fpscr |= FPSCR_OFC | FPSCR_IXC;
    }
    if status.exact_nonzero
        && result.is_finite()
        && result.abs() < f32::MIN_POSITIVE
        && status.inexact
    {
        state.fpscr |= FPSCR_UFC;
    }
    if status.inexact {
        state.fpscr |= FPSCR_IXC;
    }
    if result.is_nan() {
        CANONICAL_NAN
    } else {
        result.to_bits()
    }
}

/// Return the signed integer significand and base-two exponent of a finite
/// binary32 value.  This representation is exact, including subnormals.
fn finite_dyadic(bits: u32) -> Option<(i64, i32)> {
    let exponent = (bits >> 23) & 0xff;
    if exponent == 0xff {
        return None;
    }
    let fraction = bits & 0x007f_ffff;
    if exponent == 0 && fraction == 0 {
        return Some((0, 0));
    }
    let significand = if exponent == 0 {
        i64::from(fraction)
    } else {
        i64::from((1 << 23) | fraction)
    };
    let signed = if bits >> 31 == 0 {
        significand
    } else {
        -significand
    };
    let power = if exponent == 0 {
        -149
    } else {
        exponent as i32 - 127 - 23
    };
    Some((signed, power))
}

fn normalize_dyadic((significand, exponent): (i64, i32)) -> (i64, i32) {
    if significand == 0 {
        return (0, 0);
    }
    let trailing = significand.unsigned_abs().trailing_zeros();
    (significand >> trailing, exponent + trailing as i32)
}

/// Decide whether three exact dyadic terms sum to zero without narrowing
/// through a host float.  At the least occupied exponent, an odd coefficient
/// cannot be cancelled by any higher term; equal-exponent coefficients are
/// combined and normalized until that parity proof applies.
fn dyadic_sum_is_zero(mut terms: [(i64, i32); 3]) -> bool {
    for term in &mut terms {
        *term = normalize_dyadic(*term);
    }
    loop {
        let Some(minimum) = terms
            .iter()
            .filter(|(significand, _)| *significand != 0)
            .map(|(_, exponent)| *exponent)
            .min()
        else {
            return true;
        };
        let mut matching = [usize::MAX; 3];
        let mut matching_len = 0;
        for (index, (significand, exponent)) in terms.iter().enumerate() {
            if *significand != 0 && *exponent == minimum {
                matching[matching_len] = index;
                matching_len += 1;
            }
        }
        if matching_len == 1 {
            return false;
        }
        let sum = matching[..matching_len]
            .iter()
            .map(|index| terms[*index].0)
            .sum();
        terms[matching[0]] = normalize_dyadic((sum, minimum));
        for index in &matching[1..matching_len] {
            terms[*index] = (0, 0);
        }
    }
}

fn exact_sum_equals(left: u32, right: u32, result: u32, subtract: bool) -> bool {
    let (left_significand, left_exponent) = finite_dyadic(left).expect("finite operand");
    let (mut right_significand, right_exponent) = finite_dyadic(right).expect("finite operand");
    let (result_significand, result_exponent) = finite_dyadic(result).expect("finite result");
    if subtract {
        right_significand = -right_significand;
    }
    dyadic_sum_is_zero([
        (left_significand, left_exponent),
        (right_significand, right_exponent),
        (-result_significand, result_exponent),
    ])
}

fn exact_product_equals(left: u32, right: u32, result: u32) -> bool {
    let Some((left_significand, left_exponent)) = finite_dyadic(left) else {
        return false;
    };
    let Some((right_significand, right_exponent)) = finite_dyadic(right) else {
        return false;
    };
    let Some((result_significand, result_exponent)) = finite_dyadic(result) else {
        return false;
    };
    dyadic_sum_is_zero([
        (
            left_significand * right_significand,
            left_exponent + right_exponent,
        ),
        (-result_significand, result_exponent),
        (0, 0),
    ])
}

fn finite_inexact_sum(left: u32, right: u32, result: u32, subtract: bool) -> bool {
    finite_dyadic(left).is_some()
        && finite_dyadic(right).is_some()
        && finite_dyadic(result).is_some()
        && !exact_sum_equals(left, right, result, subtract)
}

fn finite_inexact_product(left: u32, right: u32, result: u32) -> bool {
    finite_dyadic(left).is_some()
        && finite_dyadic(right).is_some()
        && finite_dyadic(result).is_some()
        && !exact_product_equals(left, right, result)
}

fn finite_inexact_quotient(left: u32, right: u32, result: u32) -> bool {
    finite_dyadic(left).is_some()
        && finite_dyadic(right).is_some()
        && finite_dyadic(result).is_some()
        && f32::from_bits(right) != 0.0
        && !exact_product_equals(result, right, left)
}

fn fp_sd(first: u16, second: u16) -> u8 {
    (((second >> 12) & 0xf) as u8) * 2 + ((first >> 6) & 1) as u8
}

fn fp_sn(_first: u16, second: u16) -> u8 {
    // Binary encodings keep Vn in first[3:0] and N in second[7].
    // The caller supplies first through the helper below to keep extraction
    // visibly paired with the architectural bit split.
    (((_first & 0xf) as u8) * 2) + ((second >> 7) & 1) as u8
}

fn fp_sm(second: u16) -> u8 {
    ((second & 0xf) as u8) * 2 + ((second >> 5) & 1) as u8
}

fn require_fpu(state: &CortexM4State, pc: u32) -> Result<(), CortexM4Error> {
    if state.fpu_enabled {
        Ok(())
    } else {
        Err(CortexM4Error::FloatingPointUnavailable { pc })
    }
}

fn require_general_registers(
    first: u16,
    second: u16,
    pc: u32,
    registers: &[u8],
) -> Result<(), CortexM4Error> {
    if registers.iter().all(|register| *register <= 12) {
        Ok(())
    } else {
        Err(unknown(first, second, pc))
    }
}

fn unknown(first: u16, second: u16, pc: u32) -> CortexM4Error {
    CortexM4Error::UnknownInstruction32 { first, second, pc }
}

fn set_q(state: &mut CortexM4State, saturated: bool) {
    if saturated {
        state.core.apsr |= APSR_Q;
    }
}

fn saturate_i32(value: i64) -> (i32, bool) {
    if value > i64::from(i32::MAX) {
        (i32::MAX, true)
    } else if value < i64::from(i32::MIN) {
        (i32::MIN, true)
    } else {
        (value as i32, false)
    }
}

fn signed_saturate(value: i64, bits: u32) -> (i32, bool) {
    if bits == 32 {
        return saturate_i32(value);
    }
    let minimum = -(1_i64 << (bits - 1));
    let maximum = (1_i64 << (bits - 1)) - 1;
    if value < minimum {
        (minimum as i32, true)
    } else if value > maximum {
        (maximum as i32, true)
    } else {
        (value as i32, false)
    }
}

fn unsigned_saturate(value: i64, bits: u32) -> (u32, bool) {
    let maximum = if bits == 0 { 0 } else { (1_i64 << bits) - 1 };
    if value < 0 {
        (0, true)
    } else if value > maximum {
        (maximum as u32, true)
    } else {
        (value as u32, false)
    }
}

fn signaling_nan(bits: u32) -> bool {
    bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0 && bits & 0x0040_0000 == 0
}

fn record_input_denormal(state: &mut CortexM4State, bits: u32) {
    if bits & 0x7f80_0000 == 0 && bits & 0x007f_ffff != 0 {
        state.fpscr |= FPSCR_IDC;
    }
}

fn validate_extension_state(state: &CortexM4State) -> Result<(), CortexM4Error> {
    if state.ge & !0xf != 0 {
        return Err(CortexM4Error::InvalidState {
            reason: "APSR.GE contains reserved bits",
        });
    }
    if state.fpscr & !FPSCR_MASK != 0 {
        return Err(CortexM4Error::UnsupportedFpscr { value: state.fpscr });
    }
    Ok(())
}

#[cfg(test)]
thread_local! {
    static FAIL_FINAL_STATE_SNAPSHOT_ONCE: Cell<bool> = const { Cell::new(false) };
    static FAIL_STATE_CLONE_ON_CALL: Cell<usize> = const { Cell::new(0) };
}

fn snapshot_final_state(state: &CortexM4State) -> Result<CortexM4State, CortexM4Error> {
    #[cfg(test)]
    if FAIL_FINAL_STATE_SNAPSHOT_ONCE.with(|fail| fail.replace(false)) {
        return Err(CortexM4Error::Core(CortexM3Error::AllocationFailed));
    }
    try_clone_state(state)
}

fn try_clone_state(state: &CortexM4State) -> Result<CortexM4State, CortexM4Error> {
    #[cfg(test)]
    if FAIL_STATE_CLONE_ON_CALL.with(|fail| {
        let remaining = fail.get();
        if remaining == 0 {
            false
        } else {
            fail.set(remaining - 1);
            remaining == 1
        }
    }) {
        return Err(CortexM4Error::Core(CortexM3Error::AllocationFailed));
    }
    Ok(CortexM4State {
        core: try_clone_core(&state.core)?,
        ge: state.ge,
        fpu_enabled: state.fpu_enabled,
        s_registers: state.s_registers,
        fpscr: state.fpscr,
    })
}

fn try_clone_core(state: &CortexM3State) -> Result<CortexM3State, CortexM4Error> {
    let mut memory = Vec::new();
    memory
        .try_reserve_exact(state.memory.len())
        .map_err(|_| CortexM4Error::Core(CortexM3Error::AllocationFailed))?;
    memory.extend_from_slice(&state.memory);
    Ok(CortexM3State {
        registers: state.registers,
        msp: state.msp,
        psp: state.psp,
        lr: state.lr,
        pc: state.pc,
        apsr: state.apsr,
        ipsr: state.ipsr,
        itstate: state.itstate,
        primask: state.primask,
        basepri: state.basepri,
        faultmask: state.faultmask,
        control: state.control,
        handler_mode: state.handler_mode,
        memory,
        halted: state.halted,
        sleeping: state.sleeping,
        waiting_for_event: state.waiting_for_event,
        event_registered: state.event_registered,
        exclusive_address: state.exclusive_address,
        loaded_origin: state.loaded_origin,
        loaded_len: state.loaded_len,
    })
}

fn check_retention(max_steps: usize) -> Result<(), CortexM4Error> {
    if max_steps > MAX_RETAINED_STEPS {
        Err(CortexM3Error::RetentionLimitExceeded {
            requested: max_steps,
            limit: MAX_RETAINED_STEPS,
        }
        .into())
    } else {
        Ok(())
    }
}

fn instruction_width(first: u16) -> usize {
    if matches!(first >> 11, 0b11101..=0b11111) {
        4
    } else {
        2
    }
}

fn fetch16(state: &CortexM3State, pc: u32) -> Result<u16, CortexM4Error> {
    if pc & 1 != 0 {
        return Err(CortexM3Error::MisalignedFetch { pc }.into());
    }
    if state.loaded_len < 2
        || pc
            .checked_sub(state.loaded_origin)
            .is_none_or(|offset| offset > state.loaded_len.saturating_sub(2) as u32)
    {
        return Err(CortexM3Error::FetchOutsideProgram { pc, width: 2 }.into());
    }
    let start =
        usize::try_from(pc).map_err(|_| CortexM3Error::FetchOutsideProgram { pc, width: 2 })?;
    Ok(u16::from_le_bytes([
        state.memory[start],
        state.memory[start + 1],
    ]))
}

fn read_core_register(state: &CortexM3State, index: u8) -> Result<u32, CortexM4Error> {
    match index {
        0..=12 => Ok(state.registers[usize::from(index)]),
        SP => Ok(if !state.handler_mode && state.control & 2 != 0 {
            state.psp
        } else {
            state.msp
        }),
        LR => Ok(state.lr),
        PC => Ok(state.pc),
        _ => Err(CortexM3Error::InvalidRegister { index }.into()),
    }
}

fn write_core_register(
    state: &mut CortexM3State,
    index: u8,
    value: u32,
) -> Result<(), CortexM4Error> {
    match index {
        0..=12 => state.registers[usize::from(index)] = value,
        SP if !state.handler_mode && state.control & 2 != 0 => state.psp = value & !3,
        SP => state.msp = value & !3,
        LR => state.lr = value,
        PC => state.pc = value & !1,
        _ => return Err(CortexM3Error::InvalidRegister { index }.into()),
    }
    Ok(())
}

fn checked_range(address: u32, width: usize) -> Result<std::ops::Range<usize>, CortexM4Error> {
    if !matches!(width, 1 | 2 | 4) {
        return Err(CortexM3Error::DataOutOfRange { address, width }.into());
    }
    let start =
        usize::try_from(address).map_err(|_| CortexM3Error::DataOutOfRange { address, width })?;
    let end = start
        .checked_add(width)
        .filter(|end| *end <= MEMORY_SIZE)
        .ok_or(CortexM3Error::DataOutOfRange { address, width })?;
    Ok(start..end)
}

fn read_core_memory(
    state: &CortexM3State,
    address: u32,
    width: usize,
) -> Result<u32, CortexM4Error> {
    let range = checked_range(address, width)?;
    let mut bytes = [0; 4];
    bytes[..width].copy_from_slice(&state.memory[range]);
    Ok(u32::from_le_bytes(bytes))
}

fn write_core_memory(
    state: &mut CortexM3State,
    address: u32,
    width: usize,
    value: u32,
) -> Result<(), CortexM4Error> {
    let range = checked_range(address, width)?;
    state.memory[range].copy_from_slice(&value.to_le_bytes()[..width]);
    state.exclusive_address = None;
    Ok(())
}

fn condition_holds(state: &CortexM3State, condition: u8) -> bool {
    let n = state.apsr & APSR_N != 0;
    let z = state.apsr & APSR_Z != 0;
    let c = state.apsr & APSR_C != 0;
    let v = state.apsr & APSR_V != 0;
    match condition {
        0 => z,
        1 => !z,
        2 => c,
        3 => !c,
        4 => n,
        5 => !n,
        6 => v,
        7 => !v,
        8 => c && !z,
        9 => !c || z,
        10 => n == v,
        11 => n != v,
        12 => !z && n == v,
        13 => z || n != v,
        14 => true,
        _ => false,
    }
}

fn advance_itstate(itstate: u8) -> u8 {
    if itstate & 7 == 0 {
        0
    } else {
        (itstate & 0xe0) | ((itstate << 1) & 0x1f)
    }
}

fn make_trace(
    before: CortexM4State,
    next: &CortexM4State,
    first: u16,
    second: u16,
    mnemonic: &'static str,
) -> Result<StepTrace, CortexM4Error> {
    let mut raw = [0; 4];
    raw[..2].copy_from_slice(&first.to_le_bytes());
    raw[2..].copy_from_slice(&second.to_le_bytes());
    Ok(StepTrace {
        pc_before: before.core.pc,
        pc_after: next.core.pc,
        raw,
        width: 4,
        mnemonic,
        state_before: before,
        state_after: try_clone_state(next)?,
    })
}

/// Little-endian Cortex-M4 extension encoding helpers.
pub mod encoding {
    #[must_use]
    pub const fn qadd(rd: u8, rm: u8, rn: u8) -> [u16; 2] {
        [
            0xfa80 | (rn as u16 & 0xf),
            0xf080 | ((rd as u16 & 0xf) << 8) | (rm as u16 & 0xf),
        ]
    }

    #[must_use]
    pub const fn qsub(rd: u8, rm: u8, rn: u8) -> [u16; 2] {
        [
            0xfa80 | (rn as u16 & 0xf),
            0xf0a0 | ((rd as u16 & 0xf) << 8) | (rm as u16 & 0xf),
        ]
    }

    #[must_use]
    pub const fn sadd8(rd: u8, rn: u8, rm: u8) -> [u16; 2] {
        [
            0xfa80 | (rn as u16 & 0xf),
            0xf000 | ((rd as u16 & 0xf) << 8) | (rm as u16 & 0xf),
        ]
    }

    #[must_use]
    pub const fn uadd8(rd: u8, rn: u8, rm: u8) -> [u16; 2] {
        [
            0xfa80 | (rn as u16 & 0xf),
            0xf040 | ((rd as u16 & 0xf) << 8) | (rm as u16 & 0xf),
        ]
    }

    #[must_use]
    pub const fn ssat(rd: u8, bits: u8, rn: u8) -> [u16; 2] {
        [
            0xf300 | (rn as u16 & 0xf),
            ((rd as u16 & 0xf) << 8) | (bits.saturating_sub(1) as u16 & 0x1f),
        ]
    }

    #[must_use]
    pub const fn usat(rd: u8, bits: u8, rn: u8) -> [u16; 2] {
        [
            0xf380 | (rn as u16 & 0xf),
            ((rd as u16 & 0xf) << 8) | (bits as u16 & 0x1f),
        ]
    }

    #[must_use]
    pub const fn smuad(rd: u8, rn: u8, rm: u8) -> [u16; 2] {
        [
            0xfb20 | (rn as u16 & 0xf),
            0xf000 | ((rd as u16 & 0xf) << 8) | (rm as u16 & 0xf),
        ]
    }

    #[must_use]
    pub const fn smlad(rd: u8, rn: u8, rm: u8, ra: u8) -> [u16; 2] {
        [
            0xfb20 | (rn as u16 & 0xf),
            ((ra as u16 & 0xf) << 12) | ((rd as u16 & 0xf) << 8) | (rm as u16 & 0xf),
        ]
    }

    const fn fp_binary(base: u16, subtract: bool, sd: u8, sn: u8, sm: u8) -> [u16; 2] {
        [
            base | ((sd as u16 & 1) << 6) | ((sn as u16 >> 1) & 0xf),
            ((sd as u16 >> 1) << 12)
                | 0x0a00
                | ((sn as u16 & 1) << 7)
                | ((subtract as u16) << 6)
                | ((sm as u16 & 1) << 5)
                | ((sm as u16 >> 1) & 0xf),
        ]
    }

    #[must_use]
    pub const fn vadd_f32(sd: u8, sn: u8, sm: u8) -> [u16; 2] {
        fp_binary(0xee30, false, sd, sn, sm)
    }

    #[must_use]
    pub const fn vsub_f32(sd: u8, sn: u8, sm: u8) -> [u16; 2] {
        fp_binary(0xee30, true, sd, sn, sm)
    }

    #[must_use]
    pub const fn vmul_f32(sd: u8, sn: u8, sm: u8) -> [u16; 2] {
        fp_binary(0xee20, false, sd, sn, sm)
    }

    #[must_use]
    pub const fn vdiv_f32(sd: u8, sn: u8, sm: u8) -> [u16; 2] {
        fp_binary(0xee80, false, sd, sn, sm)
    }

    const fn fp_unary(base: u16, variant: bool, sd: u8, sm: u8) -> [u16; 2] {
        [
            base | ((sd as u16 & 1) << 6),
            ((sd as u16 >> 1) << 12)
                | 0x0a40
                | ((variant as u16) << 7)
                | ((sm as u16 & 1) << 5)
                | ((sm as u16 >> 1) & 0xf),
        ]
    }

    #[must_use]
    pub const fn vmov_f32(sd: u8, sm: u8) -> [u16; 2] {
        fp_unary(0xeeb0, false, sd, sm)
    }

    #[must_use]
    pub const fn vabs_f32(sd: u8, sm: u8) -> [u16; 2] {
        fp_unary(0xeeb0, true, sd, sm)
    }

    #[must_use]
    pub const fn vneg_f32(sd: u8, sm: u8) -> [u16; 2] {
        fp_unary(0xeeb1, false, sd, sm)
    }

    #[must_use]
    pub const fn vsqrt_f32(sd: u8, sm: u8) -> [u16; 2] {
        fp_unary(0xeeb1, true, sd, sm)
    }

    #[must_use]
    pub const fn vcmp_f32(sn: u8, sm: u8) -> [u16; 2] {
        [
            0xeeb4 | ((sn as u16 & 1) << 6),
            ((sn as u16 >> 1) << 12) | 0x0a40 | ((sm as u16 & 1) << 5) | ((sm as u16 >> 1) & 0xf),
        ]
    }

    #[must_use]
    pub const fn vmrs(rt: u8) -> [u16; 2] {
        [0xeef1, ((rt as u16 & 0xf) << 12) | 0x0a10]
    }

    #[must_use]
    pub const fn vmsr(rt: u8) -> [u16; 2] {
        [0xeee1, ((rt as u16 & 0xf) << 12) | 0x0a10]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inherited_next_state_allocation_failure_is_typed_and_atomic() {
        let mut simulator = CortexM4Simulator::new();
        simulator
            .load(&core_encoding::program(&[core_encoding::movs(0, 7)]), 0x100)
            .unwrap();
        let before = simulator.get_state();

        // The inherited path performs one fallible whole-state clone before
        // fetch, then a second clone for the candidate Cortex-M4 state.  Fail
        // that candidate allocation deterministically.
        FAIL_STATE_CLONE_ON_CALL.with(|fail| fail.set(2));
        assert_eq!(
            simulator.step(),
            Err(CortexM4Error::Core(CortexM3Error::AllocationFailed))
        );
        assert_eq!(simulator.get_state(), before);
    }

    #[test]
    fn final_state_snapshot_allocation_failure_rolls_back_run() {
        let mut simulator = CortexM4Simulator::new();
        simulator
            .load(&core_encoding::program(&[core_encoding::bkpt(0)]), 0x100)
            .unwrap();
        let before = simulator.get_state();
        FAIL_FINAL_STATE_SNAPSHOT_ONCE.with(|fail| fail.set(true));
        assert_eq!(
            simulator.run(1),
            Err(CortexM4Error::Core(CortexM3Error::AllocationFailed))
        );
        assert_eq!(simulator.get_state(), before);
    }
}
