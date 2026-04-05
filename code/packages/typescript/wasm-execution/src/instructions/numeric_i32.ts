/**
 * numeric_i32.ts --- 32-bit integer instruction handlers for WASM.
 *
 * ===========================================================================
 * OVERVIEW: i32 ARITHMETIC IN WEBASSEMBLY
 * ===========================================================================
 *
 * WebAssembly's i32 type represents 32-bit integers. Unlike most high-level
 * languages, WASM makes *no* distinction between signed and unsigned integers
 * at the storage level --- both are just 32 bits of data. The *interpretation*
 * (signed vs unsigned) is determined by which instruction you use:
 *
 *   - ``i32.lt_s`` treats operands as signed (two's complement).
 *   - ``i32.lt_u`` treats operands as unsigned.
 *   - ``i32.add`` doesn't care --- addition is the same for both!
 *
 * This is exactly how x86 and ARM CPUs work: there's no "signed register"
 * or "unsigned register". The ALU just adds/subtracts/shifts bits, and
 * different comparison instructions interpret the flags differently.
 *
 * ===========================================================================
 * JAVASCRIPT NUMERIC QUIRKS
 * ===========================================================================
 *
 * JavaScript ``number`` is a 64-bit IEEE 754 double. It can represent all
 * 32-bit integers exactly (both signed and unsigned), but arithmetic results
 * may produce values outside the 32-bit range. We use bit tricks to keep
 * values in the i32 range:
 *
 *   - ``(result) | 0``   --- Truncates to signed 32-bit integer.
 *   - ``(value) >>> 0``  --- Converts to unsigned 32-bit integer (for comparisons).
 *   - ``Math.imul(a, b)`` --- Full 32-bit multiplication without double-precision loss.
 *   - ``Math.clz32(a)``  --- Count leading zeros (built into V8).
 *
 * The ``| 0`` trick works because JavaScript's bitwise operators convert
 * their operands to 32-bit signed integers before operating. So:
 *
 *   (2147483647 + 1) | 0  ===  -2147483648  (wraps around, just like C!)
 *
 * ===========================================================================
 * INSTRUCTION MAP
 * ===========================================================================
 *
 *   Opcode   Mnemonic        Category      Stack Effect
 *   ------   --------        --------      ------------
 *   0x41     i32.const       constant      [] -> [i32]
 *   0x45     i32.eqz         comparison    [i32] -> [i32]
 *   0x46     i32.eq          comparison    [i32 i32] -> [i32]
 *   0x47     i32.ne          comparison    [i32 i32] -> [i32]
 *   0x48     i32.lt_s        comparison    [i32 i32] -> [i32]
 *   0x49     i32.lt_u        comparison    [i32 i32] -> [i32]
 *   0x4A     i32.gt_s        comparison    [i32 i32] -> [i32]
 *   0x4B     i32.gt_u        comparison    [i32 i32] -> [i32]
 *   0x4C     i32.le_s        comparison    [i32 i32] -> [i32]
 *   0x4D     i32.le_u        comparison    [i32 i32] -> [i32]
 *   0x4E     i32.ge_s        comparison    [i32 i32] -> [i32]
 *   0x4F     i32.ge_u        comparison    [i32 i32] -> [i32]
 *   0x67     i32.clz         unary         [i32] -> [i32]
 *   0x68     i32.ctz         unary         [i32] -> [i32]
 *   0x69     i32.popcnt      unary         [i32] -> [i32]
 *   0x6A     i32.add         arithmetic    [i32 i32] -> [i32]
 *   0x6B     i32.sub         arithmetic    [i32 i32] -> [i32]
 *   0x6C     i32.mul         arithmetic    [i32 i32] -> [i32]
 *   0x6D     i32.div_s       arithmetic    [i32 i32] -> [i32]
 *   0x6E     i32.div_u       arithmetic    [i32 i32] -> [i32]
 *   0x6F     i32.rem_s       arithmetic    [i32 i32] -> [i32]
 *   0x70     i32.rem_u       arithmetic    [i32 i32] -> [i32]
 *   0x71     i32.and         bitwise       [i32 i32] -> [i32]
 *   0x72     i32.or          bitwise       [i32 i32] -> [i32]
 *   0x73     i32.xor         bitwise       [i32 i32] -> [i32]
 *   0x74     i32.shl         bitwise       [i32 i32] -> [i32]
 *   0x75     i32.shr_s       bitwise       [i32 i32] -> [i32]
 *   0x76     i32.shr_u       bitwise       [i32 i32] -> [i32]
 *   0x77     i32.rotl        bitwise       [i32 i32] -> [i32]
 *   0x78     i32.rotr        bitwise       [i32 i32] -> [i32]
 *
 * ===========================================================================
 * POP ORDER: b FIRST, THEN a
 * ===========================================================================
 *
 * The WASM stack is last-in, first-out. For a binary operation like ``add``:
 *
 *   1. The program pushes ``a`` (the left operand).
 *   2. The program pushes ``b`` (the right operand) --- this is now on top.
 *   3. The ``add`` instruction executes:
 *      - Pop ``b`` (it's on top).
 *      - Pop ``a`` (it was underneath).
 *      - Push ``a + b``.
 *
 * Getting the pop order wrong is a common source of bugs, especially for
 * non-commutative operations like subtraction and division!
 *
 * @module
 */

import type { GenericVM } from "@coding-adventures/virtual-machine";
import { TrapError } from "../host_interface.js";
import { i32, asI32 } from "../values.js";
import type { WasmExecutionContext } from "../types.js";

// ===========================================================================
// Constants
// ===========================================================================

/** The minimum signed 32-bit integer: -2,147,483,648 (0x80000000). */
const INT32_MIN = -2147483648;

// ===========================================================================
// Helper: Count trailing zeros
// ===========================================================================

/**
 * Count the number of trailing zero bits in a 32-bit integer.
 *
 * There's no built-in ``Math.ctz32`` in JavaScript (unlike ``Math.clz32``
 * for leading zeros), so we implement it manually. The algorithm checks
 * each bit from the LSB upward:
 *
 *   Example: ctz32(0b00101000) = 3  (three zeros before the first 1)
 *   Example: ctz32(0) = 32          (all 32 bits are zero)
 *
 * A faster approach uses de Bruijn sequences, but the loop is clearer.
 */
function ctz32(value: number): number {
  if (value === 0) return 32;
  let count = 0;
  /* Check each bit starting from the least significant. When we find a
     1-bit, stop counting. The ``>>> 0`` ensures unsigned interpretation. */
  let v = value >>> 0;
  while ((v & 1) === 0) {
    count++;
    v >>>= 1;
  }
  return count;
}

// ===========================================================================
// Helper: Population count (count set bits)
// ===========================================================================

/**
 * Count the number of 1-bits in a 32-bit integer.
 *
 * Also known as "population count" or "Hamming weight". This is useful in
 * bit manipulation, error correction codes, and combinatorics.
 *
 *   Example: popcnt32(0b10110011) = 5  (five 1-bits)
 *   Example: popcnt32(0) = 0
 *   Example: popcnt32(-1) = 32         (all bits set)
 *
 * We use the classic "sideways addition" algorithm, which processes all
 * 32 bits in O(log 32) = 5 steps instead of looping through each bit.
 */
function popcnt32(value: number): number {
  let v = value >>> 0;
  /* Step 1: Count pairs of bits. Each 2-bit field holds the count of
     1-bits in the original 2-bit field. */
  v = v - ((v >>> 1) & 0x55555555);
  /* Step 2: Sum pairs of 2-bit counts into 4-bit counts. */
  v = (v & 0x33333333) + ((v >>> 2) & 0x33333333);
  /* Step 3: Sum pairs of 4-bit counts into 8-bit counts. */
  v = (v + (v >>> 4)) & 0x0f0f0f0f;
  /* Step 4: Sum all 8-bit counts into a single result. The multiply
     trick sums all four bytes in one operation. */
  return (Math.imul(v, 0x01010101) >>> 24);
}

// ===========================================================================
// Registration Function
// ===========================================================================

/**
 * Register all 33 i32 numeric instruction handlers on the given GenericVM.
 *
 * Each handler:
 *   1. Pops operands from the typed stack (using ``vm.popTyped()``).
 *   2. Performs the operation.
 *   3. Pushes the result (using ``vm.pushTyped()``).
 *   4. Advances the program counter (``vm.advancePc()``).
 *
 * @param vm - The GenericVM to register handlers on.
 */
export function registerNumericI32(vm: GenericVM): void {

  // =========================================================================
  // 0x41: i32.const --- Push an i32 constant
  // =========================================================================
  //
  // The simplest instruction: read the immediate operand from the
  // instruction and push it onto the stack. The operand was decoded
  // by the pre-instruction hook (from LEB128 in the raw bytecodes).
  //
  //   Stack: [] -> [operand]
  //
  vm.registerContextOpcode<WasmExecutionContext>(0x41, (vm, instr, _code, _ctx) => {
    vm.pushTyped(i32(instr.operand as number));
    vm.advancePc();
    return "i32.const";
  });

  // =========================================================================
  // 0x45: i32.eqz --- Test if zero
  // =========================================================================
  //
  // A UNARY operation: pops ONE value (not two!) and tests if it's zero.
  // Returns 1 (true) if the value is 0, otherwise 0 (false).
  //
  // This is WASM's equivalent of C's ``!value`` or ``value == 0``.
  //
  //   Stack: [a] -> [a === 0 ? 1 : 0]
  //
  vm.registerContextOpcode<WasmExecutionContext>(0x45, (vm, _instr, _code, _ctx) => {
    const a = asI32(vm.popTyped());
    vm.pushTyped(i32(a === 0 ? 1 : 0));
    vm.advancePc();
    return "i32.eqz";
  });

  // =========================================================================
  // 0x46 - 0x4F: Comparison Operations
  // =========================================================================
  //
  // All comparisons pop TWO values, compare them, and push 1 (true) or
  // 0 (false). The result is always an i32, even though it represents
  // a boolean.
  //
  // IMPORTANT: Pop order is b first (top of stack), then a. The comparison
  // is ``a <op> b``.
  //
  // For signed comparisons, ``| 0`` ensures two's complement interpretation.
  // For unsigned comparisons, ``>>> 0`` converts to unsigned.
  //

  // 0x46: i32.eq --- Equal
  vm.registerContextOpcode<WasmExecutionContext>(0x46, (vm, _instr, _code, _ctx) => {
    const b = asI32(vm.popTyped());
    const a = asI32(vm.popTyped());
    vm.pushTyped(i32(a === b ? 1 : 0));
    vm.advancePc();
    return "i32.eq";
  });

  // 0x47: i32.ne --- Not equal
  vm.registerContextOpcode<WasmExecutionContext>(0x47, (vm, _instr, _code, _ctx) => {
    const b = asI32(vm.popTyped());
    const a = asI32(vm.popTyped());
    vm.pushTyped(i32(a !== b ? 1 : 0));
    vm.advancePc();
    return "i32.ne";
  });

  // 0x48: i32.lt_s --- Less than (signed)
  //
  // Two's complement signed comparison. The ``| 0`` ensures that the JS
  // number is treated as a signed 32-bit integer. For example:
  //   (0xFFFFFFFF | 0) === -1   (signed interpretation of all-ones)
  //
  vm.registerContextOpcode<WasmExecutionContext>(0x48, (vm, _instr, _code, _ctx) => {
    const b = asI32(vm.popTyped());
    const a = asI32(vm.popTyped());
    vm.pushTyped(i32((a | 0) < (b | 0) ? 1 : 0));
    vm.advancePc();
    return "i32.lt_s";
  });

  // 0x49: i32.lt_u --- Less than (unsigned)
  //
  // ``>>> 0`` converts to an unsigned 32-bit value. For example:
  //   (-1 >>> 0) === 4294967295   (unsigned interpretation of -1)
  //
  vm.registerContextOpcode<WasmExecutionContext>(0x49, (vm, _instr, _code, _ctx) => {
    const b = asI32(vm.popTyped());
    const a = asI32(vm.popTyped());
    vm.pushTyped(i32((a >>> 0) < (b >>> 0) ? 1 : 0));
    vm.advancePc();
    return "i32.lt_u";
  });

  // 0x4A: i32.gt_s --- Greater than (signed)
  vm.registerContextOpcode<WasmExecutionContext>(0x4a, (vm, _instr, _code, _ctx) => {
    const b = asI32(vm.popTyped());
    const a = asI32(vm.popTyped());
    vm.pushTyped(i32((a | 0) > (b | 0) ? 1 : 0));
    vm.advancePc();
    return "i32.gt_s";
  });

  // 0x4B: i32.gt_u --- Greater than (unsigned)
  vm.registerContextOpcode<WasmExecutionContext>(0x4b, (vm, _instr, _code, _ctx) => {
    const b = asI32(vm.popTyped());
    const a = asI32(vm.popTyped());
    vm.pushTyped(i32((a >>> 0) > (b >>> 0) ? 1 : 0));
    vm.advancePc();
    return "i32.gt_u";
  });

  // 0x4C: i32.le_s --- Less than or equal (signed)
  vm.registerContextOpcode<WasmExecutionContext>(0x4c, (vm, _instr, _code, _ctx) => {
    const b = asI32(vm.popTyped());
    const a = asI32(vm.popTyped());
    vm.pushTyped(i32((a | 0) <= (b | 0) ? 1 : 0));
    vm.advancePc();
    return "i32.le_s";
  });

  // 0x4D: i32.le_u --- Less than or equal (unsigned)
  vm.registerContextOpcode<WasmExecutionContext>(0x4d, (vm, _instr, _code, _ctx) => {
    const b = asI32(vm.popTyped());
    const a = asI32(vm.popTyped());
    vm.pushTyped(i32((a >>> 0) <= (b >>> 0) ? 1 : 0));
    vm.advancePc();
    return "i32.le_u";
  });

  // 0x4E: i32.ge_s --- Greater than or equal (signed)
  vm.registerContextOpcode<WasmExecutionContext>(0x4e, (vm, _instr, _code, _ctx) => {
    const b = asI32(vm.popTyped());
    const a = asI32(vm.popTyped());
    vm.pushTyped(i32((a | 0) >= (b | 0) ? 1 : 0));
    vm.advancePc();
    return "i32.ge_s";
  });

  // 0x4F: i32.ge_u --- Greater than or equal (unsigned)
  vm.registerContextOpcode<WasmExecutionContext>(0x4f, (vm, _instr, _code, _ctx) => {
    const b = asI32(vm.popTyped());
    const a = asI32(vm.popTyped());
    vm.pushTyped(i32((a >>> 0) >= (b >>> 0) ? 1 : 0));
    vm.advancePc();
    return "i32.ge_u";
  });

  // =========================================================================
  // 0x67 - 0x69: Unary Bit Operations
  // =========================================================================

  // 0x67: i32.clz --- Count leading zeros
  //
  // Returns the number of leading zero bits in the 32-bit representation.
  // JavaScript provides Math.clz32() which does exactly this.
  //
  //   clz32(0x00000001) = 31  (31 leading zeros before the LSB)
  //   clz32(0x80000000) = 0   (MSB is set, no leading zeros)
  //   clz32(0x00000000) = 32  (all zeros)
  //
  vm.registerContextOpcode<WasmExecutionContext>(0x67, (vm, _instr, _code, _ctx) => {
    const a = asI32(vm.popTyped());
    vm.pushTyped(i32(Math.clz32(a)));
    vm.advancePc();
    return "i32.clz";
  });

  // 0x68: i32.ctz --- Count trailing zeros
  //
  // Returns the number of trailing zero bits. No built-in JS function for this,
  // so we use our own implementation above.
  //
  //   ctz32(0x80000000) = 31  (31 trailing zeros)
  //   ctz32(0x00000001) = 0   (LSB is set)
  //   ctz32(0x00000000) = 32  (all zeros)
  //
  vm.registerContextOpcode<WasmExecutionContext>(0x68, (vm, _instr, _code, _ctx) => {
    const a = asI32(vm.popTyped());
    vm.pushTyped(i32(ctz32(a)));
    vm.advancePc();
    return "i32.ctz";
  });

  // 0x69: i32.popcnt --- Population count (count 1-bits)
  //
  //   popcnt32(0b11001010) = 4  (four 1-bits)
  //
  vm.registerContextOpcode<WasmExecutionContext>(0x69, (vm, _instr, _code, _ctx) => {
    const a = asI32(vm.popTyped());
    vm.pushTyped(i32(popcnt32(a)));
    vm.advancePc();
    return "i32.popcnt";
  });

  // =========================================================================
  // 0x6A - 0x70: Arithmetic Operations
  // =========================================================================

  // 0x6A: i32.add --- Wrapping addition
  //
  // Add two i32 values. The ``| 0`` truncates to 32 bits, giving us
  // proper wrapping behavior (e.g., INT32_MAX + 1 wraps to INT32_MIN).
  //
  vm.registerContextOpcode<WasmExecutionContext>(0x6a, (vm, _instr, _code, _ctx) => {
    const b = asI32(vm.popTyped());
    const a = asI32(vm.popTyped());
    vm.pushTyped(i32((a + b) | 0));
    vm.advancePc();
    return "i32.add";
  });

  // 0x6B: i32.sub --- Wrapping subtraction
  vm.registerContextOpcode<WasmExecutionContext>(0x6b, (vm, _instr, _code, _ctx) => {
    const b = asI32(vm.popTyped());
    const a = asI32(vm.popTyped());
    vm.pushTyped(i32((a - b) | 0));
    vm.advancePc();
    return "i32.sub";
  });

  // 0x6C: i32.mul --- Wrapping multiplication
  //
  // We use Math.imul() instead of ``(a * b) | 0`` because JavaScript's
  // ``*`` operator uses double-precision floats internally. For large
  // values, the intermediate result may lose precision:
  //
  //   0x7FFFFFFF * 0x7FFFFFFF = 4611686014132420609  (needs 63 bits!)
  //
  // A JS double only has 53 bits of mantissa, so ``*`` would give the
  // wrong result. Math.imul() performs true 32-bit multiplication.
  //
  vm.registerContextOpcode<WasmExecutionContext>(0x6c, (vm, _instr, _code, _ctx) => {
    const b = asI32(vm.popTyped());
    const a = asI32(vm.popTyped());
    vm.pushTyped(i32(Math.imul(a, b)));
    vm.advancePc();
    return "i32.mul";
  });

  // 0x6D: i32.div_s --- Signed division (trapping)
  //
  // WASM integer division TRAPS on two conditions:
  //   1. Division by zero (like most CPUs).
  //   2. INT32_MIN / -1 (result would be INT32_MAX + 1, which overflows).
  //
  // The second case is a hardware quirk: on x86, ``IDIV`` with these
  // operands causes a #DE (divide error) exception. WASM follows suit.
  //
  vm.registerContextOpcode<WasmExecutionContext>(0x6d, (vm, _instr, _code, _ctx) => {
    const b = asI32(vm.popTyped());
    const a = asI32(vm.popTyped());
    if (b === 0) {
      throw new TrapError("integer divide by zero");
    }
    if ((a | 0) === INT32_MIN && (b | 0) === -1) {
      throw new TrapError("integer overflow");
    }
    vm.pushTyped(i32(((a | 0) / (b | 0)) | 0));
    vm.advancePc();
    return "i32.div_s";
  });

  // 0x6E: i32.div_u --- Unsigned division (trapping)
  //
  // Same as div_s but interprets operands as unsigned. No overflow trap
  // because unsigned division of any two values fits in 32 bits.
  //
  vm.registerContextOpcode<WasmExecutionContext>(0x6e, (vm, _instr, _code, _ctx) => {
    const b = asI32(vm.popTyped());
    const a = asI32(vm.popTyped());
    if ((b >>> 0) === 0) {
      throw new TrapError("integer divide by zero");
    }
    vm.pushTyped(i32(((a >>> 0) / (b >>> 0)) | 0));
    vm.advancePc();
    return "i32.div_u";
  });

  // 0x6F: i32.rem_s --- Signed remainder (trapping on zero divisor)
  //
  // Note: The WASM spec says that if a is INT32_MIN and b is -1, the
  // result is 0 (not a trap). This differs from div_s which traps.
  // The reason: the remainder of any integer divided by -1 is always 0.
  //
  vm.registerContextOpcode<WasmExecutionContext>(0x6f, (vm, _instr, _code, _ctx) => {
    const b = asI32(vm.popTyped());
    const a = asI32(vm.popTyped());
    if (b === 0) {
      throw new TrapError("integer divide by zero");
    }
    /* Special case: INT32_MIN % -1. JavaScript's ``%`` operator would
       return 0 on most engines, but the spec mandates it explicitly. */
    if ((a | 0) === INT32_MIN && (b | 0) === -1) {
      vm.pushTyped(i32(0));
    } else {
      vm.pushTyped(i32((a % b) | 0));
    }
    vm.advancePc();
    return "i32.rem_s";
  });

  // 0x70: i32.rem_u --- Unsigned remainder (trapping on zero divisor)
  vm.registerContextOpcode<WasmExecutionContext>(0x70, (vm, _instr, _code, _ctx) => {
    const b = asI32(vm.popTyped());
    const a = asI32(vm.popTyped());
    if ((b >>> 0) === 0) {
      throw new TrapError("integer divide by zero");
    }
    vm.pushTyped(i32(((a >>> 0) % (b >>> 0)) | 0));
    vm.advancePc();
    return "i32.rem_u";
  });

  // =========================================================================
  // 0x71 - 0x73: Bitwise Logic
  // =========================================================================
  //
  // These are straightforward bitwise operations. JavaScript's bitwise
  // operators already work on 32-bit integers, so no special handling
  // is needed.
  //

  // 0x71: i32.and --- Bitwise AND
  vm.registerContextOpcode<WasmExecutionContext>(0x71, (vm, _instr, _code, _ctx) => {
    const b = asI32(vm.popTyped());
    const a = asI32(vm.popTyped());
    vm.pushTyped(i32((a & b) | 0));
    vm.advancePc();
    return "i32.and";
  });

  // 0x72: i32.or --- Bitwise OR
  vm.registerContextOpcode<WasmExecutionContext>(0x72, (vm, _instr, _code, _ctx) => {
    const b = asI32(vm.popTyped());
    const a = asI32(vm.popTyped());
    vm.pushTyped(i32((a | b) | 0));
    vm.advancePc();
    return "i32.or";
  });

  // 0x73: i32.xor --- Bitwise XOR
  vm.registerContextOpcode<WasmExecutionContext>(0x73, (vm, _instr, _code, _ctx) => {
    const b = asI32(vm.popTyped());
    const a = asI32(vm.popTyped());
    vm.pushTyped(i32((a ^ b) | 0));
    vm.advancePc();
    return "i32.xor";
  });

  // =========================================================================
  // 0x74 - 0x78: Shift and Rotate Operations
  // =========================================================================
  //
  // WASM shift/rotate amounts are taken modulo 32 (masked with ``& 31``).
  // This means shifting by 32 is the same as shifting by 0 (a no-op),
  // shifting by 33 is the same as shifting by 1, etc.
  //
  // This matches x86 behavior, where the shift count is masked to 5 bits.
  //

  // 0x74: i32.shl --- Shift left
  //
  //   (a << (b & 31)) | 0
  //
  // Shifts ``a`` left by ``b mod 32`` positions. Vacated bits are filled
  // with zeros.
  //
  vm.registerContextOpcode<WasmExecutionContext>(0x74, (vm, _instr, _code, _ctx) => {
    const b = asI32(vm.popTyped());
    const a = asI32(vm.popTyped());
    vm.pushTyped(i32((a << (b & 31)) | 0));
    vm.advancePc();
    return "i32.shl";
  });

  // 0x75: i32.shr_s --- Arithmetic shift right (sign-extending)
  //
  // Shifts ``a`` right, filling vacated bits with the sign bit.
  // JavaScript's ``>>`` operator already does this.
  //
  //   -8 >> 1 === -4  (sign bit is preserved)
  //
  vm.registerContextOpcode<WasmExecutionContext>(0x75, (vm, _instr, _code, _ctx) => {
    const b = asI32(vm.popTyped());
    const a = asI32(vm.popTyped());
    vm.pushTyped(i32((a >> (b & 31)) | 0));
    vm.advancePc();
    return "i32.shr_s";
  });

  // 0x76: i32.shr_u --- Logical shift right (zero-filling)
  //
  // Shifts ``a`` right, filling vacated bits with zeros.
  // JavaScript's ``>>>`` operator does this.
  //
  //   (-1 >>> 1) === 2147483647  (sign bit becomes 0)
  //
  vm.registerContextOpcode<WasmExecutionContext>(0x76, (vm, _instr, _code, _ctx) => {
    const b = asI32(vm.popTyped());
    const a = asI32(vm.popTyped());
    /* Note: ``>>> 0`` at the end would give unsigned result, but we want
       the value stored as a signed i32, so we use ``| 0`` instead. */
    vm.pushTyped(i32((a >>> (b & 31)) | 0));
    vm.advancePc();
    return "i32.shr_u";
  });

  // 0x77: i32.rotl --- Rotate left
  //
  // A rotation is like a shift, but bits that "fall off" one end wrap
  // around to the other end. No bits are lost.
  //
  //   rotl(0b1010...0011, 2) = 0b10...001110  (top 2 bits wrap to bottom)
  //
  // Implementation: (a << n) | (a >>> (32 - n))
  //   - Shift left by n: moves bits left, zeros fill the right.
  //   - Shift right by (32-n): captures the bits that fell off the left.
  //   - OR them together: combines both halves.
  //
  vm.registerContextOpcode<WasmExecutionContext>(0x77, (vm, _instr, _code, _ctx) => {
    const b = asI32(vm.popTyped());
    const a = asI32(vm.popTyped());
    const n = b & 31;
    vm.pushTyped(i32(((a << n) | (a >>> (32 - n))) | 0));
    vm.advancePc();
    return "i32.rotl";
  });

  // 0x78: i32.rotr --- Rotate right
  //
  // Same idea as rotl, but in the opposite direction.
  //
  //   rotr(0b1010...0011, 2) = 0b111010...00  (bottom 2 bits wrap to top)
  //
  vm.registerContextOpcode<WasmExecutionContext>(0x78, (vm, _instr, _code, _ctx) => {
    const b = asI32(vm.popTyped());
    const a = asI32(vm.popTyped());
    const n = b & 31;
    vm.pushTyped(i32(((a >>> n) | (a << (32 - n))) | 0));
    vm.advancePc();
    return "i32.rotr";
  });
}
