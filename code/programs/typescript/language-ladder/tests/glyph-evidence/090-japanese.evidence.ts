import { expect } from "vitest";
import type { GlyphEvidence } from "./types";

export default [
  {
    suite: "shared Perso-Arabic letters retain script-owned provenance",
    suiteOrder: 50,
    caseOrder: 90,
    name: "keeps Japanese ね as a source-backed two-run hiragana path",
    verify: ({ SCRIPTS }) => {
      const japanese = SCRIPTS.find((script) => script.script === "japanese")!
        .letters.find((entry) => entry.glyph === "ね")!;
      expect(japanese.sound).toBe("ne");
      expect(japanese.role).toBe("hiragana");
      expect(japanese.penLifts).toBe(1);
      expect(japanese.strokeOrder).toHaveLength(2);
      expect(japanese.strokeOrder[0]).toMatch(/short left vertical/i);
      expect(japanese.strokeOrder[1]).toMatch(/upper right.*sweep left.*diagonal.*return.*lower-right loop/i);
      expect(japanese.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Hiragana_%E3%81%AD_stroke_order_animation.gif",
      );
    },
  },
  {
    suite: "shared Perso-Arabic letters retain script-owned provenance",
    suiteOrder: 50,
    caseOrder: 100,
    name: "keeps Japanese み as a source-backed two-run hiragana path",
    verify: ({ SCRIPTS }) => {
      const japanese = SCRIPTS.find((script) => script.script === "japanese")!
        .letters.find((entry) => entry.glyph === "み")!;
      expect(japanese.sound).toBe("mi");
      expect(japanese.role).toBe("hiragana");
      expect(japanese.penLifts).toBe(1);
      expect(japanese.strokeOrder).toHaveLength(2);
      expect(japanese.strokeOrder[0]).toMatch(/top bar.*lower-left loop.*middle/i);
      expect(japanese.strokeOrder[1]).toMatch(/high on the right.*curve down and left.*turning upward/i);
      expect(japanese.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Hiragana_%E3%81%BF_stroke_order_animation.gif",
      );
    },
  },
  {
    suite: "shared Perso-Arabic letters retain script-owned provenance",
    suiteOrder: 50,
    caseOrder: 110,
    name: "keeps Japanese せ as a source-backed three-run hiragana path",
    verify: ({ SCRIPTS }) => {
      const japanese = SCRIPTS.find((script) => script.script === "japanese")!
        .letters.find((entry) => entry.glyph === "せ")!;
      expect(japanese.sound).toBe("se");
      expect(japanese.role).toBe("hiragana");
      expect(japanese.penLifts).toBe(2);
      expect(japanese.strokeOrder).toHaveLength(3);
      expect(japanese.strokeOrder[0]).toMatch(/long crossing horizontal.*left to right/i);
      expect(japanese.strokeOrder[1]).toMatch(/left crossing.*curve right along the base/i);
      expect(japanese.strokeOrder[2]).toMatch(/right crossing.*hook left/i);
      expect(japanese.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Hiragana_%E3%81%9B_stroke_order_animation.gif",
      );
    },
  },
  {
    suite: "shared Perso-Arabic letters retain script-owned provenance",
    suiteOrder: 50,
    caseOrder: 120,
    name: "keeps Japanese て as a source-backed one-run hiragana path",
    verify: ({ SCRIPTS }) => {
      const japanese = SCRIPTS.find((script) => script.script === "japanese")!
        .letters.find((entry) => entry.glyph === "て")!;
      expect(japanese.sound).toBe("te");
      expect(japanese.role).toBe("hiragana");
      expect(japanese.penLifts).toBe(0);
      expect(japanese.strokeOrder).toHaveLength(3);
      expect(japanese.strokeOrder[0]).toMatch(/high horizontal.*left to right/i);
      expect(japanese.strokeOrder[1]).toMatch(/without lifting.*down and left.*diagonal/i);
      expect(japanese.strokeOrder[2]).toMatch(/without lifting.*lower curve.*right/i);
      expect(japanese.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Hiragana_%E3%81%A6_stroke_order_animation.gif",
      );
    },
  },
  {
    suite: "shared Perso-Arabic letters retain script-owned provenance",
    suiteOrder: 50,
    caseOrder: 130,
    name: "keeps Japanese な as a source-backed four-run hiragana path",
    verify: ({ SCRIPTS }) => {
      const japanese = SCRIPTS.find((script) => script.script === "japanese")!
        .letters.find((entry) => entry.glyph === "な")!;
      expect(japanese.sound).toBe("na");
      expect(japanese.role).toBe("hiragana");
      expect(japanese.penLifts).toBe(3);
      expect(japanese.strokeOrder).toHaveLength(4);
      expect(japanese.strokeOrder[0]).toMatch(/upper-left horizontal.*left to right/i);
      expect(japanese.strokeOrder[1]).toMatch(/crossing left-falling stem/i);
      expect(japanese.strokeOrder[2]).toMatch(/upper-right diagonal.*down and right/i);
      expect(japanese.strokeOrder[3]).toMatch(/lower-right stem.*loop.*right/i);
      expect(japanese.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Hiragana_%E3%81%AA_stroke_order_animation.gif",
      );
    },
  },
  {
    suite: "shared Perso-Arabic letters retain script-owned provenance",
    suiteOrder: 50,
    caseOrder: 140,
    name: "keeps Japanese わ as a source-backed two-run hiragana path",
    verify: ({ SCRIPTS }) => {
      const japanese = SCRIPTS.find((script) => script.script === "japanese")!
        .letters.find((entry) => entry.glyph === "わ")!;
      expect(japanese.sound).toBe("wa");
      expect(japanese.role).toBe("hiragana");
      expect(japanese.penLifts).toBe(1);
      expect(japanese.strokeOrder).toHaveLength(2);
      expect(japanese.strokeOrder[0]).toMatch(/long left vertical/i);
      expect(japanese.strokeOrder[1]).toMatch(/upper left.*sweep right.*hook down and left.*central crossing.*right loop/i);
      expect(japanese.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Hiragana_%E3%82%8F_stroke_order_animation.gif",
      );
    },
  },
  {
    suite: "shared Perso-Arabic letters retain script-owned provenance",
    suiteOrder: 50,
    caseOrder: 150,
    name: "keeps Japanese ゆ as a source-backed two-run hiragana path",
    verify: ({ SCRIPTS }) => {
      const japanese = SCRIPTS.find((script) => script.script === "japanese")!
        .letters.find((entry) => entry.glyph === "ゆ")!;
      expect(japanese.sound).toBe("yu");
      expect(japanese.role).toBe("hiragana");
      expect(japanese.penLifts).toBe(1);
      expect(japanese.strokeOrder).toHaveLength(2);
      expect(japanese.strokeOrder[0]).toMatch(/left stem.*high shoulder.*clockwise.*broad loop.*inner finish/i);
      expect(japanese.strokeOrder[1]).toMatch(/high above.*center.*down and left/i);
      expect(japanese.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Hiragana_%E3%82%86_stroke_order_animation.gif",
      );
    },
  },
] satisfies readonly GlyphEvidence[];
