/**
 * forme-errors — CancellationError tests
 */

import { describe, it, expect } from "vitest";
import {
  CancellationError,
  StageError,
  isCancellationError,
} from "../src/index.js";

describe("CancellationError", () => {
  it("is an Error but NOT a StageError (deliberate, FM01 §6.3)", () => {
    const e = new CancellationError("user cancelled");
    expect(e).toBeInstanceOf(Error);
    expect(e).toBeInstanceOf(CancellationError);
    expect(e).not.toBeInstanceOf(StageError);
  });

  it("carries an optional reason", () => {
    const e = new CancellationError("deadline exceeded");
    expect(e.reason).toBe("deadline exceeded");
    expect(e.message).toBe("deadline exceeded");
  });

  it("defaults to a non-null message but a null reason when none provided", () => {
    const e = new CancellationError();
    expect(e.message).toBe("Cancelled");
    expect(e.reason).toBeNull();
  });

  it("sets name to CancellationError", () => {
    expect(new CancellationError().name).toBe("CancellationError");
  });
});

describe("isCancellationError", () => {
  it("returns true for an actual CancellationError instance", () => {
    expect(isCancellationError(new CancellationError())).toBe(true);
  });

  it("returns false for plain Errors", () => {
    expect(isCancellationError(new Error("x"))).toBe(false);
    expect(isCancellationError(new StageError({ code: "X", message: "y" }))).toBe(false);
  });

  it("returns false for non-error values", () => {
    expect(isCancellationError(undefined)).toBe(false);
    expect(isCancellationError(null)).toBe(false);
    expect(isCancellationError("oops")).toBe(false);
    expect(isCancellationError(42)).toBe(false);
    expect(isCancellationError({})).toBe(false);
  });

  it("returns true for cross-realm copies via duck-typing on `name`", () => {
    // Models the scenario where a CancellationError crosses a Worker
    // boundary and the receiving side doesn't have the same class
    // identity but the structured-clone preserved `.name`.
    const ducklike = { name: "CancellationError", message: "x", reason: "x" };
    expect(isCancellationError(ducklike)).toBe(true);
  });
});
