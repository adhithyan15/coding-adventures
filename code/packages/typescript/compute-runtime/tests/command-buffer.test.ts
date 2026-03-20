/**
 * Tests for CommandBuffer -- recording and state transitions.
 */

import { describe, it, expect } from "vitest";
import { limm, halt } from "@coding-adventures/gpu-core";
import {
  RuntimeInstance,
  CommandBuffer,
  CommandBufferState,
  MemoryType,
  BufferUsage,
  PipelineStage,
  AccessFlags,
  makePipelineBarrier,
  makeDescriptorBinding,
} from "../src/index.js";

function makeDevice() {
  const instance = new RuntimeInstance();
  const physical = instance.enumeratePhysicalDevices()[0];
  return instance.createLogicalDevice(physical);
}

describe("Lifecycle", () => {
  it("starts in INITIAL state", () => {
    const cb = new CommandBuffer();
    expect(cb.state).toBe(CommandBufferState.INITIAL);
  });

  it("transitions to RECORDING on begin()", () => {
    const cb = new CommandBuffer();
    cb.begin();
    expect(cb.state).toBe(CommandBufferState.RECORDING);
  });

  it("transitions to RECORDED on end()", () => {
    const cb = new CommandBuffer();
    cb.begin();
    cb.end();
    expect(cb.state).toBe(CommandBufferState.RECORDED);
  });

  it("transitions to INITIAL on reset()", () => {
    const cb = new CommandBuffer();
    cb.begin();
    cb.end();
    cb.reset();
    expect(cb.state).toBe(CommandBufferState.INITIAL);
  });

  it("begin from RECORDING throws", () => {
    const cb = new CommandBuffer();
    cb.begin();
    expect(() => cb.begin()).toThrow(/recording/);
  });

  it("end from INITIAL throws", () => {
    const cb = new CommandBuffer();
    expect(() => cb.end()).toThrow(/initial/);
  });

  it("record without begin throws", () => {
    const device = makeDevice();
    const shader = device.createShaderModule({ code: [limm(0, 1.0), halt()] });
    const layout = device.createDescriptorSetLayout([]);
    const plLayout = device.createPipelineLayout([layout]);
    const pipeline = device.createComputePipeline(shader, plLayout);

    const cb = new CommandBuffer();
    expect(() => cb.cmdBindPipeline(pipeline)).toThrow(/initial/);
  });

  it("has unique IDs", () => {
    const cb1 = new CommandBuffer();
    const cb2 = new CommandBuffer();
    expect(cb1.commandBufferId).not.toBe(cb2.commandBufferId);
  });

  it("reuse after reset", () => {
    const device = makeDevice();
    const shader = device.createShaderModule({ code: [limm(0, 1.0), halt()] });
    const layout = device.createDescriptorSetLayout([]);
    const plLayout = device.createPipelineLayout([layout]);
    const pipeline = device.createComputePipeline(shader, plLayout);

    const cb = new CommandBuffer();
    cb.begin();
    cb.cmdBindPipeline(pipeline);
    cb.end();
    expect(cb.commands.length).toBe(1);

    cb.reset();
    expect(cb.commands.length).toBe(0);
    expect(cb.state).toBe(CommandBufferState.INITIAL);

    cb.begin();
    cb.cmdBindPipeline(pipeline);
    cb.cmdBindPipeline(pipeline);
    cb.end();
    expect(cb.commands.length).toBe(2);
  });
});

describe("ComputeCommands", () => {
  it("bind pipeline", () => {
    const device = makeDevice();
    const shader = device.createShaderModule({ code: [limm(0, 1.0), halt()] });
    const layout = device.createDescriptorSetLayout([]);
    const plLayout = device.createPipelineLayout([layout]);
    const pipeline = device.createComputePipeline(shader, plLayout);

    const cb = new CommandBuffer();
    cb.begin();
    cb.cmdBindPipeline(pipeline);
    cb.end();
    expect(cb.commands.length).toBe(1);
    expect(cb.commands[0].command).toBe("bind_pipeline");
  });

  it("bind descriptor set", () => {
    const device = makeDevice();
    const layout = device.createDescriptorSetLayout([
      makeDescriptorBinding({ binding: 0, type: "storage" }),
    ]);
    const descSet = device.createDescriptorSet(layout);

    const cb = new CommandBuffer();
    cb.begin();
    cb.cmdBindDescriptorSet(descSet);
    cb.end();
    expect(cb.commands.length).toBe(1);
    expect(cb.commands[0].command).toBe("bind_descriptor_set");
  });

  it("dispatch", () => {
    const device = makeDevice();
    const shader = device.createShaderModule({ code: [limm(0, 1.0), halt()] });
    const layout = device.createDescriptorSetLayout([]);
    const plLayout = device.createPipelineLayout([layout]);
    const pipeline = device.createComputePipeline(shader, plLayout);

    const cb = new CommandBuffer();
    cb.begin();
    cb.cmdBindPipeline(pipeline);
    cb.cmdDispatch(4, 1, 1);
    cb.end();
    expect(cb.commands.length).toBe(2);
    expect(cb.commands[1].command).toBe("dispatch");
    expect(cb.commands[1].args.group_x).toBe(4);
  });

  it("dispatch without pipeline throws", () => {
    const cb = new CommandBuffer();
    cb.begin();
    expect(() => cb.cmdDispatch(1, 1, 1)).toThrow(/no pipeline/);
  });

  it("push constants", () => {
    const cb = new CommandBuffer();
    cb.begin();
    cb.cmdPushConstants(0, new Uint8Array([0x00, 0x00, 0x80, 0x3f])); // 1.0f
    cb.end();
    expect(cb.commands[0].command).toBe("push_constants");
    expect(cb.commands[0].args.size).toBe(4);
  });

  it("dispatch indirect", () => {
    const device = makeDevice();
    const mm = device.memoryManager;
    const buf = mm.allocate(12, MemoryType.DEVICE_LOCAL, BufferUsage.INDIRECT);
    const shader = device.createShaderModule({ code: [limm(0, 1.0), halt()] });
    const layout = device.createDescriptorSetLayout([]);
    const plLayout = device.createPipelineLayout([layout]);
    const pipeline = device.createComputePipeline(shader, plLayout);

    const cb = new CommandBuffer();
    cb.begin();
    cb.cmdBindPipeline(pipeline);
    cb.cmdDispatchIndirect(buf);
    cb.end();
    expect(cb.commands[1].command).toBe("dispatch_indirect");
  });
});

describe("TransferCommands", () => {
  it("copy buffer", () => {
    const device = makeDevice();
    const mm = device.memoryManager;
    const src = mm.allocate(64, MemoryType.DEVICE_LOCAL, BufferUsage.TRANSFER_SRC);
    const dst = mm.allocate(64, MemoryType.DEVICE_LOCAL, BufferUsage.TRANSFER_DST);

    const cb = new CommandBuffer();
    cb.begin();
    cb.cmdCopyBuffer(src, dst, 64);
    cb.end();
    expect(cb.commands[0].command).toBe("copy_buffer");
    expect(cb.commands[0].args.size).toBe(64);
  });

  it("fill buffer", () => {
    const device = makeDevice();
    const mm = device.memoryManager;
    const buf = mm.allocate(64, MemoryType.DEVICE_LOCAL, BufferUsage.TRANSFER_DST);

    const cb = new CommandBuffer();
    cb.begin();
    cb.cmdFillBuffer(buf, 0);
    cb.end();
    expect(cb.commands[0].command).toBe("fill_buffer");
    expect(cb.commands[0].args.value).toBe(0);
  });

  it("update buffer", () => {
    const device = makeDevice();
    const mm = device.memoryManager;
    const buf = mm.allocate(64, MemoryType.DEVICE_LOCAL, BufferUsage.TRANSFER_DST);

    const cb = new CommandBuffer();
    cb.begin();
    cb.cmdUpdateBuffer(buf, 0, new Uint8Array(16).fill(0x42));
    cb.end();
    expect(cb.commands[0].command).toBe("update_buffer");
  });
});

describe("SyncCommands", () => {
  it("pipeline barrier", () => {
    const cb = new CommandBuffer();
    cb.begin();
    cb.cmdPipelineBarrier(makePipelineBarrier({
      srcStage: PipelineStage.COMPUTE,
      dstStage: PipelineStage.TRANSFER,
      memoryBarriers: [{ srcAccess: AccessFlags.SHADER_WRITE, dstAccess: AccessFlags.TRANSFER_READ }],
    }));
    cb.end();
    expect(cb.commands[0].command).toBe("pipeline_barrier");
    expect(cb.commands[0].args.memory_barrier_count).toBe(1);
  });

  it("set event", () => {
    const device = makeDevice();
    const event = device.createEvent();
    const cb = new CommandBuffer();
    cb.begin();
    cb.cmdSetEvent(event, PipelineStage.COMPUTE);
    cb.end();
    expect(cb.commands[0].command).toBe("set_event");
  });

  it("wait event", () => {
    const device = makeDevice();
    const event = device.createEvent();
    const cb = new CommandBuffer();
    cb.begin();
    cb.cmdWaitEvent(event, PipelineStage.COMPUTE, PipelineStage.COMPUTE);
    cb.end();
    expect(cb.commands[0].command).toBe("wait_event");
  });

  it("reset event", () => {
    const device = makeDevice();
    const event = device.createEvent();
    const cb = new CommandBuffer();
    cb.begin();
    cb.cmdResetEvent(event, PipelineStage.COMPUTE);
    cb.end();
    expect(cb.commands[0].command).toBe("reset_event");
  });
});

describe("CommandList", () => {
  it("records multiple commands", () => {
    const device = makeDevice();
    const mm = device.memoryManager;
    const buf = mm.allocate(
      64,
      MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
      BufferUsage.STORAGE | BufferUsage.TRANSFER_DST,
    );
    const shader = device.createShaderModule({ code: [limm(0, 1.0), halt()] });
    const layout = device.createDescriptorSetLayout([makeDescriptorBinding({ binding: 0 })]);
    const plLayout = device.createPipelineLayout([layout]);
    const pipeline = device.createComputePipeline(shader, plLayout);
    const descSet = device.createDescriptorSet(layout);
    descSet.write(0, buf);

    const cb = new CommandBuffer();
    cb.begin();
    cb.cmdBindPipeline(pipeline);
    cb.cmdBindDescriptorSet(descSet);
    cb.cmdDispatch(1, 1, 1);
    cb.cmdPipelineBarrier(makePipelineBarrier({
      srcStage: PipelineStage.COMPUTE,
      dstStage: PipelineStage.TRANSFER,
    }));
    cb.cmdFillBuffer(buf, 0);
    cb.end();

    const commands = cb.commands;
    expect(commands.length).toBe(5);
    expect(commands[0].command).toBe("bind_pipeline");
    expect(commands[1].command).toBe("bind_descriptor_set");
    expect(commands[2].command).toBe("dispatch");
    expect(commands[3].command).toBe("pipeline_barrier");
    expect(commands[4].command).toBe("fill_buffer");
  });
});
