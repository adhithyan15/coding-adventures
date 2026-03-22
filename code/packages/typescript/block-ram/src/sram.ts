/**
 * SRAM -- Static Random-Access Memory at the gate level.
 *
 * === What is SRAM? ===
 *
 * SRAM (Static Random-Access Memory) is the fastest type of memory in a
 * computer. It's used for CPU caches (L1/L2/L3), register files, and FPGA
 * Block RAM. "Static" means the memory holds its value as long as power is
 * supplied -- unlike DRAM, which must be periodically refreshed.
 *
 * === The SRAM Cell -- 6 Transistors Holding 1 Bit ===
 *
 * In real hardware, each SRAM cell uses 6 transistors:
 * - 2 cross-coupled inverters forming a bistable latch (stores the bit)
 * - 2 access transistors controlled by the word line (gates read/write)
 *
 * We model this at the gate level:
 * - Cross-coupled inverters = two NOT gates in a feedback loop
 *   (identical to the logic behind an SR latch from logic_gates.sequential)
 * - Access transistors = AND gates that pass data only when word_line=1
 *
 * The cell has three operations:
 * - **Hold** (word_line=0): Access transistors block external access.
 *   The inverter loop maintains the stored value indefinitely.
 * - **Read** (word_line=1): Access transistors open. The stored value
 *   appears on the bit lines without disturbing it.
 * - **Write** (word_line=1 + drive bit lines): The external driver
 *   overpowers the internal inverters, forcing a new value.
 *
 * === From Cell to Array ===
 *
 * A RAM chip is a 2D grid of SRAM cells. To access a specific cell:
 * 1. A **row decoder** converts address bits into a one-hot word line signal
 * 2. A **column MUX** selects which columns to read/write
 *
 * This module provides:
 * - SRAMCell: single-bit storage at the gate level
 * - SRAMArray: 2D grid with row/column addressing
 */

import { type Bit, validateBit } from "@coding-adventures/logic-gates";

/**
 * Single-bit storage element modeled at the gate level.
 *
 * Internally, this is a pair of cross-coupled inverters (forming a
 * bistable latch) gated by access transistors controlled by the word line.
 *
 * In our simulation, we model the steady-state behavior directly rather
 * than simulating individual gate delays:
 * - word_line=0: cell is isolated, value is retained
 * - word_line=1, reading: value is output
 * - word_line=1, writing: new value overwrites stored value
 *
 * This matches the real behavior of a 6T SRAM cell while keeping the
 * simulation fast enough to model arrays of thousands of cells.
 *
 * @example
 * const cell = new SRAMCell();
 * cell.value        // 0
 * cell.write(1, 1);
 * cell.value        // 1
 * cell.read(1)      // 1
 * cell.read(0)      // null (not selected)
 */
export class SRAMCell {
  private _value: Bit = 0;

  /**
   * Read the stored bit if the cell is selected.
   *
   * @param wordLine - 1 = cell selected (access transistors open),
   *                   0 = cell not selected (isolated)
   * @returns The stored bit (0 or 1) when wordLine=1,
   *          null when wordLine=0 (cell not selected, no output).
   */
  read(wordLine: Bit): Bit | null {
    validateBit(wordLine, "wordLine");

    if (wordLine === 0) {
      return null;
    }

    return this._value;
  }

  /**
   * Write a bit to the cell if selected.
   *
   * When wordLine=1, the access transistors open and the external
   * bit line driver overpowers the internal inverter loop, forcing
   * the cell to store the new value.
   *
   * When wordLine=0, the access transistors are closed and the
   * write has no effect -- the cell retains its previous value.
   *
   * @param wordLine - 1 = cell selected, 0 = cell not selected
   * @param bitLine - The value to store (0 or 1)
   */
  write(wordLine: Bit, bitLine: Bit): void {
    validateBit(wordLine, "wordLine");
    validateBit(bitLine, "bitLine");

    if (wordLine === 1) {
      this._value = bitLine;
    }
  }

  /** Current stored value (for inspection/debugging). */
  get value(): Bit {
    return this._value;
  }
}

/**
 * 2D grid of SRAM cells with row/column addressing.
 *
 * An SRAM array organizes cells into rows and columns:
 * - Each row shares a word line (activated by the row decoder)
 * - Each column shares a bit line (carries data in/out)
 *
 * To read: activate a row's word line -> all cells in that row
 * output their values onto their respective bit lines.
 *
 * To write: activate a row's word line and drive the bit lines
 * with the desired data -> all cells in that row store the new values.
 *
 * Memory map (4x4 array example):
 *
 *     Row 0 (WL0): [Cell00] [Cell01] [Cell02] [Cell03]
 *     Row 1 (WL1): [Cell10] [Cell11] [Cell12] [Cell13]
 *     Row 2 (WL2): [Cell20] [Cell21] [Cell22] [Cell23]
 *     Row 3 (WL3): [Cell30] [Cell31] [Cell32] [Cell33]
 *
 * @example
 * const arr = new SRAMArray(4, 8);     // 4 rows x 8 columns
 * arr.write(0, [1,0,1,0, 0,1,0,1]);
 * arr.read(0)   // [1, 0, 1, 0, 0, 1, 0, 1]
 * arr.read(1)   // [0, 0, 0, 0, 0, 0, 0, 0]
 */
export class SRAMArray {
  private readonly _rows: number;
  private readonly _cols: number;
  private readonly _cells: SRAMCell[][];

  /**
   * Create an SRAM array initialized to all zeros.
   *
   * @param rows - Number of rows (>= 1)
   * @param cols - Number of columns (>= 1)
   * @throws RangeError if rows or cols < 1
   */
  constructor(rows: number, cols: number) {
    if (rows < 1) {
      throw new RangeError(`rows must be >= 1, got ${rows}`);
    }
    if (cols < 1) {
      throw new RangeError(`cols must be >= 1, got ${cols}`);
    }

    this._rows = rows;
    this._cols = cols;
    this._cells = Array.from({ length: rows }, () =>
      Array.from({ length: cols }, () => new SRAMCell()),
    );
  }

  /**
   * Read all columns of a row.
   *
   * Activates the word line for the given row, causing all cells
   * in that row to output their stored values.
   *
   * @param row - Row index (0 to rows-1)
   * @returns Array of bits, one per column.
   * @throws RangeError if row is out of range.
   */
  read(row: number): Bit[] {
    this._validateRow(row);

    // Activate word line for this row -- read all cells
    const result: Bit[] = [];
    for (const cell of this._cells[row]) {
      const val = cell.read(1);
      // word_line=1 always returns Bit, not null
      result.push(val as Bit);
    }
    return result;
  }

  /**
   * Write data to a row.
   *
   * Activates the word line for the given row and drives the bit
   * lines with the given data, storing values in all cells of the row.
   *
   * @param row - Row index (0 to rows-1)
   * @param data - Array of bits to write, one per column.
   *              Length must equal the number of columns.
   * @throws RangeError if row is out of range or data length doesn't match cols.
   * @throws TypeError if data is not an array.
   */
  write(row: number, data: Bit[]): void {
    this._validateRow(row);

    if (!Array.isArray(data)) {
      throw new TypeError("data must be an array of bits");
    }

    if (data.length !== this._cols) {
      throw new RangeError(
        `data length ${data.length} does not match cols ${this._cols}`,
      );
    }

    for (let i = 0; i < data.length; i++) {
      validateBit(data[i], `data[${i}]`);
    }

    // Activate word line and drive bit lines
    for (let col = 0; col < data.length; col++) {
      this._cells[row][col].write(1, data[col]);
    }
  }

  /** Array dimensions as [rows, cols]. */
  get shape(): [number, number] {
    return [this._rows, this._cols];
  }

  /** Check that row index is in range. */
  private _validateRow(row: number): void {
    if (typeof row !== "number" || !Number.isInteger(row)) {
      throw new TypeError(`row must be an integer, got ${typeof row}`);
    }
    if (row < 0 || row >= this._rows) {
      throw new RangeError(
        `row ${row} out of range [0, ${this._rows - 1}]`,
      );
    }
  }
}
