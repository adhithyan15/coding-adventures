/**
 * Tests for the convenience API: createBlas() and useBackend().
 */

import { describe, it, expect } from "vitest";
import { createBlas, useBackend } from "../src/convenience.js";
import { globalRegistry } from "../src/registry.js";
import { CpuBlas } from "../src/backends/cpu.js";

// Ensure CPU is registered for these tests
globalRegistry.register("cpu", CpuBlas);

describe("createBlas", () => {
  it("should create a CPU backend by name", () => {
    const blas = createBlas("cpu");
    expect(blas.name).toBe("cpu");
  });

  it("should create auto backend (falls through to cpu)", () => {
    const blas = createBlas("auto");
    expect(blas).toBeDefined();
    expect(blas.name).toBeDefined();
  });

  it("should default to auto when no argument is given", () => {
    const blas = createBlas();
    expect(blas).toBeDefined();
  });

  it("should throw for unknown backend", () => {
    expect(() => createBlas("nonexistent")).toThrow();
  });
});

describe("useBackend", () => {
  it("should return a backend for temporary use", () => {
    const blas = useBackend("cpu");
    expect(blas.name).toBe("cpu");
  });

  it("should create a fresh backend each time", () => {
    const a = useBackend("cpu");
    const b = useBackend("cpu");
    expect(a).not.toBe(b);
  });
});
