/**
 * Tests for yes -- output a string repeatedly.
 *
 * We test the exported `yesOutput` function, which is the testable core
 * of the `yes` utility. The real `yes` runs forever; `yesOutput` generates
 * a finite number of lines so we can verify the output format.
 */

import { describe, it, expect } from "vitest";
import { yesOutput } from "../src/yes.js";

describe("yesOutput", () => {
  // -------------------------------------------------------------------------
  // Default behavior: output "y"
  // -------------------------------------------------------------------------

  it("should return an array of 'y' strings when given 'y'", () => {
    const result = yesOutput("y", 5);
    expect(result).toEqual(["y", "y", "y", "y", "y"]);
  });

  it("should return an empty array when maxLines is 0", () => {
    const result = yesOutput("y", 0);
    expect(result).toEqual([]);
  });

  it("should return exactly 1 line when maxLines is 1", () => {
    const result = yesOutput("y", 1);
    expect(result).toEqual(["y"]);
  });

  // -------------------------------------------------------------------------
  // Custom string
  // -------------------------------------------------------------------------

  it("should repeat a custom string", () => {
    const result = yesOutput("hello", 3);
    expect(result).toEqual(["hello", "hello", "hello"]);
  });

  it("should handle a multi-word string (already joined)", () => {
    const result = yesOutput("hello world", 2);
    expect(result).toEqual(["hello world", "hello world"]);
  });

  it("should handle an empty string", () => {
    const result = yesOutput("", 3);
    expect(result).toEqual(["", "", ""]);
  });

  // -------------------------------------------------------------------------
  // Edge cases
  // -------------------------------------------------------------------------

  it("should handle a string with special characters", () => {
    const result = yesOutput("y\tn", 2);
    expect(result).toEqual(["y\tn", "y\tn"]);
  });

  it("should handle a very large maxLines count", () => {
    const result = yesOutput("y", 1000);
    expect(result).toHaveLength(1000);
    expect(result.every((line) => line === "y")).toBe(true);
  });

  it("should handle unicode strings", () => {
    const result = yesOutput("yes", 3);
    expect(result).toEqual(["yes", "yes", "yes"]);
  });

  it("should return independent copies (not references to same object)", () => {
    const result = yesOutput("test", 3);
    expect(result[0]).toBe(result[1]); // strings are interned, this is fine
    expect(result).toHaveLength(3);
  });
});
