/**
 * Global Memory -- device-wide VRAM / HBM simulator.
 *
 * === What is Global Memory? ===
 *
 * Global memory is the large, high-bandwidth memory that serves the entire
 * accelerator device. Every compute unit can read from and write to global
 * memory, making it the shared data store for all parallel computation.
 *
 *     NVIDIA: HBM3 (High Bandwidth Memory) -- 80 GB on H100
 *     AMD:    GDDR6 -- 24 GB on RX 7900 XTX
 *     Google: HBM2e -- 32 GB per TPU v4 chip
 *     Intel:  GDDR6 -- 16 GB on Arc A770
 *     Apple:  Unified LPDDR5 -- shared with CPU/GPU, up to 192 GB
 *
 * === Key Properties ===
 *
 * 1. **High bandwidth**: 1-3 TB/s. Much faster than CPU memory (~50 GB/s).
 * 2. **High latency**: ~400-800 cycles to service a request.
 * 3. **Shared**: ALL compute units on the device share global memory.
 * 4. **Coalescing**: The memory controller can merge multiple thread
 *    requests into fewer wide transactions if addresses are contiguous.
 * 5. **Partitioned**: Memory is physically split across channels/stacks.
 *
 * === Sparse Memory Representation ===
 *
 * Real devices have 16-80 GB of VRAM. We obviously can't allocate that in
 * a simulator. Instead, we use a sparse Map: only addresses that have
 * been written to consume actual memory. A read to an uninitialized address
 * returns zeros (matching real hardware behavior after cudaMemset).
 */

import {
  type GlobalMemoryStats,
  type MemoryTransaction,
  makeGlobalMemoryStats,
  updateEfficiency,
} from "./protocols.js";

/**
 * Global memory implementation with coalescing and partitioning.
 *
 * This models the device-wide memory (VRAM/HBM) that all compute units
 * share. It tracks access patterns, coalescing efficiency, and partition
 * conflicts to help identify memory bottlenecks.
 *
 * Usage:
 *     const mem = new SimpleGlobalMemory({ capacity: 1024 * 1024, channels: 4 });
 *     const addr = mem.allocate(256);
 *     mem.copyFromHost(addr, new Uint8Array(256));
 *     mem.write(addr, new Uint8Array([0x41, 0x42, 0x43, 0x44]));
 *     const data = mem.read(addr, 4);
 */
export class SimpleGlobalMemory {
  private readonly _capacity: number;
  private readonly _bandwidth: number;
  private readonly _latency: number;
  private readonly _channels: number;
  private readonly _transactionSize: number;
  private readonly _hostBandwidth: number;
  private readonly _hostLatency: number;
  private readonly _unified: boolean;

  /** Sparse storage -- only written addresses consume memory. */
  private _data: Map<number, number>;

  /** Simple bump allocator. */
  private _nextFree: number;
  private _allocations: Map<number, number>; // startAddr -> size

  /** Statistics. */
  private _stats: GlobalMemoryStats;

  constructor(opts: {
    capacity?: number;
    bandwidth?: number;
    latency?: number;
    channels?: number;
    transactionSize?: number;
    hostBandwidth?: number;
    hostLatency?: number;
    unified?: boolean;
  } = {}) {
    this._capacity = opts.capacity ?? 16 * 1024 * 1024;
    this._bandwidth = opts.bandwidth ?? 1000.0;
    this._latency = opts.latency ?? 400;
    this._channels = opts.channels ?? 8;
    this._transactionSize = opts.transactionSize ?? 128;
    this._hostBandwidth = opts.hostBandwidth ?? 64.0;
    this._hostLatency = opts.hostLatency ?? 1000;
    this._unified = opts.unified ?? false;

    this._data = new Map();
    this._nextFree = 0;
    this._allocations = new Map();
    this._stats = makeGlobalMemoryStats();
  }

  // --- Properties ---

  /** Total memory in bytes. */
  get capacity(): number {
    return this._capacity;
  }

  /** Peak bandwidth in bytes per cycle. */
  get bandwidth(): number {
    return this._bandwidth;
  }

  /** Access statistics (recalculates efficiency). */
  get stats(): GlobalMemoryStats {
    updateEfficiency(this._stats);
    return this._stats;
  }

  // --- Allocation ---

  /**
   * Allocate memory. Returns the start address.
   *
   * Uses a simple bump allocator with alignment. Like cudaMalloc,
   * this returns a device pointer that can be passed to kernels.
   *
   * @param size      Number of bytes to allocate.
   * @param alignment Alignment in bytes (default 256 for cache lines).
   * @throws Error if not enough memory remains.
   */
  allocate(size: number, alignment: number = 256): number {
    // Align the next free pointer
    const aligned = Math.ceil(this._nextFree / alignment) * alignment;

    if (aligned + size > this._capacity) {
      throw new Error(
        `Out of device memory: requested ${size} bytes at ${aligned}, capacity ${this._capacity}`,
      );
    }

    this._allocations.set(aligned, size);
    this._nextFree = aligned + size;
    return aligned;
  }

  /**
   * Free a previous allocation.
   *
   * Note: our simple bump allocator doesn't reclaim memory. But for
   * simulation purposes, this tracks that the free was called.
   */
  free(address: number): void {
    this._allocations.delete(address);
  }

  // --- Read / Write ---

  /**
   * Read bytes from global memory.
   *
   * Uninitialized addresses return zeros (like cudaMemset(0)).
   *
   * @throws RangeError if address is out of range.
   */
  read(address: number, size: number): Uint8Array {
    if (address < 0 || address + size > this._capacity) {
      throw new RangeError(
        `Address ${address}+${size} out of range [0, ${this._capacity})`,
      );
    }

    this._stats.totalReads += 1;
    this._stats.bytesTransferred += size;

    const result = new Uint8Array(size);
    for (let i = 0; i < size; i++) {
      result[i] = this._data.get(address + i) ?? 0;
    }
    return result;
  }

  /**
   * Write bytes to global memory.
   *
   * @throws RangeError if address is out of range.
   */
  write(address: number, data: Uint8Array): void {
    const size = data.length;
    if (address < 0 || address + size > this._capacity) {
      throw new RangeError(
        `Address ${address}+${size} out of range [0, ${this._capacity})`,
      );
    }

    this._stats.totalWrites += 1;
    this._stats.bytesTransferred += size;

    for (let i = 0; i < size; i++) {
      this._data.set(address + i, data[i]);
    }
  }

  // --- Host transfers ---

  /**
   * Copy from host (CPU) to device memory.
   *
   * Like cudaMemcpy(dst, src, size, cudaMemcpyHostToDevice).
   *
   * For unified memory (Apple), this is zero-cost -- no actual data
   * movement, just a page table remap.
   *
   * @returns Number of cycles consumed by the transfer.
   */
  copyFromHost(
    dstAddr: number,
    data: Uint8Array,
    hostBandwidth?: number,
  ): number {
    this.write(dstAddr, data);

    const bw = hostBandwidth ?? this._hostBandwidth;
    const size = data.length;
    this._stats.hostToDeviceBytes += size;

    if (this._unified) {
      // Unified memory: zero-copy
      return 0;
    }

    // Transfer time = latency + size / bandwidth
    const cycles = bw > 0 ? this._hostLatency + Math.floor(size / bw) : 0;
    this._stats.hostTransferCycles += cycles;
    return cycles;
  }

  /**
   * Copy from device memory to host (CPU).
   *
   * Like cudaMemcpy(dst, src, size, cudaMemcpyDeviceToHost).
   *
   * @returns Tuple of [data, cyclesConsumed].
   */
  copyToHost(
    srcAddr: number,
    size: number,
    hostBandwidth?: number,
  ): [Uint8Array, number] {
    const data = this.read(srcAddr, size);

    const bw = hostBandwidth ?? this._hostBandwidth;
    this._stats.deviceToHostBytes += size;

    if (this._unified) {
      return [data, 0];
    }

    const cycles = bw > 0 ? this._hostLatency + Math.floor(size / bw) : 0;
    this._stats.hostTransferCycles += cycles;
    return [data, cycles];
  }

  // --- Coalescing ---

  /**
   * Given per-thread addresses, merge into coalesced transactions.
   *
   * === Coalescing Algorithm ===
   *
   * 1. For each thread's address, compute which transaction-sized
   *    aligned region it falls in.
   * 2. Group threads by aligned region.
   * 3. Each group becomes one transaction.
   *
   * The fewer transactions, the better -- ideal is 1 transaction
   * for 32 threads (128 bytes of contiguous access).
   */
  coalesce(addresses: number[], size: number = 4): MemoryTransaction[] {
    const ts = this._transactionSize;

    // Group threads by aligned transaction address
    const groups = new Map<number, number>(); // aligned_addr -> thread_mask
    for (let threadIdx = 0; threadIdx < addresses.length; threadIdx++) {
      const addr = addresses[threadIdx];
      const aligned = Math.floor(addr / ts) * ts;
      const existing = groups.get(aligned) ?? 0;
      groups.set(aligned, existing | (1 << threadIdx));
    }

    // Sort by address and create transactions
    const sortedEntries = [...groups.entries()].sort((a, b) => a[0] - b[0]);
    const transactions: MemoryTransaction[] = sortedEntries.map(
      ([aligned, mask]) => ({
        address: aligned,
        size: ts,
        threadMask: mask,
      }),
    );

    // Track stats
    this._stats.totalRequests += addresses.length;
    this._stats.totalTransactions += transactions.length;

    // Check partition conflicts
    const channelsHit = new Map<number, number>();
    for (const txn of transactions) {
      const channel = Math.floor(txn.address / ts) % this._channels;
      channelsHit.set(channel, (channelsHit.get(channel) ?? 0) + 1);
    }
    for (const count of channelsHit.values()) {
      if (count > 1) {
        this._stats.partitionConflicts += count - 1;
      }
    }

    return transactions;
  }

  // --- Reset ---

  /** Clear all data, allocations, and statistics. */
  reset(): void {
    this._data.clear();
    this._nextFree = 0;
    this._allocations.clear();
    this._stats = makeGlobalMemoryStats();
  }
}
