/**
 * # Process Manager — fork, exec, wait, signals, and priority scheduling
 *
 * Every program you run on your computer is a **process** — an instance of a
 * program in execution. When you open a text editor, that is a process. When
 * you run `ls` in the terminal, that is a process. Your operating system might
 * be running hundreds of processes simultaneously.
 *
 * But how are processes *created*? How does the shell run a command? How does
 * a web server handle multiple clients? The answer lies in three elegant Unix
 * system calls: **fork**, **exec**, and **wait**.
 *
 * ## Analogy: The Restaurant Kitchen
 *
 * Think of a restaurant kitchen. The head chef (parent process) can:
 *
 * - **fork():** Clone themselves — now there are two identical chefs with the
 *   same knowledge and state. The clone (child) can do different work.
 * - **exec():** The clone throws away their current recipe book and picks up a
 *   completely different one. They are still the same person (same PID), but
 *   now they are cooking something entirely different.
 * - **wait():** The head chef pauses and watches the clone work. When the clone
 *   finishes and leaves, the head chef resumes.
 *
 * This is exactly how your shell works when you type `ls`:
 *
 * ```
 * Shell (PID 100)
 * |
 * +-- fork() --> creates child (PID 101), exact copy of the shell
 * |   |
 * |   +-- [Child PID 101]: exec("ls")
 * |   |     Replaces shell code with ls code. ls runs, prints files, exits.
 * |   |
 * |   +-- [Parent PID 100]: wait(101)
 * |         Pauses until child exits. Resumes when ls is done.
 * |
 * +-- Shell prompt appears again.
 * ```
 *
 * ## Signals
 *
 * Signals are **software interrupts** sent between processes. They are how Unix
 * processes communicate and control each other. When you press Ctrl+C, the
 * terminal sends SIGINT to the foreground process. When you run `kill <pid>`,
 * the shell sends SIGTERM.
 *
 * Some signals can be caught (the process can define custom behavior), while
 * others like SIGKILL and SIGSTOP are **uncatchable** — the kernel enforces
 * them unconditionally.
 *
 * ## Priority Scheduling
 *
 * Not all processes are equal. A keyboard handler should respond instantly,
 * while a background file indexer can wait. Priority scheduling ensures that
 * important processes get CPU time first. We use the Unix convention where
 * **lower numbers mean higher priority** (priority 0 is the most important,
 * priority 39 is the least).
 *
 * @module
 */

// ============================================================================
// Process State
// ============================================================================

/**
 * ProcessState represents the lifecycle stages of a process.
 *
 * Every process in a Unix system exists in exactly one of these states at any
 * given moment. The transitions between states are triggered by system calls,
 * signals, and scheduler decisions.
 *
 * ```
 * State Transition Diagram:
 *
 *   fork() --> READY --[scheduled]--> RUNNING --[exit/signal]--> ZOMBIE
 *                ^                      |                          |
 *                |                      v                     [wait()]
 *                +---- BLOCKED <--[I/O/wait]                      |
 *                                                              REMOVED
 *
 *   SIGSTOP --> STOPPED --[SIGCONT]--> READY
 * ```
 *
 * - **READY (0):** The process is loaded in memory and waiting for CPU time.
 *   It has everything it needs to run; it is just waiting for the scheduler
 *   to pick it.
 *
 * - **RUNNING (1):** The process is currently executing on the CPU. Only one
 *   process can be RUNNING per CPU core at a time.
 *
 * - **BLOCKED (2):** The process is waiting for something external — a disk
 *   read, network data, or a child process to exit. It cannot run until the
 *   event it is waiting for occurs.
 *
 * - **TERMINATED (3):** The process has finished execution (called exit() or
 *   received a fatal signal). This is a transient state before becoming ZOMBIE.
 *
 * - **ZOMBIE (4):** The process has exited, but its parent has not yet called
 *   wait() to retrieve its exit status. The process control block is kept
 *   around so the parent can read the exit code. Zombies consume no CPU or
 *   memory — only their PCB entry remains.
 */
export enum ProcessState {
  READY = 0,
  RUNNING = 1,
  BLOCKED = 2,
  TERMINATED = 3,
  ZOMBIE = 4,
}

// ============================================================================
// Signal
// ============================================================================

/**
 * Signal represents the standard POSIX signals we implement.
 *
 * Signals are identified by integer numbers. These specific numbers are defined
 * by POSIX and are the same on every Unix-like system (Linux, macOS, FreeBSD).
 * We implement only the six most essential signals:
 *
 * ```
 * Signal Table:
 * +----------+--------+-------------------+-------------+
 * | Name     | Number | Default Action    | Catchable?  |
 * +----------+--------+-------------------+-------------+
 * | SIGINT   |   2    | Terminate         | Yes         |
 * | SIGKILL  |   9    | Terminate         | NO          |
 * | SIGTERM  |  15    | Terminate         | Yes         |
 * | SIGCHLD  |  17    | Ignore            | Yes         |
 * | SIGCONT  |  18    | Continue          | Yes         |
 * | SIGSTOP  |  19    | Stop              | NO          |
 * +----------+--------+-------------------+-------------+
 * ```
 *
 * **Why can't SIGKILL and SIGSTOP be caught?** If a process could ignore
 * SIGKILL, there would be no way to forcibly terminate a misbehaving process.
 * The kernel must always have a "nuclear option." Similarly, SIGSTOP must
 * always work so that debuggers and job control can freeze any process.
 */
export enum Signal {
  /** Interrupt — sent by Ctrl+C. Politely asks the process to stop. */
  SIGINT = 2,

  /** Kill — unconditionally terminates. Cannot be caught or ignored. */
  SIGKILL = 9,

  /** Terminate — polite shutdown request. Default for `kill <pid>`. */
  SIGTERM = 15,

  /** Child status changed — sent to parent when child exits/stops. */
  SIGCHLD = 17,

  /** Continue — resume a stopped process. */
  SIGCONT = 18,

  /** Stop — suspend the process. Cannot be caught or ignored. */
  SIGSTOP = 19,
}

// ============================================================================
// Process Control Block (PCB)
// ============================================================================

/**
 * The ProcessControlBlock is the kernel's data structure for tracking a process.
 *
 * Think of it as the process's "passport" — it contains everything the kernel
 * needs to know: who the process is (PID, name), what it was doing when it
 * last stopped (registers, program counter), its family relationships (parent,
 * children), and any pending mail (signals).
 *
 * When the kernel context-switches away from a process, it saves all the CPU
 * state into the PCB. When it switches back, it restores from the PCB. This
 * is how multiple processes share a single CPU — they take turns, and their
 * state is preserved in between.
 *
 * ## Fields
 *
 * ```
 * PCB Layout:
 * +------------------+---------------------------------------------------+
 * | Field            | Purpose                                           |
 * +------------------+---------------------------------------------------+
 * | pid              | Unique identifier (like a Social Security number) |
 * | name             | Human-readable label (like "bash" or "nginx")     |
 * | state            | Current lifecycle stage (READY, RUNNING, etc.)    |
 * | registers        | Saved CPU registers (x0-x31 in RISC-V)           |
 * | pc               | Program counter — next instruction to execute     |
 * | sp               | Stack pointer — top of the process's stack        |
 * | memory_base      | Start of the process's memory region              |
 * | memory_size      | Size of the process's memory region               |
 * | parent_pid       | Who created this process (via fork)               |
 * | children         | List of child PIDs this process has forked        |
 * | pending_signals  | Signals waiting to be delivered                   |
 * | signal_handlers  | Custom handlers registered for specific signals   |
 * | signal_mask      | Signals currently blocked from delivery           |
 * | priority         | Scheduling priority (0=highest, 39=lowest)        |
 * | cpu_time         | Total CPU cycles consumed (for profiling)         |
 * | exit_code        | Exit status (only meaningful when ZOMBIE)         |
 * +------------------+---------------------------------------------------+
 * ```
 */
export interface ProcessControlBlock {
  pid: number;
  name: string;
  state: ProcessState;

  /**
   * Saved CPU registers. RISC-V has 32 general-purpose registers (x0-x31).
   * When the process is not running, these hold the values the registers had
   * when the process was last preempted or voluntarily yielded the CPU.
   */
  registers: number[];

  /** Program counter — the address of the next instruction to execute. */
  pc: number;

  /** Stack pointer — points to the top of the process's call stack. */
  sp: number;

  /** Base address of the process's memory region. */
  memory_base: number;

  /** Size of the process's memory region in bytes. */
  memory_size: number;

  /** PID of the parent process. PID 0 (init) has parent_pid = 0. */
  parent_pid: number;

  /** PIDs of all child processes created by fork(). */
  children: number[];

  /** Signals that have been sent but not yet delivered. */
  pending_signals: Signal[];

  /**
   * Custom signal handlers. Maps a signal number to the address of the
   * handler function. If a signal is not in this map, the default action
   * is used (usually: terminate the process).
   */
  signal_handlers: Map<Signal, number>;

  /**
   * Signal mask — signals in this set are temporarily blocked from delivery.
   * Note: SIGKILL and SIGSTOP cannot be masked (the kernel ignores attempts
   * to mask them).
   */
  signal_mask: Set<Signal>;

  /**
   * Scheduling priority, 0-39. Lower number = higher priority.
   *
   * - 0: Highest priority (kernel tasks, real-time processes)
   * - 20: Default for user processes
   * - 39: Lowest priority (background/idle tasks)
   *
   * This follows the Unix "nice" value convention. A process can "be nice"
   * by increasing its priority number, giving other processes more CPU time.
   */
  priority: number;

  /** Total CPU cycles consumed by this process across its lifetime. */
  cpu_time: number;

  /**
   * Exit code set when the process terminates. 0 means success, nonzero
   * means error. Only meaningful when state is ZOMBIE.
   */
  exit_code: number;
}

// ============================================================================
// PCB Creation
// ============================================================================

/**
 * Creates a new ProcessControlBlock with sensible defaults.
 *
 * Every new process starts in the READY state, with zeroed registers, no
 * children, no pending signals, default priority (20), and zero CPU time.
 *
 * @param pid - Unique process identifier
 * @param name - Human-readable process name
 * @param parent_pid - PID of the process that created this one (default: 0)
 * @returns A fully initialized PCB
 *
 * @example
 * ```ts
 * const pcb = createPCB(1, "init");
 * // pcb.pid === 1
 * // pcb.state === ProcessState.READY
 * // pcb.priority === 20
 * // pcb.registers.length === 32
 * ```
 */
export function createPCB(
  pid: number,
  name: string,
  parent_pid: number = 0
): ProcessControlBlock {
  return {
    pid,
    name,
    state: ProcessState.READY,
    registers: new Array(32).fill(0),
    pc: 0,
    sp: 0,
    memory_base: 0,
    memory_size: 0,
    parent_pid,
    children: [],
    pending_signals: [],
    signal_handlers: new Map(),
    signal_mask: new Set(),
    priority: 20,
    cpu_time: 0,
    exit_code: 0,
  };
}

// ============================================================================
// Signal Manager
// ============================================================================

/**
 * The SignalManager handles all signal-related operations: sending, delivering,
 * masking, and handling signals.
 *
 * ## Signal Delivery Flow
 *
 * When process A sends a signal to process B, the signal does not take effect
 * immediately. Instead, it goes through several stages:
 *
 * ```
 * 1. send_signal(B, SIGTERM)
 *    --> SIGTERM is added to B's pending_signals list
 *
 * 2. When B is next scheduled:
 *    --> deliver_pending(B) is called
 *    --> Kernel checks: is SIGTERM masked? If yes, skip.
 *    --> Kernel checks: does B have a handler for SIGTERM?
 *        --> YES: redirect PC to handler address
 *        --> NO: apply default action (terminate)
 * ```
 *
 * This two-phase approach (enqueue then deliver) means signals are not
 * processed during critical sections of kernel code — only at safe points
 * like context switch boundaries.
 */
export class SignalManager {
  /**
   * Sends a signal to a process by adding it to the pending list.
   *
   * This does NOT immediately act on the signal. The signal sits in the
   * pending list until deliver_pending() is called (typically at the next
   * context switch).
   *
   * **Exception:** SIGKILL and SIGSTOP are never added to pending — they
   * are handled immediately because they cannot be caught or blocked.
   *
   * @param pcb - The target process
   * @param signal - The signal to send
   * @returns true if the signal was enqueued, false if it was handled immediately
   */
  send_signal(pcb: ProcessControlBlock, signal: Signal): boolean {
    // -----------------------------------------------------------------------
    // SIGKILL: The nuclear option. Immediately terminates the process.
    // No handler, no mask, no escape. This is the kernel's guarantee that
    // any process can be killed — even one stuck in an infinite loop.
    // -----------------------------------------------------------------------
    if (signal === Signal.SIGKILL) {
      pcb.state = ProcessState.ZOMBIE;
      pcb.exit_code = 128 + Signal.SIGKILL; // Convention: 128 + signal number
      return false;
    }

    // -----------------------------------------------------------------------
    // SIGSTOP: Freeze the process. Like SIGKILL, it cannot be caught.
    // Used by debuggers (gdb sends SIGSTOP to pause a process) and by
    // job control (Ctrl+Z in the shell).
    // -----------------------------------------------------------------------
    if (signal === Signal.SIGSTOP) {
      pcb.state = ProcessState.BLOCKED;
      return false;
    }

    // -----------------------------------------------------------------------
    // SIGCONT: Resume a stopped process. Even if the process has a handler
    // for SIGCONT, the resume happens unconditionally first.
    // -----------------------------------------------------------------------
    if (signal === Signal.SIGCONT) {
      if (pcb.state === ProcessState.BLOCKED) {
        pcb.state = ProcessState.READY;
      }
      return false;
    }

    // -----------------------------------------------------------------------
    // All other signals: add to pending list for deferred delivery.
    // -----------------------------------------------------------------------
    pcb.pending_signals.push(signal);
    return true;
  }

  /**
   * Delivers all pending signals to a process.
   *
   * Called by the scheduler just before a process resumes execution. Each
   * pending signal is checked against the signal mask and handler table:
   *
   * 1. If the signal is masked, it stays pending (skipped for now).
   * 2. If the process has a custom handler, we record the handler address
   *    (the kernel would redirect PC to this address).
   * 3. If no handler exists, the default action is applied.
   *
   * @param pcb - The process to deliver signals to
   * @returns Array of handler addresses that were invoked, or "default"
   *          for signals that used the default action
   */
  deliver_pending(
    pcb: ProcessControlBlock
  ): Array<{ signal: Signal; action: number | "default" }> {
    const delivered: Array<{ signal: Signal; action: number | "default" }> = [];
    const still_pending: Signal[] = [];

    for (const signal of pcb.pending_signals) {
      // ---------------------------------------------------------------------
      // Masked signals stay in the pending queue. They will be delivered
      // later, when the process unmasks them.
      // Note: SIGKILL and SIGSTOP can never be masked (enforced by mask()).
      // ---------------------------------------------------------------------
      if (pcb.signal_mask.has(signal)) {
        still_pending.push(signal);
        continue;
      }

      // ---------------------------------------------------------------------
      // Check for a custom handler.
      // ---------------------------------------------------------------------
      const handler = pcb.signal_handlers.get(signal);
      if (handler !== undefined) {
        delivered.push({ signal, action: handler });
      } else {
        // -------------------------------------------------------------------
        // No handler — apply the default action.
        // For most signals, the default is to terminate the process.
        // SIGCHLD is special: its default action is to ignore.
        // -------------------------------------------------------------------
        if (this.is_fatal(signal)) {
          pcb.state = ProcessState.ZOMBIE;
          pcb.exit_code = 128 + signal;
        }
        delivered.push({ signal, action: "default" });
      }
    }

    pcb.pending_signals = still_pending;
    return delivered;
  }

  /**
   * Registers a custom signal handler for a process.
   *
   * When the signal is delivered, instead of the default action (usually
   * terminate), the kernel will redirect execution to the handler address.
   * This is how programs implement graceful shutdown, cleanup, etc.
   *
   * **Restriction:** SIGKILL and SIGSTOP cannot have custom handlers.
   * The kernel silently ignores attempts to register handlers for them.
   *
   * @param pcb - The process registering the handler
   * @param signal - Which signal to handle
   * @param handler_addr - Address of the handler function in the process's memory
   * @returns true if the handler was registered, false if the signal is uncatchable
   */
  register_handler(
    pcb: ProcessControlBlock,
    signal: Signal,
    handler_addr: number
  ): boolean {
    // SIGKILL and SIGSTOP are the kernel's "override" signals — they must
    // always work regardless of what the process wants.
    if (signal === Signal.SIGKILL || signal === Signal.SIGSTOP) {
      return false;
    }
    pcb.signal_handlers.set(signal, handler_addr);
    return true;
  }

  /**
   * Adds a signal to the process's signal mask, blocking it from delivery.
   *
   * Masked signals accumulate in the pending queue but are not delivered
   * until unmasked. This is used to protect critical sections of code
   * from being interrupted.
   *
   * @param pcb - The process
   * @param signal - The signal to mask
   * @returns true if the signal was masked, false if it cannot be masked
   */
  mask(pcb: ProcessControlBlock, signal: Signal): boolean {
    // SIGKILL and SIGSTOP cannot be masked — the kernel must always be
    // able to kill or stop any process.
    if (signal === Signal.SIGKILL || signal === Signal.SIGSTOP) {
      return false;
    }
    pcb.signal_mask.add(signal);
    return true;
  }

  /**
   * Removes a signal from the process's signal mask, allowing delivery.
   *
   * Any pending instances of this signal will be delivered at the next
   * call to deliver_pending().
   *
   * @param pcb - The process
   * @param signal - The signal to unmask
   */
  unmask(pcb: ProcessControlBlock, signal: Signal): void {
    pcb.signal_mask.delete(signal);
  }

  /**
   * Determines whether a signal's default action is fatal (terminates the
   * process).
   *
   * Most signals are fatal by default. The notable exception is SIGCHLD,
   * which is ignored by default — the parent only needs to handle it if
   * it wants to be notified when children exit.
   *
   * @param signal - The signal to check
   * @returns true if the default action would terminate the process
   */
  is_fatal(signal: Signal): boolean {
    // SIGCHLD's default action is "ignore" — the parent is not forced to
    // handle it. All other signals we implement default to termination.
    // SIGCONT's default is "continue" (not fatal).
    if (signal === Signal.SIGCHLD || signal === Signal.SIGCONT) {
      return false;
    }
    return true;
  }
}

// ============================================================================
// Process Manager
// ============================================================================

/**
 * The ProcessManager implements the core process lifecycle operations:
 * create, fork, exec, wait, kill, and exit.
 *
 * It maintains a process table (a map from PID to PCB) and a monotonically
 * increasing PID counter. PID 0 is reserved for the init process, which is
 * the ancestor of all other processes.
 *
 * ## Process Table
 *
 * The process table is the kernel's registry of all processes in the system.
 * Every operation that creates, modifies, or destroys a process goes through
 * the process table.
 *
 * ```
 * Process Table:
 * +-----+--------+---------+-----------+
 * | PID | Name   | State   | Parent    |
 * +-----+--------+---------+-----------+
 * |  0  | init   | RUNNING | 0 (self)  |
 * |  1  | shell  | READY   | 0         |
 * |  2  | editor | BLOCKED | 1         |
 * |  3  | ls     | ZOMBIE  | 1         |
 * +-----+--------+---------+-----------+
 * ```
 */
export class ProcessManager {
  /** Map from PID to ProcessControlBlock. */
  private process_table: Map<number, ProcessControlBlock> = new Map();

  /** Next PID to assign. Starts at 0 for init, then increments. */
  private next_pid: number = 0;

  /** Signal manager for handling signal operations. */
  private signal_manager: SignalManager = new SignalManager();

  /**
   * Returns the process table for inspection (useful for testing).
   */
  get_process_table(): Map<number, ProcessControlBlock> {
    return this.process_table;
  }

  /**
   * Returns the signal manager instance.
   */
  get_signal_manager(): SignalManager {
    return this.signal_manager;
  }

  /**
   * Creates a new process and adds it to the process table.
   *
   * This is the low-level process creation function. It allocates a new PID,
   * creates a PCB, and adds it to the process table. Unlike fork(), this
   * creates a process from scratch rather than cloning an existing one.
   *
   * @param name - Human-readable name for the process
   * @param parent_pid - PID of the parent process (default: 0 for init)
   * @returns The newly created PCB
   *
   * @example
   * ```ts
   * const pm = new ProcessManager();
   * const init = pm.create_process("init");     // PID 0
   * const shell = pm.create_process("shell", 0); // PID 1, child of init
   * ```
   */
  create_process(name: string, parent_pid: number = 0): ProcessControlBlock {
    const pid = this.next_pid++;
    const pcb = createPCB(pid, name, parent_pid);
    this.process_table.set(pid, pcb);

    // If this process has a parent, add it to the parent's children list.
    const parent = this.process_table.get(parent_pid);
    if (parent && parent.pid !== pid) {
      parent.children.push(pid);
    }

    return pcb;
  }

  /**
   * Forks a process — creates an (almost) exact copy with a new PID.
   *
   * fork() is the Unix mechanism for creating new processes. The child is a
   * clone of the parent: same registers, same program counter, same memory
   * contents, same signal handlers. The only differences are:
   *
   * - The child gets a new, unique PID.
   * - The child's parent_pid points to the original process.
   * - The child starts with an empty children list.
   * - The child starts with no pending signals.
   * - The child's cpu_time resets to 0.
   * - The return value differs: parent gets child's PID, child gets 0.
   *
   * This last point is the key insight. After fork(), both processes resume
   * at the same instruction. The ONLY way they can tell themselves apart is
   * by checking the return value:
   *
   * ```
   * result = fork()
   * if result == 0:
   *     "I am the child"
   * else:
   *     "I am the parent, child's PID is {result}"
   * ```
   *
   * @param parent_pid - PID of the process to fork
   * @returns Object with parent_result (child's PID) and child_pid, or null if parent not found
   */
  fork(
    parent_pid: number
  ): { parent_result: number; child_pid: number } | null {
    const parent = this.process_table.get(parent_pid);
    if (!parent) {
      return null;
    }

    // Step 1: Allocate a new PID for the child.
    const child_pid = this.next_pid++;

    // Step 2: Create the child PCB as a copy of the parent.
    const child: ProcessControlBlock = {
      pid: child_pid,
      name: parent.name,
      state: ProcessState.READY,
      // Copy registers — the child resumes with the same register state.
      // (In real Unix, register a0 would differ: child gets 0.)
      registers: [...parent.registers],
      pc: parent.pc,
      sp: parent.sp,
      memory_base: parent.memory_base,
      memory_size: parent.memory_size,
      parent_pid: parent.pid,
      // Child starts with no children of its own.
      children: [],
      // No pending signals — child starts clean.
      pending_signals: [],
      // Signal handlers are inherited — the child has the same custom
      // handlers as the parent.
      signal_handlers: new Map(parent.signal_handlers),
      // Signal mask is inherited too.
      signal_mask: new Set(parent.signal_mask),
      // Same priority as parent.
      priority: parent.priority,
      // Fresh CPU time counter.
      cpu_time: 0,
      exit_code: 0,
    };

    // Step 3: Set the fork return values in the register file.
    // Convention: register 10 (a0 in RISC-V) holds the return value.
    // Parent sees the child's PID; child sees 0.
    parent.registers[10] = child_pid;
    child.registers[10] = 0;

    // Step 4: Update parent's children list.
    parent.children.push(child_pid);

    // Step 5: Add child to the process table.
    this.process_table.set(child_pid, child);

    return { parent_result: child_pid, child_pid };
  }

  /**
   * Replaces a process's program with a new one.
   *
   * exec() is the complement to fork(). While fork() creates a copy, exec()
   * transforms a process into something completely different. The PID stays
   * the same (the process is still "the same person"), but:
   *
   * - All registers are zeroed (fresh start).
   * - The program counter is set to the new entry point.
   * - The stack pointer is set to the new stack top.
   * - Signal handlers are cleared (the new program knows nothing about the
   *   old program's handlers).
   * - Pending signals are cleared.
   *
   * Things that do NOT change:
   * - PID (the process keeps its identity)
   * - Parent PID (same parent)
   * - Children list (children survive exec)
   * - Priority (inherited from before)
   * - CPU time (continues accumulating)
   *
   * @param pid - PID of the process to transform
   * @param entry_point - Address of the first instruction in the new program
   * @param stack_top - Address of the top of the new stack
   * @returns true if exec succeeded, false if the PID was not found
   */
  exec(pid: number, entry_point: number, stack_top: number): boolean {
    const pcb = this.process_table.get(pid);
    if (!pcb) {
      return false;
    }

    // Step 1: Zero all registers — the new program starts fresh.
    pcb.registers = new Array(32).fill(0);

    // Step 2: Set the program counter to the entry point of the new program.
    pcb.pc = entry_point;

    // Step 3: Set the stack pointer. The stack grows downward in memory,
    // so the initial SP points to the top of the allocated stack region.
    pcb.sp = stack_top;

    // Step 4: Clear signal handlers — the new program does not inherit the
    // old program's signal handling code (which no longer exists in memory).
    pcb.signal_handlers = new Map();

    // Step 5: Clear pending signals — start clean.
    pcb.pending_signals = [];

    // Note: PID, parent_pid, children, priority, and cpu_time are preserved.
    // The process keeps its identity and relationships through exec().

    return true;
  }

  /**
   * Waits for a child process to exit and reaps its zombie.
   *
   * When a process calls wait(), it is saying: "I want to know when one of
   * my children finishes." If a child has already exited (is in ZOMBIE state),
   * wait() immediately returns its PID and exit code. If no children are
   * zombies, wait() returns null (in a real OS, the parent would block).
   *
   * **Reaping:** When wait() finds a zombie child, it removes the child's
   * PCB from the process table entirely. This is called "reaping" — the
   * zombie is finally laid to rest.
   *
   * @param parent_pid - PID of the waiting parent
   * @returns Object with child_pid and exit_code, or null if no zombie children
   */
  wait(
    parent_pid: number
  ): { child_pid: number; exit_code: number } | null {
    const parent = this.process_table.get(parent_pid);
    if (!parent) {
      return null;
    }

    // Search through the parent's children for any zombie.
    for (let i = 0; i < parent.children.length; i++) {
      const child_pid = parent.children[i];
      const child = this.process_table.get(child_pid);

      if (child && child.state === ProcessState.ZOMBIE) {
        const exit_code = child.exit_code;

        // Reap the zombie: remove from parent's children list.
        parent.children.splice(i, 1);

        // Remove the zombie's PCB from the process table.
        this.process_table.delete(child_pid);

        return { child_pid, exit_code };
      }
    }

    // No zombie children found. In a real OS, the parent would block here
    // until a child exits. In our simulation, we return null.
    return null;
  }

  /**
   * Sends a signal to a process.
   *
   * Despite its name, kill() does not necessarily kill the target. It sends
   * a signal, which the target might catch and handle gracefully. Only
   * SIGKILL guarantees termination.
   *
   * @param target_pid - PID of the process to signal
   * @param signal - The signal to send
   * @returns true if the signal was sent, false if the target does not exist
   */
  kill(target_pid: number, signal: Signal): boolean {
    const target = this.process_table.get(target_pid);
    if (!target) {
      return false;
    }

    this.signal_manager.send_signal(target, signal);

    // If the signal caused the process to become a zombie, send SIGCHLD
    // to its parent so the parent knows a child has exited.
    if (target.state === ProcessState.ZOMBIE) {
      const parent = this.process_table.get(target.parent_pid);
      if (parent) {
        this.signal_manager.send_signal(parent, Signal.SIGCHLD);
      }
    }

    return true;
  }

  /**
   * Terminates a process, making it a zombie.
   *
   * When a process calls exit(), several things happen:
   *
   * 1. The process's state is set to ZOMBIE. It is dead, but its PCB
   *    remains so the parent can retrieve the exit code via wait().
   *
   * 2. All of the process's children are "reparented" to init (PID 0).
   *    Orphaned children need a parent, and init is the universal adoptive
   *    parent in Unix. Init periodically calls wait() to reap orphaned
   *    zombies.
   *
   * 3. SIGCHLD is sent to the process's parent, notifying it that a child
   *    has exited.
   *
   * @param pid - PID of the process to terminate
   * @param exit_code - Exit status (0 = success, nonzero = error)
   * @returns true if the process was terminated, false if not found
   */
  exit_process(pid: number, exit_code: number): boolean {
    const pcb = this.process_table.get(pid);
    if (!pcb) {
      return false;
    }

    // Step 1: Set the process to ZOMBIE state with the given exit code.
    pcb.state = ProcessState.ZOMBIE;
    pcb.exit_code = exit_code;

    // Step 2: Reparent all children to init (PID 0).
    // In Unix, when a parent dies, its children become orphans. The init
    // process adopts them. This prevents zombie accumulation — init always
    // reaps its children.
    const init = this.process_table.get(0);
    for (const child_pid of pcb.children) {
      const child = this.process_table.get(child_pid);
      if (child) {
        child.parent_pid = 0;
        if (init && init.pid !== pcb.pid) {
          init.children.push(child_pid);
        }
      }
    }
    pcb.children = [];

    // Step 3: Send SIGCHLD to the parent.
    const parent = this.process_table.get(pcb.parent_pid);
    if (parent) {
      this.signal_manager.send_signal(parent, Signal.SIGCHLD);
    }

    return true;
  }

  /**
   * Looks up a process by PID.
   *
   * @param pid - The PID to look up
   * @returns The PCB, or undefined if not found
   */
  get_process(pid: number): ProcessControlBlock | undefined {
    return this.process_table.get(pid);
  }
}

// ============================================================================
// Priority Scheduler
// ============================================================================

/**
 * The PriorityScheduler replaces simple round-robin scheduling with
 * priority-based scheduling.
 *
 * ## How It Works
 *
 * The scheduler maintains 40 queues, one for each priority level (0-39).
 * When it needs to pick the next process to run, it scans from priority 0
 * (highest) to priority 39 (lowest), returning the first process it finds.
 *
 * Within the same priority level, processes are served in FIFO order
 * (round-robin). This means:
 * - A priority-5 process ALWAYS runs before a priority-20 process.
 * - Two priority-20 processes take turns (round-robin).
 *
 * ```
 * Ready Queues:
 *
 * Priority 0:  [kernel_timer]        <-- runs first
 * Priority 1:  []
 * Priority 2:  []
 * ...
 * Priority 5:  [keyboard_handler]    <-- runs second
 * ...
 * Priority 20: [bash, vim, firefox]  <-- round-robin among these
 * ...
 * Priority 39: [backup_daemon]       <-- runs last (if ever)
 * ```
 *
 * ## Time Quantum
 *
 * Higher-priority processes get longer time slices (more CPU cycles before
 * being preempted). This further ensures that important processes can do
 * meaningful work without being interrupted too frequently.
 *
 * ```
 * Priority   Time Quantum (cycles)
 * --------   ---------------------
 *    0            200
 *   20            100
 *   39             50
 * ```
 *
 * Formula: quantum = 200 - (priority * (150 / 39))
 *
 * ## Starvation Warning
 *
 * This scheduler can starve low-priority processes if high-priority ones
 * never block. Real schedulers (like Linux CFS) use "aging" to prevent
 * this — gradually boosting the priority of starved processes. We do not
 * implement aging here, but note it as a real-world consideration.
 */
export class PriorityScheduler {
  /**
   * 40 ready queues, one per priority level. Each queue is a FIFO list of
   * ProcessControlBlocks.
   */
  private ready_queues: ProcessControlBlock[][] = Array.from(
    { length: 40 },
    () => []
  );

  /** The currently running process, or null if idle. */
  private current_process: ProcessControlBlock | null = null;

  /**
   * Adds a process to the appropriate ready queue based on its priority.
   *
   * The process is placed at the END of its priority queue (FIFO order).
   * This ensures round-robin behavior within the same priority level.
   *
   * @param pcb - The process to enqueue
   * @throws Error if the priority is out of range (0-39)
   */
  enqueue(pcb: ProcessControlBlock): void {
    const priority = pcb.priority;
    if (priority < 0 || priority > 39) {
      throw new Error(
        `Priority ${priority} is out of range (0-39). ` +
          `Priority 0 is highest, 39 is lowest.`
      );
    }
    pcb.state = ProcessState.READY;
    this.ready_queues[priority].push(pcb);
  }

  /**
   * Selects the highest-priority process to run next.
   *
   * Scans from priority 0 (highest) to 39 (lowest), returning the first
   * process found. Within the same priority, takes the front of the queue
   * (FIFO/round-robin).
   *
   * @returns The next process to run, or null if all queues are empty
   */
  schedule(): ProcessControlBlock | null {
    for (let priority = 0; priority < 40; priority++) {
      if (this.ready_queues[priority].length > 0) {
        const next = this.ready_queues[priority].shift()!;
        next.state = ProcessState.RUNNING;
        this.current_process = next;
        return next;
      }
    }
    // All queues are empty — nothing to run.
    this.current_process = null;
    return null;
  }

  /**
   * Preempts the currently running process, putting it back in its queue.
   *
   * Called by the timer interrupt handler when a process's time quantum
   * expires. The process goes to the END of its priority queue, giving
   * other processes at the same priority level a chance to run.
   *
   * @param pcb - The process being preempted
   */
  preempt(pcb: ProcessControlBlock): void {
    pcb.state = ProcessState.READY;
    this.enqueue(pcb);
  }

  /**
   * Changes a process's priority.
   *
   * If the process is currently in a ready queue, it is moved to the
   * correct queue for its new priority. This is used by the kernel to
   * boost priorities of starved processes or to implement the `nice`
   * command.
   *
   * @param pcb - The process to re-prioritize
   * @param new_priority - New priority value (0-39)
   * @throws Error if the priority is out of range
   */
  set_priority(pcb: ProcessControlBlock, new_priority: number): void {
    if (new_priority < 0 || new_priority > 39) {
      throw new Error(
        `Priority ${new_priority} is out of range (0-39).`
      );
    }

    const old_priority = pcb.priority;

    // Remove from old queue if present.
    if (old_priority !== new_priority) {
      const old_queue = this.ready_queues[old_priority];
      const idx = old_queue.indexOf(pcb);
      if (idx !== -1) {
        old_queue.splice(idx, 1);
        pcb.priority = new_priority;
        this.ready_queues[new_priority].push(pcb);
      } else {
        // Process is not in any queue (maybe running or blocked).
        // Just update the priority field.
        pcb.priority = new_priority;
      }
    }
  }

  /**
   * Calculates the time quantum (in CPU cycles) for a given priority.
   *
   * Higher-priority processes get more cycles before being preempted.
   * This makes sense intuitively: a real-time audio process (priority 0)
   * needs to process audio buffers without interruption, while a background
   * backup job (priority 39) can afford to be interrupted frequently.
   *
   * Formula: quantum = 200 - floor(priority * 150 / 39)
   *
   * ```
   * Examples:
   *   Priority  0 --> 200 cycles
   *   Priority 20 --> 123 cycles
   *   Priority 39 -->  50 cycles
   * ```
   *
   * @param priority - The priority level (0-39)
   * @returns Number of CPU cycles the process gets before preemption
   */
  get_time_quantum(priority: number): number {
    if (priority < 0 || priority > 39) {
      throw new Error(`Priority ${priority} is out of range (0-39).`);
    }
    return 200 - Math.floor((priority * 150) / 39);
  }

  /**
   * Returns the currently running process.
   */
  get_current(): ProcessControlBlock | null {
    return this.current_process;
  }

  /**
   * Returns the ready queues for inspection (useful for testing).
   */
  get_ready_queues(): ProcessControlBlock[][] {
    return this.ready_queues;
  }
}
