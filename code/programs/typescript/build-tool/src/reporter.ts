/**
 * reporter.ts -- Build Report Formatting
 * =======================================
 *
 * This module formats and prints a summary table of build results. The output
 * is human-readable and designed for terminal display.
 *
 * ## Output format
 *
 * ```
 * Build Report
 * ============
 * Package                    Status     Duration
 * python/logic-gates         SKIPPED    -
 * python/arithmetic          BUILT      2.3s
 * python/arm-simulator       FAILED     0.5s
 * python/riscv-simulator     DEP-SKIP   - (dep failed)
 *
 * Total: 21 packages | 5 built | 14 skipped | 1 failed | 1 dep-skipped
 * ```
 *
 * The report is designed to be scannable at a glance: the status column
 * tells you immediately what happened to each package, and the summary
 * line gives you the high-level picture.
 */

import type { BuildResult } from "./executor.js";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/**
 * Status display names mapping.
 *
 * These are the human-readable labels shown in the report table.
 * They're kept short to fit in a fixed-width column.
 */
export const STATUS_DISPLAY: Record<string, string> = {
  built: "BUILT",
  failed: "FAILED",
  skipped: "SKIPPED",
  "dep-skipped": "DEP-SKIP",
  "would-build": "WOULD-BUILD",
};

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/**
 * Format a duration for display.
 *
 * Returns "-" for zero/negligible durations (less than 10ms),
 * otherwise formats as "X.Ys" with one decimal place.
 *
 * @param seconds - Duration in seconds.
 * @returns Formatted duration string.
 */
export function formatDuration(seconds: number): string {
  if (seconds < 0.01) {
    return "-";
  }
  return `${seconds.toFixed(1)}s`;
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Format a build report as a string.
 *
 * The report includes:
 * 1. A header ("Build Report")
 * 2. A table with Package, Status, and Duration columns
 * 3. A summary line with counts for each status type
 *
 * Packages are sorted alphabetically for consistent output.
 *
 * @param results - Mapping from package name to BuildResult.
 * @returns The formatted report string.
 */
export function formatReport(results: Map<string, BuildResult>): string {
  const lines: string[] = [];

  lines.push("");
  lines.push("Build Report");
  lines.push("============");

  if (results.size === 0) {
    lines.push("No packages processed.");
    return lines.join("\n") + "\n";
  }

  // Calculate column widths.
  const maxNameLen = Math.max(
    ...Array.from(results.keys()).map((n) => n.length),
    "Package".length,
  );

  // Header.
  lines.push(
    `${"Package".padEnd(maxNameLen)}   ${"Status".padEnd(12)} Duration`,
  );

  // Sort results by name for consistent output.
  const sortedNames = Array.from(results.keys()).sort();

  for (const name of sortedNames) {
    const result = results.get(name)!;
    const status = STATUS_DISPLAY[result.status] ?? result.status.toUpperCase();
    let duration = formatDuration(result.duration);

    if (result.status === "dep-skipped") {
      duration = "- (dep failed)";
    }

    lines.push(`${name.padEnd(maxNameLen)}   ${status.padEnd(12)} ${duration}`);
  }

  // Summary line.
  const total = results.size;
  const built = Array.from(results.values()).filter(
    (r) => r.status === "built",
  ).length;
  const skipped = Array.from(results.values()).filter(
    (r) => r.status === "skipped",
  ).length;
  const failed = Array.from(results.values()).filter(
    (r) => r.status === "failed",
  ).length;
  const depSkipped = Array.from(results.values()).filter(
    (r) => r.status === "dep-skipped",
  ).length;
  const wouldBuild = Array.from(results.values()).filter(
    (r) => r.status === "would-build",
  ).length;

  let summary = `\nTotal: ${total} packages`;
  if (built) summary += ` | ${built} built`;
  if (skipped) summary += ` | ${skipped} skipped`;
  if (failed) summary += ` | ${failed} failed`;
  if (depSkipped) summary += ` | ${depSkipped} dep-skipped`;
  if (wouldBuild) summary += ` | ${wouldBuild} would-build`;

  lines.push(summary);

  return lines.join("\n") + "\n";
}

/**
 * Print a summary table of build results to the console.
 *
 * @param results - Mapping from package name to BuildResult.
 */
export function printReport(results: Map<string, BuildResult>): void {
  const report = formatReport(results);
  process.stdout.write(report);
}
