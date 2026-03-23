/**
 * Register File -- the CPU's fast, small storage.
 *
 * === What are registers? ===
 *
 * Registers are the fastest storage in a computer. They sit inside the CPU
 * itself and can be read or written in a single clock cycle. A typical CPU
 * has between 8 and 32 registers, each holding one "word" of data (e.g., 32
 * bits on a 32-bit CPU).
 *
 * Think of registers like the small whiteboard on your desk. You can glance
 * at it instantly (fast), but it only holds a few things. Memory (RAM) is
 * like a filing cabinet across the room -- it holds much more, but you have
 * to walk over to get something (slow).
 *
 * === Why so few? ===
 *
 * Registers are expensive to build because they need to be extremely fast.
 * Each register is made of flip-flops (built from logic gates), and the
 * wiring to connect them all to the ALU grows quadratically with the number
 * of registers. So CPUs use a small number of very fast registers combined
 * with large but slower memory.
 *
 * === Register conventions ===
 *
 * Different architectures assign special meaning to certain registers:
 *   - RISC-V: x0 is hardwired to 0, x1 = return address, x2 = stack pointer
 *   - ARM: R13 = stack pointer, R14 = link register, R15 = program counter
 *   - Intel 4004: 16 4-bit registers + a 4-bit accumulator
 *
 * Our RegisterFile is generic -- the ISA simulator decides which registers
 * have special behavior (like x0 always being 0 in RISC-V).
 */

/**
 * A set of numbered registers, each holding an integer value.
 *
 * The register file is like a tiny array of named storage slots:
 *
 *     +-----+-----+-----+-----+-----+-----+
 *     | R0  | R1  | R2  | R3  | ... | R15 |
 *     |  0  |  0  |  0  |  0  |     |  0  |
 *     +-----+-----+-----+-----+-----+-----+
 *
 * Read and write by register number:
 *     registers.read(1)       -> value in R1
 *     registers.write(1, 42)  -> R1 = 42
 *
 * Example:
 *     const regs = new RegisterFile(16, 32);
 *     regs.write(1, 42);
 *     regs.read(1);  // 42
 */
export class RegisterFile {
  /** How many registers this file contains. */
  readonly numRegisters: number;

  /** How many bits wide each register is (e.g., 8, 16, 32). */
  readonly bitWidth: number;

  /** Internal storage -- one slot per register. */
  private readonly values: number[];

  /**
   * The bitmask used to enforce bit-width limits.
   * For an 8-bit register, this is 0xFF. For 32-bit, 0xFFFFFFFF.
   */
  private readonly maxValue: number;

  constructor(numRegisters: number = 16, bitWidth: number = 32) {
    this.numRegisters = numRegisters;
    this.bitWidth = bitWidth;
    this.values = new Array<number>(numRegisters).fill(0);
    // JavaScript bit shifts operate on 32-bit integers, so (1 << 32) wraps
    // to 1 instead of producing 2^32. We use unsigned right shift (>>> 0) to
    // handle the 32-bit case correctly: -1 >>> 0 === 0xFFFFFFFF.
    this.maxValue = bitWidth >= 32 ? 0xFFFFFFFF : (1 << bitWidth) - 1;
  }

  /**
   * Read the value stored in register `index`.
   *
   * Example:
   *     const regs = new RegisterFile(4);
   *     regs.write(2, 100);
   *     regs.read(2);  // 100
   */
  read(index: number): number {
    if (index < 0 || index >= this.numRegisters) {
      throw new RangeError(
        `Register index ${index} out of range (0-${this.numRegisters - 1})`
      );
    }
    return this.values[index];
  }

  /**
   * Write a value to register `index`.
   *
   * Values are masked to the register's bit width. For example, on a
   * 32-bit register file, writing 2^32 wraps to 0.
   *
   * Example:
   *     const regs = new RegisterFile(4, 8);
   *     regs.write(0, 256);  // 256 doesn't fit in 8 bits
   *     regs.read(0);        // 0 -- wrapped: 256 & 0xFF = 0
   */
  write(index: number, value: number): void {
    if (index < 0 || index >= this.numRegisters) {
      throw new RangeError(
        `Register index ${index} out of range (0-${this.numRegisters - 1})`
      );
    }
    // The >>> 0 converts the result to an unsigned 32-bit integer.
    // Without it, (0xFFFFFFFF & 0xFFFFFFFF) yields -1 (signed).
    this.values[index] = (value & this.maxValue) >>> 0;
  }

  /**
   * Return all register values as a record for inspection.
   *
   * Example:
   *     const regs = new RegisterFile(4);
   *     regs.write(1, 5);
   *     regs.dump();  // { R0: 0, R1: 5, R2: 0, R3: 0 }
   */
  dump(): Record<string, number> {
    const result: Record<string, number> = {};
    for (let i = 0; i < this.numRegisters; i++) {
      result[`R${i}`] = this.values[i];
    }
    return result;
  }
}
