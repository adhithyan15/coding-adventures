import { describe, it, expect } from "vitest";
import {
  isFalseFriend,
  specialConsonant,
  toLetterView,
  buildScriptView,
  scriptSummary,
  falseFriends,
} from "../src/core.ts";
import type { Letter, ScriptData } from "../src/types.ts";
import { SCRIPTS } from "../src/data.ts";

function letter(overrides: Partial<Letter> = {}): Letter {
  return {
    glyph: "x",
    sound: "x",
    role: "consonant",
    components: ["a", "b"],
    strokeOrder: ["1", "2", "3"],
    strokeOrderNote: "conventional",
    ...overrides,
  };
}

function script(overrides: Partial<ScriptData> = {}): ScriptData {
  return {
    script: "test",
    name: "Test",
    font: "",
    direction: "ltr",
    system: "alphabet",
    letters: [],
    ...overrides,
  };
}

describe("isFalseFriend", () => {
  it("detects the FALSE FRIEND marker case-insensitively", () => {
    expect(isFalseFriend({ notes: "FALSE FRIEND: looks like B, says v" })).toBe(true);
    expect(isFalseFriend({ notes: "a false friend of Latin p" })).toBe(true);
  });
  it("is false without the marker or without notes", () => {
    expect(isFalseFriend({ notes: "From Greek delta." })).toBe(false);
    expect(isFalseFriend({ notes: "" })).toBe(false);
    expect(isFalseFriend({})).toBe(false);
  });
});

describe("specialConsonant", () => {
  it("flags the retroflex/alveolar special consonants by their ISO-15919 mark", () => {
    expect(specialConsonant({ sound: "ḷa" })?.plain).toBe("l"); // U+1E37 dot below
    expect(specialConsonant({ sound: "ḷī" })?.plain).toBe("l"); // signed form too
    expect(specialConsonant({ sound: "ṟa" })?.plain).toBe("r"); // U+1E5F line below
    expect(specialConsonant({ sound: "ṉa" })?.plain).toBe("n"); // U+1E49 line below
    expect(specialConsonant({ sound: "ḷa" })?.hint).toMatch(/retroflex/i);
  });
  it("CONTROL: the ordinary l / r / n and the ring-below vocalic r̥ are NOT special", () => {
    expect(specialConsonant({ sound: "la" })).toBeNull();
    expect(specialConsonant({ sound: "ra" })).toBeNull();
    expect(specialConsonant({ sound: "na" })).toBeNull();
    expect(specialConsonant({ sound: "kr̥" })).toBeNull(); // r + U+0325 ring, a vowel — not ṟ
    expect(specialConsonant({ sound: "" })).toBeNull();
  });
  it("marks exactly the LLA/RRA/NNNA rows in the real generated data", () => {
    // Telugu has ḷa and ṟa (no ṉa); every one of their 13 syllables is flagged,
    // and nothing else in the 455-syllable inventory is.
    const telugu = SCRIPTS.find((s) => s.script === "telugu")!;
    const flagged = telugu.letters.filter((l) => specialConsonant(l) !== null);
    expect(flagged.length).toBe(26); // 2 special consonants × 13 vowels
    expect(new Set(flagged.map((l) => specialConsonant(l)!.plain))).toEqual(new Set(["l", "r"]));
  });
});

describe("toLetterView", () => {
  it("maps fields and computes strokeCount + falseFriend", () => {
    const v = toLetterView(letter({ glyph: "в", sound: "v", notes: "FALSE FRIEND" }));
    expect(v.glyph).toBe("в");
    expect(v.strokeCount).toBe(3);
    expect(v.falseFriend).toBe(true);
    expect(v.components).toEqual(["a", "b"]);
  });

  it("tolerates missing components/strokeOrder/notes", () => {
    const bare = { glyph: "o", sound: "o", role: "vowel", strokeOrderNote: "x" } as unknown as Letter;
    const v = toLetterView(bare);
    expect(v.components).toEqual([]);
    expect(v.strokeOrder).toEqual([]);
    expect(v.strokeCount).toBe(0);
    expect(v.notes).toBe("");
    expect(v.falseFriend).toBe(false);
  });

  it("passes through tone and inherentVowel when present", () => {
    const v = toLetterView(letter({ tone: "1", inherentVowel: "a" }));
    expect(v.tone).toBe("1");
    expect(v.inherentVowel).toBe("a");
  });
});

describe("buildScriptView", () => {
  it("preserves inventory order and length", () => {
    const data = script({
      letters: [letter({ glyph: "a" }), letter({ glyph: "b" }), letter({ glyph: "c" })],
    });
    const views = buildScriptView(data);
    expect(views.map((v) => v.glyph)).toEqual(["a", "b", "c"]);
  });
});

describe("scriptSummary", () => {
  it("counts letters and false friends and defaults complete to false", () => {
    const data = script({
      letters: [
        letter({ glyph: "a" }),
        letter({ glyph: "в", notes: "FALSE FRIEND" }),
        letter({ glyph: "р", notes: "false friend of p" }),
      ],
    });
    const s = scriptSummary(data);
    expect(s.letterCount).toBe(3);
    expect(s.falseFriendCount).toBe(2);
    expect(s.complete).toBe(false);
    expect(s.direction).toBe("ltr");
  });

  it("honors an explicit complete flag", () => {
    expect(scriptSummary(script({ complete: true })).complete).toBe(true);
  });
});

describe("falseFriends", () => {
  it("returns only flagged letters, in order", () => {
    const data = script({
      letters: [letter({ glyph: "a" }), letter({ glyph: "в", notes: "FALSE FRIEND" })],
    });
    expect(falseFriends(data).map((v) => v.glyph)).toEqual(["в"]);
  });
});

// --- integration with the real curriculum data ------------------------------

describe("real script data", () => {
  it("ships at least the five expected scripts", () => {
    const names = SCRIPTS.map((s) => s.script);
    expect(names).toEqual(expect.arrayContaining(["cyrillic", "hebrew", "chinese", "arabic", "devanagari"]));
  });

  it("every letter of every script has a glyph, components, and stroke order", () => {
    for (const data of SCRIPTS) {
      const views = buildScriptView(data);
      expect(views.length).toBeGreaterThan(0);
      for (const v of views) {
        expect(v.glyph.length).toBeGreaterThan(0);
        expect(Array.isArray(v.components)).toBe(true);
        expect(Array.isArray(v.strokeOrder)).toBe(true);
      }
    }
  });

  it("every script carries an identification signature (its at-a-glance tell)", () => {
    // Powers a "spot the script" identification mode. Each signature was
    // written against the rendered font, not from memory.
    for (const data of SCRIPTS) {
      expect(data.signature, `${data.script} is missing its identification signature`).toBeTruthy();
      expect((data.signature ?? "").length, `${data.script} signature is too short to be a real tell`).toBeGreaterThan(20);
    }
  });

  it("Cyrillic flags its Latin-lookalike false friends (в, р, с, н)", () => {
    const cyr = SCRIPTS.find((s) => s.script === "cyrillic")!;
    const ffGlyphs = falseFriends(cyr).map((v) => v.glyph);
    for (const g of ["в", "р", "с", "н"]) {
      expect(ffGlyphs).toContain(g);
    }
  });
});
