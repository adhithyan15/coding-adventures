/**
 * forme-pipeline-config — loadTsConfig tests
 *
 * Uses the `importModule` test hook so we don't have to write actual
 * TS files in a temp dir.
 */

import { describe, it, expect, vi } from "vitest";
import { loadTsConfig } from "../src/index.js";

describe("loadTsConfig — happy path", () => {
  it("returns the default export of the imported module", async () => {
    const config = { name: "x", settings: {}, stages: [] };
    const importModule = vi.fn(async () => ({ default: config }));
    const result = await loadTsConfig("/abs/forme.config.ts", { importModule });
    expect(result).toBe(config);
    expect(importModule).toHaveBeenCalledTimes(1);
  });

  it("converts the path to a file:// URL", async () => {
    let received = "";
    const importModule = vi.fn(async (specifier: string) => {
      received = specifier;
      return { default: {} };
    });
    await loadTsConfig("/abs/forme.config.ts", { importModule });
    expect(received.startsWith("file://")).toBe(true);
    expect(received).toContain("forme.config.ts");
  });

  it("resolves relative paths against the supplied cwd", async () => {
    let received = "";
    const importModule = vi.fn(async (specifier: string) => {
      received = specifier;
      return { default: {} };
    });
    await loadTsConfig("forme.config.ts", { cwd: "/some/work/dir", importModule });
    expect(received).toContain("/some/work/dir/forme.config.ts");
  });
});

describe("loadTsConfig — rejection paths", () => {
  it("rejects empty path", async () => {
    await expect(loadTsConfig("")).rejects.toThrow(/non-empty string/);
  });

  it("rejects non-string path", async () => {
    // @ts-expect-error — runtime check
    await expect(loadTsConfig(null)).rejects.toThrow(/non-empty string/);
  });

  it("wraps import errors with the file:// URL in the message", async () => {
    const importModule = async () => { throw new Error("module borked"); };
    await expect(loadTsConfig("/abs/x.ts", { importModule })).rejects.toThrow(
      /failed to import.*x\.ts.*module borked/s,
    );
  });

  it("handles non-Error throws inside the importer", async () => {
    const importModule = async () => { throw "string error"; };
    await expect(loadTsConfig("/abs/x.ts", { importModule })).rejects.toThrow(
      /failed to import.*string error/s,
    );
  });

  it("rejects when the imported module is not an object", async () => {
    const importModule = async () => 42;
    await expect(loadTsConfig("/abs/x.ts", { importModule })).rejects.toThrow(
      /did not produce an object/,
    );
  });

  it("rejects when default export is missing", async () => {
    const importModule = async () => ({ named: { name: "x" } });
    await expect(loadTsConfig("/abs/x.ts", { importModule })).rejects.toThrow(
      /no default export/,
    );
  });

  it("rejects when default export is not an object", async () => {
    const importModule = async () => ({ default: "not a config" });
    await expect(loadTsConfig("/abs/x.ts", { importModule })).rejects.toThrow(
      /no default export/,
    );
  });

  it("rejects when default export is null", async () => {
    const importModule = async () => ({ default: null });
    await expect(loadTsConfig("/abs/x.ts", { importModule })).rejects.toThrow(
      /no default export/,
    );
  });
});
