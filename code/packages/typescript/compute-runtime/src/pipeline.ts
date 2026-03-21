/**
 * Pipeline -- compiled kernels, descriptor sets, shader modules.
 *
 * === What is a Pipeline? ===
 *
 * A pipeline is a **compiled kernel ready to execute**. In Vulkan terms, it
 * packages three things together:
 *
 *     1. ShaderModule -- the compiled program (instructions)
 *     2. PipelineLayout -- what data the kernel expects (descriptor set layout)
 *     3. Pipeline -- the combined, ready-to-dispatch object
 *
 * Think of it like a function call:
 *     - ShaderModule = the function body (code)
 *     - DescriptorSetLayout = the function signature (parameter types)
 *     - DescriptorSet = the actual arguments (concrete buffers)
 *     - Pipeline = the compiled function ready to call
 */

import type { Buffer } from "./memory.js";
import type { DescriptorBinding } from "./protocols.js";

// =========================================================================
// ShaderModule -- compiled program
// =========================================================================

/**
 * A compiled program ready to be used in a pipeline.
 *
 * === GPU vs Dataflow ===
 *
 * For GPU-style devices (NVIDIA, AMD, Intel), the code is a list of
 * instructions from our GenericISA (gpu-core package).
 *
 * For dataflow-style devices (TPU, ANE), the code is an operation
 * descriptor -- just the operation name and parameters.
 */
export class ShaderModule {
  private static _nextId = 0;

  private readonly _id: number;
  private readonly _code: unknown[] | null;
  private readonly _operation: string;
  private readonly _entryPoint: string;
  private readonly _localSize: readonly [number, number, number];

  constructor(options: {
    code?: unknown[] | null;
    operation?: string;
    entryPoint?: string;
    localSize?: readonly [number, number, number];
  } = {}) {
    this._id = ShaderModule._nextId++;
    this._code = options.code ?? null;
    this._operation = options.operation ?? "";
    this._entryPoint = options.entryPoint ?? "main";
    this._localSize = options.localSize ?? [32, 1, 1];
  }

  /** Unique identifier. */
  get moduleId(): number {
    return this._id;
  }

  /** GPU-style: list of instructions. null for dataflow. */
  get code(): unknown[] | null {
    return this._code;
  }

  /** Dataflow-style: operation name (e.g., 'matmul'). Empty for GPU. */
  get operation(): string {
    return this._operation;
  }

  /** Entry point name (typically 'main'). */
  get entryPoint(): string {
    return this._entryPoint;
  }

  /** Workgroup dimensions declared in the shader. */
  get localSize(): readonly [number, number, number] {
    return this._localSize;
  }

  /** True if this is a GPU-style shader (has instruction code). */
  get isGpuStyle(): boolean {
    return this._code !== null;
  }

  /** True if this is a dataflow-style shader (has operation name). */
  get isDataflowStyle(): boolean {
    return this._operation.length > 0;
  }
}

// =========================================================================
// DescriptorSetLayout -- describes the shape of data bindings
// =========================================================================

/**
 * Describes what data a kernel expects.
 *
 * A layout is like a function signature -- it says "this kernel takes
 * 3 storage buffers." It doesn't say WHICH buffers, just how many
 * and what type.
 */
export class DescriptorSetLayout {
  private static _nextId = 0;

  private readonly _id: number;
  private readonly _bindings: readonly DescriptorBinding[];

  constructor(bindings: DescriptorBinding[]) {
    this._id = DescriptorSetLayout._nextId++;
    this._bindings = Object.freeze([...bindings]);
  }

  /** Unique identifier. */
  get layoutId(): number {
    return this._id;
  }

  /** The binding slots in this layout. */
  get bindings(): readonly DescriptorBinding[] {
    return this._bindings;
  }
}

// =========================================================================
// PipelineLayout -- shader + descriptor layout + push constants
// =========================================================================

/**
 * Describes the complete interface of a pipeline.
 *
 * Combines:
 * - Descriptor set layouts (what buffers the kernel reads/writes)
 * - Push constant size (small inline data like alpha in SAXPY)
 */
export class PipelineLayout {
  private static _nextId = 0;

  private readonly _id: number;
  private readonly _setLayouts: DescriptorSetLayout[];
  private readonly _pushConstantSize: number;

  constructor(
    setLayouts: DescriptorSetLayout[],
    pushConstantSize = 0,
  ) {
    this._id = PipelineLayout._nextId++;
    this._setLayouts = [...setLayouts];
    this._pushConstantSize = pushConstantSize;
  }

  /** Unique identifier. */
  get layoutId(): number {
    return this._id;
  }

  /** Descriptor set layouts used by this pipeline. */
  get setLayouts(): DescriptorSetLayout[] {
    return this._setLayouts;
  }

  /** Maximum bytes for push constants. */
  get pushConstantSize(): number {
    return this._pushConstantSize;
  }
}

// =========================================================================
// Pipeline -- compiled, ready to dispatch
// =========================================================================

/**
 * A compiled kernel bound to a pipeline layout.
 *
 * Once created, bind it in a command buffer:
 *     cb.cmdBindPipeline(pipeline)
 *     cb.cmdDispatch(gridX, gridY, gridZ)
 */
export class Pipeline {
  private static _nextId = 0;

  private readonly _id: number;
  private readonly _shader: ShaderModule;
  private readonly _layout: PipelineLayout;

  constructor(shader: ShaderModule, layout: PipelineLayout) {
    this._id = Pipeline._nextId++;
    this._shader = shader;
    this._layout = layout;
  }

  /** Unique identifier. */
  get pipelineId(): number {
    return this._id;
  }

  /** The compiled shader module. */
  get shader(): ShaderModule {
    return this._shader;
  }

  /** The pipeline layout (descriptor sets + push constants). */
  get layout(): PipelineLayout {
    return this._layout;
  }

  /** Local workgroup dimensions from the shader. */
  get workgroupSize(): readonly [number, number, number] {
    return this._shader.localSize;
  }
}

// =========================================================================
// DescriptorSet -- concrete buffer bindings
// =========================================================================

/**
 * Concrete buffer assignments for a descriptor set layout.
 *
 * Layout says: "binding 0 is a storage buffer"
 * Set says:    "binding 0 is buf_x (address 0x1000, 4096 bytes)"
 */
export class DescriptorSet {
  private static _nextId = 0;

  private readonly _id: number;
  private readonly _layout: DescriptorSetLayout;
  private readonly _bindings: Map<number, Buffer>;

  constructor(layout: DescriptorSetLayout) {
    this._id = DescriptorSet._nextId++;
    this._layout = layout;
    this._bindings = new Map();
  }

  /** Unique identifier. */
  get setId(): number {
    return this._id;
  }

  /** The layout this set was created from. */
  get layout(): DescriptorSetLayout {
    return this._layout;
  }

  /** Current buffer bindings (binding number -> Buffer). */
  get bindings(): Map<number, Buffer> {
    return new Map(this._bindings);
  }

  /**
   * Bind a buffer to a slot.
   *
   * @param binding - Slot number (must exist in layout).
   * @param buffer - The buffer to bind.
   * @throws Error if binding doesn't exist in layout or buffer is freed.
   */
  write(binding: number, buffer: Buffer): void {
    const validBindings = new Set(this._layout.bindings.map((b) => b.binding));
    if (!validBindings.has(binding)) {
      throw new Error(
        `Binding ${binding} not in layout (valid: {${[...validBindings].join(", ")}})`,
      );
    }
    if (buffer.freed) {
      throw new Error(
        `Cannot bind freed buffer ${buffer.bufferId} to binding ${binding}`,
      );
    }
    this._bindings.set(binding, buffer);
  }

  /** Get the buffer at a binding slot, or null if not bound. */
  getBuffer(binding: number): Buffer | null {
    return this._bindings.get(binding) ?? null;
  }
}
