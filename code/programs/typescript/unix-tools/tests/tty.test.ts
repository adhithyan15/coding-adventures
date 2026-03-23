/**
 * Tests for tty -- print the terminal name.
 *
 * We test the exported `getTtyInfo` function, which checks whether stdin
 * is connected to a terminal and returns the appropriate information.
 *
 * Note: In a test environment, stdin is typically NOT a TTY (it's piped
 * from the test runner). So we primarily test the "not a tty" path
 * directly, and test the TTY path by mocking.
 */

import { describe, it, expect, vi, afterEach } from "vitest";
import { getTtyInfo } from "../src/tty.js";

describe("getTtyInfo", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  // -------------------------------------------------------------------------
  // Non-TTY case (normal in test environment)
  // -------------------------------------------------------------------------

  it("should return isTTY: false when stdin is not a terminal", () => {
    // In the test runner, stdin is typically piped, so isTTY is undefined/false.
    // We mock it explicitly to be sure.
    const originalIsTTY = process.stdin.isTTY;
    Object.defineProperty(process.stdin, "isTTY", {
      value: undefined,
      writable: true,
      configurable: true,
    });

    try {
      const info = getTtyInfo();
      expect(info.isTTY).toBe(false);
      expect(info.name).toBe("not a tty");
    } finally {
      Object.defineProperty(process.stdin, "isTTY", {
        value: originalIsTTY,
        writable: true,
        configurable: true,
      });
    }
  });

  // -------------------------------------------------------------------------
  // TTY case (mocked)
  // -------------------------------------------------------------------------

  it("should return isTTY: true when stdin is a terminal", () => {
    const originalIsTTY = process.stdin.isTTY;
    Object.defineProperty(process.stdin, "isTTY", {
      value: true,
      writable: true,
      configurable: true,
    });

    try {
      const info = getTtyInfo();
      expect(info.isTTY).toBe(true);
      expect(info.name).toBe("/dev/tty");
    } finally {
      Object.defineProperty(process.stdin, "isTTY", {
        value: originalIsTTY,
        writable: true,
        configurable: true,
      });
    }
  });

  // -------------------------------------------------------------------------
  // Return value structure
  // -------------------------------------------------------------------------

  it("should return an object with isTTY and name properties", () => {
    const info = getTtyInfo();
    expect(info).toHaveProperty("isTTY");
    expect(info).toHaveProperty("name");
    expect(typeof info.isTTY).toBe("boolean");
    expect(typeof info.name).toBe("string");
  });

  // -------------------------------------------------------------------------
  // Name values
  // -------------------------------------------------------------------------

  it("should return 'not a tty' as the name when not a terminal", () => {
    const originalIsTTY = process.stdin.isTTY;
    Object.defineProperty(process.stdin, "isTTY", {
      value: false,
      writable: true,
      configurable: true,
    });

    try {
      const info = getTtyInfo();
      expect(info.name).toBe("not a tty");
    } finally {
      Object.defineProperty(process.stdin, "isTTY", {
        value: originalIsTTY,
        writable: true,
        configurable: true,
      });
    }
  });

  it("should return a device path when stdin is a terminal", () => {
    const originalIsTTY = process.stdin.isTTY;
    Object.defineProperty(process.stdin, "isTTY", {
      value: true,
      writable: true,
      configurable: true,
    });

    try {
      const info = getTtyInfo();
      // The device path should start with /dev/
      expect(info.name).toMatch(/^\/dev\//);
    } finally {
      Object.defineProperty(process.stdin, "isTTY", {
        value: originalIsTTY,
        writable: true,
        configurable: true,
      });
    }
  });
});
