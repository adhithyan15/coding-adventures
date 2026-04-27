//! S04 OS Kernel -- minimal monolithic kernel with process management,
//! round-robin scheduler, system calls, and memory management.
//!
//! The kernel operates at the Rust level -- syscall handlers, the scheduler,
//! and memory management are Rust functions. The hello-world and idle
//! programs are real RISC-V machine code.

use riscv_simulator::encoding::*;

// Well-known addresses
pub const DEFAULT_KERNEL_BASE: u32       = 0x00020000;
pub const DEFAULT_IDLE_PROCESS_BASE: u32 = 0x00030000;
pub const DEFAULT_IDLE_PROCESS_SIZE: u32 = 0x00010000;
pub const DEFAULT_USER_PROCESS_BASE: u32 = 0x00040000;
pub const DEFAULT_USER_PROCESS_SIZE: u32 = 0x00010000;
pub const DEFAULT_KERNEL_STACK_TOP: u32  = 0x0006FFF0;

// Syscall numbers
pub const SYS_EXIT: u32  = 0;
pub const SYS_WRITE: u32 = 1;
pub const SYS_READ: u32  = 2;
pub const SYS_YIELD: u32 = 3;

// Register numbers
pub const REG_A0: usize = 10;
pub const REG_A1: usize = 11;
pub const REG_A2: usize = 12;
pub const REG_A7: usize = 17;
pub const REG_SP: usize = 2;

// Process states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState { Ready, Running, Blocked, Terminated }

impl std::fmt::Display for ProcessState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessState::Ready => write!(f, "Ready"),
            ProcessState::Running => write!(f, "Running"),
            ProcessState::Blocked => write!(f, "Blocked"),
            ProcessState::Terminated => write!(f, "Terminated"),
        }
    }
}

// Memory permissions
pub const PERM_READ: u8    = 0x01;
pub const PERM_WRITE: u8   = 0x02;
pub const PERM_EXECUTE: u8 = 0x04;

#[derive(Debug, Clone)]
pub struct MemoryRegion {
    pub base: u32, pub size: u32, pub permissions: u8, pub owner: i32, pub name: String,
}

pub struct MemoryManager { pub regions: Vec<MemoryRegion> }

impl MemoryManager {
    pub fn new(regions: Vec<MemoryRegion>) -> Self { Self { regions } }

    pub fn find_region(&self, address: u32) -> Option<&MemoryRegion> {
        self.regions.iter().find(|r| address >= r.base && address < r.base + r.size)
    }

    pub fn check_access(&self, pid: i32, address: u32, perm: u8) -> bool {
        match self.find_region(address) {
            None => false,
            Some(r) => {
                if r.owner != -1 && r.owner != pid { return false; }
                (r.permissions & perm) == perm
            }
        }
    }

    pub fn region_count(&self) -> usize { self.regions.len() }
}

#[derive(Debug, Clone)]
pub struct ProcessControlBlock {
    pub pid: i32, pub state: ProcessState, pub saved_registers: [u32; 32],
    pub saved_pc: u32, pub stack_pointer: u32, pub memory_base: u32,
    pub memory_size: u32, pub name: String, pub exit_code: i32,
}

pub struct Scheduler {
    pub process_table: Vec<ProcessControlBlock>,
    pub current: usize,
}

impl Scheduler {
    pub fn new(table: Vec<ProcessControlBlock>) -> Self { Self { process_table: table, current: 0 } }

    pub fn schedule(&self) -> usize {
        let n = self.process_table.len();
        if n == 0 { return 0; }
        for i in 1..=n {
            let idx = (self.current + i) % n;
            if self.process_table[idx].state == ProcessState::Ready { return idx; }
        }
        if self.current < n && self.process_table[self.current].state == ProcessState::Ready {
            return self.current;
        }
        0
    }

    pub fn context_switch(&mut self, from: usize, to: usize) {
        if from < self.process_table.len() && self.process_table[from].state == ProcessState::Running {
            self.process_table[from].state = ProcessState::Ready;
        }
        if to < self.process_table.len() {
            self.process_table[to].state = ProcessState::Running;
        }
        self.current = to;
    }
}

/// Trait for accessing CPU registers from syscall handlers.
pub trait RegisterAccess {
    fn read_register(&self, index: usize) -> u32;
    fn write_register(&mut self, index: usize, value: u32);
}

/// Trait for accessing CPU memory from syscall handlers.
pub trait MemAccess {
    fn read_memory_byte(&self, address: u32) -> u8;
}

/// Generate the idle process binary (infinite yield loop).
pub fn generate_idle_program() -> Vec<u8> {
    let instructions = vec![
        encode_addi(REG_A7 as u32, 0, SYS_YIELD as i32),
        encode_ecall(),
        encode_jal(0, -8),
    ];
    assemble(&instructions)
}

/// Generate the hello-world process binary.
pub fn generate_hello_world_program(mem_base: u32) -> Vec<u8> {
    let data_offset: u32 = 0x100;
    let data_addr = mem_base + data_offset;
    let message = b"Hello World\n";

    let mut instructions = Vec::new();
    let mut upper = (data_addr >> 12) & 0xFFFFF;
    let lower = data_addr & 0xFFF;
    if lower >= 0x800 { upper = (upper + 1) & 0xFFFFF; }

    instructions.push(encode_lui(REG_A1 as u32, upper));
    if lower != 0 {
        let sl = if lower >= 0x800 { lower as i32 - 0x1000 } else { lower as i32 };
        instructions.push(encode_addi(REG_A1 as u32, REG_A1 as u32, sl));
    }
    instructions.push(encode_addi(REG_A0 as u32, 0, 1));
    instructions.push(encode_addi(REG_A2 as u32, 0, message.len() as i32));
    instructions.push(encode_addi(REG_A7 as u32, 0, SYS_WRITE as i32));
    instructions.push(encode_ecall());
    instructions.push(encode_addi(REG_A0 as u32, 0, 0));
    instructions.push(encode_addi(REG_A7 as u32, 0, SYS_EXIT as i32));
    instructions.push(encode_ecall());

    let code = assemble(&instructions);
    let mut binary = vec![0u8; data_offset as usize + message.len()];
    binary[..code.len()].copy_from_slice(&code);
    binary[data_offset as usize..data_offset as usize + message.len()].copy_from_slice(message);
    binary
}

pub fn generate_hello_world_binary() -> Vec<u8> {
    generate_hello_world_program(DEFAULT_USER_PROCESS_BASE)
}

/// Simplified Kernel that works with a display driver for syscall handling.
pub struct Kernel {
    pub process_table: Vec<ProcessControlBlock>,
    pub current_process: i32,
    pub scheduler: Option<Scheduler>,
    pub memory_manager: Option<MemoryManager>,
    pub keyboard_buffer: Vec<u8>,
    pub booted: bool,
    next_pid: i32,
    pub max_processes: usize,
}

impl Kernel {
    pub fn new(max_processes: usize) -> Self {
        Self {
            process_table: Vec::new(),
            current_process: 0,
            scheduler: None,
            memory_manager: None,
            keyboard_buffer: Vec::new(),
            booted: false,
            next_pid: 0,
            max_processes,
        }
    }

    pub fn boot(&mut self) {
        let idle_binary = generate_idle_program();
        self.create_process("idle", &idle_binary, DEFAULT_IDLE_PROCESS_BASE, DEFAULT_IDLE_PROCESS_SIZE);
        let hw_binary = generate_hello_world_program(DEFAULT_USER_PROCESS_BASE);
        self.create_process("hello-world", &hw_binary, DEFAULT_USER_PROCESS_BASE, DEFAULT_USER_PROCESS_SIZE);

        if self.process_table.len() > 1 {
            self.process_table[1].state = ProcessState::Running;
            self.current_process = 1;
        }

        let mut sched = Scheduler::new(self.process_table.clone());
        sched.current = 1;
        self.scheduler = Some(sched);
        self.booted = true;
    }

    pub fn create_process(&mut self, name: &str, _binary: &[u8], mem_base: u32, mem_size: u32) -> i32 {
        if self.process_table.len() >= self.max_processes { return -1; }
        let pid = self.next_pid;
        self.next_pid += 1;
        let mut pcb = ProcessControlBlock {
            pid, state: ProcessState::Ready,
            saved_registers: [0u32; 32],
            saved_pc: mem_base,
            stack_pointer: mem_base + mem_size - 16,
            memory_base: mem_base, memory_size: mem_size,
            name: name.to_string(), exit_code: 0,
        };
        pcb.saved_registers[REG_SP] = pcb.stack_pointer;
        self.process_table.push(pcb);
        pid
    }

    pub fn handle_syscall(&mut self, syscall_num: u32, regs: &mut dyn RegisterAccess, mem: &dyn MemAccess, display: Option<&mut display::DisplayDriver>) -> bool {
        match syscall_num {
            SYS_EXIT => {
                let exit_code = regs.read_register(REG_A0) as i32;
                let pid = self.current_process as usize;
                if pid < self.process_table.len() {
                    self.process_table[pid].state = ProcessState::Terminated;
                    self.process_table[pid].exit_code = exit_code;
                }
                self.sync_scheduler();
                let next = self.scheduler.as_ref().map(|s| s.schedule()).unwrap_or(0);
                if let Some(s) = &mut self.scheduler { s.context_switch(pid, next); }
                self.current_process = next as i32;
                true
            }
            SYS_WRITE => {
                let fd = regs.read_register(REG_A0);
                let buf_addr = regs.read_register(REG_A1);
                let length = regs.read_register(REG_A2);
                if fd != 1 { regs.write_register(REG_A0, 0); return true; }
                if let Some(disp) = display {
                    let mut written = 0u32;
                    for i in 0..length {
                        let ch = mem.read_memory_byte(buf_addr + i);
                        disp.put_char(ch);
                        written += 1;
                    }
                    regs.write_register(REG_A0, written);
                } else {
                    regs.write_register(REG_A0, 0);
                }
                true
            }
            SYS_READ => {
                let fd = regs.read_register(REG_A0);
                let length = regs.read_register(REG_A2);
                if fd != 0 { regs.write_register(REG_A0, 0); return true; }
                let available = self.keyboard_buffer.len() as u32;
                let to_read = length.min(available);
                regs.write_register(REG_A0, to_read);
                if to_read > 0 { self.keyboard_buffer.drain(..to_read as usize); }
                true
            }
            SYS_YIELD => {
                let pid = self.current_process as usize;
                if pid < self.process_table.len() && self.process_table[pid].state == ProcessState::Running {
                    self.process_table[pid].state = ProcessState::Ready;
                }
                self.sync_scheduler();
                let next = self.scheduler.as_ref().map(|s| s.schedule()).unwrap_or(0);
                if let Some(s) = &mut self.scheduler { s.context_switch(pid, next); }
                self.current_process = next as i32;
                true
            }
            _ => {
                let pid = self.current_process as usize;
                if pid < self.process_table.len() {
                    self.process_table[pid].state = ProcessState::Terminated;
                    self.process_table[pid].exit_code = -1;
                }
                false
            }
        }
    }

    fn sync_scheduler(&mut self) {
        if let Some(s) = &mut self.scheduler {
            s.process_table = self.process_table.clone();
        }
    }

    pub fn is_idle(&self) -> bool {
        self.process_table.iter().all(|pcb| pcb.pid == 0 || pcb.state == ProcessState::Terminated)
    }

    pub fn process_count(&self) -> usize { self.process_table.len() }

    pub fn get_current_pcb(&self) -> Option<&ProcessControlBlock> {
        let idx = self.current_process as usize;
        if idx < self.process_table.len() { Some(&self.process_table[idx]) } else { None }
    }

    pub fn add_keystroke(&mut self, ch: u8) { self.keyboard_buffer.push(ch); }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_process() {
        let mut k = Kernel::new(16);
        let pid = k.create_process("test", &[1,2,3,4], 0x40000, 0x10000);
        assert_eq!(pid, 0);
        assert_eq!(k.process_table.len(), 1);
        assert_eq!(k.process_table[0].name, "test");
        assert_eq!(k.process_table[0].state, ProcessState::Ready);
    }

    #[test]
    fn test_max_processes() {
        let mut k = Kernel::new(2);
        k.create_process("p0", &[], 0x30000, 0x10000);
        k.create_process("p1", &[], 0x40000, 0x10000);
        assert_eq!(k.create_process("p2", &[], 0x50000, 0x10000), -1);
    }

    #[test]
    fn test_scheduler_round_robin() {
        let procs = vec![
            ProcessControlBlock { pid: 0, state: ProcessState::Ready, saved_registers: [0;32], saved_pc: 0, stack_pointer: 0, memory_base: 0, memory_size: 0, name: "idle".to_string(), exit_code: 0 },
            ProcessControlBlock { pid: 1, state: ProcessState::Ready, saved_registers: [0;32], saved_pc: 0, stack_pointer: 0, memory_base: 0, memory_size: 0, name: "hello".to_string(), exit_code: 0 },
        ];
        let sched = Scheduler::new(procs);
        assert_eq!(sched.schedule(), 1);
    }

    #[test]
    fn test_scheduler_skip_terminated() {
        let procs = vec![
            ProcessControlBlock { pid: 0, state: ProcessState::Ready, saved_registers: [0;32], saved_pc: 0, stack_pointer: 0, memory_base: 0, memory_size: 0, name: "idle".to_string(), exit_code: 0 },
            ProcessControlBlock { pid: 1, state: ProcessState::Terminated, saved_registers: [0;32], saved_pc: 0, stack_pointer: 0, memory_base: 0, memory_size: 0, name: "hello".to_string(), exit_code: 0 },
        ];
        let sched = Scheduler::new(procs);
        assert_eq!(sched.schedule(), 0);
    }

    #[test]
    fn test_kernel_boot() {
        let mut k = Kernel::new(16);
        k.boot();
        assert!(k.booted);
        assert_eq!(k.process_count(), 2);
        assert_eq!(k.process_table[0].name, "idle");
        assert_eq!(k.process_table[1].name, "hello-world");
        assert_eq!(k.current_process, 1);
    }

    #[test]
    fn test_is_idle() {
        let mut k = Kernel::new(16);
        k.boot();
        assert!(!k.is_idle());
        k.process_table[1].state = ProcessState::Terminated;
        assert!(k.is_idle());
    }

    #[test]
    fn test_idle_program() {
        let binary = generate_idle_program();
        assert!(!binary.is_empty());
        assert_eq!(binary.len() % 4, 0);
    }

    #[test]
    fn test_hello_world_program() {
        let binary = generate_hello_world_program(0x00040000);
        assert!(!binary.is_empty());
        let message = b"Hello World\n";
        assert_eq!(&binary[0x100..0x100+message.len()], message);
    }

    #[test]
    fn test_process_state_display() {
        assert_eq!(format!("{}", ProcessState::Ready), "Ready");
        assert_eq!(format!("{}", ProcessState::Running), "Running");
        assert_eq!(format!("{}", ProcessState::Terminated), "Terminated");
    }

    #[test]
    fn test_memory_manager() {
        let mm = MemoryManager::new(vec![
            MemoryRegion { base: 0x1000, size: 0x1000, permissions: PERM_READ | PERM_WRITE, owner: -1, name: "K".to_string() },
            MemoryRegion { base: 0x3000, size: 0x1000, permissions: PERM_READ, owner: 1, name: "P1".to_string() },
        ]);
        assert!(mm.check_access(0, 0x1000, PERM_READ));
        assert!(!mm.check_access(0, 0x3000, PERM_READ));
        assert!(mm.check_access(1, 0x3000, PERM_READ));
        assert!(!mm.check_access(0, 0x9000, PERM_READ));
    }

    #[test]
    fn test_add_keystroke() {
        let mut k = Kernel::new(16);
        k.boot();
        k.add_keystroke(b'H');
        k.add_keystroke(b'i');
        assert_eq!(k.keyboard_buffer, vec![b'H', b'i']);
    }
}
