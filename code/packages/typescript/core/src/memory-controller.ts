/**
 * MemoryController -- serializes memory requests from multiple cores.
 *
 * In a multi-core system, multiple cores may request memory access in the
 * same clock cycle. The memory controller queues and serializes them.
 *
 * The memory is a flat Uint8Array. Word reads/writes use little-endian
 * byte ordering, matching modern ARM and x86 architectures.
 */

// =========================================================================
// MemoryReadResult
// =========================================================================

/** A completed read -- data delivered to a requester. */
export interface MemoryReadResult {
  requesterID: number;
  address: number;
  data: Uint8Array;
}

// =========================================================================
// Internal request types
// =========================================================================

interface MemoryRequest {
  address: number;
  numBytes: number;
  requesterID: number;
  cyclesLeft: number;
}

interface MemoryWriteRequest {
  address: number;
  data: Uint8Array;
  requesterID: number;
  cyclesLeft: number;
}

// =========================================================================
// MemoryController
// =========================================================================

export class MemoryController {
  private _memory: Uint8Array;
  private _latency: number;
  private _pendingReads: MemoryRequest[] = [];
  private _pendingWrites: MemoryWriteRequest[] = [];

  constructor(memory: Uint8Array, latency: number) {
    this._memory = memory;
    this._latency = latency;
  }

  /** Submits a read request. Completes after `latency` cycles. */
  requestRead(address: number, numBytes: number, requesterID: number): void {
    this._pendingReads.push({
      address,
      numBytes,
      requesterID,
      cyclesLeft: this._latency,
    });
  }

  /** Submits a write request. Completes after `latency` cycles. */
  requestWrite(address: number, data: Uint8Array, requesterID: number): void {
    const dataCopy = new Uint8Array(data);
    this._pendingWrites.push({
      address,
      data: dataCopy,
      requesterID,
      cyclesLeft: this._latency,
    });
  }

  /**
   * Advances the memory controller by one cycle.
   * Returns completed read results.
   */
  tick(): MemoryReadResult[] {
    const completed: MemoryReadResult[] = [];

    // Process pending reads.
    const remainingReads: MemoryRequest[] = [];
    for (const req of this._pendingReads) {
      req.cyclesLeft--;
      if (req.cyclesLeft <= 0) {
        const data = this.readMemory(req.address, req.numBytes);
        completed.push({
          requesterID: req.requesterID,
          address: req.address,
          data,
        });
      } else {
        remainingReads.push(req);
      }
    }
    this._pendingReads = remainingReads;

    // Process pending writes.
    const remainingWrites: MemoryWriteRequest[] = [];
    for (const req of this._pendingWrites) {
      req.cyclesLeft--;
      if (req.cyclesLeft <= 0) {
        this.writeMemory(req.address, req.data);
      } else {
        remainingWrites.push(req);
      }
    }
    this._pendingWrites = remainingWrites;

    return completed;
  }

  /** Reads a 32-bit word from memory (little-endian). */
  readWord(address: number): number {
    if (address < 0 || address + 4 > this._memory.length) return 0;
    return (
      this._memory[address] |
      (this._memory[address + 1] << 8) |
      (this._memory[address + 2] << 16) |
      (this._memory[address + 3] << 24)
    );
  }

  /** Writes a 32-bit word to memory (little-endian). */
  writeWord(address: number, value: number): void {
    if (address < 0 || address + 4 > this._memory.length) return;
    this._memory[address] = value & 0xff;
    this._memory[address + 1] = (value >> 8) & 0xff;
    this._memory[address + 2] = (value >> 16) & 0xff;
    this._memory[address + 3] = (value >> 24) & 0xff;
  }

  /** Copies program bytes into memory starting at the given address. */
  loadProgram(program: Uint8Array, startAddress: number): void {
    if (startAddress < 0 || startAddress + program.length > this._memory.length) return;
    this._memory.set(program, startAddress);
  }

  /** Returns the total size of memory in bytes. */
  memorySize(): number {
    return this._memory.length;
  }

  /** Returns the number of in-flight requests. */
  pendingCount(): number {
    return this._pendingReads.length + this._pendingWrites.length;
  }

  private readMemory(address: number, numBytes: number): Uint8Array {
    if (address < 0 || address + numBytes > this._memory.length) {
      return new Uint8Array(numBytes);
    }
    return new Uint8Array(this._memory.slice(address, address + numBytes));
  }

  private writeMemory(address: number, data: Uint8Array): void {
    if (address < 0 || address + data.length > this._memory.length) return;
    this._memory.set(data, address);
  }
}
