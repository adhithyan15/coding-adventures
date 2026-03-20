/**
 * ValidationLayer -- catches GPU programming errors early.
 *
 * === What is a Validation Layer? ===
 *
 * In Vulkan, validation layers are optional middleware that check every API
 * call for errors. They're enabled during development and disabled in
 * production (for performance). Common errors they catch:
 *
 *     - Dispatching without binding a pipeline
 *     - Using a freed buffer in a descriptor set
 *     - Missing a barrier between write and read
 *     - Mapping a DEVICE_LOCAL-only buffer
 *     - Exceeding device limits
 *
 * Our validation layer checks every operation and raises clear error messages.
 */

import type { CommandBuffer } from "./command-buffer.js";
import type { Buffer } from "./memory.js";
import type { DescriptorSet, Pipeline } from "./pipeline.js";
import {
  BufferUsage,
  CommandBufferState,
  MemoryType,
  hasBufferUsage,
  hasMemoryType,
} from "./protocols.js";

/**
 * Raised when a validation check fails.
 *
 * These errors represent GPU programming mistakes -- things that would
 * cause undefined behavior or crashes on real hardware.
 */
export class ValidationError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ValidationError";
  }
}

/**
 * Validates runtime operations and raises clear error messages.
 *
 * === What It Checks ===
 *
 * 1. Command buffer state transitions
 * 2. Pipeline/descriptor binding
 * 3. Memory type compatibility
 * 4. Buffer usage flags
 * 5. Freed resource detection
 * 6. Barrier correctness
 */
export class ValidationLayer {
  private _warnings: string[];
  private _errors: string[];
  private _writtenBuffers: Set<number>;
  private _barrieredBuffers: Set<number>;

  constructor() {
    this._warnings = [];
    this._errors = [];
    this._writtenBuffers = new Set();
    this._barrieredBuffers = new Set();
  }

  /** All validation warnings issued so far. */
  get warnings(): string[] {
    return [...this._warnings];
  }

  /** All validation errors issued so far. */
  get errors(): string[] {
    return [...this._errors];
  }

  /** Clear all warnings and errors. */
  clear(): void {
    this._warnings = [];
    this._errors = [];
    this._writtenBuffers.clear();
    this._barrieredBuffers.clear();
  }

  // --- Command buffer validation ---

  /** Validate that begin() is allowed. */
  validateBegin(cb: CommandBuffer): void {
    if (
      cb.state !== CommandBufferState.INITIAL &&
      cb.state !== CommandBufferState.COMPLETE
    ) {
      throw new ValidationError(
        `Cannot begin CB#${cb.commandBufferId}: ` +
        `state is ${cb.state} (expected INITIAL or COMPLETE)`,
      );
    }
  }

  /** Validate that end() is allowed. */
  validateEnd(cb: CommandBuffer): void {
    if (cb.state !== CommandBufferState.RECORDING) {
      throw new ValidationError(
        `Cannot end CB#${cb.commandBufferId}: ` +
        `state is ${cb.state} (expected RECORDING)`,
      );
    }
  }

  /** Validate that a CB can be submitted. */
  validateSubmit(cb: CommandBuffer): void {
    if (cb.state !== CommandBufferState.RECORDED) {
      throw new ValidationError(
        `Cannot submit CB#${cb.commandBufferId}: ` +
        `state is ${cb.state} (expected RECORDED)`,
      );
    }
  }

  // --- Dispatch validation ---

  /** Validate a dispatch command. */
  validateDispatch(
    cb: CommandBuffer,
    groupX: number,
    groupY: number,
    groupZ: number,
  ): void {
    if (cb.boundPipeline === null) {
      throw new ValidationError(
        `Cannot dispatch in CB#${cb.commandBufferId}: ` +
        `no pipeline bound (call cmdBindPipeline first)`,
      );
    }
    if (groupX <= 0 || groupY <= 0 || groupZ <= 0) {
      throw new ValidationError(
        `Dispatch dimensions must be positive: ` +
        `(${groupX}, ${groupY}, ${groupZ})`,
      );
    }
  }

  // --- Memory validation ---

  /** Validate that a buffer can be mapped. */
  validateMap(buffer: Buffer): void {
    if (buffer.freed) {
      throw new ValidationError(
        `Cannot map freed buffer ${buffer.bufferId}`,
      );
    }
    if (buffer.mapped) {
      throw new ValidationError(
        `Buffer ${buffer.bufferId} is already mapped`,
      );
    }
    if (!hasMemoryType(buffer.memoryType, MemoryType.HOST_VISIBLE)) {
      throw new ValidationError(
        `Cannot map buffer ${buffer.bufferId}: ` +
        `not HOST_VISIBLE (type=${buffer.memoryType}). ` +
        `Use a staging buffer for DEVICE_LOCAL memory.`,
      );
    }
  }

  /** Validate that a buffer has the required usage flags. */
  validateBufferUsage(buffer: Buffer, requiredUsage: BufferUsage): void {
    if (!hasBufferUsage(buffer.usage, requiredUsage)) {
      throw new ValidationError(
        `Buffer ${buffer.bufferId} lacks required usage ` +
        `${requiredUsage} (has ${buffer.usage})`,
      );
    }
  }

  /** Validate that a buffer is not freed. */
  validateBufferNotFreed(buffer: Buffer): void {
    if (buffer.freed) {
      throw new ValidationError(
        `Buffer ${buffer.bufferId} has been freed`,
      );
    }
  }

  // --- Barrier validation ---

  /** Record that a buffer was written to (for barrier checking). */
  recordWrite(bufferId: number): void {
    this._writtenBuffers.add(bufferId);
    this._barrieredBuffers.delete(bufferId);
  }

  /** Record that a barrier was placed (covers some/all buffers). */
  recordBarrier(bufferIds?: Set<number>): void {
    if (bufferIds === undefined) {
      // Global barrier -- covers all written buffers
      for (const id of this._writtenBuffers) {
        this._barrieredBuffers.add(id);
      }
    } else {
      for (const id of bufferIds) {
        this._barrieredBuffers.add(id);
      }
    }
  }

  /** Warn if reading a buffer that was written without a barrier. */
  validateReadAfterWrite(bufferId: number): void {
    if (
      this._writtenBuffers.has(bufferId) &&
      !this._barrieredBuffers.has(bufferId)
    ) {
      this._warnings.push(
        `Reading buffer ${bufferId} after write without barrier. ` +
        `Insert cmdPipelineBarrier() between write and read.`,
      );
    }
  }

  // --- Descriptor set validation ---

  /** Validate that a descriptor set is compatible with a pipeline. */
  validateDescriptorSet(descriptorSet: DescriptorSet, pipeline: Pipeline): void {
    const layout = pipeline.layout;
    if (layout.setLayouts.length === 0) {
      return; // No descriptors needed
    }

    const expectedLayout = layout.setLayouts[0];
    for (const bindingDef of expectedLayout.bindings) {
      const buf = descriptorSet.getBuffer(bindingDef.binding);
      if (buf === null) {
        this._warnings.push(
          `Binding ${bindingDef.binding} not set in ` +
          `descriptor set ${descriptorSet.setId}`,
        );
      } else if (buf.freed) {
        throw new ValidationError(
          `Binding ${bindingDef.binding} uses freed buffer ` +
          `${buf.bufferId}`,
        );
      }
    }
  }
}
