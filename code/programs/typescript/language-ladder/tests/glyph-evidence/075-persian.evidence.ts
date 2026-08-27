import { expect } from "vitest";
import type { GlyphEvidence } from "./types";

export default [
  {
    suite: "shared Perso-Arabic letters retain script-owned provenance",
    suiteOrder: 50,
    caseOrder: 195,
    name: "keeps Persian گ as an independently sourced three-run kāf-family letter",
    verify: ({ SCRIPTS }) => {
      const persian = SCRIPTS.find((script) => script.script === "perso-arabic")!
        .letters.find((entry) => entry.glyph === "گ")!;
      expect(persian.sound).toBe("g");
      expect(persian.role).toBe("consonant");
      expect(persian.penLifts).toBe(2);
      expect(persian.strokeOrder).toHaveLength(3);
      expect(persian.strokeOrder[0]).toMatch(/stem downward.*shallow bowl.*hook/i);
      expect(persian.strokeOrder[1]).toMatch(/lift once.*long slash/i);
      expect(persian.strokeOrder[2]).toMatch(/lift again.*shorter floating slash/i);
      expect(persian.strokeOrderSource?.url).toBe(
        "https://laits.utexas.edu/persian_grammar/video/gr/kooroshalphabet",
      );
    },
  },
] satisfies readonly GlyphEvidence[];
