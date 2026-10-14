// Cyrillic school hand remains isolated-letter/list only because words join.
import type { FigureTargetPolicy } from "../figure-target-policy-types.js";

export const cyrillicFigureTargetPolicy: FigureTargetPolicy = {
  script: "cyrillic",
  tracks: ["russian"],
};
