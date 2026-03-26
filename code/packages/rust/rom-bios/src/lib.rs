//! # ROM & BIOS Firmware
//!
//! Implements the very first code that runs when the simulated computer
//! powers on. ROM (Read-Only Memory) at 0xFFFF0000 contains BIOS firmware
//! that initializes hardware and jumps to the bootloader.

pub mod rom;
pub mod hardware_info;
pub mod bios;

pub use rom::{Rom, RomConfig, DEFAULT_ROM_BASE, DEFAULT_ROM_SIZE};
pub use hardware_info::{HardwareInfo, HARDWARE_INFO_ADDRESS, HARDWARE_INFO_SIZE};
pub use bios::{BiosFirmware, BiosConfig, AnnotatedInstruction};
