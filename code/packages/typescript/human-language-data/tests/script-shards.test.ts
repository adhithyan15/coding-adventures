import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot, loadScripts } from "../src/loader.js";
import { JAPANESE_SCRIPT_PLAN, runShardCli } from "../src/shard-cli.js";
import { mergeScriptInventoryShards, scriptEntryId } from "../src/script-shards.js";
import { readShards, type Shard } from "../src/shard.js";

function shard(name: string, value: unknown): Shard {
  return { name, path: `fixture/japanese.d/${name}`, value };
}

const meta = shard("_meta.json", {
  script: "japanese",
  name: "Japanese",
  font: "fonts/NotoSansJP-Regular.otf",
  direction: "ltr",
  system: "mixed",
});

const letter = (glyph: string) => ({
  glyph,
  sound: "a",
  role: "letter",
  components: ["curve"],
  strokeOrder: ["curve"],
  strokeOrderNote: "fixture",
});

const mark = (glyph: string) => ({
  mark: glyph,
  sound: "mark",
  role: "other",
  attachesAs: "after a kana",
});

describe("Japanese script inventory shards", () => {
  it("uses stable code-point ids instead of filesystem-dependent glyph names", () => {
    expect(scriptEntryId("あ")).toBe("U-3042");
    expect(scriptEntryId("が")).toBe("U-304B-U-3099");
    expect(() => scriptEntryId("")).toThrow(/non-empty glyph/);
  });

  it("reassembles every committed Japanese entry exactly", () => {
    const root = defaultCurriculumRoot();
    const monolithPath = join(root, "data", "scripts", "japanese.json");
    const monolith = JSON.parse(readFileSync(monolithPath, "utf8")) as unknown;
    const shards = readShards(monolithPath);
    expect(shards).not.toBeNull();
    const assembled = mergeScriptInventoryShards(shards!);
    expect(assembled).toEqual(monolith);
    expect(assembled.letters).toHaveLength(46);
    expect(assembled.marks).toHaveLength(3);
    expect(loadScripts(root).japanese).toEqual(monolith);
    expect(runShardCli(["--check", JAPANESE_SCRIPT_PLAN.path], root)).toBe(0);
  });

  it("rejects a filename id that does not match its glyph", () => {
    expect(() => mergeScriptInventoryShards([
      meta,
      shard("letters/0010-U-3044.json", letter("あ")),
    ])).toThrow(/does not match.*U-3042/);
  });

  it("rejects malformed names and unknown entry kinds", () => {
    expect(() => mergeScriptInventoryShards([
      meta,
      shard("letters/U-3042.json", letter("あ")),
    ])).toThrow(/expected letters\/NNNN/);
    expect(() => mergeScriptInventoryShards([
      meta,
      shard("tones/0010-U-3042.json", letter("あ")),
    ])).toThrow(/expected letters\/NNNN/);
  });

  it("rejects duplicate ordinals before filename ordering can become ambiguous", () => {
    expect(() => mergeScriptInventoryShards([
      meta,
      shard("letters/0010-U-3042.json", letter("あ")),
      shard("letters/0010-U-3044.json", letter("い")),
    ])).toThrow(/duplicate letters ordinal '0010'/);
  });

  it("gives each glyph exactly one owner across letters and marks", () => {
    expect(() => mergeScriptInventoryShards([
      meta,
      shard("letters/0010-U-30FC.json", letter("ー")),
      shard("marks/0010-U-30FC.json", mark("ー")),
    ])).toThrow(/already owned/);
  });

  it("requires one entry object with the section's identity field", () => {
    expect(() => mergeScriptInventoryShards([
      meta,
      shard("letters/0010-U-3042.json", []),
    ])).toThrow(/one JSON object/);
    expect(() => mergeScriptInventoryShards([
      meta,
      shard("marks/0010-U-309B.json", { sound: "voicing" }),
    ])).toThrow(/non-empty 'mark'/);
  });
});
