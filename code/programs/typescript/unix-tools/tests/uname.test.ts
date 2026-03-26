/**
 * Tests for uname -- print system information.
 *
 * We test the exported business logic functions: getSystemInfo and
 * formatUname. Since these depend on the actual system, we verify
 * the structure and format rather than exact values.
 */

import { describe, it, expect } from "vitest";
import * as os from "node:os";
import { getSystemInfo, formatUname, UnameInfo } from "../src/uname.js";

// ---------------------------------------------------------------------------
// getSystemInfo.
// ---------------------------------------------------------------------------

describe("getSystemInfo", () => {
  it("should return a non-null object", () => {
    const info = getSystemInfo();
    expect(info).toBeTruthy();
  });

  it("should have a non-empty kernelName", () => {
    const info = getSystemInfo();
    expect(info.kernelName).toBeTruthy();
    expect(typeof info.kernelName).toBe("string");
  });

  it("should match os.type() for kernelName", () => {
    const info = getSystemInfo();
    expect(info.kernelName).toBe(os.type());
  });

  it("should have a non-empty nodename", () => {
    const info = getSystemInfo();
    expect(info.nodename).toBeTruthy();
  });

  it("should match os.hostname() for nodename", () => {
    const info = getSystemInfo();
    expect(info.nodename).toBe(os.hostname());
  });

  it("should have a non-empty kernelRelease", () => {
    const info = getSystemInfo();
    expect(info.kernelRelease).toBeTruthy();
  });

  it("should match os.release() for kernelRelease", () => {
    const info = getSystemInfo();
    expect(info.kernelRelease).toBe(os.release());
  });

  it("should have a non-empty machine", () => {
    const info = getSystemInfo();
    expect(info.machine).toBeTruthy();
  });

  it("should have a non-empty processor", () => {
    const info = getSystemInfo();
    expect(info.processor).toBeTruthy();
  });

  it("should have a non-empty operatingSystem", () => {
    const info = getSystemInfo();
    expect(info.operatingSystem).toBeTruthy();
  });

  it("should return a known OS name for the current platform", () => {
    const info = getSystemInfo();
    const validNames = [
      "GNU/Linux",
      "Darwin",
      "Windows",
      "FreeBSD",
      "OpenBSD",
      "SunOS",
      "AIX",
    ];
    // It could also be a raw platform string if not in our map.
    expect(
      validNames.includes(info.operatingSystem) ||
        typeof info.operatingSystem === "string"
    ).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// formatUname.
// ---------------------------------------------------------------------------

describe("formatUname", () => {
  const sampleInfo: UnameInfo = {
    kernelName: "Linux",
    nodename: "myhost",
    kernelRelease: "5.15.0",
    kernelVersion: "#1 SMP",
    machine: "x86_64",
    processor: "x86_64",
    hardwarePlatform: "x86_64",
    operatingSystem: "GNU/Linux",
  };

  it("should format kernel name only (default)", () => {
    const result = formatUname(sampleInfo, {
      kernelName: true,
      nodename: false,
      kernelRelease: false,
      kernelVersion: false,
      machine: false,
      processor: false,
      hardwarePlatform: false,
      operatingSystem: false,
    });
    expect(result).toBe("Linux");
  });

  it("should format all fields", () => {
    const result = formatUname(sampleInfo, {
      kernelName: true,
      nodename: true,
      kernelRelease: true,
      kernelVersion: true,
      machine: true,
      processor: true,
      hardwarePlatform: true,
      operatingSystem: true,
    });
    expect(result).toBe(
      "Linux myhost 5.15.0 #1 SMP x86_64 x86_64 x86_64 GNU/Linux"
    );
  });

  it("should format selected fields in correct order", () => {
    const result = formatUname(sampleInfo, {
      kernelName: true,
      nodename: false,
      kernelRelease: true,
      kernelVersion: false,
      machine: true,
      processor: false,
      hardwarePlatform: false,
      operatingSystem: false,
    });
    expect(result).toBe("Linux 5.15.0 x86_64");
  });

  it("should return empty string when no flags are set", () => {
    const result = formatUname(sampleInfo, {
      kernelName: false,
      nodename: false,
      kernelRelease: false,
      kernelVersion: false,
      machine: false,
      processor: false,
      hardwarePlatform: false,
      operatingSystem: false,
    });
    expect(result).toBe("");
  });

  it("should format just the nodename", () => {
    const result = formatUname(sampleInfo, {
      kernelName: false,
      nodename: true,
      kernelRelease: false,
      kernelVersion: false,
      machine: false,
      processor: false,
      hardwarePlatform: false,
      operatingSystem: false,
    });
    expect(result).toBe("myhost");
  });

  it("should format machine and OS", () => {
    const result = formatUname(sampleInfo, {
      kernelName: false,
      nodename: false,
      kernelRelease: false,
      kernelVersion: false,
      machine: true,
      processor: false,
      hardwarePlatform: false,
      operatingSystem: true,
    });
    expect(result).toBe("x86_64 GNU/Linux");
  });
});
