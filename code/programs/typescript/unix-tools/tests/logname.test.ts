/**
 * Tests for logname -- print the user's login name.
 *
 * We test the exported `getLoginName` function, which retrieves the
 * login name from environment variables (LOGNAME, then USER fallback).
 */

import { describe, it, expect, afterEach } from "vitest";
import { getLoginName } from "../src/logname.js";

describe("getLoginName", () => {
  // Save original env values so we can restore them after each test.
  const originalLogname = process.env.LOGNAME;
  const originalUser = process.env.USER;

  afterEach(() => {
    // Restore original environment.
    if (originalLogname !== undefined) {
      process.env.LOGNAME = originalLogname;
    } else {
      delete process.env.LOGNAME;
    }
    if (originalUser !== undefined) {
      process.env.USER = originalUser;
    } else {
      delete process.env.USER;
    }
  });

  // -------------------------------------------------------------------------
  // Normal operation
  // -------------------------------------------------------------------------

  it("should return a non-empty string in normal environment", () => {
    const loginName = getLoginName();
    expect(loginName).toBeTruthy();
    expect(typeof loginName).toBe("string");
  });

  // -------------------------------------------------------------------------
  // LOGNAME takes precedence
  // -------------------------------------------------------------------------

  it("should prefer LOGNAME over USER", () => {
    process.env.LOGNAME = "alice";
    process.env.USER = "bob";

    const result = getLoginName();
    expect(result).toBe("alice");
  });

  // -------------------------------------------------------------------------
  // Fallback to USER
  // -------------------------------------------------------------------------

  it("should fall back to USER when LOGNAME is not set", () => {
    delete process.env.LOGNAME;
    process.env.USER = "charlie";

    const result = getLoginName();
    expect(result).toBe("charlie");
  });

  // -------------------------------------------------------------------------
  // Both unset
  // -------------------------------------------------------------------------

  it("should return null when both LOGNAME and USER are unset", () => {
    delete process.env.LOGNAME;
    delete process.env.USER;

    const result = getLoginName();
    expect(result).toBeNull();
  });

  // -------------------------------------------------------------------------
  // Edge cases
  // -------------------------------------------------------------------------

  it("should return LOGNAME even if it is an empty string", () => {
    // An empty LOGNAME is truthy in the ?? check, so it should be returned.
    // Actually, empty string is falsy for ?? -- no, ?? only checks null/undefined.
    process.env.LOGNAME = "";
    process.env.USER = "dave";

    const result = getLoginName();
    expect(result).toBe("");
  });

  it("should handle LOGNAME with special characters", () => {
    process.env.LOGNAME = "user-with-dashes_and_underscores";

    const result = getLoginName();
    expect(result).toBe("user-with-dashes_and_underscores");
  });
});
