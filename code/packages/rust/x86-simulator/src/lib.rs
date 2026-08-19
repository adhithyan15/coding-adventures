//! # x86 / x86-64 runtime simulator
//!
//! A Rust runtime simulator that decodes and executes the 64-bit x86 machine
//! code the in-repo `x86_64-backend` emits — so that backend's output can be
//! **run on any host architecture** (notably aarch64 / Apple Silicon), instead
//! of only byte-compared locally and executed on an x86 CI runner. It is the
//! runtime sibling of `riscv-simulator`, and uses the ISA semantics specified in
//! `code/specs/07w-x86-64-simulator.md` (referenced, not restated).
//!
//! ## Two ways in
//!
//! - [`Simulator`] — the low-level engine: a [`CpuState`] + [`Memory`] + a loaded
//!   code region, with `step`/`run`.
//! - [`harness::MachineCodeHarness`] — loads the backend's per-function byte blobs
//!   + relocations, wires the System V runtime host imports (`__twig_alloc_bytes`
//!   / `putchar` / `print_i64`), and produces a ready-to-run [`Simulator`] whose
//!   `run()` returns the program's exit code. This is the piece the LANG-FULL
//!   matrix uses to execute the x86_64 column locally.
//!
//! Everything is **fail-closed**: an unknown opcode, an out-of-range access, a
//! `ud2`, or an unresolved external symbol is a [`Trap`] — a guest program can
//! never escape the sandbox.
#![allow(clippy::doc_lazy_continuation)]

pub mod decode;
pub mod execute;
pub mod flags;
pub mod harness;
pub mod memory;
pub mod state;
pub mod trap;

use std::collections::HashMap;

use decode::decode;
use execute::{exec_one, Flow};
pub use memory::Memory;
pub use state::{CpuState, Reg};
pub use trap::Trap;

/// Outcome of a single [`Simulator::step`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepOutcome {
    /// Execution continues.
    Continue,
    /// The entry function returned; carries `rax` (the raw return value).
    Halt(u64),
}

/// The x86-64 execution engine: CPU + memory + a loaded code region.
#[derive(Debug)]
pub struct Simulator {
    /// Architectural CPU state.
    pub state: CpuState,
    /// The flat address space.
    pub mem: Memory,
    /// Captured bytes written by host I/O shims (`putchar`/`print_i64`).
    pub stdout: Vec<u8>,
    /// Bytes the program reads via `getchar`, consumed front-to-back; once
    /// exhausted, `getchar` returns EOF (`-1`). Empty by default.
    input: Vec<u8>,
    /// How many `input` bytes have been consumed so far.
    input_pos: usize,
    /// The concatenated code bytes (relocations already patched).
    code: Vec<u8>,
    /// Where the code region begins in `mem`.
    code_base: u64,
    /// rel32-patch-offset (global, within `code`) → external runtime symbol.
    externals: HashMap<usize, String>,
    /// When `ret` pops this address, the entry function has returned.
    return_sentinel: u64,
    /// Runaway-loop backstop.
    step_limit: u64,
}

impl Simulator {
    /// Execute one instruction. Returns whether to continue or halt.
    pub fn step(&mut self) -> Result<StepOutcome, Trap> {
        let off = self.state.rip.wrapping_sub(self.code_base) as usize;
        let d = decode(&self.code, off)?;
        let next_ip = self.code_base.wrapping_add((off + d.len) as u64);
        let flow = exec_one(&mut self.state, &mut self.mem, &d.instr, self.code_base, next_ip, off)?;
        match flow {
            Flow::Next => { self.state.rip = next_ip; Ok(StepOutcome::Continue) }
            Flow::Jump(t) => { self.state.rip = t; Ok(StepOutcome::Continue) }
            Flow::Trap => Err(Trap::IllegalInstruction(self.state.rip)),
            Flow::Ret => {
                let sp = self.state.get(Reg::Rsp);
                let ret = self.mem.load(sp, 8)?;
                self.state.set(Reg::Rsp, sp.wrapping_add(8));
                if ret == self.return_sentinel {
                    Ok(StepOutcome::Halt(self.state.get(Reg::Rax)))
                } else {
                    self.state.rip = ret;
                    Ok(StepOutcome::Continue)
                }
            }
            Flow::Call { target, site } => {
                if let Some(sym) = self.externals.get(&site).cloned() {
                    self.host_call(&sym)?;
                    self.state.rip = next_ip; // host shim ran; skip past the call
                } else {
                    // Internal call: push the return address and jump.
                    let sp = self.state.get(Reg::Rsp).wrapping_sub(8);
                    self.mem.store(sp, 8, next_ip)?;
                    self.state.set(Reg::Rsp, sp);
                    self.state.rip = target;
                }
                Ok(StepOutcome::Continue)
            }
        }
    }

    /// Run from the entry until the entry function returns; return the process
    /// exit code (`rax & 0xFF`, matching `run_native`/`run_wasm`).
    pub fn run(&mut self) -> Result<i32, Trap> {
        for _ in 0..self.step_limit {
            if let StepOutcome::Halt(rax) = self.step()? {
                return Ok((rax & 0xFF) as i32);
            }
        }
        Err(Trap::StepLimitExceeded)
    }

    /// Dispatch a System V call to a host runtime symbol (args in rdi/rsi/…,
    /// result in rax) — the analogue of `wasm-runtime`'s host imports.
    fn host_call(&mut self, sym: &str) -> Result<(), Trap> {
        match sym {
            // `void* __twig_alloc_bytes(size_t n)` — bump-allocate `n` zeroed bytes.
            "__twig_alloc_bytes" => {
                let n = self.state.get(Reg::Rdi);
                let ptr = self.mem.alloc(n)?;
                self.state.set(Reg::Rax, ptr);
            }
            // `int putchar(int c)` — capture the byte.  The backend emits the
            // runtime-prefixed `__twig_putchar`; libc-style `putchar` is accepted
            // too (same shim, like the `print_i64` aliases below).
            "__twig_putchar" | "putchar" => {
                let c = self.state.get(Reg::Rdi) as u8;
                self.stdout.push(c);
                self.state.set(Reg::Rax, c as u64);
            }
            // `int getchar()` — consume the next input byte, or EOF (`-1`) once
            // the buffer is drained.  Returning `-1` (not 0) matches the libc /
            // native convention; the Brainfuck IIR clamps a negative `getchar`
            // to 0 itself, so a `,[.,]` cat loop halts at end-of-input.
            "__twig_getchar" | "getchar" => {
                let r = match self.input.get(self.input_pos) {
                    Some(&b) => { self.input_pos += 1; b as u64 }
                    None => u64::MAX, // EOF (-1)
                };
                self.state.set(Reg::Rax, r);
            }
            // `void __twig_print_i64(i64)` — print the decimal value.
            "__twig_print_i64" | "__print_i64" | "print_i64" => {
                let v = self.state.get(Reg::Rdi) as i64;
                self.stdout.extend(v.to_string().into_bytes());
            }
            // `void __twig_gc_write_barrier(parent, child)` — accepted, does nothing.
            //
            // The x86_64-backend emits this call after every `field_store` and
            // `array_set` (#10489), because a *real* run links against the
            // generational collector in `gc-core-capi`, where the barrier records
            // an old→young edge in the remembered set so a minor collection does
            // not free a young object that only an old object still points at.
            //
            // This simulator has no collector to remember anything *for*: `Memory`
            // is a bump allocator that never reclaims, so every object stays live
            // for the whole run by construction and the remembered set has no
            // reader. The barrier is therefore *semantically* a no-op here — not
            // merely unimplemented — and the two arguments in rdi/rsi are read by
            // nobody. What matters is only that it resolves: a `ret`-equivalent, so
            // the array programs in the language matrix run past their stores
            // instead of trapping `UnresolvedExternal`.
            //
            // Note this is a `void` shim: it deliberately leaves rax alone rather
            // than zeroing it, matching the C prototype (a caller that reads rax
            // after a void call is already wrong, and clobbering it would mask that).
            "__twig_gc_write_barrier" => {}
            other => return Err(Trap::UnresolvedExternal(other.to_string())),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The exact bytes the x86_64-backend emits for `const_u64 v=42; ret_u64 v`.
    const MIN_FN: &[u8] = &[
        0x55, 0x48, 0x89, 0xE5, 0x48, 0x81, 0xEC, 0x10, 0x00, 0x00, 0x00,
        0x48, 0xC7, 0xC0, 0x2A, 0x00, 0x00, 0x00, 0x48, 0x89, 0x85, 0xF8,
        0xFF, 0xFF, 0xFF, 0x48, 0x8B, 0x85, 0xF8, 0xFF, 0xFF, 0xFF, 0x48,
        0x89, 0xEC, 0x5D, 0xC3,
    ];

    #[test]
    fn runs_a_backend_compiled_const_ret_to_exit_42() {
        let mut sim = harness::MachineCodeHarness::new()
            .function("main", MIN_FN, &[])
            .build("main")
            .expect("entry exists");
        assert_eq!(sim.run().unwrap(), 42, "the simulator runs real x86_64 codegen → 42");
    }
}
