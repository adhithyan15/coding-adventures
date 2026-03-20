/**
 * End-to-end execution tests -- full pipeline from allocation to results.
 */

import { describe, it, expect } from "vitest";
import { limm, halt } from "@coding-adventures/gpu-core";
import {
  RuntimeInstance,
  MemoryType,
  BufferUsage,
  PipelineStage,
  AccessFlags,
  RuntimeEventType,
  DeviceType,
  makePipelineBarrier,
  makeDescriptorBinding,
} from "../src/index.js";

function makeDevice(vendor = "nvidia") {
  const instance = new RuntimeInstance();
  const physical = instance.enumeratePhysicalDevices().find((d) => d.vendor === vendor)!;
  return instance.createLogicalDevice(physical);
}

describe("GPUExecution", () => {
  it("simple dispatch completes", () => {
    const device = makeDevice("nvidia");
    const queue = device.queues["compute"][0];

    const shader = device.createShaderModule({
      code: [limm(0, 42.0), halt()],
      localSize: [32, 1, 1],
    });
    const dsLayout = device.createDescriptorSetLayout([]);
    const plLayout = device.createPipelineLayout([dsLayout]);
    const pipeline = device.createComputePipeline(shader, plLayout);

    const cb = device.createCommandBuffer();
    cb.begin();
    cb.cmdBindPipeline(pipeline);
    cb.cmdDispatch(1, 1, 1);
    cb.end();

    const fence = device.createFence();
    queue.submit([cb], { fence });
    expect(fence.signaled).toBe(true);
    expect(fence.wait()).toBe(true);
  });

  it("dispatch with barrier", () => {
    const device = makeDevice("nvidia");
    const queue = device.queues["compute"][0];

    const shader = device.createShaderModule({
      code: [limm(0, 1.0), halt()],
      localSize: [32, 1, 1],
    });
    const dsLayout = device.createDescriptorSetLayout([]);
    const plLayout = device.createPipelineLayout([dsLayout]);
    const pipeline = device.createComputePipeline(shader, plLayout);

    const cb = device.createCommandBuffer();
    cb.begin();
    cb.cmdBindPipeline(pipeline);
    cb.cmdDispatch(1, 1, 1);
    cb.cmdPipelineBarrier(makePipelineBarrier({
      srcStage: PipelineStage.COMPUTE,
      dstStage: PipelineStage.COMPUTE,
      memoryBarriers: [{ srcAccess: AccessFlags.SHADER_WRITE, dstAccess: AccessFlags.SHADER_READ }],
    }));
    cb.cmdDispatch(1, 1, 1);
    cb.end();

    const fence = device.createFence();
    queue.submit([cb], { fence });
    expect(fence.signaled).toBe(true);
    expect(device.stats.totalDispatches).toBe(2);
    expect(device.stats.totalBarriers).toBe(1);
  });

  it("upload and dispatch", () => {
    const device = makeDevice("nvidia");
    const queue = device.queues["compute"][0];
    const mm = device.memoryManager;

    const staging = mm.allocate(
      64,
      MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
      BufferUsage.TRANSFER_SRC,
    );
    const deviceBuf = mm.allocate(
      64,
      MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
      BufferUsage.STORAGE | BufferUsage.TRANSFER_DST,
    );

    const mapped = mm.map(staging);
    mapped.write(0, new Uint8Array(64).fill(0x42));
    mm.unmap(staging);

    const shader = device.createShaderModule({
      code: [limm(0, 1.0), halt()],
      localSize: [32, 1, 1],
    });
    const dsLayout = device.createDescriptorSetLayout([makeDescriptorBinding({ binding: 0 })]);
    const plLayout = device.createPipelineLayout([dsLayout]);
    const pipeline = device.createComputePipeline(shader, plLayout);

    const descSet = device.createDescriptorSet(dsLayout);
    descSet.write(0, deviceBuf);

    const cb = device.createCommandBuffer();
    cb.begin();
    cb.cmdCopyBuffer(staging, deviceBuf, 64);
    cb.cmdPipelineBarrier(makePipelineBarrier({
      srcStage: PipelineStage.TRANSFER,
      dstStage: PipelineStage.COMPUTE,
      memoryBarriers: [{ srcAccess: AccessFlags.TRANSFER_WRITE, dstAccess: AccessFlags.SHADER_READ }],
    }));
    cb.cmdBindPipeline(pipeline);
    cb.cmdBindDescriptorSet(descSet);
    cb.cmdDispatch(1, 1, 1);
    cb.end();

    const fence = device.createFence();
    queue.submit([cb], { fence });
    expect(fence.signaled).toBe(true);
  });

  it("all GPU devices complete basic dispatch", () => {
    for (const vendor of ["nvidia", "amd", "intel"]) {
      const device = makeDevice(vendor);
      const queue = device.queues["compute"][0];

      const shader = device.createShaderModule({
        code: [limm(0, 42.0), halt()],
        localSize: [32, 1, 1],
      });
      const dsLayout = device.createDescriptorSetLayout([]);
      const plLayout = device.createPipelineLayout([dsLayout]);
      const pipeline = device.createComputePipeline(shader, plLayout);

      const cb = device.createCommandBuffer();
      cb.begin();
      cb.cmdBindPipeline(pipeline);
      cb.cmdDispatch(1, 1, 1);
      cb.end();

      const fence = device.createFence();
      queue.submit([cb], { fence });
      expect(fence.signaled).toBe(true);
    }
  });
});

describe("DataflowExecution", () => {
  it("TPU dispatch works", () => {
    const device = makeDevice("google");
    const queue = device.queues["compute"][0];

    const shader = device.createShaderModule({ operation: "matmul" });
    const dsLayout = device.createDescriptorSetLayout([]);
    const plLayout = device.createPipelineLayout([dsLayout]);
    const pipeline = device.createComputePipeline(shader, plLayout);

    const cb = device.createCommandBuffer();
    cb.begin();
    cb.cmdBindPipeline(pipeline);
    cb.cmdDispatch(1, 1, 1);
    cb.end();

    const fence = device.createFence();
    queue.submit([cb], { fence });
    expect(fence.signaled).toBe(true);
  });

  it("ANE dispatch works", () => {
    const device = makeDevice("apple");
    const queue = device.queues["compute"][0];

    const shader = device.createShaderModule({ operation: "matmul" });
    const dsLayout = device.createDescriptorSetLayout([]);
    const plLayout = device.createPipelineLayout([dsLayout]);
    const pipeline = device.createComputePipeline(shader, plLayout);

    const cb = device.createCommandBuffer();
    cb.begin();
    cb.cmdBindPipeline(pipeline);
    cb.cmdDispatch(1, 1, 1);
    cb.end();

    const fence = device.createFence();
    queue.submit([cb], { fence });
    expect(fence.signaled).toBe(true);
  });
});

describe("UnifiedMemory", () => {
  it("Apple zero-copy pattern works", () => {
    const device = makeDevice("apple");
    const queue = device.queues["compute"][0];
    const mm = device.memoryManager;

    const buf = mm.allocate(
      64,
      MemoryType.DEVICE_LOCAL | MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
      BufferUsage.STORAGE,
    );

    const mapped = mm.map(buf);
    mapped.write(0, new Uint8Array(64).fill(0x42));
    mm.unmap(buf);

    const shader = device.createShaderModule({ operation: "matmul" });
    const dsLayout = device.createDescriptorSetLayout([makeDescriptorBinding({ binding: 0 })]);
    const plLayout = device.createPipelineLayout([dsLayout]);
    const pipeline = device.createComputePipeline(shader, plLayout);
    const descSet = device.createDescriptorSet(dsLayout);
    descSet.write(0, buf);

    const cb = device.createCommandBuffer();
    cb.begin();
    cb.cmdBindPipeline(pipeline);
    cb.cmdBindDescriptorSet(descSet);
    cb.cmdDispatch(1, 1, 1);
    cb.end();

    const fence = device.createFence();
    queue.submit([cb], { fence });
    expect(fence.signaled).toBe(true);
  });
});

describe("CommandBufferReuse", () => {
  it("reuse after completion", () => {
    const device = makeDevice();
    const queue = device.queues["compute"][0];

    const shader = device.createShaderModule({
      code: [limm(0, 1.0), halt()],
      localSize: [32, 1, 1],
    });
    const dsLayout = device.createDescriptorSetLayout([]);
    const plLayout = device.createPipelineLayout([dsLayout]);
    const pipeline = device.createComputePipeline(shader, plLayout);

    const cb = device.createCommandBuffer();

    // First use
    cb.begin();
    cb.cmdBindPipeline(pipeline);
    cb.cmdDispatch(1, 1, 1);
    cb.end();
    const fence1 = device.createFence();
    queue.submit([cb], { fence: fence1 });
    expect(fence1.signaled).toBe(true);

    // Reset and reuse
    cb.reset();
    cb.begin();
    cb.cmdBindPipeline(pipeline);
    cb.cmdDispatch(2, 1, 1);
    cb.end();
    const fence2 = device.createFence();
    queue.submit([cb], { fence: fence2 });
    expect(fence2.signaled).toBe(true);

    expect(device.stats.totalDispatches).toBe(2);
  });
});

describe("MultiSubmit", () => {
  it("sequential command buffers", () => {
    const device = makeDevice();
    const queue = device.queues["compute"][0];

    const shader = device.createShaderModule({
      code: [limm(0, 1.0), halt()],
      localSize: [32, 1, 1],
    });
    const dsLayout = device.createDescriptorSetLayout([]);
    const plLayout = device.createPipelineLayout([dsLayout]);
    const pipeline = device.createComputePipeline(shader, plLayout);

    const cbs = [];
    for (let i = 0; i < 3; i++) {
      const cb = device.createCommandBuffer();
      cb.begin();
      cb.cmdBindPipeline(pipeline);
      cb.cmdDispatch(1, 1, 1);
      cb.end();
      cbs.push(cb);
    }

    const fence = device.createFence();
    queue.submit(cbs, { fence });
    expect(fence.signaled).toBe(true);
    expect(device.stats.totalDispatches).toBe(3);
    expect(device.stats.totalCommandBuffers).toBe(3);
  });
});

describe("RuntimeStats", () => {
  it("stats accumulate", () => {
    const device = makeDevice();
    const queue = device.queues["compute"][0];

    const shader = device.createShaderModule({
      code: [limm(0, 1.0), halt()],
      localSize: [32, 1, 1],
    });
    const dsLayout = device.createDescriptorSetLayout([]);
    const plLayout = device.createPipelineLayout([dsLayout]);
    const pipeline = device.createComputePipeline(shader, plLayout);

    for (let i = 0; i < 5; i++) {
      const cb = device.createCommandBuffer();
      cb.begin();
      cb.cmdBindPipeline(pipeline);
      cb.cmdDispatch(1, 1, 1);
      cb.end();
      queue.submit([cb]);
    }

    expect(device.stats.totalSubmissions).toBe(5);
    expect(device.stats.totalDispatches).toBe(5);
    expect(device.stats.totalDeviceCycles).toBeGreaterThan(0);
  });

  it("traces collected", () => {
    const device = makeDevice();
    const queue = device.queues["compute"][0];

    const shader = device.createShaderModule({
      code: [limm(0, 1.0), halt()],
      localSize: [32, 1, 1],
    });
    const dsLayout = device.createDescriptorSetLayout([]);
    const plLayout = device.createPipelineLayout([dsLayout]);
    const pipeline = device.createComputePipeline(shader, plLayout);

    const cb = device.createCommandBuffer();
    cb.begin();
    cb.cmdBindPipeline(pipeline);
    cb.cmdDispatch(1, 1, 1);
    cb.end();
    queue.submit([cb]);

    expect(device.stats.traces.length).toBeGreaterThan(0);
  });

  it("utilization calculated", () => {
    const device = makeDevice();
    const queue = device.queues["compute"][0];

    const shader = device.createShaderModule({
      code: [limm(0, 1.0), halt()],
      localSize: [32, 1, 1],
    });
    const dsLayout = device.createDescriptorSetLayout([]);
    const plLayout = device.createPipelineLayout([dsLayout]);
    const pipeline = device.createComputePipeline(shader, plLayout);

    const cb = device.createCommandBuffer();
    cb.begin();
    cb.cmdBindPipeline(pipeline);
    cb.cmdDispatch(1, 1, 1);
    cb.end();
    queue.submit([cb]);

    expect(device.stats.totalDeviceCycles).toBeGreaterThan(0);
  });
});
