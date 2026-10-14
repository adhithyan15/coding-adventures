// Chinese characters stand apart in derived sequences.
import type { FigureTargetPolicy } from "../figure-target-policy-types.js";

export const chineseFigureTargetPolicy: FigureTargetPolicy = {
  script: "chinese",
  tracks: ["chinese"],
  separateLetters: 10,
};
