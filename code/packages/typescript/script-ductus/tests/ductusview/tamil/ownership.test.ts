import { existsSync, lstatSync, readFileSync, readdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

import { DUCTUS } from "../../../src/strokes";

const ownerDirectory = dirname(fileURLToPath(import.meta.url));
const ownerFilename = (glyph: string): string =>
  `U-${glyph.codePointAt(0)!.toString(16).toUpperCase()}.test.ts`;
const tamilGlyphs = Object.values(DUCTUS)
  .filter((letter) => letter.script === "tamil")
  .map((letter) => letter.glyph);

describe("Tamil Ductus view-evidence ownership", () => {
  it("discovers exactly one ASCII codepoint owner module per Tamil glyph", () => {
    const actual = readdirSync(ownerDirectory)
      .filter((name) => name !== "ownership.test.ts")
      .map((name) => {
        const stat = lstatSync(resolve(ownerDirectory, name));
        expect(
          stat.isSymbolicLink(),
          `${name} must not be a symbolic link`,
        ).toBe(false);
        expect(stat.isFile(), `${name} must be a regular file`).toBe(true);
        return name;
      })
      .sort();
    const expected = tamilGlyphs.map(ownerFilename).sort();

    expect(actual).toEqual(expected);
  });

  it("keeps direct DUCTUS lookups confined to the matching glyph owner", () => {
    for (const glyph of tamilGlyphs) {
      const name = ownerFilename(glyph);
      const source = readFileSync(resolve(ownerDirectory, name), "utf8");
      const ownedGlyphs = [...source.matchAll(/DUCTUS\["([^"]+)"\]/gu)].map(
        (match) => match[1],
      );

      expect(new Set(ownedGlyphs), name).toEqual(new Set([glyph]));
    }
  });

  it("does not revive the former script-wide Tamil test root", () => {
    expect(existsSync(resolve(ownerDirectory, "../tamil.test.ts"))).toBe(false);
  });
});
