import { expect } from "vitest";
import type { GlyphEvidence } from "./types";

export default [
  {
    suite: "shared Perso-Arabic letters retain script-owned provenance",
    suiteOrder: 50,
    caseOrder: 20,
    name: "keeps Persian and Urdu maddah on each script's sourced alif carrier",
    verify: ({ SCRIPTS }) => {
      const persian = SCRIPTS.find((script) => script.script === "perso-arabic")!
        .marks!.find((entry) => entry.mark === "ٓ")!;
      const urdu = SCRIPTS.find((script) => script.script === "urdu-nastaliq")!
        .marks!.find((entry) => entry.mark === "ٓ")!;
      expect(persian.example?.combined).toBe("آ");
      expect(urdu.example?.combined).toBe("آ");
      expect(persian.compositionOrder?.[0]).toMatch(/Persian alif carrier downward/i);
      expect(urdu.compositionOrder?.[0]).toMatch(/Urdu alif carrier downward/i);
      expect(persian.compositionSource?.url).toBe(urdu.compositionSource?.url);
      expect(persian.compositionSource?.variation).toMatch(/Persian curriculum/i);
      expect(urdu.compositionSource?.variation).toMatch(/Urdu curriculum.*Nastaliq/i);
    },
  },
  {
    suite: "shared Perso-Arabic letters retain script-owned provenance",
    suiteOrder: 50,
    caseOrder: 30,
    name: "keeps Persian and Urdu چ body-first with independently sourced provenance",
    verify: ({ SCRIPTS }) => {
      const persian = SCRIPTS.find((script) => script.script === "perso-arabic")!
        .letters.find((entry) => entry.glyph === "چ")!;
      const urdu = SCRIPTS.find((script) => script.script === "urdu-nastaliq")!
        .letters.find((entry) => entry.glyph === "چ")!;
      expect(persian.sound).toBe("ch");
      expect(persian.penLifts).toBe(3);
      expect(persian.strokeOrder).toHaveLength(5);
      expect(persian.strokeOrder[0]).toMatch(/head.*left to right/i);
      expect(persian.strokeOrder[2]).toMatch(/lower-left dot/i);
      expect(persian.strokeOrder[4]).toMatch(/lower-center dot/i);
      expect(persian.strokeOrderSource?.url).toContain(
        "laits.utexas.edu/persian_grammar/video",
      );
      expect(urdu.sound).toBe("ch");
      expect(urdu.penLifts).toBe(3);
      expect(urdu.strokeOrder).toHaveLength(5);
      expect(urdu.strokeOrder[0]).toMatch(/pointed hooked head/i);
      expect(urdu.strokeOrder[2]).toMatch(/lower-left dot/i);
      expect(urdu.strokeOrder[4]).toMatch(/lower-center dot/i);
      expect(urdu.strokeOrderSource?.url).toBe(
        "https://openbooks.library.northwestern.edu/zerozabar/chapter/te-mim-jim-che/",
      );
      expect(persian.strokeOrderSource?.url).not.toBe(urdu.strokeOrderSource?.url);
    },
  },
  {
    suite: "shared Perso-Arabic letters retain script-owned provenance",
    suiteOrder: 50,
    caseOrder: 40,
    name: "keeps Persian and Urdu ح zero-lift with independently sourced provenance",
    verify: ({ SCRIPTS }) => {
      const persian = SCRIPTS.find((script) => script.script === "perso-arabic")!
        .letters.find((entry) => entry.glyph === "ح")!;
      const urdu = SCRIPTS.find((script) => script.script === "urdu-nastaliq")!
        .letters.find((entry) => entry.glyph === "ح")!;
      expect(persian.sound).toBe("h");
      expect(urdu.sound).toBe("h");
      expect(persian.penLifts).toBe(0);
      expect(urdu.penLifts).toBe(0);
      expect(persian.strokeOrder).toHaveLength(2);
      expect(urdu.strokeOrder).toHaveLength(2);
      expect(persian.strokeOrder[0]).toMatch(/head.*left to right/i);
      expect(urdu.strokeOrder[0]).toMatch(/pointed hooked head/i);
      expect(persian.strokeOrderSource?.url).not.toBe(urdu.strokeOrderSource?.url);
      expect(urdu.notes).toMatch(/baṛī he.*Arabic-derived.*chhoṭī he.*do-chashmī he/i);
    },
  },
  {
    suite: "shared Perso-Arabic letters retain script-owned provenance",
    suiteOrder: 50,
    caseOrder: 50,
    name: "keeps Persian and Urdu ق body-first with independently sourced provenance",
    verify: ({ SCRIPTS }) => {
      const persian = SCRIPTS.find((script) => script.script === "perso-arabic")!
        .letters.find((entry) => entry.glyph === "ق")!;
      const urdu = SCRIPTS.find((script) => script.script === "urdu-nastaliq")!
        .letters.find((entry) => entry.glyph === "ق")!;
      for (const letter of [persian, urdu]) {
        expect(letter.penLifts).toBe(2);
        expect(letter.strokeOrder).toHaveLength(4);
        expect(letter.strokeOrder[1]).toMatch(/deep bowl.*without lifting/i);
        expect(letter.strokeOrder[2]).toMatch(/upper-right dot/i);
        expect(letter.strokeOrder[3]).toMatch(/upper-left dot/i);
      }
      expect(persian.strokeOrderSource?.url).not.toBe(urdu.strokeOrderSource?.url);
    },
  },
  {
    suite: "shared Perso-Arabic letters retain script-owned provenance",
    suiteOrder: 50,
    caseOrder: 60,
    name: "keeps Persian and Urdu ط body-first with independently sourced provenance",
    verify: ({ SCRIPTS }) => {
      const persian = SCRIPTS.find((script) => script.script === "perso-arabic")!
        .letters.find((entry) => entry.glyph === "ط")!;
      const urdu = SCRIPTS.find((script) => script.script === "urdu-nastaliq")!
        .letters.find((entry) => entry.glyph === "ط")!;
      for (const letter of [persian, urdu]) {
        expect(letter.penLifts).toBe(1);
        expect(letter.strokeOrder).toHaveLength(2);
        expect(letter.strokeOrder[0]).toMatch(/loop|oval/i);
        expect(letter.strokeOrder[1]).toMatch(/lift.*upright/i);
      }
      expect(persian.strokeOrderSource?.url).not.toBe(urdu.strokeOrderSource?.url);
    },
  },
  {
    suite: "shared Perso-Arabic letters retain script-owned provenance",
    suiteOrder: 50,
    caseOrder: 80,
    name: "keeps Arabic, Persian, and Urdu ب separate while Urdu preserves main-line-first order",
    verify: ({ SCRIPTS }) => {
      const arabic = SCRIPTS.find((script) => script.script === "arabic")!
        .letters.find((entry) => entry.glyph === "ب")!;
      const persian = SCRIPTS.find((script) => script.script === "perso-arabic")!
        .letters.find((entry) => entry.glyph === "ب")!;
      const urdu = SCRIPTS.find((script) => script.script === "urdu-nastaliq")!
        .letters.find((entry) => entry.glyph === "ب")!;
      expect(urdu.penLifts).toBe(1);
      expect(urdu.strokeOrder).toEqual([
        "sweep the independent be-series bowl from right to left",
        "after one lift, place the single dot below",
      ]);
      expect(urdu.strokeOrderSource?.url).toBe(
        "https://openbooks.library.northwestern.edu/zerozabar/chapter/be-kaf-and-short-vowels/",
      );
      expect(new Set([
        arabic.strokeOrderSource?.url,
        persian.strokeOrderSource?.url,
        urdu.strokeOrderSource?.url,
      ]).size).toBe(3);
    },
  },
  {
    suite: "shared Perso-Arabic letters retain script-owned provenance",
    suiteOrder: 50,
    caseOrder: 220,
    name: "keeps Persian and Urdu پ separate while both preserve the four-stroke triangle order",
    verify: ({ SCRIPTS }) => {
      const persian = SCRIPTS.find((script) => script.script === "perso-arabic")!
        .letters.find((entry) => entry.glyph === "پ")!;
      const urdu = SCRIPTS.find((script) => script.script === "urdu-nastaliq")!
        .letters.find((entry) => entry.glyph === "پ")!;
      expect(persian.penLifts).toBe(3);
      expect(urdu.penLifts).toBe(3);
      expect(persian.strokeOrder).toHaveLength(4);
      expect(urdu.strokeOrder).toHaveLength(4);
      expect(urdu.strokeOrder[1]).toMatch(/lower-left dot.*main line/i);
      expect(urdu.strokeOrder[3]).toMatch(/lower-center dot/i);
      expect(persian.strokeOrderSource?.url).not.toBe(urdu.strokeOrderSource?.url);
    },
  },
  {
    suite: "shared Perso-Arabic letters retain script-owned provenance",
    suiteOrder: 50,
    caseOrder: 230,
    name: "keeps Persian and Urdu ف separate while both use one lifted upper dot",
    verify: ({ SCRIPTS }) => {
      const persian = SCRIPTS.find((script) => script.script === "perso-arabic")!
        .letters.find((entry) => entry.glyph === "ف")!;
      const urdu = SCRIPTS.find((script) => script.script === "urdu-nastaliq")!
        .letters.find((entry) => entry.glyph === "ف")!;
      expect(persian.penLifts).toBe(1);
      expect(urdu.penLifts).toBe(1);
      expect(persian.strokeOrder).toHaveLength(3);
      expect(urdu.strokeOrder).toHaveLength(3);
      expect(persian.strokeOrderSource?.url).not.toBe(urdu.strokeOrderSource?.url);
    },
  },
  {
    suite: "shared Perso-Arabic letters retain script-owned provenance",
    suiteOrder: 50,
    caseOrder: 240,
    name: "keeps Persian and Urdu ی as separately sourced zero-lift independent forms",
    verify: ({ SCRIPTS }) => {
      const persian = SCRIPTS.find((script) => script.script === "perso-arabic")!
        .letters.find((entry) => entry.glyph === "ی")!;
      const urdu = SCRIPTS.find((script) => script.script === "urdu-nastaliq")!
        .letters.find((entry) => entry.glyph === "ی")!;
      expect(persian.penLifts).toBe(0);
      expect(urdu.penLifts).toBe(0);
      expect(persian.strokeOrder).toHaveLength(2);
      expect(urdu.strokeOrder).toHaveLength(2);
      expect(persian.strokeOrder[0]).toMatch(/upper right.*S curve/i);
      expect(urdu.strokeOrder[0]).toMatch(/upper right.*S curve/i);
      expect(persian.strokeOrderSource?.url).not.toBe(urdu.strokeOrderSource?.url);
    },
  },
  {
    suite: "shared Perso-Arabic letters retain script-owned provenance",
    suiteOrder: 50,
    caseOrder: 250,
    name: "keeps Persian and Urdu ز as independently sourced body-and-dot letters",
    verify: ({ SCRIPTS }) => {
      const persian = SCRIPTS.find((script) => script.script === "perso-arabic")!
        .letters.find((entry) => entry.glyph === "ز")!;
      const urdu = SCRIPTS.find((script) => script.script === "urdu-nastaliq")!
        .letters.find((entry) => entry.glyph === "ز")!;
      for (const entry of [persian, urdu]) {
        expect(entry.sound).toBe("z");
        expect(entry.role).toBe("consonant");
        expect(entry.penLifts).toBe(1);
        expect(entry.strokeOrder).toHaveLength(3);
        expect(entry.strokeOrder[1]).toMatch(
          /(?:curv.*left.*without lifting|without lifting.*left.*curve)/i,
        );
        expect(entry.strokeOrder[2]).toMatch(/lift.*dot above/i);
      }
      expect(persian.strokeOrderSource?.url).not.toBe(urdu.strokeOrderSource?.url);
    },
  },
] satisfies readonly GlyphEvidence[];
