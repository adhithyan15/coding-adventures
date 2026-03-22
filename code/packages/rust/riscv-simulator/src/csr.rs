//! Control and Status Register (CSR) file for M-mode.

use std::collections::HashMap;

pub const CSR_MSTATUS: u32  = 0x300;
pub const CSR_MTVEC: u32    = 0x305;
pub const CSR_MSCRATCH: u32 = 0x340;
pub const CSR_MEPC: u32     = 0x341;
pub const CSR_MCAUSE: u32   = 0x342;

pub const MIE: u32 = 1 << 3;
pub const CAUSE_ECALL_M_MODE: u32 = 11;

/// Machine-mode Control and Status Register file.
pub struct CSRFile {
    regs: HashMap<u32, u32>,
}

impl CSRFile {
    pub fn new() -> Self {
        Self { regs: HashMap::new() }
    }

    pub fn read(&self, addr: u32) -> u32 {
        self.regs.get(&addr).copied().unwrap_or(0)
    }

    pub fn write(&mut self, addr: u32, value: u32) {
        self.regs.insert(addr, value);
    }

    pub fn read_write(&mut self, addr: u32, new_value: u32) -> u32 {
        let old = self.read(addr);
        self.write(addr, new_value);
        old
    }

    pub fn read_set(&mut self, addr: u32, mask: u32) -> u32 {
        let old = self.read(addr);
        self.write(addr, old | mask);
        old
    }

    pub fn read_clear(&mut self, addr: u32, mask: u32) -> u32 {
        let old = self.read(addr);
        self.write(addr, old & !mask);
        old
    }
}

impl Default for CSRFile {
    fn default() -> Self {
        Self::new()
    }
}
