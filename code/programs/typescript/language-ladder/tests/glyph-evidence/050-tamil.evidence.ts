import { expect } from "vitest";
import type { GlyphEvidence } from "./types";

export default [
  {
    suite: "Tamil independent vowels in the starter inventory",
    suiteOrder: 40,
    caseOrder: 10,
    name: "keeps short உ sourced as one joined Frame 16 run",
    verify: ({ SCRIPTS }) => {
      const tamil = SCRIPTS.find((script) => script.script === "tamil")!;
      const shortU = tamil.letters.find((entry) => entry.glyph === "உ")!;
      expect(shortU.role).toBe("independent-vowel");
      expect(shortU.sound).toBe("u");
      expect(shortU.penLifts).toBe(0);
      expect(shortU.strokeOrder).toHaveLength(3);
      expect(shortU.strokeOrderSource?.url).toBe(
        "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      );
    },
  },
] satisfies readonly GlyphEvidence[];
