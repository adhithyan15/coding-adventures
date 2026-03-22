/**
 * Instruction decoder -- combinational logic that maps opcodes to control signals.
 *
 * === How instruction decoding works in hardware ===
 *
 * The decoder takes an 8-bit instruction byte and produces control signals
 * that tell the rest of the CPU what to do. In the real 4004, this was a
 * combinational logic network -- a forest of AND, OR, and NOT gates that
 * pattern-match the opcode bits.
 *
 * For example, to detect LDM (0xD_):
 *     is_ldm = AND(bit7, bit6, NOT(bit5), bit4)  => bits 7654 = 1101
 *
 * The decoder doesn't use sequential logic -- it's purely combinational.
 * Given the same input bits, it always produces the same output signals.
 *
 * === Control signals ===
 *
 * The decoder outputs tell the control unit what to do:
 *     - write_acc:    Write a value to the accumulator
 *     - write_reg:    Write a value to the register file
 *     - write_carry:  Update the carry flag
 *     - alu_add:      Route through ALU add
 *     - alu_sub:      Route through ALU subtract
 *     - is_jump:      This is a jump instruction
 *     - is_call:      This is JMS (push return address)
 *     - is_return:    This is BBL (pop and return)
 *     - is_two_byte:  Instruction is 2 bytes
 *     - uses_ram:     Instruction accesses RAM
 *     - reg_index:    Which register (lower nibble)
 *     - pair_index:   Which register pair
 *     - immediate:    Immediate value from instruction
 */

import { AND, NOT, OR, type Bit } from "@coding-adventures/logic-gates";

/**
 * Control signals produced by the instruction decoder.
 *
 * Every field represents a wire carrying a 0 or 1 signal, or a
 * multi-bit value extracted from the instruction.
 */
export interface DecodedInstruction {
  /** Original instruction bytes */
  raw: number;
  raw2: number | null;

  /** Upper and lower nibbles */
  upper: number;
  lower: number;

  /** Instruction family detection (from gate logic) */
  isNop: Bit;
  isHlt: Bit;
  isLdm: Bit;
  isLd: Bit;
  isXch: Bit;
  isInc: Bit;
  isAdd: Bit;
  isSub: Bit;
  isJun: Bit;
  isJcn: Bit;
  isIsz: Bit;
  isJms: Bit;
  isBbl: Bit;
  isFim: Bit;
  isSrc: Bit;
  isFin: Bit;
  isJin: Bit;
  isIo: Bit;
  isAccum: Bit;

  /** Two-byte flag */
  isTwoByte: Bit;

  /** Operand extraction */
  regIndex: number;
  pairIndex: number;
  immediate: number;
  condition: number;

  /** For 2-byte instructions */
  addr12: number;
  addr8: number;
}

/**
 * Decode an instruction byte into control signals using gates.
 *
 * In real hardware, this is a combinational circuit -- no clock needed.
 * The input bits propagate through AND/OR/NOT gate trees to produce
 * the output control signals.
 *
 * @param raw - First instruction byte (0x00-0xFF).
 * @param raw2 - Second byte for 2-byte instructions, or null.
 * @returns DecodedInstruction with all control signals set.
 */
export function decode(raw: number, raw2: number | null = null): DecodedInstruction {
  // Extract individual bits using AND gates (masking)
  const b7 = ((raw >> 7) & 1) as Bit;
  const b6 = ((raw >> 6) & 1) as Bit;
  const b5 = ((raw >> 5) & 1) as Bit;
  const b4 = ((raw >> 4) & 1) as Bit;
  const b3 = ((raw >> 3) & 1) as Bit;
  const b2 = ((raw >> 2) & 1) as Bit;
  const b1 = ((raw >> 1) & 1) as Bit;
  const b0 = (raw & 1) as Bit;

  const upper = (raw >> 4) & 0xf;
  const lower = raw & 0xf;

  // --- Instruction family detection ---
  // Each family is detected by AND-ing the upper nibble bits.
  // Using NOT for inverted bits.

  // NOP = 0x00: all bits zero
  let isNop = AND(
    AND(NOT(b7), NOT(b6)),
    AND(AND(NOT(b5), NOT(b4)), AND(NOT(b3), NOT(b2))),
  );
  isNop = AND(isNop, AND(NOT(b1), NOT(b0)));

  // HLT = 0x01: only b0 is 1
  let isHlt = AND(
    AND(NOT(b7), NOT(b6)),
    AND(AND(NOT(b5), NOT(b4)), AND(NOT(b3), NOT(b2))),
  );
  isHlt = AND(isHlt, AND(NOT(b1), b0));

  // Upper nibble patterns (using gate logic):
  // 0x1_ = 0001 : JCN
  const isJcnFamily = AND(AND(NOT(b7), NOT(b6)), AND(NOT(b5), b4));

  // 0x2_ = 0010 : FIM (even b0) or SRC (odd b0)
  const is2x = AND(AND(NOT(b7), NOT(b6)), AND(b5, NOT(b4)));
  const isFim = AND(is2x, NOT(b0));
  const isSrc = AND(is2x, b0);

  // 0x3_ = 0011 : FIN (even b0) or JIN (odd b0)
  const is3x = AND(AND(NOT(b7), NOT(b6)), AND(b5, b4));
  const isFin = AND(is3x, NOT(b0));
  const isJin = AND(is3x, b0);

  // 0x4_ = 0100 : JUN
  const isJunFamily = AND(AND(NOT(b7), b6), AND(NOT(b5), NOT(b4)));

  // 0x5_ = 0101 : JMS
  const isJmsFamily = AND(AND(NOT(b7), b6), AND(NOT(b5), b4));

  // 0x6_ = 0110 : INC
  const isIncFamily = AND(AND(NOT(b7), b6), AND(b5, NOT(b4)));

  // 0x7_ = 0111 : ISZ
  const isIszFamily = AND(AND(NOT(b7), b6), AND(b5, b4));

  // 0x8_ = 1000 : ADD
  const isAddFamily = AND(AND(b7, NOT(b6)), AND(NOT(b5), NOT(b4)));

  // 0x9_ = 1001 : SUB
  const isSubFamily = AND(AND(b7, NOT(b6)), AND(NOT(b5), b4));

  // 0xA_ = 1010 : LD
  const isLdFamily = AND(AND(b7, NOT(b6)), AND(b5, NOT(b4)));

  // 0xB_ = 1011 : XCH
  const isXchFamily = AND(AND(b7, NOT(b6)), AND(b5, b4));

  // 0xC_ = 1100 : BBL
  const isBblFamily = AND(AND(b7, b6), AND(NOT(b5), NOT(b4)));

  // 0xD_ = 1101 : LDM
  const isLdmFamily = AND(AND(b7, b6), AND(NOT(b5), b4));

  // 0xE_ = 1110 : I/O operations
  const isIoFamily = AND(AND(b7, b6), AND(b5, NOT(b4)));

  // 0xF_ = 1111 : accumulator operations
  const isAccumFamily = AND(AND(b7, b6), AND(b5, b4));

  // Two-byte detection
  const isTwoByte = OR(
    OR(isJcnFamily, isJunFamily),
    OR(OR(isJmsFamily, isIszFamily), isFim),
  );

  // Operand extraction
  const regIndex = lower;
  const pairIndex = lower >> 1;
  const immediate = lower;
  const condition = lower;

  // 12-bit address for JUN/JMS
  const second = raw2 !== null ? raw2 : 0;
  const addr12 = (lower << 8) | second;
  const addr8 = second;

  return {
    raw,
    raw2,
    upper,
    lower,
    isNop,
    isHlt,
    isLdm: isLdmFamily,
    isLd: isLdFamily,
    isXch: isXchFamily,
    isInc: isIncFamily,
    isAdd: isAddFamily,
    isSub: isSubFamily,
    isJun: isJunFamily,
    isJcn: isJcnFamily,
    isIsz: isIszFamily,
    isJms: isJmsFamily,
    isBbl: isBblFamily,
    isFim,
    isSrc,
    isFin,
    isJin,
    isIo: isIoFamily,
    isAccum: isAccumFamily,
    isTwoByte,
    regIndex,
    pairIndex,
    immediate,
    condition,
    addr12,
    addr8,
  };
}
