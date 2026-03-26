/**
 * ==========================================================================
 * ARM1 Demo Programs — Pre-assembled Machine Code
 * ==========================================================================
 *
 * Each program is a list of 32-bit ARM1 instructions encoded in
 * little-endian byte order. All programs terminate with SWI #0x123456
 * which the ARM1 simulator treats as a halt signal.
 *
 * # How ARM1 Immediates Work
 *
 * The ARM1 doesn't fit arbitrary 32-bit constants into 12 bits. Instead,
 * it encodes them as an 8-bit value rotated right by an even number of
 * positions:
 *
 *   actual_value = imm8 ROR (rotate_field × 2)
 *
 * So to encode 0x200 (512): 0x200 = 0x02 ROR 24 = 0x02 with rotate=12
 * In the instruction: bits 11:8 = 12 (0xC), bits 7:0 = 2 → 0x...C02
 *
 * # Branch Offset Calculation
 *
 * ARM1 branches encode a signed 24-bit word offset. The CPU adds this
 * (shifted left 2) to PC+8 (accounting for the 3-stage pipeline):
 *
 *   target = PC_at_branch + 8 + (signed_offset × 4)
 *   offset = (target - PC_at_branch - 8) / 4
 */

/** Convert a 32-bit word to 4 bytes in little-endian order. */
function word(n: number): number[] {
  n = n >>> 0;
  return [n & 0xFF, (n >>> 8) & 0xFF, (n >>> 16) & 0xFF, (n >>> 24) & 0xFF];
}

/** Pack a sequence of 32-bit instruction words into a byte array. */
function instructions(...words: number[]): number[] {
  return words.flatMap(word);
}

export interface Program {
  /** Display name shown in the selector. */
  name: string;
  /** One-sentence description of what the program computes. */
  description: string;
  /** Machine code bytes loaded at address 0x0000. */
  code: number[];
  /** Optional data bytes loaded at dataAddr. */
  data?: number[];
  /** Address where data bytes are placed (default: none). */
  dataAddr?: number;
  /** Human-readable description of the expected result. */
  expectedResult: string;
  /** Assembly listing for the Decode view tooltip. */
  listing: string[];
}

// ==========================================================================
// Program 1: Fibonacci
// ==========================================================================
//
// Compute fib(10) = 55 using an iterative algorithm.
//
// The Fibonacci sequence: 0, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55 ...
// After 10 loop iterations, R1 holds fib(10) = 55.
//
//   R0 = n     (countdown from 10 to 0)
//   R1 = a     (previous Fibonacci number, starts 0)
//   R2 = b     (current Fibonacci number, starts 1)
//   R3 = temp  (scratch for computing a+b)
//
// Loop body: temp = a+b, a = b, b = temp, n--
//
// Instruction layout (word addresses):
//   0x00  MOV R0, #10       E3A0000A  R0 ← 10
//   0x04  MOV R1, #0        E3A01000  R1 ← 0  (fib(0))
//   0x08  MOV R2, #1        E3A02001  R2 ← 1  (fib(1))
//   0x0C  CMP R0, #0        E3500000  ← loop: set flags on R0
//   0x10  BEQ done  (+4)    0A000004  if Z=1, jump to 0x28
//   0x14  SUB R0, R0, #1    E2400001  R0 ← R0-1
//   0x18  ADD R3, R1, R2    E0813002  R3 ← R1+R2 (next fib)
//   0x1C  MOV R1, R2        E1A01002  R1 ← R2
//   0x20  MOV R2, R3        E1A02003  R2 ← R3
//   0x24  B   loop  (-8)    EAFFFFF8  jump back to 0x0C
//   0x28  MOV R0, R1        E1A00001  ← done: R0 ← result
//   0x2C  SWI #0x123456     EF123456  halt

export const FIBONACCI: Program = {
  name: "Fibonacci",
  description: "Compute fib(10) = 55 using iterative addition. Demonstrates CMP, BEQ, and loop structure.",
  expectedResult: "R0 = 55 (fib(10))",
  listing: [
    "0x00  MOV R0, #10          ; n = 10",
    "0x04  MOV R1, #0           ; a = 0 = fib(0)",
    "0x08  MOV R2, #1           ; b = 1 = fib(1)",
    "0x0C  CMP R0, #0           ; loop: flags ← R0 - 0",
    "0x10  BEQ done             ;   if Z=1 (R0=0), branch to done",
    "0x14  SUB R0, R0, #1       ;   n--",
    "0x18  ADD R3, R1, R2       ;   temp ← a + b",
    "0x1C  MOV R1, R2           ;   a ← b",
    "0x20  MOV R2, R3           ;   b ← temp",
    "0x24  B   loop             ;   repeat",
    "0x28  MOV R0, R1           ; done: R0 ← result",
    "0x2C  SWI #0x123456        ; halt",
  ],
  code: instructions(
    0xE3A0000A,  // MOV R0, #10
    0xE3A01000,  // MOV R1, #0
    0xE3A02001,  // MOV R2, #1
    0xE3500000,  // CMP R0, #0        (loop)
    0x0A000004,  // BEQ +4            → 0x28 done
    0xE2400001,  // SUB R0, R0, #1
    0xE0813002,  // ADD R3, R1, R2
    0xE1A01002,  // MOV R1, R2
    0xE1A02003,  // MOV R2, R3
    0xEAFFFFF8,  // B   -8            → 0x0C loop
    0xE1A00001,  // MOV R0, R1        (done)
    0xEF123456,  // SWI #0x123456
  ),
};

// ==========================================================================
// Program 2: Sum 1..10
// ==========================================================================
//
// Accumulate 10 + 9 + 8 + ... + 1 = 55.
//
// Uses SUBS (SUB with flag update) to decrement the counter and test
// for zero in a single instruction — no CMP needed.
//
//   R0 = counter (10 down to 0)
//   R1 = sum (accumulates R0 each iteration)
//
//   0x00  MOV R0, #10        E3A0000A  R0 ← 10
//   0x04  MOV R1, #0         E3A01000  R1 ← 0
//   0x08  ADD R1, R1, R0     E0811000  ← loop: R1 += R0
//   0x0C  SUBS R0, R0, #1    E2500001  R0 ← R0-1, set flags
//   0x10  BNE loop  (-4)     1AFFFFFC  if Z=0, jump back
//   0x14  SWI #0x123456      EF123456  halt

export const SUM_1_TO_10: Program = {
  name: "Sum 1..10",
  description: "Accumulate 10+9+…+1 = 55 using SUBS+BNE. The SUBS instruction sets flags without a separate CMP.",
  expectedResult: "R1 = 55",
  listing: [
    "0x00  MOV R0, #10          ; counter = 10",
    "0x04  MOV R1, #0           ; sum = 0",
    "0x08  ADD R1, R1, R0       ; loop: sum += counter",
    "0x0C  SUBS R0, R0, #1      ;   counter--, set Z if counter=0",
    "0x10  BNE loop             ;   if Z=0, branch back",
    "0x14  SWI #0x123456        ; halt",
  ],
  code: instructions(
    0xE3A0000A,  // MOV R0, #10
    0xE3A01000,  // MOV R1, #0
    0xE0811000,  // ADD R1, R1, R0    (loop)
    0xE2500001,  // SUBS R0, R0, #1
    0x1AFFFFFC,  // BNE -4            → 0x08 loop
    0xEF123456,  // SWI #0x123456
  ),
};

// ==========================================================================
// Program 3: Array Maximum
// ==========================================================================
//
// Find the maximum value in [5, 2, 8, 1, 9, 3, 7], terminated with 0.
// Demonstrates post-index LDR, conditional execution (MOVGT), and CMP.
//
// Key ARM1 feature: EVERY instruction has a 4-bit condition code.
// MOVGT executes only when the "Greater Than" condition holds (Z=0 & N=V).
// This avoids a branch instruction entirely.
//
//   R0 = pointer (walks through array at 0x200)
//   R1 = running maximum
//   R2 = current element
//
//   0x00  MOV R0, #0x200      E3A00C02  R0 ← 0x200 (array base)
//   0x04  MOV R1, #0          E3A01000  R1 ← 0 (max = 0)
//   0x08  LDR R2,[R0],#4      E4902004  ← loop: load word, R0 += 4
//   0x0C  CMP R2, #0          E3520000  flags ← R2 - 0
//   0x10  BEQ done  (+2)      0A000002  if zero sentinel, halt
//   0x14  CMP R2, R1          E1520001  flags ← R2 - R1
//   0x18  MOVGT R1, R2        C1A01002  if R2>R1: max ← R2
//   0x1C  B   loop  (-7)      EAFFFFF9  repeat
//   0x20  SWI #0x123456       EF123456  ← done

export const ARRAY_MAX: Program = {
  name: "Array Max",
  description: "Find maximum in [5,2,8,1,9,3,7,0]. Shows post-index LDR and MOVGT conditional execution — no branch needed.",
  expectedResult: "R1 = 9 (maximum)",
  listing: [
    "0x00  MOV R0, #0x200       ; R0 = pointer to array at 0x200",
    "0x04  MOV R1, #0           ; max = 0",
    "0x08  LDR R2, [R0], #4     ; loop: R2 = *R0, R0 += 4",
    "0x0C  CMP R2, #0           ;   is this the zero sentinel?",
    "0x10  BEQ done             ;   yes → done",
    "0x14  CMP R2, R1           ;   R2 vs current max",
    "0x18  MOVGT R1, R2         ;   if R2 > R1: max = R2  (conditional!)",
    "0x1C  B   loop             ;   next element",
    "0x20  SWI #0x123456        ; done: halt",
  ],
  code: instructions(
    0xE3A00C02,  // MOV R0, #0x200    (imm8=2, rotate=12 → 2 ROR 24 = 0x200)
    0xE3A01000,  // MOV R1, #0
    0xE4902004,  // LDR R2, [R0], #4  (loop)
    0xE3520000,  // CMP R2, #0
    0x0A000002,  // BEQ +2            → 0x20 done
    0xE1520001,  // CMP R2, R1
    0xC1A01002,  // MOVGT R1, R2      (cond GT = 0xC)
    0xEAFFFFF9,  // B   -7            → 0x08 loop
    0xEF123456,  // SWI #0x123456     (done)
  ),
  data: [
    ...word(5), ...word(2), ...word(8), ...word(1),
    ...word(9), ...word(3), ...word(7), ...word(0),
  ],
  dataAddr: 0x200,
};

// ==========================================================================
// Program 4: Barrel Shifter Demo
// ==========================================================================
//
// Execute four MOV instructions that each apply a different shift type
// to Operand2. After each step, watch the Barrel Shifter tab to see
// the bits moving.
//
// The ARM1's barrel shifter operates entirely within the data path —
// shifting happens "for free" without consuming extra clock cycles.
//
// Instruction encoding of shift operands (register, immediate shift amount):
//
//   bits 11:7 = shift_imm (5 bits)
//   bits  6:5 = shift_type (00=LSL, 01=LSR, 10=ASR, 11=ROR)
//   bit   4   = 0 (immediate, not register shift)
//   bits  3:0 = Rm
//
//   Operand2 = (shift_imm << 7) | (shift_type << 5) | Rm
//
//   Shift  Input       Amount  Encoding  Result
//   LSL    0x000000A5  8       (8<<7)|(0<<5)|0 = 0x400  → 0x0000A500
//   LSR    0x0000A500  4       (4<<7)|(1<<5)|0 = 0x220  → 0x00000A50
//   ASR    0x0000A500  4       (4<<7)|(2<<5)|0 = 0x240  → 0x00000A50 (positive)
//   ROR    0x0000A500  8       (8<<7)|(3<<5)|0 = 0x460  → 0x000000A5
//
//   0x00  MOV R0, #0xA5        E3A000A5   R0 ← 165 = 0xA5
//   0x04  MOV R0, R0, LSL #8   E1A00400   R0 ← R0 << 8  = 0x0000A500
//   0x08  MOV R1, R0, LSR #4   E1A01220   R1 ← R0 >> 4  = 0x00000A50
//   0x0C  MOV R2, R0, ASR #4   E1A02240   R2 ← R0 asr 4 = 0x00000A50
//   0x10  MOV R3, R0, ROR #8   E1A03460   R3 ← R0 ror 8 = 0x000000A5
//   0x14  SWI #0x123456        EF123456   halt

export const BARREL_SHIFTER_DEMO: Program = {
  name: "Barrel Shifter",
  description: "Step through LSL, LSR, ASR, and ROR — each as a free Operand2 shift. Watch the Barrel Shifter tab.",
  expectedResult: "R0=0xA500 (42240), R1=0x0A50 (2640), R2=0x0A50 (2640), R3=0xA5 (165)",
  listing: [
    "0x00  MOV R0, #0xA5         ; R0 = 0xA5 = 1010_0101 b",
    "0x04  MOV R0, R0, LSL #8    ; R0 <<= 8  → 0x0000A500",
    "0x08  MOV R1, R0, LSR #4    ; R1 = R0 >> 4  → 0x00000A50",
    "0x0C  MOV R2, R0, ASR #4    ; R2 = R0 asr 4 → 0x00000A50 (positive=same)",
    "0x10  MOV R3, R0, ROR #8    ; R3 = R0 ror 8 → 0x000000A5 (bits wrap back!)",
    "0x14  SWI #0x123456         ; halt",
  ],
  code: instructions(
    0xE3A000A5,  // MOV R0, #0xA5                 imm8=0xA5, rotate=0
    0xE1A00400,  // MOV R0, R0 LSL #8   operand2=(8<<7)|(0<<5)|0 = 0x400
    0xE1A01220,  // MOV R1, R0 LSR #4   operand2=(4<<7)|(1<<5)|0 = 0x220
    0xE1A02240,  // MOV R2, R0 ASR #4   operand2=(4<<7)|(2<<5)|0 = 0x240
    0xE1A03460,  // MOV R3, R0 ROR #8   operand2=(8<<7)|(3<<5)|0 = 0x460
    0xEF123456,  // SWI #0x123456
  ),
};

export const PROGRAMS: Program[] = [
  FIBONACCI,
  SUM_1_TO_10,
  ARRAY_MAX,
  BARREL_SHIFTER_DEMO,
];
