/**
 * Tests for Pipeline, ShaderModule, DescriptorSet.
 */

import { describe, it, expect } from "vitest";
import { limm, halt } from "@coding-adventures/gpu-core";
import {
  RuntimeInstance,
  MemoryType,
  BufferUsage,
  ShaderModule,
  DescriptorSetLayout,
  PipelineLayout,
  Pipeline,
  DescriptorSet,
  makeDescriptorBinding,
} from "../src/index.js";

function makeDevice() {
  const instance = new RuntimeInstance();
  const physical = instance.enumeratePhysicalDevices()[0];
  return instance.createLogicalDevice(physical);
}

describe("ShaderModule", () => {
  it("creates GPU-style shader", () => {
    const shader = new ShaderModule({ code: [limm(0, 1.0), halt()] });
    expect(shader.isGpuStyle).toBe(true);
    expect(shader.isDataflowStyle).toBe(false);
    expect(shader.code).not.toBeNull();
    expect(shader.code!.length).toBe(2);
  });

  it("creates dataflow-style shader", () => {
    const shader = new ShaderModule({ operation: "matmul" });
    expect(shader.isDataflowStyle).toBe(true);
    expect(shader.isGpuStyle).toBe(false);
    expect(shader.operation).toBe("matmul");
  });

  it("respects local size", () => {
    const shader = new ShaderModule({ code: [halt()], localSize: [256, 1, 1] });
    expect(shader.localSize).toEqual([256, 1, 1]);
  });

  it("respects entry point", () => {
    const shader = new ShaderModule({ code: [halt()], entryPoint: "compute_main" });
    expect(shader.entryPoint).toBe("compute_main");
  });

  it("has unique IDs", () => {
    const s1 = new ShaderModule({ code: [halt()] });
    const s2 = new ShaderModule({ code: [halt()] });
    expect(s1.moduleId).not.toBe(s2.moduleId);
  });

  it("defaults entry point to main", () => {
    const shader = new ShaderModule({ code: [halt()] });
    expect(shader.entryPoint).toBe("main");
  });

  it("defaults local size to (32, 1, 1)", () => {
    const shader = new ShaderModule({ code: [halt()] });
    expect(shader.localSize).toEqual([32, 1, 1]);
  });
});

describe("DescriptorSetLayout", () => {
  it("creates basic layout", () => {
    const layout = new DescriptorSetLayout([
      makeDescriptorBinding({ binding: 0, type: "storage" }),
      makeDescriptorBinding({ binding: 1, type: "storage" }),
    ]);
    expect(layout.bindings.length).toBe(2);
    expect(layout.bindings[0].binding).toBe(0);
    expect(layout.bindings[1].binding).toBe(1);
  });

  it("creates empty layout", () => {
    const layout = new DescriptorSetLayout([]);
    expect(layout.bindings.length).toBe(0);
  });

  it("handles uniform binding", () => {
    const layout = new DescriptorSetLayout([
      makeDescriptorBinding({ binding: 0, type: "uniform" }),
    ]);
    expect(layout.bindings[0].type).toBe("uniform");
  });

  it("has unique IDs", () => {
    const l1 = new DescriptorSetLayout([]);
    const l2 = new DescriptorSetLayout([]);
    expect(l1.layoutId).not.toBe(l2.layoutId);
  });
});

describe("PipelineLayout", () => {
  it("creates with push constants", () => {
    const dsLayout = new DescriptorSetLayout([
      makeDescriptorBinding({ binding: 0, type: "storage" }),
    ]);
    const layout = new PipelineLayout([dsLayout], 16);
    expect(layout.setLayouts.length).toBe(1);
    expect(layout.pushConstantSize).toBe(16);
  });

  it("defaults push constant size to 0", () => {
    const layout = new PipelineLayout([]);
    expect(layout.pushConstantSize).toBe(0);
  });

  it("has unique IDs", () => {
    const l1 = new PipelineLayout([]);
    const l2 = new PipelineLayout([]);
    expect(l1.layoutId).not.toBe(l2.layoutId);
  });
});

describe("Pipeline", () => {
  it("creates pipeline", () => {
    const shader = new ShaderModule({ code: [limm(0, 1.0), halt()] });
    const dsLayout = new DescriptorSetLayout([]);
    const plLayout = new PipelineLayout([dsLayout]);
    const pipeline = new Pipeline(shader, plLayout);
    expect(pipeline.shader).toBe(shader);
    expect(pipeline.layout).toBe(plLayout);
  });

  it("reports workgroup size", () => {
    const shader = new ShaderModule({ code: [halt()], localSize: [128, 2, 1] });
    const plLayout = new PipelineLayout([]);
    const pipeline = new Pipeline(shader, plLayout);
    expect(pipeline.workgroupSize).toEqual([128, 2, 1]);
  });

  it("has unique IDs", () => {
    const shader = new ShaderModule({ code: [halt()] });
    const plLayout = new PipelineLayout([]);
    const p1 = new Pipeline(shader, plLayout);
    const p2 = new Pipeline(shader, plLayout);
    expect(p1.pipelineId).not.toBe(p2.pipelineId);
  });
});

describe("DescriptorSet", () => {
  it("writes and reads buffer", () => {
    const device = makeDevice();
    const mm = device.memoryManager;
    const buf = mm.allocate(64, MemoryType.DEVICE_LOCAL, BufferUsage.STORAGE);
    const layout = new DescriptorSetLayout([makeDescriptorBinding({ binding: 0 })]);
    const descSet = new DescriptorSet(layout);
    descSet.write(0, buf);
    expect(descSet.getBuffer(0)).toBe(buf);
  });

  it("handles multiple bindings", () => {
    const device = makeDevice();
    const mm = device.memoryManager;
    const bufX = mm.allocate(64, MemoryType.DEVICE_LOCAL, BufferUsage.STORAGE);
    const bufY = mm.allocate(64, MemoryType.DEVICE_LOCAL, BufferUsage.STORAGE);
    const layout = new DescriptorSetLayout([
      makeDescriptorBinding({ binding: 0 }),
      makeDescriptorBinding({ binding: 1 }),
    ]);
    const descSet = new DescriptorSet(layout);
    descSet.write(0, bufX);
    descSet.write(1, bufY);
    expect(descSet.getBuffer(0)).toBe(bufX);
    expect(descSet.getBuffer(1)).toBe(bufY);
  });

  it("rejects invalid binding", () => {
    const device = makeDevice();
    const mm = device.memoryManager;
    const buf = mm.allocate(64, MemoryType.DEVICE_LOCAL, BufferUsage.STORAGE);
    const layout = new DescriptorSetLayout([makeDescriptorBinding({ binding: 0 })]);
    const descSet = new DescriptorSet(layout);
    expect(() => descSet.write(99, buf)).toThrow(/not in layout/);
  });

  it("rejects freed buffer", () => {
    const device = makeDevice();
    const mm = device.memoryManager;
    const buf = mm.allocate(64, MemoryType.DEVICE_LOCAL, BufferUsage.STORAGE);
    mm.free(buf);
    const layout = new DescriptorSetLayout([makeDescriptorBinding({ binding: 0 })]);
    const descSet = new DescriptorSet(layout);
    expect(() => descSet.write(0, buf)).toThrow(/freed/);
  });

  it("returns null for unbound binding", () => {
    const layout = new DescriptorSetLayout([makeDescriptorBinding({ binding: 0 })]);
    const descSet = new DescriptorSet(layout);
    expect(descSet.getBuffer(0)).toBeNull();
  });

  it("exposes bindings map", () => {
    const device = makeDevice();
    const mm = device.memoryManager;
    const buf = mm.allocate(64, MemoryType.DEVICE_LOCAL, BufferUsage.STORAGE);
    const layout = new DescriptorSetLayout([makeDescriptorBinding({ binding: 0 })]);
    const descSet = new DescriptorSet(layout);
    descSet.write(0, buf);
    const bindings = descSet.bindings;
    expect(bindings.has(0)).toBe(true);
    expect(bindings.get(0)).toBe(buf);
  });

  it("has unique IDs", () => {
    const layout = new DescriptorSetLayout([]);
    const d1 = new DescriptorSet(layout);
    const d2 = new DescriptorSet(layout);
    expect(d1.setId).not.toBe(d2.setId);
  });
});

describe("DeviceFactory", () => {
  it("creates shader module via device", () => {
    const device = makeDevice();
    const shader = device.createShaderModule({ code: [limm(0, 1.0), halt()], localSize: [64, 1, 1] });
    expect(shader.isGpuStyle).toBe(true);
    expect(shader.localSize).toEqual([64, 1, 1]);
  });

  it("creates dataflow shader via device", () => {
    const device = makeDevice();
    const shader = device.createShaderModule({ operation: "matmul" });
    expect(shader.isDataflowStyle).toBe(true);
  });

  it("creates full pipeline via device", () => {
    const device = makeDevice();
    const shader = device.createShaderModule({ code: [limm(0, 1.0), halt()] });
    const dsLayout = device.createDescriptorSetLayout([makeDescriptorBinding({ binding: 0 })]);
    const plLayout = device.createPipelineLayout([dsLayout], 4);
    const pipeline = device.createComputePipeline(shader, plLayout);
    expect(pipeline.shader).toBe(shader);
    expect(pipeline.layout).toBe(plLayout);
    expect(pipeline.layout.pushConstantSize).toBe(4);
  });
});
