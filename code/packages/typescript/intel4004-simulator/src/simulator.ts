/**
 * Intel 4004 Simulator -- the world's first commercial microprocessor.
 *
 * === What is the Intel 4004? ===
 *
 * The Intel 4004 was the world's first commercial single-chip microprocessor,
 * released by Intel in 1971. It was designed by Federico Faggin, Ted Hoff, and
 * Stanley Mazor for the Busicom 141-PF calculator -- a Japanese desktop printing
 * calculator. Intel negotiated to retain the rights to the chip design, which
 * turned out to be one of the most consequential business decisions in history.
 *
 * The entire processor contained just 2,300 transistors. For perspective, a
 * modern CPU has billions. The 4004 ran at 740 kHz -- about a million times
 * slower than today's processors. Yet it proved that a general-purpose processor
 * could be built on a single chip, launching the microprocessor revolution.
 *
 * === Why 4-bit? ===
 *
 * The 4004 is a 4-bit processor. Every data value is 4 bits wide (0-15). This
 * seems tiny, but it was perfect for its intended purpose: calculators. A single
 * decimal digit (0-9) fits in 4 bits, which is exactly what Binary-Coded Decimal
 * (BCD) arithmetic needs. The Busicom calculator used BCD throughout, so 4 bits
 * was the natural data width.
 *
 * All values in this simulator are masked to 4 bits (& 0xF). This is the
 * fundamental constraint of the architecture -- there are no 8-bit, 16-bit, or
 * 32-bit values anywhere in the data path.
 *
 * === Accumulator architecture ===
 *
 * The 4004 uses an accumulator architecture. This means almost every arithmetic
 * operation works through a single special register called the Accumulator (A):
 *
 *     - To add two numbers: load one into A, store it in a register, load the
 *       other into A, then add the register to A. The result is in A.
 *     - There is no "add register to register" instruction.
 *
 * This is very different from other architectures:
 *
 *     RISC-V (register-register):  add x3, x1, x2     Any register to any register.
 *     WASM (stack-based):          i32.add              Pops two, pushes result.
 *     Intel 4004 (accumulator):    ADD R0               A = A + R0. Always uses A.
 *
 * The accumulator pattern means more instructions to do the same work, but
 * simpler hardware -- which mattered enormously in 1971 when every transistor
 * was precious.
 *
 * === Registers ===
 *
 *     Accumulator (A):  4 bits. The center of all computation.
 *     R0-R15:           16 general registers, each 4 bits.
 *     Carry flag:       1 bit. Set on arithmetic overflow/borrow.
 *     PC:               12-bit program counter (addresses 4096 bytes of ROM).
 *     Stack:            3-level hardware stack (12-bit return addresses).
 *
 * === Memory ===
 *
 *     ROM:  4096 x 8-bit bytes of program storage.
 *     RAM:  4 banks x 4 registers x (16 main + 4 status) nibbles.
 *
 * === Complete Instruction Set (46 instructions) ===
 *
 *     0x00       NOP          No operation
 *     0x01       HLT          Halt (simulator-only)
 *     0x1_       JCN c,a  *   Conditional jump (c=condition nibble)
 *     0x2_ even  FIM Pp,d *   Fetch immediate to register pair
 *     0x2_ odd   SRC Pp       Send register control (pair as address)
 *     0x3_ even  FIN Pp       Fetch indirect from ROM via P0
 *     0x3_ odd   JIN Pp       Jump indirect via register pair
 *     0x4_       JUN a    *   Unconditional jump (12-bit address)
 *     0x5_       JMS a    *   Jump to subroutine
 *     0x6_       INC Rn       Increment register
 *     0x7_       ISZ Rn,a *   Increment and skip if zero
 *     0x8_       ADD Rn       Add register to accumulator
 *     0x9_       SUB Rn       Subtract register from accumulator
 *     0xA_       LD Rn        Load register into accumulator
 *     0xB_       XCH Rn       Exchange accumulator and register
 *     0xC_       BBL n        Branch back and load
 *     0xD_       LDM n        Load immediate into accumulator
 *     0xE0-0xEF  I/O ops      RAM/ROM read/write operations
 *     0xF0-0xFD  Accum ops    Accumulator manipulation
 *
 *     * = 2-byte instruction (second byte is data or address)
 *
 * === Instruction encoding ===
 *
 * Most instructions are 8 bits (1 byte). Some are 2 bytes (JCN, FIM, JUN,
 * JMS, ISZ). The upper nibble is the opcode family, and the lower nibble
 * is the operand (a register number, immediate value, or condition code):
 *
 *     +----------+----------+
 *     |  opcode  | operand  |
 *     |  bits 7-4| bits 3-0 |
 *     +----------+----------+
 *
 * For 2-byte instructions, the second byte provides additional data:
 * an 8-bit address component, immediate data, or jump target.
 */

import {
  ExecutionResult,
  StepTrace,
} from "@coding-adventures/simulator-protocol";

import type { Intel4004State } from "./state.js";

// ---------------------------------------------------------------------------
// Trace -- what happened during one instruction
// ---------------------------------------------------------------------------
// Every step() call returns one of these, giving a complete picture of what
// the instruction did. This is the 4004 equivalent of RISC-V's PipelineTrace,
// but simpler -- no pipeline stages, just fetch-decode-execute in one cycle.

/**
 * Record of a single instruction execution.
 *
 * Fields:
 *     address:            PC where this instruction was fetched from.
 *     raw:                The raw first byte (0x00-0xFF).
 *     raw2:               The raw second byte for 2-byte instructions, else undefined.
 *     mnemonic:           Human-readable instruction (e.g., "LDM 1", "ADD R0").
 *     accumulatorBefore:  Value of A before execution.
 *     accumulatorAfter:   Value of A after execution.
 *     carryBefore:        Carry flag before execution.
 *     carryAfter:         Carry flag after execution.
 */
export interface Intel4004Trace {
  address: number;
  raw: number;
  raw2?: number;
  mnemonic: string;
  accumulatorBefore: number;
  accumulatorAfter: number;
  carryBefore: boolean;
  carryAfter: boolean;
}

// ---------------------------------------------------------------------------
// Helper: detect 2-byte instructions
// ---------------------------------------------------------------------------

/**
 * Return true if the raw byte starts a 2-byte instruction.
 *
 * The 4004 has five 2-byte instruction families:
 *     0x1_ JCN  -- conditional jump
 *     0x2_ FIM  -- fetch immediate (even lower nibble only)
 *     0x4_ JUN  -- unconditional jump
 *     0x5_ JMS  -- jump to subroutine
 *     0x7_ ISZ  -- increment and skip if zero
 */
function isTwoByte(raw: number): boolean {
  const upper = (raw >> 4) & 0xf;
  if (upper === 0x1 || upper === 0x4 || upper === 0x5 || upper === 0x7) {
    return true;
  }
  // FIM is 0x2_ with even lower nibble
  return upper === 0x2 && (raw & 0x1) === 0;
}

function freezeNumbers(values: readonly number[]): readonly number[] {
  return Object.freeze([...values]);
}

function freezeRam(
  ram: readonly (readonly (readonly number[])[])[]
): readonly (readonly (readonly number[])[])[] {
  return Object.freeze(
    ram.map((bank) =>
      Object.freeze(
        bank.map((register) => Object.freeze([...register]))
      )
    )
  );
}

// ---------------------------------------------------------------------------
// The simulator
// ---------------------------------------------------------------------------

/**
 * A simulator for the complete Intel 4004 microprocessor instruction set.
 *
 * This is a standalone implementation -- the 4004's accumulator architecture
 * is too different from register-register machines (like RISC-V) to share
 * a generic CPU base class. The 4-bit data width, single accumulator, and
 * carry flag are all unique to this style of machine.
 *
 * Usage:
 *     const sim = new Intel4004Simulator();
 *     const program = new Uint8Array([0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01]);
 *     const traces = sim.run(program);
 *     sim.registers[1]; // => 3
 *
 * State:
 *     accumulator:    4-bit accumulator (0-15). The heart of computation.
 *     registers:      16 general-purpose 4-bit registers (R0-R15).
 *     carry:          Carry/borrow flag from the last arithmetic operation.
 *     memory:         ROM holding the program bytes (4096 bytes).
 *     pc:             12-bit program counter -- index into memory.
 *     halted:         True after HLT is executed.
 *     hwStack:        3-level hardware call stack (12-bit return addresses).
 *     stackPointer:   Current stack position (0-2), wraps mod 3.
 *     ram:            4 banks x 4 registers x 16 nibbles of main memory.
 *     ramStatus:      4 banks x 4 registers x 4 status nibbles.
 *     ramOutput:      4-element output port array (one per bank).
 *     ramBank:        Currently selected RAM bank (set by DCL).
 *     ramRegister:    Currently selected RAM register (set by SRC).
 *     ramCharacter:   Currently selected RAM character (set by SRC).
 *     romPort:        ROM I/O port value.
 */
export class Intel4004Simulator {
  // --- Registers ---
  // The accumulator is where all arithmetic happens.
  // It's 4 bits, so values are always 0-15.
  accumulator: number = 0;

  // 16 general-purpose registers, each 4 bits.
  // These hold intermediate values -- you swap them in and out of A
  // using XCH to do multi-step computations.
  registers: number[];

  // --- Flags ---
  // The carry flag is set when an ADD overflows past 15, or when a
  // SUB does NOT borrow (complement-add result > 15).
  // In the 4004, carry=true after SUB means NO borrow occurred.
  carry: boolean = false;

  // --- Memory ---
  // The 4004 had separate ROM (program) and RAM (data) address spaces.
  // ROM: 4096 bytes of program storage.
  memory: Uint8Array;

  // --- RAM: 4 banks x 4 registers x (16 main + 4 status) nibbles ---
  // ram[bank][register][character] for main characters (0-15)
  ram: number[][][];

  // ramStatus[bank][register][statusIndex] for status characters (0-3)
  ramStatus: number[][][];

  // RAM output port (written by WMP), one per bank
  ramOutput: number[];

  // --- RAM addressing (set by SRC and DCL) ---
  ramBank: number = 0;
  ramRegister: number = 0;
  ramCharacter: number = 0;

  // --- ROM I/O port ---
  romPort: number = 0;

  // --- Hardware call stack ---
  // The 4004 has a 3-level hardware stack for subroutine calls.
  // There is no stack overflow exception -- the 4th push silently
  // overwrites the oldest entry (wrap mod 3).
  hwStack: number[];
  stackPointer: number = 0;

  // --- Control ---
  pc: number = 0;
  halted: boolean = false;

  constructor(memorySize: number = 4096) {
    this.registers = new Array(16).fill(0);
    this.memory = new Uint8Array(memorySize);
    this.ram = this._makeRam();
    this.ramStatus = this._makeRamStatus();
    this.ramOutput = [0, 0, 0, 0];
    this.hwStack = [0, 0, 0];
  }

  // -----------------------------------------------------------------
  // RAM initialization helpers
  // -----------------------------------------------------------------

  /** Create a fresh 4 x 4 x 16 RAM array (4 banks, 4 registers, 16 nibbles). */
  private _makeRam(): number[][][] {
    return Array.from({ length: 4 }, () =>
      Array.from({ length: 4 }, () => new Array(16).fill(0))
    );
  }

  /** Create a fresh 4 x 4 x 4 RAM status array. */
  private _makeRamStatus(): number[][][] {
    return Array.from({ length: 4 }, () =>
      Array.from({ length: 4 }, () => new Array(4).fill(0))
    );
  }

  // -----------------------------------------------------------------
  // Register pair helpers
  // -----------------------------------------------------------------
  // The 4004 organizes its 16 registers into 8 pairs:
  //   Pair 0 = R0:R1, Pair 1 = R2:R3, ..., Pair 7 = R14:R15
  // The even register holds the high nibble, the odd register holds
  // the low nibble, forming an 8-bit value.

  /**
   * Read an 8-bit value from a register pair.
   * Pair 0 = R0:R1, Pair 1 = R2:R3, etc.
   * High nibble is the even register, low nibble is the odd register.
   */
  private _readPair(pairIdx: number): number {
    const highReg = pairIdx * 2;
    const lowReg = highReg + 1;
    return (this.registers[highReg] << 4) | this.registers[lowReg];
  }

  /** Write an 8-bit value to a register pair. */
  private _writePair(pairIdx: number, value: number): void {
    const highReg = pairIdx * 2;
    const lowReg = highReg + 1;
    this.registers[highReg] = (value >> 4) & 0xf;
    this.registers[lowReg] = value & 0xf;
  }

  // -----------------------------------------------------------------
  // Stack helpers
  // -----------------------------------------------------------------

  /**
   * Push a return address onto the 3-level hardware stack.
   *
   * The real 4004 wraps silently on overflow -- the 4th push overwrites
   * the oldest entry. There is no stack overflow exception.
   */
  private _stackPush(address: number): void {
    this.hwStack[this.stackPointer] = address & 0xfff;
    this.stackPointer = (this.stackPointer + 1) % 3;
  }

  /** Pop a return address from the hardware stack. */
  private _stackPop(): number {
    this.stackPointer = (this.stackPointer - 1 + 3) % 3;
    return this.hwStack[this.stackPointer];
  }

  // -----------------------------------------------------------------
  // RAM helpers
  // -----------------------------------------------------------------

  /** Read the current RAM main character (set by SRC + DCL). */
  private _ramReadMain(): number {
    return this.ram[this.ramBank][this.ramRegister][this.ramCharacter];
  }

  /** Write to the current RAM main character. */
  private _ramWriteMain(value: number): void {
    this.ram[this.ramBank][this.ramRegister][this.ramCharacter] = value & 0xf;
  }

  /** Read a RAM status character (0-3) for the current register. */
  private _ramReadStatus(index: number): number {
    return this.ramStatus[this.ramBank][this.ramRegister][index];
  }

  /** Write a RAM status character (0-3). */
  private _ramWriteStatus(index: number, value: number): void {
    this.ramStatus[this.ramBank][this.ramRegister][index] = value & 0xf;
  }

  // -----------------------------------------------------------------
  // Public API
  // -----------------------------------------------------------------

  /**
   * Load a program into ROM starting at address 0.
   *
   * Each byte in the program is one byte of ROM. Instructions are 8 bits,
   * but some (JUN, JMS, JCN, FIM, ISZ) are 2 bytes.
   */
  loadProgram(program: Uint8Array): void {
    this.memory.fill(0);
    for (let i = 0; i < program.length; i++) {
      this.memory[i] = program[i];
    }
  }

  load(program: Uint8Array): void {
    this.loadProgram(program);
  }

  /**
   * Reset all CPU state to initial values.
   */
  reset(): void {
    this.accumulator = 0;
    this.registers = new Array(16).fill(0);
    this.carry = false;
    this.memory = new Uint8Array(this.memory.length);
    this.ram = this._makeRam();
    this.ramStatus = this._makeRamStatus();
    this.ramOutput = [0, 0, 0, 0];
    this.ramBank = 0;
    this.ramRegister = 0;
    this.ramCharacter = 0;
    this.romPort = 0;
    this.hwStack = [0, 0, 0];
    this.stackPointer = 0;
    this.pc = 0;
    this.halted = false;
  }

  /**
   * Fetch, decode, and execute one instruction.
   *
   * This is the core of the simulator. The 4004 doesn't have a pipeline --
   * it completes each instruction before starting the next. The sequence is:
   *
   * 1. FETCH:   Read the byte at memory[PC]. If it's a 2-byte instruction,
   *             also read the next byte.
   * 2. DECODE:  Split into opcode (upper nibble) and operand (lower nibble).
   * 3. EXECUTE: Perform the operation, update state.
   *
   * Returns an Intel4004Trace with complete before/after state.
   */
  step(): Intel4004Trace {
    if (this.halted) {
      throw new Error("CPU is halted -- cannot step further");
    }

    // --- Fetch ---
    const address = this.pc;
    const raw = this.memory[this.pc];

    // Check for 2-byte instruction and fetch second byte
    let raw2: number | undefined;
    if (isTwoByte(raw)) {
      raw2 = this.memory[this.pc + 1];
    }

    // Advance PC past the instruction bytes BEFORE execution,
    // so jump targets work correctly
    this.pc += raw2 !== undefined ? 2 : 1;

    // --- Snapshot state before execution ---
    const accBefore = this.accumulator;
    const carryBefore = this.carry;

    // --- Decode ---
    const upper = (raw >> 4) & 0xf;
    const lower = raw & 0xf;

    // --- Execute ---
    const mnemonic = this._execute(upper, lower, raw, raw2, address);

    // --- Build trace ---
    return {
      address,
      raw,
      raw2,
      mnemonic,
      accumulatorBefore: accBefore,
      accumulatorAfter: this.accumulator,
      carryBefore,
      carryAfter: this.carry,
    };
  }

  /**
   * Load and run a program, returning a trace of every instruction.
   *
   * Execution continues until HLT is encountered or maxSteps is reached.
   * The maxSteps limit prevents infinite loops from hanging the simulator.
   *
   * Example -- the x = 1 + 2 program:
   *
   *     const sim = new Intel4004Simulator();
   *     const traces = sim.run(new Uint8Array([0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01]));
   *     sim.registers[1]; // => 3
   */
  run(program: Uint8Array, maxSteps: number = 10000): Intel4004Trace[] {
    this.reset();
    this.load(program);

    const traces: Intel4004Trace[] = [];

    for (let i = 0; i < maxSteps; i++) {
      if (this.halted || this.pc >= this.memory.length) break;
      const trace = this.step();
      traces.push(trace);
    }

    return traces;
  }

  getState(): Intel4004State {
    return Object.freeze({
      accumulator: this.accumulator,
      registers: freezeNumbers(this.registers),
      carry: this.carry,
      pc: this.pc,
      halted: this.halted,
      ram: freezeRam(this.ram),
      hwStack: freezeNumbers(this.hwStack),
      stackPointer: this.stackPointer,
    });
  }

  execute(
    program: Uint8Array,
    maxSteps: number = 100_000
  ): ExecutionResult<Intel4004State> {
    this.reset();
    this.load(program);

    const traces: StepTrace[] = [];
    let steps = 0;
    let error: string | null = null;

    try {
      while (!this.halted && steps < maxSteps) {
        const trace = this.step();
        traces.push(
          new StepTrace(trace.address, this.pc, trace.mnemonic, trace.mnemonic)
        );
        steps += 1;
      }
    } catch (caught) {
      error = caught instanceof Error ? caught.message : String(caught);
    }

    if (error === null && !this.halted) {
      error = `max_steps (${maxSteps}) exceeded`;
    }

    return new ExecutionResult({
      halted: this.halted,
      steps,
      finalState: this.getState(),
      error,
      traces,
    });
  }

  // -----------------------------------------------------------------
  // Instruction dispatch
  // -----------------------------------------------------------------

  /**
   * Dispatch and execute a decoded instruction.
   *
   * The 4004 instruction set is organized by the upper nibble:
   *   0x0_: NOP (0x00), HLT (0x01 -- simulator only)
   *   0x1_: JCN (conditional jump, 2-byte)
   *   0x2_: FIM (even, 2-byte) or SRC (odd)
   *   0x3_: FIN (even) or JIN (odd)
   *   0x4_: JUN (unconditional jump, 2-byte)
   *   0x5_: JMS (call subroutine, 2-byte)
   *   0x6_: INC (increment register)
   *   0x7_: ISZ (increment and skip if zero, 2-byte)
   *   0x8_: ADD (add register to A)
   *   0x9_: SUB (subtract register from A)
   *   0xA_: LD (load register into A)
   *   0xB_: XCH (exchange A and register)
   *   0xC_: BBL (return from subroutine)
   *   0xD_: LDM (load immediate)
   *   0xE_: I/O operations (RAM/ROM read/write)
   *   0xF_: Accumulator operations
   *
   * @param upper - Upper nibble of the instruction byte (opcode family)
   * @param lower - Lower nibble (register, immediate, or condition code)
   * @param raw   - The full first byte
   * @param raw2  - The second byte for 2-byte instructions, or undefined
   * @param fetchAddr - The PC address where this instruction was fetched
   * @returns The mnemonic string for tracing
   */
  private _execute(
    upper: number,
    lower: number,
    raw: number,
    raw2: number | undefined,
    fetchAddr: number
  ): string {
    switch (upper) {
      case 0x0:
        return this._exec0x(raw);
      case 0x1:
        return this._execJcn(lower, raw2!, fetchAddr);
      case 0x2:
        return (lower & 0x1) === 0
          ? this._execFim(lower >> 1, raw2!)
          : this._execSrc(lower >> 1);
      case 0x3:
        return (lower & 0x1) === 0
          ? this._execFin(lower >> 1, fetchAddr)
          : this._execJin(lower >> 1, fetchAddr);
      case 0x4:
        return this._execJun(lower, raw2!);
      case 0x5:
        return this._execJms(lower, raw2!, fetchAddr);
      case 0x6:
        return this._execInc(lower);
      case 0x7:
        return this._execIsz(lower, raw2!, fetchAddr);
      case 0x8:
        return this._execAdd(lower);
      case 0x9:
        return this._execSub(lower);
      case 0xa:
        return this._execLd(lower);
      case 0xb:
        return this._execXch(lower);
      case 0xc:
        return this._execBbl(lower);
      case 0xd:
        return this._execLdm(lower);
      case 0xe:
        return this._execIo(lower);
      case 0xf:
        return this._execAccum(lower);
      default:
        return `UNKNOWN(0x${raw.toString(16).toUpperCase().padStart(2, "0")})`;
    }
  }

  // -----------------------------------------------------------------
  // 0x0_: NOP and HLT
  // -----------------------------------------------------------------

  /**
   * Handle 0x0_ instructions: NOP (0x00) and HLT (0x01).
   *
   * NOP: No operation. The PC has already been advanced.
   * HLT: Halt execution. Not a real 4004 instruction -- we added it
   *       for the simulator. The real 4004 had no halt; it just kept
   *       fetching instructions forever (or until power off).
   */
  private _exec0x(raw: number): string {
    if (raw === 0x00) {
      return "NOP";
    }
    if (raw === 0x01) {
      this.halted = true;
      return "HLT";
    }
    return `UNKNOWN(0x${raw.toString(16).toUpperCase().padStart(2, "0")})`;
  }

  // -----------------------------------------------------------------
  // 0xD_: LDM -- Load immediate into accumulator
  // -----------------------------------------------------------------

  /**
   * LDM N (0xDN): Load a 4-bit immediate value into the accumulator.
   *
   * This is the simplest instruction: put a 4-bit constant into A.
   * This is how you get values into the machine -- there's no "load
   * from memory" for arbitrary constants.
   */
  private _execLdm(n: number): string {
    this.accumulator = n & 0xf;
    return `LDM ${n}`;
  }

  // -----------------------------------------------------------------
  // 0xA_: LD -- Load register into accumulator
  // -----------------------------------------------------------------

  /**
   * LD Rn (0xAR): Load register value into accumulator. A = Rn.
   *
   * Unlike XCH, this is a one-way copy. The register keeps its value.
   */
  private _execLd(reg: number): string {
    this.accumulator = this.registers[reg] & 0xf;
    return `LD R${reg}`;
  }

  // -----------------------------------------------------------------
  // 0xB_: XCH -- Exchange accumulator with register
  // -----------------------------------------------------------------

  /**
   * XCH Rn (0xBN): Exchange accumulator with register.
   *
   * Swap A and Rn. This is the 4004's way of moving data between
   * the accumulator and registers. There's no "move" instruction --
   * you always swap both ways.
   */
  private _execXch(reg: number): string {
    const oldA = this.accumulator;
    this.accumulator = this.registers[reg] & 0xf;
    this.registers[reg] = oldA & 0xf;
    return `XCH R${reg}`;
  }

  // -----------------------------------------------------------------
  // 0x6_: INC -- Increment register
  // -----------------------------------------------------------------

  /**
   * INC Rn (0x6R): Increment register. Rn = (Rn + 1) & 0xF.
   *
   * Note: INC does NOT affect the carry flag. It's purely a register
   * increment with 4-bit wrap-around.
   */
  private _execInc(reg: number): string {
    this.registers[reg] = (this.registers[reg] + 1) & 0xf;
    return `INC R${reg}`;
  }

  // -----------------------------------------------------------------
  // 0x8_: ADD -- Add register to accumulator
  // -----------------------------------------------------------------

  /**
   * ADD Rn (0x8R): Add register to accumulator with carry.
   *
   * A = A + Rn + carry. Carry is set if result > 15.
   *
   * The carry flag participates in the addition -- this is how multi-digit
   * BCD arithmetic works. After adding two BCD digits, the carry propagates
   * to the next digit pair.
   */
  private _execAdd(reg: number): string {
    const result =
      this.accumulator + this.registers[reg] + (this.carry ? 1 : 0);
    this.carry = result > 0xf;
    this.accumulator = result & 0xf;
    return `ADD R${reg}`;
  }

  // -----------------------------------------------------------------
  // 0x9_: SUB -- Subtract register from accumulator
  // -----------------------------------------------------------------

  /**
   * SUB Rn (0x9R): Subtract register from accumulator with borrow.
   *
   * A = A + ~Rn + (carry ? 0 : 1)
   *
   * The 4004 uses complement-add for subtraction. The carry flag is
   * INVERTED from what you might expect:
   *   - carry=true  means NO borrow occurred (result >= 0)
   *   - carry=false means borrow occurred (result was negative before wrap)
   *
   * This matches the MCS-4 manual's definition. The initial carry state
   * acts as an inverse borrow-in. When carry is false (no previous borrow),
   * we add 1 to the complement to get proper subtraction.
   *
   * Truth table for 5 - 3 (carry initially false = no previous borrow):
   *   complement of 3: ~0011 & 0xF = 1100 = 12
   *   borrow_in = 1 (since carry is false)
   *   result = 5 + 12 + 1 = 18 > 15, so carry = true (no borrow)
   *   A = 18 & 0xF = 2 (correct: 5 - 3 = 2)
   */
  private _execSub(reg: number): string {
    const complement = (~this.registers[reg]) & 0xf;
    const borrowIn = this.carry ? 0 : 1;
    const result = this.accumulator + complement + borrowIn;
    this.carry = result > 0xf;
    this.accumulator = result & 0xf;
    return `SUB R${reg}`;
  }

  // -----------------------------------------------------------------
  // 0x1_: JCN -- Conditional jump
  // -----------------------------------------------------------------

  /**
   * JCN cond,addr (0x1C 0xAA): Conditional jump.
   *
   * The condition nibble C has 4 bits:
   *     Bit 3 (0x8): INVERT -- if set, invert the final test result
   *     Bit 2 (0x4): TEST_ZERO -- test if accumulator == 0
   *     Bit 1 (0x2): TEST_CARRY -- test if carry == 1
   *     Bit 0 (0x1): TEST_PIN -- test input pin (always 0 in simulator)
   *
   * Multiple test bits can be set -- they are OR'd together. If the
   * (possibly inverted) result is true, the jump is taken.
   *
   * The target address is within the same 256-byte page as the
   * instruction following the JCN (i.e., PC after fetching both bytes).
   */
  private _execJcn(cond: number, addr: number, fetchAddr: number): string {
    // Evaluate condition tests (OR'd together)
    let testResult = false;
    if (cond & 0x4) {
      // Test accumulator == 0
      testResult = testResult || this.accumulator === 0;
    }
    if (cond & 0x2) {
      // Test carry == 1
      testResult = testResult || this.carry;
    }
    if (cond & 0x1) {
      // Test input pin (always false in simulator)
      testResult = testResult || false;
    }

    // Invert if bit 3 is set
    if (cond & 0x8) {
      testResult = !testResult;
    }

    if (testResult) {
      // Target is within the same 256-byte page as PC after fetch
      const page = (fetchAddr + 2) & 0xf00;
      this.pc = page | addr;
    }
    // If not taken, PC already advanced past the 2-byte instruction

    return `JCN ${cond},${addr.toString(16).toUpperCase().padStart(2, "0")}`;
  }

  // -----------------------------------------------------------------
  // 0x2_ even: FIM -- Fetch immediate to register pair
  // -----------------------------------------------------------------

  /**
   * FIM Pp,data (0x2P 0xDD): Fetch immediate to register pair.
   *
   * Load the 8-bit immediate data into register pair Pp.
   * High nibble goes to R(2*p), low nibble goes to R(2*p+1).
   */
  private _execFim(pair: number, data: number): string {
    this._writePair(pair, data);
    return `FIM P${pair},0x${data.toString(16).toUpperCase().padStart(2, "0")}`;
  }

  // -----------------------------------------------------------------
  // 0x2_ odd: SRC -- Send register control
  // -----------------------------------------------------------------

  /**
   * SRC Pp (0x2P+1): Send register control.
   *
   * Send the 8-bit value in register pair Pp as an address for
   * subsequent RAM/ROM I/O operations. The high nibble selects the
   * RAM register (0-3), the low nibble selects the character (0-15).
   */
  private _execSrc(pair: number): string {
    const pairVal = this._readPair(pair);
    this.ramRegister = (pairVal >> 4) & 0xf;
    this.ramCharacter = pairVal & 0xf;
    return `SRC P${pair}`;
  }

  // -----------------------------------------------------------------
  // 0x3_ even: FIN -- Fetch indirect from ROM
  // -----------------------------------------------------------------

  /**
   * FIN Pp (0x3P): Fetch indirect from ROM.
   *
   * Read the ROM byte at the address given by register pair P0 (R0:R1),
   * and store the result into register pair Pp.
   *
   * The address used is within the same page as the current PC
   * (bits 11-8 of PC are preserved, bits 7-0 come from P0).
   */
  private _execFin(pair: number, fetchAddr: number): string {
    // Address comes from P0 (R0:R1)
    const p0Val = this._readPair(0);
    // Same page as current instruction
    const currentPage = fetchAddr & 0xf00;
    const romAddr = currentPage | p0Val;
    const romByte =
      romAddr < this.memory.length ? this.memory[romAddr] : 0;

    this._writePair(pair, romByte);
    return `FIN P${pair}`;
  }

  // -----------------------------------------------------------------
  // 0x3_ odd: JIN -- Jump indirect
  // -----------------------------------------------------------------

  /**
   * JIN Pp (0x3P+1): Jump indirect.
   *
   * Jump to the address formed by the current page and register pair Pp.
   * PC[11:8] stays the same, PC[7:0] = pair value.
   */
  private _execJin(pair: number, fetchAddr: number): string {
    const pairVal = this._readPair(pair);
    const currentPage = fetchAddr & 0xf00;
    this.pc = currentPage | pairVal;
    return `JIN P${pair}`;
  }

  // -----------------------------------------------------------------
  // 0x4_: JUN -- Unconditional jump
  // -----------------------------------------------------------------

  /**
   * JUN addr (0x4H 0xLL): Unconditional jump to 12-bit address.
   *
   * The lower nibble of the first byte provides bits 11-8 of the
   * target address; the second byte provides bits 7-0.
   */
  private _execJun(highNibble: number, lowByte: number): string {
    const addr = (highNibble << 8) | lowByte;
    this.pc = addr;
    return `JUN 0x${addr.toString(16).toUpperCase().padStart(3, "0")}`;
  }

  // -----------------------------------------------------------------
  // 0x5_: JMS -- Jump to subroutine
  // -----------------------------------------------------------------

  /**
   * JMS addr (0x5H 0xLL): Jump to subroutine.
   *
   * Push the address of the NEXT instruction onto the hardware stack,
   * then jump to the 12-bit target address.
   *
   * The return address is the address after this 2-byte instruction,
   * which is fetchAddr + 2. The PC has already been advanced past the
   * 2-byte instruction, so we push the current PC value.
   */
  private _execJms(
    highNibble: number,
    lowByte: number,
    fetchAddr: number
  ): string {
    const addr = (highNibble << 8) | lowByte;
    // Push return address (address of instruction AFTER this 2-byte JMS)
    const returnAddr = fetchAddr + 2;
    this._stackPush(returnAddr);
    this.pc = addr;
    return `JMS 0x${addr.toString(16).toUpperCase().padStart(3, "0")}`;
  }

  // -----------------------------------------------------------------
  // 0xC_: BBL -- Branch back and load
  // -----------------------------------------------------------------

  /**
   * BBL N (0xCN): Branch back and load.
   *
   * Pop the top of the hardware stack, load N into the accumulator,
   * and jump to the popped address.
   *
   * This is the 4004's "return from subroutine" instruction with a
   * twist -- it also loads an immediate value into A. This lets a
   * subroutine return a simple status code.
   */
  private _execBbl(n: number): string {
    this.accumulator = n & 0xf;
    const returnAddr = this._stackPop();
    this.pc = returnAddr;
    return `BBL ${n}`;
  }

  // -----------------------------------------------------------------
  // 0x7_: ISZ -- Increment and skip if zero
  // -----------------------------------------------------------------

  /**
   * ISZ Rn,addr (0x7R 0xAA): Increment register, skip if zero.
   *
   * Increment Rn. If Rn != 0 after increment, jump to addr.
   * If Rn == 0 (wrapped from 15), continue to next instruction.
   *
   * This is the 4004's loop counter instruction. Load a register with
   * a negative count (in 4-bit two's complement, e.g., -4 = 12), then
   * ISZ will loop until the register wraps to 0.
   *
   * The target address is within the same 256-byte page.
   */
  private _execIsz(reg: number, addr: number, fetchAddr: number): string {
    this.registers[reg] = (this.registers[reg] + 1) & 0xf;

    if (this.registers[reg] !== 0) {
      const page = (fetchAddr + 2) & 0xf00;
      this.pc = page | addr;
    }
    // If zero, PC already advanced past the 2-byte instruction

    return `ISZ R${reg},0x${addr.toString(16).toUpperCase().padStart(2, "0")}`;
  }

  // -----------------------------------------------------------------
  // 0xE_: I/O operations (RAM/ROM read/write)
  // -----------------------------------------------------------------

  /**
   * Handle the 0xE_ instruction family: RAM and ROM I/O operations.
   *
   * These instructions interact with the RAM data memory and ROM I/O
   * ports. The RAM address is set by a prior SRC instruction (which
   * sets ramRegister and ramCharacter) and DCL (which sets ramBank).
   *
   *     0xE0 WRM  -- Write accumulator to RAM main character
   *     0xE1 WMP  -- Write accumulator to RAM output port
   *     0xE2 WRR  -- Write accumulator to ROM I/O port
   *     0xE3 WPM  -- Write program RAM (NOP in simulator)
   *     0xE4 WR0  -- Write accumulator to RAM status 0
   *     0xE5 WR1  -- Write accumulator to RAM status 1
   *     0xE6 WR2  -- Write accumulator to RAM status 2
   *     0xE7 WR3  -- Write accumulator to RAM status 3
   *     0xE8 SBM  -- Subtract RAM from accumulator
   *     0xE9 RDM  -- Read RAM main character into accumulator
   *     0xEA RDR  -- Read ROM I/O port into accumulator
   *     0xEB ADM  -- Add RAM to accumulator
   *     0xEC RD0  -- Read RAM status 0 into accumulator
   *     0xED RD1  -- Read RAM status 1 into accumulator
   *     0xEE RD2  -- Read RAM status 2 into accumulator
   *     0xEF RD3  -- Read RAM status 3 into accumulator
   */
  private _execIo(lower: number): string {
    switch (lower) {
      // --- Write operations ---

      case 0x0: {
        // WRM: Write accumulator to RAM main character
        this._ramWriteMain(this.accumulator);
        return "WRM";
      }

      case 0x1: {
        // WMP: Write accumulator to RAM output port
        this.ramOutput[this.ramBank] = this.accumulator & 0xf;
        return "WMP";
      }

      case 0x2: {
        // WRR: Write accumulator to ROM I/O port
        this.romPort = this.accumulator & 0xf;
        return "WRR";
      }

      case 0x3: {
        // WPM: Write program RAM -- not simulated (EPROM programming).
        // We treat it as a NOP.
        return "WPM";
      }

      case 0x4: {
        // WR0: Write accumulator to RAM status character 0
        this._ramWriteStatus(0, this.accumulator);
        return "WR0";
      }

      case 0x5: {
        // WR1: Write accumulator to RAM status character 1
        this._ramWriteStatus(1, this.accumulator);
        return "WR1";
      }

      case 0x6: {
        // WR2: Write accumulator to RAM status character 2
        this._ramWriteStatus(2, this.accumulator);
        return "WR2";
      }

      case 0x7: {
        // WR3: Write accumulator to RAM status character 3
        this._ramWriteStatus(3, this.accumulator);
        return "WR3";
      }

      // --- Arithmetic with RAM ---

      case 0x8: {
        // SBM: Subtract RAM main character from accumulator.
        // Uses complement-add, same as SUB but with RAM value.
        const ramVal = this._ramReadMain();
        const complement = (~ramVal) & 0xf;
        const borrowIn = this.carry ? 0 : 1;
        const result = this.accumulator + complement + borrowIn;
        this.carry = result > 0xf;
        this.accumulator = result & 0xf;
        return "SBM";
      }

      // --- Read operations ---

      case 0x9: {
        // RDM: Read RAM main character into accumulator
        this.accumulator = this._ramReadMain();
        return "RDM";
      }

      case 0xa: {
        // RDR: Read ROM I/O port into accumulator
        this.accumulator = this.romPort & 0xf;
        return "RDR";
      }

      case 0xb: {
        // ADM: Add RAM main character to accumulator with carry.
        // Same as ADD but with RAM value instead of register.
        const ramVal = this._ramReadMain();
        const result =
          this.accumulator + ramVal + (this.carry ? 1 : 0);
        this.carry = result > 0xf;
        this.accumulator = result & 0xf;
        return "ADM";
      }

      case 0xc: {
        // RD0: Read RAM status character 0
        this.accumulator = this._ramReadStatus(0);
        return "RD0";
      }

      case 0xd: {
        // RD1: Read RAM status character 1
        this.accumulator = this._ramReadStatus(1);
        return "RD1";
      }

      case 0xe: {
        // RD2: Read RAM status character 2
        this.accumulator = this._ramReadStatus(2);
        return "RD2";
      }

      case 0xf: {
        // RD3: Read RAM status character 3
        this.accumulator = this._ramReadStatus(3);
        return "RD3";
      }

      default:
        return `UNKNOWN(0xE${lower.toString(16).toUpperCase()})`;
    }
  }

  // -----------------------------------------------------------------
  // 0xF_: Accumulator operations
  // -----------------------------------------------------------------

  /**
   * Handle the 0xF_ instruction family: accumulator manipulation.
   *
   * These instructions perform various operations on the accumulator
   * and carry flag without involving registers or memory.
   *
   *     0xF0 CLB  -- Clear both (A=0, carry=0)
   *     0xF1 CLC  -- Clear carry
   *     0xF2 IAC  -- Increment accumulator
   *     0xF3 CMC  -- Complement carry
   *     0xF4 CMA  -- Complement accumulator
   *     0xF5 RAL  -- Rotate left through carry
   *     0xF6 RAR  -- Rotate right through carry
   *     0xF7 TCC  -- Transfer carry to accumulator
   *     0xF8 DAC  -- Decrement accumulator
   *     0xF9 TCS  -- Transfer carry subtract
   *     0xFA STC  -- Set carry
   *     0xFB DAA  -- Decimal adjust accumulator
   *     0xFC KBP  -- Keyboard process
   *     0xFD DCL  -- Designate command line (select RAM bank)
   */
  private _execAccum(lower: number): string {
    switch (lower) {
      case 0x0: {
        // CLB: Clear both. A = 0, carry = false.
        this.accumulator = 0;
        this.carry = false;
        return "CLB";
      }

      case 0x1: {
        // CLC: Clear carry flag.
        this.carry = false;
        return "CLC";
      }

      case 0x2: {
        // IAC: Increment accumulator. A = (A + 1) & 0xF.
        // Carry is set if A was 15 (wraps to 0).
        const result = this.accumulator + 1;
        this.carry = result > 0xf;
        this.accumulator = result & 0xf;
        return "IAC";
      }

      case 0x3: {
        // CMC: Complement carry. carry = !carry.
        this.carry = !this.carry;
        return "CMC";
      }

      case 0x4: {
        // CMA: Complement accumulator. A = ~A & 0xF (4-bit NOT).
        // For example: A=0101 becomes A=1010.
        this.accumulator = (~this.accumulator) & 0xf;
        return "CMA";
      }

      case 0x5: {
        // RAL: Rotate accumulator left through carry.
        //
        // Before: [carry | A3 A2 A1 A0]
        // After:  [A3   | A2 A1 A0 carry_old]
        //
        // The carry shifts into the lowest bit, and the highest bit
        // shifts into carry. This is a 5-bit rotation through carry.
        const oldCarry = this.carry ? 1 : 0;
        this.carry = (this.accumulator & 0x8) !== 0;
        this.accumulator = ((this.accumulator << 1) | oldCarry) & 0xf;
        return "RAL";
      }

      case 0x6: {
        // RAR: Rotate accumulator right through carry.
        //
        // Before: [carry | A3 A2 A1 A0]
        // After:  [A0   | carry_old A3 A2 A1]
        //
        // The carry shifts into the highest bit, and the lowest bit
        // shifts into carry. This is a 5-bit rotation through carry.
        const oldCarry = this.carry ? 1 : 0;
        this.carry = (this.accumulator & 0x1) !== 0;
        this.accumulator =
          ((this.accumulator >> 1) | (oldCarry << 3)) & 0xf;
        return "RAR";
      }

      case 0x7: {
        // TCC: Transfer carry to accumulator, clear carry.
        // A = 1 if carry was set, else 0. Carry is always cleared.
        this.accumulator = this.carry ? 1 : 0;
        this.carry = false;
        return "TCC";
      }

      case 0x8: {
        // DAC: Decrement accumulator. A = (A - 1) & 0xF.
        // Carry is SET if no borrow (A > 0), CLEARED if borrow (A was 0).
        const result = this.accumulator - 1;
        this.carry = result >= 0;
        this.accumulator = result & 0xf;
        return "DAC";
      }

      case 0x9: {
        // TCS: Transfer carry subtract.
        // A = 10 if carry was set, else 9. Carry is always cleared.
        //
        // This is used in BCD subtraction: it provides the tens-complement
        // correction factor.
        this.accumulator = this.carry ? 10 : 9;
        this.carry = false;
        return "TCS";
      }

      case 0xa: {
        // STC: Set carry. carry = true.
        this.carry = true;
        return "STC";
      }

      case 0xb: {
        // DAA: Decimal adjust accumulator (BCD correction).
        //
        // If A > 9 or carry is set, add 6 to A. If the addition causes
        // overflow past 15, set carry.
        //
        // This exists because the 4004 was built for BCD calculators.
        // When you add two BCD digits (0-9 each), the result might be
        // > 9 (e.g., 7 + 8 = 15). DAA corrects this by adding 6,
        // wrapping to the correct BCD digit (15 + 6 = 21, keep lower
        // nibble 5, set carry for the tens digit).
        if (this.accumulator > 9 || this.carry) {
          const result = this.accumulator + 6;
          if (result > 0xf) {
            this.carry = true;
          }
          this.accumulator = result & 0xf;
        }
        return "DAA";
      }

      case 0xc: {
        // KBP: Keyboard process.
        //
        // Converts a 1-hot encoded input to a binary position number:
        //     0b0000 (0)  -> 0  (no key pressed)
        //     0b0001 (1)  -> 1  (key 1)
        //     0b0010 (2)  -> 2  (key 2)
        //     0b0100 (4)  -> 3  (key 3)
        //     0b1000 (8)  -> 4  (key 4)
        //     anything else -> 15 (error: multiple keys pressed)
        //
        // This was designed for the Busicom calculator's keyboard scanning.
        const kbpTable: Record<number, number> = {
          0: 0,
          1: 1,
          2: 2,
          4: 3,
          8: 4,
        };
        this.accumulator =
          kbpTable[this.accumulator] !== undefined
            ? kbpTable[this.accumulator]
            : 15;
        return "KBP";
      }

      case 0xd: {
        // DCL: Designate command line (select RAM bank).
        //
        // The lower 3 bits of A select the RAM bank (0-7, but only 0-3
        // are typically used since the 4004 has 4 RAM banks).
        this.ramBank = this.accumulator & 0x7;
        if (this.ramBank > 3) {
          this.ramBank = this.ramBank & 0x3;
        }
        return "DCL";
      }

      default:
        return `UNKNOWN(0xF${lower.toString(16).toUpperCase()})`;
    }
  }
}
