//! D-flip-flop-backed SPARC V8 register windows and control state.

use crate::bits::bits_to_u32;
use crate::state::{clock_bit, clock_two_bits, clock_word};
use sparc_v8_simulator::execute::Psr;

pub const NWINDOWS: u32 = 3;
pub const NUM_PHYS: usize = 56;
pub const MEM_SIZE: usize = 0x1_0000;

/// Map a logical register number (0-31) and CWP to physical storage.
pub fn virt_to_phys(virt: u32, cwp: u32) -> usize {
    assert!(
        virt < 32,
        "virt_to_phys: logical register {virt} out of range 0..31"
    );
    let cwp = cwp % NWINDOWS;
    if virt < 8 {
        virt as usize
    } else if virt < 24 {
        8 + cwp as usize * 16 + (virt as usize - 8)
    } else {
        8 + ((cwp + 1) % NWINDOWS) as usize * 16 + (virt as usize - 24)
    }
}

/// The 1,896 DFFs outside memory and the halt latch: 56 physical registers,
/// PC/nPC, Y, four PSR flags, and two bits each for CWP/save depth.
pub struct RegisterFile {
    phys: [[u8; 32]; NUM_PHYS],
    pc: [u8; 32],
    npc: [u8; 32],
    y: [u8; 32],
    psr: [u8; 4],
    cwp: [u8; 2],
    save_depth: [u8; 2],
}

impl Default for RegisterFile {
    fn default() -> Self {
        let mut result = Self {
            phys: [[0; 32]; NUM_PHYS],
            pc: [0; 32],
            npc: [0; 32],
            y: [0; 32],
            psr: [0; 4],
            cwp: [0; 2],
            save_depth: [0; 2],
        };
        result.npc[2] = 1;
        result
    }
}

impl RegisterFile {
    pub fn new() -> Self {
        Self::default()
    }

    /// Read a logical register. `%g0` is tied to zero.
    pub fn read(&self, virt: u32) -> u32 {
        if virt == 0 {
            0
        } else {
            bits_to_u32(&self.phys[virt_to_phys(virt, self.read_cwp())])
        }
    }

    /// Clock a logical register. Writes to `%g0` are discarded.
    pub fn write(&mut self, virt: u32, value: u32) {
        if virt != 0 {
            let physical = virt_to_phys(virt, self.read_cwp());
            clock_word(&mut self.phys[physical], value);
        }
    }

    pub(crate) fn physical_registers(&self) -> [u32; NUM_PHYS] {
        std::array::from_fn(|index| bits_to_u32(&self.phys[index]))
    }

    pub(crate) fn restore_physical(&mut self, values: [u32; NUM_PHYS]) {
        for (storage, value) in self.phys.iter_mut().zip(values) {
            clock_word(storage, value);
        }
    }

    pub fn read_pc(&self) -> u32 {
        bits_to_u32(&self.pc)
    }

    pub fn write_pc(&mut self, value: u32) {
        clock_word(&mut self.pc, value);
    }

    pub fn read_npc(&self) -> u32 {
        bits_to_u32(&self.npc)
    }

    pub fn write_npc(&mut self, value: u32) {
        clock_word(&mut self.npc, value);
    }

    pub fn read_y(&self) -> u32 {
        bits_to_u32(&self.y)
    }

    pub fn write_y(&mut self, value: u32) {
        clock_word(&mut self.y, value);
    }

    pub fn read_psr(&self) -> Psr {
        Psr {
            n: self.psr[0] != 0,
            z: self.psr[1] != 0,
            v: self.psr[2] != 0,
            c: self.psr[3] != 0,
        }
    }

    pub fn write_psr(&mut self, psr: Psr) {
        for (storage, value) in self.psr.iter_mut().zip([psr.n, psr.z, psr.v, psr.c]) {
            clock_bit(storage, value);
        }
    }

    pub fn read_cwp(&self) -> u32 {
        u32::from(self.cwp[0]) | (u32::from(self.cwp[1]) << 1)
    }

    pub fn write_cwp(&mut self, value: u32) {
        clock_two_bits(&mut self.cwp, value);
    }

    pub fn read_save_depth(&self) -> u32 {
        u32::from(self.save_depth[0]) | (u32::from(self.save_depth[1]) << 1)
    }

    pub fn write_save_depth(&mut self, value: u32) {
        clock_two_bits(&mut self.save_depth, value);
    }

    /// Rotate backward for SAVE after the caller computes the result.
    pub fn rotate_save(&mut self) -> Result<(), &'static str> {
        let depth = self.read_save_depth();
        if depth >= NWINDOWS - 1 {
            return Err("register window overflow");
        }
        self.write_cwp((self.read_cwp() + NWINDOWS - 1) % NWINDOWS);
        self.write_save_depth(depth + 1);
        Ok(())
    }

    /// Rotate forward for RESTORE.
    pub fn rotate_restore(&mut self) {
        self.write_cwp((self.read_cwp() + 1) % NWINDOWS);
        self.write_save_depth(self.read_save_depth().saturating_sub(1));
    }

    /// Clear all persistent register/control DFFs and initialize nPC to four.
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globals_are_shared_across_windows() {
        let mut rf = RegisterFile::new();
        rf.write(1, 42);
        rf.write_cwp(1);
        assert_eq!(rf.read(1), 42);
    }

    #[test]
    fn out_regs_of_caller_become_in_regs_of_callee() {
        let mut rf = RegisterFile::new();
        rf.write(8, 99);
        rf.write_cwp(2);
        assert_eq!(rf.read(24), 99);
    }

    #[test]
    fn g0_always_reads_zero() {
        let mut rf = RegisterFile::new();
        rf.write(0, 0xDEAD_BEEF);
        assert_eq!(rf.read(0), 0);
    }

    #[test]
    fn all_control_fields_clock_through_dffs() {
        let mut rf = RegisterFile::new();
        rf.write_pc(0x1234_5678);
        rf.write_npc(0x1234_567c);
        rf.write_y(0xfeed_beef);
        rf.write_psr(Psr {
            n: true,
            z: false,
            v: true,
            c: true,
        });
        rf.write_cwp(2);
        rf.write_save_depth(1);
        assert_eq!(rf.read_pc(), 0x1234_5678);
        assert_eq!(rf.read_npc(), 0x1234_567c);
        assert_eq!(rf.read_y(), 0xfeed_beef);
        assert_eq!(
            rf.read_psr(),
            Psr {
                n: true,
                z: false,
                v: true,
                c: true
            }
        );
        assert_eq!(rf.read_cwp(), 2);
        assert_eq!(rf.read_save_depth(), 1);
    }

    #[test]
    fn virt_to_phys_windowed_cwp0() {
        assert_eq!(virt_to_phys(8, 0), 8);
        assert_eq!(virt_to_phys(16, 0), 16);
        assert_eq!(virt_to_phys(24, 0), 24);
    }
}
