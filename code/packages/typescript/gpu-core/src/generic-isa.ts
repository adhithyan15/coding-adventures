/**
 * GenericISA -- a simplified, educational instruction set.
 *
 * === What is this? ===
 *
 * This is the default InstructionSet implementation -- a vendor-neutral ISA
 * designed for teaching, not for matching any real hardware. It proves that
 * the pluggable ISA design works: if you can implement GenericISA, you can
 * implement NVIDIA PTX, AMD GCN, Intel Xe, or ARM Mali the same way.
 *
 * === How it works ===
 *
 * The GenericISA.execute() method is a big switch statement. For each
 * opcode, it:
 * 1. Reads source registers
 * 2. Calls the appropriate fp-arithmetic function
 * 3. Writes the result to the destination register
 * 4. Returns an ExecuteResult describing what happened
 *
 *     FADD R2, R0, R1:
 *         a = registers.read(R0)          // read 3.14
 *         b = registers.read(R1)          // read 2.71
 *         result = fpAdd(a, b)            // 3.14 + 2.71 = 5.85
 *         registers.write(R2, result)     // store in R2
 *         return ExecuteResult("R2 = R0 + R1 = 3.14 + 2.71 = 5.85", ...)
 *
 * === Future ISAs follow the same pattern ===
 *
 *     class PTXISA implements InstructionSet {
 *         execute(instruction, registers, memory) {
 *             switch (instruction.opcode) {
 *                 case PTXOp.ADD_F32:    // same as FADD but with PTX naming
 *                 case PTXOp.FMA_RN_F32: // same as FFMA but with PTX naming
 *             }
 *         }
 *     }
 *
 * The GPUCore doesn't care which ISA is plugged in -- it just calls
 * isa.execute() and processes the ExecuteResult.
 */

import {
  bitsToFloat,
  fpAbs,
  fpAdd,
  fpCompare,
  fpFma,
  fpMul,
  fpNeg,
  fpSub,
} from "@coding-adventures/fp-arithmetic";

import type { LocalMemory } from "./memory.js";
import type { Instruction } from "./opcodes.js";
import { Opcode } from "./opcodes.js";
import type { ExecuteResult, InstructionSet } from "./protocols.js";
import { makeExecuteResult } from "./protocols.js";
import type { FPRegisterFile } from "./registers.js";

export class GenericISA implements InstructionSet {
  /**
   * A simplified, educational instruction set for GPU cores.
   *
   * This ISA is not tied to any vendor -- it's a teaching tool. It has
   * 16 opcodes covering arithmetic, memory, data movement, and control
   * flow. Any floating-point program can be expressed with these.
   *
   * To use a different ISA, create a class with the same execute() method
   * signature and pass it to new GPUCore({ isa: yourISA }).
   */

  get name(): string {
    return "Generic";
  }

  /**
   * Execute a single instruction.
   *
   * This is the heart of the ISA -- a dispatch table that maps opcodes
   * to their implementations. Each case reads operands, performs the
   * operation, writes results, and returns a trace description.
   */
  execute(
    instruction: Instruction,
    registers: FPRegisterFile,
    memory: LocalMemory,
  ): ExecuteResult {
    switch (instruction.opcode) {
      // --- Floating-point arithmetic ---
      case Opcode.FADD:
        return this._execFadd(instruction, registers);
      case Opcode.FSUB:
        return this._execFsub(instruction, registers);
      case Opcode.FMUL:
        return this._execFmul(instruction, registers);
      case Opcode.FFMA:
        return this._execFfma(instruction, registers);
      case Opcode.FNEG:
        return this._execFneg(instruction, registers);
      case Opcode.FABS:
        return this._execFabs(instruction, registers);

      // --- Memory ---
      case Opcode.LOAD:
        return this._execLoad(instruction, registers, memory);
      case Opcode.STORE:
        return this._execStore(instruction, registers, memory);

      // --- Data movement ---
      case Opcode.MOV:
        return this._execMov(instruction, registers);
      case Opcode.LIMM:
        return this._execLimm(instruction, registers);

      // --- Control flow ---
      case Opcode.BEQ:
        return this._execBeq(instruction, registers);
      case Opcode.BLT:
        return this._execBlt(instruction, registers);
      case Opcode.BNE:
        return this._execBne(instruction, registers);
      case Opcode.JMP:
        return this._execJmp(instruction);
      case Opcode.NOP:
        return makeExecuteResult({ description: "No operation" });
      case Opcode.HALT:
        return makeExecuteResult({ description: "Halted", halted: true });
      default:
        throw new Error(`Unknown opcode: ${instruction.opcode}`);
    }
  }

  // --- Arithmetic implementations ---

  /** FADD Rd, Rs1, Rs2 -> Rd = Rs1 + Rs2. */
  private _execFadd(inst: Instruction, regs: FPRegisterFile): ExecuteResult {
    const a = regs.read(inst.rs1);
    const b = regs.read(inst.rs2);
    const result = fpAdd(a, b);
    regs.write(inst.rd, result);
    const aF = bitsToFloat(a);
    const bF = bitsToFloat(b);
    const rF = bitsToFloat(result);
    return makeExecuteResult({
      description: `R${inst.rd} = R${inst.rs1} + R${inst.rs2} = ${aF} + ${bF} = ${rF}`,
      registersChanged: { [`R${inst.rd}`]: rF },
    });
  }

  /** FSUB Rd, Rs1, Rs2 -> Rd = Rs1 - Rs2. */
  private _execFsub(inst: Instruction, regs: FPRegisterFile): ExecuteResult {
    const a = regs.read(inst.rs1);
    const b = regs.read(inst.rs2);
    const result = fpSub(a, b);
    regs.write(inst.rd, result);
    const aF = bitsToFloat(a);
    const bF = bitsToFloat(b);
    const rF = bitsToFloat(result);
    return makeExecuteResult({
      description: `R${inst.rd} = R${inst.rs1} - R${inst.rs2} = ${aF} - ${bF} = ${rF}`,
      registersChanged: { [`R${inst.rd}`]: rF },
    });
  }

  /** FMUL Rd, Rs1, Rs2 -> Rd = Rs1 * Rs2. */
  private _execFmul(inst: Instruction, regs: FPRegisterFile): ExecuteResult {
    const a = regs.read(inst.rs1);
    const b = regs.read(inst.rs2);
    const result = fpMul(a, b);
    regs.write(inst.rd, result);
    const aF = bitsToFloat(a);
    const bF = bitsToFloat(b);
    const rF = bitsToFloat(result);
    return makeExecuteResult({
      description: `R${inst.rd} = R${inst.rs1} * R${inst.rs2} = ${aF} * ${bF} = ${rF}`,
      registersChanged: { [`R${inst.rd}`]: rF },
    });
  }

  /** FFMA Rd, Rs1, Rs2, Rs3 -> Rd = Rs1 * Rs2 + Rs3. */
  private _execFfma(inst: Instruction, regs: FPRegisterFile): ExecuteResult {
    const a = regs.read(inst.rs1);
    const b = regs.read(inst.rs2);
    const c = regs.read(inst.rs3);
    const result = fpFma(a, b, c);
    regs.write(inst.rd, result);
    const aF = bitsToFloat(a);
    const bF = bitsToFloat(b);
    const cF = bitsToFloat(c);
    const rF = bitsToFloat(result);
    return makeExecuteResult({
      description: `R${inst.rd} = R${inst.rs1} * R${inst.rs2} + R${inst.rs3} = ${aF} * ${bF} + ${cF} = ${rF}`,
      registersChanged: { [`R${inst.rd}`]: rF },
    });
  }

  /** FNEG Rd, Rs1 -> Rd = -Rs1. */
  private _execFneg(inst: Instruction, regs: FPRegisterFile): ExecuteResult {
    const a = regs.read(inst.rs1);
    const result = fpNeg(a);
    regs.write(inst.rd, result);
    const aF = bitsToFloat(a);
    const rF = bitsToFloat(result);
    return makeExecuteResult({
      description: `R${inst.rd} = -R${inst.rs1} = -${aF} = ${rF}`,
      registersChanged: { [`R${inst.rd}`]: rF },
    });
  }

  /** FABS Rd, Rs1 -> Rd = |Rs1|. */
  private _execFabs(inst: Instruction, regs: FPRegisterFile): ExecuteResult {
    const a = regs.read(inst.rs1);
    const result = fpAbs(a);
    regs.write(inst.rd, result);
    const aF = bitsToFloat(a);
    const rF = bitsToFloat(result);
    return makeExecuteResult({
      description: `R${inst.rd} = |R${inst.rs1}| = |${aF}| = ${rF}`,
      registersChanged: { [`R${inst.rd}`]: rF },
    });
  }

  // --- Memory implementations ---

  /** LOAD Rd, [Rs1+imm] -> Rd = Mem[Rs1 + immediate]. */
  private _execLoad(
    inst: Instruction,
    regs: FPRegisterFile,
    memory: LocalMemory,
  ): ExecuteResult {
    const base = bitsToFloat(regs.read(inst.rs1));
    const address = Math.trunc(base + inst.immediate);
    const value = memory.loadFloat(address, regs.fmt);
    regs.write(inst.rd, value);
    const valF = bitsToFloat(value);
    return makeExecuteResult({
      description: `R${inst.rd} = Mem[R${inst.rs1}+${inst.immediate}] = Mem[${address}] = ${valF}`,
      registersChanged: { [`R${inst.rd}`]: valF },
    });
  }

  /** STORE [Rs1+imm], Rs2 -> Mem[Rs1 + immediate] = Rs2. */
  private _execStore(
    inst: Instruction,
    regs: FPRegisterFile,
    memory: LocalMemory,
  ): ExecuteResult {
    const base = bitsToFloat(regs.read(inst.rs1));
    const address = Math.trunc(base + inst.immediate);
    const value = regs.read(inst.rs2);
    memory.storeFloat(address, value);
    const valF = bitsToFloat(value);
    return makeExecuteResult({
      description: `Mem[R${inst.rs1}+${inst.immediate}] = R${inst.rs2} -> Mem[${address}] = ${valF}`,
      memoryChanged: { [address]: valF },
    });
  }

  // --- Data movement implementations ---

  /** MOV Rd, Rs1 -> Rd = Rs1. */
  private _execMov(inst: Instruction, regs: FPRegisterFile): ExecuteResult {
    const value = regs.read(inst.rs1);
    regs.write(inst.rd, value);
    const valF = bitsToFloat(value);
    return makeExecuteResult({
      description: `R${inst.rd} = R${inst.rs1} = ${valF}`,
      registersChanged: { [`R${inst.rd}`]: valF },
    });
  }

  /** LIMM Rd, immediate -> Rd = float literal. */
  private _execLimm(inst: Instruction, regs: FPRegisterFile): ExecuteResult {
    regs.writeFloat(inst.rd, inst.immediate);
    return makeExecuteResult({
      description: `R${inst.rd} = ${inst.immediate}`,
      registersChanged: { [`R${inst.rd}`]: inst.immediate },
    });
  }

  // --- Control flow implementations ---

  /** BEQ Rs1, Rs2, offset -> if Rs1 == Rs2: PC += offset. */
  private _execBeq(inst: Instruction, regs: FPRegisterFile): ExecuteResult {
    const cmp = fpCompare(regs.read(inst.rs1), regs.read(inst.rs2));
    const taken = cmp === 0;
    const offset = taken ? Math.trunc(inst.immediate) : 1;
    const aF = bitsToFloat(regs.read(inst.rs1));
    const bF = bitsToFloat(regs.read(inst.rs2));
    return makeExecuteResult({
      description: `BEQ R${inst.rs1}(${aF}) == R${inst.rs2}(${bF})? ${taken ? "Yes -> branch" : "No -> fall through"}`,
      nextPcOffset: offset,
    });
  }

  /** BLT Rs1, Rs2, offset -> if Rs1 < Rs2: PC += offset. */
  private _execBlt(inst: Instruction, regs: FPRegisterFile): ExecuteResult {
    const cmp = fpCompare(regs.read(inst.rs1), regs.read(inst.rs2));
    const taken = cmp < 0;
    const offset = taken ? Math.trunc(inst.immediate) : 1;
    const aF = bitsToFloat(regs.read(inst.rs1));
    const bF = bitsToFloat(regs.read(inst.rs2));
    return makeExecuteResult({
      description: `BLT R${inst.rs1}(${aF}) < R${inst.rs2}(${bF})? ${taken ? "Yes -> branch" : "No -> fall through"}`,
      nextPcOffset: offset,
    });
  }

  /** BNE Rs1, Rs2, offset -> if Rs1 != Rs2: PC += offset. */
  private _execBne(inst: Instruction, regs: FPRegisterFile): ExecuteResult {
    const cmp = fpCompare(regs.read(inst.rs1), regs.read(inst.rs2));
    const taken = cmp !== 0;
    const offset = taken ? Math.trunc(inst.immediate) : 1;
    const aF = bitsToFloat(regs.read(inst.rs1));
    const bF = bitsToFloat(regs.read(inst.rs2));
    return makeExecuteResult({
      description: `BNE R${inst.rs1}(${aF}) != R${inst.rs2}(${bF})? ${taken ? "Yes -> branch" : "No -> fall through"}`,
      nextPcOffset: offset,
    });
  }

  /** JMP target -> PC = target (absolute jump). */
  private _execJmp(inst: Instruction): ExecuteResult {
    const target = Math.trunc(inst.immediate);
    return makeExecuteResult({
      description: `Jump to PC=${target}`,
      nextPcOffset: target,
      absoluteJump: true,
    });
  }
}
