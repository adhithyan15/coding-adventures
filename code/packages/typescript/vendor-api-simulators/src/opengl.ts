/**
 * OpenGL Compute Simulator -- the legacy global state machine.
 *
 * === What is OpenGL? ===
 *
 * OpenGL is the oldest surviving GPU API (1992). Compute shaders were bolted
 * on in OpenGL 4.3 (2012). OpenGL uses a **global state machine** model where
 * you bind things to "current" state and then issue commands that operate on
 * whatever is currently bound.
 *
 * === The State Machine Model ===
 *
 *     glUseProgram(prog)           // Sets "current program" globally
 *     glBindBufferBase(0, buf_a)   // Sets "buffer at binding 0" globally
 *     glDispatchCompute(4, 1, 1)   // Uses WHATEVER is currently bound
 *
 * === Integer Handles ===
 *
 * OpenGL uses integer handles (GLuint) for everything. You never get a
 * typed object -- just a number:
 *
 *     const shader = gl.createShader(GL_COMPUTE_SHADER);   // Returns 1
 *     const program = gl.createProgram();                   // Returns 2
 *     const buffers = gl.genBuffers(2);                     // Returns [3, 4]
 */

import {
  type Buffer as RuntimeBuffer,
  BufferUsage,
  MemoryType,
  makeDescriptorBinding,
} from "@coding-adventures/compute-runtime";

import { BaseVendorSimulator } from "./base.js";

// =========================================================================
// OpenGL constants
// =========================================================================

// Shader types
export const GL_COMPUTE_SHADER = 0x91b9;

// Buffer targets
export const GL_SHADER_STORAGE_BUFFER = 0x90d2;
export const GL_ARRAY_BUFFER = 0x8892;
export const GL_UNIFORM_BUFFER = 0x8a11;

// Buffer usage hints
export const GL_STATIC_DRAW = 0x88e4;
export const GL_DYNAMIC_DRAW = 0x88e8;
export const GL_STREAM_DRAW = 0x88e0;

// Map access bits
export const GL_MAP_READ_BIT = 0x0001;
export const GL_MAP_WRITE_BIT = 0x0002;

// Memory barrier bits
export const GL_SHADER_STORAGE_BARRIER_BIT = 0x00002000;
export const GL_BUFFER_UPDATE_BARRIER_BIT = 0x00000200;
export const GL_ALL_BARRIER_BITS = 0xffffffff;

// Sync object results
export const GL_ALREADY_SIGNALED = 0x911a;
export const GL_CONDITION_SATISFIED = 0x911c;
export const GL_TIMEOUT_EXPIRED = 0x911b;
export const GL_WAIT_FAILED = 0x911d;

// Sync flags
export const GL_SYNC_FLUSH_COMMANDS_BIT = 0x00000001;
export const GL_SYNC_GPU_COMMANDS_COMPLETE = 0x9117;

// =========================================================================
// GLContext -- the main OpenGL state machine
// =========================================================================

/**
 * OpenGL context -- a global state machine for GPU programming.
 *
 * === The State Machine ===
 *
 * GLContext maintains global state that commands operate on:
 * - _currentProgram:  Which program is currently active
 * - _boundBuffers:    Which buffers are bound to which targets/indices
 * - _programs:        Map of GL handle -> Layer 5 Pipeline
 * - _shaders:         Map of GL handle -> shader source + code
 * - _buffers:         Map of GL handle -> Layer 5 Buffer
 *
 * === Usage ===
 *
 *     const gl = new GLContext();
 *     const shader = gl.createShader(GL_COMPUTE_SHADER);
 *     gl.shaderSource(shader, "saxpy");
 *     gl.compileShader(shader);
 *     const program = gl.createProgram();
 *     gl.attachShader(program, shader);
 *     gl.linkProgram(program);
 *     gl.useProgram(program);
 *     gl.dispatchCompute(4, 1, 1);
 */
export class GLContext extends BaseVendorSimulator {
  private _currentProgram: number | null = null;
  private readonly _boundBuffers: Map<string, number> = new Map();
  private readonly _targetBuffers: Map<number, number> = new Map();

  /** @internal */ readonly _shaders: Map<
    number,
    { source: string; code: unknown[] | null; compiled: boolean; type: number }
  > = new Map();
  /** @internal */ readonly _programs: Map<
    number,
    {
      pipeline: unknown;
      shaders: number[];
      linked: boolean;
      shaderModule: unknown;
    }
  > = new Map();
  /** @internal */ readonly _buffers: Map<number, RuntimeBuffer | null> = new Map();
  private readonly _syncs: Map<number, import("@coding-adventures/compute-runtime").Fence> = new Map();
  private readonly _uniforms: Map<string, unknown> = new Map();
  private _nextId = 1;

  constructor() {
    super();
  }

  private _genId(): number {
    return this._nextId++;
  }

  // =================================================================
  // Shader management
  // =================================================================

  /**
   * Create a shader object (glCreateShader).
   *
   * @throws Error if shaderType is not GL_COMPUTE_SHADER.
   */
  createShader(shaderType: number): number {
    if (shaderType !== GL_COMPUTE_SHADER) {
      throw new Error(
        `Only GL_COMPUTE_SHADER (0x${GL_COMPUTE_SHADER.toString(16).toUpperCase()}) is supported, ` +
        `got 0x${shaderType.toString(16).toUpperCase()}`,
      );
    }
    const handle = this._genId();
    this._shaders.set(handle, {
      source: "",
      code: null,
      compiled: false,
      type: shaderType,
    });
    return handle;
  }

  /**
   * Set shader source code (glShaderSource).
   *
   * @throws Error if shader handle is invalid.
   */
  shaderSource(shader: number, source: string): void {
    if (!this._shaders.has(shader)) {
      throw new Error(`Invalid shader handle ${shader}`);
    }
    this._shaders.get(shader)!.source = source;
  }

  /**
   * Compile a shader (glCompileShader).
   *
   * @throws Error if shader handle is invalid.
   */
  compileShader(shader: number): void {
    if (!this._shaders.has(shader)) {
      throw new Error(`Invalid shader handle ${shader}`);
    }
    this._shaders.get(shader)!.compiled = true;
  }

  /** Delete a shader object (glDeleteShader). */
  deleteShader(shader: number): void {
    this._shaders.delete(shader);
  }

  // =================================================================
  // Program management
  // =================================================================

  /** Create a program object (glCreateProgram). */
  createProgram(): number {
    const handle = this._genId();
    this._programs.set(handle, {
      pipeline: null,
      shaders: [],
      linked: false,
      shaderModule: null,
    });
    return handle;
  }

  /**
   * Attach a shader to a program (glAttachShader).
   *
   * @throws Error if either handle is invalid.
   */
  attachShader(program: number, shader: number): void {
    if (!this._programs.has(program)) {
      throw new Error(`Invalid program handle ${program}`);
    }
    if (!this._shaders.has(shader)) {
      throw new Error(`Invalid shader handle ${shader}`);
    }
    this._programs.get(program)!.shaders.push(shader);
  }

  /**
   * Link a program (glLinkProgram).
   *
   * @throws Error if program handle is invalid or no shaders attached.
   */
  linkProgram(program: number): void {
    if (!this._programs.has(program)) {
      throw new Error(`Invalid program handle ${program}`);
    }

    const prog = this._programs.get(program)!;
    if (prog.shaders.length === 0) {
      throw new Error(`Program ${program} has no attached shaders`);
    }

    // Get shader code from the first compute shader
    const shaderHandle = prog.shaders[0];
    const shaderInfo = this._shaders.get(shaderHandle);
    const code = shaderInfo?.code ?? null;

    // Create Layer 5 pipeline
    const shader = this._logicalDevice.createShaderModule({ code });
    const dsLayout = this._logicalDevice.createDescriptorSetLayout([]);
    const plLayout = this._logicalDevice.createPipelineLayout([dsLayout]);
    const pipeline = this._logicalDevice.createComputePipeline(shader, plLayout);

    prog.pipeline = pipeline;
    prog.shaderModule = shader;
    prog.linked = true;
  }

  /**
   * Set the active program (glUseProgram).
   *
   * @throws Error if program handle is invalid (and not 0).
   */
  useProgram(program: number): void {
    if (program === 0) {
      this._currentProgram = null;
      return;
    }
    if (!this._programs.has(program)) {
      throw new Error(`Invalid program handle ${program}`);
    }
    if (!this._programs.get(program)!.linked) {
      throw new Error(`Program ${program} is not linked`);
    }
    this._currentProgram = program;
  }

  /** Delete a program object (glDeleteProgram). */
  deleteProgram(program: number): void {
    if (this._currentProgram === program) {
      this._currentProgram = null;
    }
    this._programs.delete(program);
  }

  // =================================================================
  // Buffer management
  // =================================================================

  /** Generate buffer objects (glGenBuffers). */
  genBuffers(count: number): number[] {
    const handles: number[] = [];
    for (let i = 0; i < count; i++) {
      const handle = this._genId();
      this._buffers.set(handle, null);
      handles.push(handle);
    }
    return handles;
  }

  /** Delete buffer objects (glDeleteBuffers). */
  deleteBuffers(buffers: number[]): void {
    for (const handle of buffers) {
      if (this._buffers.has(handle)) {
        const buf = this._buffers.get(handle);
        if (buf && !buf.freed) {
          this._memoryManager.free(buf);
        }
      }
      this._buffers.delete(handle);
      // Remove from bindings
      for (const [key, val] of this._boundBuffers.entries()) {
        if (val === handle) {
          this._boundBuffers.delete(key);
        }
      }
      for (const [key, val] of this._targetBuffers.entries()) {
        if (val === handle) {
          this._targetBuffers.delete(key);
        }
      }
    }
  }

  /** Bind a buffer to a target (glBindBuffer). */
  bindBuffer(target: number, buffer: number): void {
    if (buffer === 0) {
      this._targetBuffers.delete(target);
      return;
    }
    if (!this._buffers.has(buffer)) {
      throw new Error(`Invalid buffer handle ${buffer}`);
    }
    this._targetBuffers.set(target, buffer);
  }

  /**
   * Allocate and optionally fill a buffer (glBufferData).
   *
   * @throws Error if no buffer is bound to the target.
   */
  bufferData(
    target: number,
    size: number,
    data: Uint8Array | null,
    _usage: number,
  ): void {
    if (!this._targetBuffers.has(target)) {
      throw new Error(`No buffer bound to target 0x${target.toString(16).toUpperCase()}`);
    }

    const handle = this._targetBuffers.get(target)!;

    // Free old allocation if exists
    const oldBuf = this._buffers.get(handle);
    if (oldBuf && !oldBuf.freed) {
      this._memoryManager.free(oldBuf);
    }

    // Allocate new buffer
    const memType =
      MemoryType.DEVICE_LOCAL |
      MemoryType.HOST_VISIBLE |
      MemoryType.HOST_COHERENT;
    const bufUsage =
      BufferUsage.STORAGE | BufferUsage.TRANSFER_SRC | BufferUsage.TRANSFER_DST;
    const buf = this._memoryManager.allocate(size, memType, bufUsage);
    this._buffers.set(handle, buf);

    // Upload initial data if provided
    if (data !== null) {
      const mapped = this._memoryManager.map(buf);
      mapped.write(0, new Uint8Array(data.buffer, data.byteOffset, Math.min(data.length, size)));
      this._memoryManager.unmap(buf);
    }
  }

  /** Update a portion of a buffer (glBufferSubData). */
  bufferSubData(target: number, offset: number, data: Uint8Array): void {
    if (!this._targetBuffers.has(target)) {
      throw new Error(`No buffer bound to target 0x${target.toString(16).toUpperCase()}`);
    }
    const handle = this._targetBuffers.get(target)!;
    const buf = this._buffers.get(handle);
    if (!buf) {
      throw new Error(`Buffer ${handle} has no data store`);
    }

    const mapped = this._memoryManager.map(buf);
    mapped.write(offset, data);
    this._memoryManager.unmap(buf);
  }

  /** Bind a buffer to an indexed binding point (glBindBufferBase). */
  bindBufferBase(target: number, index: number, buffer: number): void {
    if (!this._buffers.has(buffer)) {
      throw new Error(`Invalid buffer handle ${buffer}`);
    }
    this._boundBuffers.set(`${target}:${index}`, buffer);
  }

  /**
   * Map a buffer region for CPU access (glMapBufferRange).
   *
   * @throws Error if no buffer is bound to the target.
   */
  mapBufferRange(
    target: number,
    offset: number,
    length: number,
    _access: number,
  ): Uint8Array {
    if (!this._targetBuffers.has(target)) {
      throw new Error(`No buffer bound to target 0x${target.toString(16).toUpperCase()}`);
    }
    const handle = this._targetBuffers.get(target)!;
    const buf = this._buffers.get(handle);
    if (!buf) {
      throw new Error(`Buffer ${handle} has no data store`);
    }

    this._memoryManager.invalidate(buf);
    const data = this._memoryManager.getBufferData(buf.bufferId);
    return new Uint8Array(data.slice(offset, offset + length));
  }

  /** Unmap a buffer (glUnmapBuffer). */
  unmapBuffer(_target: number): boolean {
    return true;
  }

  // =================================================================
  // Compute dispatch
  // =================================================================

  /**
   * Dispatch compute work groups (glDispatchCompute).
   *
   * Uses whatever program and SSBO bindings are currently active.
   *
   * @throws Error if no program is active.
   */
  dispatchCompute(
    numGroupsX: number,
    numGroupsY = 1,
    numGroupsZ = 1,
  ): void {
    if (this._currentProgram === null) {
      throw new Error(
        "No program is currently active (call useProgram first)",
      );
    }

    const prog = this._programs.get(this._currentProgram)!;
    const device = this._logicalDevice;

    // Get shader code
    let shaderCode: unknown[] | null = null;
    if (prog.shaders.length > 0) {
      const shaderHandle = prog.shaders[0];
      const shaderInfo = this._shaders.get(shaderHandle);
      if (shaderInfo) {
        shaderCode = shaderInfo.code;
      }
    }

    // Find all SSBO bindings
    const ssboBindings: Map<number, RuntimeBuffer> = new Map();
    for (const [key, handle] of this._boundBuffers.entries()) {
      const [targetStr, indexStr] = key.split(":");
      const target = Number(targetStr);
      const index = Number(indexStr);
      if (target === GL_SHADER_STORAGE_BUFFER && this._buffers.has(handle)) {
        const buf = this._buffers.get(handle);
        if (buf) {
          ssboBindings.set(index, buf);
        }
      }
    }

    // Create shader module
    const shader = device.createShaderModule({ code: shaderCode });

    // Create descriptor set with SSBO bindings
    const sortedKeys = [...ssboBindings.keys()].sort((a, b) => a - b);
    const bindings = sortedKeys.map((i) =>
      makeDescriptorBinding({ binding: i, type: "storage" }),
    );
    const dsLayout = device.createDescriptorSetLayout(bindings);
    const plLayout = device.createPipelineLayout([dsLayout]);
    const pipeline = device.createComputePipeline(shader, plLayout);

    const ds = device.createDescriptorSet(dsLayout);
    for (const i of sortedKeys) {
      ds.write(i, ssboBindings.get(i)!);
    }

    // Record and submit
    this._createAndSubmitCb((cb) => {
      cb.cmdBindPipeline(pipeline);
      cb.cmdBindDescriptorSet(ds);
      cb.cmdDispatch(numGroupsX, numGroupsY, numGroupsZ);
    });
  }

  // =================================================================
  // Synchronization
  // =================================================================

  /** Insert a memory barrier (glMemoryBarrier). No-op in simulator. */
  memoryBarrier(_barriers: number): void {
    // No-op in synchronous simulator
  }

  /** Create a sync object (glFenceSync). */
  fenceSync(): number {
    const handle = this._genId();
    const fence = this._logicalDevice.createFence(true);
    this._syncs.set(handle, fence);
    return handle;
  }

  /** Wait for a sync object (glClientWaitSync). */
  clientWaitSync(sync: number, _flags: number, timeout: number): number {
    if (!this._syncs.has(sync)) {
      return GL_WAIT_FAILED;
    }

    const fence = this._syncs.get(sync)!;
    if (fence.signaled) {
      return GL_ALREADY_SIGNALED;
    }

    const result = fence.wait(timeout);
    if (result) {
      return GL_CONDITION_SATISFIED;
    }
    return GL_TIMEOUT_EXPIRED;
  }

  /** Delete a sync object (glDeleteSync). */
  deleteSync(sync: number): void {
    this._syncs.delete(sync);
  }

  /** Block until all GL commands complete (glFinish). */
  finish(): void {
    this._logicalDevice.waitIdle();
  }

  // =================================================================
  // Uniforms
  // =================================================================

  /** Get the location of a uniform variable. */
  getUniformLocation(program: number, name: string): number {
    if (!this._programs.has(program)) {
      throw new Error(`Invalid program handle ${program}`);
    }
    // Deterministic hash-based location
    let hash = 0;
    for (let i = 0; i < name.length; i++) {
      hash = (hash * 31 + name.charCodeAt(i)) & 0x7fffffff;
    }
    return hash;
  }

  /** Set a float uniform (glUniform1f). */
  uniform1f(location: number, value: number): void {
    if (this._currentProgram !== null) {
      this._uniforms.set(`${this._currentProgram}:${location}`, value);
    }
  }

  /** Set an integer uniform (glUniform1i). */
  uniform1i(location: number, value: number): void {
    if (this._currentProgram !== null) {
      this._uniforms.set(`${this._currentProgram}:${location}`, value);
    }
  }
}
