import { createHash } from "node:crypto";
import { readFileSync, readdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

import { DUCTUS } from "../src/strokes";

const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const ownerNames = [
  "arabic-family",
  "chinese",
  "cyrillic",
  "devanagari",
  "gujarati",
  "hebrew",
  "japanese",
  "kannada",
  "malayalam",
  "tamil",
  "telugu",
];

describe("stroke ownership migration baseline", () => {
  it("preserves the exact ordered registry and parsed data", () => {
    const counts = Object.values(DUCTUS).reduce<Record<string, number>>(
      (out, letter) => {
        out[letter.script] = (out[letter.script] ?? 0) + 1;
        return out;
      },
      {},
    );
    expect({
      keys: Object.keys(DUCTUS).length,
      keyHash: sha256(JSON.stringify(Object.keys(DUCTUS))),
      dataHash: sha256(JSON.stringify(DUCTUS)),
      counts: Object.fromEntries(
        Object.entries(counts).sort(([a], [b]) => a.localeCompare(b)),
      ),
    }).toEqual({
      keys: 330,
      keyHash:
        "dfbc3a4264318948f47cd52a076282f03e69ce64dfbb98e2145a7c5fa8896542",
      dataHash:
        "482c15657edc14bc02f3a07e7493f03202a4f5a7786125e9fa3fe309d25e7ffb",
      counts: {
        arabic: 32,
        chinese: 43,
        cyrillic: 33,
        devanagari: 43,
        gujarati: 44,
        hebrew: 22,
        japanese: 12,
        kannada: 7,
        malayalam: 10,
        "perso-arabic": 24,
        tamil: 25,
        telugu: 6,
        "urdu-nastaliq": 29,
      },
    });
  });

  it("keeps authored entries in stable owner modules", () => {
    const ownerDir = resolve(packageRoot, "src/strokes");
    expect(
      readdirSync(ownerDir)
        .filter((name) => name.endsWith(".ts") && name !== "registry.ts")
        .map((name) => name.replace(/\.ts$/, ""))
        .sort(),
    ).toEqual(ownerNames);

    const compatibilitySource = readFileSync(
      resolve(packageRoot, "src/strokes.ts"),
      "utf8",
    );
    expect(compatibilitySource).not.toMatch(/\[ductusKey\([^\n]+\)\]\s*:/);
    expect(compatibilitySource).not.toMatch(/^\s*["'][^"']+["']\s*:\s*\{\s*$/m);
  });

  it("keeps script-specific claims out of the two shared evidence roots", () => {
    for (const name of ["strokes.test.ts", "ductusview.test.ts"]) {
      const source = readFileSync(resolve(packageRoot, "tests", name), "utf8");
      expect(
        source,
        `${name} imports an owner-specific font fixture`,
      ).not.toMatch(/from\s+["']\.\/support\/font-fixtures["']/);
      expect(source, `${name} directly looks up an owner glyph`).not.toMatch(
        /DUCTUS\s*\[\s*["'][^"']+["']\s*\]/,
      );
      expect(source, `${name} names an owner script`).not.toMatch(
        /\b(?:arabic|chinese|cyrillic|devanagari|gujarati|hebrew|japanese|kannada|malayalam|tamil|telugu|urdu)\b/i,
      );
      expect(source, `${name} embeds a native owner-script glyph`).not.toMatch(
        /[\u0400-\u052f\u0590-\u06ff\u0900-\u097f\u0a80-\u0aff\u0b80-\u0cff\u0d00-\u0d7f\u3040-\u30ff\u3400-\u9fff]/u,
      );
    }
  });
});
