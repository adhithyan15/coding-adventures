/**
 * Tests for the OpenCL runtime simulator.
 */
import { describe, it, expect } from "vitest";
import { halt } from "@coding-adventures/gpu-core";

import {
  CLPlatform,
  CLDevice,
  CLContext,
  CLCommandQueue,
  CLProgram,
  CLKernel,
  CLBuffer,
  CLEvent,
  CLMemFlags,
  CLDeviceType,
  CLBuildStatus,
  CLEventStatus,
  CLDeviceInfo,
} from "../src/index.js";

describe("CLPlatform", () => {
  it("getPlatforms returns at least one platform", () => {
    const platforms = CLPlatform.getPlatforms();
    expect(platforms.length).toBeGreaterThanOrEqual(1);
  });

  it("platform has name, vendor, version", () => {
    const platform = CLPlatform.getPlatforms()[0];
    expect(platform.name.length).toBeGreaterThan(0);
    expect(platform.vendor.length).toBeGreaterThan(0);
    expect(platform.version.length).toBeGreaterThan(0);
  });

  it("getDevices returns all devices", () => {
    const platform = CLPlatform.getPlatforms()[0];
    const devices = platform.getDevices();
    expect(devices.length).toBeGreaterThan(0);
  });

  it("getDevices filters by type", () => {
    const platform = CLPlatform.getPlatforms()[0];
    const gpus = platform.getDevices(CLDeviceType.GPU);
    for (const d of gpus) {
      expect(d.deviceType).toBe(CLDeviceType.GPU);
    }
  });

  it("getDevices with ALL returns everything", () => {
    const platform = CLPlatform.getPlatforms()[0];
    const all = platform.getDevices(CLDeviceType.ALL);
    expect(all.length).toBeGreaterThan(0);
  });
});

describe("CLDevice", () => {
  it("has name, type, compute units, memory", () => {
    const platform = CLPlatform.getPlatforms()[0];
    const device = platform.getDevices()[0];
    expect(device.name.length).toBeGreaterThan(0);
    expect(device.maxComputeUnits).toBeGreaterThan(0);
    expect(device.maxWorkGroupSize).toBeGreaterThan(0);
    expect(device.globalMemSize).toBeGreaterThan(0);
  });

  it("getInfo returns correct values", () => {
    const platform = CLPlatform.getPlatforms()[0];
    const device = platform.getDevices()[0];
    expect(device.getInfo(CLDeviceInfo.NAME)).toBe(device.name);
    expect(device.getInfo(CLDeviceInfo.TYPE)).toBe(device.deviceType);
    expect(device.getInfo(CLDeviceInfo.MAX_COMPUTE_UNITS)).toBe(device.maxComputeUnits);
    expect(device.getInfo(CLDeviceInfo.MAX_WORK_GROUP_SIZE)).toBe(device.maxWorkGroupSize);
    expect(device.getInfo(CLDeviceInfo.GLOBAL_MEM_SIZE)).toBe(device.globalMemSize);
  });
});

describe("CLContext", () => {
  it("creates a context without devices", () => {
    const ctx = new CLContext();
    expect(ctx._devices.length).toBeGreaterThan(0);
  });

  it("creates a context with specific devices", () => {
    const platform = CLPlatform.getPlatforms()[0];
    const devices = platform.getDevices();
    const ctx = new CLContext([devices[0]]);
    expect(ctx._devices.length).toBe(1);
  });

  it("createBuffer creates a buffer", () => {
    const ctx = new CLContext();
    const buf = ctx.createBuffer(CLMemFlags.READ_WRITE, 256);
    expect(buf).toBeInstanceOf(CLBuffer);
    expect(buf.size).toBe(256);
    expect(buf.flags).toBe(CLMemFlags.READ_WRITE);
  });

  it("createBuffer with COPY_HOST_PTR initializes data", () => {
    const ctx = new CLContext();
    const data = new Uint8Array([1, 2, 3, 4]);
    const buf = ctx.createBuffer(
      CLMemFlags.READ_WRITE | CLMemFlags.COPY_HOST_PTR,
      4,
      data,
    );
    expect(buf.size).toBe(4);
  });

  it("createProgramWithSource creates a program", () => {
    const ctx = new CLContext();
    const prog = ctx.createProgramWithSource("kernel void foo() {}");
    expect(prog).toBeInstanceOf(CLProgram);
  });

  it("createCommandQueue creates a queue", () => {
    const ctx = new CLContext();
    const queue = ctx.createCommandQueue();
    expect(queue).toBeInstanceOf(CLCommandQueue);
  });

  it("createCommandQueue with specific device", () => {
    const ctx = new CLContext();
    const queue = ctx.createCommandQueue(ctx._devices[0]);
    expect(queue).toBeInstanceOf(CLCommandQueue);
  });
});

describe("CLProgram", () => {
  it("initial build status is NONE", () => {
    const ctx = new CLContext();
    const prog = ctx.createProgramWithSource("source");
    expect(prog.buildStatus).toBe(CLBuildStatus.NONE);
  });

  it("build sets status to SUCCESS", () => {
    const ctx = new CLContext();
    const prog = ctx.createProgramWithSource("source");
    prog.build();
    expect(prog.buildStatus).toBe(CLBuildStatus.SUCCESS);
  });

  it("createKernel after build succeeds", () => {
    const ctx = new CLContext();
    const prog = ctx.createProgramWithSource("source");
    prog.build();
    const kernel = prog.createKernel("compute_fn");
    expect(kernel).toBeInstanceOf(CLKernel);
    expect(kernel.name).toBe("compute_fn");
  });

  it("createKernel before build throws", () => {
    const ctx = new CLContext();
    const prog = ctx.createProgramWithSource("source");
    expect(() => prog.createKernel("compute_fn")).toThrow("not built");
  });
});

describe("CLKernel", () => {
  it("setArg stores arguments", () => {
    const ctx = new CLContext();
    const prog = ctx.createProgramWithSource("source");
    prog.build();
    const kernel = prog.createKernel("fn");
    const buf = ctx.createBuffer(CLMemFlags.READ_WRITE, 64);
    kernel.setArg(0, buf);
    kernel.setArg(1, 42);
    expect(kernel._args.get(0)).toBe(buf);
    expect(kernel._args.get(1)).toBe(42);
  });
});

describe("CLCommandQueue", () => {
  it("enqueueWriteBuffer writes data", () => {
    const ctx = new CLContext();
    const queue = ctx.createCommandQueue();
    const buf = ctx.createBuffer(CLMemFlags.READ_WRITE, 8);
    const data = new Uint8Array([10, 20, 30, 40, 50, 60, 70, 80]);
    const ev = queue.enqueueWriteBuffer(buf, 0, 8, data);
    expect(ev).toBeInstanceOf(CLEvent);
    expect(ev.status).toBe(CLEventStatus.COMPLETE);
  });

  it("enqueueReadBuffer reads data", () => {
    const ctx = new CLContext();
    const queue = ctx.createCommandQueue();
    const buf = ctx.createBuffer(CLMemFlags.READ_WRITE, 4);
    const src = new Uint8Array([1, 2, 3, 4]);
    queue.enqueueWriteBuffer(buf, 0, 4, src);
    const dst = new Uint8Array(4);
    queue.enqueueReadBuffer(buf, 0, 4, dst);
    expect(dst).toEqual(src);
  });

  it("enqueueCopyBuffer copies between buffers", () => {
    const ctx = new CLContext();
    const queue = ctx.createCommandQueue();
    const srcBuf = ctx.createBuffer(CLMemFlags.READ_WRITE, 8);
    const dstBuf = ctx.createBuffer(CLMemFlags.READ_WRITE, 8);
    const data = new Uint8Array(8).fill(0x55);
    queue.enqueueWriteBuffer(srcBuf, 0, 8, data);
    queue.enqueueCopyBuffer(srcBuf, dstBuf, 8);
    const result = new Uint8Array(8);
    queue.enqueueReadBuffer(dstBuf, 0, 8, result);
    expect(result).toEqual(data);
  });

  it("enqueueFillBuffer fills with a pattern", () => {
    const ctx = new CLContext();
    const queue = ctx.createCommandQueue();
    const buf = ctx.createBuffer(CLMemFlags.READ_WRITE, 8);
    queue.enqueueFillBuffer(buf, new Uint8Array([0xaa]), 0, 8);
    const result = new Uint8Array(8);
    queue.enqueueReadBuffer(buf, 0, 8, result);
    expect(result).toEqual(new Uint8Array(8).fill(0xaa));
  });

  it("enqueueNDRangeKernel dispatches work", () => {
    const ctx = new CLContext();
    const queue = ctx.createCommandQueue();
    const prog = ctx.createProgramWithSource("kernel");
    prog.build();
    const kernel = prog.createKernel("compute");
    const ev = queue.enqueueNDRangeKernel(kernel, [32]);
    expect(ev).toBeInstanceOf(CLEvent);
  });

  it("enqueueNDRangeKernel with local size", () => {
    const ctx = new CLContext();
    const queue = ctx.createCommandQueue();
    const prog = ctx.createProgramWithSource("kernel");
    prog.build();
    const kernel = prog.createKernel("compute");
    const ev = queue.enqueueNDRangeKernel(kernel, [64], [32]);
    expect(ev).toBeInstanceOf(CLEvent);
  });

  it("enqueueNDRangeKernel with buffer args", () => {
    const ctx = new CLContext();
    const queue = ctx.createCommandQueue();
    const buf = ctx.createBuffer(CLMemFlags.READ_WRITE, 64);
    const prog = ctx.createProgramWithSource("kernel");
    prog.build();
    const kernel = prog.createKernel("compute");
    kernel.setArg(0, buf);
    const ev = queue.enqueueNDRangeKernel(kernel, [32]);
    expect(ev).toBeInstanceOf(CLEvent);
  });

  it("enqueueNDRangeKernel with 2D global size", () => {
    const ctx = new CLContext();
    const queue = ctx.createCommandQueue();
    const prog = ctx.createProgramWithSource("kernel");
    prog.build();
    const kernel = prog.createKernel("compute");
    const ev = queue.enqueueNDRangeKernel(kernel, [64, 64]);
    expect(ev).toBeInstanceOf(CLEvent);
  });

  it("enqueueNDRangeKernel with 3D global size", () => {
    const ctx = new CLContext();
    const queue = ctx.createCommandQueue();
    const prog = ctx.createProgramWithSource("kernel");
    prog.build();
    const kernel = prog.createKernel("compute");
    const ev = queue.enqueueNDRangeKernel(kernel, [32, 32, 4]);
    expect(ev).toBeInstanceOf(CLEvent);
  });

  it("enqueueNDRangeKernel with wait list", () => {
    const ctx = new CLContext();
    const queue = ctx.createCommandQueue();
    const buf = ctx.createBuffer(CLMemFlags.READ_WRITE, 64);
    const writeEv = queue.enqueueWriteBuffer(buf, 0, 4, new Uint8Array([1, 2, 3, 4]));
    const prog = ctx.createProgramWithSource("kernel");
    prog.build();
    const kernel = prog.createKernel("compute");
    kernel.setArg(0, buf);
    const ev = queue.enqueueNDRangeKernel(kernel, [32], undefined, [writeEv]);
    expect(ev).toBeInstanceOf(CLEvent);
  });

  it("finish blocks until complete", () => {
    const ctx = new CLContext();
    const queue = ctx.createCommandQueue();
    queue.finish();
  });

  it("flush is a no-op", () => {
    const ctx = new CLContext();
    const queue = ctx.createCommandQueue();
    queue.flush();
  });

  it("events can be waited on", () => {
    const ctx = new CLContext();
    const queue = ctx.createCommandQueue();
    const buf = ctx.createBuffer(CLMemFlags.READ_WRITE, 4);
    const ev = queue.enqueueWriteBuffer(buf, 0, 4, new Uint8Array([1, 2, 3, 4]));
    ev.wait();
    expect(ev.status).toBe(CLEventStatus.COMPLETE);
  });
});
