/**
 * FPRegisterFile -- floating-point register storage for GPU cores.
 *
 * === What is a Register File? ===
 *
 * A register file is the fastest storage in a processor -- faster than cache,
 * faster than RAM. It's where the processor keeps the values it's currently
 * working with. Think of it like the handful of numbers you can keep in your
 * head while doing mental math.
 *
 *     Register file (in your head):
 *         "first number"  = 3.14
 *         "second number" = 2.71
 *         "result"        = ???
 *
 *     Register file (in a GPU core):
 *         R0  = 3.14  (FloatBits: sign=0, exp=[...], mantissa=[...])
 *         R1  = 2.71  (FloatBits: sign=0, exp=[...], mantissa=[...])
 *         R2  = 0.00  (will hold the result)
 *
 * === GPU vs CPU Register Files ===
 *
 * CPU registers hold integers (32 or 64 bits of binary). GPU registers hold
 * floating-point numbers (IEEE 754 FloatBits). This reflects their different
 * purposes:
 *
 *     CPU: general-purpose computation (loops, pointers, addresses -> integers)
 *     GPU: parallel numeric computation (vertices, pixels, gradients -> floats)
 *
 * === Why Configurable? ===
 *
 * Different GPU vendors use different register counts:
 *
 *     NVIDIA CUDA Core:    up to 255 registers per thread
 *     AMD Stream Processor: 256 VGPRs (Vector General Purpose Registers)
 *     Intel Vector Engine:  128 GRF entries (General Register File)
 *     ARM Mali:            64 registers per thread
 *
 * By making the register count a constructor parameter, the same GPUCore
 * class can simulate any vendor's register architecture.
 *
 * === Register File Diagram ===
 *
 *     +------------------------------------------+
 *     |           FP Register File               |
 *     |         (32 registers x FP32)            |
 *     +------------------------------------------+
 *     |  R0:  [0][01111111][00000000000...0]     |  = +1.0
 *     |  R1:  [0][10000000][00000000000...0]     |  = +2.0
 *     |  R2:  [0][00000000][00000000000...0]     |  = +0.0
 *     |  ...                                     |
 *     |  R31: [0][00000000][00000000000...0]     |  = +0.0
 *     +------------------------------------------+
 *
 *     Each register stores a FloatBits value:
 *         sign (1 bit) + exponent (8 bits for FP32) + mantissa (23 bits for FP32)
 */

import {
  type FloatBits,
  type FloatFormat,
  FP32,
  floatToBits,
  bitsToFloat,
} from "@coding-adventures/fp-arithmetic";

export class FPRegisterFile {
  /**
   * A configurable floating-point register file.
   *
   * Stores FloatBits values (from the fp-arithmetic package) in a fixed
   * number of registers. Provides both raw FloatBits and convenience float
   * interfaces for reading and writing.
   */

  /** How many registers this file contains. */
  readonly numRegisters: number;

  /** The floating-point format for all registers (FP32, FP16, BF16). */
  readonly fmt: FloatFormat;

  /** Internal storage: an array of FloatBits values. */
  private _values: FloatBits[];

  /** The zero value in this format, cached for efficiency. */
  private readonly _zero: FloatBits;

  constructor(numRegisters: number = 32, fmt: FloatFormat = FP32) {
    if (numRegisters < 1 || numRegisters > 256) {
      throw new RangeError(
        `num_registers must be 1-256, got ${numRegisters}`,
      );
    }
    this.numRegisters = numRegisters;
    this.fmt = fmt;
    // Initialize all registers to +0.0 in the specified format.
    this._zero = floatToBits(0.0, fmt);
    this._values = Array.from({ length: numRegisters }, () => this._zero);
  }

  /**
   * Validate a register index, throwing RangeError if out of bounds.
   *
   * This guard is called before every read and write to catch programming
   * errors early with a clear message.
   */
  private _checkIndex(index: number): void {
    if (index < 0 || index >= this.numRegisters) {
      throw new RangeError(
        `Register index ${index} out of range [0, ${this.numRegisters - 1}]`,
      );
    }
  }

  /**
   * Read a register as a FloatBits value.
   *
   * This returns the raw bit-level representation, preserving all the
   * sign/exponent/mantissa detail. Use readFloat() if you just want a
   * JavaScript number.
   */
  read(index: number): FloatBits {
    this._checkIndex(index);
    return this._values[index];
  }

  /**
   * Write a FloatBits value to a register.
   *
   * This stores the exact bit pattern. Use writeFloat() to convert from
   * a JavaScript number automatically.
   */
  write(index: number, value: FloatBits): void {
    this._checkIndex(index);
    this._values[index] = value;
  }

  /**
   * Convenience: read a register as a JavaScript number.
   *
   * This decodes the FloatBits back to a number, which is useful for
   * inspection and testing but loses the bit-level detail.
   */
  readFloat(index: number): number {
    return bitsToFloat(this.read(index));
  }

  /**
   * Convenience: write a JavaScript number to a register.
   *
   * This encodes the number as FloatBits in the register file's format,
   * then stores it. Useful for setting up test inputs.
   */
  writeFloat(index: number, value: number): void {
    this.write(index, floatToBits(value, this.fmt));
  }

  /**
   * Return all register values as a record of "R{n}" -> float.
   *
   * Useful for debugging and test assertions. Only includes non-zero
   * registers to reduce noise.
   */
  dump(): Record<string, number> {
    const result: Record<string, number> = {};
    for (let i = 0; i < this.numRegisters; i++) {
      const val = bitsToFloat(this._values[i]);
      if (val !== 0.0) {
        result[`R${i}`] = val;
      }
    }
    return result;
  }

  /**
   * Return ALL register values as a record of "R{n}" -> float.
   *
   * Unlike dump(), this includes zero-valued registers.
   */
  dumpAll(): Record<string, number> {
    const result: Record<string, number> = {};
    for (let i = 0; i < this.numRegisters; i++) {
      result[`R${i}`] = bitsToFloat(this._values[i]);
    }
    return result;
  }

  /**
   * String representation for debugging.
   *
   * Shows non-zero registers for quick inspection.
   */
  toString(): string {
    const nonZero = this.dump();
    if (Object.keys(nonZero).length === 0) {
      return `FPRegisterFile(${this.numRegisters} regs, all zero)`;
    }
    const entries = Object.entries(nonZero)
      .map(([k, v]) => `${k}=${v}`)
      .join(", ");
    return `FPRegisterFile(${entries})`;
  }
}
