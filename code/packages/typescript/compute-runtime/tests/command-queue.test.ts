/**
 * Tests for CommandQueue -- submission and execution.
 */

import { describe, it, expect } from "vitest";
import { limm, halt } from "@coding-adventures/gpu-core";
import {
  RuntimeInstance,
  CommandBufferState,
  MemoryType,
  BufferUsage,
  RuntimeEventType,
  QueueType,
  PipelineStage,
  makePipelineBarrier,
} from "../src/index.js";

function makeDevice(vendor = "nvidia") {
  const instance = new RuntimeInstance();
  const physical = instance.enumeratePhysicalDevices().find((d) => d.vendor === vendor)!;
  return instance.createLogicalDevice(physical);
}

function makePipeline(device: ReturnType<typeof makeDevice>) {
  const shader = device.createShaderModule({ code: [limm(0, 42.0), halt()] });
  const dsLayout = device.createDescriptorSetLayout([]);
  const plLayout = device.createPipelineLayout([dsLayout]);
  return device.createComputePipeline(shader, plLayout);
}

describe("Submit", () => {
  it("basic submit works", () => {
    const device = makeDevice();
    const queue = device.queues["compute"][0];
    const pipeline = makePipeline(device);

    const cb = device.createCommandBuffer();
    cb.begin();
    cb.cmdBindPipeline(pipeline);
    cb.cmdDispatch(1, 1, 1);
    cb.end();

    const fence = device.createFence();
    const traces = queue.submit([cb], { fence });
    expect(fence.signaled).toBe(true);
    expect(cb.state).toBe(CommandBufferState.COMPLETE);
    expect(traces.length).toBeGreaterThan(0);
  });

  it("submit not-recorded throws", () => {
    const device = makeDevice();
    const queue = device.queues["compute"][0];
    const cb = device.createCommandBuffer();
    cb.begin(); // Still RECORDING
    expect(() => queue.submit([cb])).toThrow(/recording/);
  });

  it("fence signaled after submit", () => {
    const device = makeDevice();
    const queue = device.queues["compute"][0];
    const pipeline = makePipeline(device);

    const cb = device.createCommandBuffer();
    cb.begin();
    cb.cmdBindPipeline(pipeline);
    cb.cmdDispatch(1, 1, 1);
    cb.end();

    const fence = device.createFence();
    queue.submit([cb], { fence });
    expect(fence.wait()).toBe(true);
  });

  it("submit without fence", () => {
    const device = makeDevice();
    const queue = device.queues["compute"][0];
    const pipeline = makePipeline(device);

    const cb = device.createCommandBuffer();
    cb.begin();
    cb.cmdBindPipeline(pipeline);
    cb.cmdDispatch(1, 1, 1);
    cb.end();

    const traces = queue.submit([cb]);
    expect(traces.length).toBeGreaterThan(0);
  });

  it("multiple command buffers", () => {
    const device = makeDevice();
    const queue = device.queues["compute"][0];
    const pipeline = makePipeline(device);

    const cb1 = device.createCommandBuffer();
    cb1.begin();
    cb1.cmdBindPipeline(pipeline);
    cb1.cmdDispatch(1, 1, 1);
    cb1.end();

    const cb2 = device.createCommandBuffer();
    cb2.begin();
    cb2.cmdBindPipeline(pipeline);
    cb2.cmdDispatch(2, 1, 1);
    cb2.end();

    const fence = device.createFence();
    queue.submit([cb1, cb2], { fence });
    expect(fence.signaled).toBe(true);
    expect(cb1.state).toBe(CommandBufferState.COMPLETE);
    expect(cb2.state).toBe(CommandBufferState.COMPLETE);
  });

  it("stats updated", () => {
    const device = makeDevice();
    const queue = device.queues["compute"][0];
    const pipeline = makePipeline(device);

    const cb = device.createCommandBuffer();
    cb.begin();
    cb.cmdBindPipeline(pipeline);
    cb.cmdDispatch(1, 1, 1);
    cb.end();

    queue.submit([cb]);
    expect(device.stats.totalSubmissions).toBe(1);
    expect(device.stats.totalCommandBuffers).toBe(1);
    expect(device.stats.totalDispatches).toBe(1);
  });
});

describe("Semaphores", () => {
  it("signal semaphore", () => {
    const device = makeDevice();
    const queue = device.queues["compute"][0];
    const pipeline = makePipeline(device);
    const sem = device.createSemaphore();

    const cb = device.createCommandBuffer();
    cb.begin();
    cb.cmdBindPipeline(pipeline);
    cb.cmdDispatch(1, 1, 1);
    cb.end();

    queue.submit([cb], { signalSemaphores: [sem] });
    expect(sem.signaled).toBe(true);
  });

  it("wait semaphore", () => {
    const device = makeDevice();
    const queue = device.queues["compute"][0];
    const pipeline = makePipeline(device);
    const sem = device.createSemaphore();

    // First submission signals
    const cb1 = device.createCommandBuffer();
    cb1.begin();
    cb1.cmdBindPipeline(pipeline);
    cb1.cmdDispatch(1, 1, 1);
    cb1.end();
    queue.submit([cb1], { signalSemaphores: [sem] });

    // Second waits
    const cb2 = device.createCommandBuffer();
    cb2.begin();
    cb2.cmdBindPipeline(pipeline);
    cb2.cmdDispatch(1, 1, 1);
    cb2.end();
    queue.submit([cb2], { waitSemaphores: [sem] });

    expect(sem.signaled).toBe(false); // Consumed
  });

  it("wait unsignaled throws", () => {
    const device = makeDevice();
    const queue = device.queues["compute"][0];
    const pipeline = makePipeline(device);
    const sem = device.createSemaphore();

    const cb = device.createCommandBuffer();
    cb.begin();
    cb.cmdBindPipeline(pipeline);
    cb.cmdDispatch(1, 1, 1);
    cb.end();

    expect(() => queue.submit([cb], { waitSemaphores: [sem] })).toThrow(/not signaled/);
  });
});

describe("TransferCommands", () => {
  it("copy buffer", () => {
    const device = makeDevice();
    const queue = device.queues["compute"][0];
    const mm = device.memoryManager;

    const src = mm.allocate(64, MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT, BufferUsage.TRANSFER_SRC);
    const dst = mm.allocate(64, MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT, BufferUsage.TRANSFER_DST);

    const mapped = mm.map(src);
    mapped.write(0, new Uint8Array(64).fill(0x42));
    mm.unmap(src);

    const cb = device.createCommandBuffer();
    cb.begin();
    cb.cmdCopyBuffer(src, dst, 64);
    cb.end();

    queue.submit([cb]);
    expect(device.stats.totalTransfers).toBe(1);
  });

  it("fill buffer", () => {
    const device = makeDevice();
    const queue = device.queues["compute"][0];
    const mm = device.memoryManager;
    const buf = mm.allocate(64, MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT, BufferUsage.TRANSFER_DST);

    const cb = device.createCommandBuffer();
    cb.begin();
    cb.cmdFillBuffer(buf, 0xff);
    cb.end();

    queue.submit([cb]);
    expect(device.stats.totalTransfers).toBe(1);
  });

  it("update buffer", () => {
    const device = makeDevice();
    const queue = device.queues["compute"][0];
    const mm = device.memoryManager;
    const buf = mm.allocate(64, MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT, BufferUsage.TRANSFER_DST);

    const cb = device.createCommandBuffer();
    cb.begin();
    cb.cmdUpdateBuffer(buf, 0, new Uint8Array(16).fill(0xaa));
    cb.end();

    queue.submit([cb]);
    expect(device.stats.totalTransfers).toBe(1);
  });
});

describe("Barriers", () => {
  it("barrier recorded in stats", () => {
    const device = makeDevice();
    const queue = device.queues["compute"][0];

    const cb = device.createCommandBuffer();
    cb.begin();
    cb.cmdPipelineBarrier(makePipelineBarrier({
      srcStage: PipelineStage.COMPUTE,
      dstStage: PipelineStage.TRANSFER,
    }));
    cb.end();

    queue.submit([cb]);
    expect(device.stats.totalBarriers).toBe(1);
  });

  it("barrier produces trace", () => {
    const device = makeDevice();
    const queue = device.queues["compute"][0];

    const cb = device.createCommandBuffer();
    cb.begin();
    cb.cmdPipelineBarrier(makePipelineBarrier({
      srcStage: PipelineStage.COMPUTE,
      dstStage: PipelineStage.TRANSFER,
    }));
    cb.end();

    const traces = queue.submit([cb]);
    const barrierTraces = traces.filter((t) => t.eventType === RuntimeEventType.BARRIER);
    expect(barrierTraces.length).toBe(1);
  });
});

describe("QueueProperties", () => {
  it("has correct queue type", () => {
    const device = makeDevice();
    const queue = device.queues["compute"][0];
    expect(queue.queueType).toBe(QueueType.COMPUTE);
  });

  it("waitIdle does not throw", () => {
    const device = makeDevice();
    const queue = device.queues["compute"][0];
    expect(() => queue.waitIdle()).not.toThrow();
  });
});

describe("Traces", () => {
  it("submit produces traces", () => {
    const device = makeDevice();
    const queue = device.queues["compute"][0];
    const pipeline = makePipeline(device);

    const cb = device.createCommandBuffer();
    cb.begin();
    cb.cmdBindPipeline(pipeline);
    cb.cmdDispatch(1, 1, 1);
    cb.end();

    const traces = queue.submit([cb]);
    expect(traces.length).toBeGreaterThan(0);
    const eventTypes = new Set(traces.map((t) => t.eventType));
    expect(eventTypes.has(RuntimeEventType.SUBMIT)).toBe(true);
    expect(eventTypes.has(RuntimeEventType.BEGIN_EXECUTION)).toBe(true);
    expect(eventTypes.has(RuntimeEventType.END_EXECUTION)).toBe(true);
  });

  it("traces have format-compatible data", () => {
    const device = makeDevice();
    const queue = device.queues["compute"][0];
    const pipeline = makePipeline(device);

    const cb = device.createCommandBuffer();
    cb.begin();
    cb.cmdBindPipeline(pipeline);
    cb.cmdDispatch(1, 1, 1);
    cb.end();

    const traces = queue.submit([cb]);
    for (const trace of traces) {
      expect(typeof trace.description).toBe("string");
      expect(trace.description.length).toBeGreaterThan(0);
    }
  });
});
