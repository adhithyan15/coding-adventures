// Japanese cited glyphs stand apart in the sequence model used by the book.
import type { FigureTargetPolicy } from "../figure-target-policy-types.js";

export const japaneseFigureTargetPolicy: FigureTargetPolicy = {
  script: "japanese",
  tracks: ["japanese"],
  separateLetters: 20,
};
