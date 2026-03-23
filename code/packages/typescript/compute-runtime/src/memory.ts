/**
 * Memory management -- typed allocations, mapping, staging.
 *
 * === Memory Types on a GPU ===
 *
 * Unlike a CPU where all RAM is equally accessible, GPUs have distinct memory
 * pools with different performance characteristics:
 *
 *     Discrete GPU (NVIDIA, AMD):
 *       CPU side (system RAM)              GPU side (VRAM)
 *       +------------------+               +------------------+
 *       |   HOST_VISIBLE   |<---- PCIe --->|   DEVICE_LOCAL   |
 *       |   HOST_COHERENT  |   ~32 GB/s    |   (HBM / GDDR6)  |
 *       |   (staging pool) |               |   1-3 TB/s        |
 *       +------------------+               +------------------+
 *
 *     Unified Memory (Apple M-series):
 *       +------------------------------------------------------+
 *       |        DEVICE_LOCAL + HOST_VISIBLE + HOST_COHERENT    |
 *       |        (shared physical RAM)                          |
 *       |        Both CPU and GPU see the same bytes            |
 *       +------------------------------------------------------+
 *
 * === The Staging Buffer Pattern ===
 *
 * On discrete GPUs, the standard way to get data onto the GPU is:
 *   1. Allocate a HOST_VISIBLE staging buffer
 *   2. Map it, write your data, unmap it
 *   3. Record a cmdCopyBuffer from staging -> DEVICE_LOCAL
 *   4. Submit and wait
 *
 * On unified memory (Apple), you skip all of this -- allocate DEVICE_LOCAL +
 * HOST_VISIBLE, write directly, and the GPU sees it immediately.
 */

import type { AcceleratorDevice } from "@coding-adventures/device-simulator";

import {
  type MemoryProperties,
  type RuntimeStats,
  MemoryType,
  BufferUsage,
  RuntimeEventType,
  hasMemoryType,
  makeRuntimeTrace,
} from "./protocols.js";

// =========================================================================
// Buffer -- a typed allocation on the device
// =========================================================================

/**
 * A memory allocation on the device.
 *
 * === Buffer Lifecycle ===
 *
 *     allocate() -> Buffer (with deviceAddress)
 *     map()      -> MappedMemory (CPU can read/write)
 *     unmap()    -> buffer is GPU-only again
 *     free()     -> memory returned to pool
 */
export interface Buffer {
  readonly bufferId: number;
  readonly size: number;
  readonly memoryType: number; // Combination of MemoryType flags
  readonly usage: number; // Combination of BufferUsage flags
  deviceAddress: number;
  mapped: boolean;
  freed: boolean;
}

/** Create a Buffer with defaults. */
export function makeBuffer(
  partial: Partial<Buffer> & { bufferId: number; size: number; memoryType: number; usage: number },
): Buffer {
  return {
    deviceAddress: 0,
    mapped: false,
    freed: false,
    ...partial,
  };
}

// =========================================================================
// MappedMemory -- CPU-accessible view of a buffer
// =========================================================================

/**
 * CPU-accessible view of a mapped GPU buffer.
 *
 * Mapping makes device memory accessible to the CPU. After mapping,
 * you can read() and write() bytes. After unmap(), the CPU can no
 * longer access this memory.
 */
export class MappedMemory {
  private readonly _buffer: Buffer;
  private readonly _data: Uint8Array;
  private _dirty: boolean;

  constructor(buffer: Buffer, data: Uint8Array) {
    this._buffer = buffer;
    this._data = data;
    this._dirty = false;
  }

  /** The buffer this mapping refers to. */
  get buffer(): Buffer {
    return this._buffer;
  }

  /** Size of the mapped region. */
  get size(): number {
    return this._data.length;
  }

  /** Whether any writes have been made since mapping. */
  get dirty(): boolean {
    return this._dirty;
  }

  /**
   * Read bytes from the mapped buffer.
   *
   * @param offset - Byte offset from start of buffer.
   * @param size - Number of bytes to read.
   * @returns The requested bytes.
   * @throws Error if offset + size exceeds buffer size.
   */
  read(offset: number, size: number): Uint8Array {
    if (offset + size > this._data.length) {
      throw new Error(
        `Read out of bounds: offset=${offset}, size=${size}, ` +
        `buffer_size=${this._data.length}`,
      );
    }
    return new Uint8Array(this._data.slice(offset, offset + size));
  }

  /**
   * Write bytes to the mapped buffer.
   *
   * @param offset - Byte offset from start of buffer.
   * @param data - Bytes to write.
   * @throws Error if offset + data.length exceeds buffer size.
   */
  write(offset: number, data: Uint8Array): void {
    if (offset + data.length > this._data.length) {
      throw new Error(
        `Write out of bounds: offset=${offset}, data_size=${data.length}, ` +
        `buffer_size=${this._data.length}`,
      );
    }
    this._data.set(data, offset);
    this._dirty = true;
  }

  /** Get the full contents of the mapped buffer. */
  getData(): Uint8Array {
    return new Uint8Array(this._data);
  }
}

// =========================================================================
// MemoryManager -- allocates, maps, frees device memory
// =========================================================================

/**
 * Manages typed memory allocations on a device.
 *
 * The MemoryManager wraps Layer 6's raw malloc/free with type information.
 * Each allocation is tagged with a MemoryType and BufferUsage, which the
 * runtime uses for validation and optimization.
 */
export class MemoryManager {
  private readonly _device: AcceleratorDevice;
  private readonly _properties: MemoryProperties;
  private readonly _stats: RuntimeStats;
  private readonly _buffers: Map<number, Buffer>;
  private readonly _bufferData: Map<number, Uint8Array>;
  private _nextId: number;
  private _currentBytes: number;

  constructor(
    device: AcceleratorDevice,
    memoryProperties: MemoryProperties,
    stats: RuntimeStats,
  ) {
    this._device = device;
    this._properties = memoryProperties;
    this._stats = stats;
    this._buffers = new Map();
    this._bufferData = new Map();
    this._nextId = 0;
    this._currentBytes = 0;
  }

  /** Memory properties of the underlying device. */
  get memoryProperties(): MemoryProperties {
    return this._properties;
  }

  /**
   * Allocate a buffer on the device.
   *
   * @param size - Number of bytes to allocate.
   * @param memoryType - Where to allocate (DEVICE_LOCAL, HOST_VISIBLE, etc.).
   * @param usage - How the buffer will be used (STORAGE, TRANSFER_SRC, etc.).
   * @returns A Buffer with a valid deviceAddress.
   * @throws Error if size <= 0.
   */
  allocate(
    size: number,
    memoryType: number,
    usage: number = BufferUsage.STORAGE,
  ): Buffer {
    if (size <= 0) {
      throw new Error(`Allocation size must be positive, got ${size}`);
    }

    const deviceAddress = this._device.malloc(size);
    const bufId = this._nextId++;

    const buf: Buffer = {
      bufferId: bufId,
      size,
      memoryType,
      usage,
      deviceAddress,
      mapped: false,
      freed: false,
    };

    this._buffers.set(bufId, buf);
    this._bufferData.set(bufId, new Uint8Array(size));

    // Track stats
    this._currentBytes += size;
    this._stats.totalAllocatedBytes += size;
    this._stats.totalAllocations += 1;
    if (this._currentBytes > this._stats.peakAllocatedBytes) {
      this._stats.peakAllocatedBytes = this._currentBytes;
    }

    this._stats.traces.push(
      makeRuntimeTrace({
        eventType: RuntimeEventType.MEMORY_ALLOC,
        description: `Allocated ${size} bytes (buf#${bufId}, memType=${memoryType})`,
      }),
    );

    return buf;
  }

  /**
   * Free a device memory allocation.
   *
   * @throws Error if buffer is already freed, not found, or still mapped.
   */
  free(buffer: Buffer): void {
    if (buffer.freed) {
      throw new Error(`Buffer ${buffer.bufferId} already freed`);
    }
    if (!this._buffers.has(buffer.bufferId)) {
      throw new Error(`Buffer ${buffer.bufferId} not found`);
    }
    if (buffer.mapped) {
      throw new Error(
        `Buffer ${buffer.bufferId} is still mapped — unmap before freeing`,
      );
    }

    this._device.free(buffer.deviceAddress);
    buffer.freed = true;
    this._currentBytes -= buffer.size;
    this._buffers.delete(buffer.bufferId);
    this._bufferData.delete(buffer.bufferId);

    this._stats.totalFrees += 1;
    this._stats.traces.push(
      makeRuntimeTrace({
        eventType: RuntimeEventType.MEMORY_FREE,
        description: `Freed buf#${buffer.bufferId} (${buffer.size} bytes)`,
      }),
    );
  }

  /**
   * Map a buffer for CPU access.
   *
   * Only HOST_VISIBLE buffers can be mapped. On unified memory devices,
   * all buffers are HOST_VISIBLE so everything can be mapped.
   *
   * @throws Error if buffer is not HOST_VISIBLE, already mapped, or freed.
   */
  map(buffer: Buffer): MappedMemory {
    if (buffer.freed) {
      throw new Error(`Cannot map freed buffer ${buffer.bufferId}`);
    }
    if (buffer.mapped) {
      throw new Error(`Buffer ${buffer.bufferId} is already mapped`);
    }
    if (!hasMemoryType(buffer.memoryType, MemoryType.HOST_VISIBLE)) {
      throw new Error(
        `Cannot map buffer ${buffer.bufferId}: not HOST_VISIBLE ` +
        `(type=${buffer.memoryType})`,
      );
    }

    buffer.mapped = true;
    this._stats.totalMaps += 1;

    this._stats.traces.push(
      makeRuntimeTrace({
        eventType: RuntimeEventType.MEMORY_MAP,
        description: `Mapped buf#${buffer.bufferId}`,
      }),
    );

    return new MappedMemory(buffer, this._bufferData.get(buffer.bufferId)!);
  }

  /**
   * Unmap a buffer, ending CPU access.
   *
   * If HOST_COHERENT, data is automatically synced to the device.
   *
   * @throws Error if buffer is not currently mapped.
   */
  unmap(buffer: Buffer): void {
    if (!buffer.mapped) {
      throw new Error(`Buffer ${buffer.bufferId} is not mapped`);
    }

    // If HOST_COHERENT, automatically sync to device
    if (hasMemoryType(buffer.memoryType, MemoryType.HOST_COHERENT)) {
      const data = this._bufferData.get(buffer.bufferId)!;
      this._device.memcpyHostToDevice(buffer.deviceAddress, data);
    }

    buffer.mapped = false;
  }

  /**
   * Flush CPU writes to make them visible to GPU.
   *
   * Only needed for HOST_VISIBLE buffers without HOST_COHERENT.
   */
  flush(buffer: Buffer, offset = 0, size = 0): void {
    if (buffer.freed) {
      throw new Error(`Cannot flush freed buffer ${buffer.bufferId}`);
    }
    const actualSize = size > 0 ? size : buffer.size;
    const data = this._bufferData.get(buffer.bufferId)!;
    const slice = data.slice(offset, offset + actualSize);
    this._device.memcpyHostToDevice(buffer.deviceAddress + offset, slice);
  }

  /**
   * Invalidate CPU cache so GPU writes become visible to CPU.
   */
  invalidate(buffer: Buffer, offset = 0, size = 0): void {
    if (buffer.freed) {
      throw new Error(`Cannot invalidate freed buffer ${buffer.bufferId}`);
    }
    const actualSize = size > 0 ? size : buffer.size;
    const [data] = this._device.memcpyDeviceToHost(
      buffer.deviceAddress + offset,
      actualSize,
    );
    const bufData = this._bufferData.get(buffer.bufferId)!;
    bufData.set(data, offset);
  }

  /**
   * Look up a buffer by ID.
   *
   * @throws Error if buffer not found.
   */
  getBuffer(bufferId: number): Buffer {
    const buf = this._buffers.get(bufferId);
    if (!buf) {
      throw new Error(`Buffer ${bufferId} not found`);
    }
    return buf;
  }

  /** Number of currently allocated buffers. */
  get allocatedBufferCount(): number {
    return this._buffers.size;
  }

  /** Current total bytes allocated. */
  get currentAllocatedBytes(): number {
    return this._currentBytes;
  }

  /** Internal: get raw data for a buffer. */
  getBufferData(bufferId: number): Uint8Array {
    return this._bufferData.get(bufferId)!;
  }

  /** Internal: push buffer data to device. Returns cycles consumed. */
  syncBufferToDevice(buffer: Buffer): number {
    const data = this._bufferData.get(buffer.bufferId)!;
    return this._device.memcpyHostToDevice(buffer.deviceAddress, data);
  }

  /** Internal: pull buffer data from device. Returns cycles consumed. */
  syncBufferFromDevice(buffer: Buffer): number {
    const [data, cycles] = this._device.memcpyDeviceToHost(
      buffer.deviceAddress,
      buffer.size,
    );
    this._bufferData.get(buffer.bufferId)!.set(data);
    return cycles;
  }
}
