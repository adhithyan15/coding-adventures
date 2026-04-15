#![no_std]

use core::fmt;
use core::ptr::{read_volatile, write_volatile};

const REG_DATA: usize = 0;
const REG_INTERRUPT_ENABLE: usize = 1;
const REG_FIFO_CONTROL: usize = 2;
const REG_LINE_CONTROL: usize = 3;
const REG_MODEM_CONTROL: usize = 4;
const REG_LINE_STATUS: usize = 5;
const LINE_STATUS_TX_READY: u8 = 1 << 5;

pub struct Uart16550 {
    base: usize,
}

impl Uart16550 {
    /// # Safety
    ///
    /// `base` must point at a valid NS16550-compatible MMIO region.
    pub const unsafe fn new(base: usize) -> Self {
        Self { base }
    }

    pub const fn base(&self) -> usize {
        self.base
    }

    pub fn init(&mut self) {
        self.write_reg(REG_INTERRUPT_ENABLE, 0x00);
        self.write_reg(REG_LINE_CONTROL, 0x80);
        self.write_reg(REG_DATA, 0x03);
        self.write_reg(REG_INTERRUPT_ENABLE, 0x00);
        self.write_reg(REG_LINE_CONTROL, 0x03);
        self.write_reg(REG_FIFO_CONTROL, 0xC7);
        self.write_reg(REG_MODEM_CONTROL, 0x0B);
    }

    pub fn write_byte(&mut self, byte: u8) {
        while self.read_reg(REG_LINE_STATUS) & LINE_STATUS_TX_READY == 0 {}
        self.write_reg(REG_DATA, byte);
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            if byte == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(byte);
        }
    }

    fn read_reg(&self, offset: usize) -> u8 {
        unsafe { read_volatile((self.base + offset) as *const u8) }
    }

    fn write_reg(&self, offset: usize, value: u8) {
        unsafe { write_volatile((self.base + offset) as *mut u8, value) }
    }
}

impl fmt::Write for Uart16550 {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_bytes(s.as_bytes());
        Ok(())
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_address_is_stored() {
        let uart = unsafe { Uart16550::new(0x1000_0000) };
        assert_eq!(uart.base(), 0x1000_0000);
    }
}
