/**
 * Tests for the WebGPU runtime simulator.
 */
import { describe, it, expect } from "vitest";
import { halt } from "@coding-adventures/gpu-core";

import {
  GPU,
  GPUAdapter,
  GPUDevice,
  GPUQueue,
  GPUCommandEncoder,
  GPUComputePassEncoder,
  GPUCommandBuffer,
  GPUBuffer,
  GPUShaderModule,
  GPUComputePipeline,
  GPUBindGroup,
  GPUBindGroupLayout,
  GPUPipelineLayout,
  GPUBufferUsage,
  GPUMapMode,
} from "../src/index.js";

describe("GPU", () => {
  it("creates a GPU entry point", () => {
    const gpu = new GPU();
    expect(gpu).toBeDefined();
  });

  it("requestAdapter returns a GPUAdapter", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    expect(adapter).toBeInstanceOf(GPUAdapter);
  });

  it("requestAdapter with low-power preference", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter({ powerPreference: "low-power" });
    expect(adapter).toBeInstanceOf(GPUAdapter);
  });

  it("requestAdapter with high-performance preference", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter({ powerPreference: "high-performance" });
    expect(adapter).toBeInstanceOf(GPUAdapter);
  });
});

describe("GPUAdapter", () => {
  it("has a name", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    expect(adapter.name.length).toBeGreaterThan(0);
  });

  it("has features set", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    expect(adapter.features).toBeInstanceOf(Set);
    expect(adapter.features.has("compute")).toBe(true);
  });

  it("has limits", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    expect(adapter.limits.maxBufferSize).toBeGreaterThan(0);
    expect(adapter.limits.maxComputeWorkgroupSizeX).toBeGreaterThan(0);
  });

  it("requestDevice returns a GPUDevice", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    const device = adapter.requestDevice();
    expect(device).toBeInstanceOf(GPUDevice);
  });

  it("requestDevice with descriptor", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    const device = adapter.requestDevice({ requiredFeatures: [] });
    expect(device).toBeInstanceOf(GPUDevice);
  });
});

describe("GPUDevice", () => {
  it("has a queue", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    const device = adapter.requestDevice();
    expect(device.queue).toBeInstanceOf(GPUQueue);
  });

  it("has features and limits", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    const device = adapter.requestDevice();
    expect(device.features.has("compute")).toBe(true);
    expect(device.limits.maxBufferSize).toBeGreaterThan(0);
  });

  it("createBuffer returns a GPUBuffer", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    const device = adapter.requestDevice();
    const buffer = device.createBuffer({
      size: 256,
      usage: GPUBufferUsage.STORAGE,
    });
    expect(buffer).toBeInstanceOf(GPUBuffer);
    expect(buffer.size).toBe(256);
  });

  it("createBuffer with mappedAtCreation", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    const device = adapter.requestDevice();
    const buffer = device.createBuffer({
      size: 64,
      usage: GPUBufferUsage.MAP_WRITE | GPUBufferUsage.COPY_SRC,
      mappedAtCreation: true,
    });
    expect(buffer.size).toBe(64);
    const mapped = buffer.getMappedRange();
    expect(mapped).toBeInstanceOf(Uint8Array);
    buffer.unmap();
  });

  it("createShaderModule returns GPUShaderModule", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    const device = adapter.requestDevice();
    const shader = device.createShaderModule({ code: [halt()] });
    expect(shader).toBeInstanceOf(GPUShaderModule);
  });

  it("createShaderModule with non-array code", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    const device = adapter.requestDevice();
    const shader = device.createShaderModule({ code: "wgsl source" });
    expect(shader).toBeInstanceOf(GPUShaderModule);
  });

  it("createComputePipeline returns GPUComputePipeline", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    const device = adapter.requestDevice();
    const shader = device.createShaderModule({ code: [halt()] });
    const pipeline = device.createComputePipeline({
      layout: "auto",
      compute: { module: shader, entryPoint: "main" },
    });
    expect(pipeline).toBeInstanceOf(GPUComputePipeline);
  });

  it("createBindGroupLayout returns GPUBindGroupLayout", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    const device = adapter.requestDevice();
    const layout = device.createBindGroupLayout({
      entries: [
        { binding: 0, visibility: 4, buffer: { type: "storage" } },
      ],
    });
    expect(layout).toBeInstanceOf(GPUBindGroupLayout);
  });

  it("createPipelineLayout returns GPUPipelineLayout", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    const device = adapter.requestDevice();
    const bgLayout = device.createBindGroupLayout({ entries: [] });
    const plLayout = device.createPipelineLayout({
      bindGroupLayouts: [bgLayout],
    });
    expect(plLayout).toBeInstanceOf(GPUPipelineLayout);
  });

  it("createBindGroup returns GPUBindGroup", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    const device = adapter.requestDevice();
    const buffer = device.createBuffer({ size: 64, usage: GPUBufferUsage.STORAGE });
    const bgLayout = device.createBindGroupLayout({
      entries: [{ binding: 0, visibility: 4, buffer: { type: "storage" } }],
    });
    const bindGroup = device.createBindGroup({
      layout: bgLayout,
      entries: [{ binding: 0, resource: buffer }],
    });
    expect(bindGroup).toBeInstanceOf(GPUBindGroup);
  });

  it("createBindGroup with null layout", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    const device = adapter.requestDevice();
    const bindGroup = device.createBindGroup({
      layout: null,
      entries: [],
    });
    expect(bindGroup).toBeInstanceOf(GPUBindGroup);
  });

  it("createCommandEncoder returns GPUCommandEncoder", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    const device = adapter.requestDevice();
    const encoder = device.createCommandEncoder();
    expect(encoder).toBeInstanceOf(GPUCommandEncoder);
  });

  it("createCommandEncoder with label", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    const device = adapter.requestDevice();
    const encoder = device.createCommandEncoder({ label: "test" });
    expect(encoder).toBeInstanceOf(GPUCommandEncoder);
  });

  it("destroy completes without error", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    const device = adapter.requestDevice();
    device.destroy();
  });
});

describe("GPUBuffer", () => {
  it("mapAsync and getMappedRange work together", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    const device = adapter.requestDevice();
    const buffer = device.createBuffer({
      size: 16,
      usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
    });
    buffer.mapAsync(GPUMapMode.READ);
    const data = buffer.getMappedRange();
    expect(data).toBeInstanceOf(Uint8Array);
    expect(data.length).toBe(16);
    buffer.unmap();
  });

  it("mapAsync with offset and size", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    const device = adapter.requestDevice();
    const buffer = device.createBuffer({
      size: 64,
      usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
    });
    buffer.mapAsync(GPUMapMode.READ, 8, 16);
    const data = buffer.getMappedRange();
    expect(data.length).toBe(16);
    buffer.unmap();
  });

  it("getMappedRange without mapping throws", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    const device = adapter.requestDevice();
    const buffer = device.createBuffer({
      size: 16,
      usage: GPUBufferUsage.MAP_READ,
    });
    expect(() => buffer.getMappedRange()).toThrow("not mapped");
  });

  it("unmap without mapping throws", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    const device = adapter.requestDevice();
    const buffer = device.createBuffer({
      size: 16,
      usage: GPUBufferUsage.MAP_READ,
    });
    expect(() => buffer.unmap()).toThrow("not mapped");
  });

  it("destroy prevents further mapping", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    const device = adapter.requestDevice();
    const buffer = device.createBuffer({
      size: 16,
      usage: GPUBufferUsage.MAP_READ,
    });
    buffer.destroy();
    expect(() => buffer.mapAsync(GPUMapMode.READ)).toThrow("destroyed");
  });

  it("usage is preserved", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    const device = adapter.requestDevice();
    const buffer = device.createBuffer({
      size: 32,
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
    });
    expect(buffer.usage).toBe(GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST);
  });
});

describe("GPUComputePipeline", () => {
  it("getBindGroupLayout returns layout at index 0", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    const device = adapter.requestDevice();
    const shader = device.createShaderModule({ code: [halt()] });
    const pipeline = device.createComputePipeline({
      layout: "auto",
      compute: { module: shader },
    });
    const layout = pipeline.getBindGroupLayout(0);
    expect(layout).toBeInstanceOf(GPUBindGroupLayout);
  });

  it("getBindGroupLayout with out-of-range index throws", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    const device = adapter.requestDevice();
    const shader = device.createShaderModule({ code: [halt()] });
    const pipeline = device.createComputePipeline({
      layout: "auto",
      compute: { module: shader },
    });
    expect(() => pipeline.getBindGroupLayout(99)).toThrow("out of range");
  });
});

describe("GPUCommandEncoder and GPUComputePassEncoder", () => {
  it("beginComputePass returns GPUComputePassEncoder", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    const device = adapter.requestDevice();
    const encoder = device.createCommandEncoder();
    const pass = encoder.beginComputePass();
    expect(pass).toBeInstanceOf(GPUComputePassEncoder);
  });

  it("compute pass dispatch workgroups", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    const device = adapter.requestDevice();
    const shader = device.createShaderModule({ code: [halt()] });
    const pipeline = device.createComputePipeline({
      layout: "auto",
      compute: { module: shader },
    });

    const encoder = device.createCommandEncoder();
    const pass = encoder.beginComputePass();
    pass.setPipeline(pipeline);
    pass.dispatchWorkgroups(4, 1, 1);
    pass.end();
    const cmdBuf = encoder.finish();
    expect(cmdBuf).toBeInstanceOf(GPUCommandBuffer);
  });

  it("dispatch without pipeline throws", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    const device = adapter.requestDevice();
    const encoder = device.createCommandEncoder();
    const pass = encoder.beginComputePass();
    expect(() => pass.dispatchWorkgroups(1)).toThrow("No pipeline set");
  });

  it("setBindGroup and dispatch", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    const device = adapter.requestDevice();
    const buffer = device.createBuffer({ size: 64, usage: GPUBufferUsage.STORAGE });
    const shader = device.createShaderModule({ code: [halt()] });
    const pipeline = device.createComputePipeline({
      layout: "auto",
      compute: { module: shader },
    });
    const bgLayout = pipeline.getBindGroupLayout(0);
    const bindGroup = device.createBindGroup({
      layout: bgLayout,
      entries: [{ binding: 0, resource: buffer }],
    });

    const encoder = device.createCommandEncoder();
    const pass = encoder.beginComputePass();
    pass.setPipeline(pipeline);
    pass.setBindGroup(0, bindGroup);
    pass.dispatchWorkgroups(2);
    pass.end();
    const cmdBuf = encoder.finish();
    device.queue.submit([cmdBuf]);
  });

  it("copyBufferToBuffer records copy", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    const device = adapter.requestDevice();
    const src = device.createBuffer({ size: 32, usage: GPUBufferUsage.COPY_SRC | GPUBufferUsage.STORAGE });
    const dst = device.createBuffer({ size: 32, usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.STORAGE });

    const encoder = device.createCommandEncoder();
    encoder.copyBufferToBuffer(src, 0, dst, 0, 32);
    const cmdBuf = encoder.finish();
    device.queue.submit([cmdBuf]);
  });
});

describe("GPUQueue", () => {
  it("submit processes command buffers", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    const device = adapter.requestDevice();
    const encoder = device.createCommandEncoder();
    const cmdBuf = encoder.finish();
    device.queue.submit([cmdBuf]);
  });

  it("writeBuffer writes data to buffer", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    const device = adapter.requestDevice();
    const buffer = device.createBuffer({
      size: 8,
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.MAP_READ,
    });
    const data = new Uint8Array([10, 20, 30, 40, 50, 60, 70, 80]);
    device.queue.writeBuffer(buffer, 0, data);

    buffer.mapAsync(GPUMapMode.READ);
    const result = buffer.getMappedRange();
    expect(result).toEqual(data);
    buffer.unmap();
  });
});
