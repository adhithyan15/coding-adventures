//! Exact 64 KiB Spec 07w functional-machine lifecycle.

use crate::decode::decode;
use crate::execute::{exec_one, Flow};
use crate::memory::Memory;
use crate::state::{CpuState, Flags, Reg};
use crate::trap::Trap;

/// Normative flat x86-64 memory size.
pub const MEMORY_SIZE: usize = 65_536;

/// Complete architectural state at an accepted instruction boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct X86State {
    pub rip: u64,
    pub gpr: [u64; 16],
    pub rflags: u64,
    pub memory: Vec<u8>,
    pub halted: bool,
    pub loaded_origin: u64,
    pub loaded_len: usize,
}

/// Checked lifecycle failure. Every variant is transition-atomic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum X86Error {
    Halted,
    InvalidState(String),
    ProgramOutOfRange { origin: u64, length: usize },
    TruncatedInstruction { rip: u64 },
    UnknownInstruction { rip: u64, opcode: u8 },
    IllegalInstruction { rip: u64 },
    DivideError { rip: u64 },
    StepLimitExceeded { max_steps: usize },
    UnsupportedCall { rip: u64 },
}

impl std::fmt::Display for X86Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Halted => f.write_str("CPU is halted"),
            Self::InvalidState(message) => f.write_str(message),
            Self::ProgramOutOfRange { origin, length } => write!(
                f,
                "program of {length} bytes at {origin:#x} exceeds 64 KiB memory"
            ),
            Self::TruncatedInstruction { rip } => {
                write!(
                    f,
                    "instruction at {rip:#x} crosses installed program bounds"
                )
            }
            Self::UnknownInstruction { rip, opcode } => {
                write!(f, "unknown opcode {opcode:#04x} at {rip:#x}")
            }
            Self::IllegalInstruction { rip } => write!(f, "illegal instruction at {rip:#x}"),
            Self::DivideError { rip } => write!(f, "divide error at {rip:#x}"),
            Self::StepLimitExceeded { max_steps } => {
                write!(f, "step limit {max_steps} exceeded")
            }
            Self::UnsupportedCall { rip } => {
                write!(
                    f,
                    "host call at {rip:#x} is outside the architectural machine"
                )
            }
        }
    }
}

impl std::error::Error for X86Error {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct X86StepTrace {
    pub rip_before: u64,
    pub rip_after: u64,
    pub raw: Vec<u8>,
    pub mnemonic: String,
    pub state_before: X86State,
    pub state_after: X86State,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct X86ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub traces: Vec<X86StepTrace>,
    pub final_state: X86State,
}

/// Functional Spec 07w machine, independent of the backend runtime harness.
#[derive(Debug, Clone)]
pub struct X86FunctionalSimulator {
    cpu: CpuState,
    memory: Memory,
    halted: bool,
    loaded_origin: u64,
    loaded_len: usize,
}

impl Default for X86FunctionalSimulator {
    fn default() -> Self {
        Self::new()
    }
}

impl X86FunctionalSimulator {
    pub fn new() -> Self {
        let mut cpu = CpuState::default();
        cpu.set(Reg::Rsp, MEMORY_SIZE as u64 - 8);
        Self {
            cpu,
            memory: Memory::new_wrapping(MEMORY_SIZE),
            halted: false,
            loaded_origin: 0,
            loaded_len: 0,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn get_state(&self) -> X86State {
        X86State {
            rip: self.cpu.rip,
            gpr: self.cpu.gpr,
            rflags: flags_to_word(self.cpu.flags),
            memory: self.memory.snapshot(),
            halted: self.halted,
            loaded_origin: self.loaded_origin,
            loaded_len: self.loaded_len,
        }
    }

    pub fn restore(&mut self, state: &X86State) -> Result<(), X86Error> {
        validate_state(state)?;
        let mut memory = Memory::new_wrapping(MEMORY_SIZE);
        memory
            .restore_bytes(&state.memory)
            .map_err(|_| X86Error::InvalidState("memory restore failed".into()))?;
        self.cpu = CpuState {
            gpr: state.gpr,
            rip: state.rip,
            flags: word_to_flags(state.rflags),
            xmm: [0; 16],
        };
        self.memory = memory;
        self.halted = state.halted;
        self.loaded_origin = state.loaded_origin;
        self.loaded_len = state.loaded_len;
        Ok(())
    }

    pub fn load_checked(&mut self, program: &[u8]) -> Result<(), X86Error> {
        self.load_at_checked(program, 0)
    }

    pub fn load_at_checked(&mut self, program: &[u8], origin: u64) -> Result<(), X86Error> {
        let end = origin
            .checked_add(program.len() as u64)
            .filter(|end| *end <= MEMORY_SIZE as u64)
            .ok_or(X86Error::ProgramOutOfRange {
                origin,
                length: program.len(),
            })?;
        let mut next = Self::new();
        for (offset, byte) in program.iter().copied().enumerate() {
            next.memory
                .store(origin + offset as u64, 1, u64::from(byte))
                .expect("validated wrapping memory");
        }
        next.cpu.rip = origin;
        next.loaded_origin = origin;
        next.loaded_len = (end - origin) as usize;
        *self = next;
        Ok(())
    }

    pub fn read_register_checked(&self, index: usize) -> Result<u64, X86Error> {
        self.cpu
            .gpr
            .get(index)
            .copied()
            .ok_or_else(|| X86Error::InvalidState(format!("register {index} is outside r0-r15")))
    }

    pub fn write_register_checked(&mut self, index: usize, value: u64) -> Result<(), X86Error> {
        let register =
            self.cpu.gpr.get_mut(index).ok_or_else(|| {
                X86Error::InvalidState(format!("register {index} is outside r0-r15"))
            })?;
        *register = value;
        Ok(())
    }

    pub fn read_byte(&self, address: u64) -> u8 {
        self.memory
            .load(address, 1)
            .expect("wrapping memory cannot fault") as u8
    }

    pub fn write_byte(&mut self, address: u64, value: u8) {
        self.memory
            .store(address, 1, u64::from(value))
            .expect("wrapping memory cannot fault");
    }

    pub fn step_checked(&mut self) -> Result<X86StepTrace, X86Error> {
        if self.halted {
            return Err(X86Error::Halted);
        }
        let before = self.get_state();
        let rip = before.rip;
        let available = installed_bytes(&before)?;
        let fetch_len = available.min(15);
        let bytes: Vec<u8> = (0..fetch_len)
            .map(|offset| self.read_byte(rip + offset as u64))
            .collect();
        let decoded = decode(&bytes, 0).map_err(|trap| map_decode_error(trap, rip))?;
        let raw = bytes[..decoded.len].to_vec();
        let mut cpu = self.cpu.clone();
        let mut memory = self.memory.clone();
        let next_ip = rip.wrapping_add(decoded.len as u64);
        let flow = exec_one(&mut cpu, &mut memory, &decoded.instr, rip, next_ip, 0)
            .map_err(|trap| map_exec_error(trap, rip))?;
        let mnemonic = format!("{:?}", decoded.instr);
        match flow {
            Flow::Next => cpu.rip = next_ip,
            Flow::Jump(target) => cpu.rip = target,
            Flow::Call { target, .. } => {
                let stack = cpu.get(Reg::Rsp).wrapping_sub(8);
                memory
                    .store(stack, 8, next_ip)
                    .map_err(|trap| map_exec_error(trap, rip))?;
                cpu.set(Reg::Rsp, stack);
                cpu.rip = target;
            }
            Flow::Ret => {
                let stack = cpu.get(Reg::Rsp);
                cpu.rip = memory
                    .load(stack, 8)
                    .map_err(|trap| map_exec_error(trap, rip))?;
                cpu.set(Reg::Rsp, stack.wrapping_add(8));
            }
            Flow::RetAdjust(bytes) => {
                let stack = cpu.get(Reg::Rsp);
                cpu.rip = memory
                    .load(stack, 8)
                    .map_err(|trap| map_exec_error(trap, rip))?;
                cpu.set(
                    Reg::Rsp,
                    stack.wrapping_add(8).wrapping_add(u64::from(bytes)),
                );
            }
            Flow::Trap => return Err(X86Error::IllegalInstruction { rip }),
            Flow::Halt => {
                cpu.rip = next_ip;
                self.halted = true;
            }
        }
        self.cpu = cpu;
        self.memory = memory;
        let after = self.get_state();
        Ok(X86StepTrace {
            rip_before: rip,
            rip_after: after.rip,
            raw,
            mnemonic,
            state_before: before,
            state_after: after,
        })
    }

    pub fn run_loaded_checked(&mut self, max_steps: usize) -> Result<X86ExecutionResult, X86Error> {
        let before = self.clone();
        let mut traces = Vec::new();
        for _ in 0..max_steps {
            match self.step_checked() {
                Ok(trace) => {
                    let halted = trace.state_after.halted;
                    traces.push(trace);
                    if halted {
                        return Ok(X86ExecutionResult {
                            halted: true,
                            steps: traces.len(),
                            traces,
                            final_state: self.get_state(),
                        });
                    }
                }
                Err(error) => {
                    *self = before;
                    return Err(error);
                }
            }
        }
        *self = before;
        Err(X86Error::StepLimitExceeded { max_steps })
    }

    pub fn run_checked(
        &mut self,
        program: &[u8],
        max_steps: usize,
    ) -> Result<X86ExecutionResult, X86Error> {
        let before = self.clone();
        self.load_checked(program)?;
        match self.run_loaded_checked(max_steps) {
            Ok(result) => Ok(result),
            Err(error) => {
                *self = before;
                Err(error)
            }
        }
    }
}

fn installed_bytes(state: &X86State) -> Result<usize, X86Error> {
    let end = state.loaded_origin + state.loaded_len as u64;
    if state.rip < state.loaded_origin || state.rip >= end {
        return Err(X86Error::TruncatedInstruction { rip: state.rip });
    }
    Ok((end - state.rip) as usize)
}

fn validate_state(state: &X86State) -> Result<(), X86Error> {
    if state.memory.len() != MEMORY_SIZE {
        return Err(X86Error::InvalidState(format!(
            "memory must contain exactly {MEMORY_SIZE} bytes"
        )));
    }
    if state.loaded_origin as usize > MEMORY_SIZE
        || state.loaded_origin as usize + state.loaded_len > MEMORY_SIZE
    {
        return Err(X86Error::InvalidState(
            "installed program range is invalid".into(),
        ));
    }
    Ok(())
}

fn flags_to_word(flags: Flags) -> u64 {
    u64::from(flags.cf)
        | (u64::from(flags.pf) << 2)
        | (u64::from(flags.zf) << 6)
        | (u64::from(flags.sf) << 7)
        | (u64::from(flags.of) << 11)
}

fn word_to_flags(word: u64) -> Flags {
    Flags {
        cf: word & 1 != 0,
        pf: word & (1 << 2) != 0,
        af: false,
        zf: word & (1 << 6) != 0,
        sf: word & (1 << 7) != 0,
        of: word & (1 << 11) != 0,
    }
}

fn map_decode_error(trap: Trap, rip: u64) -> X86Error {
    match trap {
        Trap::DecodeError { opcode: 0, .. } => X86Error::TruncatedInstruction { rip },
        Trap::DecodeError { opcode, .. } => X86Error::UnknownInstruction { rip, opcode },
        other => map_exec_error(other, rip),
    }
}

fn map_exec_error(trap: Trap, rip: u64) -> X86Error {
    match trap {
        Trap::IllegalInstruction(_) => X86Error::IllegalInstruction { rip },
        Trap::DivideError(_) => X86Error::DivideError { rip },
        Trap::DecodeError { opcode, .. } => X86Error::UnknownInstruction { rip, opcode },
        Trap::StepLimitExceeded => X86Error::StepLimitExceeded { max_steps: 0 },
        Trap::UnresolvedExternal(_) => X86Error::UnsupportedCall { rip },
        Trap::MemoryFault(_) => X86Error::InvalidState("architectural memory fault".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_state_load_trace_and_halt() {
        let mut machine = X86FunctionalSimulator::new();
        let result = machine.run_checked(&[0x90, 0xf4], 4).unwrap();
        assert!(result.halted);
        assert_eq!(result.steps, 2);
        assert_eq!(result.final_state.memory.len(), MEMORY_SIZE);
        assert_eq!(result.final_state.gpr[Reg::Rsp as usize], 0xfff8);
        assert_eq!(result.traces[1].raw, [0xf4]);
    }

    #[test]
    fn checked_failures_are_atomic() {
        let mut machine = X86FunctionalSimulator::new();
        machine.load_checked(&[0x0f]).unwrap();
        let before = machine.get_state();
        assert_eq!(
            machine.step_checked(),
            Err(X86Error::TruncatedInstruction { rip: 0 })
        );
        assert_eq!(machine.get_state(), before);
    }

    #[test]
    fn python_width_stack_multiply_and_inc_dec_edges() {
        let mut machine = X86FunctionalSimulator::new();
        let program = [
            0x48, 0xb8, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, // rax=-1
            0xb8, 0x01, 0x00, 0x00, 0x00, // eax=1, zero extends
            0x6a, 0xff, // push -1
            0x59, // pop rcx
            0x48, 0x6b, 0xd1, 0x04, // imul rdx,rcx,4
            0x48, 0xff, 0xc0, // inc rax
            0x48, 0xff, 0xc8, // dec rax
            0xf4,
        ];
        let result = machine.run_checked(&program, 16).unwrap();
        assert_eq!(result.final_state.gpr[Reg::Rax as usize], 1);
        assert_eq!(result.final_state.gpr[Reg::Rcx as usize], u64::MAX);
        assert_eq!(result.final_state.gpr[Reg::Rdx as usize], u64::MAX - 3);
        assert_eq!(result.final_state.gpr[Reg::Rsp as usize], 0xfff8);
    }

    #[test]
    fn unsigned_mul_writes_rdx_rax() {
        let mut machine = X86FunctionalSimulator::new();
        let mut program = vec![0x48, 0xb8];
        program.extend_from_slice(&0x8000_0000_0000_0000_u64.to_le_bytes());
        program.extend_from_slice(&[0x48, 0xb9]);
        program.extend_from_slice(&2_u64.to_le_bytes());
        program.extend_from_slice(&[0x48, 0xf7, 0xe1, 0xf4]);
        let state = machine.run_checked(&program, 8).unwrap().final_state;
        assert_eq!(state.gpr[Reg::Rax as usize], 0);
        assert_eq!(state.gpr[Reg::Rdx as usize], 1);
    }

    #[test]
    fn adc_and_sbb_consume_borrow_exactly() {
        let mut machine = X86FunctionalSimulator::new();
        let mut program = vec![0x48, 0xb8];
        program.extend_from_slice(&0_u64.to_le_bytes());
        program.extend_from_slice(&[0x48, 0xb9]);
        program.extend_from_slice(&0_u64.to_le_bytes());
        program.extend_from_slice(&[
            0x48, 0x83, 0xe9, 0x01, // sub rcx,1 => CF
            0x48, 0x83, 0xd0, 0x00, // adc rax,0 => 1
            0x48, 0x83, 0xe9, 0x01, // sub rcx,1 => no CF
            0x48, 0x83, 0xd8, 0x00, // sbb rax,0 => unchanged
            0xf4,
        ]);
        let state = machine.run_checked(&program, 16).unwrap().final_state;
        assert_eq!(state.gpr[Reg::Rax as usize], 1);
    }

    #[test]
    fn rotate_and_cl_shift_match_x86_count_and_flag_rules() {
        let mut machine = X86FunctionalSimulator::new();
        let mut program = vec![0x48, 0xb8];
        program.extend_from_slice(&0x8000_0000_0000_0001_u64.to_le_bytes());
        program.extend_from_slice(&[0x48, 0xb9]);
        program.extend_from_slice(&1_u64.to_le_bytes());
        program.extend_from_slice(&[
            0x48, 0xd3, 0xc0, // rol rax,cl => 3, CF=1
            0x48, 0xd3, 0xc8, // ror rax,cl => original value
            0x48, 0xb9, 0, 0, 0, 0, 0, 0, 0, 0, // mov rcx,0, flags unchanged
            0x48, 0x83, 0xea, 0x01, // sub rdx,1 => CF=1
            0x48, 0xd3, 0xe0, // shl rax,cl must preserve value and flags
            0xf4,
        ]);
        let state = machine.run_checked(&program, 16).unwrap().final_state;
        assert_eq!(state.gpr[Reg::Rax as usize], 0x8000_0000_0000_0001);
        assert_eq!(state.rflags & 1, 1);
    }

    #[test]
    fn extensions_exchange_conditionals_and_bit_ops_match_python_surface() {
        let mut machine = X86FunctionalSimulator::new();
        let mut program = vec![0x48, 0xb9];
        program.extend_from_slice(&0xff_u64.to_le_bytes());
        program.extend_from_slice(&[
            0x48, 0x0f, 0xbe, 0xc1, // movsx rax,cl => -1
            0xb9, 0x00, 0x00, 0x00, 0x80, // mov ecx,0x80000000
            0x48, 0x63, 0xd1, // movsxd rdx,ecx
            0x48, 0x87, 0xc1, // xchg rax,rcx
            0x48, 0x39, 0xc0, // cmp rax,rax => ZF
            0x48, 0x0f, 0x44, 0xd8, // cmove rbx,rax
            0x48, 0x0f, 0xbc, 0xf3, // bsf rsi,rbx
            0x48, 0x0f, 0xbd, 0xfb, // bsr rdi,rbx
            0x48, 0x0f, 0xa3, 0xf8, // bt rax,rdi => top bit
            0x48, 0x0f, 0xc8, // bswap rax
            0xf4,
        ]);
        let state = machine.run_checked(&program, 24).unwrap().final_state;
        assert_eq!(state.gpr[Reg::Rax as usize], 0x0000_0080_0000_0000);
        assert_eq!(state.gpr[Reg::Rcx as usize], u64::MAX);
        assert_eq!(state.gpr[Reg::Rdx as usize], 0xffff_ffff_8000_0000);
        assert_eq!(state.gpr[Reg::Rbx as usize], 0x8000_0000);
        assert_eq!(state.gpr[Reg::Rsi as usize], 31);
        assert_eq!(state.gpr[Reg::Rdi as usize], 31);
        assert_eq!(state.rflags & 1, 1);
    }

    #[test]
    fn repeated_stores_and_counter_branches_are_complete() {
        let mut machine = X86FunctionalSimulator::new();
        let mut program = vec![0x48, 0xb8];
        program.extend_from_slice(&0x1122_3344_5566_7788_u64.to_le_bytes());
        program.extend_from_slice(&[0x48, 0xb9]);
        program.extend_from_slice(&2_u64.to_le_bytes());
        program.extend_from_slice(&[0x48, 0xbf]);
        program.extend_from_slice(&0x100_u64.to_le_bytes());
        program.extend_from_slice(&[
            0xf3, 0x48, 0xab, // rep stosq
            0x48, 0xb9, 0x03, 0, 0, 0, 0, 0, 0, 0, // mov rcx,3
            0x48, 0x83, 0xc2, 0x01, // add rdx,1
            0xe2, 0xfa, // loop back to add
            0xe3, 0x0a, // jrcxz skips the following movabs
            0x48, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0xf4,
        ]);
        let state = machine.run_checked(&program, 32).unwrap().final_state;
        assert_eq!(state.gpr[Reg::Rdx as usize], 3);
        assert_eq!(state.gpr[Reg::Rcx as usize], 0);
        assert_eq!(state.gpr[Reg::Rdi as usize], 0x110);
        assert_eq!(state.gpr[Reg::Rax as usize], 0x1122_3344_5566_7788);
        assert_eq!(
            &state.memory[0x100..0x110],
            &[
                0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, 0x88, 0x77, 0x66, 0x55, 0x44, 0x33,
                0x22, 0x11,
            ]
        );
    }

    #[test]
    fn reverse_alu_forms_and_ret_cleanup_execute() {
        let mut machine = X86FunctionalSimulator::new();
        let mut program = vec![0x48, 0xb8];
        program.extend_from_slice(&0xf0_u64.to_le_bytes());
        program.extend_from_slice(&[0x48, 0xb9]);
        program.extend_from_slice(&0x0f_u64.to_le_bytes());
        program.extend_from_slice(&[
            0x48, 0x0b, 0xc1, // or rax,rcx => ff
            0x48, 0x23, 0xc1, // and rax,rcx => 0f
            0x48, 0x33, 0xc1, // xor rax,rcx => 0
            0xe8, 0x01, 0, 0, 0, // call function after hlt
            0xf4, 0xc2, 0x10, 0x00, // ret 16
        ]);
        let state = machine.run_checked(&program, 16).unwrap().final_state;
        assert_eq!(state.gpr[Reg::Rax as usize], 0);
        assert_eq!(state.gpr[Reg::Rsp as usize], 0x1_0008);
    }
}
