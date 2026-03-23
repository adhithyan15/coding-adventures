/**
 * Tests for ValidationLayer -- error detection.
 */

import { describe, it, expect } from "vitest";
import { halt } from "@coding-adventures/gpu-core";
import {
  RuntimeInstance,
  CommandBuffer,
  CommandBufferState,
  MemoryType,
  BufferUsage,
  ValidationError,
  ValidationLayer,
  makeDescriptorBinding,
  makeBuffer,
} from "../src/index.js";

function makeDevice() {
  const instance = new RuntimeInstance();
  const physical = instance.enumeratePhysicalDevices()[0];
  return instance.createLogicalDevice(physical);
}

describe("CommandBufferValidation", () => {
  it("validateBegin accepts INITIAL", () => {
    const vl = new ValidationLayer();
    const cb = new CommandBuffer();
    expect(() => vl.validateBegin(cb)).not.toThrow();
  });

  it("validateBegin rejects RECORDING", () => {
    const vl = new ValidationLayer();
    const cb = new CommandBuffer();
    cb.begin();
    expect(() => vl.validateBegin(cb)).toThrow(ValidationError);
    expect(() => vl.validateBegin(cb)).toThrow(/recording/);
  });

  it("validateEnd accepts RECORDING", () => {
    const vl = new ValidationLayer();
    const cb = new CommandBuffer();
    cb.begin();
    expect(() => vl.validateEnd(cb)).not.toThrow();
  });

  it("validateEnd rejects INITIAL", () => {
    const vl = new ValidationLayer();
    const cb = new CommandBuffer();
    expect(() => vl.validateEnd(cb)).toThrow(ValidationError);
    expect(() => vl.validateEnd(cb)).toThrow(/initial/);
  });

  it("validateSubmit accepts RECORDED", () => {
    const vl = new ValidationLayer();
    const cb = new CommandBuffer();
    cb.begin();
    cb.end();
    expect(() => vl.validateSubmit(cb)).not.toThrow();
  });

  it("validateSubmit rejects INITIAL", () => {
    const vl = new ValidationLayer();
    const cb = new CommandBuffer();
    expect(() => vl.validateSubmit(cb)).toThrow(ValidationError);
    expect(() => vl.validateSubmit(cb)).toThrow(/initial/);
  });
});

describe("DispatchValidation", () => {
  it("rejects dispatch without pipeline", () => {
    const vl = new ValidationLayer();
    const cb = new CommandBuffer();
    cb.begin();
    expect(() => vl.validateDispatch(cb, 1, 1, 1)).toThrow(ValidationError);
    expect(() => vl.validateDispatch(cb, 1, 1, 1)).toThrow(/no pipeline/);
  });

  it("rejects negative dimensions", () => {
    const vl = new ValidationLayer();
    const device = makeDevice();
    const shader = device.createShaderModule({ code: [halt()] });
    const layout = device.createDescriptorSetLayout([]);
    const plLayout = device.createPipelineLayout([layout]);
    const pipeline = device.createComputePipeline(shader, plLayout);

    const cb = new CommandBuffer();
    cb.begin();
    cb.cmdBindPipeline(pipeline);
    expect(() => vl.validateDispatch(cb, -1, 1, 1)).toThrow(/positive/);
  });

  it("rejects zero dimensions", () => {
    const vl = new ValidationLayer();
    const device = makeDevice();
    const shader = device.createShaderModule({ code: [halt()] });
    const layout = device.createDescriptorSetLayout([]);
    const plLayout = device.createPipelineLayout([layout]);
    const pipeline = device.createComputePipeline(shader, plLayout);

    const cb = new CommandBuffer();
    cb.begin();
    cb.cmdBindPipeline(pipeline);
    expect(() => vl.validateDispatch(cb, 0, 1, 1)).toThrow(/positive/);
  });

  it("accepts valid dispatch", () => {
    const vl = new ValidationLayer();
    const device = makeDevice();
    const shader = device.createShaderModule({ code: [halt()] });
    const layout = device.createDescriptorSetLayout([]);
    const plLayout = device.createPipelineLayout([layout]);
    const pipeline = device.createComputePipeline(shader, plLayout);

    const cb = new CommandBuffer();
    cb.begin();
    cb.cmdBindPipeline(pipeline);
    expect(() => vl.validateDispatch(cb, 4, 2, 1)).not.toThrow();
  });
});

describe("MemoryValidation", () => {
  it("accepts HOST_VISIBLE for map", () => {
    const vl = new ValidationLayer();
    const buf = makeBuffer({
      bufferId: 0,
      size: 64,
      memoryType: MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
      usage: BufferUsage.STORAGE,
    });
    expect(() => vl.validateMap(buf)).not.toThrow();
  });

  it("rejects DEVICE_LOCAL for map", () => {
    const vl = new ValidationLayer();
    const buf = makeBuffer({
      bufferId: 0,
      size: 64,
      memoryType: MemoryType.DEVICE_LOCAL,
      usage: BufferUsage.STORAGE,
    });
    expect(() => vl.validateMap(buf)).toThrow(ValidationError);
    expect(() => vl.validateMap(buf)).toThrow(/HOST_VISIBLE/);
  });

  it("rejects freed buffer for map", () => {
    const vl = new ValidationLayer();
    const buf = makeBuffer({
      bufferId: 0,
      size: 64,
      memoryType: MemoryType.HOST_VISIBLE,
      usage: BufferUsage.STORAGE,
      freed: true,
    });
    expect(() => vl.validateMap(buf)).toThrow(ValidationError);
    expect(() => vl.validateMap(buf)).toThrow(/freed/);
  });

  it("rejects already mapped buffer", () => {
    const vl = new ValidationLayer();
    const buf = makeBuffer({
      bufferId: 0,
      size: 64,
      memoryType: MemoryType.HOST_VISIBLE,
      usage: BufferUsage.STORAGE,
      mapped: true,
    });
    expect(() => vl.validateMap(buf)).toThrow(ValidationError);
    expect(() => vl.validateMap(buf)).toThrow(/already mapped/);
  });

  it("validates buffer usage present", () => {
    const vl = new ValidationLayer();
    const buf = makeBuffer({
      bufferId: 0,
      size: 64,
      memoryType: MemoryType.DEVICE_LOCAL,
      usage: BufferUsage.STORAGE,
    });
    expect(() => vl.validateBufferUsage(buf, BufferUsage.STORAGE)).not.toThrow();
  });

  it("validates buffer usage missing", () => {
    const vl = new ValidationLayer();
    const buf = makeBuffer({
      bufferId: 0,
      size: 64,
      memoryType: MemoryType.DEVICE_LOCAL,
      usage: BufferUsage.STORAGE,
    });
    expect(() => vl.validateBufferUsage(buf, BufferUsage.TRANSFER_SRC)).toThrow(/lacks required/);
  });

  it("validates buffer not freed", () => {
    const vl = new ValidationLayer();
    const buf = makeBuffer({
      bufferId: 0,
      size: 64,
      memoryType: MemoryType.DEVICE_LOCAL,
      usage: BufferUsage.STORAGE,
      freed: true,
    });
    expect(() => vl.validateBufferNotFreed(buf)).toThrow(/freed/);
  });
});

describe("BarrierValidation", () => {
  it("warns on write without barrier", () => {
    const vl = new ValidationLayer();
    vl.recordWrite(42);
    vl.validateReadAfterWrite(42);
    expect(vl.warnings.length).toBe(1);
    expect(vl.warnings[0].toLowerCase()).toContain("barrier");
  });

  it("no warning with global barrier", () => {
    const vl = new ValidationLayer();
    vl.recordWrite(42);
    vl.recordBarrier();
    vl.validateReadAfterWrite(42);
    expect(vl.warnings.length).toBe(0);
  });

  it("no warning for unwritten buffer", () => {
    const vl = new ValidationLayer();
    vl.validateReadAfterWrite(99);
    expect(vl.warnings.length).toBe(0);
  });

  it("barrier covers specific buffers", () => {
    const vl = new ValidationLayer();
    vl.recordWrite(10);
    vl.recordWrite(20);
    vl.recordBarrier(new Set([10]));

    vl.validateReadAfterWrite(10); // OK, barriered
    expect(vl.warnings.length).toBe(0);

    vl.validateReadAfterWrite(20); // Not barriered
    expect(vl.warnings.length).toBe(1);
  });

  it("clear resets state", () => {
    const vl = new ValidationLayer();
    vl.recordWrite(1);
    vl.validateReadAfterWrite(1);
    expect(vl.warnings.length).toBe(1);
    vl.clear();
    expect(vl.warnings.length).toBe(0);
    expect(vl.errors.length).toBe(0);
  });
});

describe("DescriptorSetValidation", () => {
  it("valid descriptor set passes", () => {
    const vl = new ValidationLayer();
    const device = makeDevice();
    const mm = device.memoryManager;
    const buf = mm.allocate(64, MemoryType.DEVICE_LOCAL, BufferUsage.STORAGE);

    const dsLayout = device.createDescriptorSetLayout([makeDescriptorBinding({ binding: 0 })]);
    const descSet = device.createDescriptorSet(dsLayout);
    descSet.write(0, buf);

    const shader = device.createShaderModule({ code: [halt()] });
    const plLayout = device.createPipelineLayout([dsLayout]);
    const pipeline = device.createComputePipeline(shader, plLayout);

    vl.validateDescriptorSet(descSet, pipeline);
    expect(vl.warnings.length).toBe(0);
  });

  it("missing binding warns", () => {
    const vl = new ValidationLayer();
    const device = makeDevice();

    const dsLayout = device.createDescriptorSetLayout([makeDescriptorBinding({ binding: 0 })]);
    const descSet = device.createDescriptorSet(dsLayout);
    // Don't write binding 0

    const shader = device.createShaderModule({ code: [halt()] });
    const plLayout = device.createPipelineLayout([dsLayout]);
    const pipeline = device.createComputePipeline(shader, plLayout);

    vl.validateDescriptorSet(descSet, pipeline);
    expect(vl.warnings.length).toBe(1);
    expect(vl.warnings[0]).toContain("not set");
  });

  it("freed buffer in descriptor throws", () => {
    const vl = new ValidationLayer();
    const device = makeDevice();
    const mm = device.memoryManager;
    const buf = mm.allocate(64, MemoryType.DEVICE_LOCAL, BufferUsage.STORAGE);

    const dsLayout = device.createDescriptorSetLayout([makeDescriptorBinding({ binding: 0 })]);
    const descSet = device.createDescriptorSet(dsLayout);
    descSet.write(0, buf);
    mm.free(buf);

    const shader = device.createShaderModule({ code: [halt()] });
    const plLayout = device.createPipelineLayout([dsLayout]);
    const pipeline = device.createComputePipeline(shader, plLayout);

    expect(() => vl.validateDescriptorSet(descSet, pipeline)).toThrow(ValidationError);
    expect(() => vl.validateDescriptorSet(descSet, pipeline)).toThrow(/freed/);
  });
});
