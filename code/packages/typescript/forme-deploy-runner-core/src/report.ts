import { createHash } from "node:crypto";
import { canonicalDeployManifest } from "./manifest.js";
import { parseDeployManifest } from "./manifest.js";
import { assertDeployPlanForManifest } from "./plan.js";
import type {
  DeployManifest,
  DeployPlan,
  DeployReport,
  DeployReportFile,
} from "./types.js";

const ZERO_TIME = "1970-01-01T00:00:00.000Z";

export function createDryRunReport(
  manifestInput: unknown,
  plan: DeployPlan,
  target: string,
): DeployReport {
  const manifest = parseDeployManifest(manifestInput);
  assertDeployPlanForManifest(plan, manifest);
  if (target.length === 0) throw new TypeError("deploy report target must be non-empty");
  const files: Record<string, DeployReportFile> = Object.create(null) as Record<string, DeployReportFile>;
  for (const entry of [...plan.entries].sort((a, b) => compareText(a.outputPath, b.outputPath))) {
    files[entry.outputPath] = Object.freeze({
      action: entry.action,
      bytesWritten: 0,
      elapsedMs: 0,
    });
  }
  return Object.freeze({
    version: 1,
    manifestSha256: createHash("sha256")
      .update(canonicalDeployManifest(manifest))
      .digest("base64"),
    target,
    startedAt: ZERO_TIME,
    finishedAt: ZERO_TIME,
    status: "success",
    files: Object.freeze(files),
    summary: Object.freeze({
      ...plan.summary,
      failed: 0,
      totalBytesWritten: 0,
      totalElapsedMs: 0,
    }),
  });
}

export function serializeDeployReport(report: DeployReport): string {
  const files: Record<string, DeployReportFile> = Object.create(null) as Record<string, DeployReportFile>;
  for (const path of Object.keys(report.files).sort(compareText)) {
    const entry = report.files[path];
    if (entry === undefined) continue;
    files[path] = {
      action: entry.action,
      bytesWritten: entry.bytesWritten,
      elapsedMs: entry.elapsedMs,
      ...(entry.error === undefined ? {} : {
        error: { code: entry.error.code, message: entry.error.message },
      }),
    };
  }
  return `${JSON.stringify({
    version: report.version,
    manifestSha256: report.manifestSha256,
    target: report.target,
    startedAt: report.startedAt,
    finishedAt: report.finishedAt,
    status: report.status,
    files,
    summary: {
      created: report.summary.created,
      updated: report.summary.updated,
      skipped: report.summary.skipped,
      deleted: report.summary.deleted,
      failed: report.summary.failed,
      totalBytesWritten: report.summary.totalBytesWritten,
      totalElapsedMs: report.summary.totalElapsedMs,
    },
  }, null, 2)}\n`;
}

function compareText(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}
