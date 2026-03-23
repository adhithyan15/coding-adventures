/**
 * Tests for reporter.ts -- Build Report Formatting
 *
 * These tests verify that the reporter:
 * - Formats build results into a readable table
 * - Shows correct status labels
 * - Formats durations properly
 * - Includes a summary line with correct counts
 * - Handles empty results
 * - Sorts packages alphabetically
 */

import { describe, it, expect } from "vitest";
import {
  formatReport,
  formatDuration,
  STATUS_DISPLAY,
} from "../src/reporter.js";
import type { BuildResult } from "../src/executor.js";

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

function makeResult(
  name: string,
  status: BuildResult["status"],
  duration = 0,
): BuildResult {
  return {
    packageName: name,
    status,
    duration,
    stdout: "",
    stderr: "",
    returnCode: status === "failed" ? 1 : 0,
  };
}

// ---------------------------------------------------------------------------
// Tests: formatDuration
// ---------------------------------------------------------------------------

describe("formatDuration", () => {
  it('should return "-" for zero duration', () => {
    expect(formatDuration(0)).toBe("-");
  });

  it('should return "-" for negligible duration', () => {
    expect(formatDuration(0.005)).toBe("-");
  });

  it("should format seconds with one decimal", () => {
    expect(formatDuration(2.345)).toBe("2.3s");
  });

  it("should handle large durations", () => {
    expect(formatDuration(120.7)).toBe("120.7s");
  });

  it("should handle exactly 0.01 seconds", () => {
    expect(formatDuration(0.01)).toBe("0.0s");
  });
});

// ---------------------------------------------------------------------------
// Tests: STATUS_DISPLAY
// ---------------------------------------------------------------------------

describe("STATUS_DISPLAY", () => {
  it("should have display names for all statuses", () => {
    expect(STATUS_DISPLAY.built).toBe("BUILT");
    expect(STATUS_DISPLAY.failed).toBe("FAILED");
    expect(STATUS_DISPLAY.skipped).toBe("SKIPPED");
    expect(STATUS_DISPLAY["dep-skipped"]).toBe("DEP-SKIP");
    expect(STATUS_DISPLAY["would-build"]).toBe("WOULD-BUILD");
  });
});

// ---------------------------------------------------------------------------
// Tests: formatReport
// ---------------------------------------------------------------------------

describe("formatReport", () => {
  it("should handle empty results", () => {
    const report = formatReport(new Map());
    expect(report).toContain("No packages processed.");
  });

  it("should include header", () => {
    const results = new Map<string, BuildResult>();
    results.set("python/pkg", makeResult("python/pkg", "built", 1.5));

    const report = formatReport(results);
    expect(report).toContain("Build Report");
    expect(report).toContain("============");
  });

  it("should show built packages with duration", () => {
    const results = new Map<string, BuildResult>();
    results.set("python/pkg", makeResult("python/pkg", "built", 2.3));

    const report = formatReport(results);
    expect(report).toContain("python/pkg");
    expect(report).toContain("BUILT");
    expect(report).toContain("2.3s");
  });

  it("should show skipped packages", () => {
    const results = new Map<string, BuildResult>();
    results.set("python/pkg", makeResult("python/pkg", "skipped"));

    const report = formatReport(results);
    expect(report).toContain("SKIPPED");
  });

  it("should show dep-skipped with special duration", () => {
    const results = new Map<string, BuildResult>();
    results.set("python/pkg", makeResult("python/pkg", "dep-skipped"));

    const report = formatReport(results);
    expect(report).toContain("DEP-SKIP");
    expect(report).toContain("- (dep failed)");
  });

  it("should show would-build in dry run", () => {
    const results = new Map<string, BuildResult>();
    results.set("python/pkg", makeResult("python/pkg", "would-build"));

    const report = formatReport(results);
    expect(report).toContain("WOULD-BUILD");
  });

  it("should include summary line with correct counts", () => {
    const results = new Map<string, BuildResult>();
    results.set("python/a", makeResult("python/a", "built", 1.0));
    results.set("python/b", makeResult("python/b", "skipped"));
    results.set("python/c", makeResult("python/c", "failed", 0.5));
    results.set("python/d", makeResult("python/d", "dep-skipped"));

    const report = formatReport(results);
    expect(report).toContain("Total: 4 packages");
    expect(report).toContain("1 built");
    expect(report).toContain("1 skipped");
    expect(report).toContain("1 failed");
    expect(report).toContain("1 dep-skipped");
  });

  it("should sort packages alphabetically", () => {
    const results = new Map<string, BuildResult>();
    results.set("python/z-pkg", makeResult("python/z-pkg", "built", 1.0));
    results.set("python/a-pkg", makeResult("python/a-pkg", "built", 2.0));

    const report = formatReport(results);
    const aIdx = report.indexOf("python/a-pkg");
    const zIdx = report.indexOf("python/z-pkg");
    expect(aIdx).toBeLessThan(zIdx);
  });

  it("should omit zero counts from summary", () => {
    const results = new Map<string, BuildResult>();
    results.set("python/a", makeResult("python/a", "built", 1.0));

    const report = formatReport(results);
    expect(report).toContain("1 built");
    expect(report).not.toContain("skipped");
    expect(report).not.toContain("failed");
  });
});
