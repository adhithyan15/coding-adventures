import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import {
  createDeployPlan,
  createDryRunReport,
  parseDeployManifest,
  serializeDeployReport,
} from "../src/index.js";
import type { DeployReport } from "../src/index.js";

const sha = (value: string): string => createHash("sha256").update(value).digest("base64");

function current() {
  return parseDeployManifest({
    version: 1,
    fileCount: 2,
    totalSizeBytes: 2,
    files: {
      "z.txt": { outputPath: "z.txt", contentType: "text/plain", sizeBytes: 1, sha256: sha("z"), source: "extra" },
      "a.txt": { outputPath: "a.txt", contentType: "text/plain", sizeBytes: 1, sha256: sha("a"), source: "extra" },
    },
  });
}

describe("createDryRunReport", () => {
  it("emits a byte-stable sorted report with zero write timing", () => {
    const manifest = current();
    const report = createDryRunReport(manifest, createDeployPlan(manifest), "fs");
    expect(Object.keys(report.files)).toEqual(["a.txt", "z.txt"]);
    expect(report.startedAt).toBe("1970-01-01T00:00:00.000Z");
    expect(report.finishedAt).toBe("1970-01-01T00:00:00.000Z");
    expect(report.summary).toEqual({
      created: 2,
      updated: 0,
      skipped: 0,
      deleted: 0,
      failed: 0,
      totalBytesWritten: 0,
      totalElapsedMs: 0,
    });
    expect(serializeDeployReport(report)).toBe(serializeDeployReport(report));
    expect(serializeDeployReport(report)).toMatch(/^\{\n  "version": 1,/);
  });

  it("rejects an empty target name", () => {
    const manifest = current();
    expect(() => createDryRunReport(manifest, createDeployPlan(manifest), ""))
      .toThrow(/target/);
  });

  it("canonicalizes file order in reports from any trusted producer", () => {
    const manifest = current();
    const original = createDryRunReport(manifest, createDeployPlan(manifest), "fs");
    const reversed = {
      ...original,
      files: Object.fromEntries(Object.entries(original.files).reverse()),
    } as DeployReport;
    expect(serializeDeployReport(reversed)).toBe(serializeDeployReport(original));
  });
});
