//! # Process Manager (D14)
//!
//! The Process Manager implements Unix-style process management: the fundamental
//! operations that let an operating system create, run, and manage multiple
//! programs simultaneously.
//!
//! ## Analogy
//!
//! Think of a restaurant kitchen. The head chef (parent process) can:
//! - **fork():** Clone themselves — now there are two identical chefs with the
//!   same knowledge and state. The clone (child) can do different work.
//! - **exec():** The clone throws away their current recipe book and picks up a
//!   completely different one. Same person (same PID), different job.
//! - **wait():** The head chef pauses and watches the clone work. When the clone
//!   finishes, the head chef resumes.
//!
//! This is exactly how your shell works when you type `ls`:
//!
//! ```text
//! Shell (PID 100)
//! │
//! ├── fork() → child (PID 101), an exact copy of the shell
//! │   ├── [Child]: exec("ls") → replaces shell code with ls code
//! │   └── [Parent]: wait(101) → pauses until child exits
//! │
//! └── Shell prompt appears again
//! ```
//!
//! ## Components
//!
//! - [`ProcessState`]: Enum of process lifecycle states
//! - [`Signal`]: Enum of POSIX signal types
//! - [`ProcessControlBlock`]: Per-process kernel data structure
//! - [`SignalManager`]: Signal delivery, masking, and handler registration
//! - [`ProcessManager`]: fork/exec/wait/kill/exit operations
//! - [`PriorityScheduler`]: Priority-based scheduling with round-robin

use std::collections::{HashMap, HashSet, VecDeque};

// ============================================================================
// ProcessState
// ============================================================================

/// The possible states a process can be in during its lifecycle.
///
/// ```text
///                    fork()
///                      │
///                      ▼
///               ┌──────────┐
///      ┌───────►│  READY   │◄──────────────┐
///      │        └────┬─────┘               │
///      │             │ schedule()           │
///      │             ▼                     │
///      │        ┌──────────┐    I/O       │
///      │        │ RUNNING  │─────────►┌───┴────┐
///      │        └────┬─────┘          │BLOCKED │
///      │             │                └────────┘
/// SIGCONT            │ exit()
///      │             ▼
/// ┌────┴───┐   ┌──────────┐  wait()   ┌────────────┐
/// │STOPPED │   │  ZOMBIE  │──────────►│TERMINATED  │
/// └────────┘   └──────────┘           └────────────┘
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcessState {
    /// Process is loaded and waiting for CPU time. The scheduler can pick it.
    Ready = 0,
    /// Process is currently executing on the CPU.
    Running = 1,
    /// Process is waiting for an external event (I/O, signal, child exit).
    Blocked = 2,
    /// Process has been fully cleaned up and removed.
    Terminated = 3,
    /// Process has exited but parent hasn't called wait() yet.
    /// The kernel keeps the PCB so the parent can retrieve the exit status.
    Zombie = 4,
}

// ============================================================================
// Signal
// ============================================================================

/// POSIX signal types for inter-process communication.
///
/// Signals are "software interrupts" — a way to notify a process that
/// something happened. They are the Unix mechanism for:
/// - User interrupts (Ctrl+C sends SIGINT)
/// - Graceful shutdown requests (SIGTERM)
/// - Forced termination (SIGKILL)
/// - Child status notification (SIGCHLD)
/// - Process control (SIGSTOP/SIGCONT)
///
/// ## Why These Specific Numbers?
///
/// The numbers come from POSIX, the standard that defines Unix behavior.
/// Every Unix system uses the same numbering. We implement only the 6 most
/// essential signals; real systems define about 31.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Signal {
    /// Interrupt (2). Sent by Ctrl+C. Can be caught.
    SigInt = 2,
    /// Kill (9). Unconditionally terminates. CANNOT be caught or blocked.
    SigKill = 9,
    /// Terminate (15). Polite exit request. Can be caught for graceful shutdown.
    SigTerm = 15,
    /// Child status changed (17). Sent to parent when child exits/stops.
    SigChld = 17,
    /// Continue (18). Resumes a stopped process.
    SigCont = 18,
    /// Stop (19). Suspends the process. CANNOT be caught or blocked.
    SigStop = 19,
}

impl Signal {
    /// Returns the integer value of this signal (matches POSIX numbering).
    pub fn value(self) -> i32 {
        self as i32
    }

    /// Returns true if this signal cannot be caught, blocked, or ignored.
    ///
    /// SIGKILL and SIGSTOP are the two uncatchable signals. This is a
    /// security feature: the kernel must always be able to kill or stop
    /// any process, no matter what the process does.
    pub fn is_uncatchable(self) -> bool {
        matches!(self, Signal::SigKill | Signal::SigStop)
    }

    /// Returns true if this signal's default action is to terminate the process.
    pub fn is_fatal_by_default(self) -> bool {
        matches!(self, Signal::SigInt | Signal::SigKill | Signal::SigTerm)
    }

    /// Returns the human-readable name of this signal.
    pub fn name(self) -> &'static str {
        match self {
            Signal::SigInt => "SIGINT",
            Signal::SigKill => "SIGKILL",
            Signal::SigTerm => "SIGTERM",
            Signal::SigChld => "SIGCHLD",
            Signal::SigCont => "SIGCONT",
            Signal::SigStop => "SIGSTOP",
        }
    }
}

// ============================================================================
// SignalAction
// ============================================================================

/// The action to take when delivering a signal to a process.
///
/// After checking masks and handlers, the signal manager determines one
/// of these actions for each pending signal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignalAction {
    /// Terminate the process (SIGKILL, or default for SIGTERM/SIGINT).
    Kill,
    /// Stop (suspend) the process (SIGSTOP).
    Stop,
    /// Continue (resume) a stopped process (SIGCONT).
    Continue,
    /// Run the custom handler at the given address.
    Handler(u32),
    /// Ignore the signal (default for SIGCHLD).
    Ignore,
}

// ============================================================================
// ProcessControlBlock
// ============================================================================

/// The kernel's per-process data structure.
///
/// Every running process has a PCB that stores everything the kernel needs
/// to manage it: identity (PID, name), CPU state (registers, PC, SP),
/// memory bounds, process relationships (parent, children), signal state,
/// and scheduling priority.
///
/// Think of it like a patient's medical chart in a hospital. Every time
/// a doctor (the CPU) switches to a new patient (process), they consult
/// the chart to know where they left off.
///
/// ## Fields
///
/// | Category    | Field            | Description                              |
/// |-------------|------------------|------------------------------------------|
/// | Identity    | pid              | Unique process identifier                |
/// | Identity    | name             | Human-readable name                      |
/// | CPU State   | registers        | 32 RISC-V general-purpose registers      |
/// | CPU State   | pc               | Program counter (next instruction)       |
/// | CPU State   | sp               | Stack pointer                            |
/// | Memory      | memory_base      | Start of process's memory region         |
/// | Memory      | memory_size      | Size of memory region in bytes           |
/// | Lifecycle   | state            | Current ProcessState                     |
/// | Lifecycle   | exit_code        | Exit status (meaningful in Zombie state)  |
/// | Relations   | parent_pid       | PID of the parent process                |
/// | Relations   | children         | PIDs of all child processes               |
/// | Signals     | pending_signals  | Signals waiting to be delivered           |
/// | Signals     | signal_handlers  | Custom handler addresses per signal       |
/// | Signals     | signal_mask      | Set of currently blocked signals          |
/// | Scheduling  | priority         | 0-39 (lower = higher priority)           |
/// | Scheduling  | cpu_time         | Total CPU cycles consumed                |
#[derive(Debug, Clone)]
pub struct ProcessControlBlock {
    pub pid: u32,
    pub name: String,
    pub state: ProcessState,
    /// 32 general-purpose registers (RISC-V x0–x31).
    pub registers: [i32; 32],
    /// Program counter: address of the next instruction to execute.
    pub pc: u32,
    /// Stack pointer: top of the process's stack.
    pub sp: u32,
    /// Base address of the process's memory region.
    pub memory_base: u32,
    /// Size of the process's memory region in bytes.
    pub memory_size: u32,
    /// PID of the parent process. PID 0 and PID 1 have parent_pid = 0.
    pub parent_pid: u32,
    /// PIDs of all child processes.
    pub children: Vec<u32>,
    /// Signals queued for delivery on next scheduling.
    pub pending_signals: Vec<Signal>,
    /// Custom signal handlers: signal → handler address.
    pub signal_handlers: HashMap<Signal, u32>,
    /// Blocked signals that accumulate but are not delivered.
    pub signal_mask: HashSet<Signal>,
    /// Scheduling priority: 0 = highest, 39 = lowest, 20 = default.
    pub priority: u8,
    /// Total CPU cycles consumed by this process.
    pub cpu_time: u64,
    /// Exit status code. Only meaningful when state is Zombie.
    pub exit_code: i32,
}

/// Number of general-purpose registers in RISC-V.
pub const NUM_REGISTERS: usize = 32;

/// Default scheduling priority for user processes (Unix "nice" convention).
pub const DEFAULT_PRIORITY: u8 = 20;

/// Maximum valid priority (lowest scheduling priority).
pub const MAX_PRIORITY: u8 = 39;

impl ProcessControlBlock {
    /// Creates a new PCB with sensible defaults.
    ///
    /// All registers start at zero (like a freshly powered CPU),
    /// the process starts in Ready state, and it has no parent,
    /// children, or pending signals.
    pub fn new(pid: u32, name: &str, priority: Option<u8>) -> Self {
        let prio = priority.unwrap_or(DEFAULT_PRIORITY).min(MAX_PRIORITY);
        ProcessControlBlock {
            pid,
            name: name.to_string(),
            state: ProcessState::Ready,
            registers: [0; NUM_REGISTERS],
            pc: 0,
            sp: 0,
            memory_base: 0,
            memory_size: 0,
            parent_pid: 0,
            children: Vec::new(),
            pending_signals: Vec::new(),
            signal_handlers: HashMap::new(),
            signal_mask: HashSet::new(),
            priority: prio,
            cpu_time: 0,
            exit_code: 0,
        }
    }

    /// Creates a fork copy of this PCB for a child process.
    ///
    /// The child gets:
    /// - A new PID (provided by caller)
    /// - parent_pid set to this process's PID
    /// - Same registers, PC, SP, memory bounds, priority
    /// - Same signal handlers (inherited)
    /// - Empty children list, pending signals, and cpu_time = 0
    pub fn fork_copy(&self, new_pid: u32) -> Self {
        ProcessControlBlock {
            pid: new_pid,
            name: self.name.clone(),
            state: ProcessState::Ready,
            registers: self.registers,
            pc: self.pc,
            sp: self.sp,
            memory_base: self.memory_base,
            memory_size: self.memory_size,
            parent_pid: self.pid,
            children: Vec::new(),
            pending_signals: Vec::new(),
            signal_handlers: self.signal_handlers.clone(),
            signal_mask: self.signal_mask.clone(),
            priority: self.priority,
            cpu_time: 0,
            exit_code: 0,
        }
    }

    /// Returns true if the process is in Ready state.
    pub fn is_ready(&self) -> bool {
        self.state == ProcessState::Ready
    }

    /// Returns true if the process is currently Running.
    pub fn is_running(&self) -> bool {
        self.state == ProcessState::Running
    }

    /// Returns true if the process is Blocked.
    pub fn is_blocked(&self) -> bool {
        self.state == ProcessState::Blocked
    }

    /// Returns true if the process is a Zombie (exited, not reaped).
    pub fn is_zombie(&self) -> bool {
        self.state == ProcessState::Zombie
    }

    /// Returns true if the process has been Terminated (fully cleaned up).
    pub fn is_terminated(&self) -> bool {
        self.state == ProcessState::Terminated
    }
}

// ============================================================================
// SignalManager
// ============================================================================

/// Handles signal delivery, masking, and handler registration.
///
/// The SignalManager operates on ProcessControlBlock instances directly,
/// modifying their signal state. It implements the Unix signal delivery
/// rules:
///
/// 1. SIGKILL always kills — cannot be caught, blocked, or ignored.
/// 2. SIGSTOP always stops — cannot be caught, blocked, or ignored.
/// 3. SIGCONT always resumes a stopped process.
/// 4. If a custom handler is registered, redirect execution to it.
/// 5. Otherwise, apply the default action (terminate for SIGINT/SIGTERM,
///    ignore for SIGCHLD).
pub struct SignalManager;

impl SignalManager {
    /// Creates a new SignalManager.
    pub fn new() -> Self {
        SignalManager
    }

    /// Sends a signal to a process by adding it to the pending list.
    ///
    /// This does NOT immediately deliver the signal. Signals are delivered
    /// when the process is next scheduled (see `deliver_pending`).
    pub fn send_signal(&self, pcb: &mut ProcessControlBlock, signal: Signal) {
        pcb.pending_signals.push(signal);
    }

    /// Delivers all pending signals and returns the actions to take.
    ///
    /// Masked signals (except SIGKILL/SIGSTOP) remain pending.
    /// Each delivered signal produces a `SignalAction` telling the kernel
    /// what to do.
    pub fn deliver_pending(&self, pcb: &mut ProcessControlBlock) -> Vec<(Signal, SignalAction)> {
        let mut actions = Vec::new();
        let mut remaining = Vec::new();

        // Take pending signals out to avoid borrow conflicts.
        let pending = std::mem::take(&mut pcb.pending_signals);

        for signal in pending {
            // Masked signals stay pending, but SIGKILL/SIGSTOP bypass the mask.
            if pcb.signal_mask.contains(&signal) && !signal.is_uncatchable() {
                remaining.push(signal);
                continue;
            }

            let action = self.determine_action(pcb, signal);
            actions.push((signal, action));
        }

        pcb.pending_signals = remaining;
        actions
    }

    /// Registers a custom signal handler.
    ///
    /// SIGKILL and SIGSTOP cannot have custom handlers (returns false).
    pub fn register_handler(
        &self,
        pcb: &mut ProcessControlBlock,
        signal: Signal,
        handler_address: u32,
    ) -> bool {
        if signal.is_uncatchable() {
            return false;
        }
        pcb.signal_handlers.insert(signal, handler_address);
        true
    }

    /// Adds a signal to the process's mask (blocks it).
    ///
    /// SIGKILL and SIGSTOP cannot be masked (returns false).
    pub fn mask(&self, pcb: &mut ProcessControlBlock, signal: Signal) -> bool {
        if signal.is_uncatchable() {
            return false;
        }
        pcb.signal_mask.insert(signal);
        true
    }

    /// Removes a signal from the process's mask (unblocks it).
    pub fn unmask(&self, pcb: &mut ProcessControlBlock, signal: Signal) -> bool {
        pcb.signal_mask.remove(&signal);
        true
    }

    /// Returns true if the signal would be fatal for this process.
    ///
    /// A signal is fatal if it's SIGKILL (always) or if it's fatal by
    /// default and the process has no custom handler for it.
    pub fn is_fatal(&self, pcb: &ProcessControlBlock, signal: Signal) -> bool {
        if signal == Signal::SigKill {
            return true;
        }
        signal.is_fatal_by_default() && !pcb.signal_handlers.contains_key(&signal)
    }

    /// Determines the action for a specific signal.
    fn determine_action(&self, pcb: &ProcessControlBlock, signal: Signal) -> SignalAction {
        match signal {
            Signal::SigKill => SignalAction::Kill,
            Signal::SigStop => SignalAction::Stop,
            Signal::SigCont => SignalAction::Continue,
            _ => {
                if let Some(&addr) = pcb.signal_handlers.get(&signal) {
                    SignalAction::Handler(addr)
                } else if signal.is_fatal_by_default() {
                    SignalAction::Kill
                } else {
                    // Non-fatal signals with no handler are ignored (e.g., SIGCHLD).
                    SignalAction::Ignore
                }
            }
        }
    }
}

impl Default for SignalManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// ProcessManager
// ============================================================================

/// Manages the process table and implements fork/exec/wait/kill/exit.
///
/// The ProcessManager is the heart of Unix process management. It maintains
/// a hash table mapping PIDs to ProcessControlBlocks and provides the five
/// fundamental operations:
///
/// - **create_process**: Bootstrap a new process (for init, idle)
/// - **fork**: Clone a process (parent gets child PID, child gets 0)
/// - **exec**: Replace a process's program (zero registers, set PC/SP)
/// - **wait**: Reap a zombie child (return exit code, free PCB)
/// - **kill**: Send a signal to a process
/// - **exit_process**: Terminate voluntarily (become zombie, reparent children)
pub struct ProcessManager {
    /// Maps PID → ProcessControlBlock.
    pub process_table: HashMap<u32, ProcessControlBlock>,
    /// Next PID to assign (monotonically increasing).
    next_pid: u32,
    /// Signal manager for all signal operations.
    pub signal_manager: SignalManager,
}

impl ProcessManager {
    /// Creates a new ProcessManager with an empty process table.
    pub fn new() -> Self {
        ProcessManager {
            process_table: HashMap::new(),
            next_pid: 0,
            signal_manager: SignalManager::new(),
        }
    }

    /// Creates a new process and adds it to the process table.
    ///
    /// This is the primitive creation operation, used to bootstrap
    /// the first processes (idle, init) that aren't forked.
    pub fn create_process(&mut self, name: &str, priority: Option<u8>) -> u32 {
        let pid = self.next_pid;
        self.next_pid += 1;

        let pcb = ProcessControlBlock::new(pid, name, priority);
        self.process_table.insert(pid, pcb);
        pid
    }

    /// Forks a process, creating a child that is an exact copy.
    ///
    /// ## How fork() works
    ///
    /// 1. Allocate a new PID for the child.
    /// 2. Copy the parent's PCB (registers, PC, memory, handlers, priority).
    /// 3. Reset child-specific fields (empty children, no pending signals, cpu_time=0).
    /// 4. Set return values: parent's register a0 = child_pid, child's a0 = 0.
    /// 5. Add child to parent's children list and process table.
    ///
    /// ## Return values
    ///
    /// The magic of fork() is that both processes resume from the same point
    /// but see different return values in register x10 (a0):
    /// - Parent sees the child's PID
    /// - Child sees 0
    ///
    /// This is how the program knows which process it is:
    /// ```text
    /// pid = fork()
    /// if pid == 0: "I am the child"
    /// else: "I am the parent, child PID is pid"
    /// ```
    pub fn fork(&mut self, parent_pid: u32) -> Option<u32> {
        // Verify parent exists.
        if !self.process_table.contains_key(&parent_pid) {
            return None;
        }

        // Step 1: Allocate new PID.
        let child_pid = self.next_pid;
        self.next_pid += 1;

        // Step 2-3: Create child PCB by copying parent.
        let child_pcb = self.process_table[&parent_pid].fork_copy(child_pid);

        // Step 4: Set return values in register a0 (x10).
        // Parent sees child_pid.
        self.process_table
            .get_mut(&parent_pid)
            .unwrap()
            .registers[10] = child_pid as i32;

        // Step 5: Add child to parent's children list.
        self.process_table
            .get_mut(&parent_pid)
            .unwrap()
            .children
            .push(child_pid);

        // Insert child into process table (child's a0 is already 0 from fork_copy).
        self.process_table.insert(child_pid, child_pcb);

        Some(child_pid)
    }

    /// Replaces a process's program (exec).
    ///
    /// The PID stays the same, but registers are zeroed, PC is set to
    /// the entry point, SP is set to the new stack, and signal handlers
    /// and pending signals are cleared.
    ///
    /// ## What changes vs. what stays
    ///
    /// | Changes          | Stays            |
    /// |------------------|------------------|
    /// | Registers (zero) | PID              |
    /// | PC (entry point) | Parent PID       |
    /// | SP (new stack)   | Children         |
    /// | Signal handlers  | Priority         |
    /// | Pending signals  | CPU time         |
    pub fn exec(&mut self, pid: u32, entry_point: u32, stack_pointer: u32) -> bool {
        let pcb = match self.process_table.get_mut(&pid) {
            Some(p) => p,
            None => return false,
        };

        // Zero all registers.
        pcb.registers = [0; NUM_REGISTERS];

        // Set PC to new program's entry point.
        pcb.pc = entry_point;

        // Set stack pointer.
        pcb.sp = stack_pointer;

        // Clear signal handlers (new program doesn't know old handlers).
        pcb.signal_handlers.clear();

        // Clear pending signals (new program shouldn't inherit old signals).
        pcb.pending_signals.clear();

        true
    }

    /// Waits for a zombie child and reaps it.
    ///
    /// Scans the parent's children list for any child in Zombie state.
    /// If found, returns `Some((child_pid, exit_code))` and removes the
    /// zombie from the process table.
    ///
    /// Returns `None` if no zombie children exist (in a real OS, the
    /// parent would block until a child exits).
    pub fn wait(&mut self, parent_pid: u32) -> Option<(u32, i32)> {
        // Find a zombie child.
        let zombie_pid = {
            let parent = self.process_table.get(&parent_pid)?;
            parent.children.iter().find(|&&child_pid| {
                self.process_table
                    .get(&child_pid)
                    .map_or(false, |c| c.is_zombie())
            }).copied()
        };

        let zombie_pid = zombie_pid?;

        // Get exit code before removing.
        let exit_code = self.process_table[&zombie_pid].exit_code;

        // Remove zombie from process table.
        self.process_table.remove(&zombie_pid);

        // Remove from parent's children list.
        if let Some(parent) = self.process_table.get_mut(&parent_pid) {
            parent.children.retain(|&pid| pid != zombie_pid);
        }

        Some((zombie_pid, exit_code))
    }

    /// Sends a signal to a process.
    ///
    /// This is the Unix kill() system call. Despite its name, it doesn't
    /// necessarily kill — it delivers a signal. Only SIGKILL guarantees
    /// termination.
    pub fn kill(&mut self, target_pid: u32, signal: Signal) -> bool {
        let pcb = match self.process_table.get_mut(&target_pid) {
            Some(p) => p,
            None => return false,
        };
        self.signal_manager.send_signal(pcb, signal);
        true
    }

    /// Terminates a process voluntarily.
    ///
    /// 1. Sets state to Zombie.
    /// 2. Records exit code.
    /// 3. Reparents all children to PID 0 (idle/init).
    /// 4. Sends SIGCHLD to the parent.
    ///
    /// The process becomes a zombie until its parent calls wait().
    pub fn exit_process(&mut self, pid: u32, exit_code: i32) -> bool {
        // Collect children list first (to avoid borrow issues).
        let (children, parent_pid) = {
            let pcb = match self.process_table.get_mut(&pid) {
                Some(p) => p,
                None => return false,
            };
            pcb.state = ProcessState::Zombie;
            pcb.exit_code = exit_code;
            let children = pcb.children.clone();
            pcb.children.clear();
            (children, pcb.parent_pid)
        };

        // Reparent children to PID 0.
        for child_pid in &children {
            if let Some(child) = self.process_table.get_mut(child_pid) {
                child.parent_pid = 0;
            }
            // Add to PID 0's children list.
            if let Some(init) = self.process_table.get_mut(&0) {
                init.children.push(*child_pid);
            }
        }

        // Send SIGCHLD to parent.
        if let Some(parent) = self.process_table.get_mut(&parent_pid) {
            self.signal_manager.send_signal(parent, Signal::SigChld);
        }

        true
    }

    /// Returns the number of processes in the table.
    pub fn process_count(&self) -> usize {
        self.process_table.len()
    }

    /// Returns true if a process with the given PID exists.
    pub fn process_exists(&self, pid: u32) -> bool {
        self.process_table.contains_key(&pid)
    }

    /// Returns a reference to the PCB for a given PID.
    pub fn get_process(&self, pid: u32) -> Option<&ProcessControlBlock> {
        self.process_table.get(&pid)
    }
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// PriorityScheduler
// ============================================================================

/// Priority-based scheduler with round-robin within each level.
///
/// Maintains 40 separate FIFO queues (one per priority level, 0–39).
/// When scheduling, it picks the front process from the highest-priority
/// (lowest-numbered) non-empty queue.
///
/// ## Time Quantum
///
/// Higher-priority processes get larger time slices:
/// - Priority 0: 200 cycles (kernel tasks)
/// - Priority 20: ~123 cycles (normal users)
/// - Priority 39: 50 cycles (background)
///
/// ## Starvation
///
/// A continuous stream of high-priority processes could prevent low-priority
/// ones from running. Real schedulers use "aging" to address this. We note
/// it as a future enhancement.
pub struct PriorityScheduler {
    /// 40 ready queues, one per priority level.
    ready_queues: Vec<VecDeque<ProcessControlBlock>>,
}

/// Number of priority levels (0-39 inclusive).
pub const NUM_PRIORITIES: usize = 40;
/// Maximum time quantum (highest priority).
pub const MAX_QUANTUM: u32 = 200;
/// Minimum time quantum (lowest priority).
pub const MIN_QUANTUM: u32 = 50;

impl PriorityScheduler {
    /// Creates a new scheduler with 40 empty ready queues.
    pub fn new() -> Self {
        let mut queues = Vec::with_capacity(NUM_PRIORITIES);
        for _ in 0..NUM_PRIORITIES {
            queues.push(VecDeque::new());
        }
        PriorityScheduler {
            ready_queues: queues,
        }
    }

    /// Adds a process to the back of its priority queue.
    pub fn add_process(&mut self, pcb: ProcessControlBlock) {
        let priority = (pcb.priority as usize).min(NUM_PRIORITIES - 1);
        self.ready_queues[priority].push_back(pcb);
    }

    /// Removes a process by PID from all queues.
    ///
    /// Returns the removed PCB, or None if not found.
    pub fn remove_process(&mut self, pid: u32) -> Option<ProcessControlBlock> {
        for queue in &mut self.ready_queues {
            if let Some(pos) = queue.iter().position(|pcb| pcb.pid == pid) {
                return queue.remove(pos);
            }
        }
        None
    }

    /// Selects the next process to run.
    ///
    /// Iterates from priority 0 (highest) to 39 (lowest), returning
    /// the front process from the first non-empty queue. The process
    /// is removed from the queue; the caller re-enqueues it after its
    /// time slice expires.
    pub fn schedule(&mut self) -> Option<ProcessControlBlock> {
        for queue in &mut self.ready_queues {
            if !queue.is_empty() {
                return queue.pop_front();
            }
        }
        None
    }

    /// Changes a process's priority, moving it to the new queue.
    ///
    /// Returns true if the process was found and moved.
    pub fn set_priority(&mut self, pid: u32, new_priority: u8) -> bool {
        let new_priority = new_priority.min(MAX_PRIORITY);

        let mut pcb = match self.remove_process(pid) {
            Some(p) => p,
            None => return false,
        };

        pcb.priority = new_priority;
        self.add_process(pcb);
        true
    }

    /// Calculates the time quantum for a priority level.
    ///
    /// Linearly interpolates between MAX_QUANTUM (priority 0) and
    /// MIN_QUANTUM (priority 39).
    pub fn time_quantum_for(priority: u8) -> u32 {
        let p = (priority as u32).min(MAX_PRIORITY as u32);
        let range = MAX_QUANTUM - MIN_QUANTUM;
        MAX_QUANTUM - (p * range / (NUM_PRIORITIES as u32 - 1))
    }

    /// Returns the total number of processes across all queues.
    pub fn total_ready(&self) -> usize {
        self.ready_queues.iter().map(|q| q.len()).sum()
    }

    /// Returns true if all queues are empty.
    pub fn is_empty(&self) -> bool {
        self.ready_queues.iter().all(|q| q.is_empty())
    }
}

impl Default for PriorityScheduler {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- ProcessState Tests ----

    #[test]
    fn test_process_state_values() {
        assert_eq!(ProcessState::Ready as i32, 0);
        assert_eq!(ProcessState::Running as i32, 1);
        assert_eq!(ProcessState::Blocked as i32, 2);
        assert_eq!(ProcessState::Terminated as i32, 3);
        assert_eq!(ProcessState::Zombie as i32, 4);
    }

    // ---- Signal Tests ----

    #[test]
    fn test_signal_values_match_posix() {
        assert_eq!(Signal::SigInt.value(), 2);
        assert_eq!(Signal::SigKill.value(), 9);
        assert_eq!(Signal::SigTerm.value(), 15);
        assert_eq!(Signal::SigChld.value(), 17);
        assert_eq!(Signal::SigCont.value(), 18);
        assert_eq!(Signal::SigStop.value(), 19);
    }

    #[test]
    fn test_uncatchable_signals() {
        assert!(Signal::SigKill.is_uncatchable());
        assert!(Signal::SigStop.is_uncatchable());
        assert!(!Signal::SigTerm.is_uncatchable());
        assert!(!Signal::SigInt.is_uncatchable());
        assert!(!Signal::SigChld.is_uncatchable());
        assert!(!Signal::SigCont.is_uncatchable());
    }

    #[test]
    fn test_fatal_by_default() {
        assert!(Signal::SigInt.is_fatal_by_default());
        assert!(Signal::SigKill.is_fatal_by_default());
        assert!(Signal::SigTerm.is_fatal_by_default());
        assert!(!Signal::SigChld.is_fatal_by_default());
        assert!(!Signal::SigCont.is_fatal_by_default());
        assert!(!Signal::SigStop.is_fatal_by_default());
    }

    #[test]
    fn test_signal_names() {
        assert_eq!(Signal::SigInt.name(), "SIGINT");
        assert_eq!(Signal::SigKill.name(), "SIGKILL");
        assert_eq!(Signal::SigTerm.name(), "SIGTERM");
        assert_eq!(Signal::SigChld.name(), "SIGCHLD");
        assert_eq!(Signal::SigCont.name(), "SIGCONT");
        assert_eq!(Signal::SigStop.name(), "SIGSTOP");
    }

    // ---- ProcessControlBlock Tests ----

    #[test]
    fn test_pcb_creation_defaults() {
        let pcb = ProcessControlBlock::new(1, "test", None);
        assert_eq!(pcb.pid, 1);
        assert_eq!(pcb.name, "test");
        assert_eq!(pcb.state, ProcessState::Ready);
        assert_eq!(pcb.priority, 20);
        assert_eq!(pcb.pc, 0);
        assert_eq!(pcb.sp, 0);
        assert_eq!(pcb.memory_base, 0);
        assert_eq!(pcb.memory_size, 0);
        assert_eq!(pcb.parent_pid, 0);
        assert_eq!(pcb.cpu_time, 0);
        assert_eq!(pcb.exit_code, 0);
        assert!(pcb.children.is_empty());
        assert!(pcb.pending_signals.is_empty());
        assert!(pcb.signal_handlers.is_empty());
        assert!(pcb.signal_mask.is_empty());
    }

    #[test]
    fn test_pcb_registers_initialized_to_zero() {
        let pcb = ProcessControlBlock::new(1, "test", None);
        assert_eq!(pcb.registers.len(), 32);
        assert!(pcb.registers.iter().all(|&r| r == 0));
    }

    #[test]
    fn test_pcb_custom_priority() {
        let pcb = ProcessControlBlock::new(1, "high", Some(5));
        assert_eq!(pcb.priority, 5);
    }

    #[test]
    fn test_pcb_priority_clamped() {
        let pcb = ProcessControlBlock::new(1, "over", Some(100));
        assert_eq!(pcb.priority, 39);
    }

    #[test]
    fn test_pcb_state_predicates() {
        let mut pcb = ProcessControlBlock::new(1, "test", None);
        assert!(pcb.is_ready());
        assert!(!pcb.is_running());

        pcb.state = ProcessState::Running;
        assert!(pcb.is_running());
        assert!(!pcb.is_ready());

        pcb.state = ProcessState::Blocked;
        assert!(pcb.is_blocked());

        pcb.state = ProcessState::Zombie;
        assert!(pcb.is_zombie());

        pcb.state = ProcessState::Terminated;
        assert!(pcb.is_terminated());
    }

    #[test]
    fn test_pcb_fork_copy() {
        let mut parent = ProcessControlBlock::new(1, "parent", Some(10));
        parent.registers[5] = 42;
        parent.pc = 0x1000;
        parent.sp = 0x7FFF;
        parent.memory_base = 0x2000;
        parent.memory_size = 4096;
        parent.signal_handlers.insert(Signal::SigTerm, 0x3000);
        parent.cpu_time = 500;

        let child = parent.fork_copy(99);

        assert_eq!(child.pid, 99);
        assert_eq!(child.parent_pid, 1);
        assert_eq!(child.name, "parent");
        assert_eq!(child.registers[5], 42);
        assert_eq!(child.pc, 0x1000);
        assert_eq!(child.sp, 0x7FFF);
        assert_eq!(child.memory_base, 0x2000);
        assert_eq!(child.memory_size, 4096);
        assert_eq!(child.priority, 10);
        assert_eq!(child.signal_handlers[&Signal::SigTerm], 0x3000);

        // Reset fields.
        assert_eq!(child.state, ProcessState::Ready);
        assert!(child.children.is_empty());
        assert!(child.pending_signals.is_empty());
        assert_eq!(child.cpu_time, 0);
        assert_eq!(child.exit_code, 0);
    }

    #[test]
    fn test_fork_copy_independent_handlers() {
        let mut parent = ProcessControlBlock::new(1, "p", None);
        parent.signal_handlers.insert(Signal::SigTerm, 0x1000);

        let mut child = parent.fork_copy(2);
        child.signal_handlers.insert(Signal::SigInt, 0x2000);

        assert!(!parent.signal_handlers.contains_key(&Signal::SigInt));
    }

    // ---- SignalManager Tests ----

    #[test]
    fn test_send_signal_adds_to_pending() {
        let sm = SignalManager::new();
        let mut pcb = ProcessControlBlock::new(1, "test", None);

        sm.send_signal(&mut pcb, Signal::SigTerm);
        assert_eq!(pcb.pending_signals, vec![Signal::SigTerm]);
    }

    #[test]
    fn test_send_multiple_signals() {
        let sm = SignalManager::new();
        let mut pcb = ProcessControlBlock::new(1, "test", None);

        sm.send_signal(&mut pcb, Signal::SigTerm);
        sm.send_signal(&mut pcb, Signal::SigInt);
        assert_eq!(pcb.pending_signals, vec![Signal::SigTerm, Signal::SigInt]);
    }

    #[test]
    fn test_deliver_fatal_signal_without_handler() {
        let sm = SignalManager::new();
        let mut pcb = ProcessControlBlock::new(1, "test", None);

        sm.send_signal(&mut pcb, Signal::SigTerm);
        let actions = sm.deliver_pending(&mut pcb);

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0], (Signal::SigTerm, SignalAction::Kill));
        assert!(pcb.pending_signals.is_empty());
    }

    #[test]
    fn test_deliver_signal_with_handler() {
        let sm = SignalManager::new();
        let mut pcb = ProcessControlBlock::new(1, "test", None);

        sm.register_handler(&mut pcb, Signal::SigTerm, 0x1000);
        sm.send_signal(&mut pcb, Signal::SigTerm);
        let actions = sm.deliver_pending(&mut pcb);

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0], (Signal::SigTerm, SignalAction::Handler(0x1000)));
    }

    #[test]
    fn test_deliver_sigkill_always_kills() {
        let sm = SignalManager::new();
        let mut pcb = ProcessControlBlock::new(1, "test", None);

        sm.send_signal(&mut pcb, Signal::SigKill);
        let actions = sm.deliver_pending(&mut pcb);

        assert_eq!(actions[0].1, SignalAction::Kill);
    }

    #[test]
    fn test_deliver_sigstop() {
        let sm = SignalManager::new();
        let mut pcb = ProcessControlBlock::new(1, "test", None);

        sm.send_signal(&mut pcb, Signal::SigStop);
        let actions = sm.deliver_pending(&mut pcb);

        assert_eq!(actions[0].1, SignalAction::Stop);
    }

    #[test]
    fn test_deliver_sigcont() {
        let sm = SignalManager::new();
        let mut pcb = ProcessControlBlock::new(1, "test", None);

        sm.send_signal(&mut pcb, Signal::SigCont);
        let actions = sm.deliver_pending(&mut pcb);

        assert_eq!(actions[0].1, SignalAction::Continue);
    }

    #[test]
    fn test_deliver_sigchld_ignored() {
        let sm = SignalManager::new();
        let mut pcb = ProcessControlBlock::new(1, "test", None);

        sm.send_signal(&mut pcb, Signal::SigChld);
        let actions = sm.deliver_pending(&mut pcb);

        assert_eq!(actions[0].1, SignalAction::Ignore);
    }

    #[test]
    fn test_cannot_register_handler_for_sigkill() {
        let sm = SignalManager::new();
        let mut pcb = ProcessControlBlock::new(1, "test", None);
        assert!(!sm.register_handler(&mut pcb, Signal::SigKill, 0x1000));
        assert!(!pcb.signal_handlers.contains_key(&Signal::SigKill));
    }

    #[test]
    fn test_cannot_register_handler_for_sigstop() {
        let sm = SignalManager::new();
        let mut pcb = ProcessControlBlock::new(1, "test", None);
        assert!(!sm.register_handler(&mut pcb, Signal::SigStop, 0x1000));
    }

    #[test]
    fn test_mask_signal() {
        let sm = SignalManager::new();
        let mut pcb = ProcessControlBlock::new(1, "test", None);

        assert!(sm.mask(&mut pcb, Signal::SigTerm));
        assert!(pcb.signal_mask.contains(&Signal::SigTerm));
    }

    #[test]
    fn test_unmask_signal() {
        let sm = SignalManager::new();
        let mut pcb = ProcessControlBlock::new(1, "test", None);

        sm.mask(&mut pcb, Signal::SigTerm);
        sm.unmask(&mut pcb, Signal::SigTerm);
        assert!(!pcb.signal_mask.contains(&Signal::SigTerm));
    }

    #[test]
    fn test_cannot_mask_sigkill() {
        let sm = SignalManager::new();
        let mut pcb = ProcessControlBlock::new(1, "test", None);
        assert!(!sm.mask(&mut pcb, Signal::SigKill));
    }

    #[test]
    fn test_cannot_mask_sigstop() {
        let sm = SignalManager::new();
        let mut pcb = ProcessControlBlock::new(1, "test", None);
        assert!(!sm.mask(&mut pcb, Signal::SigStop));
    }

    #[test]
    fn test_masked_signal_stays_pending() {
        let sm = SignalManager::new();
        let mut pcb = ProcessControlBlock::new(1, "test", None);

        sm.mask(&mut pcb, Signal::SigTerm);
        sm.send_signal(&mut pcb, Signal::SigTerm);
        let actions = sm.deliver_pending(&mut pcb);

        assert!(actions.is_empty());
        assert_eq!(pcb.pending_signals, vec![Signal::SigTerm]);
    }

    #[test]
    fn test_unmask_then_deliver() {
        let sm = SignalManager::new();
        let mut pcb = ProcessControlBlock::new(1, "test", None);

        sm.mask(&mut pcb, Signal::SigTerm);
        sm.send_signal(&mut pcb, Signal::SigTerm);
        sm.deliver_pending(&mut pcb); // stays pending

        sm.unmask(&mut pcb, Signal::SigTerm);
        let actions = sm.deliver_pending(&mut pcb);

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].1, SignalAction::Kill);
        assert!(pcb.pending_signals.is_empty());
    }

    #[test]
    fn test_sigkill_bypasses_mask() {
        let sm = SignalManager::new();
        let mut pcb = ProcessControlBlock::new(1, "test", None);

        // Manually insert SIGKILL into mask (can't do it via sm.mask).
        pcb.signal_mask.insert(Signal::SigKill);
        sm.send_signal(&mut pcb, Signal::SigKill);
        let actions = sm.deliver_pending(&mut pcb);

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].1, SignalAction::Kill);
    }

    #[test]
    fn test_is_fatal_sigkill() {
        let sm = SignalManager::new();
        let pcb = ProcessControlBlock::new(1, "test", None);
        assert!(sm.is_fatal(&pcb, Signal::SigKill));
    }

    #[test]
    fn test_is_fatal_sigterm_without_handler() {
        let sm = SignalManager::new();
        let pcb = ProcessControlBlock::new(1, "test", None);
        assert!(sm.is_fatal(&pcb, Signal::SigTerm));
    }

    #[test]
    fn test_is_fatal_sigterm_with_handler() {
        let sm = SignalManager::new();
        let mut pcb = ProcessControlBlock::new(1, "test", None);
        sm.register_handler(&mut pcb, Signal::SigTerm, 0x1000);
        assert!(!sm.is_fatal(&pcb, Signal::SigTerm));
    }

    #[test]
    fn test_sigchld_not_fatal() {
        let sm = SignalManager::new();
        let pcb = ProcessControlBlock::new(1, "test", None);
        assert!(!sm.is_fatal(&pcb, Signal::SigChld));
    }

    // ---- ProcessManager Tests ----

    #[test]
    fn test_create_process_sequential_pids() {
        let mut pm = ProcessManager::new();
        let p1 = pm.create_process("a", None);
        let p2 = pm.create_process("b", None);
        assert_eq!(p2, p1 + 1);
    }

    #[test]
    fn test_create_process_sets_name() {
        let mut pm = ProcessManager::new();
        let pid = pm.create_process("my_proc", None);
        assert_eq!(pm.get_process(pid).unwrap().name, "my_proc");
    }

    #[test]
    fn test_create_process_default_priority() {
        let mut pm = ProcessManager::new();
        let pid = pm.create_process("normal", None);
        assert_eq!(pm.get_process(pid).unwrap().priority, 20);
    }

    #[test]
    fn test_create_process_custom_priority() {
        let mut pm = ProcessManager::new();
        let pid = pm.create_process("kernel", Some(0));
        assert_eq!(pm.get_process(pid).unwrap().priority, 0);
    }

    #[test]
    fn test_create_process_is_ready() {
        let mut pm = ProcessManager::new();
        let pid = pm.create_process("new", None);
        assert!(pm.get_process(pid).unwrap().is_ready());
    }

    #[test]
    fn test_process_count() {
        let mut pm = ProcessManager::new();
        assert_eq!(pm.process_count(), 0);
        pm.create_process("a", None);
        assert_eq!(pm.process_count(), 1);
        pm.create_process("b", None);
        assert_eq!(pm.process_count(), 2);
    }

    #[test]
    fn test_process_exists() {
        let mut pm = ProcessManager::new();
        let pid = pm.create_process("exists", None);
        assert!(pm.process_exists(pid));
        assert!(!pm.process_exists(9999));
    }

    #[test]
    fn test_fork_returns_child_pid() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", None);
        let child = pm.fork(init).unwrap();
        assert!(child > init);
    }

    #[test]
    fn test_fork_child_different_pid() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", None);
        let child = pm.fork(init).unwrap();
        assert_ne!(init, child);
    }

    #[test]
    fn test_fork_child_parent_pid() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", None);
        let child = pm.fork(init).unwrap();
        assert_eq!(pm.get_process(child).unwrap().parent_pid, init);
    }

    #[test]
    fn test_fork_child_in_parent_children() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", None);
        let child = pm.fork(init).unwrap();
        assert!(pm.get_process(init).unwrap().children.contains(&child));
    }

    #[test]
    fn test_fork_child_is_ready() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", None);
        let child = pm.fork(init).unwrap();
        assert!(pm.get_process(child).unwrap().is_ready());
    }

    #[test]
    fn test_fork_child_cpu_time_zero() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", None);
        pm.process_table.get_mut(&init).unwrap().cpu_time = 500;
        let child = pm.fork(init).unwrap();
        assert_eq!(pm.get_process(child).unwrap().cpu_time, 0);
    }

    #[test]
    fn test_fork_child_inherits_priority() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", Some(5));
        let child = pm.fork(init).unwrap();
        assert_eq!(pm.get_process(child).unwrap().priority, 5);
    }

    #[test]
    fn test_fork_return_values_in_registers() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", None);
        let child = pm.fork(init).unwrap();

        // Parent's a0 = child pid.
        assert_eq!(pm.get_process(init).unwrap().registers[10], child as i32);
        // Child's a0 = 0.
        assert_eq!(pm.get_process(child).unwrap().registers[10], 0);
    }

    #[test]
    fn test_fork_copies_registers() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", None);
        pm.process_table.get_mut(&init).unwrap().registers[5] = 42;
        pm.process_table.get_mut(&init).unwrap().registers[15] = 99;

        let child = pm.fork(init).unwrap();
        assert_eq!(pm.get_process(child).unwrap().registers[5], 42);
        assert_eq!(pm.get_process(child).unwrap().registers[15], 99);
    }

    #[test]
    fn test_fork_copies_pc_sp() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", None);
        pm.process_table.get_mut(&init).unwrap().pc = 0x1000;
        pm.process_table.get_mut(&init).unwrap().sp = 0x7FFF;

        let child = pm.fork(init).unwrap();
        assert_eq!(pm.get_process(child).unwrap().pc, 0x1000);
        assert_eq!(pm.get_process(child).unwrap().sp, 0x7FFF);
    }

    #[test]
    fn test_fork_inherits_signal_handlers() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", None);
        pm.process_table
            .get_mut(&init)
            .unwrap()
            .signal_handlers
            .insert(Signal::SigTerm, 0x3000);

        let child = pm.fork(init).unwrap();
        assert_eq!(
            pm.get_process(child).unwrap().signal_handlers[&Signal::SigTerm],
            0x3000
        );
    }

    #[test]
    fn test_fork_child_empty_children() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", None);
        let child = pm.fork(init).unwrap();
        assert!(pm.get_process(child).unwrap().children.is_empty());
    }

    #[test]
    fn test_fork_child_no_pending_signals() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", None);
        pm.process_table
            .get_mut(&init)
            .unwrap()
            .pending_signals
            .push(Signal::SigTerm);

        let child = pm.fork(init).unwrap();
        assert!(pm.get_process(child).unwrap().pending_signals.is_empty());
    }

    #[test]
    fn test_fork_nonexistent_parent() {
        let mut pm = ProcessManager::new();
        assert!(pm.fork(9999).is_none());
    }

    #[test]
    fn test_exec_sets_pc() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", None);
        let child = pm.fork(init).unwrap();

        pm.exec(child, 0x10000, 0x7FFFF);
        assert_eq!(pm.get_process(child).unwrap().pc, 0x10000);
    }

    #[test]
    fn test_exec_sets_sp() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", None);
        let child = pm.fork(init).unwrap();

        pm.exec(child, 0x10000, 0x7FFFF);
        assert_eq!(pm.get_process(child).unwrap().sp, 0x7FFFF);
    }

    #[test]
    fn test_exec_zeros_registers() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", None);
        let child = pm.fork(init).unwrap();
        pm.process_table.get_mut(&child).unwrap().registers[5] = 42;

        pm.exec(child, 0x10000, 0x7FFFF);
        assert!(pm.get_process(child).unwrap().registers.iter().all(|&r| r == 0));
    }

    #[test]
    fn test_exec_clears_handlers() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", None);
        let child = pm.fork(init).unwrap();
        pm.process_table
            .get_mut(&child)
            .unwrap()
            .signal_handlers
            .insert(Signal::SigTerm, 0x1000);

        pm.exec(child, 0x10000, 0x7FFFF);
        assert!(pm.get_process(child).unwrap().signal_handlers.is_empty());
    }

    #[test]
    fn test_exec_clears_pending_signals() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", None);
        let child = pm.fork(init).unwrap();
        pm.process_table
            .get_mut(&child)
            .unwrap()
            .pending_signals
            .push(Signal::SigTerm);

        pm.exec(child, 0x10000, 0x7FFFF);
        assert!(pm.get_process(child).unwrap().pending_signals.is_empty());
    }

    #[test]
    fn test_exec_preserves_pid() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", None);
        let child = pm.fork(init).unwrap();

        pm.exec(child, 0x10000, 0x7FFFF);
        assert_eq!(pm.get_process(child).unwrap().pid, child);
    }

    #[test]
    fn test_exec_preserves_parent_pid() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", None);
        let child = pm.fork(init).unwrap();

        pm.exec(child, 0x10000, 0x7FFFF);
        assert_eq!(pm.get_process(child).unwrap().parent_pid, init);
    }

    #[test]
    fn test_exec_nonexistent_pid() {
        let mut pm = ProcessManager::new();
        assert!(!pm.exec(9999, 0x1000, 0x7FFF));
    }

    #[test]
    fn test_wait_returns_zombie_child() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", None);
        let child = pm.fork(init).unwrap();

        pm.exit_process(child, 42);
        let result = pm.wait(init);

        assert_eq!(result, Some((child, 42)));
    }

    #[test]
    fn test_wait_removes_zombie() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", None);
        let child = pm.fork(init).unwrap();

        pm.exit_process(child, 0);
        pm.wait(init);

        assert!(!pm.process_exists(child));
    }

    #[test]
    fn test_wait_removes_from_children_list() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", None);
        let child = pm.fork(init).unwrap();

        pm.exit_process(child, 0);
        pm.wait(init);

        assert!(!pm.get_process(init).unwrap().children.contains(&child));
    }

    #[test]
    fn test_wait_returns_none_when_no_zombies() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", None);
        pm.fork(init).unwrap(); // child is Ready, not Zombie

        assert!(pm.wait(init).is_none());
    }

    #[test]
    fn test_wait_returns_none_when_no_children() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", None);
        assert!(pm.wait(init).is_none());
    }

    #[test]
    fn test_wait_nonexistent_parent() {
        let mut pm = ProcessManager::new();
        assert!(pm.wait(9999).is_none());
    }

    #[test]
    fn test_kill_sends_signal() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", None);
        let child = pm.fork(init).unwrap();

        assert!(pm.kill(child, Signal::SigTerm));
        assert!(pm.get_process(child).unwrap().pending_signals.contains(&Signal::SigTerm));
    }

    #[test]
    fn test_kill_nonexistent_pid() {
        let mut pm = ProcessManager::new();
        assert!(!pm.kill(9999, Signal::SigTerm));
    }

    #[test]
    fn test_exit_sets_zombie() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", None);
        let child = pm.fork(init).unwrap();

        pm.exit_process(child, 0);
        assert!(pm.get_process(child).unwrap().is_zombie());
    }

    #[test]
    fn test_exit_sets_exit_code() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", None);
        let child = pm.fork(init).unwrap();

        pm.exit_process(child, 42);
        assert_eq!(pm.get_process(child).unwrap().exit_code, 42);
    }

    #[test]
    fn test_exit_reparents_children() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", None);
        let child = pm.fork(init).unwrap();
        let grandchild = pm.fork(child).unwrap();

        pm.exit_process(child, 0);
        assert_eq!(pm.get_process(grandchild).unwrap().parent_pid, 0);
    }

    #[test]
    fn test_exit_sends_sigchld() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", None);
        let child = pm.fork(init).unwrap();

        pm.exit_process(child, 0);
        assert!(pm.get_process(init).unwrap().pending_signals.contains(&Signal::SigChld));
    }

    #[test]
    fn test_exit_clears_children() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", None);
        let child = pm.fork(init).unwrap();
        pm.fork(child).unwrap(); // grandchild

        pm.exit_process(child, 0);
        assert!(pm.get_process(child).unwrap().children.is_empty());
    }

    #[test]
    fn test_exit_nonexistent_pid() {
        let mut pm = ProcessManager::new();
        assert!(!pm.exit_process(9999, 0));
    }

    // ---- Integration: fork + exec + wait ----

    #[test]
    fn test_fork_exec_wait_lifecycle() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", None);

        let child = pm.fork(init).unwrap();
        assert!(pm.process_exists(child));

        pm.exec(child, 0x10000, 0x7FFFF);
        assert_eq!(pm.get_process(child).unwrap().pc, 0x10000);

        pm.exit_process(child, 42);
        assert!(pm.get_process(child).unwrap().is_zombie());

        let result = pm.wait(init).unwrap();
        assert_eq!(result, (child, 42));
        assert!(!pm.process_exists(child));
    }

    #[test]
    fn test_multiple_fork_and_wait() {
        let mut pm = ProcessManager::new();
        let init = pm.create_process("init", None);

        let c1 = pm.fork(init).unwrap();
        let c2 = pm.fork(init).unwrap();
        let c3 = pm.fork(init).unwrap();

        pm.exit_process(c2, 20);
        pm.exit_process(c1, 10);
        pm.exit_process(c3, 30);

        let mut codes = Vec::new();
        for _ in 0..3 {
            if let Some((_, code)) = pm.wait(init) {
                codes.push(code);
            }
        }
        codes.sort();
        assert_eq!(codes, vec![10, 20, 30]);
    }

    // ---- PriorityScheduler Tests ----

    fn make_pcb(pid: u32, name: &str, priority: u8) -> ProcessControlBlock {
        ProcessControlBlock::new(pid, name, Some(priority))
    }

    #[test]
    fn test_schedule_empty() {
        let mut sched = PriorityScheduler::new();
        assert!(sched.schedule().is_none());
    }

    #[test]
    fn test_schedule_single_process() {
        let mut sched = PriorityScheduler::new();
        sched.add_process(make_pcb(1, "solo", 20));

        let result = sched.schedule().unwrap();
        assert_eq!(result.pid, 1);
    }

    #[test]
    fn test_schedule_picks_highest_priority() {
        let mut sched = PriorityScheduler::new();

        // Add in reverse order.
        sched.add_process(make_pcb(3, "low", 39));
        sched.add_process(make_pcb(2, "normal", 20));
        sched.add_process(make_pcb(1, "high", 5));

        assert_eq!(sched.schedule().unwrap().pid, 1);
        assert_eq!(sched.schedule().unwrap().pid, 2);
        assert_eq!(sched.schedule().unwrap().pid, 3);
        assert!(sched.schedule().is_none());
    }

    #[test]
    fn test_round_robin_same_priority() {
        let mut sched = PriorityScheduler::new();
        sched.add_process(make_pcb(1, "a", 20));
        sched.add_process(make_pcb(2, "b", 20));

        assert_eq!(sched.schedule().unwrap().pid, 1);
        assert_eq!(sched.schedule().unwrap().pid, 2);
        assert!(sched.schedule().is_none());
    }

    #[test]
    fn test_round_robin_with_readd() {
        let mut sched = PriorityScheduler::new();
        sched.add_process(make_pcb(1, "a", 20));
        sched.add_process(make_pcb(2, "b", 20));

        let first = sched.schedule().unwrap();
        assert_eq!(first.pid, 1);
        sched.add_process(first);

        assert_eq!(sched.schedule().unwrap().pid, 2);
    }

    #[test]
    fn test_remove_process() {
        let mut sched = PriorityScheduler::new();
        sched.add_process(make_pcb(1, "removable", 20));

        let removed = sched.remove_process(1);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().pid, 1);
        assert!(sched.schedule().is_none());
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut sched = PriorityScheduler::new();
        assert!(sched.remove_process(999).is_none());
    }

    #[test]
    fn test_remove_from_middle() {
        let mut sched = PriorityScheduler::new();
        sched.add_process(make_pcb(1, "a", 20));
        sched.add_process(make_pcb(2, "b", 20));
        sched.add_process(make_pcb(3, "c", 20));

        sched.remove_process(2);

        assert_eq!(sched.schedule().unwrap().pid, 1);
        assert_eq!(sched.schedule().unwrap().pid, 3);
        assert!(sched.schedule().is_none());
    }

    #[test]
    fn test_set_priority() {
        let mut sched = PriorityScheduler::new();
        sched.add_process(make_pcb(1, "moving", 20));

        assert!(sched.set_priority(1, 5));
        let result = sched.schedule().unwrap();
        assert_eq!(result.pid, 1);
        assert_eq!(result.priority, 5);
    }

    #[test]
    fn test_set_priority_nonexistent() {
        let mut sched = PriorityScheduler::new();
        assert!(!sched.set_priority(999, 10));
    }

    #[test]
    fn test_set_priority_clamps() {
        let mut sched = PriorityScheduler::new();
        sched.add_process(make_pcb(1, "clamped", 20));

        sched.set_priority(1, 100);
        let result = sched.schedule().unwrap();
        assert_eq!(result.priority, 39);
    }

    #[test]
    fn test_set_priority_affects_order() {
        let mut sched = PriorityScheduler::new();
        sched.add_process(make_pcb(1, "a", 20));
        sched.add_process(make_pcb(2, "b", 20));

        // Boost b to higher priority.
        sched.set_priority(2, 5);

        assert_eq!(sched.schedule().unwrap().pid, 2);
        assert_eq!(sched.schedule().unwrap().pid, 1);
    }

    #[test]
    fn test_time_quantum_highest() {
        assert_eq!(PriorityScheduler::time_quantum_for(0), 200);
    }

    #[test]
    fn test_time_quantum_lowest() {
        assert_eq!(PriorityScheduler::time_quantum_for(39), 50);
    }

    #[test]
    fn test_time_quantum_middle() {
        let q = PriorityScheduler::time_quantum_for(20);
        assert!(q > 50 && q < 200);
    }

    #[test]
    fn test_time_quantum_clamped() {
        assert_eq!(PriorityScheduler::time_quantum_for(100), 50);
    }

    #[test]
    fn test_total_ready() {
        let mut sched = PriorityScheduler::new();
        assert_eq!(sched.total_ready(), 0);

        sched.add_process(make_pcb(1, "a", 20));
        assert_eq!(sched.total_ready(), 1);

        sched.add_process(make_pcb(2, "b", 5));
        assert_eq!(sched.total_ready(), 2);
    }

    #[test]
    fn test_is_empty() {
        let mut sched = PriorityScheduler::new();
        assert!(sched.is_empty());

        sched.add_process(make_pcb(1, "a", 20));
        assert!(!sched.is_empty());
    }

    #[test]
    fn test_preemption_scenario() {
        let mut sched = PriorityScheduler::new();
        let low = make_pcb(1, "background", 30);
        sched.add_process(low);

        let scheduled = sched.schedule().unwrap();
        assert_eq!(scheduled.pid, 1);

        // High-priority process arrives.
        sched.add_process(make_pcb(2, "keyboard", 0));
        // Preempted low goes back.
        sched.add_process(scheduled);

        let next = sched.schedule().unwrap();
        assert_eq!(next.pid, 2);
    }
}
