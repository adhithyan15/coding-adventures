//! DFF-backed Armv7E-M Cortex-M4 architectural gate simulator.
//!
//! The accepted Cortex-M3 gate machine remains the sole executor for inherited
//! instructions. This crate owns only the gate-decoded DSP/FPv4-SP delta and
//! clocks its 1,061 additional persistent bits through repository DFFs.

use core::fmt;

#[cfg(test)]
use std::cell::Cell;

use cortex_m3_gatelevel::CortexM3GateLevel;
pub use cortex_m3_gatelevel::{
    CortexM3Error, CortexM3State, InstructionClass as CortexM3InstructionClass, LR,
    MAX_RETAINED_BYTES, MAX_RETAINED_STEPS, MEMORY_SIZE, PC, SP,
};
use logic_gates::gates::{and_gate, not_gate, or_gate};
use logic_gates::sequential::{register, FlipFlopState};

mod floating;
pub mod gates;

const APSR_N: u32 = 1 << 31;
const APSR_Z: u32 = 1 << 30;
const APSR_C: u32 = 1 << 29;
const APSR_V: u32 = 1 << 28;
const APSR_Q: u32 = 1 << 27;
const APSR_GE_SHIFT: u32 = 16;
const APSR_GE_MASK: u32 = 0xf << APSR_GE_SHIFT;
const FPSCR_NZCV: u32 = 0xf000_0000;
const FPSCR_IDC: u32 = 1 << 7;
const FPSCR_IXC: u32 = 1 << 4;
const FPSCR_UFC: u32 = 1 << 3;
const FPSCR_OFC: u32 = 1 << 2;
const FPSCR_DZC: u32 = 1 << 1;
const FPSCR_IOC: u32 = 1;
const FPSCR_MASK: u32 =
    FPSCR_NZCV | FPSCR_IDC | FPSCR_IXC | FPSCR_UFC | FPSCR_OFC | FPSCR_DZC | FPSCR_IOC;

/// Exact persistent inventory: accepted M3 storage plus GE, FP configuration,
/// thirty-two binary32 registers, and FPSCR.
pub const FLIP_FLOP_COUNT: usize = cortex_m3_gatelevel::FLIP_FLOP_COUNT + 4 + 1 + 32 * 32 + 32;
pub const ESTIMATED_GATE_COUNT: usize =
    cortex_m3_gatelevel::ESTIMATED_GATE_COUNT + (4 + 1 + 32 * 32 + 32) * 6 + 262_144;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CortexM4State {
    pub core: CortexM3State,
    pub ge: u8,
    pub fpu_enabled: bool,
    pub s_registers: [u32; 32],
    pub fpscr: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepTrace {
    pub pc_before: u32,
    pub pc_after: u32,
    pub raw: [u8; 4],
    pub width: usize,
    pub mnemonic: &'static str,
    pub state_before: CortexM4State,
    pub state_after: CortexM4State,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionResult {
    pub stopped: bool,
    pub steps: usize,
    pub traces: Vec<StepTrace>,
    pub final_state: CortexM4State,
}

/// Complete gate-backed Cortex-M4 machine.
#[derive(Clone, Debug)]
pub struct CortexM4GateLevel {
    core: CortexM3GateLevel,
    ge: [u8; 4],
    fpu_enabled: u8,
    s_registers: [[u8; 32]; 32],
    fpscr: [u8; 32],
}

impl Default for CortexM4GateLevel {
    fn default() -> Self {
        Self::new()
    }
}

impl CortexM4GateLevel {
    #[must_use]
    pub fn new() -> Self {
        Self {
            core: CortexM3GateLevel::new(),
            ge: [0; 4],
            fpu_enabled: 0,
            s_registers: [[0; 32]; 32],
            fpscr: [0; 32],
        }
    }

    pub fn reset(&mut self) {
        self.core.reset();
        clock_bits(&mut self.ge, 0);
        clock_bit(&mut self.fpu_enabled, false);
        for register_bits in &mut self.s_registers {
            clock_bits(register_bits, 0);
        }
        clock_bits(&mut self.fpscr, 0);
    }

    pub fn try_get_state(&self) -> Result<CortexM4State, CortexM4Error> {
        Ok(CortexM4State {
            core: self.core.try_get_state()?,
            ge: read_bits(&self.ge) as u8,
            fpu_enabled: self.fpu_enabled == 1,
            s_registers: self.s_registers.map(|bits| read_bits(&bits) as u32),
            fpscr: read_bits(&self.fpscr) as u32,
        })
    }

    #[must_use]
    pub fn get_state(&self) -> CortexM4State {
        self.try_get_state()
            .expect("an infallible snapshot API cannot report host allocation failure")
    }

    pub fn restore(&mut self, state: CortexM4State) -> Result<(), CortexM4Error> {
        validate_extension_state(&state)?;
        let mut candidate = self.core.clone();
        candidate.restore(clone_core(&state.core)?)?;
        self.core.restore(clone_core(&state.core)?)?;
        self.clock_extension(&state);
        Ok(())
    }

    pub fn load(&mut self, program: &[u8], origin: u32) -> Result<(), CortexM4Error> {
        let mut candidate = self.core.clone();
        candidate.load(program, origin)?;
        let enabled = self.fpu_enabled == 1;
        self.core.restore(candidate.try_get_state()?)?;
        clock_bits(&mut self.ge, 0);
        clock_bit(&mut self.fpu_enabled, enabled);
        for register_bits in &mut self.s_registers {
            clock_bits(register_bits, 0);
        }
        clock_bits(&mut self.fpscr, 0);
        Ok(())
    }

    pub fn set_fpu_enabled(&mut self, enabled: bool) {
        clock_bit(&mut self.fpu_enabled, enabled);
    }

    #[must_use]
    pub const fn fpu_enabled(&self) -> bool {
        self.fpu_enabled == 1
    }

    pub fn read_register(&self, index: u8) -> Result<u32, CortexM4Error> {
        self.core.read_register(index).map_err(Into::into)
    }

    pub fn write_register(&mut self, index: u8, value: u32) -> Result<(), CortexM4Error> {
        self.core.write_register(index, value).map_err(Into::into)
    }

    pub fn read_s_register(&self, index: u8) -> Result<u32, CortexM4Error> {
        self.s_registers
            .get(usize::from(index))
            .map(|bits| read_bits(bits) as u32)
            .ok_or(CortexM4Error::InvalidSRegister { index })
    }

    pub fn write_s_register(&mut self, index: u8, value: u32) -> Result<(), CortexM4Error> {
        let target = self
            .s_registers
            .get_mut(usize::from(index))
            .ok_or(CortexM4Error::InvalidSRegister { index })?;
        clock_bits(target, u64::from(value));
        Ok(())
    }

    pub fn read_memory(&self, address: u32, width: usize) -> Result<u32, CortexM4Error> {
        self.core.read_memory(address, width).map_err(Into::into)
    }

    pub fn write_memory(
        &mut self,
        address: u32,
        width: usize,
        value: u32,
    ) -> Result<(), CortexM4Error> {
        self.core
            .write_memory(address, width, value)
            .map_err(Into::into)
    }

    pub fn signal_event(&mut self) {
        self.core.signal_event();
    }

    pub fn signal_interrupt(&mut self) {
        self.core.signal_interrupt();
    }

    pub fn step(&mut self) -> Result<StepTrace, CortexM4Error> {
        let before = self.try_get_state()?;
        if before.core.halted {
            return Err(CortexM3Error::Halted.into());
        }
        if before.core.sleeping {
            return Err(CortexM3Error::Sleeping.into());
        }
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
            let second = second.expect("all extension instructions are 32-bit");
            validate_extension_operands(first, second, class, pc)?;
            let mut next = clone_state(&before)?;
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
            self.commit(&next)?;
            return Ok(trace);
        }

        let mut inherited = self.core.clone();
        let core_trace = inherited.step()?;
        let mut next = clone_state(&before)?;
        next.core = clone_core(&core_trace.state_after)?;
        integrate_inherited_ge(&before, &mut next, first, second, core_trace.mnemonic);
        let trace = StepTrace {
            pc_before: core_trace.pc_before,
            pc_after: core_trace.pc_after,
            raw: core_trace.raw,
            width: core_trace.width,
            mnemonic: core_trace.mnemonic,
            state_before: before,
            state_after: clone_state(&next)?,
        };
        self.commit(&next)?;
        Ok(trace)
    }

    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, CortexM4Error> {
        check_retention(max_steps)?;
        preflight_snapshots(max_steps)?;
        let before = self.try_get_state()?;
        let mut traces = Vec::new();
        traces
            .try_reserve_exact(max_steps)
            .map_err(|_| CortexM4Error::Core(CortexM3Error::AllocationFailed))?;
        for _ in 0..max_steps {
            let stopped = match self.step() {
                Ok(trace) => {
                    let stopped = trace.state_after.core.halted || trace.state_after.core.sleeping;
                    traces.push(trace);
                    stopped
                }
                Err(error) => {
                    self.restore(before)?;
                    return Err(error);
                }
            };
            if stopped {
                let final_state = match snapshot_final_state(self) {
                    Ok(state) => state,
                    Err(error) => {
                        self.restore(before)?;
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
        self.restore(before)?;
        Err(CortexM3Error::StepLimitExceeded { limit: max_steps }.into())
    }

    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u32,
        max_steps: usize,
    ) -> Result<ExecutionResult, CortexM4Error> {
        check_retention(max_steps)?;
        preflight_snapshots(max_steps)?;
        let before = self.try_get_state()?;
        self.load(program, origin)?;
        match self.run(max_steps) {
            Ok(result) => Ok(result),
            Err(error) => {
                self.restore(before)?;
                Err(error)
            }
        }
    }

    #[must_use]
    pub const fn flip_flop_count(&self) -> usize {
        FLIP_FLOP_COUNT
    }

    #[must_use]
    pub const fn gate_count(&self) -> usize {
        ESTIMATED_GATE_COUNT
    }

    fn commit(&mut self, state: &CortexM4State) -> Result<(), CortexM4Error> {
        validate_extension_state(state)?;
        self.core.restore(clone_core(&state.core)?)?;
        self.clock_extension(state);
        Ok(())
    }

    fn clock_extension(&mut self, state: &CortexM4State) {
        clock_bits(&mut self.ge, u64::from(state.ge));
        clock_bit(&mut self.fpu_enabled, state.fpu_enabled);
        for (target, value) in self.s_registers.iter_mut().zip(state.s_registers) {
            clock_bits(target, u64::from(value));
        }
        clock_bits(&mut self.fpscr, u64::from(state.fpscr));
    }
}

pub fn classify(first: u16, second: Option<u16>) -> Result<InstructionClass, CortexM4Error> {
    if let Some(class) = classify_extension(first, second) {
        Ok(class)
    } else {
        cortex_m3_gatelevel::classify(first, second)
            .map(InstructionClass::Core)
            .map_err(Into::into)
    }
}

fn classify_extension(first: u16, second: Option<u16>) -> Option<InstructionClass> {
    let second = second?;
    let dsp = gates::masked_eq16(first, 0xfff0, 0xfa80);
    if dsp == 1 {
        if gates::masked_eq16(second, 0xf0f0, 0xf080) == 1
            || gates::masked_eq16(second, 0xf0f0, 0xf0a0) == 1
        {
            return Some(InstructionClass::Saturating);
        }
        if gates::masked_eq16(second, 0xf0f0, 0xf000) == 1
            || gates::masked_eq16(second, 0xf0f0, 0xf040) == 1
        {
            return Some(InstructionClass::ParallelBytes);
        }
    }
    let saturation = or_gate(
        gates::masked_eq16(first, 0xffd0, 0xf300),
        gates::masked_eq16(first, 0xffd0, 0xf380),
    );
    if and_gate(saturation, gates::masked_eq16(second, 0x8020, 0)) == 1 {
        return Some(InstructionClass::Saturating);
    }
    if and_gate(
        gates::masked_eq16(first, 0xfff0, 0xfb20),
        gates::masked_eq16(second, 0x00f0, 0),
    ) == 1
    {
        return Some(InstructionClass::DspMultiply);
    }
    let binary = [
        (0xffb0, 0xee30, 0x0f50, 0x0a00),
        (0xffb0, 0xee30, 0x0f50, 0x0a40),
        (0xffb0, 0xee20, 0x0f50, 0x0a00),
        (0xffb0, 0xee80, 0x0f50, 0x0a00),
    ]
    .into_iter()
    .fold(0, |line, (fm, fv, sm, sv)| {
        or_gate(
            line,
            and_gate(
                gates::masked_eq16(first, fm, fv),
                gates::masked_eq16(second, sm, sv),
            ),
        )
    });
    if binary == 1 {
        return Some(InstructionClass::FloatingPointBinary);
    }
    let unary_first = or_gate(
        gates::masked_eq16(first, 0xffbf, 0xeeb0),
        gates::masked_eq16(first, 0xffbf, 0xeeb1),
    );
    let unary_second = or_gate(
        gates::masked_eq16(second, 0x0fd0, 0x0a40),
        gates::masked_eq16(second, 0x0fd0, 0x0ac0),
    );
    if and_gate(unary_first, unary_second) == 1 {
        return Some(InstructionClass::FloatingPointUnary);
    }
    if and_gate(
        gates::masked_eq16(first, 0xffbf, 0xeeb4),
        gates::masked_eq16(second, 0x0fd0, 0x0a40),
    ) == 1
    {
        return Some(InstructionClass::FloatingPointCompare);
    }
    if (gates::masked_eq16(first, 0xffff, 0xeef1) == 1
        || gates::masked_eq16(first, 0xffff, 0xeee1) == 1)
        && gates::masked_eq16(second, 0x0fff, 0x0a10) == 1
    {
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
        InstructionClass::Core(_) => unreachable!("inherited classes use the M3 gate executor"),
    }
}

fn execute_saturating(
    state: &mut CortexM4State,
    first: u16,
    second: u16,
    pc: u32,
) -> Result<&'static str, CortexM4Error> {
    if gates::masked_eq16(first, 0xfff0, 0xfa80) == 1 {
        let rd = gates::field_u16(second, 8, 4);
        let rn = gates::field_u16(first, 0, 4);
        let rm = gates::field_u16(second, 0, 4);
        require_general(first, second, pc, &[rd, rn, rm])?;
        let subtract = gates::masked_eq16(second, 0x00f0, 0x00a0);
        let (value, saturated) = gates::saturating_add(
            read_core(&state.core, rm)?,
            read_core(&state.core, rn)?,
            subtract,
        );
        write_core(&mut state.core, rd, value)?;
        sticky_q(state, saturated);
        return Ok(if subtract == 1 { "qsub" } else { "qadd" });
    }
    let rd = gates::field_u16(second, 8, 4);
    let rn = gates::field_u16(first, 0, 4);
    require_general(first, second, pc, &[rd, rn])?;
    let shift = u32::from((gates::field_u16(second, 12, 3) << 2) | gates::field_u16(second, 6, 2));
    let source = read_core(&state.core, rn)?;
    let arithmetic = gates::field_u16(first, 5, 1);
    let right_amount = gates::select_32(shift, 31, gates::is_zero_32(shift));
    let shifted = gates::select_32(
        gates::shift_left_32(source, shift),
        gates::shift_arithmetic_right_32(source, right_amount),
        arithmetic,
    );
    if gates::masked_eq16(first, 0x0080, 0) == 1 {
        let (value, saturated) =
            gates::signed_saturate(shifted, u32::from(gates::field_u16(second, 0, 5)) + 1);
        write_core(&mut state.core, rd, value)?;
        sticky_q(state, saturated);
        Ok("ssat")
    } else {
        let (value, saturated) =
            gates::unsigned_saturate(shifted, u32::from(gates::field_u16(second, 0, 5)));
        write_core(&mut state.core, rd, value)?;
        sticky_q(state, saturated);
        Ok("usat")
    }
}

fn execute_parallel(
    state: &mut CortexM4State,
    first: u16,
    second: u16,
    pc: u32,
) -> Result<&'static str, CortexM4Error> {
    let rd = gates::field_u16(second, 8, 4);
    let rn = gates::field_u16(first, 0, 4);
    let rm = gates::field_u16(second, 0, 4);
    require_general(first, second, pc, &[rd, rn, rm])?;
    let unsigned = gates::field_u16(second, 6, 1);
    let (value, ge) = gates::parallel_add8(
        read_core(&state.core, rn)?,
        read_core(&state.core, rm)?,
        unsigned,
    );
    write_core(&mut state.core, rd, value)?;
    state.ge = ge;
    Ok(if unsigned == 1 { "uadd8" } else { "sadd8" })
}

fn execute_dsp_multiply(
    state: &mut CortexM4State,
    first: u16,
    second: u16,
    pc: u32,
) -> Result<&'static str, CortexM4Error> {
    let rd = gates::field_u16(second, 8, 4);
    let rn = gates::field_u16(first, 0, 4);
    let rm = gates::field_u16(second, 0, 4);
    let ra = gates::field_u16(second, 12, 4);
    require_general(first, second, pc, &[rd, rn, rm])?;
    let accumulator_absent = gates::masked_eq16(u16::from(ra), 0xf, u16::from(PC));
    if accumulator_absent == 0 {
        require_general(first, second, pc, &[ra])?;
    }
    let add_acc = not_gate(accumulator_absent);
    let accumulator = if accumulator_absent == 1 {
        0
    } else {
        read_core(&state.core, ra)?
    };
    let (value, overflow) = gates::dual_multiply_add(
        read_core(&state.core, rn)?,
        read_core(&state.core, rm)?,
        accumulator,
        add_acc,
    );
    write_core(&mut state.core, rd, value)?;
    sticky_q(state, overflow);
    Ok(if add_acc == 1 { "smlad" } else { "smuad" })
}

fn execute_fp_binary(
    state: &mut CortexM4State,
    first: u16,
    second: u16,
    pc: u32,
) -> Result<&'static str, CortexM4Error> {
    require_fpu(state, pc)?;
    let operation = if gates::masked_eq16(first, 0xffb0, 0xee30) == 1 {
        if gates::masked_eq16(second, 0x0040, 0x0040) == 1 {
            floating::BinaryOperation::Subtract
        } else {
            floating::BinaryOperation::Add
        }
    } else if gates::masked_eq16(first, 0xffb0, 0xee20) == 1 {
        floating::BinaryOperation::Multiply
    } else if gates::masked_eq16(first, 0xffb0, 0xee80) == 1 {
        floating::BinaryOperation::Divide
    } else {
        return Err(unknown(first, second, pc));
    };
    let result = floating::binary(
        state.s_registers[usize::from(fp_sn(first, second))],
        state.s_registers[usize::from(fp_sm(second))],
        operation,
    );
    apply_fp_status(state, result.status);
    state.s_registers[usize::from(fp_sd(first, second))] = result.bits;
    Ok(match operation {
        floating::BinaryOperation::Add => "vadd.f32",
        floating::BinaryOperation::Subtract => "vsub.f32",
        floating::BinaryOperation::Multiply => "vmul.f32",
        floating::BinaryOperation::Divide => "vdiv.f32",
    })
}

fn execute_fp_unary(
    state: &mut CortexM4State,
    first: u16,
    second: u16,
    pc: u32,
) -> Result<&'static str, CortexM4Error> {
    require_fpu(state, pc)?;
    let source = state.s_registers[usize::from(fp_sm(second))];
    let absolute = and_gate(
        gates::masked_eq16(first, 0x000f, 0),
        gates::masked_eq16(second, 0x0080, 0x0080),
    );
    let negate = and_gate(
        gates::masked_eq16(first, 0x000f, 1),
        gates::masked_eq16(second, 0x0080, 0),
    );
    let square_root = and_gate(
        gates::masked_eq16(first, 0x000f, 1),
        gates::masked_eq16(second, 0x0080, 0x0080),
    );
    let (result, mnemonic) = if absolute == 1 {
        let mut wires = gates::u32_to_bits(source);
        wires[31] = 0;
        (gates::bits_to_u32(&wires), "vabs.f32")
    } else if negate == 1 {
        let mut wires = gates::u32_to_bits(source);
        wires[31] = logic_gates::gates::not_gate(wires[31]);
        (gates::bits_to_u32(&wires), "vneg.f32")
    } else if square_root == 1 {
        let result = floating::square_root(source);
        apply_fp_status(state, result.status);
        (result.bits, "vsqrt.f32")
    } else if gates::masked_eq16(first, 0x000f, 0) == 1
        && gates::masked_eq16(second, 0x0080, 0) == 1
    {
        (source, "vmov.f32")
    } else {
        return Err(unknown(first, second, pc));
    };
    state.s_registers[usize::from(fp_sd(first, second))] = result;
    Ok(mnemonic)
}

fn execute_fp_compare(
    state: &mut CortexM4State,
    first: u16,
    second: u16,
    pc: u32,
) -> Result<&'static str, CortexM4Error> {
    require_fpu(state, pc)?;
    let (nzcv, invalid, denormal) = floating::compare(
        state.s_registers[usize::from(fp_sd(first, second))],
        state.s_registers[usize::from(fp_sm(second))],
    );
    state.fpscr = (state.fpscr & !FPSCR_NZCV) | nzcv;
    if invalid == 1 {
        state.fpscr |= FPSCR_IOC;
    }
    if denormal == 1 {
        state.fpscr |= FPSCR_IDC;
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
    let rt = gates::field_u16(second, 12, 4);
    require_general(first, second, pc, &[rt])?;
    if gates::masked_eq16(first, 0xffff, 0xeef1) == 1 {
        write_core(&mut state.core, rt, state.fpscr)?;
        Ok("vmrs")
    } else {
        let value = read_core(&state.core, rt)?;
        if value & !FPSCR_MASK != 0 {
            return Err(CortexM4Error::UnsupportedFpscr { value });
        }
        state.fpscr = value;
        Ok("vmsr")
    }
}

fn apply_fp_status(state: &mut CortexM4State, status: floating::Status) {
    if status.invalid == 1 {
        state.fpscr |= FPSCR_IOC;
    }
    if status.divide_by_zero == 1 {
        state.fpscr |= FPSCR_DZC;
    }
    if status.overflow == 1 {
        state.fpscr |= FPSCR_OFC | FPSCR_IXC;
    }
    if status.underflow == 1 {
        state.fpscr |= FPSCR_UFC;
    }
    if status.inexact == 1 {
        state.fpscr |= FPSCR_IXC;
    }
    if status.input_denormal == 1 {
        state.fpscr |= FPSCR_IDC;
    }
}

fn validate_extension_operands(
    first: u16,
    second: u16,
    class: InstructionClass,
    pc: u32,
) -> Result<(), CortexM4Error> {
    let rd = gates::field_u16(second, 8, 4);
    let rn = gates::field_u16(first, 0, 4);
    let rm = gates::field_u16(second, 0, 4);
    match class {
        InstructionClass::Saturating if first & 0xfff0 == 0xfa80 => {
            require_general(first, second, pc, &[rd, rn, rm])
        }
        InstructionClass::Saturating => require_general(first, second, pc, &[rd, rn]),
        InstructionClass::ParallelBytes => require_general(first, second, pc, &[rd, rn, rm]),
        InstructionClass::DspMultiply => {
            let ra = gates::field_u16(second, 12, 4);
            require_general(first, second, pc, &[rd, rn, rm])?;
            if ra == PC {
                Ok(())
            } else {
                require_general(first, second, pc, &[ra])
            }
        }
        InstructionClass::FloatingPointStatus => {
            require_general(first, second, pc, &[gates::field_u16(second, 12, 4)])
        }
        InstructionClass::FloatingPointBinary
        | InstructionClass::FloatingPointUnary
        | InstructionClass::FloatingPointCompare => Ok(()),
        InstructionClass::Core(_) => unreachable!("core operands belong to M3"),
    }
}

fn integrate_inherited_ge(
    before: &CortexM4State,
    next: &mut CortexM4State,
    first: u16,
    second: Option<u16>,
    mnemonic: &str,
) {
    let Some(second) = second else { return };
    if second as u8 > 3 {
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

fn sticky_q(state: &mut CortexM4State, input: u8) {
    let old = u8::from(state.core.apsr & APSR_Q != 0);
    let q = or_gate(old, input);
    state.core.apsr = (state.core.apsr & !APSR_Q) | (u32::from(q) << 27);
}

fn read_core(state: &CortexM3State, index: u8) -> Result<u32, CortexM4Error> {
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

fn write_core(state: &mut CortexM3State, index: u8, value: u32) -> Result<(), CortexM4Error> {
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

fn instruction_width(first: u16) -> usize {
    if matches!(first >> 11, 0b11101..=0b11111) {
        4
    } else {
        2
    }
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

fn fp_sd(first: u16, second: u16) -> u8 {
    gates::field_u16(second, 12, 4) * 2 + gates::field_u16(first, 6, 1)
}

fn fp_sn(first: u16, second: u16) -> u8 {
    gates::field_u16(first, 0, 4) * 2 + gates::field_u16(second, 7, 1)
}

fn fp_sm(second: u16) -> u8 {
    gates::field_u16(second, 0, 4) * 2 + gates::field_u16(second, 5, 1)
}

fn require_fpu(state: &CortexM4State, pc: u32) -> Result<(), CortexM4Error> {
    if state.fpu_enabled {
        Ok(())
    } else {
        Err(CortexM4Error::FloatingPointUnavailable { pc })
    }
}

fn require_general(
    first: u16,
    second: u16,
    pc: u32,
    registers: &[u8],
) -> Result<(), CortexM4Error> {
    if registers
        .iter()
        .all(|register| !matches!(*register, SP | PC))
    {
        Ok(())
    } else {
        Err(unknown(first, second, pc))
    }
}

fn unknown(first: u16, second: u16, pc: u32) -> CortexM4Error {
    CortexM4Error::UnknownInstruction32 { first, second, pc }
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
    let mut core = CortexM3GateLevel::new();
    core.restore(clone_core(&state.core)?)?;
    Ok(())
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
        state_after: clone_state(next)?,
    })
}

fn clone_state(state: &CortexM4State) -> Result<CortexM4State, CortexM4Error> {
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
        core: clone_core(&state.core)?,
        ge: state.ge,
        fpu_enabled: state.fpu_enabled,
        s_registers: state.s_registers,
        fpscr: state.fpscr,
    })
}

fn snapshot_final_state(machine: &CortexM4GateLevel) -> Result<CortexM4State, CortexM4Error> {
    #[cfg(test)]
    if FAIL_FINAL_STATE_SNAPSHOT_ONCE.with(|fail| fail.replace(false)) {
        return Err(CortexM4Error::Core(CortexM3Error::AllocationFailed));
    }
    machine.try_get_state()
}

fn clone_core(state: &CortexM3State) -> Result<CortexM3State, CortexM4Error> {
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

fn preflight_snapshots(max_steps: usize) -> Result<(), CortexM4Error> {
    let snapshots = max_steps
        .checked_mul(2)
        .and_then(|count| count.checked_add(4))
        .ok_or(CortexM4Error::Core(CortexM3Error::AllocationFailed))?;
    let bytes = snapshots
        .checked_mul(MEMORY_SIZE)
        .ok_or(CortexM4Error::Core(CortexM3Error::AllocationFailed))?;
    if bytes > MAX_RETAINED_BYTES + 4 * MEMORY_SIZE {
        return Err(CortexM3Error::RetentionLimitExceeded {
            requested: max_steps,
            limit: MAX_RETAINED_STEPS,
        }
        .into());
    }
    let mut reservation = Vec::<u8>::new();
    reservation
        .try_reserve_exact(bytes)
        .map_err(|_| CortexM4Error::Core(CortexM3Error::AllocationFailed))?;
    Ok(())
}

fn stable(q: u8) -> FlipFlopState {
    FlipFlopState {
        master_q: q,
        master_q_bar: q ^ 1,
        slave_q: q,
        slave_q_bar: q ^ 1,
    }
}

fn clock_bits<const N: usize>(target: &mut [u8; N], value: u64) {
    let input: [u8; N] = std::array::from_fn(|bit| ((value >> bit) & 1) as u8);
    let mut states: [FlipFlopState; N] = std::array::from_fn(|bit| stable(target[bit]));
    register(&input, 0, &mut states);
    target.copy_from_slice(&register(&input, 1, &mut states));
}

fn clock_bit(target: &mut u8, value: bool) {
    let input = [u8::from(value)];
    let mut state = [stable(*target)];
    register(&input, 0, &mut state);
    *target = register(&input, 1, &mut state)[0];
}

fn read_bits(bits: &[u8]) -> u64 {
    bits.iter()
        .copied()
        .enumerate()
        .fold(0, |value, (bit, q)| value | (u64::from(q) << bit))
}

/// Little-endian Cortex-M4 extension encodings used by examples and tests.
pub mod encoding {
    pub use cortex_m4_encoding::*;

    mod cortex_m4_encoding {
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
                ((sn as u16 >> 1) << 12)
                    | 0x0a40
                    | ((sm as u16 & 1) << 5)
                    | ((sm as u16 >> 1) & 0xf),
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
}

#[cfg(test)]
thread_local! {
    static FAIL_FINAL_STATE_SNAPSHOT_ONCE: Cell<bool> = const { Cell::new(false) };
    static FAIL_STATE_CLONE_ON_CALL: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
mod allocation_tests {
    use super::*;

    fn image(words: &[u16]) -> Vec<u8> {
        words.iter().flat_map(|word| word.to_le_bytes()).collect()
    }

    #[test]
    fn extension_candidate_allocation_failure_is_typed_and_atomic() {
        let mut machine = CortexM4GateLevel::new();
        machine
            .load(&image(&encoding::qadd(0, 1, 2)), 0x100)
            .unwrap();
        let before = machine.get_state();
        FAIL_STATE_CLONE_ON_CALL.with(|fail| fail.set(1));
        assert_eq!(
            machine.step(),
            Err(CortexM4Error::Core(CortexM3Error::AllocationFailed))
        );
        assert_eq!(machine.get_state(), before);
    }

    #[test]
    fn final_snapshot_allocation_failure_rolls_back_run() {
        let mut machine = CortexM4GateLevel::new();
        machine.load(&image(&[0xbe00]), 0x100).unwrap();
        let before = machine.get_state();
        FAIL_FINAL_STATE_SNAPSHOT_ONCE.with(|fail| fail.set(true));
        assert_eq!(
            machine.run(1),
            Err(CortexM4Error::Core(CortexM3Error::AllocationFailed))
        );
        assert_eq!(machine.get_state(), before);
    }

    #[test]
    fn run_allocation_failure_after_one_committed_step_restores_entry_state() {
        let mut machine = CortexM4GateLevel::new();
        machine.load(&image(&[0x2001, 0xbe00]), 0x100).unwrap();
        machine.set_fpu_enabled(true);
        machine.write_s_register(31, 0x7fc0_1234).unwrap();
        let before = machine.get_state();

        // Each inherited step clones its complete candidate twice. The third
        // clone is therefore reached only after MOVS has committed and the
        // second instruction has begun settling.
        FAIL_STATE_CLONE_ON_CALL.with(|fail| fail.set(3));
        assert_eq!(
            machine.run(2),
            Err(CortexM4Error::Core(CortexM3Error::AllocationFailed))
        );
        assert_eq!(machine.get_state(), before);
    }

    #[test]
    fn execute_allocation_failure_after_one_committed_step_restores_call_entry_state() {
        let mut machine = CortexM4GateLevel::new();
        machine.load(&image(&[0xbe00]), 0x180).unwrap();
        machine.set_fpu_enabled(true);
        machine.write_s_register(31, 0x7fc0_5678).unwrap();
        machine.write_register(4, 0xfeed_beef).unwrap();
        let before = machine.get_state();

        FAIL_STATE_CLONE_ON_CALL.with(|fail| fail.set(3));
        assert_eq!(
            machine.execute(&image(&[0x2001, 0xbe00]), 0x100, 2),
            Err(CortexM4Error::Core(CortexM3Error::AllocationFailed))
        );
        assert_eq!(machine.get_state(), before);
    }
}
