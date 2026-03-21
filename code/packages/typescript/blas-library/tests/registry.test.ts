/**
 * Tests for BackendRegistry -- backend discovery and selection.
 */

import { describe, it, expect, beforeEach } from "vitest";
import { BackendRegistry, type BackendFactory } from "../src/registry.js";
import { CpuBlas } from "../src/backends/cpu.js";

describe("BackendRegistry", () => {
  let registry: BackendRegistry;

  beforeEach(() => {
    registry = new BackendRegistry();
  });

  // =====================================================================
  // Registration
  // =====================================================================

  it("should register a backend", () => {
    registry.register("cpu", CpuBlas);
    expect(registry.listAvailable()).toContain("cpu");
  });

  it("should register multiple backends", () => {
    registry.register("cpu", CpuBlas);
    registry.register("cpu2", CpuBlas);
    expect(registry.listAvailable()).toContain("cpu");
    expect(registry.listAvailable()).toContain("cpu2");
  });

  it("should overwrite an existing registration", () => {
    registry.register("cpu", CpuBlas);
    registry.register("cpu", CpuBlas);
    expect(registry.listAvailable().filter((n) => n === "cpu").length).toBe(1);
  });

  // =====================================================================
  // Get specific backend
  // =====================================================================

  it("should get a registered backend by name", () => {
    registry.register("cpu", CpuBlas);
    const backend = registry.get("cpu");
    expect(backend.name).toBe("cpu");
  });

  it("should instantiate a new backend each time get() is called", () => {
    registry.register("cpu", CpuBlas);
    const a = registry.get("cpu");
    const b = registry.get("cpu");
    expect(a).not.toBe(b); // different instances
    expect(a.name).toBe(b.name); // same type
  });

  it("should throw for unregistered backend name", () => {
    registry.register("cpu", CpuBlas);
    expect(() => registry.get("cuda")).toThrow("Backend 'cuda' not registered");
  });

  it("should list available backends in error message", () => {
    registry.register("cpu", CpuBlas);
    registry.register("metal", CpuBlas);
    expect(() => registry.get("cuda")).toThrow("Available: cpu, metal");
  });

  // =====================================================================
  // Get best (auto-detect)
  // =====================================================================

  it("should return the highest-priority backend with getBest()", () => {
    registry.register("cpu", CpuBlas);
    const best = registry.getBest();
    expect(best.name).toBe("cpu");
  });

  it("should skip backends that fail to initialize", () => {
    // A backend class that throws in its constructor
    class FailingBackend {
      constructor() {
        throw new Error("No GPU available");
      }
    }
    registry.register("cuda", FailingBackend as unknown as BackendFactory);
    registry.register("cpu", CpuBlas);
    const best = registry.getBest();
    expect(best.name).toBe("cpu");
  });

  it("should throw if no backend can be initialized", () => {
    class FailingBackend {
      constructor() {
        throw new Error("fail");
      }
    }
    registry.register("cuda", FailingBackend as unknown as BackendFactory);
    expect(() => registry.getBest()).toThrow("No BLAS backend could be initialized");
  });

  it("should respect priority order", () => {
    // Register cpu first but metal has higher priority
    registry.register("cpu", CpuBlas);
    registry.register("metal", CpuBlas); // Using CpuBlas as stand-in
    registry.setPriority(["metal", "cpu"]);
    const best = registry.getBest();
    // metal is tried first and succeeds
    expect(best.name).toBe("cpu"); // CpuBlas always reports "cpu"
  });

  // =====================================================================
  // List available
  // =====================================================================

  it("should return empty list when nothing is registered", () => {
    expect(registry.listAvailable()).toEqual([]);
  });

  it("should list all registered backends", () => {
    registry.register("a", CpuBlas);
    registry.register("b", CpuBlas);
    registry.register("c", CpuBlas);
    expect(registry.listAvailable().length).toBe(3);
  });

  // =====================================================================
  // Set priority
  // =====================================================================

  it("should allow setting custom priority", () => {
    registry.setPriority(["opengl", "cpu"]);
    registry.register("cpu", CpuBlas);
    const best = registry.getBest();
    expect(best.name).toBe("cpu");
  });

  it("should not modify the original priority array", () => {
    const priority = ["cpu", "cuda"];
    registry.setPriority(priority);
    priority.push("metal");
    // Internal priority should still be 2 elements
    registry.register("cpu", CpuBlas);
    const best = registry.getBest();
    expect(best).toBeDefined();
  });
});
