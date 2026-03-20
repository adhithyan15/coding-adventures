/**
 * Tests for the barrel export (index.ts) and auto-registration.
 */

import { describe, it, expect } from "vitest";
import {
  createBlas,
  useBackend,
  globalRegistry,
  CpuBlas,
  CudaBlas,
  MetalBlas,
  VulkanBlas,
  OpenClBlas,
  WebGpuBlas,
  OpenGlBlas,
  Vector,
  Matrix,
  Transpose,
  Side,
  StorageOrder,
  fromMatrixPkg,
  toMatrixPkg,
  BackendRegistry,
  GpuBlasBase,
} from "../src/index.js";

describe("Barrel exports", () => {
  // =====================================================================
  // Type exports
  // =====================================================================

  it("should export Vector class", () => {
    const v = new Vector([1, 2, 3], 3);
    expect(v.size).toBe(3);
  });

  it("should export Matrix class", () => {
    const m = new Matrix([1, 2, 3, 4], 2, 2);
    expect(m.rows).toBe(2);
  });

  it("should export StorageOrder enum", () => {
    expect(StorageOrder.ROW_MAJOR).toBeDefined();
    expect(StorageOrder.COLUMN_MAJOR).toBeDefined();
  });

  it("should export Transpose enum", () => {
    expect(Transpose.NO_TRANS).toBeDefined();
    expect(Transpose.TRANS).toBeDefined();
  });

  it("should export Side enum", () => {
    expect(Side.LEFT).toBeDefined();
    expect(Side.RIGHT).toBeDefined();
  });

  it("should export fromMatrixPkg", () => {
    expect(typeof fromMatrixPkg).toBe("function");
  });

  it("should export toMatrixPkg", () => {
    expect(typeof toMatrixPkg).toBe("function");
  });

  // =====================================================================
  // Backend exports
  // =====================================================================

  it("should export CpuBlas", () => {
    const blas = new CpuBlas();
    expect(blas.name).toBe("cpu");
  });

  it("should export CudaBlas", () => {
    const blas = new CudaBlas();
    expect(blas.name).toBe("cuda");
  });

  it("should export MetalBlas", () => {
    const blas = new MetalBlas();
    expect(blas.name).toBe("metal");
  });

  it("should export VulkanBlas", () => {
    const blas = new VulkanBlas();
    expect(blas.name).toBe("vulkan");
  });

  it("should export OpenClBlas", () => {
    const blas = new OpenClBlas();
    expect(blas.name).toBe("opencl");
  });

  it("should export WebGpuBlas", () => {
    const blas = new WebGpuBlas();
    expect(blas.name).toBe("webgpu");
  });

  it("should export OpenGlBlas", () => {
    const blas = new OpenGlBlas();
    expect(blas.name).toBe("opengl");
  });

  it("should export GpuBlasBase", () => {
    expect(GpuBlasBase).toBeDefined();
  });

  // =====================================================================
  // Registry exports
  // =====================================================================

  it("should export BackendRegistry class", () => {
    const reg = new BackendRegistry();
    expect(reg.listAvailable()).toEqual([]);
  });

  it("should export globalRegistry instance", () => {
    expect(globalRegistry).toBeDefined();
    expect(globalRegistry.listAvailable().length).toBeGreaterThan(0);
  });

  // =====================================================================
  // Convenience API exports
  // =====================================================================

  it("should export createBlas", () => {
    const blas = createBlas("cpu");
    expect(blas.name).toBe("cpu");
  });

  it("should export useBackend", () => {
    const blas = useBackend("cpu");
    expect(blas.name).toBe("cpu");
  });

  // =====================================================================
  // Auto-registration: all 7 backends should be registered
  // =====================================================================

  it("should auto-register all 7 backends", () => {
    const available = globalRegistry.listAvailable();
    expect(available).toContain("cpu");
    expect(available).toContain("cuda");
    expect(available).toContain("metal");
    expect(available).toContain("vulkan");
    expect(available).toContain("opencl");
    expect(available).toContain("webgpu");
    expect(available).toContain("opengl");
  });

  it("should be able to create any backend via createBlas", () => {
    const names = ["cpu", "cuda", "metal", "vulkan", "opencl", "webgpu", "opengl"];
    for (const name of names) {
      const blas = createBlas(name);
      expect(blas.name).toBe(name);
    }
  });

  it("auto mode should return a working backend", () => {
    const blas = createBlas("auto");
    expect(blas).toBeDefined();
    // Verify it can actually do math
    const x = new Vector([1, 2, 3], 3);
    const y = new Vector([4, 5, 6], 3);
    const result = blas.saxpy(1.0, x, y);
    expect(result.size).toBe(3);
  });
});
