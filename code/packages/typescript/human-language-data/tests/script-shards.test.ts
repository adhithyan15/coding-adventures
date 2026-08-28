import { createHash } from "node:crypto";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot, loadScripts } from "../src/loader.js";
import {
  JAPANESE_SCRIPT_PLAN,
  PERSO_ARABIC_SCRIPT_PLAN,
  TAMIL_SCRIPT_PLAN,
  URDU_NASTALIQ_SCRIPT_PLAN,
  runShardCli,
} from "../src/shard-cli.js";
import {
  mergeScriptInventoryShards,
  scriptEntryId,
} from "../src/script-shards.js";
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

const INVENTORIES = [
  {
    name: "japanese",
    plan: JAPANESE_SCRIPT_PLAN,
    letters: 46,
    marks: 3,
    digest: "1b65688867c0f378984dcaf47cbeb6d24f3806d240263adf8484de9a4b995ad6",
  },
  {
    name: "perso-arabic",
    plan: PERSO_ARABIC_SCRIPT_PLAN,
    letters: 24,
    marks: 1,
    digest: "a4c339e47e75ffdd1aa7111d1cde5017ad6d70c9a9499b91694c6a3384d63d19",
  },
  {
    name: "tamil",
    plan: TAMIL_SCRIPT_PLAN,
    letters: 25,
    marks: 9,
    identityDigest:
      "12edac447f5a0ae29f2b89a88d326b7893cbd7176ac1809c9dffad1f1f260421",
    metadataDigest:
      "738d50ab1ce59e4b9ef40123c4c4d07d2f59ddef8347f857937b1274d0153e53",
  },
  {
    name: "urdu-nastaliq",
    plan: URDU_NASTALIQ_SCRIPT_PLAN,
    letters: 29,
    marks: 2,
    digest: "75c5ff2a3b74a1681036ccacbd3355951a00c2dd8b199e92be69dcbaf580c342",
  },
] as const;

describe("Japanese script inventory shards", () => {
  it("uses stable code-point ids instead of filesystem-dependent glyph names", () => {
    expect(scriptEntryId("あ")).toBe("U-3042");
    expect(scriptEntryId("が")).toBe("U-304B-U-3099");
    expect(() => scriptEntryId("")).toThrow(/non-empty glyph/);
  });

  it("rejects a filename id that does not match its glyph", () => {
    expect(() =>
      mergeScriptInventoryShards([
        meta,
        shard("letters/0010-U-3044.json", letter("あ")),
      ]),
    ).toThrow(/does not match.*U-3042/);
  });

  it("rejects malformed names and unknown entry kinds", () => {
    expect(() =>
      mergeScriptInventoryShards([
        meta,
        shard("letters/U-3042.json", letter("あ")),
      ]),
    ).toThrow(/expected letters\/NNNN/);
    expect(() =>
      mergeScriptInventoryShards([
        meta,
        shard("tones/0010-U-3042.json", letter("あ")),
      ]),
    ).toThrow(/expected letters\/NNNN/);
  });

  it("rejects duplicate ordinals before filename ordering can become ambiguous", () => {
    expect(() =>
      mergeScriptInventoryShards([
        meta,
        shard("letters/0010-U-3042.json", letter("あ")),
        shard("letters/0010-U-3044.json", letter("い")),
      ]),
    ).toThrow(/duplicate letters ordinal '0010'/);
  });

  it("gives each glyph exactly one owner across letters and marks", () => {
    expect(() =>
      mergeScriptInventoryShards([
        meta,
        shard("letters/0010-U-30FC.json", letter("ー")),
        shard("marks/0010-U-30FC.json", mark("ー")),
      ]),
    ).toThrow(/already owned/);
  });

  it("requires one entry object with the section's identity field", () => {
    expect(() =>
      mergeScriptInventoryShards([meta, shard("letters/0010-U-3042.json", [])]),
    ).toThrow(/one JSON object/);
    expect(() =>
      mergeScriptInventoryShards([
        meta,
        shard("marks/0010-U-309B.json", { sound: "voicing" }),
      ]),
    ).toThrow(/non-empty 'mark'/);
  });
});

describe("shard-native script inventories", () => {
  it.each(INVENTORIES)(
    "reconstructs $name exactly and forbids its aggregate",
    (inventory) => {
      const { name, plan, letters, marks } = inventory;
      const root = defaultCurriculumRoot();
      const monolithPath = join(root, "data", "scripts", `${name}.json`);
      expect(existsSync(monolithPath)).toBe(false);
      const shards = readShards(monolithPath);
      expect(shards).not.toBeNull();
      const assembled = mergeScriptInventoryShards(shards!);
      expect(assembled.letters).toHaveLength(letters);
      expect(assembled.marks).toHaveLength(marks);
      if ("digest" in inventory) {
        expect(
          createHash("sha256").update(JSON.stringify(assembled)).digest("hex"),
        ).toBe(inventory.digest);
      } else {
        const identities = [
          ...assembled.letters.map((entry) => entry.glyph),
          ...(assembled.marks ?? []).map((entry) => entry.mark),
        ];
        const { letters: _letters, marks: _marks, ...metadata } = assembled;
        expect(
          createHash("sha256").update(JSON.stringify(identities)).digest("hex"),
        ).toBe(inventory.identityDigest);
        expect(
          createHash("sha256").update(JSON.stringify(metadata)).digest("hex"),
        ).toBe(inventory.metadataDigest);
      }
      expect(loadScripts(root)[name]).toEqual(assembled);
      expect(plan.monolith).toBe("removed");
      expect(runShardCli(["--check", plan.path], root)).toBe(0);
    },
  );
});
