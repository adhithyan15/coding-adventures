/**
 * RAM Modules -- synchronous memory with read/write ports.
 *
 * === From Array to Module ===
 *
 * An SRAM array (sram.ts) provides raw row-level read/write. A RAM module
 * adds the interface that digital circuits actually use:
 *
 * 1. **Address decoding** -- binary address bits select a row
 * 2. **Synchronous operation** -- reads and writes happen on clock edges
 * 3. **Read modes** -- what the output shows during a write operation
 * 4. **Dual-port access** -- two independent ports for simultaneous operations
 *
 * === Read Modes ===
 *
 * During a write operation, what should the data output show? There are
 * three valid answers, and different designs need different behaviors:
 *
 * 1. **Read-first**: Output shows the OLD value at the address being written.
 *    The read happens before the write within the same cycle. Useful when
 *    you need to know what was there before overwriting it.
 *
 * 2. **Write-first** (read-after-write): Output shows the NEW value being
 *    written. The write happens first, then the read sees the new value.
 *    Useful for pipeline forwarding.
 *
 * 3. **No-change**: Output retains its previous value during writes. This
 *    saves power in FPGA Block RAMs because the read circuitry doesn't
 *    activate during writes.
 *
 * === Dual-Port RAM ===
 *
 * Two completely independent ports (A and B), each with its own address,
 * data, and write enable. Both can operate simultaneously:
 * - Read A + Read B at different addresses -> both get their data
 * - Write A + Read B at different addresses -> both succeed
 * - Write A + Write B at the SAME address -> **collision** (undefined in
 *   hardware, we raise an error)
 */

import { type Bit, validateBit } from "@coding-adventures/logic-gates";
import { SRAMArray } from "./sram.js";

/**
 * Controls what dataOut shows during a write operation.
 *
 * - READ_FIRST:  dataOut = old value (read before write)
 * - WRITE_FIRST: dataOut = new value (write before read)
 * - NO_CHANGE:   dataOut = previous read value (output unchanged)
 */
export enum ReadMode {
  READ_FIRST = "read_first",
  WRITE_FIRST = "write_first",
  NO_CHANGE = "no_change",
}

/**
 * Raised when both ports of a dual-port RAM write to the same address.
 *
 * In real hardware, simultaneous writes to the same address produce
 * undefined results (the cell may store either value, or a corrupted
 * value). We detect this and throw an error to prevent silent bugs.
 */
export class WriteCollisionError extends Error {
  readonly address: number;

  constructor(address: number) {
    super(`Write collision: both ports writing to address ${address}`);
    this.name = "WriteCollisionError";
    this.address = address;
  }
}

/**
 * Single-port synchronous RAM.
 *
 * One address port, one data bus. Each clock cycle you can do ONE
 * operation: read OR write (controlled by writeEnable).
 *
 * Interface:
 *
 *                 +----------------------------+
 *   address ------+                            |
 *                 |     Single-Port RAM        +---- dataOut
 *   dataIn -------+                            |
 *                 |     (depth x width)        |
 *   writeEn ------+                            |
 *                 |                            |
 *   clock --------+                            |
 *                 +----------------------------+
 *
 * Operations happen on the rising edge of the clock (transition 0->1).
 *
 * @example
 * const ram = new SinglePortRAM(256, 8);
 * // Write 0xFF to address 0
 * ram.tick(0, 0, [1,1,1,1,1,1,1,1], 1);
 * let out = ram.tick(1, 0, [1,1,1,1,1,1,1,1], 1);
 * // Read from address 0
 * ram.tick(0, 0, [0,0,0,0,0,0,0,0], 0);
 * out = ram.tick(1, 0, [0,0,0,0,0,0,0,0], 0);
 * // out === [1, 1, 1, 1, 1, 1, 1, 1]
 */
export class SinglePortRAM {
  private readonly _depth: number;
  private readonly _width: number;
  private readonly _readMode: ReadMode;
  private readonly _array: SRAMArray;
  private _prevClock: Bit = 0;
  private _lastRead: Bit[];

  /**
   * @param depth - Number of addressable words (>= 1)
   * @param width - Bits per word (>= 1)
   * @param readMode - What dataOut shows during writes (default: READ_FIRST)
   */
  constructor(
    depth: number,
    width: number,
    readMode: ReadMode = ReadMode.READ_FIRST,
  ) {
    if (depth < 1) {
      throw new RangeError(`depth must be >= 1, got ${depth}`);
    }
    if (width < 1) {
      throw new RangeError(`width must be >= 1, got ${width}`);
    }

    this._depth = depth;
    this._width = width;
    this._readMode = readMode;
    this._array = new SRAMArray(depth, width);
    this._lastRead = Array(width).fill(0) as Bit[];
  }

  /**
   * Execute one half-cycle. Operations happen on rising edge (0->1).
   *
   * @param clock - Clock signal (0 or 1)
   * @param address - Word address (integer, 0 to depth-1)
   * @param dataIn - Data to write (array of width bits)
   * @param writeEnable - 0 = read, 1 = write
   * @returns dataOut: array of width bits read from the address.
   *          During writes, behavior depends on readMode.
   */
  tick(
    clock: Bit,
    address: number,
    dataIn: Bit[],
    writeEnable: Bit,
  ): Bit[] {
    validateBit(clock, "clock");
    validateBit(writeEnable, "writeEnable");
    this._validateAddress(address);
    this._validateData(dataIn);

    // Detect rising edge: previous clock was 0, now it's 1
    const risingEdge = this._prevClock === 0 && clock === 1;
    this._prevClock = clock;

    if (!risingEdge) {
      return [...this._lastRead];
    }

    // Rising edge: perform the operation
    if (writeEnable === 0) {
      // Read operation
      this._lastRead = this._array.read(address);
      return [...this._lastRead];
    }

    // Write operation -- behavior depends on read mode
    if (this._readMode === ReadMode.READ_FIRST) {
      // Read the old value first, then write
      this._lastRead = this._array.read(address);
      this._array.write(address, dataIn);
      return [...this._lastRead];
    }

    if (this._readMode === ReadMode.WRITE_FIRST) {
      // Write first, then read back the new value
      this._array.write(address, dataIn);
      this._lastRead = [...dataIn];
      return [...this._lastRead];
    }

    // NO_CHANGE: write but don't update dataOut
    this._array.write(address, dataIn);
    return [...this._lastRead];
  }

  /** Number of addressable words. */
  get depth(): number {
    return this._depth;
  }

  /** Bits per word. */
  get width(): number {
    return this._width;
  }

  /**
   * Return all contents for inspection.
   * @returns Array of rows, each row is an array of bits.
   */
  dump(): Bit[][] {
    return Array.from({ length: this._depth }, (_, i) =>
      this._array.read(i),
    );
  }

  private _validateAddress(address: number): void {
    if (typeof address !== "number" || !Number.isInteger(address)) {
      throw new TypeError(`address must be an integer, got ${typeof address}`);
    }
    if (address < 0 || address >= this._depth) {
      throw new RangeError(
        `address ${address} out of range [0, ${this._depth - 1}]`,
      );
    }
  }

  private _validateData(dataIn: Bit[]): void {
    if (!Array.isArray(dataIn)) {
      throw new TypeError("dataIn must be an array of bits");
    }
    if (dataIn.length !== this._width) {
      throw new RangeError(
        `dataIn length ${dataIn.length} does not match width ${this._width}`,
      );
    }
    for (let i = 0; i < dataIn.length; i++) {
      validateBit(dataIn[i], `dataIn[${i}]`);
    }
  }
}

/**
 * True dual-port synchronous RAM.
 *
 * Two independent ports (A and B), each with its own address, data,
 * and write enable. Both ports can operate simultaneously on different
 * addresses.
 *
 * Write collision: if both ports write to the same address in the
 * same cycle, a WriteCollisionError is thrown.
 *
 * @example
 * const ram = new DualPortRAM(256, 8);
 * // Write via port A, read via port B simultaneously
 */
export class DualPortRAM {
  private readonly _depth: number;
  private readonly _width: number;
  private readonly _readModeA: ReadMode;
  private readonly _readModeB: ReadMode;
  private readonly _array: SRAMArray;
  private _prevClock: Bit = 0;
  private _lastReadA: Bit[];
  private _lastReadB: Bit[];

  /**
   * @param depth - Number of addressable words (>= 1)
   * @param width - Bits per word (>= 1)
   * @param readModeA - Read mode for port A (default: READ_FIRST)
   * @param readModeB - Read mode for port B (default: READ_FIRST)
   */
  constructor(
    depth: number,
    width: number,
    readModeA: ReadMode = ReadMode.READ_FIRST,
    readModeB: ReadMode = ReadMode.READ_FIRST,
  ) {
    if (depth < 1) {
      throw new RangeError(`depth must be >= 1, got ${depth}`);
    }
    if (width < 1) {
      throw new RangeError(`width must be >= 1, got ${width}`);
    }

    this._depth = depth;
    this._width = width;
    this._readModeA = readModeA;
    this._readModeB = readModeB;
    this._array = new SRAMArray(depth, width);
    this._lastReadA = Array(width).fill(0) as Bit[];
    this._lastReadB = Array(width).fill(0) as Bit[];
  }

  /**
   * Execute one half-cycle on both ports.
   *
   * @param clock - Clock signal (0 or 1)
   * @param addressA - Port A word address
   * @param dataInA - Port A write data
   * @param writeEnableA - Port A write enable (0=read, 1=write)
   * @param addressB - Port B word address
   * @param dataInB - Port B write data
   * @param writeEnableB - Port B write enable (0=read, 1=write)
   * @returns [dataOutA, dataOutB]: Read data from each port.
   * @throws WriteCollisionError if both ports write to the same address.
   */
  tick(
    clock: Bit,
    addressA: number,
    dataInA: Bit[],
    writeEnableA: Bit,
    addressB: number,
    dataInB: Bit[],
    writeEnableB: Bit,
  ): [Bit[], Bit[]] {
    validateBit(clock, "clock");
    validateBit(writeEnableA, "writeEnableA");
    validateBit(writeEnableB, "writeEnableB");
    this._validateAddress(addressA, "addressA");
    this._validateAddress(addressB, "addressB");
    this._validateData(dataInA, "dataInA");
    this._validateData(dataInB, "dataInB");

    const risingEdge = this._prevClock === 0 && clock === 1;
    this._prevClock = clock;

    if (!risingEdge) {
      return [[...this._lastReadA], [...this._lastReadB]];
    }

    // Check for write collision
    if (
      writeEnableA === 1 &&
      writeEnableB === 1 &&
      addressA === addressB
    ) {
      throw new WriteCollisionError(addressA);
    }

    // Process port A
    const outA = this._processPort(
      addressA,
      dataInA,
      writeEnableA,
      this._readModeA,
      this._lastReadA,
    );
    this._lastReadA = outA;

    // Process port B
    const outB = this._processPort(
      addressB,
      dataInB,
      writeEnableB,
      this._readModeB,
      this._lastReadB,
    );
    this._lastReadB = outB;

    return [[...outA], [...outB]];
  }

  /** Number of addressable words. */
  get depth(): number {
    return this._depth;
  }

  /** Bits per word. */
  get width(): number {
    return this._width;
  }

  private _processPort(
    address: number,
    dataIn: Bit[],
    writeEnable: Bit,
    readMode: ReadMode,
    lastRead: Bit[],
  ): Bit[] {
    if (writeEnable === 0) {
      return this._array.read(address);
    }

    if (readMode === ReadMode.READ_FIRST) {
      const result = this._array.read(address);
      this._array.write(address, dataIn);
      return result;
    }

    if (readMode === ReadMode.WRITE_FIRST) {
      this._array.write(address, dataIn);
      return [...dataIn];
    }

    // NO_CHANGE
    this._array.write(address, dataIn);
    return [...lastRead];
  }

  private _validateAddress(address: number, name: string = "address"): void {
    if (typeof address !== "number" || !Number.isInteger(address)) {
      throw new TypeError(`${name} must be an integer, got ${typeof address}`);
    }
    if (address < 0 || address >= this._depth) {
      throw new RangeError(
        `${name} ${address} out of range [0, ${this._depth - 1}]`,
      );
    }
  }

  private _validateData(dataIn: Bit[], name: string = "dataIn"): void {
    if (!Array.isArray(dataIn)) {
      throw new TypeError(`${name} must be an array of bits`);
    }
    if (dataIn.length !== this._width) {
      throw new RangeError(
        `${name} length ${dataIn.length} does not match width ${this._width}`,
      );
    }
    for (let i = 0; i < dataIn.length; i++) {
      validateBit(dataIn[i], `${name}[${i}]`);
    }
  }
}
