/**
 * linear_memory.ts --- WASM linear memory implementation.
 *
 * ===========================================================================
 * WHAT IS LINEAR MEMORY?
 * ===========================================================================
 *
 * WebAssembly's memory model is a contiguous, byte-addressable array of
 * bytes called "linear memory". Think of it as a flat C-style heap: you
 * can read and write individual bytes, 16-bit words, 32-bit words, or
 * 64-bit words at any byte offset within the allocated region.
 *
 * Linear memory is measured in "pages", where each page is exactly 65,536
 * bytes (64 KiB). A module declares a minimum number of pages, and
 * optionally a maximum. The ``memory.grow`` instruction can add pages at
 * runtime (up to the declared maximum).
 *
 * Memory accesses are bounds-checked: reading or writing past the end of
 * the allocated region causes a trap (a TrapError).
 *
 *   +-------------------------------------------------------------------+
 *   |  Page 0 (0x00000 - 0x0FFFF)  |  Page 1 (0x10000 - 0x1FFFF)  |...|
 *   +-------------------------------------------------------------------+
 *   ^                                                                   ^
 *   byte 0                                                   last allocated byte
 *
 * ===========================================================================
 * LITTLE-ENDIAN BYTE ORDERING
 * ===========================================================================
 *
 * WASM always uses little-endian byte order. The least-significant byte
 * comes first in memory:
 *
 *   Storing i32 value 0x01020304 at offset 0:
 *     Address:  0     1     2     3
 *     Content: [04]  [03]  [02]  [01]
 *
 * ===========================================================================
 * LOAD VARIANTS: SIGN- AND ZERO-EXTENSION
 * ===========================================================================
 *
 * WASM supports loading smaller values and extending them to full width.
 * The naming convention matches the spec:
 *
 *   loadI32_8s  = load 8 bits, sign-extend to i32
 *   loadI32_8u  = load 8 bits, zero-extend to i32
 *   loadI32_16s = load 16 bits, sign-extend to i32
 *   loadI32_16u = load 16 bits, zero-extend to i32
 *
 * Sign-extend copies the sign bit into all higher bits:
 *   byte 0xFF sign-extended to i32 = -1   (0xFFFFFFFF)
 *   byte 0xFF zero-extended to i32 = 255  (0x000000FF)
 *
 * @module
 */

import { TrapError } from "./host_interface.js";

export class LinearMemory {
  /** Bytes per WASM memory page: 64 KiB. */
  static readonly PAGE_SIZE = 65536;

  /** The backing ArrayBuffer. Replaced on grow. */
  private buffer: ArrayBuffer;

  /** A DataView for typed reads/writes with endianness control. */
  private view: DataView;

  /** Current number of allocated pages. */
  private currentPages: number;

  /** Maximum pages (null = no limit other than spec max of 65536). */
  private readonly maxPages: number | null;

  /**
   * Create a new LinearMemory.
   *
   * @param initialPages - Number of 64 KiB pages to allocate initially.
   * @param maxPages     - Optional upper bound on page count.
   */
  constructor(initialPages: number, maxPages?: number) {
    this.currentPages = initialPages;
    this.maxPages = maxPages !== undefined ? maxPages : null;
    this.buffer = new ArrayBuffer(initialPages * LinearMemory.PAGE_SIZE);
    this.view = new DataView(this.buffer);
  }

  // =========================================================================
  // Bounds Checking
  // =========================================================================

  /**
   * Validate that accessing ``width`` bytes at ``offset`` is in bounds.
   * Throws TrapError on out-of-bounds --- the WASM "trap on OOB" behavior.
   */
  private boundsCheck(offset: number, width: number): void {
    if (offset < 0 || offset + width > this.buffer.byteLength) {
      throw new TrapError(
        `Out of bounds memory access: offset=${offset}, size=${width}, ` +
          `memory size=${this.buffer.byteLength}`
      );
    }
  }

  // =========================================================================
  // Full-Width Loads
  // =========================================================================

  /** Load 4 bytes as a signed 32-bit integer (little-endian). */
  loadI32(offset: number): number {
    this.boundsCheck(offset, 4);
    return this.view.getInt32(offset, true);
  }

  /** Load 8 bytes as a signed 64-bit integer (little-endian). */
  loadI64(offset: number): bigint {
    this.boundsCheck(offset, 8);
    return this.view.getBigInt64(offset, true);
  }

  /** Load 4 bytes as a 32-bit float (little-endian). */
  loadF32(offset: number): number {
    this.boundsCheck(offset, 4);
    return this.view.getFloat32(offset, true);
  }

  /** Load 8 bytes as a 64-bit float (little-endian). */
  loadF64(offset: number): number {
    this.boundsCheck(offset, 8);
    return this.view.getFloat64(offset, true);
  }

  // =========================================================================
  // Narrow Loads for i32 (from 8-bit and 16-bit)
  // =========================================================================

  /** Load 1 byte, sign-extend to i32. */
  loadI32_8s(offset: number): number {
    this.boundsCheck(offset, 1);
    return this.view.getInt8(offset);
  }

  /** Load 1 byte, zero-extend to i32. */
  loadI32_8u(offset: number): number {
    this.boundsCheck(offset, 1);
    return this.view.getUint8(offset);
  }

  /** Load 2 bytes (little-endian), sign-extend to i32. */
  loadI32_16s(offset: number): number {
    this.boundsCheck(offset, 2);
    return this.view.getInt16(offset, true);
  }

  /** Load 2 bytes (little-endian), zero-extend to i32. */
  loadI32_16u(offset: number): number {
    this.boundsCheck(offset, 2);
    return this.view.getUint16(offset, true);
  }

  // =========================================================================
  // Narrow Loads for i64 (from 8-bit, 16-bit, and 32-bit)
  // =========================================================================

  /** Load 1 byte, sign-extend to i64. */
  loadI64_8s(offset: number): bigint {
    this.boundsCheck(offset, 1);
    return BigInt(this.view.getInt8(offset));
  }

  /** Load 1 byte, zero-extend to i64. */
  loadI64_8u(offset: number): bigint {
    this.boundsCheck(offset, 1);
    return BigInt(this.view.getUint8(offset));
  }

  /** Load 2 bytes (little-endian), sign-extend to i64. */
  loadI64_16s(offset: number): bigint {
    this.boundsCheck(offset, 2);
    return BigInt(this.view.getInt16(offset, true));
  }

  /** Load 2 bytes (little-endian), zero-extend to i64. */
  loadI64_16u(offset: number): bigint {
    this.boundsCheck(offset, 2);
    return BigInt(this.view.getUint16(offset, true));
  }

  /** Load 4 bytes (little-endian), sign-extend to i64. */
  loadI64_32s(offset: number): bigint {
    this.boundsCheck(offset, 4);
    return BigInt(this.view.getInt32(offset, true));
  }

  /** Load 4 bytes (little-endian), zero-extend to i64. */
  loadI64_32u(offset: number): bigint {
    this.boundsCheck(offset, 4);
    return BigInt(this.view.getUint32(offset, true));
  }

  // =========================================================================
  // Full-Width Stores
  // =========================================================================

  /** Store a 32-bit integer (little-endian). */
  storeI32(offset: number, value: number): void {
    this.boundsCheck(offset, 4);
    this.view.setInt32(offset, value, true);
  }

  /** Store a 64-bit integer (little-endian). */
  storeI64(offset: number, value: bigint): void {
    this.boundsCheck(offset, 8);
    this.view.setBigInt64(offset, value, true);
  }

  /** Store a 32-bit float (little-endian). */
  storeF32(offset: number, value: number): void {
    this.boundsCheck(offset, 4);
    this.view.setFloat32(offset, value, true);
  }

  /** Store a 64-bit float (little-endian). */
  storeF64(offset: number, value: number): void {
    this.boundsCheck(offset, 8);
    this.view.setFloat64(offset, value, true);
  }

  // =========================================================================
  // Narrow Stores (truncate to smaller width)
  // =========================================================================

  /** Store the low 8 bits of an i32. */
  storeI32_8(offset: number, value: number): void {
    this.boundsCheck(offset, 1);
    this.view.setInt8(offset, value);
  }

  /** Store the low 16 bits of an i32 (little-endian). */
  storeI32_16(offset: number, value: number): void {
    this.boundsCheck(offset, 2);
    this.view.setInt16(offset, value, true);
  }

  /** Store the low 8 bits of an i64. */
  storeI64_8(offset: number, value: bigint): void {
    this.boundsCheck(offset, 1);
    this.view.setInt8(offset, Number(BigInt.asIntN(8, value)));
  }

  /** Store the low 16 bits of an i64 (little-endian). */
  storeI64_16(offset: number, value: bigint): void {
    this.boundsCheck(offset, 2);
    this.view.setInt16(offset, Number(BigInt.asIntN(16, value)), true);
  }

  /** Store the low 32 bits of an i64 (little-endian). */
  storeI64_32(offset: number, value: bigint): void {
    this.boundsCheck(offset, 4);
    this.view.setInt32(offset, Number(BigInt.asIntN(32, value)), true);
  }

  // =========================================================================
  // Memory Growth
  // =========================================================================

  /**
   * Grow memory by ``deltaPages`` pages. Returns the old page count on
   * success, or -1 if growth would exceed the maximum.
   *
   * The WASM spec defines grow failure as a normal result (not a trap),
   * so programs can check the return value and handle it gracefully.
   */
  grow(deltaPages: number): number {
    const oldPages = this.currentPages;
    const newPages = oldPages + deltaPages;

    if (this.maxPages !== null && newPages > this.maxPages) {
      return -1;
    }

    /* WASM spec caps memory at 65536 pages (4 GiB). */
    if (newPages > 65536) {
      return -1;
    }

    const newBuffer = new ArrayBuffer(newPages * LinearMemory.PAGE_SIZE);
    new Uint8Array(newBuffer).set(new Uint8Array(this.buffer));
    this.buffer = newBuffer;
    this.view = new DataView(this.buffer);
    this.currentPages = newPages;
    return oldPages;
  }

  // =========================================================================
  // Size Queries
  // =========================================================================

  /** Return current memory size in pages. */
  size(): number {
    return this.currentPages;
  }

  /** Return current memory size in bytes. */
  byteLength(): number {
    return this.buffer.byteLength;
  }

  // =========================================================================
  // Raw Byte Access
  // =========================================================================

  /**
   * Write a raw byte array into memory at the given offset.
   *
   * Used during module instantiation to initialize memory from data
   * segments (e.g., string constants, lookup tables).
   */
  writeBytes(offset: number, data: Uint8Array): void {
    this.boundsCheck(offset, data.length);
    new Uint8Array(this.buffer).set(data, offset);
  }
}
