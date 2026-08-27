// Regression gate for HL24 §6. The aggregator is deliberately inspected as
// source: runtime tests cannot tell whether somebody put a concrete script
// assertion back into the shared file and recreated the merge hot spot.

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

// Vitest's jsdom transform can expose an http: import.meta.url, so source-shape
// gates resolve from the package working directory instead of assuming file:.
const aggregatorPath = resolve(process.cwd(), "tests/independentvowels.test.ts");

describe("script-owned glyph evidence", () => {
  it("keeps the stable aggregator free of per-script assertions", () => {
    const source = readFileSync(aggregatorPath, "utf8");

    expect(source.match(/\bexpect\s*\(/g) ?? []).toHaveLength(0);
    expect(source.match(/\bit\s*\(/g) ?? []).toHaveLength(1);
    expect(source).not.toMatch(/\.find\s*\(.*\.script\s*===/s);
    expect(source).not.toMatch(
      /["'`](?:arabic|cyrillic|japanese|kannada|malayalam|perso-arabic|tamil|telugu|urdu-nastaliq)["'`]/i,
    );
  });
});
