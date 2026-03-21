//! BIOS Firmware Generator -- the first code that runs on power-on.
//!
//! Generates RISC-V machine code for the BIOS boot sequence:
//! 1. Memory probe (or use configured size)
//! 2. IDT initialization (256 entries)
//! 3. HardwareInfo write at 0x00001000
//! 4. Jump to bootloader at 0x00010000

use riscv_simulator::encoding::*;
use crate::rom::DEFAULT_ROM_BASE;

// Well-known addresses
const ISR_STUB_BASE: u32 = 0x00000800;
const PROBE_START: u32 = 0x00100000;
const PROBE_STEP: u32 = 0x00100000;
const PROBE_LIMIT: u32 = 0xFFFB0000;
const DEFAULT_BOOTLOADER_ENTRY: u32 = 0x00010000;
const DEFAULT_FRAMEBUFFER_BASE: u32 = 0xFFFB0000;
const HARDWARE_INFO_ADDR: u32 = 0x00001000;

/// BIOS configuration.
#[derive(Debug, Clone)]
pub struct BiosConfig {
    pub memory_size: u32,
    pub display_columns: u32,
    pub display_rows: u32,
    pub framebuffer_base: u32,
    pub bootloader_entry: u32,
}

impl Default for BiosConfig {
    fn default() -> Self {
        Self {
            memory_size: 0,
            display_columns: 80,
            display_rows: 25,
            framebuffer_base: DEFAULT_FRAMEBUFFER_BASE,
            bootloader_entry: DEFAULT_BOOTLOADER_ENTRY,
        }
    }
}

/// Annotated instruction with address, machine code, assembly, and comment.
#[derive(Debug, Clone)]
pub struct AnnotatedInstruction {
    pub address: u32,
    pub machine_code: u32,
    pub assembly: String,
    pub comment: String,
}

/// Generates BIOS firmware as RISC-V machine code.
pub struct BiosFirmware {
    pub config: BiosConfig,
}

impl BiosFirmware {
    pub fn new(config: BiosConfig) -> Self {
        Self { config }
    }

    /// Return firmware as raw bytes (little-endian RISC-V machine code).
    pub fn generate(&self) -> Vec<u8> {
        let annotated = self.generate_with_comments();
        let words: Vec<u32> = annotated.iter().map(|a| a.machine_code).collect();
        assemble(&words)
    }

    /// Return firmware as annotated instructions.
    pub fn generate_with_comments(&self) -> Vec<AnnotatedInstruction> {
        let mut instructions = Vec::new();
        let mut address = DEFAULT_ROM_BASE;

        let emit = |insts: &mut Vec<AnnotatedInstruction>, addr: &mut u32,
                        code: u32, asm: &str, comment: &str| {
            insts.push(AnnotatedInstruction {
                address: *addr,
                machine_code: code,
                assembly: asm.to_string(),
                comment: comment.to_string(),
            });
            *addr += 4;
        };

        // === Step 1: Memory Probe ===
        if self.config.memory_size > 0 {
            let upper = (self.config.memory_size >> 12) & 0xFFFFF;
            let lower = self.config.memory_size & 0xFFF;
            emit(&mut instructions, &mut address,
                encode_lui(8, upper),
                &format!("lui x8, 0x{:05X}", upper),
                &format!("Step 1: Load configured memory size ({} bytes)", self.config.memory_size));
            if lower != 0 {
                emit(&mut instructions, &mut address,
                    encode_addi(8, 8, sign_extend_12(lower) as i32),
                    &format!("addi x8, x8, 0x{:03X}", lower),
                    "Step 1: Add lower 12 bits");
            }
        } else {
            emit(&mut instructions, &mut address,
                encode_lui(5, PROBE_START >> 12),
                &format!("lui x5, 0x{:05X}", PROBE_START >> 12),
                "Step 1: x5 = 0x00100000 (probe start)");
            emit(&mut instructions, &mut address,
                encode_lui(6, 0xDEADC),
                "lui x6, 0xDEADC",
                "Step 1: x6 upper = 0xDEADC000 (compensated)");
            emit(&mut instructions, &mut address,
                encode_addi(6, 6, sign_extend_12(0xEEF) as i32),
                &format!("addi x6, x6, {}", sign_extend_12(0xEEF)),
                "Step 1: x6 = 0xDEADBEEF (test pattern)");
            emit(&mut instructions, &mut address,
                encode_lui(9, PROBE_LIMIT >> 12),
                &format!("lui x9, 0x{:05X}", PROBE_LIMIT >> 12),
                "Step 1: x9 = 0xFFFB0000 (probe limit)");
            emit(&mut instructions, &mut address,
                encode_lui(10, PROBE_STEP >> 12),
                &format!("lui x10, 0x{:05X}", PROBE_STEP >> 12),
                "Step 1: x10 = 0x00100000 (1 MB step)");
            emit(&mut instructions, &mut address,
                encode_sw(6, 5, 0), "sw x6, 0(x5)", "Step 1: Write test pattern");
            emit(&mut instructions, &mut address,
                encode_lw(7, 5, 0), "lw x7, 0(x5)", "Step 1: Read it back");
            emit(&mut instructions, &mut address,
                encode_bne(6, 7, 12), "bne x6, x7, +12", "Step 1: If mismatch, done");
            emit(&mut instructions, &mut address,
                encode_add(5, 5, 10), "add x5, x5, x10", "Step 1: Advance by 1 MB");
            emit(&mut instructions, &mut address,
                encode_blt(5, 9, -16), "blt x5, x9, -16", "Step 1: Loop if below limit");
            emit(&mut instructions, &mut address,
                encode_add(8, 5, 0), "add x8, x5, x0", "Step 1: x8 = detected memory size");
        }

        // === Step 2: IDT Initialization ===
        emit(&mut instructions, &mut address,
            encode_lui(11, ISR_STUB_BASE >> 12),
            &format!("lui x11, 0x{:05X}", ISR_STUB_BASE >> 12),
            "Step 2a: x11 = ISR stub base");
        if ISR_STUB_BASE & 0xFFF != 0 {
            emit(&mut instructions, &mut address,
                encode_addi(11, 11, (ISR_STUB_BASE & 0xFFF) as i32),
                &format!("addi x11, x11, {}", ISR_STUB_BASE & 0xFFF),
                "Step 2a: Add lower bits");
        }

        let fault_instr = encode_jal(0, 0);
        let upper_f = li_upper(fault_instr);
        emit(&mut instructions, &mut address,
            encode_lui(12, upper_f),
            &format!("lui x12, 0x{:05X}", upper_f),
            "Step 2a: Load fault handler instruction");
        if fault_instr & 0xFFF != 0 {
            emit(&mut instructions, &mut address,
                encode_addi(12, 12, sign_extend_12(fault_instr & 0xFFF) as i32),
                &format!("addi x12, x12, {}", sign_extend_12(fault_instr & 0xFFF)),
                "Step 2a: Fault handler lower bits");
        }
        emit(&mut instructions, &mut address,
            encode_sw(12, 11, 0), "sw x12, 0(x11)", "Step 2a: Store fault handler at 0x800");
        emit(&mut instructions, &mut address,
            encode_sw(0, 11, 4), "sw x0, 4(x11)", "Step 2a: NOP at 0x804");

        let mret_instr = encode_mret();
        let upper_m = li_upper(mret_instr);
        emit(&mut instructions, &mut address,
            encode_lui(12, upper_m),
            &format!("lui x12, 0x{:05X}", upper_m),
            "Step 2a: Load mret instruction");
        if mret_instr & 0xFFF != 0 {
            emit(&mut instructions, &mut address,
                encode_addi(12, 12, sign_extend_12(mret_instr & 0xFFF) as i32),
                &format!("addi x12, x12, {}", sign_extend_12(mret_instr & 0xFFF)),
                "Step 2a: mret lower bits");
        }
        emit(&mut instructions, &mut address,
            encode_sw(12, 11, 8), "sw x12, 8(x11)", "Step 2a: timer_isr at 0x808");
        emit(&mut instructions, &mut address,
            encode_sw(12, 11, 16), "sw x12, 16(x11)", "Step 2a: keyboard_isr at 0x810");
        emit(&mut instructions, &mut address,
            encode_sw(12, 11, 24), "sw x12, 24(x11)", "Step 2a: syscall_isr at 0x818");

        // IDT entries
        emit(&mut instructions, &mut address,
            encode_addi(13, 0, 0), "addi x13, x0, 0", "Step 2b: x13 = IDT base");
        emit(&mut instructions, &mut address,
            encode_lui(14, 1), "lui x14, 0x00001", "Step 2b: x14 = 0x1000");
        emit(&mut instructions, &mut address,
            encode_addi(14, 14, -2048), "addi x14, x14, -2048", "Step 2b: x14 = 0x800");
        emit(&mut instructions, &mut address,
            encode_lui(16, 1), "lui x16, 0x00001", "Step 2b: x16 = 0x1000");
        emit(&mut instructions, &mut address,
            encode_addi(16, 16, -2048), "addi x16, x16, -2048", "Step 2b: x16 = 0x800 (IDT end)");
        emit(&mut instructions, &mut address,
            encode_addi(17, 0, 1), "addi x17, x0, 1", "Step 2b: x17 = 1 (flags)");

        emit(&mut instructions, &mut address, encode_lui(18, 1), "lui x18, 0x00001", "Step 2b: x18");
        emit(&mut instructions, &mut address, encode_addi(18, 18, -2040), "addi x18, x18, -2040", "Step 2b: x18 = 0x808");
        emit(&mut instructions, &mut address, encode_lui(19, 1), "lui x19, 0x00001", "Step 2b: x19");
        emit(&mut instructions, &mut address, encode_addi(19, 19, -2032), "addi x19, x19, -2032", "Step 2b: x19 = 0x810");
        emit(&mut instructions, &mut address, encode_lui(20, 1), "lui x20, 0x00001", "Step 2b: x20");
        emit(&mut instructions, &mut address, encode_addi(20, 20, -2024), "addi x20, x20, -2024", "Step 2b: x20 = 0x818");

        emit(&mut instructions, &mut address, encode_addi(21, 0, 256), "addi x21, x0, 256", "Step 2b: x21 = 256 (timer)");
        emit(&mut instructions, &mut address, encode_addi(22, 0, 264), "addi x22, x0, 264", "Step 2b: x22 = 264 (keyboard)");
        emit(&mut instructions, &mut address, encode_addi(23, 0, 1024), "addi x23, x0, 1024", "Step 2b: x23 = 1024 (syscall)");

        let loop_start = address;
        emit(&mut instructions, &mut address, encode_beq(13, 21, 20), "beq x13, x21, +20", "Step 2b: Timer?");
        emit(&mut instructions, &mut address, encode_beq(13, 22, 24), "beq x13, x22, +24", "Step 2b: Keyboard?");
        emit(&mut instructions, &mut address, encode_beq(13, 23, 28), "beq x13, x23, +28", "Step 2b: Syscall?");
        emit(&mut instructions, &mut address, encode_sw(14, 13, 0), "sw x14, 0(x13)", "Step 2b: Default handler");
        emit(&mut instructions, &mut address, encode_jal(0, 24), "jal x0, +24", "Step 2b: Skip");
        emit(&mut instructions, &mut address, encode_sw(18, 13, 0), "sw x18, 0(x13)", "Step 2b: Timer ISR");
        emit(&mut instructions, &mut address, encode_jal(0, 16), "jal x0, +16", "Step 2b: Skip");
        emit(&mut instructions, &mut address, encode_sw(19, 13, 0), "sw x19, 0(x13)", "Step 2b: Keyboard ISR");
        emit(&mut instructions, &mut address, encode_jal(0, 8), "jal x0, +8", "Step 2b: Skip");
        emit(&mut instructions, &mut address, encode_sw(20, 13, 0), "sw x20, 0(x13)", "Step 2b: Syscall ISR");
        emit(&mut instructions, &mut address, encode_sw(17, 13, 4), "sw x17, 4(x13)", "Step 2b: Store flags");
        emit(&mut instructions, &mut address, encode_addi(13, 13, 8), "addi x13, x13, 8", "Step 2b: Next entry");
        let loop_offset = loop_start as i32 - address as i32;
        emit(&mut instructions, &mut address,
            encode_blt(13, 16, loop_offset), &format!("blt x13, x16, {}", loop_offset), "Step 2b: Loop");

        // === Step 3: HardwareInfo ===
        emit(&mut instructions, &mut address,
            encode_lui(5, HARDWARE_INFO_ADDR >> 12),
            &format!("lui x5, 0x{:05X}", HARDWARE_INFO_ADDR >> 12),
            "Step 3: x5 = HardwareInfo base");
        emit(&mut instructions, &mut address,
            encode_sw(8, 5, 0), "sw x8, 0(x5)", "Step 3: MemorySize");
        emit(&mut instructions, &mut address,
            encode_addi(6, 0, self.config.display_columns as i32),
            &format!("addi x6, x0, {}", self.config.display_columns),
            "Step 3: DisplayColumns");
        emit(&mut instructions, &mut address,
            encode_sw(6, 5, 4), "sw x6, 4(x5)", "Step 3: Store DisplayColumns");
        emit(&mut instructions, &mut address,
            encode_addi(6, 0, self.config.display_rows as i32),
            &format!("addi x6, x0, {}", self.config.display_rows),
            "Step 3: DisplayRows");
        emit(&mut instructions, &mut address,
            encode_sw(6, 5, 8), "sw x6, 8(x5)", "Step 3: Store DisplayRows");

        let fb_upper = self.config.framebuffer_base >> 12;
        let fb_lower = self.config.framebuffer_base & 0xFFF;
        emit(&mut instructions, &mut address,
            encode_lui(6, fb_upper),
            &format!("lui x6, 0x{:05X}", fb_upper),
            "Step 3: FramebufferBase upper");
        if fb_lower != 0 {
            emit(&mut instructions, &mut address,
                encode_addi(6, 6, sign_extend_12(fb_lower) as i32),
                &format!("addi x6, x6, {}", sign_extend_12(fb_lower)),
                "Step 3: FramebufferBase lower");
        }
        emit(&mut instructions, &mut address,
            encode_sw(6, 5, 12), "sw x6, 12(x5)", "Step 3: Store FramebufferBase");
        emit(&mut instructions, &mut address,
            encode_sw(0, 5, 16), "sw x0, 16(x5)", "Step 3: IDTBase = 0");
        emit(&mut instructions, &mut address,
            encode_addi(6, 0, 256), "addi x6, x0, 256", "Step 3: IDTEntries");
        emit(&mut instructions, &mut address,
            encode_sw(6, 5, 20), "sw x6, 20(x5)", "Step 3: Store IDTEntries");

        let bl_upper = self.config.bootloader_entry >> 12;
        let bl_lower = self.config.bootloader_entry & 0xFFF;
        emit(&mut instructions, &mut address,
            encode_lui(6, bl_upper),
            &format!("lui x6, 0x{:05X}", bl_upper),
            "Step 3: BootloaderEntry upper");
        if bl_lower != 0 {
            emit(&mut instructions, &mut address,
                encode_addi(6, 6, sign_extend_12(bl_lower) as i32),
                &format!("addi x6, x6, {}", sign_extend_12(bl_lower)),
                "Step 3: BootloaderEntry lower");
        }
        emit(&mut instructions, &mut address,
            encode_sw(6, 5, 24), "sw x6, 24(x5)", "Step 3: Store BootloaderEntry");

        // === Step 4: Jump to Bootloader ===
        emit(&mut instructions, &mut address,
            encode_lui(6, self.config.bootloader_entry >> 12),
            &format!("lui x6, 0x{:05X}", self.config.bootloader_entry >> 12),
            "Step 4: Bootloader entry upper");
        if self.config.bootloader_entry & 0xFFF != 0 {
            emit(&mut instructions, &mut address,
                encode_addi(6, 6, sign_extend_12(self.config.bootloader_entry & 0xFFF) as i32),
                &format!("addi x6, x6, {}", sign_extend_12(self.config.bootloader_entry & 0xFFF)),
                "Step 4: Bootloader entry lower");
        }
        emit(&mut instructions, &mut address,
            encode_jalr(0, 6, 0), "jalr x0, x6, 0",
            &format!("Step 4: Jump to bootloader at 0x{:08X}", self.config.bootloader_entry));

        instructions
    }
}

/// Sign-extend a 12-bit value.
fn sign_extend_12(val: u32) -> i32 {
    let val = val & 0xFFF;
    if val >= 0x800 { val as i32 - 0x1000 } else { val as i32 }
}

/// Compute upper 20-bit value for LUI, compensating for ADDI sign extension.
fn li_upper(value: u32) -> u32 {
    let mut upper = (value >> 12) & 0xFFFFF;
    if value & 0x800 != 0 {
        upper = (upper + 1) & 0xFFFFF;
    }
    upper
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rom::{Rom, RomConfig, DEFAULT_ROM_BASE, DEFAULT_ROM_SIZE};
    use crate::hardware_info::{HardwareInfo, HARDWARE_INFO_SIZE};

    // === ROM Tests ===

    #[test]
    fn test_rom_loads_firmware() {
        let rom = Rom::new(RomConfig::default(), &[0xAA, 0xBB, 0xCC, 0xDD]);
        assert_eq!(rom.size(), DEFAULT_ROM_SIZE);
    }

    #[test]
    #[should_panic(expected = "firmware larger than ROM size")]
    fn test_rom_panics_on_oversized() {
        let oversized = vec![0u8; DEFAULT_ROM_SIZE + 1];
        Rom::new(RomConfig::default(), &oversized);
    }

    #[test]
    fn test_rom_read_byte() {
        let rom = Rom::new(RomConfig::default(), &[0x12, 0x34, 0x56, 0x78]);
        assert_eq!(rom.read(DEFAULT_ROM_BASE), 0x12);
        assert_eq!(rom.read(DEFAULT_ROM_BASE + 1), 0x34);
        assert_eq!(rom.read(DEFAULT_ROM_BASE + 3), 0x78);
    }

    #[test]
    fn test_rom_read_word() {
        let rom = Rom::new(RomConfig::default(), &[0x78, 0x56, 0x34, 0x12]);
        assert_eq!(rom.read_word(DEFAULT_ROM_BASE), 0x12345678);
    }

    #[test]
    fn test_rom_write_ignored() {
        let rom = Rom::new(RomConfig::default(), &[0xAA, 0xBB]);
        rom.write(DEFAULT_ROM_BASE, 0xFF);
        assert_eq!(rom.read(DEFAULT_ROM_BASE), 0xAA);
    }

    #[test]
    fn test_rom_out_of_range() {
        let rom = Rom::new(RomConfig::default(), &[0xAA]);
        assert_eq!(rom.read(0x00000000), 0);
        assert_eq!(rom.read_word(0x00000000), 0);
    }

    #[test]
    fn test_rom_contains() {
        let rom = Rom::new(RomConfig::default(), &[0xAA]);
        assert!(rom.contains(DEFAULT_ROM_BASE));
        assert!(rom.contains(0xFFFFFFFF));
        assert!(!rom.contains(DEFAULT_ROM_BASE - 1));
        assert!(!rom.contains(0x00000000));
    }

    #[test]
    fn test_rom_empty() {
        let rom = Rom::new(RomConfig::default(), &[]);
        assert_eq!(rom.read_word(DEFAULT_ROM_BASE), 0);
    }

    // === HardwareInfo Tests ===

    #[test]
    fn test_hardware_info_defaults() {
        let info = HardwareInfo::default();
        assert_eq!(info.memory_size, 0);
        assert_eq!(info.display_columns, 80);
        assert_eq!(info.display_rows, 25);
        assert_eq!(info.framebuffer_base, 0xFFFB0000);
        assert_eq!(info.idt_entries, 256);
        assert_eq!(info.bootloader_entry, 0x00010000);
    }

    #[test]
    fn test_hardware_info_roundtrip() {
        let info = HardwareInfo { memory_size: 64 * 1024 * 1024, ..Default::default() };
        let bytes = info.to_bytes();
        assert_eq!(bytes.len(), HARDWARE_INFO_SIZE);
        let restored = HardwareInfo::from_bytes(&bytes);
        assert_eq!(restored, info);
    }

    #[test]
    #[should_panic(expected = "data too short")]
    fn test_hardware_info_short_data() {
        HardwareInfo::from_bytes(&[0x01, 0x02]);
    }

    // === BIOS Tests ===

    #[test]
    fn test_generate_non_empty() {
        let bios = BiosFirmware::new(BiosConfig::default());
        assert!(!bios.generate().is_empty());
    }

    #[test]
    fn test_generate_word_aligned() {
        let code = BiosFirmware::new(BiosConfig::default()).generate();
        assert_eq!(code.len() % 4, 0);
    }

    #[test]
    fn test_generate_deterministic() {
        let config = BiosConfig::default();
        let code1 = BiosFirmware::new(config.clone()).generate();
        let code2 = BiosFirmware::new(config).generate();
        assert_eq!(code1, code2);
    }

    #[test]
    fn test_configurable_different() {
        let code1 = BiosFirmware::new(BiosConfig::default()).generate();
        let code2 = BiosFirmware::new(BiosConfig { memory_size: 128 * 1024 * 1024, ..Default::default() }).generate();
        assert_ne!(code1, code2);
    }

    #[test]
    fn test_configured_shorter() {
        let probe = BiosFirmware::new(BiosConfig::default()).generate();
        let fixed = BiosFirmware::new(BiosConfig { memory_size: 64 * 1024 * 1024, ..Default::default() }).generate();
        assert!(fixed.len() < probe.len());
    }

    #[test]
    fn test_fits_in_rom() {
        let code = BiosFirmware::new(BiosConfig::default()).generate();
        assert!(code.len() <= DEFAULT_ROM_SIZE);
    }

    #[test]
    fn test_load_into_rom() {
        let bios = BiosFirmware::new(BiosConfig::default());
        let code = bios.generate();
        let rom = Rom::new(RomConfig::default(), &code);
        let expected = u32::from_le_bytes([code[0], code[1], code[2], code[3]]);
        assert_eq!(rom.read_word(DEFAULT_ROM_BASE), expected);
    }

    // === Annotated Tests ===

    #[test]
    fn test_annotated_matches_generate() {
        let bios = BiosFirmware::new(BiosConfig::default());
        let code = bios.generate();
        let annotated = bios.generate_with_comments();
        assert_eq!(annotated.len() * 4, code.len());
        for (i, inst) in annotated.iter().enumerate() {
            let off = i * 4;
            let expected = u32::from_le_bytes([code[off], code[off+1], code[off+2], code[off+3]]);
            assert_eq!(inst.machine_code, expected, "instruction {} mismatch", i);
        }
    }

    #[test]
    fn test_annotated_continuity() {
        let annotated = BiosFirmware::new(BiosConfig::default()).generate_with_comments();
        assert!(!annotated.is_empty());
        assert_eq!(annotated[0].address, DEFAULT_ROM_BASE);
        for i in 1..annotated.len() {
            assert_eq!(annotated[i].address, annotated[i-1].address + 4);
        }
    }

    #[test]
    fn test_annotated_non_empty_strings() {
        for (i, inst) in BiosFirmware::new(BiosConfig::default()).generate_with_comments().iter().enumerate() {
            assert!(!inst.assembly.is_empty(), "instruction {} empty assembly", i);
            assert!(!inst.comment.is_empty(), "instruction {} empty comment", i);
        }
    }

    #[test]
    fn test_annotated_mnemonics() {
        let annotated = BiosFirmware::new(BiosConfig::default()).generate_with_comments();
        let mnemonics = ["lui", "addi", "sw", "jalr"];
        for m in &mnemonics {
            assert!(annotated.iter().any(|inst| inst.assembly.starts_with(&format!("{} ", m))),
                "mnemonic {} not found", m);
        }
    }

    #[test]
    fn test_last_is_jump() {
        let annotated = BiosFirmware::new(BiosConfig::default()).generate_with_comments();
        assert!(annotated.last().unwrap().assembly.starts_with("jalr"));
    }

    #[test]
    fn test_sign_extend_12() {
        assert_eq!(sign_extend_12(0), 0);
        assert_eq!(sign_extend_12(2047), 2047);
        assert_eq!(sign_extend_12(0x800), -2048);
        assert_eq!(sign_extend_12(0xFFF), -1);
        assert_eq!(sign_extend_12(0xEEF), -273);
    }
}
