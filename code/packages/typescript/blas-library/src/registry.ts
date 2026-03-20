/**
 * Backend Registry -- find and select BLAS backends.
 *
 * === What is the Registry? ===
 *
 * The registry is a central catalog of available BLAS backends. It provides
 * three modes of selection:
 *
 *     1. EXPLICIT:    registry.get("cuda")     -- give me CUDA specifically
 *     2. AUTO-DETECT: registry.getBest()       -- give me the best available
 *     3. CUSTOM:      registry.register(...)   -- add my own backend
 *
 * === Auto-Detection Priority ===
 *
 * When you ask for "the best available backend," the registry tries each
 * backend in priority order and returns the first one that successfully
 * initializes:
 *
 *     cuda > metal > vulkan > opencl > webgpu > opengl > cpu
 *
 * CUDA is first because it's the most optimized for ML (and most GPUs are
 * NVIDIA in data centers). CPU is always last -- it's the universal fallback
 * that works everywhere.
 *
 * === How It Works Internally ===
 *
 * The registry stores *constructor functions* (not instances). When you call
 * `get("cuda")`, it instantiates a new CudaBlas() on the spot. This is
 * because GPU backends allocate device resources in the constructor, and we
 * don't want to waste GPU memory on backends that aren't being used.
 */

import type { BlasBackend } from "./protocol.js";

/**
 * A factory function (or class constructor) that creates a BlasBackend.
 * The registry stores these and calls them when a backend is requested.
 */
export type BackendFactory = new () => BlasBackend;

/**
 * Backend registry -- find and select BLAS backends.
 *
 * ================================================================
 * BACKEND REGISTRY -- FIND AND SELECT BLAS BACKENDS
 * ================================================================
 *
 * The registry keeps track of which backends are available and
 * helps the caller pick one. Three modes of selection:
 *
 * 1. EXPLICIT:    registry.get("cuda")
 * 2. AUTO-DETECT: registry.getBest()
 * 3. CUSTOM:      registry.register("myBackend", MyBlas)
 *
 * Auto-detection priority (customizable):
 *     cuda > metal > vulkan > opencl > webgpu > opengl > cpu
 *
 * CUDA is first because it's the most optimized for ML.
 * Metal is second because Apple silicon has unified memory.
 * CPU is always last -- it's the universal fallback.
 * ================================================================
 */
export class BackendRegistry {
  /**
   * The default auto-detection order. CUDA first (ML standard),
   * CPU last (universal fallback).
   */
  private static readonly DEFAULT_PRIORITY = [
    "cuda",
    "metal",
    "vulkan",
    "opencl",
    "webgpu",
    "opengl",
    "cpu",
  ];

  /** Registered backend constructors, keyed by name. */
  private _backends: Map<string, BackendFactory> = new Map();

  /** The current priority order for auto-detection. */
  private _priority: string[] = [...BackendRegistry.DEFAULT_PRIORITY];

  /**
   * Register a backend class by name.
   *
   * The class is stored but NOT instantiated yet. Instantiation happens
   * when `get()` or `getBest()` is called.
   *
   * @param name - Backend identifier (e.g., "cuda", "cpu").
   * @param backendClass - The backend constructor to register.
   */
  register(name: string, backendClass: BackendFactory): void {
    this._backends.set(name, backendClass);
  }

  /**
   * Get a specific backend by name, instantiating it on demand.
   *
   * @param name - Backend identifier.
   * @returns An instantiated backend.
   * @throws Error if the backend name is not registered.
   */
  get(name: string): BlasBackend {
    const BackendClass = this._backends.get(name);
    if (!BackendClass) {
      const available = [...this._backends.keys()].sort().join(", ");
      throw new Error(
        `Backend '${name}' not registered. Available: ${available}`
      );
    }
    return new BackendClass();
  }

  /**
   * Try each backend in priority order, return the first that works.
   *
   * Each backend is instantiated inside a try/catch. If initialization
   * fails (e.g., no GPU available), we skip to the next one. CPU always
   * works, so this never fails (as long as CPU is registered).
   *
   * @returns The highest-priority backend that successfully initializes.
   * @throws Error if no backend could be initialized.
   */
  getBest(): BlasBackend {
    for (const name of this._priority) {
      const BackendClass = this._backends.get(name);
      if (BackendClass) {
        try {
          return new BackendClass();
        } catch {
          // This backend failed to initialize -- try the next one.
          // Common reasons: no GPU driver, wrong platform, etc.
          continue;
        }
      }
    }

    const tried = this._priority.filter((n) => this._backends.has(n));
    throw new Error(
      `No BLAS backend could be initialized. Tried: [${tried.join(", ")}]`
    );
  }

  /**
   * List names of all registered backends.
   *
   * @returns An array of registered backend names.
   */
  listAvailable(): string[] {
    return [...this._backends.keys()];
  }

  /**
   * Change the auto-detection priority order.
   *
   * @param priority - New priority list (first = highest priority).
   */
  setPriority(priority: string[]): void {
    this._priority = [...priority];
  }
}

// =========================================================================
// Global registry instance -- shared across the whole application
// =========================================================================

/**
 * This is the single global registry. It's populated by index.ts when the
 * package is imported. Users can also register custom backends here.
 */
export const globalRegistry = new BackendRegistry();
