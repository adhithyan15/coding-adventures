import { describe, it, expect } from "vitest";
import { SCRIPTS } from "../src/data";
import { crossScriptSiblings } from "../src/siblings";
import { isSyllabary } from "../src/syllabary";
import { buildSyllableMatrix } from "../src/matrix";

describe("cross-script siblings (the same syllable in the sister scripts)", () => {
  it("Telugu “ki” → the real Kannada and Malayalam glyphs, matched by sound", () => {
    const sibs = crossScriptSiblings("ki", "telugu", SCRIPTS);
    // Two siblings, in SCRIPTS order (kannada before malayalam), never Telugu itself.
    expect(sibs.map((s) => s.script)).toEqual(["kannada", "malayalam"]);
    const by = Object.fromEntries(sibs.map((s) => [s.script, s]));
    expect(by.kannada.glyph).toBe("ಕಿ");
    expect(by.kannada.name).toBe("Kannada");
    expect(by.malayalam.glyph).toBe("കി");
    // The shared romanization rides along unchanged.
    expect(sibs.every((s) => s.sound === "ki")).toBe(true);
  });

  it("works symmetrically from any of the trio (Malayalam “kha” → Telugu + Kannada)", () => {
    const sibs = crossScriptSiblings("kha", "malayalam", SCRIPTS);
    expect(sibs.map((s) => s.script)).toEqual(["kannada", "telugu"]);
    expect(sibs.find((s) => s.script === "telugu")!.glyph).toBe("ఖ");
  });

  it("a syllable with no counterpart yields no siblings (Malayalam-only ṉa row)", () => {
    // The alveolar-n row (ṉa, ṉi, …) exists only in Malayalam — Telugu/Kannada
    // have no such consonant, so there is nothing to show, correctly.
    expect(crossScriptSiblings("ṉa", "malayalam", SCRIPTS)).toEqual([]);
  });

  it("CONTROL: only the syllabary trio are siblings — an alphabet is never matched", () => {
    // Cyrillic “a” must not pull in any Dravidian syllable, nor vice versa.
    expect(crossScriptSiblings("a", "cyrillic", SCRIPTS)).toEqual([]);
    // And the source script never appears in its own sibling list.
    const sibs = crossScriptSiblings("ka", "kannada", SCRIPTS);
    expect(sibs.some((s) => s.script === "kannada")).toBe(false);
    expect(sibs.map((s) => s.script)).toEqual(["telugu", "malayalam"]);
  });

  it("CONTROL: siblings are read-only — letters, isSyllabary and the matrix are untouched", () => {
    const telugu = SCRIPTS.find((s) => s.script === "telugu")!;
    const before = telugu.letters.length;
    crossScriptSiblings("ki", "telugu", SCRIPTS);
    expect(telugu.letters.length).toBe(before);
    expect(isSyllabary(telugu.letters)).toBe(true);
    expect(buildSyllableMatrix(telugu.letters as never)!.rows.length).toBe(35);
  });
});
