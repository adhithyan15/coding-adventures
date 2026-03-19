/**
 * Memory -- the CPU's large, slow storage.
 *
 * === What is memory? ===
 *
 * Memory (RAM -- Random Access Memory) is a large array of bytes that the CPU
 * can read from and write to. Unlike registers (which are tiny and fast),
 * memory can hold megabytes or gigabytes of data, but accessing it takes
 * many clock cycles.
 *
 * Every byte in memory has an "address" -- a number that identifies its
 * location, like a house number on a street. To read a byte, you tell the
 * memory controller "give me the byte at address 42." To write, you say
 * "put the value 7 at address 42."
 *
 * === Memory in our simulator ===
 *
 * We simulate memory as a Uint8Array -- a typed array of unsigned bytes.
 * Each element is one byte (0-255). Multi-byte values (like 32-bit integers)
 * are stored in consecutive bytes.
 *
 * === Byte ordering (Endianness) ===
 *
 * When storing a multi-byte value (like the 32-bit integer 0x12345678),
 * there are two ways to lay out the bytes:
 *
 *   Big-endian:    [0x12] [0x34] [0x56] [0x78]   (most significant byte first)
 *   Little-endian: [0x78] [0x56] [0x34] [0x12]   (least significant byte first)
 *
 * RISC-V and x86 use little-endian. ARM supports both. Our simulator
 * defaults to little-endian because that's what RISC-V uses.
 *
 * Think of it like writing the number 1234:
 *   - Big-endian is like English: you write the thousands digit first (1, 2, 3, 4)
 *   - Little-endian is the opposite: ones digit first (4, 3, 2, 1)
 */

/**
 * Byte-addressable memory.
 *
 * Memory is a flat array of bytes. Each byte is addressed by an integer
 * starting from 0.
 *
 *     Address:  0     1     2     3     4     5    ...
 *     Value:   [00]  [00]  [00]  [00]  [00]  [00]  ...
 *
 * Example:
 *     const mem = new Memory(1024);  // 1 KB of memory
 *     mem.writeByte(0, 42);
 *     mem.readByte(0);  // 42
 *     mem.writeWord(4, 0x12345678);  // Write a 32-bit value
 *     mem.readWord(4);  // 0x12345678
 */
export class Memory {
  /** Internal byte storage. */
  private readonly data: Uint8Array;

  /** Total number of bytes in this memory. */
  readonly size: number;

  /**
   * Create a memory of `size` bytes, all initialized to 0.
   *
   * @param size Number of bytes. Default is 64 KB (65536 bytes), which is
   *             enough for our simple programs. Real computers have billions
   *             of bytes (gigabytes).
   */
  constructor(size: number = 65536) {
    if (size < 1) {
      throw new RangeError("Memory size must be at least 1 byte");
    }
    this.data = new Uint8Array(size);
    this.size = size;
  }

  /** Verify an address is within bounds. */
  private checkAddress(address: number, numBytes: number = 1): void {
    if (address < 0 || address + numBytes > this.size) {
      throw new RangeError(
        `Memory access out of bounds: address ${address}, ` +
          `size ${numBytes}, memory size ${this.size}`
      );
    }
  }

  /**
   * Read a single byte (8 bits, value 0-255) from memory.
   *
   * Example:
   *     const mem = new Memory(16);
   *     mem.writeByte(3, 0xFF);
   *     mem.readByte(3);  // 255
   */
  readByte(address: number): number {
    this.checkAddress(address);
    return this.data[address];
  }

  /**
   * Write a single byte to memory. Value is masked to 0-255.
   *
   * Example:
   *     const mem = new Memory(16);
   *     mem.writeByte(0, 42);
   *     mem.readByte(0);  // 42
   */
  writeByte(address: number, value: number): void {
    this.checkAddress(address);
    this.data[address] = value & 0xff;
  }

  /**
   * Read a 32-bit word (4 bytes) from memory, little-endian.
   *
   * Little-endian means the least significant byte is at the lowest
   * address. For example, the value 0x12345678 is stored as:
   *
   *     Address:   [addr]  [addr+1]  [addr+2]  [addr+3]
   *     Value:      0x78    0x56      0x34      0x12
   *                 ^^^^                        ^^^^
   *                 LSB (least significant)     MSB (most significant)
   *
   * Example:
   *     const mem = new Memory(16);
   *     mem.writeWord(0, 0x12345678);
   *     mem.readWord(0);  // 0x12345678
   */
  readWord(address: number): number {
    this.checkAddress(address, 4);
    return (
      (this.data[address] |
        (this.data[address + 1] << 8) |
        (this.data[address + 2] << 16) |
        (this.data[address + 3] << 24)) >>>
      0
    );
  }

  /**
   * Write a 32-bit word to memory, little-endian.
   *
   * Example:
   *     const mem = new Memory(16);
   *     mem.writeWord(0, 3);       // 3 = 0x00000003
   *     mem.readByte(0);           // 3 (LSB)
   *     mem.readByte(1);           // 0 (next byte)
   */
  writeWord(address: number, value: number): void {
    this.checkAddress(address, 4);
    // Mask to 32 bits using unsigned right shift to handle JS number quirks
    const masked = (value & 0xffffffff) >>> 0;
    this.data[address] = masked & 0xff;
    this.data[address + 1] = (masked >>> 8) & 0xff;
    this.data[address + 2] = (masked >>> 16) & 0xff;
    this.data[address + 3] = (masked >>> 24) & 0xff;
  }

  /**
   * Load a sequence of bytes into memory starting at `address`.
   *
   * This is how programs are loaded: the machine code bytes are copied
   * into memory starting at address 0 (or wherever the program begins).
   *
   * Example:
   *     const mem = new Memory(16);
   *     mem.loadBytes(0, [0x01, 0x02, 0x03]);
   *     mem.readByte(0);  // 1
   *     mem.readByte(1);  // 2
   *     mem.readByte(2);  // 3
   */
  loadBytes(address: number, data: number[] | Uint8Array): void {
    this.checkAddress(address, data.length);
    for (let i = 0; i < data.length; i++) {
      this.data[address + i] = data[i];
    }
  }

  /**
   * Return a slice of memory as an array of byte values.
   *
   * Useful for debugging -- see what's stored in a range of addresses.
   *
   * Example:
   *     const mem = new Memory(16);
   *     mem.writeByte(0, 0xAB);
   *     mem.dump(0, 4);  // [171, 0, 0, 0]
   */
  dump(start: number = 0, length: number = 16): number[] {
    this.checkAddress(start, length);
    return Array.from(this.data.slice(start, start + length));
  }
}
