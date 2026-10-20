// Arabic is activated for isolated letters and lists; words change shape.
import type { FigureTargetPolicy } from "../figure-target-policy-types.js";

export const arabicFigureTargetPolicy: FigureTargetPolicy = {
  script: "arabic",
  tracks: ["arabic"],
};
