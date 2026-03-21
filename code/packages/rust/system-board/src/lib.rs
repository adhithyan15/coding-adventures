//! S06 System Board -- the complete simulated computer.
//!
//! Composes ROM/BIOS, Bootloader, Interrupt Handler, OS Kernel, Display,
//! and a RISC-V CPU into a complete system: PowerOn -> BIOS -> Bootloader -> Kernel -> Hello World -> Idle

use riscv_simulator::{RiscVSimulator, CSRFile};
use riscv_simulator::encoding::assemble;
use display::{DisplayDriver, DisplayConfig, DisplaySnapshot, BYTES_PER_CELL};
use bootloader::{Bootloader, BootloaderConfig, DiskImage, BOOT_PROTOCOL_MAGIC, DEFAULT_DISK_SIZE};
use os_kernel::*;

// Address space constants
pub const BOOT_PROTOCOL_ADDR: u32 = 0x00001000;
pub const BOOTLOADER_BASE: u32    = 0x00010000;
pub const KERNEL_BASE: u32        = 0x00020000;
pub const IDLE_PROCESS_BASE: u32  = 0x00030000;
pub const USER_PROCESS_BASE: u32  = 0x00040000;
pub const DISK_MAPPED_BASE: u32   = 0x10000000;

// Boot phases
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootPhase { PowerOn, Bios, Bootloader, KernelInit, UserProgram, Idle }

impl std::fmt::Display for BootPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BootPhase::PowerOn => write!(f, "PowerOn"),
            BootPhase::Bios => write!(f, "BIOS"),
            BootPhase::Bootloader => write!(f, "Bootloader"),
            BootPhase::KernelInit => write!(f, "KernelInit"),
            BootPhase::UserProgram => write!(f, "UserProgram"),
            BootPhase::Idle => write!(f, "Idle"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BootEvent { pub phase: BootPhase, pub cycle: usize, pub description: String }

pub struct BootTrace { pub events: Vec<BootEvent> }

impl BootTrace {
    pub fn new() -> Self { Self { events: Vec::new() } }
    pub fn add_event(&mut self, phase: BootPhase, cycle: usize, desc: &str) {
        self.events.push(BootEvent { phase, cycle, description: desc.to_string() });
    }
    pub fn phases(&self) -> Vec<BootPhase> {
        let mut seen = Vec::new();
        for e in &self.events {
            if !seen.contains(&e.phase) { seen.push(e.phase); }
        }
        seen
    }
    pub fn total_cycles(&self) -> usize {
        self.events.last().map(|e| e.cycle).unwrap_or(0)
    }
    pub fn phase_start_cycle(&self, phase: BootPhase) -> i32 {
        self.events.iter().find(|e| e.phase == phase).map(|e| e.cycle as i32).unwrap_or(-1)
    }
    pub fn events_in_phase(&self, phase: BootPhase) -> Vec<&BootEvent> {
        self.events.iter().filter(|e| e.phase == phase).collect()
    }
}

pub struct SystemConfig {
    pub memory_size: usize,
    pub display_config: DisplayConfig,
    pub bootloader_config: BootloaderConfig,
    pub max_processes: usize,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            memory_size: 1024 * 1024,
            display_config: DisplayConfig::default(),
            bootloader_config: BootloaderConfig::default(),
            max_processes: 16,
        }
    }
}

pub struct SystemBoard {
    pub cpu: Option<RiscVSimulator>,
    pub display_memory: Vec<u8>,
    pub kernel: Option<Kernel>,
    pub disk_image: Option<DiskImage>,
    pub trace: BootTrace,
    pub powered: bool,
    pub cycle: usize,
    pub current_phase: BootPhase,
    kernel_booted: bool,
    config: SystemConfig,
}

impl SystemBoard {
    pub fn new(config: SystemConfig) -> Self {
        Self {
            cpu: None, display_memory: Vec::new(),
            kernel: None, disk_image: None,
            trace: BootTrace::new(), powered: false,
            cycle: 0, current_phase: BootPhase::PowerOn,
            kernel_booted: false, config,
        }
    }

    pub fn power_on(&mut self) {
        if self.powered { return; }

        let mem_size = 0x10200000;
        self.cpu = Some(RiscVSimulator::new(mem_size));
        let dc = self.config.display_config;
        self.display_memory = vec![0u8; dc.columns * dc.rows * BYTES_PER_CELL];

        self.kernel = Some(Kernel::new(self.config.max_processes));
        self.disk_image = Some(DiskImage::new(DEFAULT_DISK_SIZE));

        let user_program = generate_hello_world_program(USER_PROCESS_BASE);
        let idle_binary = generate_idle_program();
        let kernel_stub_size = 16usize;
        let mut total_size = kernel_stub_size + idle_binary.len() + user_program.len();
        if total_size % 4 != 0 { total_size += 4 - (total_size % 4); }

        let mut bl_config = self.config.bootloader_config.clone();
        bl_config.kernel_size = total_size as u32;
        let bl = Bootloader::new(bl_config.clone());
        let bootloader_code = bl.generate();

        let cpu = self.cpu.as_mut().unwrap();
        write_word(cpu, BOOT_PROTOCOL_ADDR, BOOT_PROTOCOL_MAGIC);
        write_word(cpu, BOOT_PROTOCOL_ADDR + 4, self.config.memory_size as u32);
        write_word(cpu, BOOT_PROTOCOL_ADDR + 8, bl_config.kernel_disk_offset);
        write_word(cpu, BOOT_PROTOCOL_ADDR + 12, bl_config.kernel_size);
        write_word(cpu, BOOT_PROTOCOL_ADDR + 16, bl_config.kernel_load_address);
        write_word(cpu, BOOT_PROTOCOL_ADDR + 20, bl_config.stack_base);

        for (i, &b) in bootloader_code.iter().enumerate() {
            cpu.mem.write_byte(BOOTLOADER_BASE as usize + i, b);
        }

        let mut kernel_disk_data = vec![0u8; total_size];
        kernel_disk_data[kernel_stub_size..kernel_stub_size + idle_binary.len()].copy_from_slice(&idle_binary);
        kernel_disk_data[kernel_stub_size + idle_binary.len()..kernel_stub_size + idle_binary.len() + user_program.len()].copy_from_slice(&user_program);
        self.disk_image.as_mut().unwrap().load_kernel(&kernel_disk_data);

        let disk_data: Vec<u8> = self.disk_image.as_ref().unwrap().data().to_vec();
        for (i, &b) in disk_data.iter().enumerate() {
            let addr = DISK_MAPPED_BASE as usize + i;
            if addr < mem_size { cpu.mem.write_byte(addr, b); }
        }

        for (i, &b) in idle_binary.iter().enumerate() {
            cpu.mem.write_byte(IDLE_PROCESS_BASE as usize + i, b);
        }
        for (i, &b) in user_program.iter().enumerate() {
            cpu.mem.write_byte(USER_PROCESS_BASE as usize + i, b);
        }

        cpu.pc = BOOTLOADER_BASE as i32;
        cpu.csr.write(riscv_simulator::csr::CSR_MTVEC, 0xDEAD0000);

        self.powered = true;
        self.current_phase = BootPhase::PowerOn;
        self.trace.add_event(BootPhase::PowerOn, 0, "System powered on");
        self.trace.add_event(BootPhase::Bios, 0, "BIOS phase simulated");
        self.current_phase = BootPhase::Bios;
    }

    pub fn step(&mut self) {
        if !self.powered { return; }
        self.cycle += 1;
        self.cpu.as_mut().unwrap().step();
        self.detect_phase_transition();
        self.handle_trap();
    }

    pub fn run(&mut self, max_cycles: usize) {
        if !self.powered { return; }
        for _ in 0..max_cycles {
            self.step();
            if self.kernel_booted {
                if let Some(k) = &self.kernel {
                    if k.is_idle() {
                        if self.current_phase != BootPhase::Idle {
                            self.current_phase = BootPhase::Idle;
                            self.trace.add_event(BootPhase::Idle, self.cycle, "System idle");
                        }
                        break;
                    }
                }
            }
            if self.cpu.as_ref().unwrap().halted { break; }
        }
    }

    pub fn display_snapshot(&self) -> Option<DisplaySnapshot> {
        let dc = self.config.display_config;
        // Safety: we need an immutable borrow to create the driver for snapshot
        let mem_slice = &self.display_memory;
        // Build snapshot directly from display_memory
        let mut lines = Vec::with_capacity(dc.rows);
        for row in 0..dc.rows {
            let mut line = String::with_capacity(dc.columns);
            for col in 0..dc.columns {
                let offset = (row * dc.columns + col) * BYTES_PER_CELL;
                if offset < mem_slice.len() {
                    line.push(mem_slice[offset] as char);
                }
            }
            lines.push(line.trim_end().to_string());
        }
        Some(DisplaySnapshot {
            lines,
            cursor: display::CursorPosition { row: 0, col: 0 },
            rows: dc.rows,
            columns: dc.columns,
        })
    }

    pub fn is_idle(&self) -> bool {
        self.kernel_booted && self.kernel.as_ref().map(|k| k.is_idle()).unwrap_or(false)
    }

    pub fn inject_keystroke(&mut self, ch: u8) {
        if let Some(k) = &mut self.kernel { k.add_keystroke(ch); }
    }

    fn detect_phase_transition(&mut self) {
        let pc = self.cpu.as_ref().unwrap().pc as u32;
        match self.current_phase {
            BootPhase::Bios => {
                if pc >= BOOTLOADER_BASE && pc < BOOTLOADER_BASE + 0x10000 {
                    self.current_phase = BootPhase::Bootloader;
                    self.trace.add_event(BootPhase::Bootloader, self.cycle, "Bootloader executing");
                }
            }
            BootPhase::Bootloader => {
                if pc >= KERNEL_BASE && pc < KERNEL_BASE + 0x10000 {
                    self.current_phase = BootPhase::KernelInit;
                    self.trace.add_event(BootPhase::KernelInit, self.cycle, "Kernel entry reached");
                    self.initialize_kernel();
                }
            }
            BootPhase::KernelInit => {
                if pc >= USER_PROCESS_BASE && pc < USER_PROCESS_BASE + 0x10000 {
                    self.current_phase = BootPhase::UserProgram;
                    self.trace.add_event(BootPhase::UserProgram, self.cycle, "User program executing");
                }
            }
            _ => {}
        }
    }

    fn initialize_kernel(&mut self) {
        if self.kernel_booted { return; }
        if let Some(k) = &mut self.kernel {
            k.boot();
            self.kernel_booted = true;
            let count = k.process_count();
            self.trace.add_event(BootPhase::KernelInit, self.cycle, &format!("Kernel booted: {} processes", count));

            if k.process_table.len() > 1 {
                let pcb = &k.process_table[1];
                let cpu = self.cpu.as_mut().unwrap();
                cpu.pc = pcb.saved_pc as i32;
                cpu.regs.write(REG_SP, pcb.stack_pointer);
            }
        }
    }

    fn handle_trap(&mut self) {
        let pc = self.cpu.as_ref().unwrap().pc as u32;
        if pc != 0xDEAD0000 { return; }

        if !self.kernel_booted {
            let cpu = self.cpu.as_mut().unwrap();
            let mepc = cpu.csr.read(riscv_simulator::csr::CSR_MEPC);
            cpu.pc = (mepc + 4) as i32;
            let mstatus = cpu.csr.read(riscv_simulator::csr::CSR_MSTATUS);
            cpu.csr.write(riscv_simulator::csr::CSR_MSTATUS, mstatus | riscv_simulator::csr::MIE);
            return;
        }

        let cpu = self.cpu.as_mut().unwrap();
        let syscall_num = cpu.regs.read(REG_A7);
        let mepc = cpu.csr.read(riscv_simulator::csr::CSR_MEPC);

        let mut reg_adapter = CpuRegAdapter { regs: &mut cpu.regs };
        let mem_adapter = CpuMemAdapter { mem: &cpu.mem };

        // Create a temporary display driver for syscall handling
        let dc = self.config.display_config;
        let mut display_driver = DisplayDriver::new(dc, &mut self.display_memory);

        let kernel = self.kernel.as_mut().unwrap();
        kernel.handle_syscall(syscall_num, &mut reg_adapter, &mem_adapter, Some(&mut display_driver));

        // Determine next PC
        let cpu = self.cpu.as_mut().unwrap();
        let kernel = self.kernel.as_ref().unwrap();
        if let Some(pcb) = kernel.get_current_pcb() {
            if pcb.state == ProcessState::Running {
                cpu.pc = (mepc + 4) as i32;
            } else if pcb.state == ProcessState::Ready || pcb.state == ProcessState::Terminated {
                if let Some(next_pcb) = kernel.get_current_pcb() {
                    if next_pcb.state == ProcessState::Running {
                        cpu.pc = next_pcb.saved_pc as i32;
                        cpu.regs.write(REG_SP, next_pcb.stack_pointer);
                    } else if !kernel.process_table.is_empty() {
                        cpu.pc = kernel.process_table[0].saved_pc as i32;
                    }
                }
            }
        } else {
            cpu.pc = (mepc + 4) as i32;
        }

        let mstatus = cpu.csr.read(riscv_simulator::csr::CSR_MSTATUS);
        cpu.csr.write(riscv_simulator::csr::CSR_MSTATUS, mstatus | riscv_simulator::csr::MIE);

        match syscall_num {
            SYS_WRITE => self.trace.add_event(self.current_phase, self.cycle, "sys_write: bytes written"),
            SYS_EXIT => self.trace.add_event(self.current_phase, self.cycle, "sys_exit: process terminated"),
            SYS_YIELD => self.trace.add_event(self.current_phase, self.cycle, "sys_yield: context switch"),
            _ => {}
        }
    }
}

// CPU adapters for kernel syscall interface
struct CpuRegAdapter<'a> {
    regs: &'a mut cpu_simulator::RegisterFile,
}

impl<'a> RegisterAccess for CpuRegAdapter<'a> {
    fn read_register(&self, index: usize) -> u32 { self.regs.read(index) }
    fn write_register(&mut self, index: usize, value: u32) { self.regs.write(index, value); }
}

struct CpuMemAdapter<'a> {
    mem: &'a cpu_simulator::Memory,
}

impl<'a> MemAccess for CpuMemAdapter<'a> {
    fn read_memory_byte(&self, address: u32) -> u8 { self.mem.read_byte(address as usize) }
}

fn write_word(cpu: &mut RiscVSimulator, address: u32, value: u32) {
    let addr = address as usize;
    cpu.mem.write_byte(addr, (value & 0xFF) as u8);
    cpu.mem.write_byte(addr + 1, ((value >> 8) & 0xFF) as u8);
    cpu.mem.write_byte(addr + 2, ((value >> 16) & 0xFF) as u8);
    cpu.mem.write_byte(addr + 3, ((value >> 24) & 0xFF) as u8);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boot_to_hello_world() {
        let mut board = SystemBoard::new(SystemConfig::default());
        board.power_on();
        board.run(100000);
        let snap = board.display_snapshot().unwrap();
        assert!(snap.contains("Hello World"), "Display should contain 'Hello World', got: {:?}", snap.lines[0]);
    }

    #[test]
    fn test_idle_after_boot() {
        let mut board = SystemBoard::new(SystemConfig::default());
        board.power_on();
        board.run(100000);
        assert!(board.is_idle());
    }

    #[test]
    fn test_new_board_not_powered() {
        let board = SystemBoard::new(SystemConfig::default());
        assert!(!board.powered);
    }

    #[test]
    fn test_power_on() {
        let mut board = SystemBoard::new(SystemConfig::default());
        board.power_on();
        assert!(board.powered);
        assert!(board.cpu.is_some());
        assert!(board.kernel.is_some());
    }

    #[test]
    fn test_double_power_on() {
        let mut board = SystemBoard::new(SystemConfig::default());
        board.power_on();
        board.power_on();
        assert!(board.powered);
    }

    #[test]
    fn test_boot_phases() {
        let mut board = SystemBoard::new(SystemConfig::default());
        board.power_on();
        board.run(100000);
        let phases = board.trace.phases();
        assert!(!phases.is_empty());
        assert!(phases.contains(&BootPhase::PowerOn));
        assert!(phases.contains(&BootPhase::Bios));
    }

    #[test]
    fn test_trace_has_events() {
        let mut board = SystemBoard::new(SystemConfig::default());
        board.power_on();
        board.run(100000);
        assert!(!board.trace.events.is_empty());
    }

    #[test]
    fn test_total_cycles() {
        let mut board = SystemBoard::new(SystemConfig::default());
        board.power_on();
        board.run(100000);
        assert!(board.trace.total_cycles() > 0);
    }

    #[test]
    fn test_phase_start_cycle() {
        let mut board = SystemBoard::new(SystemConfig::default());
        board.power_on();
        board.run(100000);
        assert_eq!(board.trace.phase_start_cycle(BootPhase::PowerOn), 0);
    }

    #[test]
    fn test_cycle_count_positive() {
        let mut board = SystemBoard::new(SystemConfig::default());
        board.power_on();
        board.run(100000);
        assert!(board.cycle > 0);
    }

    #[test]
    fn test_step_before_power_on() {
        let mut board = SystemBoard::new(SystemConfig::default());
        board.step();
        assert_eq!(board.cycle, 0);
    }

    #[test]
    fn test_inject_keystroke() {
        let mut board = SystemBoard::new(SystemConfig::default());
        board.power_on();
        board.run(100000);
        board.inject_keystroke(b'A');
        assert_eq!(board.kernel.as_ref().unwrap().keyboard_buffer.len(), 1);
    }

    #[test]
    fn test_boot_phase_display() {
        assert_eq!(format!("{}", BootPhase::PowerOn), "PowerOn");
        assert_eq!(format!("{}", BootPhase::Idle), "Idle");
    }

    #[test]
    fn test_default_config() {
        let config = SystemConfig::default();
        assert_eq!(config.memory_size, 1024 * 1024);
        assert_eq!(config.display_config.columns, 80);
        assert_eq!(config.display_config.rows, 25);
    }
}
