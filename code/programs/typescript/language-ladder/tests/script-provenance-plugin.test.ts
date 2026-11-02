import { readFileSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

import {
  SLIM_MODULE_PREFIX,
  SLIM_SCRIPTS,
  isEagerScriptDataModuleId,
  scriptOfInventoryPath,
  scriptProvenancePlugin,
  slimProvenance,
} from "../script-provenance-plugin";

// The plugin gives script-ductus's scriptdata.ts (the app's eager path) a copy
// of the Chinese inventory without stroke-order provenance prose, and leaves
// the real file to the lazy stroke modules. These tests pin both halves, the
// chunk-group predicate that keeps them apart, and the rule for which scripts
// may be slimmed.

const ROOT = "/repo/code/learning/human-languages";
const SCRIPTDATA = "/repo/code/packages/typescript/script-ductus/src/scriptdata.ts";
const STROKES = "/repo/code/packages/typescript/script-ductus/src/strokes/chinese.ts";
const FROM_SCRIPTDATA = "../../../../learning/human-languages/data/scripts/chinese.json";

const SOURCE = {
  citation: "Hanzi Writer Data 怪.json, ordered stroke paths and medians 1–8",
  url: "https://raw.githubusercontent.com/chanind/hanzi-writer-data/x/data/%E6%80%AA.json",
  variation: "The cited PRC-order dataset supplies eight ordered strokes …",
};

describe("slimProvenance", () => {
  it("keeps only the url of every stroke source, at any depth", () => {
    const inventory = {
      script: "chinese",
      letters: [{ glyph: "怪", penLifts: 7, strokeOrderSource: SOURCE }],
      digits: [{ glyph: "೧", strokeOrderSource: SOURCE }],
      marks: [{ mark: "ੰ", strokeOrderSource: SOURCE }],
    };
    expect(slimProvenance(inventory)).toEqual({
      script: "chinese",
      letters: [{ glyph: "怪", penLifts: 7, strokeOrderSource: { url: SOURCE.url } }],
      digits: [{ glyph: "೧", strokeOrderSource: { url: SOURCE.url } }],
      marks: [{ mark: "ੰ", strokeOrderSource: { url: SOURCE.url } }],
    });
  });

  it("keeps 'has a source' true without a url, and adds no source where there was none", () => {
    expect(slimProvenance({ letters: [{ glyph: "a" }] })).toEqual({ letters: [{ glyph: "a" }] });
    expect(slimProvenance({ strokeOrderSource: { citation: "only prose" } })).toEqual({
      strokeOrderSource: {},
    });
  });

  it("keeps a data key named __proto__ as an own property", () => {
    const slim = slimProvenance(JSON.parse('{"__proto__": {"x": 1}, "glyph": "a"}')) as object;
    expect(Object.getPrototypeOf(slim)).toBe(Object.prototype);
    expect(Object.keys(slim)).toEqual(["__proto__", "glyph"]);
  });

  it("touches nothing else: other prose fields survive verbatim", () => {
    const letter = { glyph: "怪", notes: "long notes", strokeOrder: ["one", "two"] };
    expect(slimProvenance(letter)).toEqual(letter);
  });
});

describe("the eager script-data group", () => {
  it("admits the slim copies and the shard-native inventories module", () => {
    expect(isEagerScriptDataModuleId(`\0${SLIM_MODULE_PREFIX}chinese`)).toBe(true);
    expect(isEagerScriptDataModuleId("\0virtual:script-ductus-inventories")).toBe(true);
  });

  it("keeps every other inventory file it always held", () => {
    expect(isEagerScriptDataModuleId(`${ROOT}/data/scripts/arabic.json`)).toBe(true);
    expect(isEagerScriptDataModuleId(`${ROOT}/data/scripts/devanagari-ledger.json`)).toBe(true);
    expect(isEagerScriptDataModuleId(`${ROOT}/data/scripts/japanese.d/_meta.json`)).toBe(true);
  });

  it("leaves out the full file of a slimmed inventory, and unrelated modules", () => {
    expect(isEagerScriptDataModuleId(`${ROOT}/data/scripts/chinese.json`)).toBe(false);
    expect(isEagerScriptDataModuleId(`${ROOT}/core/languages.json`)).toBe(false);
    expect(isEagerScriptDataModuleId(STROKES)).toBe(false);
  });

  it("names a monolithic inventory's script", () => {
    expect(scriptOfInventoryPath(`${ROOT}/data/scripts/chinese.json`)).toBe("chinese");
    expect(scriptOfInventoryPath(`${ROOT}/data/scripts/japanese.d/_meta.json`)).toBeNull();
  });
});

describe("scriptProvenancePlugin", () => {
  const plugin = scriptProvenancePlugin({ curriculumRoot: ROOT });
  const resolveId = plugin.resolveId as (source: string, importer?: string) => string | null;

  it("runs before Vite's JSON handling, and only in builds", () => {
    expect(plugin.enforce).toBe("pre");
    expect(plugin.apply).toBe("build");
  });

  it("gives scriptdata.ts the slim copy of a slimmed inventory", () => {
    expect(resolveId(FROM_SCRIPTDATA, SCRIPTDATA)).toBe(`\0${SLIM_MODULE_PREFIX}chinese`);
  });

  it("leaves the stroke modules, other scripts and outside paths on the real file", () => {
    expect(resolveId("../../../../../learning/human-languages/data/scripts/chinese.json", STROKES)).toBeNull();
    expect(resolveId("../../../../learning/human-languages/data/scripts/arabic.json", SCRIPTDATA)).toBeNull();
    expect(resolveId("../../../../elsewhere/data/scripts/chinese.json", SCRIPTDATA)).toBeNull();
    expect(resolveId("vite", SCRIPTDATA)).toBeNull();
  });
});

describe("which scripts may be slimmed", () => {
  // A script can be slimmed only if its stroke module reads the inventory file
  // itself: then the full record still reaches the handwriting card. Arabic's
  // reads it through scriptdata's SCRIPTS, which is exactly the copy slimmed.
  const strokesDir = path.resolve(
    __dirname,
    "../../../../packages/typescript/script-ductus/src/strokes",
  );

  it.each(SLIM_SCRIPTS)("%s: the stroke module imports its own inventory file", (script) => {
    const source = readFileSync(path.join(strokesDir, `${script}.ts`), "utf8");
    expect(source).toMatch(new RegExp(`data/scripts/${script}\\.json`));
    expect(source).not.toMatch(/from "\.\.\/scriptdata(\.ts)?"/);
  });

  it("does not slim arabic, whose stroke module reads sources through scriptdata", () => {
    const source = readFileSync(path.join(strokesDir, "arabic-family.ts"), "utf8");
    expect(source).toMatch(/from "\.\.\/scriptdata\.ts"/);
    expect(SLIM_SCRIPTS).not.toContain("arabic");
  });
});
