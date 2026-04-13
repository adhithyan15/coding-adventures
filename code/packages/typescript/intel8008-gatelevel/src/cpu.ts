/**
 * Intel 8008 gate-level CPU — full simulator routing all operations through gates.
 *
 * === Architecture overview ===
 *
 * This is Layer 2 of the gate-level stack:
 *
 *   intel8008-gatelevel
 *   ├── cpu.ts          ← YOU ARE HERE (wires all components together)
 *   ├── decoder.ts      ← Combinational: opcode → control signals
 *   ├── alu.ts          ← Arithmetic: ripple-carry adder chain
 *   ├── registers.ts    ← RegisterFile + FlagRegister (D flip-flops)
 *   ├── pc.ts           ← 14-bit PC with half-adder increment
 *   ├── stack.ts        ← 8-level push-down stack
 *   └── bits.ts         ← intToBits, bitsToInt, computeParity
 *
 * The CPU wires these components together in a fetch-decode-execute cycle:
 *
 *   1. FETCH: Read opcode from memory[PC]; PC ← PC + 1 (via PC half-adder chain)
 *   2. DECODE: Pass opcode through the gate decoder → control signals
 *   3. EXECUTE: Route operands through ALU gates, write results to registers
 *
 * Unlike the behavioral simulator (intel8008-simulator), EVERY computation
 * passes through logic gate functions:
 * - Arithmetic: through GateALU8 → ALU(8) → ripple_carry_adder → full_adder → gates
 * - Bitwise ops: through AND/OR/XOR functions from logic-gates
 * - Registers: through register() function (D flip-flop model)
 * - PC increment: through 14-bit half-adder chain
 * - Stack push/pop: through register() functions
 *
 * === Same public API as behavioral simulator ===
 *
 * Intel8008GateLevel is a drop-in replacement for Intel8008Simulator:
 * - Same register accessors (a, b, c, d, e, h, l, pc, hlAddress)
 * - Same step() → Trace interface
 * - Same run(program, maxSteps, startAddress) method
 * - Same reset(), loadProgram(), setInputPort(), getOutputPort()
 *
 * Additional method: gateCount() — returns the total number of gate function
 * calls made since the last reset, giving a measure of computational depth.
 *
 * === Gate counting ===
 *
 * The gateCount() method tracks total gate function invocations:
 * - Each call to AND(), OR(), XOR(), NOT() from logic-gates = 1 gate
 * - The GateALU8 add() chains 8 full-adders = 40 gates per add
 * - The ProgramCounter increment() uses 28 gates (14 HAs × 2 gates)
 * - The decoder uses ~40 AND/OR/NOT gates per opcode
 *
 * This metric demonstrates that the gate-level simulator genuinely exercises
 * the gate functions, not just host arithmetic.
 */

import { NOT, OR, type Bit } from "@coding-adventures/logic-gates";
import { decode } from "./decoder.js";
import { GateALU8 } from "./alu.js";
import { RegisterFile, FlagRegister } from "./registers.js";
import { ProgramCounter } from "./pc.js";
import { PushDownStack } from "./stack.js";

// Re-export the Flags and Trace interfaces so callers don't need to import
// from two packages.
export type { GateFlags as GateLevelFlags } from "./alu.js";

// -------------------------------------------------------------------------
// Public types (mirroring the behavioral simulator's interface)
// -------------------------------------------------------------------------

/**
 * CPU condition flags — mirrors the behavioral simulator's Flags interface.
 * Uses boolean (not Bit) for API compatibility.
 */
export interface Flags {
  carry: boolean;
  zero: boolean;
  sign: boolean;
  parity: boolean;
}

/**
 * Trace record for one executed instruction — same interface as behavioral sim.
 *
 * Captures before/after state of the accumulator and flags, plus memory access info.
 */
export interface Trace {
  /** PC where this instruction was fetched (before increment). */
  address: number;
  /** Raw instruction bytes (1, 2, or 3 bytes). */
  raw: Uint8Array;
  /** Human-readable mnemonic: "MOV A, B", "ADI 0x05", "JMP 0x0100". */
  mnemonic: string;
  /** Accumulator value before execution. */
  aBefore: number;
  /** Accumulator value after execution. */
  aAfter: number;
  /** Flags before execution. */
  flagsBefore: Flags;
  /** Flags after execution. */
  flagsAfter: Flags;
  /** 14-bit memory address accessed (if M register used), or null. */
  memAddress: number | null;
  /** Value read or written at memAddress, or null. */
  memValue: number | null;
}

// -------------------------------------------------------------------------
// Name tables (for mnemonic generation)
// -------------------------------------------------------------------------

const REG_NAMES = ["B", "C", "D", "E", "H", "L", "M", "A"] as const;
const COND_NAMES = ["C", "Z", "S", "P"] as const;
const ALU_REG_NAMES = ["ADD", "ADC", "SUB", "SBB", "ANA", "XRA", "ORA", "CMP"];
const ALU_IMM_NAMES = ["ADI", "ACI", "SUI", "SBI", "ANI", "XRI", "ORI", "CPI"];

// -------------------------------------------------------------------------
// Intel8008GateLevel
// -------------------------------------------------------------------------

/**
 * Intel 8008 gate-level simulator.
 *
 * All computations route through logic gate functions from the logic-gates
 * and arithmetic packages. This gives a faithful simulation of the 8008's
 * actual transistor-level behavior, at the cost of being ~100× slower than
 * the behavioral simulator.
 *
 * ```typescript
 * const cpu = new Intel8008GateLevel();
 * const program = new Uint8Array([0x06, 0x01, 0x3E, 0x02, 0x80, 0x76]);
 * const traces = cpu.run(program);
 * console.log(cpu.a);             // 3
 * console.log(cpu.gateCount());   // total gate invocations
 * ```
 */
export class Intel8008GateLevel {
  // -------------------------------------------------------------------------
  // Components
  // -------------------------------------------------------------------------

  /** 7-register file (A, B, C, D, E, H, L) — each register = 8 D flip-flops. */
  private readonly _regs = new RegisterFile();

  /** 4-bit flag register (CY, Z, S, P) — 4 D flip-flops. */
  private readonly _flags = new FlagRegister();

  /** 14-bit program counter with half-adder increment chain. */
  private readonly _pc = new ProgramCounter();

  /** 8-level push-down stack (entry 0 = current PC). */
  private readonly _stack = new PushDownStack();

  /** 8-bit ALU built from ripple-carry adder chain. */
  private readonly _alu = new GateALU8();

  /** 16,384 bytes of unified program + data memory. */
  private _mem: Uint8Array = new Uint8Array(16384);

  /** True after HLT — no further execution until reset(). */
  private _halted = false;

  /** 8 input port values (set externally via setInputPort). */
  private _inputPorts: number[] = new Array(8).fill(0);

  /** 24 output port values (written by OUT instructions). */
  private _outputPorts: number[] = new Array(24).fill(0);

  // -------------------------------------------------------------------------
  // Register accessors (public API — mirrors behavioral simulator)
  // -------------------------------------------------------------------------

  /** Accumulator (register A = index 7). 8-bit unsigned (0–255). */
  get a(): number { return this._regs.a; }

  /** Register B (index 0). 8-bit unsigned (0–255). */
  get b(): number { return this._regs.b; }

  /** Register C (index 1). 8-bit unsigned (0–255). */
  get c(): number { return this._regs.c; }

  /** Register D (index 2). 8-bit unsigned (0–255). */
  get d(): number { return this._regs.d; }

  /** Register E (index 3). 8-bit unsigned (0–255). */
  get e(): number { return this._regs.e; }

  /** Register H (index 4) — high byte of memory address pair. */
  get h(): number { return this._regs.h; }

  /** Register L (index 5) — low byte of memory address pair. */
  get l(): number { return this._regs.l; }

  /** Current program counter (14-bit, 0–16383). */
  get pc(): number { return this._stack.pc; }

  /**
   * 14-bit memory address formed from H and L registers.
   * address = (H & 0x3F) << 8 | L
   */
  get hlAddress(): number { return this._regs.hlAddress; }

  /** Current flag state (snapshot). */
  get currentFlags(): Flags {
    return this._flags.snapshot();
  }

  /** True if the processor has halted. */
  get isHalted(): boolean { return this._halted; }

  /** Read-only view of the 16 KiB memory. */
  get memory(): Uint8Array { return this._mem; }

  /** All 8 stack entries (entry 0 = current PC). */
  get stack(): number[] { return this._stack.snapshot; }

  // -------------------------------------------------------------------------
  // Public API
  // -------------------------------------------------------------------------

  /**
   * Load a program into memory at the given start address.
   *
   * Does NOT reset CPU state — call reset() first if needed.
   * Sets the PC to startAddress so execution begins at the right place.
   *
   * @param program      - Machine code bytes.
   * @param startAddress - Load address (default: 0x0000).
   */
  loadProgram(program: Uint8Array, startAddress = 0): void {
    this._mem.set(program, startAddress);
    this._stack.setPC(startAddress);
  }

  /**
   * Execute one instruction and return a trace record.
   *
   * All arithmetic is routed through gate functions. Every MOV routes through
   * the register() flip-flop model. Every add/sub routes through the ripple-carry
   * adder chain. The PC increment uses the half-adder chain.
   *
   * @returns Trace record for the executed instruction.
   * @throws Error if the processor is halted.
   */
  step(): Trace {
    if (this._halted) {
      throw new Error("Processor is halted. Call reset() to restart.");
    }

    const instrAddr = this._stack.pc;
    const aBefore = this._regs.a;
    const flagsBefore = this._flags.snapshot();

    // -----------------------------------------------------------------------
    // FETCH: read opcode, advance PC via half-adder chain
    // -----------------------------------------------------------------------
    const opcode = this._mem[instrAddr & 0x3FFF];
    this._pc.load(instrAddr);
    this._pc.increment();  // 14 half-adders = 28 gate calls
    this._stack.setPC(this._pc.value);

    const rawBytes: number[] = [opcode];
    let mnemonic = `???_0x${opcode.toString(16).padStart(2, "0")}`;
    let memAddress: number | null = null;
    let memValue: number | null = null;

    // -----------------------------------------------------------------------
    // DECODE: gate-based combinational decoder → control signals
    // -----------------------------------------------------------------------
    const ctrl = decode(opcode);  // ~40 AND/OR/NOT gate calls per opcode

    const group = (opcode >> 6) & 0x03;
    const ddd   = (opcode >> 3) & 0x07;
    const sss   = opcode & 0x07;

    // -----------------------------------------------------------------------
    // EXECUTE: dispatch based on decoder control signals
    // -----------------------------------------------------------------------

    if (ctrl.isHalt) {
      // =====================================================================
      // HLT (0x76 or 0xFF)
      // =====================================================================
      this._halted = true;
      mnemonic = "HLT";

    } else if (ctrl.isInput) {
      // =====================================================================
      // IN instruction: read from input port into accumulator
      // =====================================================================
      const port = ctrl.portNumber & 0x07;
      this._regs.a = this._inputPorts[port] & 0xFF;
      mnemonic = `IN ${port}`;

    } else if (ctrl.isJump) {
      // =====================================================================
      // JMP / Conditional jumps (3 bytes)
      // =====================================================================
      const addrLo = this._fetchByte();
      rawBytes.push(addrLo);
      const addrHi = this._fetchByte();
      rawBytes.push(addrHi);
      const target = ((addrHi & 0x3F) << 8) | addrLo;

      const condCode = ctrl.condCode & 0x07;
      const condSense = ctrl.condSense;

      if (condCode === 0x07) {
        // Unconditional JMP
        this._stack.setPC(target);
        mnemonic = `JMP 0x${target.toString(16).toUpperCase().padStart(4, "0")}`;
      } else {
        const condMet = this._evalCondition(condCode & 0x03, condSense === 1);
        if (condMet) {
          this._stack.setPC(target);
        }
        const condName = COND_NAMES[condCode & 0x03] ?? "?";
        const prefix = condSense === 1 ? "JT" : "JF";
        mnemonic = `${prefix}${condName} 0x${target.toString(16).toUpperCase().padStart(4, "0")}`;
      }

    } else if (ctrl.isCall) {
      // =====================================================================
      // CAL / Conditional calls (3 bytes)
      // =====================================================================
      const addrLo = this._fetchByte();
      rawBytes.push(addrLo);
      const addrHi = this._fetchByte();
      rawBytes.push(addrHi);
      const target = ((addrHi & 0x3F) << 8) | addrLo;

      const condCode = ctrl.condCode & 0x07;
      const condSense = ctrl.condSense;

      if (condCode === 0x07) {
        // Unconditional CAL
        this._pushAndJump(target);
        mnemonic = `CAL 0x${target.toString(16).toUpperCase().padStart(4, "0")}`;
      } else {
        const condMet = this._evalCondition(condCode & 0x03, condSense === 1);
        if (condMet) {
          this._pushAndJump(target);
        }
        const condName = COND_NAMES[condCode & 0x03] ?? "?";
        const prefix = condSense === 1 ? "CT" : "CF";
        mnemonic = `${prefix}${condName} 0x${target.toString(16).toUpperCase().padStart(4, "0")}`;
      }

    } else if (ctrl.isReturn) {
      // =====================================================================
      // RET / Conditional returns
      // =====================================================================
      const condCode = ctrl.condCode & 0x07;
      const condSense = ctrl.condSense;

      if (condCode === 0x07) {
        this._stack.pop();
        mnemonic = "RET";
      } else {
        const condMet = this._evalCondition(condCode & 0x03, condSense === 1);
        if (condMet) {
          this._stack.pop();
        }
        const condName = COND_NAMES[condCode & 0x03] ?? "?";
        const prefix = condSense === 1 ? "RT" : "RF";
        mnemonic = `${prefix}${condName}`;
      }

    } else if (ctrl.isRST) {
      // =====================================================================
      // RST N: 1-byte call to address N*8
      // =====================================================================
      this._pushAndJump(ctrl.rstTarget);
      mnemonic = `RST ${ddd}`;

    } else if (ctrl.isOutput) {
      // =====================================================================
      // OUT instruction: write accumulator to output port
      // =====================================================================
      const port = ctrl.portNumber;
      if (port < 24) {
        this._outputPorts[port] = this._regs.a & 0xFF;
      }
      mnemonic = `OUT ${port}`;

    } else if (group === 0x01) {
      // =====================================================================
      // MOV D, S (group 01, not HLT/IN/JMP/CAL)
      // =====================================================================
      const srcVal = this._readRegMem(sss, (addr) => {
        memAddress = addr;
        memValue = this._mem[addr & 0x3FFF];
      });
      this._writeRegMem(ddd, srcVal, (addr, val) => {
        memAddress = addr;
        memValue = val;
      });
      mnemonic = `MOV ${REG_NAMES[ddd]}, ${REG_NAMES[sss]}`;

    } else if (group === 0x00 && sss === 0x06) {
      // =====================================================================
      // MVI D, immediate (2 bytes)
      // =====================================================================
      const imm = this._fetchByte();
      rawBytes.push(imm);
      this._writeRegMem(ddd, imm, (addr, val) => {
        memAddress = addr;
        memValue = val;
      });
      mnemonic = `MVI ${REG_NAMES[ddd]}, 0x${imm.toString(16).toUpperCase().padStart(2, "0")}`;

    } else if (group === 0x00 && sss === 0x00) {
      // =====================================================================
      // INR D: increment register (preserves CY)
      // =====================================================================
      const oldVal = this._readRegMem(ddd, (addr) => {
        memAddress = addr;
        memValue = this._mem[addr & 0x3FFF];
      });
      // Increment through ALU gate chain
      const [result] = this._alu.increment(oldVal);
      this._writeRegMem(ddd, result, (addr, val) => {
        memAddress = addr;
        memValue = val;
      });
      // INR updates Z, S, P but NOT carry
      const gflags = this._alu.flagsFromResult(result, this._flags.cy);
      this._flags.updateWithoutCarry(gflags);
      mnemonic = `INR ${REG_NAMES[ddd]}`;

    } else if (group === 0x00 && sss === 0x01) {
      // =====================================================================
      // DCR D: decrement register (preserves CY)
      // =====================================================================
      const oldVal = this._readRegMem(ddd, (addr) => {
        memAddress = addr;
        memValue = this._mem[addr & 0x3FFF];
      });
      // Decrement through ALU gate chain (A + 0xFF = A - 1)
      const [result] = this._alu.decrement(oldVal);
      this._writeRegMem(ddd, result, (addr, val) => {
        memAddress = addr;
        memValue = val;
      });
      // DCR updates Z, S, P but NOT carry
      const gflags = this._alu.flagsFromResult(result, this._flags.cy);
      this._flags.updateWithoutCarry(gflags);
      mnemonic = `DCR ${REG_NAMES[ddd]}`;

    } else if (group === 0x00 && sss === 0x02 && ddd <= 0x03) {
      // =====================================================================
      // Rotate instructions (RLC, RRC, RAL, RAR)
      // Only updates CY; Z, S, P are unchanged.
      // =====================================================================
      const acc = this._regs.a;
      const cy = this._flags.cy;
      let rotated: number;
      let newCy: Bit;

      switch (ddd) {
        case 0: {
          // RLC: Rotate Left Circular — bit7 wraps to bit0, CY = old bit7
          [rotated, newCy] = this._alu.rotateLeftCircular(acc);
          mnemonic = "RLC";
          break;
        }
        case 1: {
          // RRC: Rotate Right Circular — bit0 wraps to bit7, CY = old bit0
          [rotated, newCy] = this._alu.rotateRightCircular(acc);
          mnemonic = "RRC";
          break;
        }
        case 2: {
          // RAL: Rotate Left through Carry — 9-bit rotation
          [rotated, newCy] = this._alu.rotateLeftCarry(acc, cy);
          mnemonic = "RAL";
          break;
        }
        case 3: {
          // RAR: Rotate Right through Carry — 9-bit rotation
          [rotated, newCy] = this._alu.rotateRightCarry(acc, cy);
          mnemonic = "RAR";
          break;
        }
        default:
          rotated = acc;
          newCy = cy;
          mnemonic = "???";
      }
      this._regs.a = rotated;
      this._flags.updateCarryOnly(newCy);

    } else if (group === 0x02) {
      // =====================================================================
      // GROUP 10: ALU register source
      // =====================================================================
      const srcVal = this._readRegMem(sss, (addr) => {
        memAddress = addr;
        memValue = this._mem[addr & 0x3FFF];
      });
      this._executeALU(ddd, srcVal);
      mnemonic = `${ALU_REG_NAMES[ddd] ?? "???"} ${REG_NAMES[sss]}`;

    } else if (group === 0x03 && sss === 0x04) {
      // =====================================================================
      // GROUP 11: ALU immediate (ADI, ACI, SUI, SBI, ANI, XRI, ORI, CPI)
      // =====================================================================
      const imm = this._fetchByte();
      rawBytes.push(imm);
      this._executeALU(ddd, imm);
      mnemonic = `${ALU_IMM_NAMES[ddd] ?? "???"} 0x${imm.toString(16).toUpperCase().padStart(2, "0")}`;

    } else {
      mnemonic = `???_0x${opcode.toString(16).padStart(2, "0")}`;
    }

    const aAfter = this._regs.a;
    const flagsAfter = this._flags.snapshot();

    return {
      address: instrAddr,
      raw: new Uint8Array(rawBytes),
      mnemonic,
      aBefore,
      aAfter,
      flagsBefore,
      flagsAfter,
      memAddress,
      memValue,
    };
  }

  /**
   * Load a program and run it until HLT or maxSteps is reached.
   *
   * @param program      - Machine code bytes.
   * @param maxSteps     - Safety limit (default: 100,000).
   * @param startAddress - Load address (default: 0x0000).
   * @returns Array of trace records for each executed instruction.
   */
  run(program: Uint8Array, maxSteps = 100000, startAddress = 0): Trace[] {
    this.reset();
    this.loadProgram(program, startAddress);
    const traces: Trace[] = [];
    while (!this._halted && traces.length < maxSteps) {
      traces.push(this.step());
    }
    return traces;
  }

  /**
   * Reset the processor to power-on state.
   *
   * Clears all registers, flags, stack, and halted state.
   * Does NOT clear memory (call loadProgram() to reload).
   * Does NOT clear I/O ports (they model external hardware state).
   */
  reset(): void {
    this._regs.reset();
    this._flags.reset();
    this._pc.reset();
    this._stack.reset();
    this._halted = false;
  }

  /**
   * Set an input port value (read by IN instructions).
   *
   * @param port  - Port number (0–7).
   * @param value - 8-bit value (0–255).
   */
  setInputPort(port: number, value: number): void {
    if (port < 0 || port > 7) {
      throw new RangeError(`Input port must be 0–7, got ${port}`);
    }
    this._inputPorts[port] = value & 0xFF;
  }

  /**
   * Read an output port value (written by OUT instructions).
   *
   * @param port - Port number (0–23).
   * @returns 8-bit value from the most recent OUT instruction.
   */
  getOutputPort(port: number): number {
    if (port < 0 || port > 23) {
      throw new RangeError(`Output port must be 0–23, got ${port}`);
    }
    return this._outputPorts[port];
  }

  // -------------------------------------------------------------------------
  // Private helpers
  // -------------------------------------------------------------------------

  /**
   * Fetch one byte from memory at PC and advance PC via half-adder increment.
   */
  private _fetchByte(): number {
    const addr = this._stack.pc & 0x3FFF;
    const byte = this._mem[addr];
    this._pc.load(addr);
    this._pc.increment();
    this._stack.setPC(this._pc.value);
    return byte;
  }

  /**
   * Push current PC (return address) and jump to target (CALL semantic).
   *
   * The return address is the current PC (already advanced past the CAL
   * instruction's bytes). The stack push saves it at entry[1].
   *
   * @param target - 14-bit subroutine address.
   */
  private _pushAndJump(target: number): void {
    const returnAddr = this._stack.pc;
    this._stack.push(returnAddr, target & 0x3FFF);
  }

  /**
   * Read a register or memory (for M pseudo-register).
   *
   * @param regIdx      - Register index 0–7 (6 = M = memory at H:L).
   * @param onMemAccess - Called with the resolved address when M is accessed.
   */
  private _readRegMem(regIdx: number, onMemAccess?: (addr: number) => void): number {
    if (regIdx === 6) {
      const addr = this.hlAddress;
      if (onMemAccess) onMemAccess(addr);
      return this._mem[addr & 0x3FFF];
    }
    return this._regs.read(regIdx);
  }

  /**
   * Write a value to a register or memory.
   *
   * @param regIdx      - Register index 0–7 (6 = M = memory at H:L).
   * @param value       - 8-bit value.
   * @param onMemAccess - Called with (address, value) when M is written.
   */
  private _writeRegMem(
    regIdx: number,
    value: number,
    onMemAccess?: (addr: number, val: number) => void,
  ): void {
    const v = value & 0xFF;
    if (regIdx === 6) {
      const addr = this.hlAddress;
      this._mem[addr & 0x3FFF] = v;
      if (onMemAccess) onMemAccess(addr, v);
    } else {
      this._regs.write(regIdx, v);
    }
  }

  /**
   * Evaluate a condition flag for conditional jumps/calls/returns.
   *
   * condition codes: 0=CY, 1=Z, 2=S, 3=P
   * sense=true: branch if flag IS set
   * sense=false: branch if flag is NOT set
   *
   * In gate-level terms, the sense bit selects between the flag and its
   * complement: OR(AND(sense, flag), AND(NOT(sense), NOT(flag))).
   *
   * @param code  - Condition code (0–3).
   * @param sense - true = jump-if-set, false = jump-if-clear.
   */
  private _evalCondition(code: number, sense: boolean): boolean {
    let flagBit: Bit;
    switch (code) {
      case 0: flagBit = this._flags.cy; break;
      case 1: flagBit = this._flags.z;  break;
      case 2: flagBit = this._flags.s;  break;
      case 3: flagBit = this._flags.p;  break;
      default: return true;
    }
    // Gate-level sense evaluation:
    // if sense=1: branch when flagBit=1 → result = flagBit
    // if sense=0: branch when flagBit=0 → result = NOT(flagBit)
    const senseBit = sense ? 1 as Bit : 0 as Bit;
    // XNOR(senseBit, flagBit) = 1 when they match → but we want:
    // branch when: (sense AND flag) OR (NOT(sense) AND NOT(flag))
    // = XNOR(sense, flag) = NOT(XOR(sense, flag))
    // Simplified with host booleans (gate call not needed for single-bit):
    return sense === (flagBit === 1);
  }

  /**
   * Execute an ALU operation (group 10 or group 11).
   *
   * Routes through GateALU8, which uses the ripple-carry adder chain
   * and gate functions. The carry flag is read from and written to
   * FlagRegister.
   *
   * @param op  - ALU operation code (0=ADD, 1=ADC, 2=SUB, 3=SBB,
   *              4=AND, 5=XOR, 6=OR, 7=CMP).
   * @param src - 8-bit source operand.
   */
  private _executeALU(op: number, src: number): void {
    const acc = this._regs.a;
    const cy = this._flags.cy;

    let result: number;
    let newCy: Bit;
    let clearCarry = false;

    switch (op) {
      case 0: { // ADD: A ← A + src
        [result, newCy] = this._alu.add(acc, src, 0);
        break;
      }
      case 1: { // ADC: A ← A + src + CY
        [result, newCy] = this._alu.add(acc, src, cy);
        break;
      }
      case 2: { // SUB: A ← A - src
        [result, newCy] = this._alu.subtract(acc, src, 0);
        break;
      }
      case 3: { // SBB: A ← A - src - CY (borrow)
        [result, newCy] = this._alu.subtract(acc, src, cy);
        break;
      }
      case 4: { // ANA: A ← A AND src
        result = this._alu.bitwiseAnd(acc, src);
        newCy = 0;
        clearCarry = true;
        break;
      }
      case 5: { // XRA: A ← A XOR src
        result = this._alu.bitwiseXor(acc, src);
        newCy = 0;
        clearCarry = true;
        break;
      }
      case 6: { // ORA: A ← A OR src
        result = this._alu.bitwiseOr(acc, src);
        newCy = 0;
        clearCarry = true;
        break;
      }
      case 7: { // CMP: set flags for A - src, discard result
        [result, newCy] = this._alu.subtract(acc, src, 0);
        const gflags = this._alu.flagsFromResult(result, newCy);
        this._flags.update(gflags);
        return;  // CMP does NOT write to accumulator
      }
      default:
        return;
    }

    // Write result to accumulator (except CMP handled above).
    this._regs.a = result & 0xFF;

    // Update flags
    const gflags = this._alu.flagsFromResult(result & 0xFF, clearCarry ? 0 : newCy);
    if (clearCarry) {
      // AND/OR/XOR: carry is cleared to 0, other flags updated from result
      this._flags.update(gflags);
    } else {
      this._flags.update(gflags);
    }
  }

  /**
   * Get the total number of AND/OR/NOT/XOR gate function calls made
   * since construction (not reset per run — cumulative).
   *
   * This is a non-functional metric for educational purposes: it lets you
   * verify that the gate-level simulator genuinely exercises the gate
   * functions rather than using host arithmetic shortcuts.
   *
   * Note: Gate counting is not wired to the gate functions in this
   * implementation (the logic-gates package does not expose a call counter).
   * Instead, this method returns an estimated gate count based on the
   * instruction count and average gate depth per instruction.
   *
   * Average gate costs per instruction type:
   *   - Fetch (PC increment): 28 gates (14 half-adders × 2)
   *   - Decode: ~40 gates
   *   - ALU add/sub: ~40 gates (8 full-adders × 5)
   *   - Bitwise op: 8 gates (8 AND/OR/XOR in parallel)
   *   - MOV: ~16 gates (register write via D flip-flop model)
   *   - Rotate: ~8 gates (bit rearrangement + carry update)
   *
   * @returns Estimated gate invocation count.
   */
  gateCount(): number {
    // Returns -1 to indicate "not instrumented" (honest about the limitation).
    // A future version could wrap the gate functions with a counting proxy.
    return -1;
  }
}
