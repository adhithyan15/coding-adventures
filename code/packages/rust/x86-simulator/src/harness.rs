//! Load the `x86_64-backend`'s emitted machine code into a runnable [`Simulator`].
//!
//! This is the bridge that makes the backend's output executable on any host. It
//! mirrors `wasm-runtime`'s loader: take each function's byte blob + its
//! relocations, lay the code out in the address space, resolve **internal** calls
//! (to other functions in the module) by patching their `rel32`, route
//! **external** calls (`__twig_alloc_bytes`, `putchar`, `print_i64`, …) to host
//! shims, set up a stack with a return sentinel, and point `rip` at the entry.

use std::collections::HashMap;

use crate::memory::Memory;
use crate::state::{CpuState, Reg};
use crate::Simulator;

/// A relocation site: the byte offset of a `rel32` field *within its function*,
/// and the symbol it targets. (The lightweight mirror of the backend's
/// `ExternalReloc`, so this crate stays dependency-light — callers map their
/// reloc type onto this.)
#[derive(Debug, Clone)]
pub struct Reloc {
    /// Offset of the 4-byte `rel32` within the function's bytes.
    pub patch_offset: usize,
    /// The target symbol name.
    pub symbol: String,
}

/// Why a harness could not be built. All of these are returned (never panicked)
/// so the loader is fail-closed even on a malformed function/relocation table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    /// The named entry function was not added.
    NoSuchEntry(String),
    /// A relocation's `rel32` site (`patch_offset..patch_offset+4`) lies outside
    /// its function's bytes — a malformed reloc.
    BadReloc { symbol: String, patch_offset: usize, fn_len: usize },
    /// The concatenated code is larger than the simulator's address space.
    CodeTooLarge { code_len: usize, capacity: u64 },
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildError::NoSuchEntry(n) => write!(f, "no function named {n:?} was added"),
            BuildError::BadReloc { symbol, patch_offset, fn_len } =>
                write!(f, "relocation for {symbol:?} at patch_offset {patch_offset} is outside its {fn_len}-byte function"),
            BuildError::CodeTooLarge { code_len, capacity } =>
                write!(f, "code region ({code_len} bytes) exceeds the {capacity}-byte address space"),
        }
    }
}
impl std::error::Error for BuildError {}

/// Builder for a [`Simulator`] from backend function blobs.
#[derive(Default)]
pub struct MachineCodeHarness {
    funcs: Vec<(String, Vec<u8>, Vec<Reloc>)>,
}

// Layout constants for the flat address space.
const MEM_SIZE: usize = 1 << 20; // 1 MiB
const CODE_BASE: u64 = 0x1000;
const STACK_TOP: u64 = MEM_SIZE as u64 - 16; // 16-aligned-ish top of stack
const STACK_BOTTOM: u64 = 0x8_0000; // heap may grow up to here
const RETURN_SENTINEL: u64 = 0xDEAD_0000_0000_0000;
const STEP_LIMIT: u64 = 10_000_000;

impl MachineCodeHarness {
    /// A fresh, empty harness.
    pub fn new() -> Self {
        MachineCodeHarness::default()
    }

    /// Add a compiled function (name, machine-code bytes, relocations).
    pub fn function(mut self, name: &str, bytes: &[u8], relocs: &[Reloc]) -> Self {
        self.funcs.push((name.to_string(), bytes.to_vec(), relocs.to_vec()));
        self
    }

    /// Assemble the loaded functions into a ready-to-run [`Simulator`] starting
    /// at `entry`.
    pub fn build(self, entry: &str) -> Result<Simulator, BuildError> {
        // 1. Concatenate function bytes; record each function's global offset.
        let mut code: Vec<u8> = Vec::new();
        let mut fn_offset: HashMap<String, usize> = HashMap::new();
        let mut relocs: Vec<(usize, String)> = Vec::new(); // global patch_offset → symbol
        for (name, bytes, rs) in &self.funcs {
            let base = code.len();
            fn_offset.insert(name.clone(), base);
            code.extend_from_slice(bytes);
            for r in rs {
                // The rel32 site must lie within this function's bytes.
                if r.patch_offset.checked_add(4).map_or(true, |e| e > bytes.len()) {
                    return Err(BuildError::BadReloc {
                        symbol: r.symbol.clone(),
                        patch_offset: r.patch_offset,
                        fn_len: bytes.len(),
                    });
                }
                relocs.push((base + r.patch_offset, r.symbol.clone()));
            }
        }

        let entry_off = *fn_offset.get(entry).ok_or_else(|| BuildError::NoSuchEntry(entry.to_string()))?;

        // 2. Resolve relocations: an internal call (symbol is a function in this
        //    module) is patched so its decoded target lands on the callee; an
        //    external call stays in the `externals` map for a host shim.
        let mut externals: HashMap<usize, String> = HashMap::new();
        for (patch_off, symbol) in relocs {
            if let Some(&target) = fn_offset.get(&symbol) {
                // rel32 = target - (patch_off + 4), so decode resolves to `target`.
                let rel = (target as i64) - (patch_off as i64 + 4);
                let bytes = (rel as i32).to_le_bytes();
                code[patch_off..patch_off + 4].copy_from_slice(&bytes);
            } else {
                externals.insert(patch_off, symbol);
            }
        }

        // 3. Lay out memory: code at CODE_BASE, then the bump heap up to
        //    STACK_BOTTOM, then the stack from STACK_BOTTOM..STACK_TOP. The code
        //    must fit below the heap window, else the layout is invalid.
        let heap_base = ((CODE_BASE + code.len() as u64) + 15) & !15;
        if heap_base >= STACK_BOTTOM {
            return Err(BuildError::CodeTooLarge { code_len: code.len(), capacity: STACK_BOTTOM - CODE_BASE });
        }
        let mut mem = Memory::new(MEM_SIZE, heap_base, STACK_BOTTOM);
        // These stores are now provably in-bounds (heap_base < STACK_BOTTOM <
        // STACK_TOP < MEM_SIZE), so a failure is a harness bug, not guest input.
        mem.write_block(CODE_BASE, &code).expect("code region is in-bounds after the size check");
        let mut state = CpuState::default();
        let sp = (STACK_TOP & !0xF) - 8;
        mem.store(sp, 8, RETURN_SENTINEL).expect("stack top is in-bounds");
        state.set(Reg::Rsp, sp);
        state.set(Reg::Rbp, sp);
        state.rip = CODE_BASE + entry_off as u64;

        Ok(Simulator {
            state,
            mem,
            stdout: Vec::new(),
            code,
            code_base: CODE_BASE,
            externals,
            return_sentinel: RETURN_SENTINEL,
            step_limit: STEP_LIMIT,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_entry_fails_closed() {
        let err = MachineCodeHarness::new().function("f", &[0xC3], &[]).build("nope").unwrap_err();
        assert!(matches!(err, BuildError::NoSuchEntry(_)));
    }

    #[test]
    fn out_of_range_reloc_fails_closed_not_panic() {
        // A reloc whose rel32 site is past the 1-byte function → BadReloc, no panic.
        let relocs = [Reloc { patch_offset: 9999, symbol: "__twig_alloc_bytes".into() }];
        let err = MachineCodeHarness::new().function("f", &[0xC3], &relocs).build("f").unwrap_err();
        assert!(matches!(err, BuildError::BadReloc { .. }));
    }
}
