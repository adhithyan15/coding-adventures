/**
 * Sparse Memory -- simulating a 32-bit address space without allocating 4 GB.
 *
 * === Why sparse? ===
 *
 * A real 32-bit CPU can address 4 GB of memory (2^32 bytes). But most of
 * that address space is empty. A typical embedded system might have:
 *
 *   0x00000000 - 0x000FFFFF: 1 MB of RAM (for code and data)
 *   0xFFFB0000 - 0xFFFFFFFF: 320 KB of I/O registers (for peripherals)
 *
 * Allocating a contiguous 4 GB array would be wasteful and impractical.
 * SparseMemory solves this by mapping only the regions that actually exist.
 *
 * === How it works ===
 *
 * Think of SparseMemory as a building with multiple floors:
 *
 *   Floor 0 (0x00000000): RAM      -- read/write, for code and data
 *   Floor N (0xFFFB0000): I/O Regs -- some read-only, some read/write
 *
 * When the CPU reads address 0x00001234, we find which "floor" contains
 * that address (RAM, at base 0x00000000), compute the offset within the
 * floor (0x1234), and read from that floor's backing byte array.
 *
 * === Read-only regions ===
 *
 * Some regions should never be written to (e.g., ROM). When a region is
 * marked readOnly, writes are silently ignored, matching real hardware
 * behavior where writing to ROM has no effect.
 */

/**
 * MemoryRegion defines a contiguous block of addressable memory.
 *
 * Each region has a base address, a size, and a backing Uint8Array.
 * The region occupies addresses [base, base + size). Any access within
 * this range is translated to an offset into the data array:
 *
 *   offset = address - base
 *   value  = data[offset]
 */
export interface MemoryRegionConfig {
  /** Starting address of this region in the 32-bit address space. */
  base: number;
  /** Number of bytes in this region. */
  size: number;
  /** Human-readable label for debugging (e.g., "RAM", "ROM", "UART"). */
  name: string;
  /** When true, writes are silently discarded (models ROM/flash). */
  readOnly?: boolean;
  /** Optional pre-loaded data. If omitted, region is zero-filled. */
  data?: Uint8Array;
}

interface InternalRegion {
  base: number;
  size: number;
  name: string;
  readOnly: boolean;
  data: Uint8Array;
}

/**
 * SparseMemory maps address ranges to backing byte arrays, enabling a
 * full 32-bit address space without allocating 4 GB.
 *
 * On every access, SparseMemory searches its regions to find one that
 * contains the target address. This is a linear scan -- O(N) where N
 * is the number of regions. For the small number of regions in a
 * typical system (2-10), this is negligible.
 *
 * If no region contains the target address, the access throws an error.
 * On real hardware this would be a bus fault.
 *
 * Example:
 * ```ts
 * const mem = new SparseMemory([
 *   { base: 0x00000000, size: 0x100000, name: "RAM" },
 *   { base: 0xFFFB0000, size: 0x50000, name: "I/O", readOnly: true },
 * ]);
 * mem.writeByte(0x1000, 42);
 * mem.readByte(0x1000);  // 42
 * ```
 */
export class SparseMemory {
  /** The list of mapped memory regions. */
  readonly regions: InternalRegion[];

  constructor(regionConfigs: MemoryRegionConfig[]) {
    this.regions = regionConfigs.map((r) => ({
      base: r.base >>> 0,
      size: r.size >>> 0,
      name: r.name,
      readOnly: r.readOnly ?? false,
      data: r.data ? new Uint8Array(r.data) : new Uint8Array(r.size),
    }));
  }

  /**
   * Locate the region containing [address, address + numBytes).
   * Returns [region, offset] or throws if unmapped.
   */
  private findRegion(address: number, numBytes: number): [InternalRegion, number] {
    const addr = address >>> 0;
    const end = addr + numBytes;
    for (const r of this.regions) {
      const regionEnd = r.base + r.size;
      if (addr >= r.base && end <= regionEnd) {
        return [r, addr - r.base];
      }
    }
    throw new RangeError(
      `SparseMemory: unmapped address 0x${(addr >>> 0).toString(16).padStart(8, "0")} (accessing ${numBytes} bytes)`
    );
  }

  /** Read a single byte from the sparse address space. */
  readByte(address: number): number {
    const [region, offset] = this.findRegion(address, 1);
    return region.data[offset];
  }

  /**
   * Write a single byte to the sparse address space.
   * Writes to read-only regions are silently ignored.
   */
  writeByte(address: number, value: number): void {
    const [region, offset] = this.findRegion(address, 1);
    if (region.readOnly) return;
    region.data[offset] = value & 0xff;
  }

  /**
   * Read a 32-bit word (4 bytes) from the sparse address space,
   * little-endian byte order.
   */
  readWord(address: number): number {
    const [region, offset] = this.findRegion(address, 4);
    return (
      (region.data[offset] |
        (region.data[offset + 1] << 8) |
        (region.data[offset + 2] << 16) |
        (region.data[offset + 3] << 24)) >>>
      0
    );
  }

  /**
   * Write a 32-bit word (4 bytes) to the sparse address space,
   * little-endian byte order. Writes to read-only regions are silently ignored.
   */
  writeWord(address: number, value: number): void {
    const [region, offset] = this.findRegion(address, 4);
    if (region.readOnly) return;
    const masked = (value & 0xffffffff) >>> 0;
    region.data[offset] = masked & 0xff;
    region.data[offset + 1] = (masked >>> 8) & 0xff;
    region.data[offset + 2] = (masked >>> 16) & 0xff;
    region.data[offset + 3] = (masked >>> 24) & 0xff;
  }

  /**
   * Copy bytes into the sparse address space starting at the given address.
   * Bypasses the readOnly check -- used for initial loading of ROM contents.
   */
  loadBytes(address: number, data: number[] | Uint8Array): void {
    const [region, offset] = this.findRegion(address, data.length);
    for (let i = 0; i < data.length; i++) {
      region.data[offset + i] = data[i];
    }
  }

  /**
   * Return a copy of bytes from the sparse address space.
   * The entire range must fall within a single region.
   */
  dump(start: number, length: number): number[] {
    const [region, offset] = this.findRegion(start, length);
    return Array.from(region.data.slice(offset, offset + length));
  }

  /** Return the number of mapped regions. */
  regionCount(): number {
    return this.regions.length;
  }
}
