/**
 * Tests for the CUDA runtime simulator.
 */
import { describe, it, expect } from "vitest";
import { limm, halt } from "@coding-adventures/gpu-core";

import {
  CUDARuntime,
  CUDAStream,
  CUDAEvent,
  CUDAMemcpyKind,
  makeCUDAKernel,
  makeDim3,
} from "../src/index.js";

const simpleInstructions = [limm(0, 42.0), halt()];
const nopInstructions = [halt()];

describe("CUDA Device Management", () => {
  it("creates a runtime and selects a device", () => {
    const cuda = new CUDARuntime();
    expect(cuda._physicalDevice).toBeDefined();
    expect(cuda._logicalDevice).toBeDefined();
  });

  it("getDevice returns current device ID", () => {
    const cuda = new CUDARuntime();
    expect(cuda.getDevice()).toBe(0);
  });

  it("setDevice with valid ID succeeds", () => {
    const cuda = new CUDARuntime();
    cuda.setDevice(0);
    expect(cuda.getDevice()).toBe(0);
  });

  it("setDevice with invalid ID throws", () => {
    const cuda = new CUDARuntime();
    expect(() => cuda.setDevice(999)).toThrow("Invalid device ID");
  });

  it("setDevice with negative ID throws", () => {
    const cuda = new CUDARuntime();
    expect(() => cuda.setDevice(-1)).toThrow();
  });

  it("getDeviceProperties returns valid properties", () => {
    const cuda = new CUDARuntime();
    const props = cuda.getDeviceProperties();
    expect(props.name.length).toBeGreaterThan(0);
    expect(props.totalGlobalMem).toBeGreaterThan(0);
    expect(props.maxThreadsPerBlock).toBeGreaterThan(0);
    expect(props.warpSize).toBe(32);
    expect(props.sharedMemPerBlock).toBe(49152);
    expect(props.computeCapability).toEqual([8, 0]);
  });

  it("deviceSynchronize completes without error", () => {
    const cuda = new CUDARuntime();
    cuda.deviceSynchronize();
  });

  it("deviceReset clears streams and events", () => {
    const cuda = new CUDARuntime();
    cuda.createStream();
    cuda.createEvent();
    cuda.deviceReset();
    expect(cuda._streams.length).toBe(0);
    expect(cuda._events.length).toBe(0);
  });

  it("maxGridSize has three dimensions", () => {
    const cuda = new CUDARuntime();
    const props = cuda.getDeviceProperties();
    expect(props.maxGridSize.length).toBe(3);
  });
});

describe("CUDA Memory", () => {
  it("malloc returns a device pointer", () => {
    const cuda = new CUDARuntime();
    const ptr = cuda.malloc(256);
    expect(ptr.size).toBe(256);
    expect(ptr.deviceAddress).toBeGreaterThanOrEqual(0);
    expect(ptr._buffer).toBeDefined();
  });

  it("mallocManaged returns a device pointer", () => {
    const cuda = new CUDARuntime();
    const ptr = cuda.mallocManaged(512);
    expect(ptr.size).toBe(512);
  });

  it("free releases allocated memory", () => {
    const cuda = new CUDARuntime();
    const ptr = cuda.malloc(128);
    cuda.free(ptr);
    expect(ptr._buffer.freed).toBe(true);
  });

  it("double free throws", () => {
    const cuda = new CUDARuntime();
    const ptr = cuda.malloc(128);
    cuda.free(ptr);
    expect(() => cuda.free(ptr)).toThrow();
  });

  it("memcpy HostToDevice transfers data", () => {
    const cuda = new CUDARuntime();
    const ptr = cuda.malloc(16);
    const data = new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
    cuda.memcpy(ptr, data, 16, CUDAMemcpyKind.HostToDevice);
    const result = new Uint8Array(16);
    cuda.memcpy(result, ptr, 16, CUDAMemcpyKind.DeviceToHost);
    expect(result).toEqual(data);
  });

  it("memcpy DeviceToHost transfers data from GPU", () => {
    const cuda = new CUDARuntime();
    const ptr = cuda.malloc(8);
    const data = new Uint8Array([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00, 0x11]);
    cuda.memcpy(ptr, data, 8, CUDAMemcpyKind.HostToDevice);
    const result = new Uint8Array(8);
    cuda.memcpy(result, ptr, 8, CUDAMemcpyKind.DeviceToHost);
    expect(result).toEqual(data);
  });

  it("memcpy DeviceToDevice copies between buffers", () => {
    const cuda = new CUDARuntime();
    const src = cuda.malloc(16);
    const dst = cuda.malloc(16);
    const data = new Uint8Array(16).fill(0x42);
    cuda.memcpy(src, data, 16, CUDAMemcpyKind.HostToDevice);
    cuda.memcpy(dst, src, 16, CUDAMemcpyKind.DeviceToDevice);
    const result = new Uint8Array(16);
    cuda.memcpy(result, dst, 16, CUDAMemcpyKind.DeviceToHost);
    expect(result).toEqual(data);
  });

  it("memcpy HostToHost copies between CPU buffers", () => {
    const cuda = new CUDARuntime();
    const src = new Uint8Array([1, 2, 3, 4]);
    const dst = new Uint8Array(4);
    cuda.memcpy(dst, src, 4, CUDAMemcpyKind.HostToHost);
    expect(dst).toEqual(src);
  });

  it("memcpy HostToDevice with wrong dst type throws TypeError", () => {
    const cuda = new CUDARuntime();
    expect(() =>
      cuda.memcpy(new Uint8Array(4), new Uint8Array(4), 4, CUDAMemcpyKind.HostToDevice),
    ).toThrow(TypeError);
  });

  it("memcpy HostToDevice with wrong src type throws TypeError", () => {
    const cuda = new CUDARuntime();
    const ptr = cuda.malloc(4);
    expect(() =>
      cuda.memcpy(ptr, ptr, 4, CUDAMemcpyKind.HostToDevice),
    ).toThrow(TypeError);
  });

  it("memcpy DeviceToHost with wrong dst type throws TypeError", () => {
    const cuda = new CUDARuntime();
    const ptr = cuda.malloc(4);
    expect(() =>
      cuda.memcpy(ptr, ptr, 4, CUDAMemcpyKind.DeviceToHost),
    ).toThrow(TypeError);
  });

  it("memcpy DeviceToHost with wrong src type throws TypeError", () => {
    const cuda = new CUDARuntime();
    expect(() =>
      cuda.memcpy(new Uint8Array(4), new Uint8Array(1), 4, CUDAMemcpyKind.DeviceToHost),
    ).toThrow(TypeError);
  });

  it("memcpy DeviceToDevice with wrong dst type throws TypeError", () => {
    const cuda = new CUDARuntime();
    const ptr = cuda.malloc(4);
    expect(() =>
      cuda.memcpy(new Uint8Array(4), ptr, 4, CUDAMemcpyKind.DeviceToDevice),
    ).toThrow(TypeError);
  });

  it("memcpy DeviceToDevice with wrong src type throws TypeError", () => {
    const cuda = new CUDARuntime();
    const ptr = cuda.malloc(4);
    expect(() =>
      cuda.memcpy(ptr, new Uint8Array(1), 4, CUDAMemcpyKind.DeviceToDevice),
    ).toThrow(TypeError);
  });

  it("memcpy HostToHost with wrong dst type throws TypeError", () => {
    const cuda = new CUDARuntime();
    const ptr = cuda.malloc(4);
    expect(() =>
      cuda.memcpy(ptr, new Uint8Array(1), 4, CUDAMemcpyKind.HostToHost),
    ).toThrow(TypeError);
  });

  it("memcpy HostToHost with wrong src type throws TypeError", () => {
    const cuda = new CUDARuntime();
    const ptr = cuda.malloc(4);
    expect(() =>
      cuda.memcpy(new Uint8Array(4), ptr, 4, CUDAMemcpyKind.HostToHost),
    ).toThrow(TypeError);
  });

  it("memset fills device memory", () => {
    const cuda = new CUDARuntime();
    const ptr = cuda.malloc(16);
    cuda.memset(ptr, 0xab, 16);
    const result = new Uint8Array(16);
    cuda.memcpy(result, ptr, 16, CUDAMemcpyKind.DeviceToHost);
    expect(result).toEqual(new Uint8Array(16).fill(0xab));
  });
});

describe("CUDA Kernel Launch", () => {
  it("launches a simple kernel", () => {
    const cuda = new CUDARuntime();
    const kernel = makeCUDAKernel(simpleInstructions, "simple");
    cuda.launchKernel(kernel, makeDim3(1, 1, 1), makeDim3(32, 1, 1));
    cuda.deviceSynchronize();
  });

  it("launches with buffer arguments", () => {
    const cuda = new CUDARuntime();
    const d_x = cuda.malloc(128);
    const d_y = cuda.malloc(128);
    const kernel = makeCUDAKernel(simpleInstructions, "with_args");
    cuda.launchKernel(kernel, makeDim3(1, 1, 1), makeDim3(32, 1, 1), [d_x, d_y]);
    cuda.deviceSynchronize();
  });

  it("launches on a non-default stream", () => {
    const cuda = new CUDARuntime();
    const stream = cuda.createStream();
    const kernel = makeCUDAKernel(simpleInstructions, "stream_kernel");
    cuda.launchKernel(kernel, makeDim3(1, 1, 1), makeDim3(32, 1, 1), [], 0, stream);
    cuda.streamSynchronize(stream);
  });

  it("launches with multiple workgroups", () => {
    const cuda = new CUDARuntime();
    const kernel = makeCUDAKernel(simpleInstructions, "multi");
    cuda.launchKernel(kernel, makeDim3(4, 2, 1), makeDim3(32, 1, 1));
    cuda.deviceSynchronize();
  });

  it("dim3 has x, y, z fields", () => {
    const d = makeDim3(4, 2, 1);
    expect(d.x).toBe(4);
    expect(d.y).toBe(2);
    expect(d.z).toBe(1);
  });

  it("kernel stores its name", () => {
    const kernel = makeCUDAKernel([halt()], "test_kernel");
    expect(kernel.name).toBe("test_kernel");
  });

  it("launches with no buffer args", () => {
    const cuda = new CUDARuntime();
    const kernel = makeCUDAKernel(nopInstructions, "no_args");
    cuda.launchKernel(kernel, makeDim3(1, 1, 1), makeDim3(32, 1, 1));
  });
});

describe("CUDA Streams", () => {
  it("createStream returns a CUDAStream", () => {
    const cuda = new CUDARuntime();
    const stream = cuda.createStream();
    expect(stream).toBeInstanceOf(CUDAStream);
  });

  it("destroyStream removes the stream", () => {
    const cuda = new CUDARuntime();
    const stream = cuda.createStream();
    cuda.destroyStream(stream);
    expect(cuda._streams).not.toContain(stream);
  });

  it("destroyStream with invalid stream throws", () => {
    const cuda = new CUDARuntime();
    const stream = cuda.createStream();
    cuda.destroyStream(stream);
    expect(() => cuda.destroyStream(stream)).toThrow();
  });

  it("streamSynchronize completes without error", () => {
    const cuda = new CUDARuntime();
    const stream = cuda.createStream();
    cuda.streamSynchronize(stream);
  });

  it("can create multiple independent streams", () => {
    const cuda = new CUDARuntime();
    const s1 = cuda.createStream();
    const s2 = cuda.createStream();
    expect(s1).not.toBe(s2);
    expect(cuda._streams.length).toBe(2);
  });
});

describe("CUDA Events", () => {
  it("createEvent returns a CUDAEvent", () => {
    const cuda = new CUDARuntime();
    const event = cuda.createEvent();
    expect(event).toBeInstanceOf(CUDAEvent);
  });

  it("recordEvent marks the event as recorded", () => {
    const cuda = new CUDARuntime();
    const event = cuda.createEvent();
    cuda.recordEvent(event);
    expect(event._recorded).toBe(true);
  });

  it("recordEvent on a specific stream", () => {
    const cuda = new CUDARuntime();
    const stream = cuda.createStream();
    const event = cuda.createEvent();
    cuda.recordEvent(event, stream);
    expect(event._recorded).toBe(true);
  });

  it("synchronizeEvent waits for a recorded event", () => {
    const cuda = new CUDARuntime();
    const event = cuda.createEvent();
    cuda.recordEvent(event);
    cuda.synchronizeEvent(event);
  });

  it("synchronizeEvent on unrecorded event throws", () => {
    const cuda = new CUDARuntime();
    const event = cuda.createEvent();
    expect(() => cuda.synchronizeEvent(event)).toThrow("never recorded");
  });

  it("elapsedTime returns a number >= 0", () => {
    const cuda = new CUDARuntime();
    const start = cuda.createEvent();
    const end = cuda.createEvent();
    cuda.recordEvent(start);
    cuda.recordEvent(end);
    const elapsed = cuda.elapsedTime(start, end);
    expect(typeof elapsed).toBe("number");
    expect(elapsed).toBeGreaterThanOrEqual(0.0);
  });

  it("elapsedTime with unrecorded start throws", () => {
    const cuda = new CUDARuntime();
    const start = cuda.createEvent();
    const end = cuda.createEvent();
    cuda.recordEvent(end);
    expect(() => cuda.elapsedTime(start, end)).toThrow("Start event");
  });

  it("elapsedTime with unrecorded end throws", () => {
    const cuda = new CUDARuntime();
    const start = cuda.createEvent();
    const end = cuda.createEvent();
    cuda.recordEvent(start);
    expect(() => cuda.elapsedTime(start, end)).toThrow("End event");
  });
});
