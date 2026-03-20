/**
 * CommandQueue -- FIFO submission of command buffers to a device.
 *
 * === How Submission Works ===
 *
 * When you submit command buffers to a queue, the runtime processes them
 * sequentially, executing each recorded command against the Layer 6 device:
 *
 *     queue.submit([cb1, cb2], { fence })
 *         |
 *         +-- Execute cb1's commands:
 *         |   +-- bind_pipeline -> set current pipeline
 *         |   +-- dispatch(4, 1, 1) -> device.launchKernel() + device.run()
 *         |
 *         +-- Execute cb2's commands: ...
 *         |
 *         +-- Signal semaphores (if any)
 *         +-- Signal fence (if any)
 *
 * === Multiple Queues ===
 *
 * A device can have multiple queues. Queues of different types (compute,
 * transfer) can execute in parallel. Queues of the same type execute
 * sequentially within that queue.
 */

import type { AcceleratorDevice } from "@coding-adventures/device-simulator";
import { makeKernelDescriptor } from "@coding-adventures/device-simulator";
import type { Instruction } from "@coding-adventures/gpu-core";

import { CommandBuffer } from "./command-buffer.js";
import type { MemoryManager } from "./memory.js";
import type { DescriptorSet, Pipeline } from "./pipeline.js";
import type { Fence, Semaphore } from "./sync.js";
import {
  CommandBufferState,
  QueueType,
  RuntimeEventType,
  type RecordedCommand,
  type RuntimeStats,
  type RuntimeTrace,
  makeRuntimeTrace,
} from "./protocols.js";

export class CommandQueue {
  private readonly _queueType: QueueType;
  private readonly _queueIndex: number;
  private readonly _device: AcceleratorDevice;
  private readonly _memoryManager: MemoryManager;
  private readonly _stats: RuntimeStats;
  private _totalCycles: number;

  // Execution state
  private _currentPipeline: Pipeline | null;
  private _currentDescriptorSet: DescriptorSet | null;
  private _currentPushConstants: Uint8Array;

  constructor(
    queueType: QueueType,
    queueIndex: number,
    device: AcceleratorDevice,
    memoryManager: MemoryManager,
    stats: RuntimeStats,
  ) {
    this._queueType = queueType;
    this._queueIndex = queueIndex;
    this._device = device;
    this._memoryManager = memoryManager;
    this._stats = stats;
    this._totalCycles = 0;
    this._currentPipeline = null;
    this._currentDescriptorSet = null;
    this._currentPushConstants = new Uint8Array(0);
  }

  /** What kind of work this queue handles. */
  get queueType(): QueueType {
    return this._queueType;
  }

  /** Index within queues of the same type. */
  get queueIndex(): number {
    return this._queueIndex;
  }

  /** Total device cycles consumed by this queue. */
  get totalCycles(): number {
    return this._totalCycles;
  }

  /**
   * Submit command buffers for execution.
   *
   * === Submission Flow ===
   *
   * 1. Wait for all waitSemaphores to be signaled
   * 2. Execute each command buffer sequentially
   * 3. Signal all signalSemaphores
   * 4. Signal the fence (if provided)
   *
   * @returns List of RuntimeTrace events generated during execution.
   * @throws Error if any CB is not in RECORDED state.
   * @throws Error if a wait semaphore is not signaled.
   */
  submit(
    commandBuffers: CommandBuffer[],
    options: {
      waitSemaphores?: Semaphore[];
      signalSemaphores?: Semaphore[];
      fence?: Fence;
    } = {},
  ): RuntimeTrace[] {
    const traces: RuntimeTrace[] = [];
    const waitSems = options.waitSemaphores ?? [];
    const signalSems = options.signalSemaphores ?? [];
    const fence = options.fence ?? null;

    // Validate CB states
    for (const cb of commandBuffers) {
      if (cb.state !== CommandBufferState.RECORDED) {
        throw new Error(
          `CB#${cb.commandBufferId} is in state ${cb.state}, ` +
          `expected RECORDED`,
        );
      }
    }

    // Wait on semaphores
    for (const sem of waitSems) {
      if (!sem.signaled) {
        throw new Error(
          `Semaphore ${sem.semaphoreId} is not signaled — ` +
          `cannot proceed (possible deadlock)`,
        );
      }
      traces.push(
        makeRuntimeTrace({
          timestampCycles: this._totalCycles,
          eventType: RuntimeEventType.SEMAPHORE_WAIT,
          description: `Wait on semaphore S${sem.semaphoreId}`,
          queueType: this._queueType,
          semaphoreId: sem.semaphoreId,
        }),
      );
      sem.reset(); // Consume the semaphore
    }

    // Log submission
    this._stats.totalSubmissions += 1;
    this._stats.totalCommandBuffers += commandBuffers.length;

    const cbIds = commandBuffers.map((cb) => cb.commandBufferId);
    traces.push(
      makeRuntimeTrace({
        timestampCycles: this._totalCycles,
        eventType: RuntimeEventType.SUBMIT,
        description: `Submit CB [${cbIds.join(",")}] to ${this._queueType} queue`,
        queueType: this._queueType,
      }),
    );

    // Execute each command buffer
    for (const cb of commandBuffers) {
      cb._markPending();
      const cbTraces = this._executeCommandBuffer(cb);
      traces.push(...cbTraces);
      cb._markComplete();
    }

    // Signal semaphores
    for (const sem of signalSems) {
      sem.signal();
      this._stats.totalSemaphoreSignals += 1;
      traces.push(
        makeRuntimeTrace({
          timestampCycles: this._totalCycles,
          eventType: RuntimeEventType.SEMAPHORE_SIGNAL,
          description: `Signal semaphore S${sem.semaphoreId}`,
          queueType: this._queueType,
          semaphoreId: sem.semaphoreId,
        }),
      );
    }

    // Signal fence
    if (fence !== null) {
      fence.signal();
      traces.push(
        makeRuntimeTrace({
          timestampCycles: this._totalCycles,
          eventType: RuntimeEventType.FENCE_SIGNAL,
          description: `Signal fence F${fence.fenceId}`,
          queueType: this._queueType,
          fenceId: fence.fenceId,
        }),
      );
    }

    // Update stats
    this._stats.totalDeviceCycles = this._totalCycles;
    const total = this._stats.totalDeviceCycles + this._stats.totalIdleCycles;
    if (total > 0) {
      this._stats.gpuUtilization = this._stats.totalDeviceCycles / total;
    }
    this._stats.traces.push(...traces);

    return traces;
  }

  /**
   * Block until this queue has no pending work.
   *
   * In our synchronous simulation, submit() always runs to completion,
   * so this is a no-op.
   */
  waitIdle(): void {
    // No-op in synchronous simulation
  }

  private _executeCommandBuffer(cb: CommandBuffer): RuntimeTrace[] {
    const traces: RuntimeTrace[] = [];

    // Replay the CB's bind state
    this._currentPipeline = cb.boundPipeline;
    this._currentDescriptorSet = cb.boundDescriptorSet;

    traces.push(
      makeRuntimeTrace({
        timestampCycles: this._totalCycles,
        eventType: RuntimeEventType.BEGIN_EXECUTION,
        description: `Begin CB#${cb.commandBufferId}`,
        queueType: this._queueType,
        commandBufferId: cb.commandBufferId,
      }),
    );

    for (const cmd of cb.commands) {
      const cmdTraces = this._executeCommand(cmd);
      traces.push(...cmdTraces);
    }

    traces.push(
      makeRuntimeTrace({
        timestampCycles: this._totalCycles,
        eventType: RuntimeEventType.END_EXECUTION,
        description: `End CB#${cb.commandBufferId}`,
        queueType: this._queueType,
        commandBufferId: cb.commandBufferId,
      }),
    );

    return traces;
  }

  private _executeCommand(cmd: RecordedCommand): RuntimeTrace[] {
    const handlers: Record<string, (args: Record<string, unknown>) => RuntimeTrace[]> = {
      bind_pipeline: () => [],
      bind_descriptor_set: () => [],
      push_constants: () => [],
      dispatch: (args) => this._execDispatch(args),
      dispatch_indirect: (args) => this._execDispatchIndirect(args),
      copy_buffer: (args) => this._execCopyBuffer(args),
      fill_buffer: (args) => this._execFillBuffer(args),
      update_buffer: (args) => this._execUpdateBuffer(args),
      pipeline_barrier: (args) => this._execPipelineBarrier(args),
      set_event: () => [],
      wait_event: () => [],
      reset_event: () => [],
    };

    const handler = handlers[cmd.command];
    if (!handler) {
      throw new Error(`Unknown command: ${cmd.command}`);
    }

    return handler(cmd.args);
  }

  // =================================================================
  // Command executors
  // =================================================================

  private _execDispatch(args: Record<string, unknown>): RuntimeTrace[] {
    const groupX = args.group_x as number;
    const groupY = args.group_y as number;
    const groupZ = args.group_z as number;

    const pipeline = this._currentPipeline;
    if (pipeline === null) {
      throw new Error("No pipeline bound for dispatch");
    }

    const shader = pipeline.shader;

    let kernel;
    if (shader.isGpuStyle) {
      kernel = makeKernelDescriptor({
        name: `dispatch_${groupX}x${groupY}x${groupZ}`,
        program: shader.code as Instruction[],
        gridDim: [groupX, groupY, groupZ],
        blockDim: [...shader.localSize] as [number, number, number],
      });
    } else {
      // Dataflow-style dispatch
      kernel = makeKernelDescriptor({
        name: `op_${shader.operation}`,
        operation: shader.operation,
        inputData: [[1.0]],
        weightData: [[1.0]],
      });
    }

    this._device.launchKernel(kernel);
    const deviceTraces = this._device.run(10000);
    const cycles = deviceTraces.length;
    this._totalCycles += cycles;

    this._stats.totalDispatches += 1;

    return [
      makeRuntimeTrace({
        timestampCycles: this._totalCycles,
        eventType: RuntimeEventType.END_EXECUTION,
        description: `Dispatch (${groupX},${groupY},${groupZ}) completed in ${cycles} cycles`,
        queueType: this._queueType,
        deviceTraces: deviceTraces,
      }),
    ];
  }

  private _execDispatchIndirect(args: Record<string, unknown>): RuntimeTrace[] {
    const bufferId = args.buffer_id as number;
    const offset = args.offset as number;

    const data = this._memoryManager.getBufferData(bufferId);
    const view = new DataView(data.buffer, data.byteOffset + offset, 12);
    const groupX = view.getUint32(0, true);
    const groupY = view.getUint32(4, true);
    const groupZ = view.getUint32(8, true);

    return this._execDispatch({ group_x: groupX, group_y: groupY, group_z: groupZ });
  }

  private _execCopyBuffer(args: Record<string, unknown>): RuntimeTrace[] {
    const srcId = args.src_id as number;
    const dstId = args.dst_id as number;
    const size = args.size as number;
    const srcOffset = (args.src_offset as number) ?? 0;
    const dstOffset = (args.dst_offset as number) ?? 0;

    const srcData = this._memoryManager.getBufferData(srcId);
    const dstData = this._memoryManager.getBufferData(dstId);

    // Copy the bytes
    dstData.set(srcData.slice(srcOffset, srcOffset + size), dstOffset);

    // Also sync to device memory
    const srcBuf = this._memoryManager.getBuffer(srcId);
    const dstBuf = this._memoryManager.getBuffer(dstId);

    const [dataBytes, readCycles] = this._device.memcpyDeviceToHost(
      srcBuf.deviceAddress + srcOffset,
      size,
    );
    const writeCycles = this._device.memcpyHostToDevice(
      dstBuf.deviceAddress + dstOffset,
      dataBytes,
    );

    const cycles = readCycles + writeCycles;
    this._totalCycles += cycles;
    this._stats.totalTransfers += 1;

    return [
      makeRuntimeTrace({
        timestampCycles: this._totalCycles,
        eventType: RuntimeEventType.MEMORY_TRANSFER,
        description: `Copy ${size} bytes: buf#${srcId} -> buf#${dstId} (${cycles} cycles)`,
        queueType: this._queueType,
      }),
    ];
  }

  private _execFillBuffer(args: Record<string, unknown>): RuntimeTrace[] {
    const bufferId = args.buffer_id as number;
    const value = args.value as number;
    const offset = args.offset as number;
    const size = args.size as number;

    const bufData = this._memoryManager.getBufferData(bufferId);
    const fillByte = value & 0xff;
    for (let i = offset; i < offset + size; i++) {
      bufData[i] = fillByte;
    }

    // Sync to device
    const buf = this._memoryManager.getBuffer(bufferId);
    const fillBytes = new Uint8Array(size).fill(fillByte);
    this._device.memcpyHostToDevice(buf.deviceAddress + offset, fillBytes);

    this._stats.totalTransfers += 1;
    return [];
  }

  private _execUpdateBuffer(args: Record<string, unknown>): RuntimeTrace[] {
    const bufferId = args.buffer_id as number;
    const offset = args.offset as number;
    const data = args.data as Uint8Array;

    const bufData = this._memoryManager.getBufferData(bufferId);
    bufData.set(data, offset);

    // Sync to device
    const buf = this._memoryManager.getBuffer(bufferId);
    this._device.memcpyHostToDevice(buf.deviceAddress + offset, data);

    this._stats.totalTransfers += 1;
    return [];
  }

  private _execPipelineBarrier(args: Record<string, unknown>): RuntimeTrace[] {
    this._stats.totalBarriers += 1;
    return [
      makeRuntimeTrace({
        timestampCycles: this._totalCycles,
        eventType: RuntimeEventType.BARRIER,
        description: `Barrier: ${args.src_stage} -> ${args.dst_stage}`,
        queueType: this._queueType,
      }),
    ];
  }
}
