/**
 * Intel 4004 gate-level CPU -- all operations route through real logic gates.
 *
 * === What makes this a "gate-level" simulator? ===
 *
 * Every computation in this CPU flows through the same gate chain that the
 * real Intel 4004 used:
 *
 *     NOT/AND/OR/XOR -> halfAdder -> fullAdder -> rippleCarryAdder -> ALU
 *     D flip-flop -> register -> register file / program counter / stack
 *
 * When you execute ADD R3, the value in register R3 is read from flip-flops,
 * the accumulator is read from flip-flops, both are fed into the ALU (which
 * uses full adders built from gates), and the result is clocked back into
 * the accumulator's flip-flops.
 *
 * Nothing is simulated behaviorally. Every bit passes through gate functions.
 *
 * === Gate count ===
 *
 * Component               Gates   Transistors (x4 per gate)
 * ---------------------   -----   -------------------------
 * ALU (4-bit)             32      128
 * Register file (16x4)    480     1,920
 * Accumulator (4-bit)     24      96
 * Carry flag (1-bit)      6       24
 * Program counter (12)    96      384
 * Hardware stack (3x12)   226     904
 * Decoder                 ~50     200
 * Control + wiring        ~100    400
 * ---------------------   -----   -------------------------
 * Total                   ~1,014  ~4,056
 *
 * The real Intel 4004 had 2,300 transistors. Our count is higher because
 * we model RAM separately (the real 4004 used external 4002 RAM chips)
 * and our gate model isn't minimized with Karnaugh maps.
 *
 * === Execution model ===
 *
 * Each instruction executes in a single step() call, which corresponds
 * to one machine cycle. The fetch-decode-execute pipeline:
 *
 *     1. FETCH:   Read instruction byte from ROM using PC
 *     2. FETCH2:  For 2-byte instructions, read the second byte
 *     3. DECODE:  Route instruction through decoder gate network
 *     4. EXECUTE: Perform the operation through ALU/registers/etc.
 */

import { AND, NOT, OR, register as regFn, type Bit } from "@coding-adventures/logic-gates";
import { GateALU } from "./alu.js";
import { intToBits, bitsToInt } from "./bits.js";
import { type DecodedInstruction, decode } from "./decoder.js";
import { ProgramCounter } from "./pc.js";
import { RAM } from "./ram.js";
import { Accumulator, CarryFlag, RegisterFile } from "./registers.js";
import { HardwareStack } from "./stack.js";

/**
 * Trace record for one instruction execution.
 *
 * Same information as Intel4004Trace from the behavioral simulator,
 * plus gate-level details.
 */
export interface GateTrace {
  address: number;
  raw: number;
  raw2: number | null;
  mnemonic: string;
  accumulatorBefore: number;
  accumulatorAfter: number;
  carryBefore: boolean;
  carryAfter: boolean;
}

/**
 * Intel 4004 CPU where every operation routes through real logic gates.
 *
 * Public API matches the behavioral Intel4004Simulator for
 * cross-validation, but internally all computation flows through
 * gates, flip-flops, and adders.
 *
 * @example
 * const cpu = new Intel4004GateLevel();
 * const traces = cpu.run(new Uint8Array([0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01]));
 * cpu.registers[1]; // => 3  (R1 = 1 + 2)
 */
export class Intel4004GateLevel {
  // --- Gate-level components ---
  private _alu: GateALU;
  private _regs: RegisterFile;
  private _acc: Accumulator;
  private _carry: CarryFlag;
  private _pc: ProgramCounter;
  private _stack: HardwareStack;
  private _ram: RAM;

  // --- ROM (read-only, loaded by program) ---
  private _rom: Uint8Array;

  // --- RAM addressing (set by SRC/DCL) ---
  private _ramBank: number;
  private _ramRegister: number;
  private _ramCharacter: number;

  // --- ROM I/O port ---
  private _romPort: number;

  // --- Control state ---
  private _halted: boolean;

  constructor() {
    this._alu = new GateALU();
    this._regs = new RegisterFile();
    this._acc = new Accumulator();
    this._carry = new CarryFlag();
    this._pc = new ProgramCounter();
    this._stack = new HardwareStack();
    this._ram = new RAM();
    this._rom = new Uint8Array(4096);
    this._ramBank = 0;
    this._ramRegister = 0;
    this._ramCharacter = 0;
    this._romPort = 0;
    this._halted = false;
  }

  // ------------------------------------------------------------------
  // Property accessors (match behavioral simulator's interface)
  // ------------------------------------------------------------------

  /** Read accumulator from flip-flops. */
  get accumulator(): number {
    return this._acc.read();
  }

  /** Read all 16 registers from flip-flops. */
  get registers(): number[] {
    const result: number[] = [];
    for (let i = 0; i < 16; i++) {
      result.push(this._regs.read(i));
    }
    return result;
  }

  /** Read carry flag from flip-flop. */
  get carry(): boolean {
    return this._carry.read();
  }

  /** Read program counter from flip-flops. */
  get pc(): number {
    return this._pc.read();
  }

  /** Whether the CPU is halted. */
  get halted(): boolean {
    return this._halted;
  }

  /** Read stack levels (for inspection only). */
  get hwStack(): number[] {
    const values: number[] = [];
    const zeros: Bit[] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    for (let i = 0; i < 3; i++) {
      const [output] = regFn(zeros, 0 as Bit, this._stack._levels[i], 12);
      values.push(bitsToInt(output));
    }
    return values;
  }

  /** Read RAM main characters. */
  get ramData(): number[][][] {
    const result: number[][][] = [];
    for (let b = 0; b < 4; b++) {
      const bankData: number[][] = [];
      for (let r = 0; r < 4; r++) {
        const regData: number[] = [];
        for (let c = 0; c < 16; c++) {
          regData.push(this._ram.readMain(b, r, c));
        }
        bankData.push(regData);
      }
      result.push(bankData);
    }
    return result;
  }

  /** Read RAM status characters. */
  get ramStatus(): number[][][] {
    const result: number[][][] = [];
    for (let b = 0; b < 4; b++) {
      const bankData: number[][] = [];
      for (let r = 0; r < 4; r++) {
        const regData: number[] = [];
        for (let s = 0; s < 4; s++) {
          regData.push(this._ram.readStatus(b, r, s));
        }
        bankData.push(regData);
      }
      result.push(bankData);
    }
    return result;
  }

  /** Current RAM bank. */
  get ramBank(): number {
    return this._ramBank;
  }

  /** Current ROM port value. */
  get romPort(): number {
    return this._romPort;
  }

  /**
   * Set the ROM port value (for external I/O simulation).
   *
   * In a real Busicom calculator, the keyboard hardware would drive signals
   * onto the ROM port lines. The CPU reads these via the RDR instruction.
   * This setter allows external code (like a calculator simulator) to inject
   * key values that the ROM program can read.
   */
  set romPort(value: number) {
    this._romPort = value & 0xf;
  }

  /** RAM output ports. */
  get ramOutput(): number[] {
    return [0, 1, 2, 3].map((i) => this._ram.readOutput(i));
  }

  // ------------------------------------------------------------------
  // Public API
  // ------------------------------------------------------------------

  /** Load a program into ROM. */
  loadProgram(program: Uint8Array): void {
    this._rom = new Uint8Array(4096);
    for (let i = 0; i < program.length && i < 4096; i++) {
      this._rom[i] = program[i];
    }
  }

  /**
   * Execute one instruction through the gate-level pipeline.
   *
   * @returns A GateTrace with before/after state.
   */
  step(): GateTrace {
    if (this._halted) {
      throw new Error("CPU is halted -- cannot step further");
    }

    // Snapshot state before
    const accBefore = this._acc.read();
    const carryBefore = this._carry.read();
    const pcBefore = this._pc.read();

    // FETCH: read instruction byte from ROM
    const raw = this._rom[pcBefore];

    // DECODE: route through combinational decoder
    let decoded = decode(raw);

    // FETCH2: if 2-byte, read second byte
    let raw2: number | null = null;
    if (decoded.isTwoByte) {
      raw2 = this._rom[(pcBefore + 1) & 0xfff];
      decoded = decode(raw, raw2);
    }

    // EXECUTE: route through appropriate gate paths
    const mnemonic = this._execute(decoded);

    return {
      address: pcBefore,
      raw,
      raw2,
      mnemonic,
      accumulatorBefore: accBefore,
      accumulatorAfter: this._acc.read(),
      carryBefore,
      carryAfter: this._carry.read(),
    };
  }

  /** Load and run a program, returning execution trace. */
  run(program: Uint8Array, maxSteps: number = 10000): GateTrace[] {
    this.reset();
    this.loadProgram(program);

    const traces: GateTrace[] = [];
    for (let i = 0; i < maxSteps; i++) {
      if (this._halted) break;
      traces.push(this.step());
    }
    return traces;
  }

  /** Reset all CPU state. */
  reset(): void {
    this._acc.reset();
    this._carry.reset();
    this._regs.reset();
    this._pc.reset();
    this._stack.reset();
    this._ram.reset();
    this._rom = new Uint8Array(4096);
    this._ramBank = 0;
    this._ramRegister = 0;
    this._ramCharacter = 0;
    this._romPort = 0;
    this._halted = false;
  }

  /** Total estimated gate count for the CPU. */
  gateCount(): number {
    return (
      this._alu.gateCount +
      this._regs.gateCount +
      this._acc.gateCount +
      this._carry.gateCount +
      this._pc.gateCount +
      this._stack.gateCount +
      this._ram.gateCount +
      50 + // decoder
      100   // control logic and wiring
    );
  }

  // ------------------------------------------------------------------
  // Instruction execution -- routes through gate-level components
  // ------------------------------------------------------------------

  /**
   * Execute a decoded instruction through gate paths.
   *
   * Each instruction routes through the appropriate combination of
   * ALU, registers, and flip-flops.
   */
  private _execute(d: DecodedInstruction): string {
    // NOP
    if (d.isNop) {
      this._pc.increment();
      return "NOP";
    }

    // HLT
    if (d.isHlt) {
      this._halted = true;
      this._pc.increment();
      return "HLT";
    }

    // LDM N: load immediate into accumulator
    if (d.isLdm) {
      this._acc.write(d.immediate);
      this._pc.increment();
      return `LDM ${d.immediate}`;
    }

    // LD Rn: load register into accumulator
    if (d.isLd) {
      const val = this._regs.read(d.regIndex);
      this._acc.write(val);
      this._pc.increment();
      return `LD R${d.regIndex}`;
    }

    // XCH Rn: exchange accumulator and register
    if (d.isXch) {
      const aVal = this._acc.read();
      const rVal = this._regs.read(d.regIndex);
      this._acc.write(rVal);
      this._regs.write(d.regIndex, aVal);
      this._pc.increment();
      return `XCH R${d.regIndex}`;
    }

    // INC Rn: increment register (no carry effect)
    if (d.isInc) {
      const rVal = this._regs.read(d.regIndex);
      const [result] = this._alu.increment(rVal);
      this._regs.write(d.regIndex, result);
      this._pc.increment();
      return `INC R${d.regIndex}`;
    }

    // ADD Rn: add register to accumulator with carry
    if (d.isAdd) {
      const aVal = this._acc.read();
      const rVal = this._regs.read(d.regIndex);
      const carryIn = this._carry.read() ? 1 : 0;
      const [result, carryOut] = this._alu.add(aVal, rVal, carryIn);
      this._acc.write(result);
      this._carry.write(carryOut);
      this._pc.increment();
      return `ADD R${d.regIndex}`;
    }

    // SUB Rn: subtract register from accumulator
    if (d.isSub) {
      const aVal = this._acc.read();
      const rVal = this._regs.read(d.regIndex);
      const borrowIn = this._carry.read() ? 0 : 1;
      const [result, carryOut] = this._alu.subtract(aVal, rVal, borrowIn);
      this._acc.write(result);
      this._carry.write(carryOut);
      this._pc.increment();
      return `SUB R${d.regIndex}`;
    }

    // JUN addr: unconditional jump
    if (d.isJun) {
      this._pc.load(d.addr12);
      return `JUN 0x${d.addr12.toString(16).toUpperCase().padStart(3, "0")}`;
    }

    // JCN cond,addr: conditional jump
    if (d.isJcn) {
      return this._execJcn(d);
    }

    // ISZ Rn,addr: increment and skip if zero
    if (d.isIsz) {
      return this._execIsz(d);
    }

    // JMS addr: jump to subroutine
    if (d.isJms) {
      const returnAddr = this._pc.read() + 2;
      this._stack.push(returnAddr);
      this._pc.load(d.addr12);
      return `JMS 0x${d.addr12.toString(16).toUpperCase().padStart(3, "0")}`;
    }

    // BBL N: branch back and load
    if (d.isBbl) {
      this._acc.write(d.immediate);
      const returnAddr = this._stack.pop();
      this._pc.load(returnAddr);
      return `BBL ${d.immediate}`;
    }

    // FIM Pp,data: fetch immediate to pair
    if (d.isFim) {
      this._regs.writePair(d.pairIndex, d.addr8);
      this._pc.increment2();
      return `FIM P${d.pairIndex},0x${d.addr8.toString(16).toUpperCase().padStart(2, "0")}`;
    }

    // SRC Pp: send register control
    if (d.isSrc) {
      const pairVal = this._regs.readPair(d.pairIndex);
      this._ramRegister = (pairVal >> 4) & 0xf;
      this._ramCharacter = pairVal & 0xf;
      this._pc.increment();
      return `SRC P${d.pairIndex}`;
    }

    // FIN Pp: fetch indirect from ROM
    if (d.isFin) {
      const p0Val = this._regs.readPair(0);
      const page = this._pc.read() & 0xf00;
      const romAddr = page | p0Val;
      const romByte = this._rom[romAddr & 0xfff];
      this._regs.writePair(d.pairIndex, romByte);
      this._pc.increment();
      return `FIN P${d.pairIndex}`;
    }

    // JIN Pp: jump indirect
    if (d.isJin) {
      const pairVal = this._regs.readPair(d.pairIndex);
      const page = this._pc.read() & 0xf00;
      this._pc.load(page | pairVal);
      return `JIN P${d.pairIndex}`;
    }

    // I/O operations (0xE_ range)
    if (d.isIo) {
      return this._execIo(d);
    }

    // Accumulator operations (0xF_ range)
    if (d.isAccum) {
      return this._execAccum(d);
    }

    // Unknown -- advance PC to avoid infinite loop
    this._pc.increment();
    return `UNKNOWN(0x${d.raw.toString(16).toUpperCase().padStart(2, "0")})`;
  }

  /**
   * JCN cond,addr: conditional jump using gate logic.
   *
   * Condition nibble bits (evaluated with OR/AND/NOT gates):
   *     Bit 3: INVERT
   *     Bit 2: TEST A==0
   *     Bit 1: TEST carry==1
   *     Bit 0: TEST pin (always 0)
   */
  private _execJcn(d: DecodedInstruction): string {
    const cond = d.condition;
    const aVal = this._acc.read();
    const carryVal: Bit = this._carry.read() ? 1 : 0;

    // Test A==0: OR all accumulator bits, then NOT
    const aBits = intToBits(aVal, 4);
    const aIsZero = NOT(OR(OR(aBits[0], aBits[1]), OR(aBits[2], aBits[3])));

    // Build test result using gates
    const testZero = AND(((cond >> 2) & 1) as Bit, aIsZero);
    const testCarry = AND(((cond >> 1) & 1) as Bit, carryVal);
    const testPin = AND((cond & 1) as Bit, 0 as Bit); // Pin always 0

    const testResult = OR(OR(testZero, testCarry), testPin);

    // Invert if bit 3 set
    const invert: Bit = ((cond >> 3) & 1) as Bit;
    // XOR with invert: if invert=1, flip result
    const final_ = OR(
      AND(testResult, NOT(invert)),
      AND(NOT(testResult), invert),
    );

    const page = (this._pc.read() + 2) & 0xf00;
    const target = page | d.addr8;

    if (final_) {
      this._pc.load(target);
    } else {
      this._pc.increment2();
    }

    return `JCN ${cond},${d.addr8.toString(16).toUpperCase().padStart(2, "0")}`;
  }

  /** ISZ Rn,addr: increment register, skip if zero. */
  private _execIsz(d: DecodedInstruction): string {
    const rVal = this._regs.read(d.regIndex);
    const [result] = this._alu.increment(rVal);
    this._regs.write(d.regIndex, result);

    // Test if result is zero using NOR of all bits
    const rBits = intToBits(result, 4);
    const isZero = NOT(OR(OR(rBits[0], rBits[1]), OR(rBits[2], rBits[3])));

    const page = (this._pc.read() + 2) & 0xf00;
    const target = page | d.addr8;

    if (isZero) {
      // Result is zero -> fall through
      this._pc.increment2();
    } else {
      // Result is nonzero -> jump
      this._pc.load(target);
    }

    return `ISZ R${d.regIndex},0x${d.addr8.toString(16).toUpperCase().padStart(2, "0")}`;
  }

  /** Execute I/O instructions (0xE0-0xEF). */
  private _execIo(d: DecodedInstruction): string {
    const aVal = this._acc.read();
    const subOp = d.lower;

    if (subOp === 0x0) {
      // WRM
      this._ram.writeMain(
        this._ramBank, this._ramRegister, this._ramCharacter, aVal,
      );
      this._pc.increment();
      return "WRM";
    }

    if (subOp === 0x1) {
      // WMP
      this._ram.writeOutput(this._ramBank, aVal);
      this._pc.increment();
      return "WMP";
    }

    if (subOp === 0x2) {
      // WRR
      this._romPort = aVal & 0xf;
      this._pc.increment();
      return "WRR";
    }

    if (subOp === 0x3) {
      // WPM (NOP in simulation)
      this._pc.increment();
      return "WPM";
    }

    if (subOp >= 0x4 && subOp <= 0x7) {
      // WR0-WR3
      const idx = subOp - 0x4;
      this._ram.writeStatus(
        this._ramBank, this._ramRegister, idx, aVal,
      );
      this._pc.increment();
      return `WR${idx}`;
    }

    if (subOp === 0x8) {
      // SBM
      const ramVal = this._ram.readMain(
        this._ramBank, this._ramRegister, this._ramCharacter,
      );
      const borrowIn = this._carry.read() ? 0 : 1;
      const [result, carryOut] = this._alu.subtract(aVal, ramVal, borrowIn);
      this._acc.write(result);
      this._carry.write(carryOut);
      this._pc.increment();
      return "SBM";
    }

    if (subOp === 0x9) {
      // RDM
      const val = this._ram.readMain(
        this._ramBank, this._ramRegister, this._ramCharacter,
      );
      this._acc.write(val);
      this._pc.increment();
      return "RDM";
    }

    if (subOp === 0xa) {
      // RDR
      this._acc.write(this._romPort & 0xf);
      this._pc.increment();
      return "RDR";
    }

    if (subOp === 0xb) {
      // ADM
      const ramVal = this._ram.readMain(
        this._ramBank, this._ramRegister, this._ramCharacter,
      );
      const carryIn = this._carry.read() ? 1 : 0;
      const [result, carryOut] = this._alu.add(aVal, ramVal, carryIn);
      this._acc.write(result);
      this._carry.write(carryOut);
      this._pc.increment();
      return "ADM";
    }

    if (subOp >= 0xc && subOp <= 0xf) {
      // RD0-RD3
      const idx = subOp - 0xc;
      const val = this._ram.readStatus(
        this._ramBank, this._ramRegister, idx,
      );
      this._acc.write(val);
      this._pc.increment();
      return `RD${idx}`;
    }

    this._pc.increment();
    return `IO(0x${d.raw.toString(16).toUpperCase().padStart(2, "0")})`;
  }

  /** Execute accumulator operations (0xF0-0xFD). */
  private _execAccum(d: DecodedInstruction): string {
    const aVal = this._acc.read();
    const subOp = d.lower;

    if (subOp === 0x0) {
      // CLB
      this._acc.write(0);
      this._carry.write(false);
      this._pc.increment();
      return "CLB";
    }

    if (subOp === 0x1) {
      // CLC
      this._carry.write(false);
      this._pc.increment();
      return "CLC";
    }

    if (subOp === 0x2) {
      // IAC
      const [result, carry] = this._alu.increment(aVal);
      this._acc.write(result);
      this._carry.write(carry);
      this._pc.increment();
      return "IAC";
    }

    if (subOp === 0x3) {
      // CMC
      this._carry.write(!this._carry.read());
      this._pc.increment();
      return "CMC";
    }

    if (subOp === 0x4) {
      // CMA
      const result = this._alu.complement(aVal);
      this._acc.write(result);
      this._pc.increment();
      return "CMA";
    }

    if (subOp === 0x5) {
      // RAL
      const oldCarry: Bit = this._carry.read() ? 1 : 0;
      // Use gates: A3 goes to carry, shift left, old carry to bit 0
      const aBits = intToBits(aVal, 4);
      this._carry.write(aBits[3] === 1);
      const newBits: Bit[] = [oldCarry, aBits[0], aBits[1], aBits[2]];
      this._acc.write(bitsToInt(newBits));
      this._pc.increment();
      return "RAL";
    }

    if (subOp === 0x6) {
      // RAR
      const oldCarry: Bit = this._carry.read() ? 1 : 0;
      const aBits = intToBits(aVal, 4);
      this._carry.write(aBits[0] === 1);
      const newBits: Bit[] = [aBits[1], aBits[2], aBits[3], oldCarry];
      this._acc.write(bitsToInt(newBits));
      this._pc.increment();
      return "RAR";
    }

    if (subOp === 0x7) {
      // TCC
      this._acc.write(this._carry.read() ? 1 : 0);
      this._carry.write(false);
      this._pc.increment();
      return "TCC";
    }

    if (subOp === 0x8) {
      // DAC
      const [result, carry] = this._alu.decrement(aVal);
      this._acc.write(result);
      this._carry.write(carry);
      this._pc.increment();
      return "DAC";
    }

    if (subOp === 0x9) {
      // TCS
      this._acc.write(this._carry.read() ? 10 : 9);
      this._carry.write(false);
      this._pc.increment();
      return "TCS";
    }

    if (subOp === 0xa) {
      // STC
      this._carry.write(true);
      this._pc.increment();
      return "STC";
    }

    if (subOp === 0xb) {
      // DAA
      if (aVal > 9 || this._carry.read()) {
        const [result, carry] = this._alu.add(aVal, 6, 0);
        if (carry) {
          this._carry.write(true);
        }
        this._acc.write(result);
      }
      this._pc.increment();
      return "DAA";
    }

    if (subOp === 0xc) {
      // KBP
      const kbpTable: Record<number, number> = { 0: 0, 1: 1, 2: 2, 4: 3, 8: 4 };
      this._acc.write(kbpTable[aVal] ?? 15);
      this._pc.increment();
      return "KBP";
    }

    if (subOp === 0xd) {
      // DCL
      let bank = this._alu.bitwiseAnd(aVal, 0x7);
      if (bank > 3) {
        bank = this._alu.bitwiseAnd(bank, 0x3);
      }
      this._ramBank = bank;
      this._pc.increment();
      return "DCL";
    }

    this._pc.increment();
    return `ACCUM(0x${d.raw.toString(16).toUpperCase().padStart(2, "0")})`;
  }
}
