/**
 * Cross-API integration tests.
 *
 * These tests verify that all six vendor API simulators can coexist and
 * that the underlying compute runtime is shared correctly.
 */
import { describe, it, expect } from "vitest";
import { halt } from "@coding-adventures/gpu-core";

import {
  // CUDA
  CUDARuntime,
  CUDAMemcpyKind,
  makeCUDAKernel,
  makeDim3,
  // OpenCL
  CLPlatform,
  CLContext,
  CLMemFlags,
  // Metal
  MTLDevice,
  MTLCommandBufferStatus,
  makeMTLSize,
  // Vulkan
  VkInstance,
  VkBufferUsageFlagBits,
  VkSharingMode,
  VkResult,
  VkPipelineBindPoint,
  // WebGPU
  GPU,
  GPUBufferUsage,
  GPUMapMode,
  // OpenGL
  GLContext,
  GL_COMPUTE_SHADER,
  GL_SHADER_STORAGE_BUFFER,
  GL_STATIC_DRAW,
  GL_MAP_READ_BIT,
  // Base
  BaseVendorSimulator,
} from "../src/index.js";

describe("Cross-API: All six simulators instantiate independently", () => {
  it("CUDA creates successfully", () => {
    const cuda = new CUDARuntime();
    expect(cuda._physicalDevice).toBeDefined();
  });

  it("OpenCL creates successfully", () => {
    const ctx = new CLContext();
    expect(ctx._devices.length).toBeGreaterThan(0);
  });

  it("Metal creates successfully", () => {
    const device = new MTLDevice();
    expect(device.name.length).toBeGreaterThan(0);
  });

  it("Vulkan creates successfully", () => {
    const instance = new VkInstance();
    const devices = instance.vkEnumeratePhysicalDevices();
    expect(devices.length).toBeGreaterThan(0);
  });

  it("WebGPU creates successfully", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    expect(adapter.name.length).toBeGreaterThan(0);
  });

  it("OpenGL creates successfully", () => {
    const gl = new GLContext();
    expect(gl._logicalDevice).toBeDefined();
  });
});

describe("Cross-API: BaseVendorSimulator shared patterns", () => {
  it("all simulators extend BaseVendorSimulator", () => {
    expect(new CUDARuntime()).toBeInstanceOf(BaseVendorSimulator);
    expect(new CLContext()).toBeInstanceOf(BaseVendorSimulator);
    expect(new MTLDevice()).toBeInstanceOf(BaseVendorSimulator);
    expect(new VkInstance()).toBeInstanceOf(BaseVendorSimulator);
    expect(new GLContext()).toBeInstanceOf(BaseVendorSimulator);
  });

  it("all simulators have physical and logical devices", () => {
    const sims = [
      new CUDARuntime(),
      new CLContext(),
      new MTLDevice(),
      new VkInstance(),
      new GLContext(),
    ];
    for (const sim of sims) {
      expect(sim._physicalDevice).toBeDefined();
      expect(sim._logicalDevice).toBeDefined();
      expect(sim._computeQueue).toBeDefined();
      expect(sim._memoryManager).toBeDefined();
    }
  });
});

describe("Cross-API: Memory write/read patterns", () => {
  it("CUDA: malloc, memcpy, read back", () => {
    const cuda = new CUDARuntime();
    const ptr = cuda.malloc(8);
    const data = new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8]);
    cuda.memcpy(ptr, data, 8, CUDAMemcpyKind.HostToDevice);
    const result = new Uint8Array(8);
    cuda.memcpy(result, ptr, 8, CUDAMemcpyKind.DeviceToHost);
    expect(result).toEqual(data);
    cuda.free(ptr);
  });

  it("Metal: makeBuffer, writeBytes, contents", () => {
    const device = new MTLDevice();
    const buf = device.makeBuffer(8);
    const data = new Uint8Array([10, 20, 30, 40, 50, 60, 70, 80]);
    buf.writeBytes(data);
    const result = buf.contents();
    expect(result.slice(0, 8)).toEqual(data);
  });

  it("WebGPU: createBuffer, writeBuffer, mapAsync, getMappedRange", () => {
    const gpu = new GPU();
    const adapter = gpu.requestAdapter();
    const device = adapter.requestDevice();
    const buffer = device.createBuffer({
      size: 4,
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.MAP_READ,
    });
    const data = new Uint8Array([0xaa, 0xbb, 0xcc, 0xdd]);
    device.queue.writeBuffer(buffer, 0, data);
    buffer.mapAsync(GPUMapMode.READ);
    const result = buffer.getMappedRange();
    expect(result).toEqual(data);
    buffer.unmap();
  });

  it("OpenGL: genBuffers, bufferData, mapBufferRange", () => {
    const gl = new GLContext();
    const [buf] = gl.genBuffers(1);
    gl.bindBuffer(GL_SHADER_STORAGE_BUFFER, buf);
    const data = new Uint8Array([5, 10, 15, 20]);
    gl.bufferData(GL_SHADER_STORAGE_BUFFER, 4, data, GL_STATIC_DRAW);
    const result = gl.mapBufferRange(GL_SHADER_STORAGE_BUFFER, 0, 4, GL_MAP_READ_BIT);
    expect(result).toEqual(data);
  });
});

describe("Cross-API: Compute dispatch patterns", () => {
  it("CUDA: kernel launch", () => {
    const cuda = new CUDARuntime();
    const kernel = makeCUDAKernel([halt()], "test");
    cuda.launchKernel(kernel, makeDim3(1, 1, 1), makeDim3(32, 1, 1));
    cuda.deviceSynchronize();
  });

  it("OpenCL: enqueueNDRangeKernel", () => {
    const ctx = new CLContext();
    const queue = ctx.createCommandQueue();
    const prog = ctx.createProgramWithSource("kernel");
    prog.build();
    const kernel = prog.createKernel("compute");
    queue.enqueueNDRangeKernel(kernel, [32]);
    queue.finish();
  });

  it("Metal: command encoder dispatch", () => {
    const device = new MTLDevice();
    const queue = device.makeCommandQueue();
    const cb = queue.makeCommandBuffer();
    const encoder = cb.makeComputeCommandEncoder();
    const lib = device.makeLibrary("src");
    const func = lib.makeFunction("kern");
    const pso = device.makeComputePipelineState(func);
    encoder.setComputePipelineState(pso);
    encoder.dispatchThreadgroups(makeMTLSize(1, 1, 1), makeMTLSize(32, 1, 1));
    encoder.endEncoding();
    cb.commit();
    expect(cb.status).toBe(MTLCommandBufferStatus.completed);
  });

  it("Vulkan: command buffer dispatch", () => {
    const instance = new VkInstance();
    const physicals = instance.vkEnumeratePhysicalDevices();
    const device = instance.vkCreateDevice(physicals[0]);
    const queue = device.vkGetDeviceQueue(0, 0);
    const pool = device.vkCreateCommandPool({ queueFamilyIndex: 0 });
    const [cb] = pool.vkAllocateCommandBuffers(1);

    const shader = device.vkCreateShaderModule({ code: null });
    const dsLayout = device.vkCreateDescriptorSetLayout({ bindings: [] });
    const plLayout = device.vkCreatePipelineLayout({ setLayouts: [dsLayout], pushConstantSize: 0 });
    const [pipeline] = device.vkCreateComputePipelines([{
      shaderStage: { stage: "compute", module: shader, entryPoint: "main" },
      layout: plLayout,
    }]);
    const fence = device.vkCreateFence();

    cb.vkBeginCommandBuffer();
    cb.vkCmdBindPipeline(VkPipelineBindPoint.COMPUTE, pipeline);
    cb.vkCmdDispatch(4, 1, 1);
    cb.vkEndCommandBuffer();

    const result = queue.vkQueueSubmit(
      [{ commandBuffers: [cb], waitSemaphores: [], signalSemaphores: [] }],
      fence,
    );
    expect(result).toBe(VkResult.SUCCESS);
  });

  it("WebGPU: compute pass dispatch", () => {
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
    device.queue.submit([cmdBuf]);
  });

  it("OpenGL: dispatchCompute", () => {
    const gl = new GLContext();
    const shader = gl.createShader(GL_COMPUTE_SHADER);
    gl.shaderSource(shader, "compute");
    gl.compileShader(shader);
    const prog = gl.createProgram();
    gl.attachShader(prog, shader);
    gl.linkProgram(prog);
    gl.useProgram(prog);
    gl.dispatchCompute(4, 1, 1);
    gl.finish();
  });
});
