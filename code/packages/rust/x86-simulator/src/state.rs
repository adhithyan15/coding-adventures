//! CPU state for the x86-64 runtime simulator.
//!
//! The architectural state a 64-bit x86 program observes, restricted to what
//! the in-repo `x86_64-backend` actually uses: the 16 general-purpose registers,
//! the instruction pointer, and the arithmetic flags. (Segment registers, the
//! x87 stack, control/debug registers, etc. are out of scope — see the spec.)
//!
//! Register *numbering* follows the hardware encoding so a decoded ModRM/REX
//! register index maps straight onto `gpr[idx]`:
//!
//! | idx | 0   | 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8..15 |
//! |-----|-----|-----|-----|-----|-----|-----|-----|-----|-------|
//! | reg | RAX | RCX | RDX | RBX | RSP | RBP | RSI | RDI | R8..R15 |

/// The 16 x86-64 general-purpose registers, by hardware index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Reg {
    Rax = 0, Rcx = 1, Rdx = 2, Rbx = 3,
    Rsp = 4, Rbp = 5, Rsi = 6, Rdi = 7,
    R8 = 8, R9 = 9, R10 = 10, R11 = 11,
    R12 = 12, R13 = 13, R14 = 14, R15 = 15,
}

impl Reg {
    /// Map a 0..=15 hardware register number to a [`Reg`].
    pub fn from_index(i: u8) -> Reg {
        match i & 0xF {
            0 => Reg::Rax, 1 => Reg::Rcx, 2 => Reg::Rdx, 3 => Reg::Rbx,
            4 => Reg::Rsp, 5 => Reg::Rbp, 6 => Reg::Rsi, 7 => Reg::Rdi,
            8 => Reg::R8, 9 => Reg::R9, 10 => Reg::R10, 11 => Reg::R11,
            12 => Reg::R12, 13 => Reg::R13, 14 => Reg::R14, _ => Reg::R15,
        }
    }
}

/// The arithmetic subset of RFLAGS the backend's comparisons and branches use.
///
/// (CF/ZF/SF/OF/PF/AF — the flags `add`/`sub`/`cmp`/`test`/`shl` set and the
/// `jcc`/`setcc`/`cmovcc` family reads. The control flags — IF, DF, TF — are
/// not exercised by emitted code.)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Flags {
    /// Carry — unsigned overflow / borrow.
    pub cf: bool,
    /// Zero — result was 0.
    pub zf: bool,
    /// Sign — high bit of the result.
    pub sf: bool,
    /// Overflow — signed overflow.
    pub of: bool,
    /// Parity — even number of set bits in the low byte.
    pub pf: bool,
    /// Auxiliary carry — carry out of bit 3 (BCD); tracked for completeness.
    pub af: bool,
}

/// The full CPU state.
#[derive(Debug, Clone, Default)]
pub struct CpuState {
    /// The 16 GPRs, indexed by [`Reg`] as `usize`.
    pub gpr: [u64; 16],
    /// Instruction pointer (next instruction to fetch).
    pub rip: u64,
    /// Arithmetic flags.
    pub flags: Flags,
    /// XMM register file (raw 128-bit lanes) — used by the SSE2 phase. Present
    /// now so the integer core and the float phase share one state type.
    pub xmm: [u128; 16],
}

impl CpuState {
    /// Read a register's full 64-bit value.
    #[inline]
    pub fn get(&self, r: Reg) -> u64 { self.gpr[r as usize] }

    /// Write a register's full 64-bit value.
    #[inline]
    pub fn set(&mut self, r: Reg, v: u64) { self.gpr[r as usize] = v; }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reg_index_round_trips_the_hardware_numbering() {
        assert_eq!(Reg::from_index(0), Reg::Rax);
        assert_eq!(Reg::from_index(4), Reg::Rsp);
        assert_eq!(Reg::from_index(5), Reg::Rbp);
        assert_eq!(Reg::from_index(15), Reg::R15);
        // The low nibble selects the register (REX.B is folded in by the decoder).
        assert_eq!(Reg::from_index(0x10), Reg::Rax);
    }

    #[test]
    fn get_set_round_trip() {
        let mut s = CpuState::default();
        s.set(Reg::Rdi, 0xDEAD_BEEF);
        assert_eq!(s.get(Reg::Rdi), 0xDEAD_BEEF);
        assert_eq!(s.get(Reg::Rax), 0);
    }
}
