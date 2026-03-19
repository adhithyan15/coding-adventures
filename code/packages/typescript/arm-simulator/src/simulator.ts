/**
 * ARM Simulator -- the architecture that powers your phone.
 *
 * === What is ARM? ===
 *
 * ARM (originally Acorn RISC Machine) was designed in 1985 by Sophie Wilson and
 * Steve Furber at Acorn Computers in Cambridge, England. It was one of the first
 * commercial RISC processors -- inspired by the Berkeley RISC project that also
 * influenced MIPS and eventually RISC-V.
 *
 * ARM's big insight was power efficiency. While Intel focused on raw speed, ARM
 * optimized for low power consumption. This bet paid off spectacularly: today ARM
 * processors are in virtually every smartphone, tablet, and embedded device on
 * Earth. Apple's M-series chips are ARM. Your phone is ARM. Most of the world's
 * CPUs are ARM.
 *
 * === ARM vs RISC-V ===
 *
 *     ARM:      16 registers (R0-R15). Condition codes on every instruction.
 *               More complex encoding. Commercial (licensed by ARM Ltd).
 *               Designed 1985. Mature, battle-tested, ubiquitous.
 *
 *     RISC-V:   32 registers (x0-x31). No condition codes. Clean, regular
 *               encoding. Open-source. Designed 2010. The "clean slate" ISA.
 *
 * The biggest architectural difference is conditional execution. In ARM, EVERY
 * instruction has a 4-bit condition field. This means you can write:
 *
 *     CMP R0, R1           ; compare R0 and R1, set flags
 *     ADDGT R2, R0, R1     ; add ONLY IF R0 > R1 (Greater Than)
 *     SUBLE R2, R0, R1     ; subtract ONLY IF R0 <= R1 (Less or Equal)
 *
 * RISC-V doesn't have this -- it uses separate branch instructions instead.
 * ARM's approach reduces branch instructions (good for pipelines) but makes
 * the encoding more complex.
 *
 * === Register conventions ===
 *
 * ARM has 16 registers, each 32 bits wide:
 *
 *     R0-R3   = function arguments and return values
 *     R4-R11  = general purpose (callee-saved)
 *     R12     = IP (intra-procedure scratch register)
 *     R13     = SP (stack pointer)
 *     R14     = LR (link register -- return address)
 *     R15     = PC (program counter -- yes, it's a visible register!)
 *
 * Unlike RISC-V, ARM has no hardwired-zero register. R15 being the PC is
 * a quirk that allows some clever tricks (and some nasty bugs).
 *
 * === Instruction encoding ===
 *
 * Every ARM instruction is exactly 32 bits. The condition code is ALWAYS in
 * bits [31:28] -- this is what makes conditional execution possible.
 *
 * Data processing format:
 *     [cond(4) | 00 | I(1) | opcode(4) | S(1) | Rn(4) | Rd(4) | operand2(12)]
 *      31   28  27 26  25    24     21   20     19  16   15  12   11         0
 *
 *     cond:     Condition code (0b1110 = AL = always execute)
 *     I:        Immediate flag (1 = operand2 is an immediate, 0 = register)
 *     opcode:   Which operation (0b1101=MOV, 0b0100=ADD, 0b0010=SUB)
 *     S:        Set condition flags (we use 0 for now)
 *     Rn:       First source register
 *     Rd:       Destination register
 *     operand2: Either an 8-bit immediate with 4-bit rotation, or a register
 *
 * When I=1, operand2 encodes an immediate as:
 *     [rotate(4) | imm8(8)]
 *     The actual value is: imm8 rotated right by (rotate * 2) positions
 *
 * When I=0, operand2's lowest 4 bits are the register number (Rm).
 *
 * === MVP instruction set (just enough for x = 1 + 2) ===
 *
 *     MOV R0, #1         -> R0 = 1          (data processing, I=1, opcode=MOV)
 *     MOV R1, #2         -> R1 = 2          (data processing, I=1, opcode=MOV)
 *     ADD R2, R0, R1     -> R2 = R0 + R1    (data processing, I=0, opcode=ADD)
 *     HLT                -> halt             (custom encoding)
 */

import {
  CPU,
  Memory,
  RegisterFile,
} from "@coding-adventures/cpu-simulator";
import type {
  DecodeResult,
  ExecuteResult,
  PipelineTrace,
  InstructionDecoder,
  InstructionExecutor,
} from "@coding-adventures/cpu-simulator";

// ---------------------------------------------------------------------------
// Instruction encoding constants
// ---------------------------------------------------------------------------
// ARM data processing opcodes (bits [24:21] of the instruction word).
// The condition code in bits [31:28] is always 0b1110 (AL = always).

/** Always execute -- the most common condition code. */
export const COND_AL = 0b1110;

/** MOV Rd, operand2 (Rd = operand2, ignores Rn). */
export const OPCODE_MOV = 0b1101;

/** ADD Rd, Rn, operand2 (Rd = Rn + operand2). */
export const OPCODE_ADD = 0b0100;

/** SUB Rd, Rn, operand2 (Rd = Rn - operand2). */
export const OPCODE_SUB = 0b0010;

/**
 * We encode HLT as a special sentinel: all condition bits set to 0b1111
 * (which is "never execute" in ARMv4, repurposed here as halt).
 */
export const HLT_INSTRUCTION = 0xffffffff;

// ---------------------------------------------------------------------------
// Decoder
// ---------------------------------------------------------------------------

/**
 * Decodes ARM data processing instructions from 32-bit binary to structured fields.
 *
 * The decoder extracts the condition code, opcode, register numbers, and
 * immediate values from the raw instruction bits. It doesn't execute
 * anything -- it just figures out what the instruction means.
 *
 * ARM's encoding is more complex than RISC-V's because of the condition
 * field and the flexible operand2 encoding. But the data processing format
 * is regular: the opcode is always in bits [24:21], Rd in [15:12], and
 * Rn in [19:16].
 *
 * Example: decoding MOV R0, #1 (binary: 0xE3A00001)
 *
 *     Bits: 1110 00 1 1101 0 0000 0000 000000000001
 *           ^^^^ ^^ ^ ^^^^ ^ ^^^^ ^^^^ ^^^^^^^^^^^^
 *           cond    I  MOV S  Rn   Rd    operand2
 *
 *     Result: DecodeResult { mnemonic: "mov", fields: { rd: 0, imm: 1 } }
 */
export class ARMDecoder implements InstructionDecoder {
  /**
   * Decode a 32-bit ARM instruction.
   *
   * Checks for the HLT sentinel first, then extracts the condition
   * code and dispatches to the data processing decoder.
   */
  decode(rawInstruction: number, pc: number): DecodeResult {
    // Check for our custom halt instruction
    if (rawInstruction === HLT_INSTRUCTION) {
      return {
        mnemonic: "hlt",
        fields: {},
        rawInstruction,
      };
    }

    return this.decodeDataProcessing(rawInstruction);
  }

  /**
   * Decode an ARM data processing instruction.
   *
   * Data processing format:
   *     [cond(4) | 00 | I(1) | opcode(4) | S(1) | Rn(4) | Rd(4) | operand2(12)]
   *      31   28  27 26  25    24     21   20     19  16   15  12   11         0
   *
   * The I bit determines how operand2 is interpreted:
   *     I=1: immediate -- [rotate(4) | imm8(8)]
   *     I=0: register  -- lowest 4 bits are Rm
   *
   * Example: ADD R2, R0, R1 (register form)
   *     cond=1110, I=0, opcode=0100, S=0, Rn=0, Rd=2, operand2=...0001
   *     -> Rm=1
   */
  private decodeDataProcessing(raw: number): DecodeResult {
    const cond = (raw >>> 28) & 0xf;
    const iBit = (raw >>> 25) & 0x1;
    const opcode = (raw >>> 21) & 0xf;
    const sBit = (raw >>> 20) & 0x1;
    const rn = (raw >>> 16) & 0xf;
    const rd = (raw >>> 12) & 0xf;
    const operand2 = raw & 0xfff;

    // Determine the mnemonic from the opcode
    let mnemonic: string;
    if (opcode === OPCODE_MOV) {
      mnemonic = "mov";
    } else if (opcode === OPCODE_ADD) {
      mnemonic = "add";
    } else if (opcode === OPCODE_SUB) {
      mnemonic = "sub";
    } else {
      mnemonic = `dp_op(0b${opcode.toString(2).padStart(4, "0")})`;
    }

    // Decode operand2 based on the I bit
    if (iBit === 1) {
      // Immediate: operand2 = [rotate(4) | imm8(8)]
      // Actual value = imm8 rotated right by (rotate * 2)
      const rotate = (operand2 >>> 8) & 0xf;
      const imm8 = operand2 & 0xff;
      // Rotate right by (rotate * 2) positions in a 32-bit field
      const shift = rotate * 2;
      let immValue: number;
      if (shift > 0) {
        immValue = ((imm8 >>> shift) | (imm8 << (32 - shift))) >>> 0;
      } else {
        immValue = imm8;
      }

      const fields: Record<string, number> = {
        cond,
        i_bit: iBit,
        opcode,
        s_bit: sBit,
        rn,
        rd,
        imm: immValue,
      };

      return { mnemonic, fields, rawInstruction: raw };
    } else {
      // Register: lowest 4 bits of operand2 are Rm
      const rm = operand2 & 0xf;

      const fields: Record<string, number> = {
        cond,
        i_bit: iBit,
        opcode,
        s_bit: sBit,
        rn,
        rd,
        rm,
      };

      return { mnemonic, fields, rawInstruction: raw };
    }
  }
}

// ---------------------------------------------------------------------------
// Executor
// ---------------------------------------------------------------------------

/**
 * Executes decoded ARM instructions.
 *
 * The executor reads register values, performs the operation, writes the
 * result back, and determines the next PC.
 *
 * Unlike RISC-V, ARM has no hardwired-zero register. All 16 registers
 * (R0-R15) are writable. R15 is the PC, but in our simplified simulator
 * we manage the PC separately and don't allow direct writes to it.
 */
export class ARMExecutor implements InstructionExecutor {
  /**
   * Execute one decoded ARM instruction.
   */
  execute(
    decoded: DecodeResult,
    registers: RegisterFile,
    memory: Memory,
    pc: number
  ): ExecuteResult {
    const mnemonic = decoded.mnemonic;

    if (mnemonic === "mov") {
      return this.execMov(decoded, registers, pc);
    } else if (mnemonic === "add") {
      return this.execAdd(decoded, registers, pc);
    } else if (mnemonic === "sub") {
      return this.execSub(decoded, registers, pc);
    } else if (mnemonic === "hlt") {
      return {
        description: "Halt",
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
   * Execute: MOV Rd, #imm -> Rd = imm
   *
   * MOV is special among data processing instructions: it ignores Rn
   * and writes operand2 directly into Rd.
   *
   * Example: MOV R0, #1
   *     imm = 1
   *     Write 1 to R0
   */
  private execMov(
    decoded: DecodeResult,
    registers: RegisterFile,
    pc: number
  ): ExecuteResult {
    const rd = decoded.fields["rd"];
    const imm = decoded.fields["imm"];

    const result = (imm & 0xffffffff) >>> 0;
    registers.write(rd, result);
    const changes: Record<string, number> = { [`R${rd}`]: result };

    return {
      description: `R${rd} = ${result}`,
      registersChanged: changes,
      memoryChanged: {},
      nextPc: pc + 4,
      halted: false,
    };
  }

  /**
   * Execute: ADD Rd, Rn, Rm -> Rd = Rn + Rm
   *
   * Example: ADD R2, R0, R1  (where R0=1, R1=2)
   *     rnVal = 1, rmVal = 2
   *     result = 1 + 2 = 3
   *     Write 3 to R2
   */
  private execAdd(
    decoded: DecodeResult,
    registers: RegisterFile,
    pc: number
  ): ExecuteResult {
    const rd = decoded.fields["rd"];
    const rn = decoded.fields["rn"];
    const rm = decoded.fields["rm"];

    const rnVal = registers.read(rn);
    const rmVal = registers.read(rm);
    const result = ((rnVal + rmVal) & 0xffffffff) >>> 0;

    registers.write(rd, result);
    const changes: Record<string, number> = { [`R${rd}`]: result };

    return {
      description: `R${rd} = R${rn}(${rnVal}) + R${rm}(${rmVal}) = ${result}`,
      registersChanged: changes,
      memoryChanged: {},
      nextPc: pc + 4,
      halted: false,
    };
  }

  /**
   * Execute: SUB Rd, Rn, Rm -> Rd = Rn - Rm
   */
  private execSub(
    decoded: DecodeResult,
    registers: RegisterFile,
    pc: number
  ): ExecuteResult {
    const rd = decoded.fields["rd"];
    const rn = decoded.fields["rn"];
    const rm = decoded.fields["rm"];

    const rnVal = registers.read(rn);
    const rmVal = registers.read(rm);
    const result = ((rnVal - rmVal) & 0xffffffff) >>> 0;

    registers.write(rd, result);
    const changes: Record<string, number> = { [`R${rd}`]: result };

    return {
      description: `R${rd} = R${rn}(${rnVal}) - R${rm}(${rmVal}) = ${result}`,
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
// These functions encode ARM instructions from human-readable form
// to binary. This is a tiny assembler -- just enough to create test programs.

/**
 * Encode: MOV Rd, #imm -> 32-bit instruction.
 *
 * Data processing format with I=1, opcode=MOV(1101):
 *     [cond=1110 | 00 | I=1 | 1101 | S=0 | Rn=0000 | Rd | rotate=0000 | imm8]
 *
 * The immediate must fit in 8 bits (0-255) with no rotation for this
 * simple encoder.
 *
 * Example:
 *     encodeMovImm(0, 1)  // MOV R0, #1 -> 0xE3A00001
 */
export function encodeMovImm(rd: number, imm: number): number {
  const cond = COND_AL;
  const iBit = 1;
  const opcode = OPCODE_MOV;
  const sBit = 0;
  const rn = 0; // MOV ignores Rn, conventionally set to 0
  const imm8 = imm & 0xff;
  const rotate = 0;

  return (
    (((cond << 28) |
      (0b00 << 26) |
      (iBit << 25) |
      (opcode << 21) |
      (sBit << 20) |
      (rn << 16) |
      (rd << 12) |
      (rotate << 8) |
      imm8) >>>
      0)
  );
}

/**
 * Encode: ADD Rd, Rn, Rm -> 32-bit instruction.
 *
 * Data processing format with I=0, opcode=ADD(0100):
 *     [cond=1110 | 00 | I=0 | 0100 | S=0 | Rn | Rd | 00000000 | Rm]
 *
 * Example:
 *     encodeAdd(2, 0, 1)  // ADD R2, R0, R1 -> 0xE0802001
 */
export function encodeAdd(rd: number, rn: number, rm: number): number {
  const cond = COND_AL;
  const iBit = 0;
  const opcode = OPCODE_ADD;
  const sBit = 0;

  return (
    (((cond << 28) |
      (0b00 << 26) |
      (iBit << 25) |
      (opcode << 21) |
      (sBit << 20) |
      (rn << 16) |
      (rd << 12) |
      rm) >>>
      0)
  );
}

/**
 * Encode: SUB Rd, Rn, Rm -> 32-bit instruction.
 *
 * Data processing format with I=0, opcode=SUB(0010):
 *     [cond=1110 | 00 | I=0 | 0010 | S=0 | Rn | Rd | 00000000 | Rm]
 *
 * Example:
 *     encodeSub(2, 0, 1)  // SUB R2, R0, R1 -> 0xE0402001
 */
export function encodeSub(rd: number, rn: number, rm: number): number {
  const cond = COND_AL;
  const iBit = 0;
  const opcode = OPCODE_SUB;
  const sBit = 0;

  return (
    (((cond << 28) |
      (0b00 << 26) |
      (iBit << 25) |
      (opcode << 21) |
      (sBit << 20) |
      (rn << 16) |
      (rd << 12) |
      rm) >>>
      0)
  );
}

/**
 * Encode: HLT -> 32-bit instruction.
 *
 * We use 0xFFFFFFFF as a custom halt sentinel. In real ARM, this would
 * be an unconditional instruction with condition 0b1111. We repurpose
 * it as a clean way to stop the simulator.
 *
 * Example:
 *     encodeHlt()  // -> 0xFFFFFFFF
 */
export function encodeHlt(): number {
  return HLT_INSTRUCTION;
}

// ---------------------------------------------------------------------------
// High-level simulator
// ---------------------------------------------------------------------------

/**
 * Complete ARM simulator -- ISA + CPU in one convenient class.
 *
 * This wraps the CPU simulator with the ARM decoder and executor,
 * providing a simple interface for running ARM programs.
 *
 * Example: running x = 1 + 2
 *
 *     const sim = new ARMSimulator();
 *     const program = assemble([
 *         encodeMovImm(0, 1),     // R0 = 1
 *         encodeMovImm(1, 2),     // R1 = 2
 *         encodeAdd(2, 0, 1),     // R2 = R0 + R1 = 3
 *         encodeHlt(),            // halt
 *     ]);
 *     const traces = sim.run(program);
 *     sim.cpu.registers.read(2);  // 3
 *
 *     The pipeline trace for each instruction shows:
 *     --- Cycle 0 ---
 *       FETCH              | DECODE             | EXECUTE
 *       PC: 0x0000         | mov                | R0 = 1
 *       -> 0xE3A00001      | rd=0 imm=1         | PC -> 4
 */
export class ARMSimulator {
  /** The ARM instruction decoder. */
  decoder: ARMDecoder;

  /** The ARM instruction executor. */
  executor: ARMExecutor;

  /** The underlying CPU that runs the fetch-decode-execute cycle. */
  cpu: CPU;

  constructor(memorySize: number = 65536) {
    this.decoder = new ARMDecoder();
    this.executor = new ARMExecutor();
    this.cpu = new CPU(
      this.decoder,
      this.executor,
      16, // ARM has 16 registers (R0-R15)
      32,
      memorySize
    );
  }

  /**
   * Load and run an ARM program, returning the pipeline trace.
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
 * ARM uses little-endian byte order (in its default configuration).
 * Each instruction is 4 bytes.
 *
 * This is a convenience function for creating test programs:
 *
 *     const program = assemble([
 *         encodeMovImm(0, 1),     // R0 = 1
 *         encodeMovImm(1, 2),     // R1 = 2
 *         encodeAdd(2, 0, 1),     // R2 = R0 + R1
 *         encodeHlt(),            // halt
 *     ]);
 */
export function assemble(instructions: number[]): number[] {
  const result: number[] = [];
  for (const instr of instructions) {
    const masked = (instr & 0xffffffff) >>> 0;
    result.push(
      masked & 0xff,
      (masked >>> 8) & 0xff,
      (masked >>> 16) & 0xff,
      (masked >>> 24) & 0xff
    );
  }
  return result;
}
