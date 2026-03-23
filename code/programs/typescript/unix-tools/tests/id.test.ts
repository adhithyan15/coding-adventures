/**
 * Tests for id -- print real and effective user and group IDs.
 *
 * We test the exported business logic functions: getUserInfo and
 * formatIdDefault. Since these depend on the current user, we verify
 * structure and consistency rather than exact values.
 */

import { describe, it, expect } from "vitest";
import * as os from "node:os";
import { getUserInfo, formatIdDefault, IdInfo } from "../src/id.js";

// ---------------------------------------------------------------------------
// getUserInfo.
// ---------------------------------------------------------------------------

describe("getUserInfo", () => {
  it("should return a non-null object", () => {
    const info = getUserInfo();
    expect(info).toBeTruthy();
  });

  it("should have a numeric uid", () => {
    const info = getUserInfo();
    expect(typeof info.uid).toBe("number");
    expect(info.uid).toBeGreaterThanOrEqual(0);
  });

  it("should have a numeric gid", () => {
    const info = getUserInfo();
    expect(typeof info.gid).toBe("number");
    expect(info.gid).toBeGreaterThanOrEqual(0);
  });

  it("should have a non-empty username", () => {
    const info = getUserInfo();
    expect(info.username).toBeTruthy();
    expect(typeof info.username).toBe("string");
  });

  it("should match os.userInfo().username", () => {
    const info = getUserInfo();
    expect(info.username).toBe(os.userInfo().username);
  });

  it("should have a non-empty groupName", () => {
    const info = getUserInfo();
    expect(info.groupName).toBeTruthy();
    expect(typeof info.groupName).toBe("string");
  });

  it("should have a non-empty groups array", () => {
    const info = getUserInfo();
    expect(Array.isArray(info.groups)).toBe(true);
    expect(info.groups.length).toBeGreaterThan(0);
  });

  it("should have all numeric group IDs", () => {
    const info = getUserInfo();
    for (const gid of info.groups) {
      expect(typeof gid).toBe("number");
    }
  });

  it("should have a non-empty groupNames array", () => {
    const info = getUserInfo();
    expect(Array.isArray(info.groupNames)).toBe(true);
    expect(info.groupNames.length).toBeGreaterThan(0);
  });

  it("should have all string group names", () => {
    const info = getUserInfo();
    for (const name of info.groupNames) {
      expect(typeof name).toBe("string");
      expect(name.length).toBeGreaterThan(0);
    }
  });
});

// ---------------------------------------------------------------------------
// formatIdDefault.
// ---------------------------------------------------------------------------

describe("formatIdDefault", () => {
  const sampleInfo: IdInfo = {
    uid: 501,
    gid: 20,
    username: "testuser",
    groupName: "staff",
    groups: [20, 501],
    groupNames: ["staff", "access_bpf"],
  };

  it("should format the uid part correctly", () => {
    const result = formatIdDefault(sampleInfo);
    expect(result).toContain("uid=501(testuser)");
  });

  it("should format the gid part correctly", () => {
    const result = formatIdDefault(sampleInfo);
    expect(result).toContain("gid=20(staff)");
  });

  it("should format the groups part correctly", () => {
    const result = formatIdDefault(sampleInfo);
    expect(result).toContain("groups=20(staff),501(access_bpf)");
  });

  it("should produce the full expected output", () => {
    const result = formatIdDefault(sampleInfo);
    expect(result).toBe(
      "uid=501(testuser) gid=20(staff) groups=20(staff),501(access_bpf)"
    );
  });

  it("should handle a single group", () => {
    const info: IdInfo = {
      uid: 0,
      gid: 0,
      username: "root",
      groupName: "root",
      groups: [0],
      groupNames: ["root"],
    };
    const result = formatIdDefault(info);
    expect(result).toBe("uid=0(root) gid=0(root) groups=0(root)");
  });

  it("should handle missing group names gracefully", () => {
    const info: IdInfo = {
      uid: 1000,
      gid: 1000,
      username: "user",
      groupName: "user",
      groups: [1000, 2000],
      groupNames: ["user"],
    };
    const result = formatIdDefault(info);
    // Second group should fall back to numeric ID.
    expect(result).toContain("2000(2000)");
  });
});
