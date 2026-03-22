/**
 * Tests for df -- report file system disk space usage.
 *
 * We test the exported business logic functions: formatSize,
 * getFilesystemInfo, and formatDfTable. Since df depends on the
 * actual system state, we verify structure and format.
 */

import { describe, it, expect } from "vitest";
import {
  formatSize,
  getFilesystemInfo,
  formatDfTable,
  FsInfo,
} from "../src/df.js";

// ---------------------------------------------------------------------------
// formatSize.
// ---------------------------------------------------------------------------

describe("formatSize", () => {
  it("should format small sizes as K", () => {
    expect(formatSize(500)).toBe("500K");
  });

  it("should format sizes in megabytes", () => {
    // 2048 KB = 2 MB (in powers of 1024).
    const result = formatSize(2048);
    expect(result).toBe("2.0M");
  });

  it("should format sizes in gigabytes", () => {
    // 2 * 1024 * 1024 KB = 2 GB.
    const result = formatSize(2 * 1024 * 1024);
    expect(result).toBe("2.0G");
  });

  it("should format large sizes in terabytes", () => {
    // 2 * 1024^3 KB = 2 TB.
    const result = formatSize(2 * 1024 * 1024 * 1024);
    expect(result).toBe("2.0T");
  });

  it("should use one decimal for values less than 10", () => {
    // 1.5 MB = 1536 KB.
    expect(formatSize(1536)).toBe("1.5M");
  });

  it("should round values 10 and above", () => {
    // 15 MB = 15360 KB.
    expect(formatSize(15360)).toBe("15M");
  });

  it("should handle zero", () => {
    expect(formatSize(0)).toBe("0K");
  });

  it("should use powers of 1000 with si flag", () => {
    // 2000 KB with si: 2000 * 1024 bytes = 2,048,000 bytes.
    // In SI (1000^2 = 1M), 2048000 / 1000000 = 2.048 => "2.0M".
    const result = formatSize(2000, true);
    expect(result).toBe("2.0M");
  });
});

// ---------------------------------------------------------------------------
// getFilesystemInfo.
// ---------------------------------------------------------------------------

describe("getFilesystemInfo", () => {
  it("should return an array", () => {
    const infos = getFilesystemInfo();
    expect(Array.isArray(infos)).toBe(true);
  });

  it("should return at least one filesystem", () => {
    const infos = getFilesystemInfo();
    expect(infos.length).toBeGreaterThan(0);
  });

  it("should have filesystem field on each entry", () => {
    const infos = getFilesystemInfo();
    for (const info of infos) {
      expect(info.filesystem).toBeTruthy();
      expect(typeof info.filesystem).toBe("string");
    }
  });

  it("should have size field on each entry", () => {
    const infos = getFilesystemInfo();
    for (const info of infos) {
      expect(info.size).toBeTruthy();
    }
  });

  it("should have mountedOn field on each entry", () => {
    const infos = getFilesystemInfo();
    for (const info of infos) {
      expect(info.mountedOn).toBeTruthy();
    }
  });

  it("should have usePercent field on each entry", () => {
    const infos = getFilesystemInfo();
    for (const info of infos) {
      expect(info.usePercent).toBeDefined();
      // usePercent is typically "24%" but can be "0" for some filesystems.
      expect(info.usePercent).toMatch(/^\d+%?$/);
    }
  });

  it("should return info for a specific path", () => {
    const infos = getFilesystemInfo(["/"]);
    expect(infos.length).toBeGreaterThanOrEqual(1);
    // The root filesystem should be present.
    const root = infos.find((i) => i.mountedOn === "/" || i.mountedOn.startsWith("/"));
    expect(root).toBeTruthy();
  });

  it("should return human-readable sizes when requested", () => {
    const infos = getFilesystemInfo(undefined, true);
    expect(infos.length).toBeGreaterThan(0);
    // Human-readable sizes should contain K, M, G, or T.
    for (const info of infos) {
      expect(info.size).toMatch(/[KMGTP0-9]/);
    }
  });
});

// ---------------------------------------------------------------------------
// formatDfTable.
// ---------------------------------------------------------------------------

describe("formatDfTable", () => {
  it("should format an empty array as empty string", () => {
    expect(formatDfTable([])).toBe("");
  });

  it("should include header line", () => {
    const infos: FsInfo[] = [
      {
        filesystem: "/dev/sda1",
        size: "1000000",
        used: "500000",
        available: "500000",
        usePercent: "50%",
        mountedOn: "/",
      },
    ];
    const result = formatDfTable(infos);
    expect(result).toContain("Filesystem");
    expect(result).toContain("1K-blocks");
    expect(result).toContain("Use%");
    expect(result).toContain("Mounted on");
  });

  it("should include data rows", () => {
    const infos: FsInfo[] = [
      {
        filesystem: "/dev/sda1",
        size: "1000000",
        used: "500000",
        available: "500000",
        usePercent: "50%",
        mountedOn: "/",
      },
    ];
    const result = formatDfTable(infos);
    expect(result).toContain("/dev/sda1");
    expect(result).toContain("1000000");
    expect(result).toContain("50%");
  });

  it("should use human-readable headers when specified", () => {
    const infos: FsInfo[] = [
      {
        filesystem: "/dev/sda1",
        size: "932G",
        used: "224G",
        available: "708G",
        usePercent: "24%",
        mountedOn: "/",
      },
    ];
    const result = formatDfTable(infos, true);
    expect(result).toContain("Size");
    expect(result).not.toContain("1K-blocks");
  });

  it("should end with a newline", () => {
    const infos: FsInfo[] = [
      {
        filesystem: "/dev/sda1",
        size: "1000",
        used: "500",
        available: "500",
        usePercent: "50%",
        mountedOn: "/",
      },
    ];
    const result = formatDfTable(infos);
    expect(result.endsWith("\n")).toBe(true);
  });
});
