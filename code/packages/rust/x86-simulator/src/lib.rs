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
            // `void __twig_gc_write_barrier(i64 parent, i64 child)` — deliberately
            // a **no-op here**, and that is a real implementation, not a stub.
            //
            // On a real target this is the *generational* write barrier
            // (`gc-core-capi`'s `__gc_write_barrier`): when the mutator stores a
            // reference `child` into a field of an already-**old** object
            // `parent`, the collector must remember `parent`, because a later
            // *minor* collection only traces the young generation and would
            // otherwise never discover that an old object now points at a young
            // one — and would free a live object. The barrier's whole job is to
            // add `parent` to that remembered set. `child` is never dereferenced.
            //
            // None of that machinery exists in this simulator. `Memory::alloc`
            // (the `__twig_alloc_bytes` shim above) is a **monotonic bump
            // allocator**: `heap_next` only ever moves forward, nothing is ever
            // freed, swept, promoted, or relocated, and there is exactly one
            // generation because there is no collector at all. With no minor
            // collection to run, a remembered set has no reader, so recording
            // into one would be pure bookkeeping no observable behaviour depends
            // on. Doing nothing is therefore *semantically exact*, not an
            // approximation — a simulated program cannot distinguish this from a
            // real barrier, since the only way to observe a missed barrier is to
            // watch the collector reclaim a live object, and this heap never
            // reclaims anything.
            //
            // Consequently we read both arguments purely to document the System V
            // signature the `x86_64-backend` emits against — `parent` in RDI,
            // `child` in RSI (see `array_set`/`field_store`, which reload both
            // fresh from their stack slots into `abi.arg_regs()[0..2]` right
            // before the `call`). Being `void`, it leaves RAX alone, exactly like
            // the `__twig_print_i64` shim above: SysV treats RAX as
            // caller-clobbered, so a caller may not rely on its value either way.
            "__twig_gc_write_barrier" => {
                let _parent = self.state.get(Reg::Rdi);
                let _child = self.state.get(Reg::Rsi);
            }
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

    // The shape `x86_64-backend` emits after every `field_store` / `array_set`:
    // a `call rel32` to the GC write barrier (relocated at offset 1), then `ret`.
    // `E8 00000000` is the un-patched call; the harness records offset 1 as an
    // external relocation, so `Flow::Call` routes it through `host_call`.
    const BARRIER_CALL_FN: &[u8] = &[0xE8, 0x00, 0x00, 0x00, 0x00, 0xC3];

    /// Regression guard for the gap that made three LANG-FULL matrix cells
    /// (`basic_array`, `algol_static_array`, `algol_for_loop_array`) trap with
    /// `UnresolvedExternal("__twig_gc_write_barrier")`: once the precise-GC track
    /// taught the backend to emit a write barrier on heap stores, every array /
    /// field store in the matrix reached a symbol this dispatch table did not
    /// know. Any program doing a heap store now routes through here, so a
    /// regression would break far more than one test — this asserts the symbol
    /// resolves at all, independent of any particular frontend.
    #[test]
    fn gc_write_barrier_resolves_instead_of_trapping() {
        let relocs = [harness::Reloc { patch_offset: 1, symbol: "__twig_gc_write_barrier".into() }];
        let mut sim = harness::MachineCodeHarness::new()
            .function("main", BARRIER_CALL_FN, &relocs)
            .build("main")
            .expect("entry exists");
        assert_eq!(
            sim.run(),
            Ok(0),
            "a call to __twig_gc_write_barrier must run to a clean ret, not trap as an unresolved external",
        );
    }

    /// The barrier must be a *silent* no-op, not merely a resolved symbol: it may
    /// not allocate, and it may not emit output. This pins the two observable
    /// channels a wrong implementation would disturb — an accidental `alloc`
    /// would move the bump cursor (corrupting every later heap address the guest
    /// computed), and an accidental write would corrupt captured stdout.
    #[test]
    fn gc_write_barrier_is_a_silent_no_op() {
        let mut sim = harness::MachineCodeHarness::new()
            .function("main", MIN_FN, &[])
            .build("main")
            .expect("entry exists");
        // Non-zero, deliberately bogus arguments: the barrier never dereferences
        // either operand, so even a wild `child` must be harmless.
        sim.state.set(Reg::Rdi, 0x4141_4141);
        sim.state.set(Reg::Rsi, 0x4242_4242);
        // `alloc(0)` is an idempotent probe of the bump cursor: it reserves
        // nothing and returns the current (aligned) `heap_next`.
        let heap_before = sim.mem.alloc(0).expect("heap probe");
        sim.host_call("__twig_gc_write_barrier").expect("the barrier shim resolves");
        let heap_after = sim.mem.alloc(0).expect("heap probe");
        assert_eq!(heap_before, heap_after, "the barrier must not allocate — the heap cursor may not move");
        assert!(sim.stdout.is_empty(), "the barrier must not write to stdout");
    }

    /// The fix must not have widened the dispatch table into a catch-all: an
    /// unknown symbol still has to fail closed.
    #[test]
    fn an_unknown_external_still_traps() {
        let mut sim = harness::MachineCodeHarness::new()
            .function("main", MIN_FN, &[])
            .build("main")
            .expect("entry exists");
        assert_eq!(
            sim.host_call("__twig_not_a_real_symbol"),
            Err(Trap::UnresolvedExternal("__twig_not_a_real_symbol".to_string())),
            "unknown externals must still trap, not silently no-op",
        );
    }
}
