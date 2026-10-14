// Latin print letters stand apart; joined handwriting is not derived here.
import type { FigureTargetPolicy } from "../figure-target-policy-types.js";

export const latinFigureTargetPolicy: FigureTargetPolicy = {
  script: "latin",
  tracks: ["spanish", "german", "french", "italian", "portuguese", "latin"],
  separateLetters: 80,
};
