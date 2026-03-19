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
 *     PC:               Program counter (points to the next instruction in ROM).
 *
 * === Instruction encoding ===
 *
 * Instructions are 8 bits (1 byte). The upper nibble is the opcode, and the
 * lower nibble is the operand (a register number or immediate value):
 *
 *     +----------+----------+
 *     |  opcode  | operand  |
 *     |  bits 7-4| bits 3-0 |
 *     +----------+----------+
 *
 *     LDM N   (0xDN):  Load immediate N into accumulator. A = N.
 *     XCH RN  (0xBN):  Exchange accumulator with register N. Swap A and RN.
 *     ADD RN  (0x8N):  Add register N to accumulator with carry. A = A + RN.
 *     SUB RN  (0x9N):  Subtract register N from accumulator with borrow. A = A - RN.
 *     HLT     (0x01):  Halt execution. (Simulator-only, not a real 4004 opcode.)
 *
 * === The x = 1 + 2 program ===
 *
 * To compute x = 1 + 2 and store the result in R1:
 *
 *     LDM 1      A = 1                   -> 0xD1
 *     XCH R0     R0 = 1, A = 0           -> 0xB0
 *     LDM 2      A = 2                   -> 0xD2
 *     ADD R0     A = 2 + 1 = 3           -> 0x80
 *     XCH R1     R1 = 3, A = 0           -> 0xB1
 *     HLT        stop                    -> 0x01
 *
 * Six instructions to add two numbers! RISC-V does it in four (two loads, one
 * add, one halt). The accumulator bottleneck is the price of simpler hardware.
 */

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
 *     raw:                The raw byte (0x00-0xFF).
 *     mnemonic:           Human-readable instruction (e.g., "LDM 1", "ADD R0").
 *     accumulatorBefore:  Value of A before execution.
 *     accumulatorAfter:   Value of A after execution.
 *     carryBefore:        Carry flag before execution.
 *     carryAfter:         Carry flag after execution.
 */
export interface Intel4004Trace {
  address: number;
  raw: number;
  mnemonic: string;
  accumulatorBefore: number;
  accumulatorAfter: number;
  carryBefore: boolean;
  carryAfter: boolean;
}

// ---------------------------------------------------------------------------
// The simulator
// ---------------------------------------------------------------------------

/**
 * A simulator for the Intel 4004 microprocessor.
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
 *     accumulator:  4-bit accumulator (0-15). The heart of computation.
 *     registers:    16 general-purpose 4-bit registers (R0-R15).
 *     carry:        Carry/borrow flag from the last arithmetic operation.
 *     memory:       ROM holding the program bytes.
 *     pc:           Program counter -- index into memory.
 *     halted:       True after HLT is executed.
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
  // SUB borrows (result would be negative). This is how the 4004
  // handles multi-digit arithmetic -- carry propagates between digits.
  carry: boolean = false;

  // --- Memory ---
  // The 4004 had separate ROM (program) and RAM (data) address spaces.
  // We only model ROM here -- enough for our instruction set.
  // The original 4004 could address 4096 bytes of ROM.
  memory: Uint8Array;

  // --- Control ---
  pc: number = 0;
  halted: boolean = false;

  constructor(memorySize: number = 4096) {
    this.registers = new Array(16).fill(0);
    this.memory = new Uint8Array(memorySize);
  }

  /**
   * Load a program into ROM starting at address 0.
   *
   * Each byte in the program is one instruction. The 4004's instructions
   * are 8 bits -- much simpler than RISC-V's 32-bit encoding.
   */
  loadProgram(program: Uint8Array): void {
    for (let i = 0; i < program.length; i++) {
      this.memory[i] = program[i];
    }
  }

  /**
   * Fetch, decode, and execute one instruction.
   *
   * This is the core of the simulator. The 4004 doesn't have a pipeline --
   * it completes each instruction before starting the next. The sequence is:
   *
   * 1. FETCH:   Read the byte at memory[PC].
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
    this.pc += 1;

    // --- Snapshot state before execution ---
    const accBefore = this.accumulator;
    const carryBefore = this.carry;

    // --- Decode ---
    // Upper nibble = opcode, lower nibble = operand (register or immediate)
    const opcode = (raw >> 4) & 0xf;
    const operand = raw & 0xf;

    // --- Execute ---
    const mnemonic = this._execute(opcode, operand, raw);

    // --- Build trace ---
    return {
      address,
      raw,
      mnemonic,
      accumulatorBefore: accBefore,
      accumulatorAfter: this.accumulator,
      carryBefore,
      carryAfter: this.carry,
    };
  }

  /**
   * Dispatch and execute a decoded instruction.
   *
   * Each case handles one instruction type. The mnemonic string is
   * returned for the trace -- it's how we make the execution log
   * human-readable.
   */
  private _execute(opcode: number, operand: number, raw: number): string {
    // --- LDM N (0xDN): Load immediate into accumulator ---
    // The simplest instruction: put a 4-bit constant into A.
    // This is how you get values into the machine -- there's no
    // "load from memory" in our minimal set.
    if (opcode === 0xd) {
      this.accumulator = operand & 0xf;
      return `LDM ${operand}`;
    }

    // --- XCH RN (0xBN): Exchange accumulator with register ---
    // Swap A and RN. This is the 4004's way of moving data between
    // the accumulator and registers. There's no "move" instruction --
    // you always swap both ways. To "store" A into RN, you XCH (and
    // A gets RN's old value). To "load" RN into A, you also XCH.
    if (opcode === 0xb) {
      const reg = operand & 0xf;
      const oldA = this.accumulator;
      this.accumulator = this.registers[reg] & 0xf;
      this.registers[reg] = oldA & 0xf;
      return `XCH R${reg}`;
    }

    // --- ADD RN (0x8N): Add register to accumulator ---
    // A = A + RN. If the result exceeds 15, it wraps around and the
    // carry flag is set. For example: 15 + 1 = 0 with carry=1.
    // The carry flag enables multi-digit BCD addition -- the whole
    // reason the 4004 exists.
    if (opcode === 0x8) {
      const reg = operand & 0xf;
      const result = this.accumulator + this.registers[reg];
      this.carry = result > 0xf;
      this.accumulator = result & 0xf;
      return `ADD R${reg}`;
    }

    // --- SUB RN (0x9N): Subtract register from accumulator ---
    // A = A - RN. If the result would be negative, it wraps around
    // (two's complement in 4 bits) and the carry flag is set to
    // indicate a borrow. For example: 0 - 1 = 15 with carry=1.
    if (opcode === 0x9) {
      const reg = operand & 0xf;
      const result = this.accumulator - this.registers[reg];
      this.carry = result < 0;
      this.accumulator = result & 0xf;
      return `SUB R${reg}`;
    }

    // --- HLT (0x01): Halt ---
    // Not a real 4004 instruction -- we added it for our simulator.
    // The real 4004 had no halt; it just kept fetching instructions
    // forever (or until power off). We need a way to stop.
    if (raw === 0x01) {
      this.halted = true;
      return "HLT";
    }

    // --- Unknown instruction ---
    return `UNKNOWN(0x${raw.toString(16).toUpperCase().padStart(2, "0")})`;
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
  run(
    program: Uint8Array,
    maxSteps: number = 10000
  ): Intel4004Trace[] {
    this.loadProgram(program);
    const traces: Intel4004Trace[] = [];

    for (let i = 0; i < maxSteps; i++) {
      const trace = this.step();
      traces.push(trace);
      if (this.halted) break;
    }

    return traces;
  }
}
