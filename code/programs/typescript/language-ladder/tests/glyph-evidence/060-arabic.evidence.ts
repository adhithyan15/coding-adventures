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
] satisfies readonly GlyphEvidence[];
