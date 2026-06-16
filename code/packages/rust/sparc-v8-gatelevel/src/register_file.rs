//! SPARC V8 register file with sliding register windows.
//!
//! # Register architecture
//!
//! SPARC V8 has a *windowed* register file.  Each procedure sees 32 logical
//! registers, but only 8 of them (the globals) are truly shared; the remaining
//! 24 are a *window* into the physical register file that slides when SAVE or
//! RESTORE is executed.
//!
//! ## Physical layout (NWINDOWS = 3)
//!
//! ```text
//!  Physical regs 0–7:   globals (%g0–%g7)     — shared by all windows
//!  Physical regs 8–23:  window 0 outs/locals   — CWP=0 → %o0–%l7
//!  Physical regs 24–39: window 1 outs/locals   — CWP=1 → %o0–%l7
//!  Physical regs 40–55: window 2 outs/locals   — CWP=2 → %o0–%l7
//! ```
//!
//! ## Logical-to-physical mapping
//!
//! With `CWP` (current window pointer) in 0..NWINDOWS:
//!
//! ```text
//!  logical 0–7   (%g0–%g7):  physical 0–7    (globals, always)
//!  logical 8–23  (%o0–%l7):  physical 8 + CWP*16 + (logical-8)
//!  logical 24–31 (%i0–%i7):  physical 8 + ((CWP+1)%NWINDOWS)*16 + (logical-24)
//! ```
//!
//! The *in* registers of window N are the *out* registers of window N+1, which
//! is how the call/return argument passing mechanism works.
//!
//! ## SAVE and RESTORE
//!
//! - `SAVE`: decrement CWP (mod NWINDOWS).  The caller's `%o` registers become
//!   the callee's `%i` registers.  Detects register window overflow.
//! - `RESTORE`: increment CWP (mod NWINDOWS).  The callee's `%i` registers
//!   become the caller's `%o` registers again.

pub const NWINDOWS: u32 = 3;
pub const NUM_PHYS: usize = 56;   // 8 globals + 3 × 16 windowed
pub const MEM_SIZE: usize = 0x10000;

/// Map a logical register number (0–31) and CWP to a physical register index.
pub fn virt_to_phys(virt: u32, cwp: u32) -> usize {
    let cwp = cwp % NWINDOWS;
    if virt < 8 {
        virt as usize
    } else if virt < 24 {
        8 + (cwp as usize) * 16 + (virt as usize - 8)
    } else {
        let prev_window = ((cwp + 1) % NWINDOWS) as usize;
        8 + prev_window * 16 + (virt as usize - 24)
    }
}

/// PSR (Processor State Register) condition-code fields.
#[derive(Debug, Clone, Copy, Default)]
pub struct Psr {
    pub n: u8,   // Negative
    pub z: u8,   // Zero
    pub v: u8,   // Overflow
    pub c: u8,   // Carry / borrow
}

/// The full register file: physical integer regs, PC, Y, and PSR.
#[derive(Debug, Clone)]
pub struct RegisterFile {
    phys: [u32; NUM_PHYS],
    pub pc: u32,
    pub y: u32,
    pub psr: Psr,
    pub cwp: u32,
    /// Tracks SAVE nesting depth to detect register window overflow.
    pub save_depth: u32,
}

impl Default for RegisterFile {
    fn default() -> Self {
        Self {
            phys: [0u32; NUM_PHYS],
            pc: 0,
            y: 0,
            psr: Psr::default(),
            cwp: 0,
            save_depth: 0,
        }
    }
}

impl RegisterFile {
    pub fn new() -> Self {
        Self::default()
    }

    /// Read a logical register.  `%g0` always reads as zero.
    pub fn read(&self, virt: u32) -> u32 {
        if virt == 0 {
            return 0;
        }
        self.phys[virt_to_phys(virt, self.cwp)]
    }

    /// Write a logical register.  Writes to `%g0` are silently discarded.
    pub fn write(&mut self, virt: u32, val: u32) {
        if virt == 0 {
            return;
        }
        let phys = virt_to_phys(virt, self.cwp);
        self.phys[phys] = val;
    }

    /// SAVE: compute result *before* rotating window (the compute uses the
    /// caller's registers), then decrement CWP.
    ///
    /// Returns the computed `rs1 + src2` that must be written to `rd` in the
    /// *new* window (the callee's `%sp`).
    pub fn save(&mut self, rs1: u32, src2: u32, rd: u32, adder: impl Fn(u32, u32) -> u32) -> Result<u32, &'static str> {
        if self.save_depth >= NWINDOWS - 1 {
            return Err("register window overflow");
        }
        // Compute in caller's window.
        let result = adder(rs1, src2);
        // Rotate CWP.
        self.cwp = (self.cwp + NWINDOWS - 1) % NWINDOWS;
        self.save_depth += 1;
        // Write to callee's %sp (rd in new window).
        self.write(rd, result);
        Ok(result)
    }

    /// RESTORE: compute result in *callee's* window, rotate CWP back, then
    /// write result to `rd` in the *caller's* window.
    pub fn restore(&mut self, rs1: u32, src2: u32, rd: u32, adder: impl Fn(u32, u32) -> u32) -> u32 {
        let result = adder(rs1, src2);
        self.cwp = (self.cwp + 1) % NWINDOWS;
        if self.save_depth > 0 {
            self.save_depth -= 1;
        }
        self.write(rd, result);
        result
    }

    /// Reset all registers and PC to zero.
    pub fn reset(&mut self) {
        self.phys = [0u32; NUM_PHYS];
        self.pc = 0;
        self.y = 0;
        self.psr = Psr::default();
        self.cwp = 0;
        self.save_depth = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globals_are_shared_across_windows() {
        let mut rf = RegisterFile::new();
        rf.cwp = 0;
        rf.write(1, 42);
        rf.cwp = 1;
        // %g1 should still read 42 in window 1.
        assert_eq!(rf.read(1), 42);
    }

    #[test]
    fn out_regs_of_caller_become_in_regs_of_callee() {
        let mut rf = RegisterFile::new();
        rf.cwp = 0;
        rf.write(8, 99);  // %o0 in window 0
        // After SAVE, CWP goes to 2 (i.e., (0+3-1)%3 = 2).
        rf.cwp = 2;
        // %i0 = logical 24; phys = 8 + ((2+1)%3)*16 + 0 = 8 + 0*16 = 8.
        // That is the same physical as CWP=0,%o0.
        assert_eq!(rf.read(24), 99);
    }

    #[test]
    fn g0_always_reads_zero() {
        let mut rf = RegisterFile::new();
        rf.write(0, 0xDEAD_BEEF);
        assert_eq!(rf.read(0), 0);
    }

    #[test]
    fn virt_to_phys_globals() {
        for g in 0..8u32 {
            assert_eq!(virt_to_phys(g, 0), g as usize);
            assert_eq!(virt_to_phys(g, 1), g as usize);
        }
    }

    #[test]
    fn virt_to_phys_windowed_cwp0() {
        // %o0 = logical 8, CWP=0 → physical 8 + 0*16 + 0 = 8
        assert_eq!(virt_to_phys(8, 0), 8);
        // %l0 = logical 16, CWP=0 → physical 8 + 0*16 + 8 = 16
        assert_eq!(virt_to_phys(16, 0), 16);
        // %i0 = logical 24, CWP=0 → physical 8 + 1*16 + 0 = 24
        assert_eq!(virt_to_phys(24, 0), 24);
    }
}
