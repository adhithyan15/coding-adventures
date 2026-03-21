/**
 * OpenGlBlas -- legacy OpenGL compute BLAS backend.
 *
 * === How OpenGlBlas Works ===
 *
 * This backend wraps `GLContext` from Layer 4. OpenGL uses a global state
 * machine model -- you bind things to "current" state and then issue commands
 * that operate on whatever is currently bound.
 *
 * For each BLAS operation:
 *     1. gl.genBuffers()       -- generate buffer IDs
 *     2. gl.bufferData()       -- allocate and upload data
 *     3. (compute)             -- perform operation
 *     4. gl.mapBufferRange()   -- map buffer for reading
 *     5. gl.deleteBuffers()    -- free buffers
 *
 * OpenGL compute shaders (4.3+) use Shader Storage Buffer Objects (SSBOs)
 * for GPU-accessible storage.
 */

import {
  GLContext,
  GL_SHADER_STORAGE_BUFFER,
  GL_STATIC_DRAW,
  GL_MAP_READ_BIT,
} from "@coding-adventures/vendor-api-simulators";

import { GpuBlasBase } from "./gpu-base.js";

/**
 * OpenGL BLAS backend -- wraps GLContext from Layer 4.
 *
 * ================================================================
 * OPENGL BLAS -- LEGACY STATE MACHINE GPU ACCELERATION
 * ================================================================
 *
 * OpenGL is the oldest surviving GPU API (1992). Compute shaders
 * were added in OpenGL 4.3 (2012), bolted onto the existing state
 * machine model.
 *
 * The state machine means:
 * - glBindBuffer(target, id)  sets "current buffer" globally
 * - glBufferData(target, ...) operates on WHATEVER is currently bound
 * - You must remember what's bound at all times
 *
 * Simple for small programs, error-prone for large ones.
 *
 * Usage:
 *     const blas = new OpenGlBlas();
 *     const result = blas.sgemm(NO_TRANS, NO_TRANS, 1.0, A, B, 0.0, C);
 * ================================================================
 */
export class OpenGlBlas extends GpuBlasBase {
  private _gl: GLContext;

  constructor() {
    super();
    this._gl = new GLContext();
  }

  get name(): string {
    return "opengl";
  }

  get deviceName(): string {
    return "OpenGL Device";
  }

  /**
   * Create an OpenGL SSBO and upload data.
   *
   * The OpenGL state machine pattern:
   *     1. glGenBuffers(1)      -- create a buffer handle (integer)
   *     2. glBindBuffer(SSBO, handle)  -- bind it as "current SSBO"
   *     3. glBufferData(SSBO, size, data, STATIC_DRAW) -- allocate + upload
   */
  protected _upload(data: Uint8Array): number {
    const bufId = this._gl.genBuffers(1)[0];
    this._gl.bindBuffer(GL_SHADER_STORAGE_BUFFER, bufId);
    this._gl.bufferData(GL_SHADER_STORAGE_BUFFER, data.length, data, GL_STATIC_DRAW);
    return bufId;
  }

  /**
   * Map the OpenGL buffer for reading and copy data out.
   *
   *     1. glBindBuffer(SSBO, handle) -- bind as current
   *     2. glMapBufferRange(SSBO, 0, size, READ) -- get CPU pointer
   *     3. Copy data
   *     4. glUnmapBuffer(SSBO) -- release the mapping
   */
  protected _download(handle: unknown, size: number): Uint8Array {
    const bufId = handle as number;
    this._gl.bindBuffer(GL_SHADER_STORAGE_BUFFER, bufId);
    const mapped = this._gl.mapBufferRange(
      GL_SHADER_STORAGE_BUFFER, 0, size, GL_MAP_READ_BIT,
    );
    const data = new Uint8Array(mapped.slice(0, size));
    this._gl.unmapBuffer(GL_SHADER_STORAGE_BUFFER);
    return data;
  }

  /**
   * Delete the OpenGL buffer.
   *
   * Unlike most modern APIs, OpenGL requires explicit deletion:
   *     glDeleteBuffers([handle])
   */
  protected _free(handle: unknown): void {
    this._gl.deleteBuffers([handle as number]);
  }
}
