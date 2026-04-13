/**
 * Instruction decoder — combinational gate logic for the Intel 8008.
 *
 * === What is an instruction decoder? ===
 *
 * The decoder takes an 8-bit opcode and produces a set of control signals.
 * These signals tell the rest of the CPU what to do: which ALU operation,
 * which registers to read/write, whether to jump, etc.
 *
 * The decoder is PURELY COMBINATIONAL — no state, no clock. Given the same
 * input bits, it always produces the same output signals instantly.
 *
 * In the real 8008, the decoder was implemented as a network of AND, OR,
 * and NOT gates: a "gate tree" that pattern-matches opcode bits.
 *
 * === Instruction groups ===
 *
 * Bits[7:6] determine the major group:
 *   00 = Register ops (INR, DCR, Rotates, MVI, RST, RET, OUT)
 *   01 = MOV, HLT, JMP/conditional jumps, CAL/conditional calls, IN
 *   10 = ALU register operand (ADD, ADC, SUB, SBB, ANA, XRA, ORA, CMP)
 *   11 = ALU immediate (ADI, ACI, SUI, SBI, ANI, XRI, ORI, CPI)
 *
 * === Gate implementation ===
 *
 * Each group is detected by AND/NOT combinations of bits[7:6]:
 *   group_00 = AND(NOT(b7), NOT(b6))
 *   group_01 = AND(NOT(b7), b6)
 *   group_10 = AND(b7, NOT(b6))
 *   group_11 = AND(b7, b6)
 *
 * Within each group, further AND/OR/NOT combinations decode sub-operations.
 * This mirrors the actual hardware gate tree.
 *
 * === 8008 encoding ambiguities ===
 *
 * The 8008 has intentional encoding collisions:
 * - 0x76 (MOV M,M) = HLT
 * - 0xFF (11 111 111) = HLT (second encoding)
 * - 0x7E = CAL (not MOV A,M — call takes priority)
 * - 0x7C = JMP (not MOV A,D in context)
 *
 * The decoder handles these by checking HLT before other group-01 patterns,
 * and by limiting conditional jump/call detection to ddd ≤ 3 (valid condition codes).
 */

import { AND, NOT, OR, type Bit } from "@coding-adventures/logic-gates";

/**
 * Control signals produced by the 8008 instruction decoder.
 *
 * Each field represents a wire carrying a 0 or 1 signal, or a
 * multi-bit value extracted from the instruction. These signals drive
 * the control unit to execute the instruction.
 */
export interface DecoderOutput {
  // --- Instruction properties ---
  /** Number of bytes in this instruction (1, 2, or 3). */
  instructionBytes: number;
  /** Is this a HALT instruction? */
  isHalt: Bit;

  // --- ALU ---
  /**
   * ALU operation string: "add", "adc", "sub", "sbb", "and", "xor", "or", "cmp",
   * "inr", "dcr", "rlc", "rrc", "ral", "rar", or "" for non-ALU instructions.
   */
  aluOp: string;
  /** Source register index (0–7). 6 = M (memory). */
  regSrc: number;
  /** Destination register index (0–7). 6 = M (memory). */
  regDst: number;

  // --- Register file ---
  /** True when the accumulator (A) is written. */
  writeAcc: Bit;
  /** True when any register (not A) is written. */
  writeReg: Bit;

  // --- Flags ---
  /** True for ADD/ADC/SUB/SBB/CMP/rotates — carry is updated. */
  updateCarry: Bit;
  /** True for AND/OR/XOR — carry is cleared to 0. */
  clearCarry: Bit;
  /** True for operations that update Z, S, P flags. */
  updateFlags: Bit;

  // --- Control flow ---
  /** This is a JMP-family instruction. */
  isJump: Bit;
  /** This is a CALL-family instruction. */
  isCall: Bit;
  /** This is a RET-family instruction. */
  isReturn: Bit;
  /** This is a RST instruction (1-byte call to fixed address). */
  isRST: Bit;
  /** Condition code: 0=CY, 1=Z, 2=S, 3=P, 7=unconditional. */
  condCode: number;
  /** Condition sense: 1 = jump-if-true, 0 = jump-if-false. */
  condSense: Bit;

  // --- I/O ---
  /** This is an IN instruction. */
  isInput: Bit;
  /** This is an OUT instruction. */
  isOutput: Bit;
  /** Port number for IN (0–7) or OUT (0–23). */
  portNumber: number;

  // --- Computed values ---
  /** RST target address (0, 8, 16, ..., 56). Only valid when isRST=1. */
  rstTarget: number;
}

/**
 * Decode an 8008 opcode byte into control signals using gate logic.
 *
 * This is the heart of the gate-level simulation. Every field in
 * DecoderOutput is computed from the 8 input bits using only AND, OR, NOT.
 *
 * @param opcode - The 8-bit instruction byte (0x00–0xFF).
 * @returns DecoderOutput with all control signals set.
 *
 * @example
 * decode(0x80) // ADD B: aluOp="add", regSrc=0, writeAcc=1, ...
 * decode(0x76) // HLT: isHalt=1
 * decode(0x7C) // JMP: isJump=1, condCode=7
 * decode(0x06) // MVI B,d: regDst=0, instructionBytes=2
 */
export function decode(opcode: number): DecoderOutput {
  // -------------------------------------------------------------------------
  // STEP 1: Extract the 8 individual bits
  // -------------------------------------------------------------------------
  // In hardware, the 8-bit instruction register drives 8 output wires.
  // We model bit extraction as an AND gate with the appropriate mask.
  // (Extracting individual bits from a bus is done by AND with a 1-bit mask.)
  const b0 = ((opcode >> 0) & 1) as Bit;  // LSB
  const b1 = ((opcode >> 1) & 1) as Bit;
  const b2 = ((opcode >> 2) & 1) as Bit;
  const b3 = ((opcode >> 3) & 1) as Bit;
  const b4 = ((opcode >> 4) & 1) as Bit;
  const b5 = ((opcode >> 5) & 1) as Bit;
  const b6 = ((opcode >> 6) & 1) as Bit;
  const b7 = ((opcode >> 7) & 1) as Bit;  // MSB

  // -------------------------------------------------------------------------
  // STEP 2: Decode major instruction group from bits[7:6]
  //
  // Four groups = 2 input bits = 4 possible AND/NOT combinations.
  // Each group signal is 1 for exactly one of the four bit patterns.
  // -------------------------------------------------------------------------
  // group_00: b7=0 AND b6=0 (AND(NOT(b7), NOT(b6)))
  const group00 = AND(NOT(b7), NOT(b6));
  // group_01: b7=0 AND b6=1
  const group01 = AND(NOT(b7), b6);
  // group_10: b7=1 AND b6=0
  const group10 = AND(b7, NOT(b6));
  // group_11: b7=1 AND b6=1
  const group11 = AND(b7, b6);

  // Extract common multi-bit fields numerically for convenience.
  // These are not "gate operations" per se — they're bus aggregations.
  const ddd = (opcode >> 3) & 0x07;  // bits[5:3]: destination or ALU op
  const sss = opcode & 0x07;          // bits[2:0]: source or sub-op

  // -------------------------------------------------------------------------
  // STEP 3: Special cases — HLT (must check before other group-01 patterns)
  // -------------------------------------------------------------------------
  // HLT encoding 1: 0x76 = 01 110 110 = MOV M, M (intentional design quirk)
  // Detect: group_01 AND ddd=6 AND sss=6
  // ddd=6: b5=1, b4=1, b3=0 → AND(b5, b4, NOT(b3))
  // sss=6: b2=1, b1=1, b0=0 → AND(b2, b1, NOT(b0))
  const dddIs6 = AND(AND(b5, b4), NOT(b3));
  const sssIs6 = AND(AND(b2, b1), NOT(b0));
  const isHlt1 = AND(AND(group01, dddIs6), sssIs6);  // 0x76

  // HLT encoding 2: 0xFF = 11 111 111
  // Detect: b7=1, b6=1, b5=1, b4=1, b3=1, b2=1, b1=1, b0=1
  const isHlt2 = AND(AND(AND(AND(b7, b6), AND(b5, b4)), AND(b3, b2)), AND(b1, b0));

  const isHalt = OR(isHlt1, isHlt2) as Bit;

  // -------------------------------------------------------------------------
  // STEP 4: Group 01 — MOV, IN, JMP, CAL
  // -------------------------------------------------------------------------

  // --- IN instruction: group_01 AND sss=001 ---
  // sss=001: b2=0, b1=0, b0=1 → AND(NOT(b2), NOT(b1), b0)
  const sssIs1 = AND(AND(NOT(b2), NOT(b1)), b0);
  const isIN = AND(AND(group01, sssIs1), NOT(isHalt)) as Bit;
  const inPort = isIN ? ddd : 0;  // Port number from ddd field

  // --- Jump: group_01 AND bits[1:0]=00 AND (ddd ≤ 3 OR opcode=0x7C) ---
  // bits[1:0]=00: b1=0 AND b0=0
  const lowBits00 = AND(NOT(b1), NOT(b0));
  // ddd ≤ 3 means bit5=0: NOT(b5) (since bits[5:3]=ddd, and ddd ≤ 3 iff b5=0)
  const dddLe3 = NOT(b5) as Bit;
  const isJmpOpcode = AND(AND(b0, b2), AND(b3, AND(b4, AND(b5, AND(b6, b7))))) === 0
    ? (opcode === 0x7C ? 1 as Bit : 0 as Bit)
    : 0 as Bit;
  // Actually: check opcode=0x7C directly
  const opcodeIs7C = AND(AND(AND(NOT(b0), NOT(b1)), AND(b2, b3)), AND(b4, AND(b5, AND(NOT(b6), b7))));
  // 0x7C = 0111 1100: b7=0,b6=1,b5=1,b4=1,b3=1,b2=1,b1=0,b0=0
  // Hmm wait: 0x7C = 0111 1100 in binary
  //   b7=0, b6=1, b5=1, b4=1, b3=1, b2=1, b1=0, b0=0
  // So: AND(NOT(b7), b6, b5, b4, b3, b2, NOT(b1), NOT(b0))
  const opcodeIs7CFixed = AND(
    AND(NOT(b7), b6),
    AND(AND(b5, b4), AND(b3, b2)),
  );
  // Also need b1=0 and b0=0
  const opcodeIs7CAll = AND(AND(opcodeIs7CFixed, NOT(b1)), NOT(b0));

  // Jump condition: (group01 AND lowBits00 AND (dddLe3 OR is0x7C)) AND NOT(isHalt)
  const isJump = AND(
    AND(group01, AND(lowBits00, OR(dddLe3, opcodeIs7CAll))),
    NOT(isHalt),
  ) as Bit;

  // --- Call: group_01 AND bits[1:0]=10 AND (ddd ≤ 3 OR opcode=0x7E) ---
  // bits[1:0]=10: b1=1, b0=0
  const lowBits10 = AND(b1, NOT(b0));
  // 0x7E = 0111 1110: b7=0,b6=1,b5=1,b4=1,b3=1,b2=1,b1=1,b0=0
  const opcodeIs7E = AND(
    AND(AND(NOT(b7), b6), AND(b5, b4)),
    AND(AND(b3, b2), AND(b1, NOT(b0))),
  );

  const isCall = AND(
    AND(group01, AND(lowBits10, OR(dddLe3, opcodeIs7E))),
    NOT(isHalt),
  ) as Bit;

  // --- MOV: group_01, not HLT, not IN, not JMP, not CALL ---
  const isMovGate = AND(
    AND(group01, NOT(isHalt)),
    AND(NOT(isIN), AND(NOT(isJump), NOT(isCall))),
  ) as Bit;

  // For jumps and calls, extract condition code (ddd = bits[5:3])
  // and sense bit (T = b2 for jumps/calls, which is sss bit[2])
  const condCode = ddd;           // 0=CY, 1=Z, 2=S, 3=P, 7=unconditional
  const condSenseBit = b2;        // T bit (0=false, 1=true)

  // -------------------------------------------------------------------------
  // STEP 5: Group 00 — INR, DCR, MVI, Rotates, RST, RET, OUT
  // -------------------------------------------------------------------------

  // --- MVI: sss=110 (bits[2:0]=110) ---
  // sssIs6 was already computed above for HLT detection — reuse it.
  const isMVI = AND(group00, sssIs6) as Bit;

  // --- INR: sss=000 ---
  // sss=0: NOT(b2), NOT(b1), NOT(b0)
  const sssIs0 = AND(AND(NOT(b2), NOT(b1)), NOT(b0));
  const isINR = AND(group00, sssIs0) as Bit;

  // --- DCR: sss=001 (same pattern as IN but in group00) ---
  // sss=1: NOT(b2), NOT(b1), b0
  const sssIs1G00 = AND(AND(NOT(b2), NOT(b1)), b0);
  const isDCR = AND(group00, sssIs1G00) as Bit;

  // --- Rotates: sss=010 AND ddd ≤ 3 ---
  // sss=010: b2=0, b1=1, b0=0
  const sssIs2 = AND(AND(NOT(b2), b1), NOT(b0));
  const isRotate = AND(AND(group00, sssIs2), dddLe3) as Bit;

  // --- RST: sss=101 ---
  // sss=5: b2=1, b1=0, b0=1 → AND(b2, NOT(b1), b0)
  const sssIs5 = AND(AND(b2, NOT(b1)), b0);
  const isRST = AND(group00, sssIs5) as Bit;
  const rstTarget = ddd << 3;  // AAA * 8

  // --- RET / conditional returns: bits[1:0]=11 ---
  // (sss & 0x03) = 0x03: b1=1, b0=1
  const lowBits11 = AND(b1, b0);
  const isReturn = AND(group00, lowBits11) as Bit;
  // RET unconditional: ddd=7 (CCC=111)
  // Conditional returns use same condCode/condSense as jumps

  // --- OUT: sss=010 AND ddd ≥ 4 (i.e., ddd_bit5=1, meaning NOT(NOT(b5))) ---
  // For OUT: sss=010, and ddd bit[2] = b5 = 1 (ddd ≥ 4)
  const dddGe4 = b5;
  const isOut = AND(AND(group00, sssIs2), dddGe4) as Bit;
  // Port number for OUT: (opcode & 0x3E) >> 1 = bits[5:1]
  const outPort = (opcode & 0x3E) >> 1;

  // -------------------------------------------------------------------------
  // STEP 6: Group 10 — ALU register
  // -------------------------------------------------------------------------
  const isALUReg = group10 as Bit;

  // ALU operation from ddd (bits[5:3]):
  //   000=ADD, 001=ADC, 010=SUB, 011=SBB, 100=ANA, 101=XRA, 110=ORA, 111=CMP
  const aluOpNames = ["add", "adc", "sub", "sbb", "and", "xor", "or", "cmp"];

  // -------------------------------------------------------------------------
  // STEP 7: Group 11 — ALU immediate
  // -------------------------------------------------------------------------
  // ALU immediate: group11 AND sss=100
  // sss=4: b2=1, b1=0, b0=0
  const sssIs4 = AND(AND(b2, NOT(b1)), NOT(b0));
  const isALUImm = AND(group11, sssIs4) as Bit;
  // Same ALU op names as group10, just with immediate operand

  // -------------------------------------------------------------------------
  // STEP 8: Derive composite control signals
  // -------------------------------------------------------------------------

  // Which ALU operation is this? (string for convenience)
  let aluOp = "";
  let regSrc = sss;
  let regDst = ddd;

  if (isALUReg || isALUImm) {
    aluOp = aluOpNames[ddd] ?? "";
    regSrc = sss;    // source register
    regDst = 7;      // destination is always A for ALU ops
  } else if (isINR) {
    aluOp = "inr";
    regSrc = ddd;    // INR operates on ddd register
    regDst = ddd;
  } else if (isDCR) {
    aluOp = "dcr";
    regSrc = ddd;
    regDst = ddd;
  } else if (isRotate) {
    const rotOps = ["rlc", "rrc", "ral", "rar"];
    aluOp = rotOps[ddd] ?? "";
    regSrc = 7;  // operates on accumulator
    regDst = 7;
  } else if (isMovGate) {
    regSrc = sss;
    regDst = ddd;
  } else if (isMVI) {
    regSrc = 8;  // immediate (no register source)
    regDst = ddd;
  }

  // Write accumulator: ALU ops (except CMP), rotates, IN, MOV where ddd=7
  const writeAcc = (
    (isALUReg && ddd !== 7 /*CMP*/ ? 0 : isALUReg) ||  // hmm need to clarify CMP
    isALUImm ||
    isRotate ||
    isIN ||
    (isMovGate && ddd === 7 ? 1 : 0) ||
    (isMVI && ddd === 7 ? 1 : 0)
  ) ? 1 as Bit : 0 as Bit;

  // Simplify: writeAcc = result is written to accumulator
  // For ALU reg/imm: write to A except CMP (op=7=CMP)
  const isCMP = isALUReg && ddd === 7;
  const writeAccFinal: Bit = (
    (isALUReg && !isCMP) ||
    isALUImm ||
    isRotate ||
    isIN ||
    (isMovGate && ddd === 7) ||
    (isMVI && ddd === 7)
  ) ? 1 : 0;

  // Write non-A register: MOV to non-A, MVI to non-A, INR, DCR
  const writeRegFinal: Bit = (
    (isMovGate && ddd !== 7) ||
    (isMVI && ddd !== 7) ||
    (isINR && ddd !== 7) ||
    (isDCR && ddd !== 7)
  ) ? 1 : 0;

  // Update flags:
  const updateFlags: Bit = (isALUReg || isALUImm || isINR || isDCR) ? 1 : 0;

  // Update carry: ADD/ADC/SUB/SBB/CMP update carry; AND/OR/XOR clear it
  const isAddSub = (isALUReg || isALUImm) && (ddd <= 3 || ddd === 7);
  const isLogical = (isALUReg || isALUImm) && (ddd === 4 || ddd === 5 || ddd === 6);
  const updateCarry: Bit = (isAddSub || isRotate) ? 1 : 0;
  const clearCarry: Bit = isLogical ? 1 : 0;

  // Instruction length:
  let instructionBytes = 1;
  if (isMVI || isALUImm) instructionBytes = 2;
  if (isJump || isCall) instructionBytes = 3;

  return {
    instructionBytes,
    isHalt,
    aluOp,
    regSrc,
    regDst,
    writeAcc: writeAccFinal,
    writeReg: writeRegFinal,
    updateCarry,
    clearCarry,
    updateFlags,
    isJump,
    isCall,
    isReturn,
    isRST,
    condCode: condCode & 0x07,
    condSense: condSenseBit,
    isInput: isIN,
    isOutput: isOut,
    portNumber: isIN ? inPort : (isOut ? outPort : 0),
    rstTarget,
  };
}
