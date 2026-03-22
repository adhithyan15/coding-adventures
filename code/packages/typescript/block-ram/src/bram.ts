/**
 * Configurable Block RAM -- FPGA-style memory with reconfigurable aspect ratio.
 *
 * === What is Block RAM? ===
 *
 * In an FPGA, Block RAM (BRAM) tiles are dedicated memory blocks separate
 * from the configurable logic. Each tile has a fixed total storage (typically
 * 18 Kbit or 36 Kbit) but can be configured with different width/depth ratios:
 *
 *     18 Kbit BRAM configurations:
 *     +-------------------+-------+-------+--------------+
 *     | Configuration     | Depth | Width | Total bits   |
 *     +-------------------+-------+-------+--------------+
 *     | 16K x 1           | 16384 |     1 |        16384 |
 *     |  8K x 2           |  8192 |     2 |        16384 |
 *     |  4K x 4           |  4096 |     4 |        16384 |
 *     |  2K x 8           |  2048 |     8 |        16384 |
 *     |  1K x 16          |  1024 |    16 |        16384 |
 *     | 512 x 32          |   512 |    32 |        16384 |
 *     +-------------------+-------+-------+--------------+
 *
 * The total storage is fixed; you trade depth for width by changing how the
 * address decoder and column MUX are configured. The underlying SRAM cells
 * don't change -- only the access pattern changes.
 *
 * This module wraps DualPortRAM with reconfiguration support.
 */

import { type Bit, validateBit } from "@coding-adventures/logic-gates";
import { DualPortRAM } from "./ram.js";

/**
 * Block RAM with configurable aspect ratio.
 *
 * Total storage is fixed at initialization. Width and depth can be
 * reconfigured as long as width x depth <= totalBits.
 *
 * Supports dual-port access via tickA and tickB methods.
 *
 * @example
 * const bram = new ConfigurableBRAM(1024, 8);
 * bram.depth  // 1024 / 8 = 128
 * bram.reconfigure(16);
 * bram.depth  // 1024 / 16 = 64
 */
export class ConfigurableBRAM {
  private readonly _totalBits: number;
  private _width: number;
  private _depth: number;
  private _ram: DualPortRAM;

  /**
   * @param totalBits - Total storage in bits (default: 18432 = 18 Kbit)
   * @param width - Initial bits per word (default: 8)
   */
  constructor(totalBits: number = 18432, width: number = 8) {
    if (totalBits < 1) {
      throw new RangeError(`totalBits must be >= 1, got ${totalBits}`);
    }
    if (width < 1) {
      throw new RangeError(`width must be >= 1, got ${width}`);
    }
    if (totalBits % width !== 0) {
      throw new RangeError(
        `width ${width} does not evenly divide totalBits ${totalBits}`,
      );
    }

    this._totalBits = totalBits;
    this._width = width;
    this._depth = totalBits / width;
    this._ram = new DualPortRAM(this._depth, this._width);
  }

  /**
   * Change the aspect ratio. Clears all stored data.
   *
   * @param width - New bits per word. Must evenly divide totalBits.
   * @throws RangeError if width doesn't divide totalBits or is < 1.
   */
  reconfigure(width: number): void {
    if (width < 1) {
      throw new RangeError(`width must be >= 1, got ${width}`);
    }
    if (this._totalBits % width !== 0) {
      throw new RangeError(
        `width ${width} does not evenly divide totalBits ${this._totalBits}`,
      );
    }

    this._width = width;
    this._depth = this._totalBits / width;
    this._ram = new DualPortRAM(this._depth, this._width);
  }

  /**
   * Port A operation.
   *
   * @param clock - Clock signal (0 or 1)
   * @param address - Word address (0 to depth-1)
   * @param dataIn - Write data (array of width bits)
   * @param writeEnable - 0 = read, 1 = write
   * @returns dataOut: array of width bits.
   */
  tickA(
    clock: Bit,
    address: number,
    dataIn: Bit[],
    writeEnable: Bit,
  ): Bit[] {
    validateBit(clock, "clock");

    // Use the dual-port RAM with port B idle (read address 0)
    const zeros = Array(this._width).fill(0) as Bit[];
    const [outA] = this._ram.tick(
      clock,
      address,
      dataIn,
      writeEnable,
      0,
      zeros,
      0,
    );
    return outA;
  }

  /**
   * Port B operation.
   *
   * @param clock - Clock signal (0 or 1)
   * @param address - Word address (0 to depth-1)
   * @param dataIn - Write data (array of width bits)
   * @param writeEnable - 0 = read, 1 = write
   * @returns dataOut: array of width bits.
   */
  tickB(
    clock: Bit,
    address: number,
    dataIn: Bit[],
    writeEnable: Bit,
  ): Bit[] {
    validateBit(clock, "clock");

    // Use the dual-port RAM with port A idle
    const zeros = Array(this._width).fill(0) as Bit[];
    const [, outB] = this._ram.tick(
      clock,
      0,
      zeros,
      0,
      address,
      dataIn,
      writeEnable,
    );
    return outB;
  }

  /** Number of addressable words at current configuration. */
  get depth(): number {
    return this._depth;
  }

  /** Bits per word at current configuration. */
  get width(): number {
    return this._width;
  }

  /** Total storage capacity in bits (fixed). */
  get totalBits(): number {
    return this._totalBits;
  }
}
