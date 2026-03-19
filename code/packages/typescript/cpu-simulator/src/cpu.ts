/**
 * CPU -- the central processing unit that ties everything together.
 *
 * === What is a CPU? ===
 *
 * The CPU (Central Processing Unit) is the "brain" of a computer. But unlike
 * a human brain, it's extremely simple -- it can only do one thing:
 *
 *     Read an instruction, figure out what it means, do what it says. Repeat.
 *
 * That's it. That's all a CPU does. The power of a computer comes not from
 * the complexity of individual operations (they're trivial -- add two numbers,
 * copy a value, compare two things) but from doing billions of them per second.
 *
 * === CPU components ===
 *
 * A CPU has four main parts:
 *
 *     +------------------------------------------------------+
 *     |                        CPU                           |
 *     |                                                      |
 *     |  +----------+  +----------------+  +-------------+   |
 *     |  | Program  |  |  Register File |  |    ALU      |   |
 *     |  | Counter  |  | R0  R1  R2 ... |  |  (add, sub, |   |
 *     |  |  (PC)    |  | [0] [0] [0]    |  |   and, or)  |   |
 *     |  +----------+  +----------------+  +-------------+   |
 *     |                                                      |
 *     |  +------------------------------------------+        |
 *     |  |  Control Unit (fetch-decode-execute)      |        |
 *     |  +------------------------------------------+        |
 *     |                                                      |
 *     +---------------------+--------------------------------+
 *                           | (memory bus)
 *                           v
 *     +------------------------------------------------------+
 *     |                      Memory                          |
 *     |  [instruction 0] [instruction 1] [data] [data] ...   |
 *     +------------------------------------------------------+
 *
 *   - Program Counter (PC): A special register that holds the address of
 *     the next instruction to execute. It's like a bookmark in a book.
 *
 *   - Register File: A small set of fast storage slots (see registers.ts).
 *
 *   - ALU: The arithmetic/logic unit that does actual computation
 *     (see the arithmetic package).
 *
 *   - Control Unit: The logic that orchestrates the fetch-decode-execute
 *     cycle -- reading instructions, decoding them, and dispatching
 *     operations to the ALU and registers.
 *
 *   - Memory: External storage connected via a "bus" (see memory.ts).
 *
 * === How this module works ===
 *
 * This module provides the CPU shell -- registers, memory, PC, and the
 * pipeline framework. It does NOT know how to decode specific instructions
 * (that's ISA-specific). Instead, it accepts a `decoder` and `executor`
 * that are provided by the ISA simulator (RISC-V, ARM, etc.).
 *
 * This separation means the same CPU can run RISC-V, ARM, WASM, or 4004
 * instructions -- you just plug in a different decoder.
 */

import { Memory } from "./memory.js";
import type {
  DecodeResult,
  ExecuteResult,
  FetchResult,
  PipelineTrace,
} from "./pipeline.js";
import { RegisterFile } from "./registers.js";

// ---------------------------------------------------------------------------
// Decoder and Executor interfaces
// ---------------------------------------------------------------------------
// These define the interface that ISA simulators must implement.
// The CPU calls decode() and execute() -- the ISA provides the implementation.

/**
 * Interface for ISA-specific instruction decoding.
 *
 * The CPU fetches raw bits from memory and passes them to the decoder.
 * The decoder figures out what those bits mean in the context of a
 * specific instruction set (RISC-V, ARM, etc.).
 */
export interface InstructionDecoder {
  decode(rawInstruction: number, pc: number): DecodeResult;
}

/**
 * Interface for ISA-specific instruction execution.
 *
 * The CPU passes the decoded instruction to the executor, along with
 * the register file and memory. The executor performs the operation
 * and returns what changed.
 */
export interface InstructionExecutor {
  execute(
    decoded: DecodeResult,
    registers: RegisterFile,
    memory: Memory,
    pc: number
  ): ExecuteResult;
}

// ---------------------------------------------------------------------------
// CPU State
// ---------------------------------------------------------------------------

/**
 * A snapshot of the entire CPU state at a point in time.
 *
 * This is useful for debugging and visualization -- you can capture
 * the state before and after each instruction to see what changed.
 *
 * Example:
 *     {
 *         pc: 8,
 *         registers: { R0: 0, R1: 1, R2: 2, R3: 3 },
 *         halted: false,
 *         cycle: 2,
 *     }
 */
export interface CPUState {
  pc: number;
  registers: Record<string, number>;
  halted: boolean;
  cycle: number;
}

// ---------------------------------------------------------------------------
// CPU
// ---------------------------------------------------------------------------

/**
 * A generic CPU that executes instructions through a visible pipeline.
 *
 * The CPU doesn't know what instruction set it's running -- that's
 * determined by the decoder and executor you provide. This makes it
 * reusable across RISC-V, ARM, WASM, and Intel 4004.
 *
 * Usage:
 *     1. Create a CPU with a decoder and executor
 *     2. Load a program into memory
 *     3. Call step() to execute one instruction (visible pipeline)
 *     4. Or call run() to execute until halt
 *
 * Example:
 *     const cpu = new CPU(myRiscvDecoder, myRiscvExecutor, 32, 32);
 *     cpu.loadProgram(machineCodeBytes);
 *     const trace = cpu.step();
 *     console.log(formatPipeline(trace));
 *     // --- Cycle 0 ---
 *     //   FETCH              | DECODE             | EXECUTE
 *     //   PC: 0x0000         | addi x1, x0, 1     | x1 = 1
 *     //   -> 0x00100093      | rd=1 rs1=0 imm=1   | PC -> 4
 */
export class CPU {
  /** The CPU's register file -- fast internal storage. */
  registers: RegisterFile;

  /** The CPU's main memory -- program instructions and data live here. */
  memory: Memory;

  /** Program counter -- address of the next instruction to fetch. */
  pc: number = 0;

  /** Whether the CPU has halted (received a halt instruction). */
  halted: boolean = false;

  /** How many instructions have been executed so far. */
  cycle: number = 0;

  /** The ISA-specific instruction decoder. */
  private readonly decoder: InstructionDecoder;

  /** The ISA-specific instruction executor. */
  private readonly executor: InstructionExecutor;

  constructor(
    decoder: InstructionDecoder,
    executor: InstructionExecutor,
    numRegisters: number = 16,
    bitWidth: number = 32,
    memorySize: number = 65536
  ) {
    this.registers = new RegisterFile(numRegisters, bitWidth);
    this.memory = new Memory(memorySize);
    this.decoder = decoder;
    this.executor = executor;
  }

  /**
   * Capture the current CPU state as a snapshot.
   */
  get state(): CPUState {
    return {
      pc: this.pc,
      registers: this.registers.dump(),
      halted: this.halted,
      cycle: this.cycle,
    };
  }

  /**
   * Load machine code bytes into memory.
   *
   * This is how programs get into the computer -- the bytes are copied
   * into memory starting at `startAddress`. The PC is set to point
   * at the first instruction.
   *
   * Example:
   *     cpu.loadProgram([0x93, 0x00, 0x10, 0x00]);  // addi x1, x0, 1
   */
  loadProgram(
    program: number[] | Uint8Array,
    startAddress: number = 0
  ): void {
    this.memory.loadBytes(startAddress, program);
    this.pc = startAddress;
  }

  /**
   * Execute ONE instruction through the full pipeline.
   *
   * This is the core of the CPU -- the fetch-decode-execute cycle
   * made visible. Each call to step() processes one instruction
   * and returns a PipelineTrace showing what happened at each stage.
   *
   * The three stages:
   *
   *     +-----------+    +-----------+    +-----------+
   *     |   FETCH   |--->|  DECODE   |--->|  EXECUTE  |
   *     |           |    |           |    |           |
   *     | Read 4    |    | What does |    | Do the    |
   *     | bytes at  |    | this      |    | operation,|
   *     | PC from   |    | binary    |    | update    |
   *     | memory    |    | mean?     |    | registers |
   *     +-----------+    +-----------+    +-----------+
   *
   * @returns PipelineTrace with fetch, decode, and execute results.
   * @throws Error if the CPU has halted (no more instructions).
   */
  step(): PipelineTrace {
    if (this.halted) {
      throw new Error(
        "CPU has halted -- no more instructions to execute"
      );
    }

    // === STAGE 1: FETCH ===
    // Read 4 bytes from memory at the current PC.
    // These 4 bytes form one 32-bit instruction.
    const rawInstruction = this.memory.readWord(this.pc);
    const fetchResult: FetchResult = {
      pc: this.pc,
      rawInstruction,
    };

    // === STAGE 2: DECODE ===
    // Pass the raw bits to the ISA-specific decoder.
    // The decoder extracts the opcode, register numbers, and immediate values.
    const decodeResult = this.decoder.decode(rawInstruction, this.pc);

    // === STAGE 3: EXECUTE ===
    // Pass the decoded instruction to the ISA-specific executor.
    // The executor reads registers, uses the ALU, writes results back.
    const executeResult = this.executor.execute(
      decodeResult,
      this.registers,
      this.memory,
      this.pc
    );

    // === UPDATE CPU STATE ===
    // After execution, update the PC and check if we should halt.
    this.pc = executeResult.nextPc;
    this.halted = executeResult.halted;

    // Build the complete pipeline trace for this instruction
    const trace: PipelineTrace = {
      cycle: this.cycle,
      fetch: fetchResult,
      decode: decodeResult,
      execute: executeResult,
      registerSnapshot: this.registers.dump(),
    };

    this.cycle += 1;
    return trace;
  }

  /**
   * Run the CPU until it halts or hits the step limit.
   *
   * Returns an array of PipelineTrace objects -- one for each instruction
   * executed. This gives you the complete execution history.
   *
   * Example:
   *     const traces = cpu.run();
   *     for (const trace of traces) {
   *         console.log(formatPipeline(trace));
   *     }
   *
   * @param maxSteps Safety limit to prevent infinite loops.
   * @returns Array of PipelineTrace objects, one per instruction.
   */
  run(maxSteps: number = 10000): PipelineTrace[] {
    const traces: PipelineTrace[] = [];
    for (let i = 0; i < maxSteps; i++) {
      if (this.halted) {
        break;
      }
      traces.push(this.step());
    }
    return traces;
  }
}
