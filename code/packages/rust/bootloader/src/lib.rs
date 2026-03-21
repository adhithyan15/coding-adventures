//! S02 Bootloader -- generates RISC-V machine code for loading the OS
//! kernel from disk into RAM and transferring control to it.
//!
//! The bootloader executes in four phases:
//!   1. Validate boot protocol magic (0xB007CAFE)
//!   2. Read boot parameters (kernel location, size)
//!   3. Copy kernel from disk to RAM (word-by-word loop)
//!   4. Set stack pointer and jump to kernel entry

use riscv_simulator::encoding::*;

// Well-known addresses
pub const DEFAULT_ENTRY_ADDRESS: u32     = 0x00010000;
pub const DEFAULT_KERNEL_DISK_OFFSET: u32 = 0x00080000;
pub const DEFAULT_KERNEL_LOAD_ADDRESS: u32 = 0x00020000;
pub const DEFAULT_STACK_BASE: u32        = 0x0006FFF0;
pub const DISK_MEMORY_MAP_BASE: u32      = 0x10000000;
pub const BOOT_PROTOCOL_ADDRESS: u32     = 0x00001000;
pub const BOOT_PROTOCOL_MAGIC: u32       = 0xB007CAFE;

// Disk image constants
pub const DISK_KERNEL_OFFSET: u32    = 0x00080000;
pub const DISK_USER_PROGRAM_BASE: u32 = 0x00100000;
pub const DEFAULT_DISK_SIZE: usize   = 2 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct BootloaderConfig {
    pub entry_address: u32,
    pub kernel_disk_offset: u32,
    pub kernel_load_address: u32,
    pub kernel_size: u32,
    pub stack_base: u32,
}

impl Default for BootloaderConfig {
    fn default() -> Self {
        Self {
            entry_address: DEFAULT_ENTRY_ADDRESS,
            kernel_disk_offset: DEFAULT_KERNEL_DISK_OFFSET,
            kernel_load_address: DEFAULT_KERNEL_LOAD_ADDRESS,
            kernel_size: 0,
            stack_base: DEFAULT_STACK_BASE,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnnotatedInstruction {
    pub address: u32,
    pub machine_code: u32,
    pub assembly: String,
    pub comment: String,
}

pub struct Bootloader {
    pub config: BootloaderConfig,
}

impl Bootloader {
    pub fn new(config: BootloaderConfig) -> Self { Self { config } }

    pub fn generate(&self) -> Vec<u8> {
        let annotated = self.generate_with_comments();
        let words: Vec<u32> = annotated.iter().map(|a| a.machine_code).collect();
        assemble(&words)
    }

    pub fn generate_with_comments(&self) -> Vec<AnnotatedInstruction> {
        let mut instructions = Vec::new();
        let mut address = self.config.entry_address;

        macro_rules! emit {
            ($code:expr, $asm:expr, $comment:expr) => {{
                instructions.push(AnnotatedInstruction {
                    address, machine_code: $code,
                    assembly: $asm.to_string(), comment: $comment.to_string(),
                });
                address += 4;
            }};
        }

        // Phase 1: Validate Boot Protocol
        emit!(encode_lui(5, 1), "lui t0, 0x00001", "Phase 1: t0 = boot protocol addr");
        emit!(encode_lw(6, 5, 0), "lw t1, 0(t0)", "Phase 1: t1 = magic number");
        emit!(encode_lui(7, 0xB007D), "lui t2, 0xB007D", "Phase 1: t2 upper");
        let signed_afe = sign_extend_12(0xAFE);
        emit!(encode_addi(7, 7, signed_afe), &format!("addi t2, t2, {}", signed_afe), "Phase 1: t2 = 0xB007CAFE");

        let halt_branch_index = instructions.len();
        emit!(encode_bne(6, 7, 0), "bne t1, t2, halt", "Phase 1: halt if bad magic");

        // Phase 2: Read Boot Parameters
        let source = DISK_MEMORY_MAP_BASE.wrapping_add(self.config.kernel_disk_offset);
        address = emit_load_imm(&mut instructions, address, 5, source, "Phase 2: t0 = source");
        address = emit_load_imm(&mut instructions, address, 6, self.config.kernel_load_address, "Phase 2: t1 = dest");
        address = emit_load_imm(&mut instructions, address, 7, self.config.kernel_size, "Phase 2: t2 = size");

        // Phase 3: Copy kernel
        emit!(encode_beq(7, 0, 24), "beq t2, x0, +24", "Phase 3: skip if size 0");
        let copy_loop_addr = address;
        emit!(encode_lw(28, 5, 0), "lw t3, 0(t0)", "Phase 3: load word");
        emit!(encode_sw(28, 6, 0), "sw t3, 0(t1)", "Phase 3: store word");
        emit!(encode_addi(5, 5, 4), "addi t0, t0, 4", "Phase 3: src += 4");
        emit!(encode_addi(6, 6, 4), "addi t1, t1, 4", "Phase 3: dst += 4");
        emit!(encode_addi(7, 7, -4), "addi t2, t2, -4", "Phase 3: remaining -= 4");
        let loop_offset = copy_loop_addr as i32 - address as i32;
        emit!(encode_bne(7, 0, loop_offset), &format!("bne t2, x0, {}", loop_offset), "Phase 3: loop");

        // Phase 4: Set stack and jump
        address = emit_load_imm(&mut instructions, address, 2, self.config.stack_base, "Phase 4: sp = stack");
        address = emit_load_imm(&mut instructions, address, 5, self.config.kernel_load_address, "Phase 4: t0 = kernel");
        emit!(encode_jalr(0, 5, 0), "jalr x0, t0, 0", "Phase 4: jump to kernel");

        let halt_addr = address;
        emit!(encode_jal(0, 0), "jal x0, 0", "Halt: infinite loop");

        // Patch halt branch
        let branch_pc = instructions[halt_branch_index].address;
        let halt_offset = halt_addr as i32 - branch_pc as i32;
        instructions[halt_branch_index].machine_code = encode_bne(6, 7, halt_offset);
        instructions[halt_branch_index].assembly = format!("bne t1, t2, +{}", halt_offset);

        instructions
    }

    pub fn instruction_count(&self) -> usize { self.generate_with_comments().len() }

    pub fn estimate_cycles(&self) -> usize {
        (self.config.kernel_size as usize / 4) * 6 + 20
    }
}

fn sign_extend_12(val: i32) -> i32 {
    let v = val & 0xFFF;
    if v >= 0x800 { v - 0x1000 } else { v }
}

fn emit_load_imm(instructions: &mut Vec<AnnotatedInstruction>, mut address: u32, rd: u32, value: u32, comment: &str) -> u32 {
    let mut upper = (value >> 12) & 0xFFFFF;
    let lower = value & 0xFFF;
    if lower >= 0x800 { upper = (upper + 1) & 0xFFFFF; }
    let rn = match rd { 2 => "sp", 5 => "t0", 6 => "t1", 7 => "t2", _ => "x?" };

    if upper != 0 {
        instructions.push(AnnotatedInstruction { address, machine_code: encode_lui(rd, upper), assembly: format!("lui {}, 0x{:05X}", rn, upper), comment: comment.to_string() });
        address += 4;
        if lower != 0 {
            let sl = sign_extend_12(lower as i32);
            instructions.push(AnnotatedInstruction { address, machine_code: encode_addi(rd, rd, sl), assembly: format!("addi {}, {}, {}", rn, rn, sl), comment: comment.to_string() });
            address += 4;
        }
    } else if lower != 0 {
        let sl = sign_extend_12(lower as i32);
        instructions.push(AnnotatedInstruction { address, machine_code: encode_addi(rd, 0, sl), assembly: format!("addi {}, x0, {}", rn, sl), comment: comment.to_string() });
        address += 4;
    } else {
        instructions.push(AnnotatedInstruction { address, machine_code: encode_addi(rd, 0, 0), assembly: format!("addi {}, x0, 0", rn), comment: format!("{} (value = 0)", comment) });
        address += 4;
    }
    address
}

// =========================================================================
// DiskImage
// =========================================================================

pub struct DiskImage {
    data: Vec<u8>,
}

impl DiskImage {
    pub fn new(size_bytes: usize) -> Self { Self { data: vec![0u8; size_bytes] } }

    pub fn load_kernel(&mut self, binary: &[u8]) { self.load_at(DISK_KERNEL_OFFSET as usize, binary); }

    pub fn load_at(&mut self, offset: usize, data: &[u8]) {
        assert!(offset + data.len() <= self.data.len(), "DiskImage: data exceeds disk size");
        self.data[offset..offset + data.len()].copy_from_slice(data);
    }

    pub fn read_word(&self, offset: usize) -> u32 {
        if offset + 4 > self.data.len() { return 0; }
        u32::from_le_bytes([self.data[offset], self.data[offset+1], self.data[offset+2], self.data[offset+3]])
    }

    pub fn read_byte_at(&self, offset: usize) -> u8 {
        if offset >= self.data.len() { return 0; }
        self.data[offset]
    }

    pub fn data(&self) -> &[u8] { &self.data }
    pub fn size(&self) -> usize { self.data.len() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = BootloaderConfig::default();
        assert_eq!(config.entry_address, DEFAULT_ENTRY_ADDRESS);
        assert_eq!(config.kernel_size, 0);
    }

    #[test]
    fn test_generate() {
        let mut config = BootloaderConfig::default();
        config.kernel_size = 256;
        let bl = Bootloader::new(config);
        let binary = bl.generate();
        assert!(!binary.is_empty());
        assert_eq!(binary.len() % 4, 0);
    }

    #[test]
    fn test_generate_with_comments() {
        let mut config = BootloaderConfig::default();
        config.kernel_size = 256;
        let bl = Bootloader::new(config);
        let annotated = bl.generate_with_comments();
        assert!(!annotated.is_empty());
        for instr in &annotated {
            assert!(!instr.assembly.is_empty());
            assert!(!instr.comment.is_empty());
        }
    }

    #[test]
    fn test_instruction_count() {
        let mut config = BootloaderConfig::default();
        config.kernel_size = 256;
        let bl = Bootloader::new(config);
        assert!(bl.instruction_count() > 0);
    }

    #[test]
    fn test_estimate_cycles() {
        let mut config = BootloaderConfig::default();
        config.kernel_size = 4096;
        let bl = Bootloader::new(config);
        assert_eq!(bl.estimate_cycles(), 6164);
    }

    #[test]
    fn test_disk_image() {
        let mut disk = DiskImage::new(DEFAULT_DISK_SIZE);
        disk.load_kernel(&[0xDE, 0xAD, 0xBE, 0xEF]);
        assert_eq!(disk.read_byte_at(DISK_KERNEL_OFFSET as usize), 0xDE);
        assert_eq!(disk.read_word(DISK_KERNEL_OFFSET as usize), 0xEFBEADDE);
    }

    #[test]
    fn test_disk_image_size() {
        let disk = DiskImage::new(1024);
        assert_eq!(disk.size(), 1024);
    }

    #[test]
    fn test_phases_in_comments() {
        let mut config = BootloaderConfig::default();
        config.kernel_size = 256;
        let bl = Bootloader::new(config);
        let comments: String = bl.generate_with_comments().iter().map(|i| i.comment.clone()).collect::<Vec<_>>().join(" ");
        assert!(comments.contains("Phase 1"));
        assert!(comments.contains("Phase 2"));
        assert!(comments.contains("Phase 3"));
        assert!(comments.contains("Phase 4"));
    }

    #[test]
    fn test_magic_constant() {
        assert_eq!(BOOT_PROTOCOL_MAGIC, 0xB007CAFE);
    }
}
