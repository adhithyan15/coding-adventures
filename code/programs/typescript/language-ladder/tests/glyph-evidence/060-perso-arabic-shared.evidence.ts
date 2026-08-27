import { expect } from "vitest";
import type { GlyphEvidence } from "./types";

export default [
  {
    suite: "shared Perso-Arabic letters retain script-owned provenance",
    suiteOrder: 50,
    caseOrder: 10,
    name: "keeps Arabic maddah as sourced alif-plus-mark composition",
    verify: ({ SCRIPTS }) => {
      const arabic = SCRIPTS.find((script) => script.script === "arabic")!;
      const maddah = arabic.marks!.find((entry) => entry.mark === "ٓ")!;
      expect(maddah.sound).toBe("ʾā (long initial ā)");
      expect(maddah.example?.combined).toBe("آ");
      expect(maddah.compositionOrder).toHaveLength(2);
      expect(maddah.compositionOrder?.[0]).toMatch(/alif carrier downward/i);
      expect(maddah.compositionOrder?.[1]).toMatch(/maddah above.*horizontal wave/i);
      expect(maddah.compositionSource?.url).toBe(
        "https://www.unicode.org/versions/Unicode17.0.0/core-spec/chapter-9/",
      );
    },
  },
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
    caseOrder: 70,
    name: "keeps Urdu ھ as one sourced two-eyed aspiration path",
    verify: ({ SCRIPTS }) => {
      const urdu = SCRIPTS.find((script) => script.script === "urdu-nastaliq")!
        .letters.find((entry) => entry.glyph === "ھ")!;
      expect(urdu.role).toBe("other");
      expect(urdu.sound).toMatch(/aspirates the preceding consonant/i);
      expect(urdu.penLifts).toBe(0);
      expect(urdu.strokeOrder).toHaveLength(4);
      expect(urdu.strokeOrder[0]).toMatch(/right eye clockwise/i);
      expect(urdu.strokeOrder[3]).toMatch(/leftward sweep.*without lifting/i);
      expect(urdu.strokeOrderSource?.url).toBe(
        "https://openbooks.library.northwestern.edu/zerozabar/chapter/chhoti-he-do-chashmi-he-chhoti-ye-bari-ye/",
      );
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
    caseOrder: 190,
    name: "keeps Urdu گ as a source-backed three-run kāf-family letter",
    verify: ({ SCRIPTS }) => {
      const urdu = SCRIPTS.find((script) => script.script === "urdu-nastaliq")!
        .letters.find((entry) => entry.glyph === "گ")!;
      expect(urdu.sound).toBe("g");
      expect(urdu.role).toBe("consonant");
      expect(urdu.penLifts).toBe(2);
      expect(urdu.strokeOrder).toHaveLength(4);
      expect(urdu.strokeOrder[1]).toMatch(/right to left.*flatter bowl.*hook/i);
      expect(urdu.strokeOrder[2]).toMatch(/one lift.*long slash/i);
      expect(urdu.strokeOrder[3]).toMatch(/another lift.*shorter floating slash/i);
      expect(urdu.strokeOrderSource?.url).toBe(
        "https://openbooks.library.northwestern.edu/zerozabar/chapter/pe-gaf-alif-lam/",
      );
    },
  },
  {
    suite: "shared Perso-Arabic letters retain script-owned provenance",
    suiteOrder: 50,
    caseOrder: 200,
    name: "keeps Urdu ت as a source-backed bowl-and-two-dots letter",
    verify: ({ SCRIPTS }) => {
      const urdu = SCRIPTS.find((script) => script.script === "urdu-nastaliq")!
        .letters.find((entry) => entry.glyph === "ت")!;
      expect(urdu.sound).toBe("t");
      expect(urdu.role).toBe("consonant");
      expect(urdu.penLifts).toBe(2);
      expect(urdu.strokeOrder).toEqual([
        "sweep the independent be-series bowl from right to left",
        "after one lift, place the left dot above the main line",
        "after another lift, place the right dot beside it",
      ]);
      expect(urdu.strokeOrderSource?.url).toBe(
        "https://openbooks.library.northwestern.edu/zerozabar/chapter/te-mim-jim-che/",
      );
    },
  },
  {
    suite: "shared Perso-Arabic letters retain script-owned provenance",
    suiteOrder: 50,
    caseOrder: 210,
    name: "keeps Urdu ئ as a source-backed carrier-plus-hamza composition",
    verify: ({ SCRIPTS }) => {
      const hamza = SCRIPTS.find((script) => script.script === "urdu-nastaliq")!
        .marks!.find((entry) => entry.mark === "ٔ")!;
      expect(hamza.examples?.map((example) => example.combined)).toEqual(["ئ"]);
      expect(hamza.compositionOrder?.[0]).toMatch(/tooth carrier.*right-to-left main line/i);
      expect(hamza.compositionOrder?.[1]).toMatch(/after lifting.*hamza above/i);
      expect(hamza.compositionSource?.url).toBe(
        "https://openbooks.library.northwestern.edu/zerozabar/chapter/ain-hamza/",
      );
      expect(hamza.compositionSource?.variation).toMatch(/بھائی.*carrier-plus-mark/i);
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
] satisfies readonly GlyphEvidence[];


