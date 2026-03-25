/**
 * Example programs for the Intel 4004 CPU step-through visualizer.
 *
 * Each program is a sequence of machine code bytes that demonstrates
 * a specific concept. Programs are designed to be short and instructive,
 * completing within 20-50 steps.
 *
 * === Intel 4004 instruction encoding ===
 *
 * Upper nibble (bits 7-4) = opcode family
 * Lower nibble (bits 3-0) = operand (register, immediate, etc.)
 *
 * Key opcodes used:
 *   0xD_ = LDM (load immediate 4-bit value into accumulator)
 *   0xB_ = XCH (exchange accumulator with register R_)
 *   0xA_ = LD  (load register R_ into accumulator)
 *   0x8_ = ADD (add register R_ to accumulator)
 *   0x9_ = SUB (subtract register R_ from accumulator)
 *   0x6_ = INC (increment register R_)
 *   0x7_ = ISZ (increment R_ and jump if not zero) [2-byte]
 *   0x4_ = JUN (unconditional jump to 12-bit address) [2-byte]
 *   0x1_ = JCN (conditional jump) [2-byte]
 *   0xF1 = CLC (clear carry)
 *   0xF0 = CLB (clear accumulator and carry)
 *   0x01 = HLT (halt)
 */

export interface ExampleProgram {
  /** Display name (i18n key). */
  nameKey: string;
  /** Brief description (i18n key). */
  descKey: string;
  /** The machine code bytes. */
  bytes: number[];
}

export const EXAMPLE_PROGRAMS: ExampleProgram[] = [
  {
    nameKey: "cpu.prog.count.name",
    descKey: "cpu.prog.count.desc",
    // Count from 0 to 5 in R0
    // 0x00: LDM 0   (D0) — load 0 into accumulator
    // 0x01: XCH R0  (B0) — store in R0
    // 0x02: INC R0  (60) — increment R0
    // 0x03: LD R0   (A0) — load R0 into accumulator
    // 0x04: SUB R1  (91) — subtract 6 (stored in R1) from accumulator
    // 0x05: JCN 0100 0x02 (14 02) — jump to 0x02 if acc != 0
    //                                (condition 4 = "not zero")
    // 0x07: HLT     (01)
    // We need to pre-load R1 with 6:
    // 0x00: LDM 6   (D6) — load 6
    // 0x01: XCH R1  (B1) — store in R1
    // 0x02: LDM 0   (D0) — load 0
    // 0x03: XCH R0  (B0) — store in R0 (counter = 0)
    // loop:
    // 0x04: INC R0  (60) — counter++
    // 0x05: LD R0   (A0) — load counter
    // 0x06: SUB R1  (91) — counter - 6
    // 0x07: JCN 4, 0x04 (14 04) — if not zero, loop
    // 0x09: HLT     (01)
    bytes: [0xd6, 0xb1, 0xd0, 0xb0, 0x60, 0xa0, 0x91, 0x14, 0x04, 0x01],
  },
  {
    nameKey: "cpu.prog.add.name",
    descKey: "cpu.prog.add.desc",
    // Add two numbers: 5 + 7 = 12
    // LDM 5  (D5) — load 5
    // XCH R0 (B0) — store 5 in R0
    // LDM 7  (D7) — load 7
    // XCH R1 (B1) — store 7 in R1
    // LD R0  (A0) — load 5 into accumulator
    // ADD R1 (81) — add 7: accumulator = 12
    // XCH R2 (B2) — store result in R2
    // HLT    (01)
    bytes: [0xd5, 0xb0, 0xd7, 0xb1, 0xa0, 0x81, 0xb2, 0x01],
  },
  {
    nameKey: "cpu.prog.fib.name",
    descKey: "cpu.prog.fib.desc",
    // Fibonacci: compute F(1) through F(7) in accumulator
    // R0 = prev (starts at 0)
    // R1 = curr (starts at 1)
    // R2 = temp
    // R3 = counter (starts at 6, counts down to 0)
    //
    // 0x00: LDM 0  (D0)  — prev = 0
    // 0x01: XCH R0 (B0)
    // 0x02: LDM 1  (D1)  — curr = 1
    // 0x03: XCH R1 (B1)
    // 0x04: LDM 6  (D6)  — counter = 6
    // 0x05: XCH R3 (B3)
    // loop:
    // 0x06: LD R1  (A1)  — acc = curr
    // 0x07: XCH R2 (B2)  — temp = curr
    // 0x08: LD R1  (A1)  — acc = curr
    // 0x09: ADD R0 (80)  — acc = curr + prev
    // 0x0A: XCH R1 (B1)  — curr = curr + prev
    // 0x0B: LD R2  (A2)  — acc = temp (old curr)
    // 0x0C: XCH R0 (B0)  — prev = old curr
    // 0x0D: ISZ R3, 0x06 (73 06) — counter++, if != 0 jump to loop
    //                               (R3: 6→7→8→9→A→B→0, stops at 0 after 10 wraps...
    //                                actually ISZ increments then jumps if NOT zero)
    // Hmm, ISZ increments and jumps if NOT zero. R3 starts at 6.
    // After 10 increments: 6→7→8→9→A→B→C→D→E→F→0 (wraps at 16)
    // That's 10 iterations. Let's use a simpler approach:
    // R3 starts at 0xA (10), after 6 increments: A→B→C→D→E→F→0, that's 6 iterations.
    // Let's start R3 at 0xA for 6 Fibonacci numbers.
    //
    // Revised:
    // 0x04: LDM 10 (DA) — counter = 10
    // After 6 increments: 10→11→12→13→14→15→0 → falls through
    bytes: [0xd0, 0xb0, 0xd1, 0xb1, 0xda, 0xb3, 0xa1, 0xb2, 0xa1, 0x80, 0xb1, 0xa2, 0xb0, 0x73, 0x06, 0x01],
  },
  {
    nameKey: "cpu.prog.branch.name",
    descKey: "cpu.prog.branch.desc",
    // Conditional branch: if A > B, store 1 in R2, else store 0
    // A=9, B=5
    // LDM 9  (D9) — load A
    // XCH R0 (B0) — store A in R0
    // LDM 5  (D5) — load B
    // XCH R1 (B1) — store B in R1
    // CLB    (F0) — clear accumulator and carry
    // LD R0  (A0) — load A
    // SUB R1 (91) — A - B (sets carry if no borrow)
    // JCN 2, skip (12 0E) — jump if carry=0 (borrow occurred, A < B)
    //                        condition 2 = "carry = 0"
    // LDM 1  (D1) — A >= B, load 1
    // XCH R2 (B2) — store result
    // JUN done (40 10) — jump to halt
    // skip:
    // LDM 0  (D0) — A < B, load 0
    // XCH R2 (B2)
    // done:
    // HLT    (01)
    bytes: [
      0xd9, 0xb0, 0xd5, 0xb1, 0xf0, 0xa0, 0x91,
      0x12, 0x0e,
      0xd1, 0xb2, 0x40, 0x10,
      0xd0, 0xb2,
      0x01,
    ],
  },
];
