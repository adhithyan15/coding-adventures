/**
 * CommandBuffer -- recorded sequence of GPU commands.
 *
 * === The Record-Then-Submit Model ===
 *
 * Instead of calling GPU operations one at a time (like CUDA), Vulkan records
 * commands into a buffer and submits the whole buffer at once:
 *
 *     // Vulkan style (explicit, batched):
 *     cb.begin()                     // start recording
 *     cb.cmdCopyBuffer(...)          // just records -- doesn't execute
 *     cb.cmdDispatch(...)            // just records -- doesn't execute
 *     cb.end()                       // stop recording
 *     queue.submit([cb])             // NOW everything executes
 *
 * === Why Batch? ===
 *
 * 1. **Driver optimization** -- the driver sees all commands at once
 * 2. **Reuse** -- submit the same CB multiple times without re-recording
 * 3. **Multi-threaded recording** -- different CPU threads record different CBs
 * 4. **Validation** -- check the entire sequence before any GPU work starts
 *
 * === State Machine ===
 *
 *     INITIAL --begin()--> RECORDING --end()--> RECORDED --submit()--> PENDING
 *        ^                                                                |
 *        +-------------------- reset() <-- COMPLETE <---------------------+
 */

import type { Buffer } from "./memory.js";
import type { DescriptorSet, Pipeline } from "./pipeline.js";
import type { Event } from "./sync.js";
import {
  CommandBufferState,
  type PipelineBarrier,
  type PipelineStage,
  type RecordedCommand,
} from "./protocols.js";

export class CommandBuffer {
  private static _nextId = 0;

  private readonly _id: number;
  private _state: CommandBufferState;
  private _commands: RecordedCommand[];

  // Currently bound state (for validation)
  private _boundPipeline: Pipeline | null;
  private _boundDescriptorSet: DescriptorSet | null;
  private _pushConstants: Uint8Array;

  constructor() {
    this._id = CommandBuffer._nextId++;
    this._state = CommandBufferState.INITIAL;
    this._commands = [];
    this._boundPipeline = null;
    this._boundDescriptorSet = null;
    this._pushConstants = new Uint8Array(0);
  }

  /** Unique identifier. */
  get commandBufferId(): number {
    return this._id;
  }

  /** Current lifecycle state. */
  get state(): CommandBufferState {
    return this._state;
  }

  /** All recorded commands. */
  get commands(): RecordedCommand[] {
    return [...this._commands];
  }

  /** Currently bound pipeline (for validation). */
  get boundPipeline(): Pipeline | null {
    return this._boundPipeline;
  }

  /** Currently bound descriptor set (for validation). */
  get boundDescriptorSet(): DescriptorSet | null {
    return this._boundDescriptorSet;
  }

  // =================================================================
  // Lifecycle
  // =================================================================

  /**
   * Start recording commands.
   *
   * Transitions: INITIAL -> RECORDING, or COMPLETE -> RECORDING (reuse).
   *
   * @throws Error if not in INITIAL or COMPLETE state.
   */
  begin(): void {
    if (
      this._state !== CommandBufferState.INITIAL &&
      this._state !== CommandBufferState.COMPLETE
    ) {
      throw new Error(
        `Cannot begin recording: state is ${this._state} ` +
        `(expected INITIAL or COMPLETE)`,
      );
    }
    this._state = CommandBufferState.RECORDING;
    this._commands = [];
    this._boundPipeline = null;
    this._boundDescriptorSet = null;
    this._pushConstants = new Uint8Array(0);
  }

  /**
   * Finish recording commands.
   *
   * Transitions: RECORDING -> RECORDED.
   *
   * @throws Error if not in RECORDING state.
   */
  end(): void {
    if (this._state !== CommandBufferState.RECORDING) {
      throw new Error(
        `Cannot end recording: state is ${this._state} ` +
        `(expected RECORDING)`,
      );
    }
    this._state = CommandBufferState.RECORDED;
  }

  /**
   * Reset to INITIAL state for reuse.
   *
   * Clears all recorded commands and bound state.
   */
  reset(): void {
    this._state = CommandBufferState.INITIAL;
    this._commands = [];
    this._boundPipeline = null;
    this._boundDescriptorSet = null;
    this._pushConstants = new Uint8Array(0);
  }

  /** Internal: mark as submitted (called by CommandQueue). */
  _markPending(): void {
    this._state = CommandBufferState.PENDING;
  }

  /** Internal: mark as finished (called by CommandQueue). */
  _markComplete(): void {
    this._state = CommandBufferState.COMPLETE;
  }

  private _requireRecording(): void {
    if (this._state !== CommandBufferState.RECORDING) {
      throw new Error(
        `Cannot record command: state is ${this._state} ` +
        `(expected RECORDING)`,
      );
    }
  }

  // =================================================================
  // Compute commands
  // =================================================================

  /** Bind a compute pipeline for subsequent dispatches. */
  cmdBindPipeline(pipeline: Pipeline): void {
    this._requireRecording();
    this._boundPipeline = pipeline;
    this._commands.push({
      command: "bind_pipeline",
      args: { pipeline_id: pipeline.pipelineId },
    });
  }

  /** Bind a descriptor set (buffer assignments) for subsequent dispatches. */
  cmdBindDescriptorSet(descriptorSet: DescriptorSet): void {
    this._requireRecording();
    this._boundDescriptorSet = descriptorSet;
    this._commands.push({
      command: "bind_descriptor_set",
      args: { set_id: descriptorSet.setId },
    });
  }

  /** Set push constant data for the next dispatch. */
  cmdPushConstants(offset: number, data: Uint8Array): void {
    this._requireRecording();
    this._pushConstants = data;
    this._commands.push({
      command: "push_constants",
      args: { offset, size: data.length },
    });
  }

  /**
   * Launch a compute kernel.
   *
   * @param groupX - Workgroups in X dimension.
   * @param groupY - Workgroups in Y dimension.
   * @param groupZ - Workgroups in Z dimension.
   * @throws Error if no pipeline is bound.
   */
  cmdDispatch(groupX: number, groupY = 1, groupZ = 1): void {
    this._requireRecording();
    if (this._boundPipeline === null) {
      throw new Error("Cannot dispatch: no pipeline bound");
    }
    this._commands.push({
      command: "dispatch",
      args: { group_x: groupX, group_y: groupY, group_z: groupZ },
    });
  }

  /** Launch a compute kernel with grid dimensions from a GPU buffer. */
  cmdDispatchIndirect(buffer: Buffer, offset = 0): void {
    this._requireRecording();
    if (this._boundPipeline === null) {
      throw new Error("Cannot dispatch: no pipeline bound");
    }
    this._commands.push({
      command: "dispatch_indirect",
      args: { buffer_id: buffer.bufferId, offset },
    });
  }

  // =================================================================
  // Transfer commands
  // =================================================================

  /** Copy data between device buffers. */
  cmdCopyBuffer(
    src: Buffer,
    dst: Buffer,
    size: number,
    srcOffset = 0,
    dstOffset = 0,
  ): void {
    this._requireRecording();
    this._commands.push({
      command: "copy_buffer",
      args: {
        src_id: src.bufferId,
        dst_id: dst.bufferId,
        size,
        src_offset: srcOffset,
        dst_offset: dstOffset,
      },
    });
  }

  /** Fill a buffer with a constant byte value. */
  cmdFillBuffer(
    buffer: Buffer,
    value: number,
    offset = 0,
    size = 0,
  ): void {
    this._requireRecording();
    this._commands.push({
      command: "fill_buffer",
      args: {
        buffer_id: buffer.bufferId,
        value,
        offset,
        size: size > 0 ? size : buffer.size,
      },
    });
  }

  /** Write small data inline from CPU to device buffer. */
  cmdUpdateBuffer(buffer: Buffer, offset: number, data: Uint8Array): void {
    this._requireRecording();
    this._commands.push({
      command: "update_buffer",
      args: {
        buffer_id: buffer.bufferId,
        offset,
        data,
      },
    });
  }

  // =================================================================
  // Synchronization commands
  // =================================================================

  /** Insert an execution + memory barrier. */
  cmdPipelineBarrier(barrier: PipelineBarrier): void {
    this._requireRecording();
    this._commands.push({
      command: "pipeline_barrier",
      args: {
        src_stage: barrier.srcStage,
        dst_stage: barrier.dstStage,
        memory_barrier_count: barrier.memoryBarriers.length,
        buffer_barrier_count: barrier.bufferBarriers.length,
      },
    });
  }

  /** Signal an event from the GPU. */
  cmdSetEvent(event: Event, stage: PipelineStage): void {
    this._requireRecording();
    this._commands.push({
      command: "set_event",
      args: { event_id: event.eventId, stage },
    });
  }

  /** Wait for an event before proceeding. */
  cmdWaitEvent(
    event: Event,
    srcStage: PipelineStage,
    dstStage: PipelineStage,
  ): void {
    this._requireRecording();
    this._commands.push({
      command: "wait_event",
      args: { event_id: event.eventId, src_stage: srcStage, dst_stage: dstStage },
    });
  }

  /** Reset an event from the GPU side. */
  cmdResetEvent(event: Event, stage: PipelineStage): void {
    this._requireRecording();
    this._commands.push({
      command: "reset_event",
      args: { event_id: event.eventId, stage },
    });
  }
}
