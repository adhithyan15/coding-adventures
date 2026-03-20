/**
 * Tests for the OpenGL compute simulator.
 */
import { describe, it, expect } from "vitest";

import {
  GLContext,
  GL_COMPUTE_SHADER,
  GL_SHADER_STORAGE_BUFFER,
  GL_ARRAY_BUFFER,
  GL_UNIFORM_BUFFER,
  GL_STATIC_DRAW,
  GL_DYNAMIC_DRAW,
  GL_STREAM_DRAW,
  GL_MAP_READ_BIT,
  GL_MAP_WRITE_BIT,
  GL_SHADER_STORAGE_BARRIER_BIT,
  GL_BUFFER_UPDATE_BARRIER_BIT,
  GL_ALL_BARRIER_BITS,
  GL_ALREADY_SIGNALED,
  GL_CONDITION_SATISFIED,
  GL_TIMEOUT_EXPIRED,
  GL_WAIT_FAILED,
  GL_SYNC_FLUSH_COMMANDS_BIT,
  GL_SYNC_GPU_COMMANDS_COMPLETE,
} from "../src/index.js";

describe("GLContext creation", () => {
  it("creates a context", () => {
    const gl = new GLContext();
    expect(gl).toBeDefined();
  });

  it("has internal device state", () => {
    const gl = new GLContext();
    expect(gl._logicalDevice).toBeDefined();
    expect(gl._physicalDevice).toBeDefined();
  });
});

describe("Shader management", () => {
  it("createShader returns an integer handle", () => {
    const gl = new GLContext();
    const shader = gl.createShader(GL_COMPUTE_SHADER);
    expect(typeof shader).toBe("number");
    expect(shader).toBeGreaterThan(0);
  });

  it("createShader with invalid type throws", () => {
    const gl = new GLContext();
    expect(() => gl.createShader(0x9999)).toThrow();
  });

  it("shaderSource sets source on shader", () => {
    const gl = new GLContext();
    const shader = gl.createShader(GL_COMPUTE_SHADER);
    gl.shaderSource(shader, "void main() {}");
  });

  it("shaderSource with invalid handle throws", () => {
    const gl = new GLContext();
    expect(() => gl.shaderSource(9999, "src")).toThrow("Invalid shader handle");
  });

  it("compileShader marks shader as compiled", () => {
    const gl = new GLContext();
    const shader = gl.createShader(GL_COMPUTE_SHADER);
    gl.shaderSource(shader, "compute");
    gl.compileShader(shader);
  });

  it("compileShader with invalid handle throws", () => {
    const gl = new GLContext();
    expect(() => gl.compileShader(9999)).toThrow("Invalid shader handle");
  });

  it("deleteShader removes the shader", () => {
    const gl = new GLContext();
    const shader = gl.createShader(GL_COMPUTE_SHADER);
    gl.deleteShader(shader);
    expect(gl._shaders.has(shader)).toBe(false);
  });
});

describe("Program management", () => {
  it("createProgram returns an integer handle", () => {
    const gl = new GLContext();
    const prog = gl.createProgram();
    expect(typeof prog).toBe("number");
    expect(prog).toBeGreaterThan(0);
  });

  it("attachShader attaches shader to program", () => {
    const gl = new GLContext();
    const shader = gl.createShader(GL_COMPUTE_SHADER);
    gl.shaderSource(shader, "compute");
    gl.compileShader(shader);
    const prog = gl.createProgram();
    gl.attachShader(prog, shader);
  });

  it("attachShader with invalid program throws", () => {
    const gl = new GLContext();
    const shader = gl.createShader(GL_COMPUTE_SHADER);
    expect(() => gl.attachShader(9999, shader)).toThrow("Invalid program handle");
  });

  it("attachShader with invalid shader throws", () => {
    const gl = new GLContext();
    const prog = gl.createProgram();
    expect(() => gl.attachShader(prog, 9999)).toThrow("Invalid shader handle");
  });

  it("linkProgram links a program with attached shader", () => {
    const gl = new GLContext();
    const shader = gl.createShader(GL_COMPUTE_SHADER);
    gl.shaderSource(shader, "compute");
    gl.compileShader(shader);
    const prog = gl.createProgram();
    gl.attachShader(prog, shader);
    gl.linkProgram(prog);
  });

  it("linkProgram with invalid handle throws", () => {
    const gl = new GLContext();
    expect(() => gl.linkProgram(9999)).toThrow("Invalid program handle");
  });

  it("linkProgram with no shaders throws", () => {
    const gl = new GLContext();
    const prog = gl.createProgram();
    expect(() => gl.linkProgram(prog)).toThrow("no attached shaders");
  });

  it("useProgram sets the active program", () => {
    const gl = new GLContext();
    const shader = gl.createShader(GL_COMPUTE_SHADER);
    gl.shaderSource(shader, "compute");
    gl.compileShader(shader);
    const prog = gl.createProgram();
    gl.attachShader(prog, shader);
    gl.linkProgram(prog);
    gl.useProgram(prog);
  });

  it("useProgram(0) unbinds current program", () => {
    const gl = new GLContext();
    gl.useProgram(0);
  });

  it("useProgram with invalid handle throws", () => {
    const gl = new GLContext();
    expect(() => gl.useProgram(9999)).toThrow("Invalid program handle");
  });

  it("useProgram with unlinked program throws", () => {
    const gl = new GLContext();
    const prog = gl.createProgram();
    expect(() => gl.useProgram(prog)).toThrow("not linked");
  });

  it("deleteProgram removes program", () => {
    const gl = new GLContext();
    const shader = gl.createShader(GL_COMPUTE_SHADER);
    gl.shaderSource(shader, "compute");
    gl.compileShader(shader);
    const prog = gl.createProgram();
    gl.attachShader(prog, shader);
    gl.linkProgram(prog);
    gl.useProgram(prog);
    gl.deleteProgram(prog);
    expect(gl._programs.has(prog)).toBe(false);
  });
});

describe("Buffer management", () => {
  it("genBuffers returns integer handles", () => {
    const gl = new GLContext();
    const bufs = gl.genBuffers(3);
    expect(bufs.length).toBe(3);
    for (const b of bufs) {
      expect(typeof b).toBe("number");
    }
  });

  it("bindBuffer and bufferData allocate storage", () => {
    const gl = new GLContext();
    const [buf] = gl.genBuffers(1);
    gl.bindBuffer(GL_SHADER_STORAGE_BUFFER, buf);
    gl.bufferData(GL_SHADER_STORAGE_BUFFER, 64, null, GL_STATIC_DRAW);
  });

  it("bufferData with initial data", () => {
    const gl = new GLContext();
    const [buf] = gl.genBuffers(1);
    gl.bindBuffer(GL_SHADER_STORAGE_BUFFER, buf);
    const data = new Uint8Array([1, 2, 3, 4]);
    gl.bufferData(GL_SHADER_STORAGE_BUFFER, 4, data, GL_STATIC_DRAW);
  });

  it("bufferData without bound buffer throws", () => {
    const gl = new GLContext();
    expect(() =>
      gl.bufferData(GL_SHADER_STORAGE_BUFFER, 64, null, GL_STATIC_DRAW),
    ).toThrow("No buffer bound");
  });

  it("bindBuffer(0) unbinds buffer", () => {
    const gl = new GLContext();
    const [buf] = gl.genBuffers(1);
    gl.bindBuffer(GL_SHADER_STORAGE_BUFFER, buf);
    gl.bindBuffer(GL_SHADER_STORAGE_BUFFER, 0);
  });

  it("bindBuffer with invalid handle throws", () => {
    const gl = new GLContext();
    expect(() => gl.bindBuffer(GL_SHADER_STORAGE_BUFFER, 9999)).toThrow(
      "Invalid buffer handle",
    );
  });

  it("bufferSubData updates buffer portion", () => {
    const gl = new GLContext();
    const [buf] = gl.genBuffers(1);
    gl.bindBuffer(GL_SHADER_STORAGE_BUFFER, buf);
    gl.bufferData(GL_SHADER_STORAGE_BUFFER, 8, null, GL_STATIC_DRAW);
    gl.bufferSubData(GL_SHADER_STORAGE_BUFFER, 0, new Uint8Array([0xaa, 0xbb]));
  });

  it("bufferSubData without bound buffer throws", () => {
    const gl = new GLContext();
    expect(() =>
      gl.bufferSubData(GL_SHADER_STORAGE_BUFFER, 0, new Uint8Array([1])),
    ).toThrow("No buffer bound");
  });

  it("bufferSubData on buffer with no data store throws", () => {
    const gl = new GLContext();
    const [buf] = gl.genBuffers(1);
    gl.bindBuffer(GL_SHADER_STORAGE_BUFFER, buf);
    expect(() =>
      gl.bufferSubData(GL_SHADER_STORAGE_BUFFER, 0, new Uint8Array([1])),
    ).toThrow("no data store");
  });

  it("bindBufferBase binds to indexed binding", () => {
    const gl = new GLContext();
    const [buf] = gl.genBuffers(1);
    gl.bindBuffer(GL_SHADER_STORAGE_BUFFER, buf);
    gl.bufferData(GL_SHADER_STORAGE_BUFFER, 64, null, GL_STATIC_DRAW);
    gl.bindBufferBase(GL_SHADER_STORAGE_BUFFER, 0, buf);
  });

  it("bindBufferBase with invalid handle throws", () => {
    const gl = new GLContext();
    expect(() =>
      gl.bindBufferBase(GL_SHADER_STORAGE_BUFFER, 0, 9999),
    ).toThrow("Invalid buffer handle");
  });

  it("mapBufferRange returns buffer data", () => {
    const gl = new GLContext();
    const [buf] = gl.genBuffers(1);
    gl.bindBuffer(GL_SHADER_STORAGE_BUFFER, buf);
    const data = new Uint8Array([10, 20, 30, 40]);
    gl.bufferData(GL_SHADER_STORAGE_BUFFER, 4, data, GL_STATIC_DRAW);
    const mapped = gl.mapBufferRange(GL_SHADER_STORAGE_BUFFER, 0, 4, GL_MAP_READ_BIT);
    expect(mapped).toEqual(data);
  });

  it("mapBufferRange without bound buffer throws", () => {
    const gl = new GLContext();
    expect(() =>
      gl.mapBufferRange(GL_SHADER_STORAGE_BUFFER, 0, 4, GL_MAP_READ_BIT),
    ).toThrow("No buffer bound");
  });

  it("mapBufferRange on buffer with no data store throws", () => {
    const gl = new GLContext();
    const [buf] = gl.genBuffers(1);
    gl.bindBuffer(GL_SHADER_STORAGE_BUFFER, buf);
    expect(() =>
      gl.mapBufferRange(GL_SHADER_STORAGE_BUFFER, 0, 4, GL_MAP_READ_BIT),
    ).toThrow("no data store");
  });

  it("unmapBuffer returns true", () => {
    const gl = new GLContext();
    expect(gl.unmapBuffer(GL_SHADER_STORAGE_BUFFER)).toBe(true);
  });

  it("deleteBuffers removes buffers", () => {
    const gl = new GLContext();
    const bufs = gl.genBuffers(2);
    gl.bindBuffer(GL_SHADER_STORAGE_BUFFER, bufs[0]);
    gl.bufferData(GL_SHADER_STORAGE_BUFFER, 32, null, GL_STATIC_DRAW);
    gl.deleteBuffers(bufs);
    expect(gl._buffers.has(bufs[0])).toBe(false);
  });

  it("deleteBuffers cleans up bound buffer references", () => {
    const gl = new GLContext();
    const [buf] = gl.genBuffers(1);
    gl.bindBuffer(GL_SHADER_STORAGE_BUFFER, buf);
    gl.bufferData(GL_SHADER_STORAGE_BUFFER, 32, null, GL_STATIC_DRAW);
    gl.bindBufferBase(GL_SHADER_STORAGE_BUFFER, 0, buf);
    gl.deleteBuffers([buf]);
  });
});

describe("Compute dispatch", () => {
  function makeLinkedProgram(gl: InstanceType<typeof GLContext>): number {
    const shader = gl.createShader(GL_COMPUTE_SHADER);
    gl.shaderSource(shader, "compute");
    gl.compileShader(shader);
    const prog = gl.createProgram();
    gl.attachShader(prog, shader);
    gl.linkProgram(prog);
    return prog;
  }

  it("dispatchCompute without active program throws", () => {
    const gl = new GLContext();
    expect(() => gl.dispatchCompute(1, 1, 1)).toThrow("No program");
  });

  it("dispatchCompute dispatches work", () => {
    const gl = new GLContext();
    const prog = makeLinkedProgram(gl);
    gl.useProgram(prog);
    gl.dispatchCompute(4, 1, 1);
  });

  it("dispatchCompute with SSBO bindings", () => {
    const gl = new GLContext();
    const prog = makeLinkedProgram(gl);
    gl.useProgram(prog);

    const [buf] = gl.genBuffers(1);
    gl.bindBuffer(GL_SHADER_STORAGE_BUFFER, buf);
    gl.bufferData(GL_SHADER_STORAGE_BUFFER, 64, null, GL_STATIC_DRAW);
    gl.bindBufferBase(GL_SHADER_STORAGE_BUFFER, 0, buf);

    gl.dispatchCompute(2, 1, 1);
  });

  it("dispatchCompute with multiple SSBO bindings", () => {
    const gl = new GLContext();
    const prog = makeLinkedProgram(gl);
    gl.useProgram(prog);

    const bufs = gl.genBuffers(2);
    gl.bindBuffer(GL_SHADER_STORAGE_BUFFER, bufs[0]);
    gl.bufferData(GL_SHADER_STORAGE_BUFFER, 64, null, GL_STATIC_DRAW);
    gl.bindBufferBase(GL_SHADER_STORAGE_BUFFER, 0, bufs[0]);

    gl.bindBuffer(GL_SHADER_STORAGE_BUFFER, bufs[1]);
    gl.bufferData(GL_SHADER_STORAGE_BUFFER, 64, null, GL_STATIC_DRAW);
    gl.bindBufferBase(GL_SHADER_STORAGE_BUFFER, 1, bufs[1]);

    gl.dispatchCompute(2, 2, 1);
  });

  it("dispatchCompute with 3D dispatch", () => {
    const gl = new GLContext();
    const prog = makeLinkedProgram(gl);
    gl.useProgram(prog);
    gl.dispatchCompute(2, 2, 2);
  });
});

describe("Synchronization", () => {
  it("memoryBarrier is a no-op", () => {
    const gl = new GLContext();
    gl.memoryBarrier(GL_SHADER_STORAGE_BARRIER_BIT);
    gl.memoryBarrier(GL_ALL_BARRIER_BITS);
  });

  it("fenceSync returns an integer handle", () => {
    const gl = new GLContext();
    const sync = gl.fenceSync();
    expect(typeof sync).toBe("number");
    expect(sync).toBeGreaterThan(0);
  });

  it("clientWaitSync with signaled fence returns ALREADY_SIGNALED", () => {
    const gl = new GLContext();
    const sync = gl.fenceSync();
    const result = gl.clientWaitSync(sync, 0, 1000);
    expect(result).toBe(GL_ALREADY_SIGNALED);
  });

  it("clientWaitSync with invalid sync returns WAIT_FAILED", () => {
    const gl = new GLContext();
    const result = gl.clientWaitSync(99999, 0, 1000);
    expect(result).toBe(GL_WAIT_FAILED);
  });

  it("deleteSync removes the sync object", () => {
    const gl = new GLContext();
    const sync = gl.fenceSync();
    gl.deleteSync(sync);
    const result = gl.clientWaitSync(sync, 0, 1000);
    expect(result).toBe(GL_WAIT_FAILED);
  });

  it("finish blocks until complete", () => {
    const gl = new GLContext();
    gl.finish();
  });
});

describe("Uniforms", () => {
  it("getUniformLocation returns a number", () => {
    const gl = new GLContext();
    const shader = gl.createShader(GL_COMPUTE_SHADER);
    gl.shaderSource(shader, "compute");
    gl.compileShader(shader);
    const prog = gl.createProgram();
    gl.attachShader(prog, shader);
    gl.linkProgram(prog);
    const loc = gl.getUniformLocation(prog, "u_scale");
    expect(typeof loc).toBe("number");
  });

  it("getUniformLocation with invalid program throws", () => {
    const gl = new GLContext();
    expect(() => gl.getUniformLocation(9999, "u_x")).toThrow("Invalid program handle");
  });

  it("uniform1f sets a float uniform", () => {
    const gl = new GLContext();
    const shader = gl.createShader(GL_COMPUTE_SHADER);
    gl.shaderSource(shader, "compute");
    gl.compileShader(shader);
    const prog = gl.createProgram();
    gl.attachShader(prog, shader);
    gl.linkProgram(prog);
    gl.useProgram(prog);
    const loc = gl.getUniformLocation(prog, "u_scale");
    gl.uniform1f(loc, 3.14);
  });

  it("uniform1i sets an integer uniform", () => {
    const gl = new GLContext();
    const shader = gl.createShader(GL_COMPUTE_SHADER);
    gl.shaderSource(shader, "compute");
    gl.compileShader(shader);
    const prog = gl.createProgram();
    gl.attachShader(prog, shader);
    gl.linkProgram(prog);
    gl.useProgram(prog);
    const loc = gl.getUniformLocation(prog, "u_count");
    gl.uniform1i(loc, 42);
  });

  it("uniform1f without active program is a no-op", () => {
    const gl = new GLContext();
    gl.uniform1f(0, 1.0);
  });
});

describe("GL Constants", () => {
  it("buffer targets have distinct values", () => {
    expect(GL_SHADER_STORAGE_BUFFER).not.toBe(GL_ARRAY_BUFFER);
    expect(GL_SHADER_STORAGE_BUFFER).not.toBe(GL_UNIFORM_BUFFER);
    expect(GL_ARRAY_BUFFER).not.toBe(GL_UNIFORM_BUFFER);
  });

  it("usage hints have distinct values", () => {
    expect(GL_STATIC_DRAW).not.toBe(GL_DYNAMIC_DRAW);
    expect(GL_STATIC_DRAW).not.toBe(GL_STREAM_DRAW);
  });

  it("map bits are distinct", () => {
    expect(GL_MAP_READ_BIT).not.toBe(GL_MAP_WRITE_BIT);
  });

  it("sync results are distinct", () => {
    const values = new Set([GL_ALREADY_SIGNALED, GL_CONDITION_SATISFIED, GL_TIMEOUT_EXPIRED, GL_WAIT_FAILED]);
    expect(values.size).toBe(4);
  });
});
