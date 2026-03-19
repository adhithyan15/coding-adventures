/**
 * RISC-V RV32I Simulator -- a clean, modern instruction set.
 *
 * === What is RISC-V? ===
 *
 * RISC-V (pronounced "risk-five") is an open-source instruction set architecture
 * (ISA) designed at UC Berkeley by Patterson and Hennessy -- the same people who
 * wrote the definitive computer architecture textbooks. It was designed from
 * scratch in 2010 with no historical baggage, making it the cleanest ISA to learn.
 *
 * "RISC" stands for Reduced Instruction Set Computer -- the philosophy that a
 * CPU should have a small number of simple instructions rather than many complex
 * ones. Each instruction does one thing well.
 *
 * === RISC-V vs other ISAs ===
 *
 *     RISC-V:     Clean, regular encoding. 32 registers. No condition codes.
 *     ARM:        More complex encoding. 16 registers. Conditional execution.
 *     WASM:       Stack-based (no registers). Modern virtual machine.
 *     Intel 4004: 4-bit accumulator. Historical (1971).
 *
 * === Register conventions ===
 *
 * RISC-V has 32 registers, each 32 bits wide:
 *
 *     x0  = always 0 (hardwired -- writes are ignored, reads always return 0)
 *     x1  = ra (return address -- where to go back after a function call)
 *     x2  = sp (stack pointer -- top of the stack)
 *     x3  = gp (global pointer)
 *     x4  = tp (thread pointer)
 *     x5-x7   = t0-t2 (temporary registers)
 *     x8-x9   = s0-s1 (saved registers)
 *     x10-x17 = a0-a7 (function arguments and return values)
 *     x18-x27 = s2-s11 (more saved registers)
 *     x28-x31 = t3-t6 (more temporaries)
 *
 * The x0 register is special and brilliant: because it's always 0, many
 * operations become simpler. To load an immediate value, you just add it to x0:
 *     addi x1, x0, 42    ->    x1 = 0 + 42 = 42
 *
 * === Instruction encoding ===
 *
 * Every RISC-V instruction is exactly 32 bits. The opcode is always in bits [6:0].
 * Register fields are always in the same positions -- this regularity makes the
 * decoder simpler than ARM's.
 *
 * R-type (register-register):
 *     +---------+-----+-----+-------+-----+---------+
 *     | funct7  | rs2 | rs1 |funct3 | rd  | opcode  |
 *     | 31   25 |24 20|19 15|14   12|11  7| 6     0 |
 *     +---------+-----+-----+-------+-----+---------+
 *
 * I-type (immediate):
 *     +--------------+-----+-------+-----+---------+
 *     |  imm[11:0]   | rs1 |funct3 | rd  | opcode  |
 *     | 31        20 |19 15|14   12|11  7| 6     0 |
 *     +--------------+-----+-------+-----+---------+
 *
 * === MVP instruction set (just enough for x = 1 + 2) ===
 *
 *     addi x1, x0, 1    ->  x1 = 0 + 1 = 1     (I-type, opcode=0010011)
 *     addi x2, x0, 2    ->  x2 = 0 + 2 = 2     (I-type, opcode=0010011)
 *     add  x3, x1, x2   ->  x3 = 1 + 2 = 3     (R-type, opcode=0110011)
 *     ecall              ->  halt                 (I-type, opcode=1110011)
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

// ---------------------------------------------------------------------------
// Instruction encoding constants
// ---------------------------------------------------------------------------
// These are the bit patterns that identify each instruction type.
// The opcode is always in bits [6:0] of the 32-bit instruction.

/** I-type arithmetic with immediate (addi, etc.) */
export const OPCODE_OP_IMM = 0b0010011;

/** R-type arithmetic (add, sub, etc.) */
export const OPCODE_OP = 0b0110011;

/** System instructions (ecall) */
export const OPCODE_SYSTEM = 0b1110011;

// ---------------------------------------------------------------------------
// Decoder
// ---------------------------------------------------------------------------

/**
 * Decodes RISC-V RV32I instructions from 32-bit binary to structured fields.
 *
 * The decoder extracts the opcode, register numbers, and immediate values
 * from the raw instruction bits. It doesn't execute anything -- it just
 * figures out what the instruction means.
 *
 * Example: decoding addi x1, x0, 1 (binary: 0x00100093)
 *
 *     Bits: 000000000001 00000 000 00001 0010011
 *           ^^^^^^^^^^^^ ^^^^^ ^^^ ^^^^^ ^^^^^^^
 *           imm=1        rs1=0 f3  rd=1  opcode=OP_IMM
 *
 *     Result: { mnemonic: "addi", fields: { rd: 1, rs1: 0, imm: 1 } }
 */
export class RiscVDecoder implements InstructionDecoder {
  /**
   * Decode a 32-bit RISC-V instruction.
   *
   * Extracts the opcode from bits [6:0], then dispatches to the
   * appropriate format decoder (R-type, I-type, etc.).
   */
  decode(rawInstruction: number, pc: number): DecodeResult {
    const opcode = rawInstruction & 0x7f; // bits [6:0]

    if (opcode === OPCODE_OP_IMM) {
      return this.decodeIType(rawInstruction, "addi");
    } else if (opcode === OPCODE_OP) {
      return this.decodeRType(rawInstruction);
    } else if (opcode === OPCODE_SYSTEM) {
      return {
        mnemonic: "ecall",
        fields: {},
        rawInstruction,
      };
    } else {
      return {
        mnemonic: `UNKNOWN(0x${opcode.toString(16).padStart(2, "0")})`,
        fields: { opcode },
        rawInstruction,
      };
    }
  }

  /**
   * Decode an R-type instruction (register-register operation).
   *
   * R-type format:
   *     [funct7 | rs2 | rs1 | funct3 | rd | opcode]
   *      31  25  24 20 19 15  14   12  11 7  6    0
   *
   * Example: add x3, x1, x2
   *     funct7=0000000, rs2=2, rs1=1, funct3=000, rd=3, opcode=0110011
   */
  private decodeRType(raw: number): DecodeResult {
    const rd = (raw >>> 7) & 0x1f;
    const funct3 = (raw >>> 12) & 0x7;
    const rs1 = (raw >>> 15) & 0x1f;
    const rs2 = (raw >>> 20) & 0x1f;
    const funct7 = (raw >>> 25) & 0x7f;

    // Determine the specific operation from funct3 and funct7
    let mnemonic: string;
    if (funct3 === 0 && funct7 === 0) {
      mnemonic = "add";
    } else if (funct3 === 0 && funct7 === 0x20) {
      mnemonic = "sub";
    } else {
      mnemonic = `r_op(f3=${funct3},f7=${funct7})`;
    }

    return {
      mnemonic,
      fields: { rd, rs1, rs2, funct3, funct7 },
      rawInstruction: raw,
    };
  }

  /**
   * Decode an I-type instruction (immediate operation).
   *
   * I-type format:
   *     [imm[11:0] | rs1 | funct3 | rd | opcode]
   *      31     20  19 15  14   12  11 7  6    0
   *
   * The immediate value is sign-extended from 12 bits to 32 bits.
   * This means bit 11 is the sign bit:
   *     0x001 = 1    (positive)
   *     0xFFF = -1   (negative, sign-extended)
   *
   * Example: addi x1, x0, 1
   *     imm=000000000001, rs1=0, funct3=000, rd=1, opcode=0010011
   */
  private decodeIType(raw: number, defaultMnemonic: string): DecodeResult {
    const rd = (raw >>> 7) & 0x1f;
    const funct3 = (raw >>> 12) & 0x7;
    const rs1 = (raw >>> 15) & 0x1f;
    let imm = (raw >>> 20) & 0xfff;

    // Sign-extend the 12-bit immediate to 32 bits
    // If bit 11 is set, the value is negative
    if (imm & 0x800) {
      imm -= 0x1000; // Convert from unsigned to signed
    }

    return {
      mnemonic: defaultMnemonic,
      fields: { rd, rs1, imm, funct3 },
      rawInstruction: raw,
    };
  }
}

// ---------------------------------------------------------------------------
// Executor
// ---------------------------------------------------------------------------

/**
 * Executes decoded RISC-V instructions.
 *
 * The executor reads register values, performs the operation (often using
 * the ALU), writes the result back, and determines the next PC.
 *
 * RISC-V special rule: register x0 is HARDWIRED to 0. Any write to x0
 * is silently ignored. Any read from x0 always returns 0. This is
 * enforced here, not in the register file (which is generic).
 */
export class RiscVExecutor implements InstructionExecutor {
  /**
   * Execute one decoded RISC-V instruction.
   */
  execute(
    decoded: DecodeResult,
    registers: RegisterFile,
    memory: Memory,
    pc: number
  ): ExecuteResult {
    const mnemonic = decoded.mnemonic;

    if (mnemonic === "addi") {
      return this.execAddi(decoded, registers, pc);
    } else if (mnemonic === "add") {
      return this.execAdd(decoded, registers, pc);
    } else if (mnemonic === "sub") {
      return this.execSub(decoded, registers, pc);
    } else if (mnemonic === "ecall") {
      return {
        description: "System call (halt)",
        registersChanged: {},
        memoryChanged: {},
        nextPc: pc,
        halted: true,
      };
    } else {
      return {
        description: `Unknown instruction: ${mnemonic}`,
        registersChanged: {},
        memoryChanged: {},
        nextPc: pc + 4,
        halted: false,
      };
    }
  }

  /**
   * Execute: addi rd, rs1, imm -> rd = rs1 + imm
   *
   * Example: addi x1, x0, 1
   *     rs1 = x0 = 0 (always zero)
   *     imm = 1
   *     result = 0 + 1 = 1
   *     Write 1 to x1
   */
  private execAddi(
    decoded: DecodeResult,
    registers: RegisterFile,
    pc: number
  ): ExecuteResult {
    const rd = decoded.fields["rd"];
    const rs1 = decoded.fields["rs1"];
    const imm = decoded.fields["imm"];

    const rs1Val = registers.read(rs1);
    const result = (rs1Val + imm) & 0xffffffff; // Mask to 32 bits

    // x0 is hardwired to 0 -- writes to x0 are silently ignored
    const changes: Record<string, number> = {};
    if (rd !== 0) {
      registers.write(rd, result);
      changes[`x${rd}`] = result;
    }

    return {
      description: `x${rd} = x${rs1}(${rs1Val}) + ${imm} = ${result}`,
      registersChanged: changes,
      memoryChanged: {},
      nextPc: pc + 4,
      halted: false,
    };
  }

  /**
   * Execute: add rd, rs1, rs2 -> rd = rs1 + rs2
   *
   * Example: add x3, x1, x2  (where x1=1, x2=2)
   *     rs1_val = 1, rs2_val = 2
   *     result = 1 + 2 = 3
   *     Write 3 to x3
   */
  private execAdd(
    decoded: DecodeResult,
    registers: RegisterFile,
    pc: number
  ): ExecuteResult {
    const rd = decoded.fields["rd"];
    const rs1 = decoded.fields["rs1"];
    const rs2 = decoded.fields["rs2"];

    const rs1Val = registers.read(rs1);
    const rs2Val = registers.read(rs2);
    const result = (rs1Val + rs2Val) & 0xffffffff;

    const changes: Record<string, number> = {};
    if (rd !== 0) {
      registers.write(rd, result);
      changes[`x${rd}`] = result;
    }

    return {
      description: `x${rd} = x${rs1}(${rs1Val}) + x${rs2}(${rs2Val}) = ${result}`,
      registersChanged: changes,
      memoryChanged: {},
      nextPc: pc + 4,
      halted: false,
    };
  }

  /**
   * Execute: sub rd, rs1, rs2 -> rd = rs1 - rs2
   */
  private execSub(
    decoded: DecodeResult,
    registers: RegisterFile,
    pc: number
  ): ExecuteResult {
    const rd = decoded.fields["rd"];
    const rs1 = decoded.fields["rs1"];
    const rs2 = decoded.fields["rs2"];

    const rs1Val = registers.read(rs1);
    const rs2Val = registers.read(rs2);
    const result = (rs1Val - rs2Val) & 0xffffffff;

    const changes: Record<string, number> = {};
    if (rd !== 0) {
      registers.write(rd, result);
      changes[`x${rd}`] = result;
    }

    return {
      description: `x${rd} = x${rs1}(${rs1Val}) - x${rs2}(${rs2Val}) = ${result}`,
      registersChanged: changes,
      memoryChanged: {},
      nextPc: pc + 4,
      halted: false,
    };
  }
}

// ---------------------------------------------------------------------------
// Assembler helpers
// ---------------------------------------------------------------------------
// These functions encode RISC-V instructions from human-readable form
// to binary. This is a tiny assembler -- just enough to create test programs.

/**
 * Encode: addi rd, rs1, imm -> 32-bit instruction.
 *
 * I-type format: [imm[11:0] | rs1 | funct3=000 | rd | opcode=0010011]
 *
 * Example:
 *     encodeAddi(1, 0, 1)  // addi x1, x0, 1 -> 0x00100093
 */
export function encodeAddi(rd: number, rs1: number, imm: number): number {
  const immBits = imm & 0xfff;
  return (
    ((immBits << 20) | (rs1 << 15) | (0 << 12) | (rd << 7) | OPCODE_OP_IMM) >>>
    0
  );
}

/**
 * Encode: add rd, rs1, rs2 -> 32-bit instruction.
 *
 * R-type format: [funct7=0 | rs2 | rs1 | funct3=000 | rd | opcode=0110011]
 *
 * Example:
 *     encodeAdd(3, 1, 2)  // add x3, x1, x2 -> 0x002081B3
 */
export function encodeAdd(rd: number, rs1: number, rs2: number): number {
  return (
    ((0 << 25) | (rs2 << 20) | (rs1 << 15) | (0 << 12) | (rd << 7) | OPCODE_OP) >>>
    0
  );
}

/**
 * Encode: sub rd, rs1, rs2 -> 32-bit instruction.
 *
 * R-type format: [funct7=0100000 | rs2 | rs1 | funct3=000 | rd | opcode=0110011]
 *
 * Example:
 *     encodeSub(3, 1, 2)  // sub x3, x1, x2 -> x3 = x1 - x2
 */
export function encodeSub(rd: number, rs1: number, rs2: number): number {
  return (
    ((0b0100000 << 25) | (rs2 << 20) | (rs1 << 15) | (0 << 12) | (rd << 7) | OPCODE_OP) >>>
    0
  );
}

/**
 * Encode: ecall -> 32-bit instruction.
 *
 * System format: [0...0 | opcode=1110011]
 *
 * Example:
 *     encodeEcall()  // -> 0x00000073
 */
export function encodeEcall(): number {
  return OPCODE_SYSTEM;
}

// ---------------------------------------------------------------------------
// High-level simulator
// ---------------------------------------------------------------------------

/**
 * Complete RISC-V simulator -- ISA + CPU in one convenient class.
 *
 * This wraps the CPU simulator with the RISC-V decoder and executor,
 * providing a simple interface for running RISC-V programs.
 *
 * Example: running x = 1 + 2
 *
 *     const sim = new RiscVSimulator();
 *     const program = assemble([
 *         encodeAddi(1, 0, 1),    // x1 = 1
 *         encodeAddi(2, 0, 2),    // x2 = 2
 *         encodeAdd(3, 1, 2),     // x3 = x1 + x2 = 3
 *         encodeEcall(),           // halt
 *     ]);
 *     const traces = sim.run(program);
 *     sim.cpu.registers.read(3);  // 3
 *
 *     The pipeline trace for each instruction shows:
 *     --- Cycle 0 ---
 *       FETCH              | DECODE             | EXECUTE
 *       PC: 0x0000         | addi               | x1 = x0(0) + 1 = 1
 *       -> 0x00100093      | rd=1 rs1=0 imm=1   | PC -> 4
 */
export class RiscVSimulator {
  /** The RISC-V instruction decoder. */
  readonly decoder: RiscVDecoder;

  /** The RISC-V instruction executor. */
  readonly executor: RiscVExecutor;

  /** The underlying generic CPU. */
  readonly cpu: CPU;

  constructor(memorySize: number = 65536) {
    this.decoder = new RiscVDecoder();
    this.executor = new RiscVExecutor();
    this.cpu = new CPU(
      this.decoder,
      this.executor,
      32, // RISC-V has 32 registers
      32,
      memorySize
    );
    // Enforce x0 = 0 (it's already 0 from initialization,
    // but the executor also prevents writes to x0)
  }

  /**
   * Load and run a RISC-V program, returning the pipeline trace.
   */
  run(program: number[] | Uint8Array): PipelineTrace[] {
    this.cpu.loadProgram(program);
    return this.cpu.run();
  }

  /**
   * Execute one instruction and return its pipeline trace.
   */
  step(): PipelineTrace {
    return this.cpu.step();
  }
}

/**
 * Convert a list of 32-bit instruction words to bytes (little-endian).
 *
 * This is a convenience function for creating test programs:
 *
 *     const program = assemble([
 *         encodeAddi(1, 0, 1),   // x1 = 1
 *         encodeAddi(2, 0, 2),   // x2 = 2
 *         encodeAdd(3, 1, 2),    // x3 = x1 + x2
 *         encodeEcall(),          // halt
 *     ]);
 *
 * Each 32-bit instruction is stored as 4 bytes in little-endian order.
 * Little-endian means the least significant byte comes first:
 *     0x00100093 -> [0x93, 0x00, 0x10, 0x00]
 */
export function assemble(instructions: number[]): number[] {
  const result: number[] = [];
  for (const instr of instructions) {
    const masked = (instr & 0xffffffff) >>> 0;
    result.push(masked & 0xff);
    result.push((masked >>> 8) & 0xff);
    result.push((masked >>> 16) & 0xff);
    result.push((masked >>> 24) & 0xff);
  }
  return result;
}
