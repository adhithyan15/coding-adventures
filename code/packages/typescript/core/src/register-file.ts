/**
 * RegisterFile -- general-purpose register file for the Core.
 *
 * The register file is the Core's fast, small storage that the pipeline
 * reads and writes every cycle.
 *
 * # Zero Register Convention
 *
 * In RISC-V and MIPS, register x0 is hardwired to 0.
 * Writes to it are silently discarded. This simplifies instruction encoding:
 *
 *     MOV Rd, Rs  = ADD Rd, Rs, x0   (add zero)
 *     NOP         = ADD x0, x0, x0   (write nothing)
 *     NEG Rd, Rs  = SUB Rd, x0, Rs   (subtract from zero)
 */

import {
  type RegisterFileConfig,
  defaultRegisterFileConfig,
} from "./config.js";

export class RegisterFile {
  private _config: RegisterFileConfig;
  private _values: number[];
  private _mask: number;

  /**
   * Creates a new register file from the given configuration.
   * All registers are initialized to 0.
   */
  constructor(config?: RegisterFileConfig | null) {
    this._config = config ?? defaultRegisterFileConfig();

    // Compute the bit mask for the register width.
    // JavaScript bitwise shifts are mod 32, so (1 << 32) === 1 (not 2^32).
    // We must handle widths >= 32 specially.
    if (this._config.width >= 53) {
      // JavaScript numbers are 64-bit floats; safe integer range is 2^53-1.
      this._mask = Number.MAX_SAFE_INTEGER;
    } else if (this._config.width >= 32) {
      // Use Math.pow to avoid the 32-bit shift wrap-around.
      this._mask = Math.pow(2, this._config.width) - 1;
    } else {
      this._mask = (1 << this._config.width) - 1;
    }

    this._values = new Array(this._config.count).fill(0);
  }

  /**
   * Returns the value of register at the given index.
   *
   * If zero register convention is enabled, reading register 0 always returns 0.
   * Returns 0 for out-of-range indices (defensive).
   */
  read(index: number): number {
    if (index < 0 || index >= this._config.count) return 0;
    if (this._config.zeroRegister && index === 0) return 0;
    return this._values[index];
  }

  /**
   * Stores a value into the register at the given index.
   *
   * The value is masked to the register width. Writes to register 0
   * are silently ignored when zero register convention is enabled.
   */
  write(index: number, value: number): void {
    if (index < 0 || index >= this._config.count) return;
    if (this._config.zeroRegister && index === 0) return;
    this._values[index] = value & this._mask;
  }

  /** Returns a copy of all register values. */
  values(): number[] {
    return [...this._values];
  }

  /** Returns the number of registers. */
  count(): number {
    return this._config.count;
  }

  /** Returns the bit width of each register. */
  width(): number {
    return this._config.width;
  }

  /** Returns the register file configuration. */
  getConfig(): RegisterFileConfig {
    return this._config;
  }

  /** Sets all registers to zero. */
  reset(): void {
    this._values.fill(0);
  }

  /** Returns a human-readable dump of non-zero registers. */
  toString(): string {
    let s = `RegisterFile(${this._config.count}x${this._config.width}):`;
    for (let i = 0; i < this._config.count; i++) {
      if (this._values[i] !== 0) {
        s += ` R${i}=${this._values[i]}`;
      }
    }
    return s;
  }
}
