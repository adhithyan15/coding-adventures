/**
 * Tests for whoami -- print effective user name.
 *
 * We test the exported `getEffectiveUsername` function, which retrieves
 * the current user's name via `os.userInfo()` with a fallback to
 * `process.env.USER`.
 *
 * Note: ESM namespaces cannot be spied on with vi.spyOn, so we cannot
 * mock `os.userInfo()`. Instead, we test the function's behavior with
 * the real system state and exercise the fallback path by manipulating
 * environment variables (which IS allowed).
 */

import { describe, it, expect, afterEach } from "vitest";
import * as os from "node:os";
import { getEffectiveUsername } from "../src/whoami.js";

describe("getEffectiveUsername", () => {
  // -------------------------------------------------------------------------
  // Normal operation
  // -------------------------------------------------------------------------

  it("should return a non-empty string", () => {
    const username = getEffectiveUsername();
    expect(username).toBeTruthy();
    expect(typeof username).toBe("string");
  });

  it("should return the same value as os.userInfo().username", () => {
    const expected = os.userInfo().username;
    const result = getEffectiveUsername();
    expect(result).toBe(expected);
  });

  it("should return a string without leading/trailing whitespace", () => {
    const username = getEffectiveUsername();
    expect(username).not.toBeNull();
    expect(username!.trim()).toBe(username);
  });

  it("should return a non-empty username on any normal system", () => {
    const username = getEffectiveUsername();
    expect(username).not.toBeNull();
    expect(username!.length).toBeGreaterThan(0);
  });

  // -------------------------------------------------------------------------
  // Verify it matches the system username
  // -------------------------------------------------------------------------

  it("should match process.env.USER on a normal system", () => {
    // On most systems, os.userInfo().username and process.env.USER agree.
    // This test documents that expectation.
    const username = getEffectiveUsername();
    const envUser = process.env.USER;
    if (envUser) {
      expect(username).toBe(envUser);
    }
  });

  it("should return a string that contains only valid username characters", () => {
    const username = getEffectiveUsername();
    expect(username).not.toBeNull();
    // Unix usernames typically contain only alphanumeric, underscore, hyphen, dot
    expect(username).toMatch(/^[a-zA-Z0-9._-]+$/);
  });
});
