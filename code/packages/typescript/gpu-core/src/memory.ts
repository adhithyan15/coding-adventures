/**
 * LocalMemory -- byte-addressable scratchpad with floating-point load/store.
 *
 * === What is Local Memory? ===
 *
 * Every GPU thread has a small, private memory area called "local memory" or
 * "scratchpad." It's used for temporary storage that doesn't fit in registers:
 * spilled variables, array elements, intermediate results.
 *
 *     +----------------------------------------------+
 *     |              Local Memory (4 KB)              |
 *     +----------------------------------------------+
 *     |  0x000: [42] [00] [48] [42]  <- 3.14 as FP32 |
 *     |  0x004: [EC] [51] [2D] [40]  <- 2.71 as FP32 |
 *     |  0x008: [00] [00] [00] [00]  <- 0.0           |
 *     |  ...                                          |
 *     |  0xFFC: [00] [00] [00] [00]                    |
 *     +----------------------------------------------+
 *
 * === How Floats Live in Memory ===
 *
 * A FloatBits value (sign + exponent + mantissa) must be converted to raw bytes
 * before it can be stored in memory. This is the same process that happens in
 * real hardware when a GPU core executes a STORE instruction:
 *
 *     1. Take the FloatBits fields: sign=0, exponent=[01111111], mantissa=[10010...]
 *     2. Concatenate into a bit string: 0_01111111_10010001000011111101101
 *     3. Group into bytes: [3F] [C9] [0F] [DB]  (that's 3.14159 in FP32)
 *     4. Write bytes to memory in little-endian order: [DB] [0F] [C9] [3F]
 *
 * Loading reverses this: read bytes, reassemble bits, create FloatBits.
 *
 * === Memory Sizes Across Vendors ===
 *
 *     NVIDIA: 512 KB local memory per thread (rarely used, slow)
 *     AMD:    Scratch memory, up to 4 MB per wavefront
 *     ARM:    Stack memory region per thread
 *     TPU:    No per-PE memory (data flows through systolic array)
 *
 * Our default of 4 KB is small but sufficient for educational programs.
 */

import {
  type FloatBits,
  type FloatFormat,
  FP32,
  floatToBits,
  bitsToFloat,
} from "@coding-adventures/fp-arithmetic";

export class LocalMemory {
  /**
   * Byte-addressable local scratchpad memory with FP-aware load/store.
   *
   * Provides both raw byte access and convenient floating-point operations
   * that handle the conversion between FloatBits and byte sequences.
   */

  /** Memory size in bytes. */
  readonly size: number;

  /** Internal storage: a Uint8Array of raw bytes. */
  private _data: Uint8Array;

  constructor(size: number = 4096) {
    if (size < 1) {
      throw new RangeError(`Memory size must be positive, got ${size}`);
    }
    this.size = size;
    this._data = new Uint8Array(size);
  }

  /**
   * Validate that an access is within bounds.
   *
   * Every memory operation calls this first. Out-of-bounds access in real
   * hardware causes a segfault; we throw a RangeError with a clear message.
   */
  private _checkBounds(address: number, numBytes: number): void {
    if (address < 0 || address + numBytes > this.size) {
      throw new RangeError(
        `Memory access at ${address}:${address + numBytes} ` +
          `out of bounds [0, ${this.size})`,
      );
    }
  }

  // --- Raw byte access ---

  /** Read a single byte from memory. */
  readByte(address: number): number {
    this._checkBounds(address, 1);
    return this._data[address];
  }

  /** Write a single byte to memory. */
  writeByte(address: number, value: number): void {
    this._checkBounds(address, 1);
    this._data[address] = value & 0xff;
  }

  /** Read multiple bytes from memory. */
  readBytes(address: number, count: number): Uint8Array {
    this._checkBounds(address, count);
    return this._data.slice(address, address + count);
  }

  /** Write multiple bytes to memory. */
  writeBytes(address: number, data: Uint8Array): void {
    this._checkBounds(address, data.length);
    this._data.set(data, address);
  }

  // --- Floating-point access ---

  /**
   * How many bytes a float format uses: FP32=4, FP16/BF16=2.
   *
   * This determines how many bytes we read/write when loading/storing
   * a floating-point value.
   */
  private _floatByteWidth(fmt: FloatFormat): number {
    return fmt.totalBits / 8;
  }

  /**
   * Convert a FloatBits to raw bytes (little-endian).
   *
   * The process:
   * 1. Concatenate sign + exponent + mantissa into one integer
   * 2. Pack that integer into bytes using DataView
   *
   * Example for FP32 value 1.0:
   *     sign=0, exponent=[0,1,1,1,1,1,1,1], mantissa=[0]*23
   *     -> bit string: 0_01111111_00000000000000000000000
   *     -> integer: 0x3F800000
   *     -> bytes (little-endian): [00, 00, 80, 3F]
   */
  private _floatbitsToBytes(value: FloatBits): Uint8Array {
    // Reassemble the bit pattern from FloatBits fields
    let bits = value.sign;
    for (const b of value.exponent) {
      bits = (bits << 1) | b;
    }
    for (const b of value.mantissa) {
      bits = (bits << 1) | b;
    }

    // Pack as bytes using DataView (little-endian)
    const byteWidth = this._floatByteWidth(value.fmt);
    if (byteWidth === 4) {
      // Use DataView for correct little-endian encoding.
      // The `>>> 0` converts to unsigned 32-bit integer, avoiding the
      // JavaScript signed-int gotcha where 0xFFFFFFFF becomes -1.
      const buf = new ArrayBuffer(4);
      new DataView(buf).setUint32(0, bits >>> 0, true); // true = little-endian
      return new Uint8Array(buf);
    }
    if (byteWidth === 2) {
      const buf = new ArrayBuffer(2);
      new DataView(buf).setUint16(0, bits & 0xffff, true);
      return new Uint8Array(buf);
    }
    throw new Error(`Unsupported float width: ${byteWidth} bytes`);
  }

  /**
   * Convert raw bytes (little-endian) back to a FloatBits.
   *
   * Reverses _floatbitsToBytes: unpack integer, split into fields.
   */
  private _bytesToFloatbits(data: Uint8Array, fmt: FloatFormat): FloatBits {
    const byteWidth = this._floatByteWidth(fmt);
    let bits: number;
    if (byteWidth === 4) {
      bits = new DataView(data.buffer, data.byteOffset, data.byteLength).getUint32(0, true);
    } else if (byteWidth === 2) {
      bits = new DataView(data.buffer, data.byteOffset, data.byteLength).getUint16(0, true);
    } else {
      throw new Error(`Unsupported float width: ${byteWidth} bytes`);
    }

    const totalBits = fmt.totalBits;
    const mantissaBits = fmt.mantissaBits;
    const exponentBits = fmt.exponentBits;

    // Mantissa is the lowest mantissaBits bits
    const mantissaMask = (1 << mantissaBits) - 1;
    const mantissaInt = bits & mantissaMask;
    const mantissa: number[] = [];
    for (let i = 0; i < mantissaBits; i++) {
      mantissa.push((mantissaInt >> (mantissaBits - 1 - i)) & 1);
    }

    // Exponent is the next exponentBits bits
    const exponentMask = (1 << exponentBits) - 1;
    const exponentInt = (bits >> mantissaBits) & exponentMask;
    const exponent: number[] = [];
    for (let i = 0; i < exponentBits; i++) {
      exponent.push((exponentInt >> (exponentBits - 1 - i)) & 1);
    }

    // Sign is the highest bit
    const sign = (bits >> (totalBits - 1)) & 1;

    return { sign, exponent, mantissa, fmt };
  }

  /**
   * Load a floating-point value from memory.
   *
   * Reads the appropriate number of bytes (4 for FP32, 2 for FP16/BF16)
   * starting at the given address, and converts them to a FloatBits.
   */
  loadFloat(address: number, fmt: FloatFormat = FP32): FloatBits {
    const byteWidth = this._floatByteWidth(fmt);
    const data = this.readBytes(address, byteWidth);
    return this._bytesToFloatbits(data, fmt);
  }

  /**
   * Store a floating-point value to memory.
   *
   * Converts the FloatBits to bytes and writes them starting at the
   * given address.
   */
  storeFloat(address: number, value: FloatBits): void {
    const data = this._floatbitsToBytes(value);
    this.writeBytes(address, data);
  }

  /** Convenience: load a float and convert to JavaScript number. */
  loadFloatAsPython(address: number, fmt: FloatFormat = FP32): number {
    return bitsToFloat(this.loadFloat(address, fmt));
  }

  /** Convenience: store a JavaScript number to memory. */
  storePythonFloat(
    address: number,
    value: number,
    fmt: FloatFormat = FP32,
  ): void {
    this.storeFloat(address, floatToBits(value, fmt));
  }

  /**
   * Return a slice of memory as an array of byte values.
   *
   * Useful for debugging. Default shows the first 64 bytes.
   */
  dump(start: number = 0, length: number = 64): number[] {
    const end = Math.min(start + length, this.size);
    return Array.from(this._data.slice(start, end));
  }

  /** String representation for debugging. */
  toString(): string {
    let used = 0;
    for (let i = 0; i < this.size; i++) {
      if (this._data[i] !== 0) used++;
    }
    return `LocalMemory(${this.size} bytes, ${used} non-zero)`;
  }
}
