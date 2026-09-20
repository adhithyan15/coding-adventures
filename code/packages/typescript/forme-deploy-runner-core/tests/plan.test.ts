import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { createDeployPlan, parseDeployManifest } from "../src/index.js";

const sha = (value: string): string => createHash("sha256").update(value).digest("base64");

function parsed(entries: readonly [string, string][]) {
  const files = Object.fromEntries(entries.map(([path, body]) => [path, {
    outputPath: path,
    contentType: "text/plain",
    sizeBytes: Buffer.byteLength(body),
    sha256: sha(body),
    source: "extra",
  }]));
  return parseDeployManifest({
    version: 1,
    fileCount: entries.length,
    totalSizeBytes: entries.reduce((sum, [, body]) => sum + Buffer.byteLength(body), 0),
    files,
  });
}

describe("createDeployPlan", () => {
  it("sorts create, update, skip, and previous-only delete actions", () => {
    const current = parsed([
      ["create.txt", "new"],
      ["skip.txt", "same"],
      ["update.txt", "after"],
    ]);
    const previous = parsed([
      ["delete.txt", "old"],
      ["skip.txt", "same"],
      ["update.txt", "before"],
    ]);

    expect(createDeployPlan(current, previous).entries).toEqual([
      expect.objectContaining({ outputPath: "create.txt", action: "create" }),
      expect.objectContaining({ outputPath: "delete.txt", action: "delete" }),
      expect.objectContaining({ outputPath: "skip.txt", action: "skip" }),
      expect.objectContaining({ outputPath: "update.txt", action: "update" }),
    ]);
  });

  it("treats every current file as create without a previous manifest", () => {
    expect(createDeployPlan(parsed([["b", "2"], ["a", "1"]]))).toEqual({
      entries: [
        expect.objectContaining({ outputPath: "a", action: "create" }),
        expect.objectContaining({ outputPath: "b", action: "create" }),
      ],
      summary: { created: 2, updated: 0, skipped: 0, deleted: 0 },
    });
  });

  it("never gives a previous manifest deletion authority over unlisted siblings", () => {
    const plan = createDeployPlan(parsed([["new.txt", "new"]]), parsed([["owned.txt", "old"]]));
    expect(plan.entries.map(entry => entry.outputPath)).toEqual(["new.txt", "owned.txt"]);
  });

  it("validates structural inputs again at the planning boundary", () => {
    const current = parsed([["new.txt", "new"]]);
    const unsafePrevious = {
      version: 1,
      fileCount: 1,
      totalSizeBytes: 1,
      files: {
        "../../outside": {
          outputPath: "../../outside",
          contentType: "text/plain",
          sizeBytes: 1,
          sha256: sha("x"),
          source: "extra",
        },
      },
    };
    expect(() => createDeployPlan(current, unsafePrevious)).toThrow(/output path|segments|portable/i);
  });

  it("freezes every plan entry so validated paths cannot be mutated", () => {
    const plan = createDeployPlan(parsed([]), parsed([["owned.txt", "old"]]));
    expect(Object.isFrozen(plan.entries[0])).toBe(true);
    expect(() => {
      (plan.entries[0] as { outputPath: string }).outputPath = "../../outside";
    }).toThrow();
    expect(plan.entries[0]?.outputPath).toBe("owned.txt");
  });

  it("updates target metadata even when content bytes are unchanged", () => {
    const current = parsed([["a.txt", "same"]]);
    const previous = parseDeployManifest({
      ...current,
      files: { "a.txt": { ...current.files["a.txt"], contentType: "application/octet-stream" } },
    });
    expect(createDeployPlan(current, previous).entries[0]?.action).toBe("update");
  });
});
