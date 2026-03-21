/**
 * RISC-V RV32I Simulator -- full base integer ISA with M-mode extensions.
 *
 * Supports all 37 RV32I instructions plus M-mode privileged extensions:
 *   - Arithmetic: add, sub, addi, slt, sltu, slti, sltiu, and, or, xor, andi, ori, xori
 *   - Shifts: sll, srl, sra, slli, srli, srai
 *   - Loads: lb, lh, lw, lbu, lhu
 *   - Stores: sb, sh, sw
 *   - Branches: beq, bne, blt, bge, bltu, bgeu
 *   - Jumps: jal, jalr
 *   - Upper immediates: lui, auipc
 *   - System: ecall, mret, csrrw, csrrs, csrrc
 */

import {
  CPU,
  type InstructionDecoder,
  type InstructionExecutor,
  type DecodeResult,
  type ExecuteResult,
  type PipelineTrace,
  type RegisterFile,
  type Memory,
} from "@coding-adventures/cpu-simulator";

import {
  OpcodeLoad, OpcodeStore, OpcodeBranch, OpcodeJAL, OpcodeJALR,
  OpcodeLUI, OpcodeAUIPC, OpcodeOpImm, OpcodeOp, OpcodeSystem,
  Funct3ADDI, Funct3SLTI, Funct3SLTIU, Funct3XORI, Funct3ORI, Funct3ANDI,
  Funct3SLLI, Funct3SRLI,
  Funct3ADD, Funct3SLL, Funct3SLT, Funct3SLTU, Funct3XOR, Funct3SRL, Funct3OR, Funct3AND,
  Funct7Normal, Funct7Alt, Funct7MRET,
  Funct3LB, Funct3LH, Funct3LW, Funct3LBU, Funct3LHU,
  Funct3SB, Funct3SH, Funct3SW,
  Funct3BEQ, Funct3BNE, Funct3BLT, Funct3BGE, Funct3BLTU, Funct3BGEU,
  Funct3PRIV, Funct3CSRRW, Funct3CSRRS, Funct3CSRRC,
} from "./opcodes.js";

import {
  CSRFile, CSR_MSTATUS, CSR_MTVEC, CSR_MEPC, CSR_MCAUSE,
  MIE, CAUSE_ECALL_MMODE,
} from "./csr.js";

// Re-export for convenience
export { CSRFile } from "./csr.js";
export {
  CSR_MSTATUS, CSR_MTVEC, CSR_MEPC, CSR_MCAUSE, MIE, CAUSE_ECALL_MMODE,
} from "./csr.js";

// === Helper: sign-extend from N bits ===
function signExtend(value: number, bits: number): number {
  const mask = 1 << (bits - 1);
  return ((value ^ mask) - mask) | 0;
}

// === Helper: to int32 (signed interpretation) ===
function toI32(v: number): number { return v | 0; }
function toU32(v: number): number { return v >>> 0; }

// =========================================================================
// Decoder
// =========================================================================

export class RiscVDecoder implements InstructionDecoder {
  decode(rawInstruction: number, pc: number): DecodeResult {
    const raw = rawInstruction >>> 0;
    const opcode = raw & 0x7f;

    switch (opcode) {
      case OpcodeOpImm: return this.decodeOpImm(raw);
      case OpcodeOp: return this.decodeRType(raw);
      case OpcodeLoad: return this.decodeLoad(raw);
      case OpcodeStore: return this.decodeSType(raw);
      case OpcodeBranch: return this.decodeBType(raw);
      case OpcodeJAL: return this.decodeJType(raw);
      case OpcodeJALR: return this.decodeJALR(raw);
      case OpcodeLUI: return this.decodeUType(raw, "lui");
      case OpcodeAUIPC: return this.decodeUType(raw, "auipc");
      case OpcodeSystem: return this.decodeSystem(raw);
      default:
        return {
          mnemonic: `UNKNOWN(0x${opcode.toString(16).padStart(2, "0")})`,
          fields: { opcode },
          rawInstruction: raw,
        };
    }
  }

  private decodeOpImm(raw: number): DecodeResult {
    const rd = (raw >>> 7) & 0x1f;
    const funct3 = (raw >>> 12) & 0x7;
    const rs1 = (raw >>> 15) & 0x1f;
    let imm = raw >>> 20;
    if (imm & 0x800) imm -= 0x1000;

    const mnemonics: Record<number, string> = {
      [Funct3ADDI]: "addi", [Funct3SLTI]: "slti", [Funct3SLTIU]: "sltiu",
      [Funct3XORI]: "xori", [Funct3ORI]: "ori", [Funct3ANDI]: "andi",
    };
    let mnemonic = mnemonics[funct3] ?? `opimm(f3=${funct3})`;

    if (funct3 === Funct3SLLI) { mnemonic = "slli"; imm = imm & 0x1f; }
    if (funct3 === Funct3SRLI) {
      const funct7 = (raw >>> 25) & 0x7f;
      mnemonic = funct7 === Funct7Alt ? "srai" : "srli";
      imm = imm & 0x1f;
    }

    return { mnemonic, fields: { rd, rs1, imm, funct3 }, rawInstruction: raw };
  }

  private decodeRType(raw: number): DecodeResult {
    const rd = (raw >>> 7) & 0x1f;
    const funct3 = (raw >>> 12) & 0x7;
    const rs1 = (raw >>> 15) & 0x1f;
    const rs2 = (raw >>> 20) & 0x1f;
    const funct7 = (raw >>> 25) & 0x7f;

    const key = funct3 * 256 + funct7;
    const map: Record<number, string> = {
      [Funct3ADD * 256 + Funct7Normal]: "add",
      [Funct3ADD * 256 + Funct7Alt]: "sub",
      [Funct3SLL * 256 + Funct7Normal]: "sll",
      [Funct3SLT * 256 + Funct7Normal]: "slt",
      [Funct3SLTU * 256 + Funct7Normal]: "sltu",
      [Funct3XOR * 256 + Funct7Normal]: "xor",
      [Funct3SRL * 256 + Funct7Normal]: "srl",
      [Funct3SRL * 256 + Funct7Alt]: "sra",
      [Funct3OR * 256 + Funct7Normal]: "or",
      [Funct3AND * 256 + Funct7Normal]: "and",
    };
    const mnemonic = map[key] ?? `r_op(f3=${funct3},f7=${funct7})`;

    return { mnemonic, fields: { rd, rs1, rs2, funct3, funct7 }, rawInstruction: raw };
  }

  private decodeLoad(raw: number): DecodeResult {
    const rd = (raw >>> 7) & 0x1f;
    const funct3 = (raw >>> 12) & 0x7;
    const rs1 = (raw >>> 15) & 0x1f;
    let imm = raw >>> 20;
    if (imm & 0x800) imm -= 0x1000;

    const mnemonics: Record<number, string> = {
      [Funct3LB]: "lb", [Funct3LH]: "lh", [Funct3LW]: "lw",
      [Funct3LBU]: "lbu", [Funct3LHU]: "lhu",
    };
    const mnemonic = mnemonics[funct3] ?? `load(f3=${funct3})`;
    return { mnemonic, fields: { rd, rs1, imm, funct3 }, rawInstruction: raw };
  }

  private decodeSType(raw: number): DecodeResult {
    const funct3 = (raw >>> 12) & 0x7;
    const rs1 = (raw >>> 15) & 0x1f;
    const rs2 = (raw >>> 20) & 0x1f;
    const immLow = (raw >>> 7) & 0x1f;
    const immHigh = (raw >>> 25) & 0x7f;
    let imm = (immHigh << 5) | immLow;
    if (imm & 0x800) imm -= 0x1000;

    const mnemonics: Record<number, string> = {
      [Funct3SB]: "sb", [Funct3SH]: "sh", [Funct3SW]: "sw",
    };
    const mnemonic = mnemonics[funct3] ?? `store(f3=${funct3})`;
    return { mnemonic, fields: { rs1, rs2, imm, funct3 }, rawInstruction: raw };
  }

  private decodeBType(raw: number): DecodeResult {
    const funct3 = (raw >>> 12) & 0x7;
    const rs1 = (raw >>> 15) & 0x1f;
    const rs2 = (raw >>> 20) & 0x1f;
    const imm12 = (raw >>> 31) & 0x1;
    const imm11 = (raw >>> 7) & 0x1;
    const imm10_5 = (raw >>> 25) & 0x3f;
    const imm4_1 = (raw >>> 8) & 0xf;
    let imm = (imm12 << 12) | (imm11 << 11) | (imm10_5 << 5) | (imm4_1 << 1);
    if (imm & 0x1000) imm -= 0x2000;

    const mnemonics: Record<number, string> = {
      [Funct3BEQ]: "beq", [Funct3BNE]: "bne", [Funct3BLT]: "blt",
      [Funct3BGE]: "bge", [Funct3BLTU]: "bltu", [Funct3BGEU]: "bgeu",
    };
    const mnemonic = mnemonics[funct3] ?? `branch(f3=${funct3})`;
    return { mnemonic, fields: { rs1, rs2, imm, funct3 }, rawInstruction: raw };
  }

  private decodeJType(raw: number): DecodeResult {
    const rd = (raw >>> 7) & 0x1f;
    const imm20 = (raw >>> 31) & 0x1;
    const imm10_1 = (raw >>> 21) & 0x3ff;
    const imm11 = (raw >>> 20) & 0x1;
    const imm19_12 = (raw >>> 12) & 0xff;
    let imm = (imm20 << 20) | (imm19_12 << 12) | (imm11 << 11) | (imm10_1 << 1);
    if (imm & 0x100000) imm -= 0x200000;
    return { mnemonic: "jal", fields: { rd, imm }, rawInstruction: raw };
  }

  private decodeJALR(raw: number): DecodeResult {
    const rd = (raw >>> 7) & 0x1f;
    const rs1 = (raw >>> 15) & 0x1f;
    let imm = raw >>> 20;
    if (imm & 0x800) imm -= 0x1000;
    return { mnemonic: "jalr", fields: { rd, rs1, imm }, rawInstruction: raw };
  }

  private decodeUType(raw: number, mnemonic: string): DecodeResult {
    const rd = (raw >>> 7) & 0x1f;
    let imm = raw >>> 12;
    if (imm & 0x80000) imm -= 0x100000;
    return { mnemonic, fields: { rd, imm }, rawInstruction: raw };
  }

  private decodeSystem(raw: number): DecodeResult {
    const funct3 = (raw >>> 12) & 0x7;
    if (funct3 === Funct3PRIV) {
      const funct7 = (raw >>> 25) & 0x7f;
      if (funct7 === Funct7MRET) return { mnemonic: "mret", fields: { funct7 }, rawInstruction: raw };
      return { mnemonic: "ecall", fields: { funct7 }, rawInstruction: raw };
    }
    const rd = (raw >>> 7) & 0x1f;
    const rs1 = (raw >>> 15) & 0x1f;
    const csr = (raw >>> 20) & 0xfff;
    const mnemonics: Record<number, string> = {
      [Funct3CSRRW]: "csrrw", [Funct3CSRRS]: "csrrs", [Funct3CSRRC]: "csrrc",
    };
    const mnemonic = mnemonics[funct3] ?? `system(f3=${funct3})`;
    return { mnemonic, fields: { rd, rs1, csr, funct3 }, rawInstruction: raw };
  }
}

// =========================================================================
// Executor
// =========================================================================

export class RiscVExecutor implements InstructionExecutor {
  csr: CSRFile | null = null;

  execute(decoded: DecodeResult, registers: RegisterFile, memory: Memory, pc: number): ExecuteResult {
    const f = decoded.fields;
    const m = decoded.mnemonic;

    const writeRd = (rd: number, value: number): Record<string, number> => {
      const changes: Record<string, number> = {};
      if (rd !== 0) {
        const v = toU32(value);
        registers.write(rd, v);
        changes[`x${rd}`] = v;
      }
      return changes;
    };

    const noMem: Record<number, number> = {};
    const noReg: Record<string, number> = {};

    // === I-type arithmetic ===
    const immArith = (name: string, op: (a: number, b: number) => number): ExecuteResult => {
      const rs1Val = toI32(registers.read(f["rs1"]));
      const result = toU32(op(rs1Val, f["imm"]));
      return { description: name, registersChanged: writeRd(f["rd"], result), memoryChanged: noMem, nextPc: pc + 4, halted: false };
    };

    // === R-type arithmetic ===
    const regArith = (name: string, op: (a: number, b: number) => number): ExecuteResult => {
      const a = registers.read(f["rs1"]);
      const b = registers.read(f["rs2"]);
      const result = toU32(op(a, b));
      return { description: name, registersChanged: writeRd(f["rd"], result), memoryChanged: noMem, nextPc: pc + 4, halted: false };
    };

    // === Branch ===
    const branch = (name: string, cond: (a: number, b: number) => boolean): ExecuteResult => {
      const a = registers.read(f["rs1"]);
      const b = registers.read(f["rs2"]);
      const taken = cond(a, b);
      return { description: name, registersChanged: noReg, memoryChanged: noMem, nextPc: taken ? pc + f["imm"] : pc + 4, halted: false };
    };

    switch (m) {
      // I-type arithmetic
      case "addi": return immArith("addi", (a, b) => (a + b) | 0);
      case "slti": return immArith("slti", (a, b) => toI32(a) < toI32(b) ? 1 : 0);
      case "sltiu": return immArith("sltiu", (a, b) => toU32(a) < toU32(b) ? 1 : 0);
      case "xori": return immArith("xori", (a, b) => a ^ b);
      case "ori": return immArith("ori", (a, b) => a | b);
      case "andi": return immArith("andi", (a, b) => a & b);
      case "slli": return immArith("slli", (a, b) => (toU32(a) << (b & 0x1f)));
      case "srli": return immArith("srli", (a, b) => toU32(a) >>> (b & 0x1f));
      case "srai": return immArith("srai", (a, b) => toI32(a) >> (b & 0x1f));

      // R-type arithmetic
      case "add": return regArith("add", (a, b) => (toI32(a) + toI32(b)) | 0);
      case "sub": return regArith("sub", (a, b) => (toI32(a) - toI32(b)) | 0);
      case "sll": return regArith("sll", (a, b) => a << (b & 0x1f));
      case "slt": return regArith("slt", (a, b) => toI32(a) < toI32(b) ? 1 : 0);
      case "sltu": return regArith("sltu", (a, b) => toU32(a) < toU32(b) ? 1 : 0);
      case "xor": return regArith("xor", (a, b) => a ^ b);
      case "srl": return regArith("srl", (a, b) => a >>> (b & 0x1f));
      case "sra": return regArith("sra", (a, b) => toI32(a) >> (b & 0x1f));
      case "or": return regArith("or", (a, b) => a | b);
      case "and": return regArith("and", (a, b) => a & b);

      // Load
      case "lb": case "lh": case "lw": case "lbu": case "lhu":
        return this.execLoad(decoded, registers, memory, pc, writeRd);

      // Store
      case "sb": case "sh": case "sw":
        return this.execStore(decoded, registers, memory, pc);

      // Branch
      case "beq": return branch("beq", (a, b) => a === b);
      case "bne": return branch("bne", (a, b) => a !== b);
      case "blt": return branch("blt", (a, b) => toI32(a) < toI32(b));
      case "bge": return branch("bge", (a, b) => toI32(a) >= toI32(b));
      case "bltu": return branch("bltu", (a, b) => toU32(a) < toU32(b));
      case "bgeu": return branch("bgeu", (a, b) => toU32(a) >= toU32(b));

      // Jump
      case "jal": {
        const returnAddr = toU32(pc + 4);
        return { description: "jal", registersChanged: writeRd(f["rd"], returnAddr), memoryChanged: noMem, nextPc: pc + f["imm"], halted: false };
      }
      case "jalr": {
        const returnAddr = toU32(pc + 4);
        const target = (toI32(registers.read(f["rs1"])) + f["imm"]) & ~1;
        return { description: "jalr", registersChanged: writeRd(f["rd"], returnAddr), memoryChanged: noMem, nextPc: target, halted: false };
      }

      // Upper immediate
      case "lui": {
        const result = toU32(f["imm"] << 12);
        return { description: "lui", registersChanged: writeRd(f["rd"], result), memoryChanged: noMem, nextPc: pc + 4, halted: false };
      }
      case "auipc": {
        const result = toU32(pc + (f["imm"] << 12));
        return { description: "auipc", registersChanged: writeRd(f["rd"], result), memoryChanged: noMem, nextPc: pc + 4, halted: false };
      }

      // System
      case "ecall": return this.execEcall(pc);
      case "mret": return this.execMret(pc);
      case "csrrw": return this.execCSRRW(decoded, registers, pc, writeRd);
      case "csrrs": return this.execCSRRS(decoded, registers, pc, writeRd);
      case "csrrc": return this.execCSRRC(decoded, registers, pc, writeRd);

      default:
        return { description: `Unknown: ${m}`, registersChanged: noReg, memoryChanged: noMem, nextPc: pc + 4, halted: false };
    }
  }

  private execLoad(decoded: DecodeResult, registers: RegisterFile, memory: Memory, pc: number,
    writeRd: (rd: number, value: number) => Record<string, number>): ExecuteResult {
    const f = decoded.fields;
    const addr = (toI32(registers.read(f["rs1"])) + f["imm"]) | 0;
    let result: number;

    switch (decoded.mnemonic) {
      case "lb": result = toI32(signExtend(memory.readByte(addr), 8)); break;
      case "lh": {
        const lo = memory.readByte(addr);
        const hi = memory.readByte(addr + 1);
        result = toI32(signExtend(lo | (hi << 8), 16));
        break;
      }
      case "lw": result = memory.readWord(addr); break;
      case "lbu": result = memory.readByte(addr); break;
      case "lhu": {
        const lo = memory.readByte(addr);
        const hi = memory.readByte(addr + 1);
        result = lo | (hi << 8);
        break;
      }
      default: result = 0;
    }

    return { description: decoded.mnemonic, registersChanged: writeRd(f["rd"], toU32(result)), memoryChanged: {}, nextPc: pc + 4, halted: false };
  }

  private execStore(decoded: DecodeResult, registers: RegisterFile, memory: Memory, pc: number): ExecuteResult {
    const f = decoded.fields;
    const addr = (toI32(registers.read(f["rs1"])) + f["imm"]) | 0;
    const val = registers.read(f["rs2"]);
    const memChanges: Record<number, number> = {};

    switch (decoded.mnemonic) {
      case "sb": {
        const b = val & 0xff;
        memory.writeByte(addr, b);
        memChanges[addr] = b;
        break;
      }
      case "sh": {
        const lo = val & 0xff;
        const hi = (val >>> 8) & 0xff;
        memory.writeByte(addr, lo);
        memory.writeByte(addr + 1, hi);
        memChanges[addr] = lo;
        memChanges[addr + 1] = hi;
        break;
      }
      case "sw": {
        memory.writeWord(addr, val);
        memChanges[addr] = val & 0xff;
        memChanges[addr + 1] = (val >>> 8) & 0xff;
        memChanges[addr + 2] = (val >>> 16) & 0xff;
        memChanges[addr + 3] = (val >>> 24) & 0xff;
        break;
      }
    }

    return { description: decoded.mnemonic, registersChanged: {}, memoryChanged: memChanges, nextPc: pc + 4, halted: false };
  }

  private execEcall(pc: number): ExecuteResult {
    if (!this.csr) {
      return { description: "ecall: halt (no CSR)", registersChanged: {}, memoryChanged: {}, nextPc: pc, halted: true };
    }
    const mtvec = this.csr.read(CSR_MTVEC);
    if (mtvec === 0) {
      return { description: "ecall: halt (mtvec=0)", registersChanged: {}, memoryChanged: {}, nextPc: pc, halted: true };
    }
    this.csr.write(CSR_MEPC, toU32(pc));
    this.csr.write(CSR_MCAUSE, CAUSE_ECALL_MMODE);
    const mstatus = this.csr.read(CSR_MSTATUS);
    this.csr.write(CSR_MSTATUS, (mstatus & ~MIE) >>> 0);
    return { description: `ecall: trap to 0x${mtvec.toString(16)}`, registersChanged: {}, memoryChanged: {}, nextPc: mtvec, halted: false };
  }

  private execMret(pc: number): ExecuteResult {
    if (!this.csr) {
      return { description: "mret: no CSR", registersChanged: {}, memoryChanged: {}, nextPc: pc + 4, halted: false };
    }
    const mepc = this.csr.read(CSR_MEPC);
    const mstatus = this.csr.read(CSR_MSTATUS);
    this.csr.write(CSR_MSTATUS, (mstatus | MIE) >>> 0);
    return { description: `mret: return to 0x${mepc.toString(16)}`, registersChanged: {}, memoryChanged: {}, nextPc: mepc, halted: false };
  }

  private execCSRRW(decoded: DecodeResult, registers: RegisterFile, pc: number,
    writeRd: (rd: number, value: number) => Record<string, number>): ExecuteResult {
    if (!this.csr) return { description: "csrrw: no CSR", registersChanged: {}, memoryChanged: {}, nextPc: pc + 4, halted: false };
    const f = decoded.fields;
    const rs1Val = registers.read(f["rs1"]);
    const oldCSR = this.csr.readWrite(f["csr"], rs1Val);
    return { description: "csrrw", registersChanged: writeRd(f["rd"], oldCSR), memoryChanged: {}, nextPc: pc + 4, halted: false };
  }

  private execCSRRS(decoded: DecodeResult, registers: RegisterFile, pc: number,
    writeRd: (rd: number, value: number) => Record<string, number>): ExecuteResult {
    if (!this.csr) return { description: "csrrs: no CSR", registersChanged: {}, memoryChanged: {}, nextPc: pc + 4, halted: false };
    const f = decoded.fields;
    const rs1Val = registers.read(f["rs1"]);
    const oldCSR = this.csr.readSet(f["csr"], rs1Val);
    return { description: "csrrs", registersChanged: writeRd(f["rd"], oldCSR), memoryChanged: {}, nextPc: pc + 4, halted: false };
  }

  private execCSRRC(decoded: DecodeResult, registers: RegisterFile, pc: number,
    writeRd: (rd: number, value: number) => Record<string, number>): ExecuteResult {
    if (!this.csr) return { description: "csrrc: no CSR", registersChanged: {}, memoryChanged: {}, nextPc: pc + 4, halted: false };
    const f = decoded.fields;
    const rs1Val = registers.read(f["rs1"]);
    const oldCSR = this.csr.readClear(f["csr"], rs1Val);
    return { description: "csrrc", registersChanged: writeRd(f["rd"], oldCSR), memoryChanged: {}, nextPc: pc + 4, halted: false };
  }
}

// =========================================================================
// High-level simulator
// =========================================================================

export class RiscVSimulator {
  readonly decoder: RiscVDecoder;
  readonly executor: RiscVExecutor;
  readonly cpu: CPU;
  readonly csr: CSRFile;

  constructor(memorySize: number = 65536) {
    this.decoder = new RiscVDecoder();
    this.csr = new CSRFile();
    this.executor = new RiscVExecutor();
    this.executor.csr = this.csr;
    this.cpu = new CPU(this.decoder, this.executor, 32, 32, memorySize);
  }

  run(program: number[] | Uint8Array): PipelineTrace[] {
    this.cpu.loadProgram(program);
    return this.cpu.run();
  }

  step(): PipelineTrace {
    return this.cpu.step();
  }
}
