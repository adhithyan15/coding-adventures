/**
 * Tests for the Metal runtime simulator.
 */
import { describe, it, expect } from "vitest";
import { halt } from "@coding-adventures/gpu-core";

import {
  MTLDevice,
  MTLCommandQueue,
  MTLCommandBuffer,
  MTLComputeCommandEncoder,
  MTLBlitCommandEncoder,
  MTLBuffer,
  MTLLibrary,
  MTLFunction,
  MTLComputePipelineState,
  MTLResourceOptions,
  MTLCommandBufferStatus,
  makeMTLSize,
  type MTLSize,
} from "../src/index.js";

describe("MTLDevice", () => {
  it("creates a device with a name", () => {
    const device = new MTLDevice();
    expect(device.name.length).toBeGreaterThan(0);
  });

  it("exposes physical and logical device", () => {
    const device = new MTLDevice();
    expect(device._physicalDevice).toBeDefined();
    expect(device._logicalDevice).toBeDefined();
  });

  it("makeCommandQueue returns an MTLCommandQueue", () => {
    const device = new MTLDevice();
    const queue = device.makeCommandQueue();
    expect(queue).toBeInstanceOf(MTLCommandQueue);
  });

  it("makeBuffer allocates with correct length", () => {
    const device = new MTLDevice();
    const buf = device.makeBuffer(512);
    expect(buf).toBeInstanceOf(MTLBuffer);
    expect(buf.length).toBe(512);
  });

  it("makeBuffer with storageModeShared", () => {
    const device = new MTLDevice();
    const buf = device.makeBuffer(256, MTLResourceOptions.storageModeShared);
    expect(buf.length).toBe(256);
  });

  it("makeBuffer with storageModePrivate", () => {
    const device = new MTLDevice();
    const buf = device.makeBuffer(128, MTLResourceOptions.storageModePrivate);
    expect(buf.length).toBe(128);
  });

  it("makeLibrary returns an MTLLibrary", () => {
    const device = new MTLDevice();
    const lib = device.makeLibrary("compute_shader_src");
    expect(lib).toBeInstanceOf(MTLLibrary);
  });

  it("makeComputePipelineState returns a pipeline state", () => {
    const device = new MTLDevice();
    const lib = device.makeLibrary("src");
    const func = lib.makeFunction("main_kernel");
    const pso = device.makeComputePipelineState(func);
    expect(pso).toBeInstanceOf(MTLComputePipelineState);
  });
});

describe("MTLBuffer (Unified Memory)", () => {
  it("writeBytes and contents round-trip data", () => {
    const device = new MTLDevice();
    const buf = device.makeBuffer(8);
    const data = new Uint8Array([10, 20, 30, 40, 50, 60, 70, 80]);
    buf.writeBytes(data);
    const result = buf.contents();
    expect(result.slice(0, 8)).toEqual(data);
  });

  it("writeBytes with offset writes at correct position", () => {
    const device = new MTLDevice();
    const buf = device.makeBuffer(8);
    buf.writeBytes(new Uint8Array([0xaa, 0xbb]), 2);
    const result = buf.contents();
    expect(result[2]).toBe(0xaa);
    expect(result[3]).toBe(0xbb);
  });

  it("contents returns current buffer data", () => {
    const device = new MTLDevice();
    const buf = device.makeBuffer(4);
    buf.writeBytes(new Uint8Array([1, 2, 3, 4]));
    const c = buf.contents();
    expect(c[0]).toBe(1);
    expect(c[3]).toBe(4);
  });

  it("length returns buffer size", () => {
    const device = new MTLDevice();
    const buf = device.makeBuffer(1024);
    expect(buf.length).toBe(1024);
  });
});

describe("MTLLibrary and MTLFunction", () => {
  it("makeFunction returns an MTLFunction", () => {
    const device = new MTLDevice();
    const lib = device.makeLibrary("shader_source");
    const func = lib.makeFunction("my_kernel");
    expect(func).toBeInstanceOf(MTLFunction);
  });

  it("function name is preserved", () => {
    const device = new MTLDevice();
    const lib = device.makeLibrary("src");
    const func = lib.makeFunction("saxpy");
    expect(func.name).toBe("saxpy");
  });

  it("makeFunction with unknown name still returns function", () => {
    const device = new MTLDevice();
    const lib = device.makeLibrary("src");
    const func = lib.makeFunction("nonexistent");
    expect(func).toBeInstanceOf(MTLFunction);
    expect(func.name).toBe("nonexistent");
  });
});

describe("MTLComputePipelineState", () => {
  it("maxTotalThreadsPerThreadgroup returns 1024", () => {
    const device = new MTLDevice();
    const lib = device.makeLibrary("src");
    const func = lib.makeFunction("kern");
    const pso = device.makeComputePipelineState(func);
    expect(pso.maxTotalThreadsPerThreadgroup).toBe(1024);
  });
});

describe("MTLCommandQueue and MTLCommandBuffer", () => {
  it("makeCommandBuffer returns an MTLCommandBuffer", () => {
    const device = new MTLDevice();
    const queue = device.makeCommandQueue();
    const cb = queue.makeCommandBuffer();
    expect(cb).toBeInstanceOf(MTLCommandBuffer);
  });

  it("command buffer starts as notEnqueued", () => {
    const device = new MTLDevice();
    const queue = device.makeCommandQueue();
    const cb = queue.makeCommandBuffer();
    expect(cb.status).toBe(MTLCommandBufferStatus.notEnqueued);
  });

  it("commit transitions status to completed", () => {
    const device = new MTLDevice();
    const queue = device.makeCommandQueue();
    const cb = queue.makeCommandBuffer();
    cb.commit();
    expect(cb.status).toBe(MTLCommandBufferStatus.completed);
  });

  it("waitUntilCompleted after commit succeeds", () => {
    const device = new MTLDevice();
    const queue = device.makeCommandQueue();
    const cb = queue.makeCommandBuffer();
    cb.commit();
    cb.waitUntilCompleted();
  });

  it("addCompletedHandler fires on commit", () => {
    const device = new MTLDevice();
    const queue = device.makeCommandQueue();
    const cb = queue.makeCommandBuffer();
    let called = false;
    cb.addCompletedHandler(() => { called = true; });
    cb.commit();
    expect(called).toBe(true);
  });
});

describe("MTLComputeCommandEncoder", () => {
  it("creates a compute encoder from command buffer", () => {
    const device = new MTLDevice();
    const queue = device.makeCommandQueue();
    const cb = queue.makeCommandBuffer();
    const encoder = cb.makeComputeCommandEncoder();
    expect(encoder).toBeInstanceOf(MTLComputeCommandEncoder);
  });

  it("dispatchThreadgroups without pipeline throws", () => {
    const device = new MTLDevice();
    const queue = device.makeCommandQueue();
    const cb = queue.makeCommandBuffer();
    const encoder = cb.makeComputeCommandEncoder();
    expect(() =>
      encoder.dispatchThreadgroups(makeMTLSize(1, 1, 1), makeMTLSize(32, 1, 1)),
    ).toThrow("No compute pipeline state set");
  });

  it("dispatchThreadgroups with pipeline succeeds", () => {
    const device = new MTLDevice();
    const queue = device.makeCommandQueue();
    const cb = queue.makeCommandBuffer();
    const encoder = cb.makeComputeCommandEncoder();

    const lib = device.makeLibrary("src");
    const func = lib.makeFunction("kern");
    const pso = device.makeComputePipelineState(func);

    encoder.setComputePipelineState(pso);
    encoder.dispatchThreadgroups(makeMTLSize(4, 1, 1), makeMTLSize(32, 1, 1));
    encoder.endEncoding();
    cb.commit();
  });

  it("setBuffer binds buffers to encoder", () => {
    const device = new MTLDevice();
    const queue = device.makeCommandQueue();
    const cb = queue.makeCommandBuffer();
    const encoder = cb.makeComputeCommandEncoder();

    const buf = device.makeBuffer(64);
    encoder.setBuffer(buf, 0, 0);

    const lib = device.makeLibrary("src");
    const func = lib.makeFunction("kern");
    const pso = device.makeComputePipelineState(func);
    encoder.setComputePipelineState(pso);
    encoder.dispatchThreadgroups(makeMTLSize(1, 1, 1), makeMTLSize(32, 1, 1));
    encoder.endEncoding();
    cb.commit();
  });

  it("setBytes stores push constant data", () => {
    const device = new MTLDevice();
    const queue = device.makeCommandQueue();
    const cb = queue.makeCommandBuffer();
    const encoder = cb.makeComputeCommandEncoder();

    encoder.setBytes(new Uint8Array([1, 2, 3, 4]), 0);
    expect(encoder._pushData.get(0)).toEqual(new Uint8Array([1, 2, 3, 4]));
  });

  it("endEncoding marks encoder as ended", () => {
    const device = new MTLDevice();
    const queue = device.makeCommandQueue();
    const cb = queue.makeCommandBuffer();
    const encoder = cb.makeComputeCommandEncoder();
    expect(encoder.ended).toBe(false);
    encoder.endEncoding();
    expect(encoder.ended).toBe(true);
  });

  it("dispatchThreads calculates grid from total threads", () => {
    const device = new MTLDevice();
    const queue = device.makeCommandQueue();
    const cb = queue.makeCommandBuffer();
    const encoder = cb.makeComputeCommandEncoder();

    const lib = device.makeLibrary("src");
    const func = lib.makeFunction("kern");
    const pso = device.makeComputePipelineState(func);
    encoder.setComputePipelineState(pso);
    encoder.dispatchThreads(makeMTLSize(128, 1, 1), makeMTLSize(32, 1, 1));
    encoder.endEncoding();
    cb.commit();
  });
});

describe("MTLBlitCommandEncoder", () => {
  it("creates a blit encoder", () => {
    const device = new MTLDevice();
    const queue = device.makeCommandQueue();
    const cb = queue.makeCommandBuffer();
    const blit = cb.makeBlitCommandEncoder();
    expect(blit).toBeInstanceOf(MTLBlitCommandEncoder);
  });

  it("copyFromBuffer copies data between buffers", () => {
    const device = new MTLDevice();
    const src = device.makeBuffer(8);
    const dst = device.makeBuffer(8);
    const data = new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8]);
    src.writeBytes(data);

    const queue = device.makeCommandQueue();
    const cb = queue.makeCommandBuffer();
    const blit = cb.makeBlitCommandEncoder();
    blit.copyFromBuffer(src, 0, dst, 0, 8);
    blit.endEncoding();
    cb.commit();

    const result = dst.contents();
    expect(result.slice(0, 8)).toEqual(data);
  });

  it("fillBuffer fills buffer with byte value", () => {
    const device = new MTLDevice();
    const buf = device.makeBuffer(8);

    const queue = device.makeCommandQueue();
    const cb = queue.makeCommandBuffer();
    const blit = cb.makeBlitCommandEncoder();
    blit.fillBuffer(buf, { start: 0, end: 8 }, 0xcc);
    blit.endEncoding();
    cb.commit();

    const result = buf.contents();
    expect(result.slice(0, 8)).toEqual(new Uint8Array(8).fill(0xcc));
  });

  it("endEncoding marks blit encoder as ended", () => {
    const device = new MTLDevice();
    const queue = device.makeCommandQueue();
    const cb = queue.makeCommandBuffer();
    const blit = cb.makeBlitCommandEncoder();
    expect(blit.ended).toBe(false);
    blit.endEncoding();
    expect(blit.ended).toBe(true);
  });
});

describe("makeMTLSize", () => {
  it("creates an MTLSize with correct fields", () => {
    const size = makeMTLSize(64, 32, 8);
    expect(size.width).toBe(64);
    expect(size.height).toBe(32);
    expect(size.depth).toBe(8);
  });
});
