//! Windowed register file for the SPARC V8 ISA.
//!
//! SPARC does not have a flat register file.  Instead, 32 *logical*
//! registers are visible in any window:
//!
//! ```text
//! %g0-%g7   (r0-r7)    -- globals, shared by every window; %g0 hardwired zero
//! %o0-%o7   (r8-r15)   -- "out" registers (arguments to callees, %o6=%sp, %o7=link)
//! %l0-%l7   (r16-r23)  -- "local" registers, private to this window
//! %i0-%i7   (r24-r31)  -- "in" registers (arguments from callers, %i6=%fp, %i7=return addr-8)
//! ```
//!
//! ...mapped onto a larger *physical* register file via a Current Window
//! Pointer (CWP).  With `NWINDOWS` windows, the physical file has
//! `8 + NWINDOWS * 16` registers: 8 shared globals, plus 16
//! (outs+locals) per window.  The *ins* of window `W` are the *outs* of
//! window `(W+1) % NWINDOWS` — this is the mechanism that makes
//! argument passing across a `SAVE`/`RESTORE` boundary free (no copying).
//!
//! `SAVE` decrements CWP (mod `NWINDOWS`) — the caller's outs become the
//! callee's ins.  `RESTORE` increments it back.
//!
//! This is a straight transcription of
//! `sparc_v8_simulator.state.{NUM_PHYS,NWINDOWS,WINDOW_BASE,virt_to_phys}`
//! (Python) — cross-checked bit-for-bit against
//! `sparc-v8-gatelevel::register_file::virt_to_phys` (the in-tree
//! gate-level SPARC V8 port), which implements the identical formula.

/// Number of register windows.  Real SPARC V8 chips shipped 7-32
/// windows; this simulator (matching the Python original and the
/// gate-level port) uses 3 — the smallest count that still lets `SAVE`
/// nest more than once before overflowing.
pub const NWINDOWS: u32 = 3;

/// Total physical registers: 8 shared globals + `NWINDOWS * 16`
/// (outs+locals per window).
pub const NUM_PHYS: usize = 8 + (NWINDOWS as usize) * 16; // 56

/// Physical base index of each window's 16 (outs+locals) registers:
/// `[8, 24, 40]` for `NWINDOWS = 3`.
fn window_base(w: u32) -> usize {
    8 + (w as usize) * 16
}

/// Map a logical (virtual) register number (0-31) and the current CWP to
/// a physical register index (0..[`NUM_PHYS`]).
///
/// ```text
/// virt 0-7   (globals):        phys = virt                       (CWP-independent)
/// virt 8-23  (outs+locals):    phys = window_base(cwp) + (virt-8)
/// virt 24-31 (ins):            phys = window_base((cwp+1) % NWINDOWS) + (virt-24)
/// ```
pub fn virt_to_phys(virt: u32, cwp: u32) -> usize {
    debug_assert!(virt < 32, "virt_to_phys: logical register {virt} out of range 0..31");
    let cwp = cwp % NWINDOWS;
    if virt < 8 {
        virt as usize
    } else if virt < 24 {
        window_base(cwp) + (virt as usize - 8)
    } else {
        let next_w = (cwp + 1) % NWINDOWS;
        window_base(next_w) + (virt as usize - 24)
    }
}

/// The physical register file plus the Current Window Pointer.
///
/// Deliberately does **not** own PC/PSR/Y — those live on
/// [`crate::simulator::SparcV8Simulator`] directly, mirroring how
/// `mips-r2000-simulator::MipsR2000Simulator` keeps `hi`/`lo` on the
/// top-level struct rather than inside `cpu_simulator::RegisterFile`.
#[derive(Debug, Clone)]
pub struct RegisterWindowFile {
    phys: [u32; NUM_PHYS],
    /// Current window pointer, `0..NWINDOWS`.
    pub cwp: u32,
    /// Number of outstanding `SAVE` frames (tracks window-overflow risk;
    /// mirrors the Python original's `_save_depth` / the gate-level
    /// port's `save_depth`).
    pub save_depth: u32,
}

impl Default for RegisterWindowFile {
    fn default() -> Self {
        Self {
            phys: [0u32; NUM_PHYS],
            cwp: 0,
            save_depth: 0,
        }
    }
}

impl RegisterWindowFile {
    pub fn new() -> Self {
        Self::default()
    }

    /// Read a logical register in the current window.  `%g0` always
    /// reads as zero.
    pub fn read(&self, virt: u32) -> u32 {
        if virt == 0 {
            return 0;
        }
        self.phys[virt_to_phys(virt, self.cwp)]
    }

    /// Write a logical register in the current window.  Writes to `%g0`
    /// are silently discarded.
    pub fn write(&mut self, virt: u32, val: u32) {
        if virt != 0 {
            let phys = virt_to_phys(virt, self.cwp);
            self.phys[phys] = val;
        }
    }

    /// Rotate the window backward for `SAVE` (procedure entry).
    ///
    /// Returns `Err` on register-window overflow — with `NWINDOWS`
    /// windows only `NWINDOWS - 1` nested `SAVE`s are legal (the
    /// `NWINDOWS`-th would wrap around and clobber the outermost live
    /// frame).  Real hardware detects this via the WIM register; this
    /// simulator tracks nesting depth directly, matching the Python
    /// original.
    pub fn rotate_save(&mut self) -> Result<(), &'static str> {
        if self.save_depth >= NWINDOWS - 1 {
            return Err("register window overflow");
        }
        self.cwp = (self.cwp + NWINDOWS - 1) % NWINDOWS;
        self.save_depth += 1;
        Ok(())
    }

    /// Rotate the window forward for `RESTORE` (procedure exit).
    pub fn rotate_restore(&mut self) {
        self.cwp = (self.cwp + 1) % NWINDOWS;
        self.save_depth = self.save_depth.saturating_sub(1);
    }

    /// Reset every physical register, CWP, and save depth to zero.
    pub fn reset(&mut self) {
        self.phys = [0u32; NUM_PHYS];
        self.cwp = 0;
        self.save_depth = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globals_are_shared_across_windows() {
        let mut rf = RegisterWindowFile::new();
        rf.write(1, 42);
        rf.cwp = 1;
        assert_eq!(rf.read(1), 42);
    }

    #[test]
    fn g0_always_reads_zero() {
        let mut rf = RegisterWindowFile::new();
        rf.write(0, 0xDEAD_BEEF);
        assert_eq!(rf.read(0), 0);
    }

    #[test]
    fn out_regs_of_caller_become_in_regs_of_callee_after_save() {
        let mut rf = RegisterWindowFile::new();
        rf.write(8, 99); // %o0 in window 0
        rf.rotate_save().unwrap();
        assert_eq!(rf.cwp, NWINDOWS - 1);
        // %i0 (virt 24) in the new window aliases %o0 of window 0.
        assert_eq!(rf.read(24), 99);
    }

    #[test]
    fn restore_undoes_save() {
        let mut rf = RegisterWindowFile::new();
        rf.write(8, 7);
        rf.rotate_save().unwrap();
        rf.rotate_restore();
        assert_eq!(rf.cwp, 0);
        assert_eq!(rf.read(8), 7);
    }

    #[test]
    fn save_overflow_detected() {
        let mut rf = RegisterWindowFile::new();
        for _ in 0..NWINDOWS - 1 {
            rf.rotate_save().unwrap();
        }
        assert!(rf.rotate_save().is_err());
    }

    #[test]
    fn virt_to_phys_globals_are_cwp_independent() {
        for g in 0..8u32 {
            assert_eq!(virt_to_phys(g, 0), g as usize);
            assert_eq!(virt_to_phys(g, 1), g as usize);
            assert_eq!(virt_to_phys(g, 2), g as usize);
        }
    }

    #[test]
    fn virt_to_phys_matches_python_derivation_example() {
        // From state.py's docstring: CWP=0, NWINDOWS=3.
        assert_eq!(virt_to_phys(8, 0), 8); // %o0
        assert_eq!(virt_to_phys(16, 0), 16); // %l0
        assert_eq!(virt_to_phys(24, 0), 24); // %i0 = window-1 outs
    }
}
