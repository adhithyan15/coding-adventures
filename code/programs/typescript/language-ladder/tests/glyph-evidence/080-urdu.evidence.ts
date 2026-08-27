import { expect } from "vitest";
import type { GlyphEvidence } from "./types";

export default [
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
] satisfies readonly GlyphEvidence[];
