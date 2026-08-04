import { describe, it, expect } from "vitest";
import { SCRIPTS } from "../src/data";
import type { ScriptData } from "../src/types";

// The three Dravidian syllabaries generated from Unicode by
// data/scripts/generate_syllabary.py. These tests ground the GLYPHS against the
// exact Unicode code points (not pasted glyph literals — those would risk the
// very mistyping the generator exists to avoid): a syllable must be the base
// consonant composed with the vowel sign, or it fails here rather than reaching
// a native reader.

function byId(id: string): ScriptData {
  const s = SCRIPTS.find((x) => x.script === id);
  if (!s) throw new Error(`script ${id} not registered in data.ts`);
  return s;
}

// KA and the "i" vowel-sign code points, per script's Unicode block.
const cp = String.fromCodePoint;
const BLOCKS = {
  telugu: { ka: 0x0c15, iSign: 0x0c3f },
  kannada: { ka: 0x0c95, iSign: 0x0cbf },
  malayalam: { ka: 0x0d15, iSign: 0x0d3f },
} as const;
const IDS = ["telugu", "kannada", "malayalam"] as const;

describe("Dravidian syllabaries are registered and abugida-shaped", () => {
  for (const id of IDS) {
    it(`${id} is present, an abugida, with a full core grid`, () => {
      const s = byId(id);
      expect(s.system).toBe("abugida");
      expect(s.direction).toBe("ltr");
      expect(s.letters.length).toBeGreaterThanOrEqual(340); // ~35 consonants × 10 vowels
      expect((s.signature ?? "").length).toBeGreaterThan(20);
    });
  }
});

describe("every syllable is recognition-ready and ductus-free", () => {
  for (const id of IDS) {
    it(`${id}: non-empty glyph + sound, role 'syllable', and NO stroke order`, () => {
      for (const l of byId(id).letters) {
        expect(l.glyph.length).toBeGreaterThan(0);
        expect(l.sound.length).toBeGreaterThan(0);
        expect(l.role).toBe("syllable");
        // CONTROL: stroke order stays paused (recognition only). If ductus data
        // ever leaked into the generated syllabary, this fails.
        expect(l.strokeOrder).toEqual([]);
        expect(l.components.length).toBeGreaterThan(0);
      }
    });
  }
});

describe("glyphs are composed from the real Unicode code points (grounding control)", () => {
  for (const id of IDS) {
    it(`${id}: 'ka' is the block's KA, and 'ki' is KA + the i-sign`, () => {
      const letters = byId(id).letters;
      const ka = letters.find((l) => l.sound === "ka")!;
      const ki = letters.find((l) => l.sound === "ki")!;
      const { ka: kaCp, iSign } = BLOCKS[id];
      // CONTROL: a hand-typed or mis-composed glyph — or a look-alike KA from the
      // wrong Dravidian block — would not equal these exact code points.
      expect(ka.glyph).toBe(cp(kaCp));
      expect(ki.glyph).toBe(cp(kaCp) + cp(iSign));
    });
  }
});

describe("the ka/kha rows read as the user expects (ka ki ku, kha khi khu)", () => {
  it("telugu composes the ka and kha rows correctly from code points", () => {
    const t = byId("telugu");
    const g = (sound: string) => t.letters.find((l) => l.sound === sound)?.glyph;
    const KA = cp(0x0c15), KHA = cp(0x0c16);
    const I = cp(0x0c3f), U = cp(0x0c41);
    expect(g("ka")).toBe(KA);
    expect(g("ki")).toBe(KA + I);
    expect(g("ku")).toBe(KA + U);
    expect(g("kha")).toBe(KHA);
    expect(g("khi")).toBe(KHA + I);
    expect(g("khu")).toBe(KHA + U);
  });
});
