//! Exact DFF-backed x86-64 lifecycle scaffold.

use crate::execution::{exec_one, Flow};
use crate::register_file::RegisterFile64;
use crate::state::{clock_bit, DffMemory};
use x86_simulator::decode::decode;
use x86_simulator::functional::{
    X86Error, X86ExecutionResult, X86FunctionalSimulator, X86State, X86StepTrace, MEMORY_SIZE,
};
use x86_simulator::memory::Memory;
use x86_simulator::state::{CpuState, Flags, Reg};
use x86_simulator::trap::Trap;

/// 524,288 memory bits + 1,024 GPR bits + 64 RIP bits + five flags + halt.
pub const FLIP_FLOP_COUNT: usize = MEMORY_SIZE * 8 + 16 * 64 + 64 + 5 + 1;

#[derive(Clone, Debug)]
pub struct X86GateSimulator {
    rf: RegisterFile64,
    memory: DffMemory,
    halt_q: u8,
    loaded_origin: u64,
    loaded_len: usize,
}

impl Default for X86GateSimulator {
    fn default() -> Self {
        Self::new()
    }
}

impl X86GateSimulator {
    pub fn new() -> Self {
        Self {
            rf: RegisterFile64::new(),
            memory: DffMemory::new(MEMORY_SIZE),
            halt_q: 0,
            loaded_origin: 0,
            loaded_len: 0,
        }
    }

    pub fn reset(&mut self) {
        self.rf = RegisterFile64::new();
        self.memory.clear();
        clock_bit(&mut self.halt_q, false);
        self.loaded_origin = 0;
        self.loaded_len = 0;
    }

    pub fn get_state(&self) -> X86State {
        X86State {
            rip: self.rf.rip(),
            gpr: self.rf.snapshot(),
            rflags: self.rf.flags(),
            memory: self.memory.snapshot(),
            halted: self.halt_q != 0,
            loaded_origin: self.loaded_origin,
            loaded_len: self.loaded_len,
        }
    }

    pub fn restore(&mut self, state: &X86State) -> Result<(), X86Error> {
        let mut validator = X86FunctionalSimulator::new();
        validator.restore(state)?;
        self.rf.restore(&state.gpr, state.rip, state.rflags);
        self.memory.restore_snapshot(&state.memory);
        clock_bit(&mut self.halt_q, state.halted);
        self.loaded_origin = state.loaded_origin;
        self.loaded_len = state.loaded_len;
        Ok(())
    }

    pub fn load_checked(&mut self, program: &[u8]) -> Result<(), X86Error> {
        self.load_at_checked(program, 0)
    }

    pub fn load_at_checked(&mut self, program: &[u8], origin: u64) -> Result<(), X86Error> {
        let mut validator = X86FunctionalSimulator::new();
        validator.load_at_checked(program, origin)?;
        self.restore(&validator.get_state())
    }

    pub fn read_register_checked(&self, index: usize) -> Result<u64, X86Error> {
        if index >= 16 {
            return Err(X86Error::InvalidState(format!(
                "register {index} is outside r0-r15"
            )));
        }
        Ok(self.rf.read(index))
    }

    pub fn write_register_checked(&mut self, index: usize, value: u64) -> Result<(), X86Error> {
        if index >= 16 {
            return Err(X86Error::InvalidState(format!(
                "register {index} is outside r0-r15"
            )));
        }
        self.rf.write(index, value);
        Ok(())
    }

    pub fn read_byte(&self, address: u64) -> u8 {
        self.memory[(address as usize) & (MEMORY_SIZE - 1)]
    }

    pub fn write_byte(&mut self, address: u64, value: u8) {
        self.memory
            .write((address as usize) & (MEMORY_SIZE - 1), value);
    }

    pub fn step_checked(&mut self) -> Result<X86StepTrace, X86Error> {
        let before = self.get_state();
        if before.halted {
            return Err(X86Error::Halted);
        }
        let rip = before.rip;
        let available = installed_bytes(&before)?;
        let fetch_len = available.min(15);
        let bytes: Vec<u8> = (0..fetch_len)
            .map(|offset| self.read_byte(rip + offset as u64))
            .collect();
        let decoded = decode(&bytes, 0).map_err(|trap| map_decode_error(trap, rip))?;
        let raw = bytes[..decoded.len].to_vec();
        let mut cpu = CpuState {
            gpr: before.gpr,
            rip,
            flags: word_to_flags(before.rflags),
            xmm: [0; 16],
        };
        let mut memory = Memory::new_wrapping(MEMORY_SIZE);
        memory
            .restore_bytes(&before.memory)
            .map_err(|trap| map_exec_error(trap, rip))?;
        let next_ip = rip.wrapping_add(decoded.len as u64);
        let flow = exec_one(&mut cpu, &mut memory, &decoded.instr, rip, next_ip, 0)
            .map_err(|trap| map_exec_error(trap, rip))?;
        let mnemonic = format!("{:?}", decoded.instr);
        let mut halted = false;
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
                halted = true;
            }
        }
        let after = X86State {
            rip: cpu.rip,
            gpr: cpu.gpr,
            rflags: flags_to_word(cpu.flags),
            memory: memory.snapshot(),
            halted,
            loaded_origin: before.loaded_origin,
            loaded_len: before.loaded_len,
        };
        self.commit_transition(&before, &after);
        Ok(X86StepTrace {
            rip_before: rip,
            rip_after: after.rip,
            raw,
            mnemonic,
            state_before: before,
            state_after: after,
        })
    }

    fn commit_transition(&mut self, before: &X86State, after: &X86State) {
        self.rf.restore(&after.gpr, after.rip, after.rflags);
        for (address, (&old, &new)) in before.memory.iter().zip(&after.memory).enumerate() {
            if old != new {
                self.memory.write(address, new);
            }
        }
        clock_bit(&mut self.halt_q, after.halted);
    }

    pub fn run_loaded_checked(&mut self, max_steps: usize) -> Result<X86ExecutionResult, X86Error> {
        let before = self.get_state();
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
                    self.restore(&before).expect("previous state is valid");
                    return Err(error);
                }
            }
        }
        self.restore(&before).expect("previous state is valid");
        Err(X86Error::StepLimitExceeded { max_steps })
    }

    pub fn run_checked(
        &mut self,
        program: &[u8],
        max_steps: usize,
    ) -> Result<X86ExecutionResult, X86Error> {
        let before = self.get_state();
        self.load_checked(program)?;
        match self.run_loaded_checked(max_steps) {
            Ok(result) => Ok(result),
            Err(error) => {
                self.restore(&before).expect("previous state is valid");
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
