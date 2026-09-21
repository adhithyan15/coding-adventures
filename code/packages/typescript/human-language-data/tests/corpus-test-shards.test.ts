import {
  mkdtempSync,
  mkdirSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { afterEach, describe, expect, it } from "vitest";
import {
  loadCorpusTestShards,
  validateCorpusTestShardNames,
} from "./corpus/corpus-test-shards.js";

const temporaryRoots: string[] = [];

afterEach(() => {
  for (const root of temporaryRoots.splice(0)) {
    rmSync(root, { recursive: true, force: true });
  }
});

function fixtureRoot(language: string, names: readonly string[]): string {
  const root = mkdtempSync(join(tmpdir(), "hl-corpus-test-shards-"));
  temporaryRoots.push(root);
  const shardRoot = join(root, language);
  mkdirSync(shardRoot);
  for (const name of names) writeFileSync(join(shardRoot, name), "export {};\n");
  return root;
}

describe("corpus test shard ownership", () => {
  it("sorts stable collision-resistant owner names", () => {
    expect(validateCorpusTestShardNames("gujarati", [
      "z-last-1234abcd.case.ts",
      "a-first-deadbeef.case.ts",
    ])).toEqual([
      "a-first-deadbeef.case.ts",
      "z-last-1234abcd.case.ts",
    ]);
  });

  it("rejects empty, unsafe, and case-fold-colliding owner sets", () => {
    expect(() => validateCorpusTestShardNames("gujarati", [])).toThrow(/no shard owners/);
    expect(() => validateCorpusTestShardNames("../gujarati", [
      "safe-owner-deadbeef.case.ts",
    ])).toThrow(/Unsafe corpus test language id/);
    expect(() => validateCorpusTestShardNames("gujarati", [
      "../unsafe-owner-deadbeef.case.ts",
    ])).toThrow(/Unsafe corpus test shard owner/);
    expect(() => validateCorpusTestShardNames("gujarati", [
      "same-owner-deadbeef.case.ts",
      "SAME-OWNER-deadbeef.case.ts",
    ])).toThrow(/Case-fold-colliding/);
  });

  it("loads the complete owner set in deterministic filename order", async () => {
    const names = [
      "z-last-1234abcd.case.ts",
      "a-first-deadbeef.case.ts",
    ];
    const root = fixtureRoot("gujarati", names);
    const loaded: string[] = [];
    await loadCorpusTestShards(
      "gujarati",
      pathToFileURL(join(root, "gujarati.test.ts")).href,
      {
        "./gujarati/z-last-1234abcd.case.ts": async () => { loaded.push("z"); },
        "./gujarati/a-first-deadbeef.case.ts": async () => { loaded.push("a"); },
      },
    );
    expect(loaded).toEqual(["a", "z"]);
  });

  it("rejects missing discovery entries", async () => {
    const names = [
      "a-first-deadbeef.case.ts",
      "b-second-1234abcd.case.ts",
    ];
    const root = fixtureRoot("gujarati", names);
    await expect(loadCorpusTestShards(
      "gujarati",
      pathToFileURL(join(root, "gujarati.test.ts")).href,
      {
        "./gujarati/a-first-deadbeef.case.ts": async () => {},
      },
    )).rejects.toThrow(/discovery mismatch/);
  });

  it.skipIf(process.platform === "win32")("rejects symlink owners", async () => {
    const root = fixtureRoot("gujarati", ["a-first-deadbeef.case.ts"]);
    symlinkSync(
      join(root, "gujarati", "a-first-deadbeef.case.ts"),
      join(root, "gujarati", "b-second-1234abcd.case.ts"),
    );
    await expect(loadCorpusTestShards(
      "gujarati",
      pathToFileURL(join(root, "gujarati.test.ts")).href,
      {
        "./gujarati/a-first-deadbeef.case.ts": async () => {},
        "./gujarati/b-second-1234abcd.case.ts": async () => {},
      },
    )).rejects.toThrow(/regular file/);
  });
});
