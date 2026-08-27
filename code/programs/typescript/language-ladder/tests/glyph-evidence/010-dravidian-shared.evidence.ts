import { expect } from "vitest";
import type { GlyphEvidence } from "./types";

const DRAVIDIAN = ["telugu", "kannada", "malayalam"] as const;

export default [
  {
    suite: "independent (word-initial) vowels",
    suiteOrder: 10,
    caseOrder: 20,
    name: "all three Dravidian scripts carry them",
    verify: ({ SCRIPTS }) => {
      DRAVIDIAN.forEach((id) => {
        const s = SCRIPTS.find((x) => x.script === id)!;
        expect(s.independentVowels?.length).toBe(13);
      });
    },
  },
  {
    suite: "independent (word-initial) vowels",
    suiteOrder: 10,
    caseOrder: 60,
    name: "an alphabet (Cyrillic) has no independent-vowel list",
    verify: ({ SCRIPTS }) => {
      const cyr = SCRIPTS.find((s) => s.script === "cyrillic")!;
      expect(cyr.independentVowels).toBeUndefined();
    },
  },
] satisfies readonly GlyphEvidence[];
