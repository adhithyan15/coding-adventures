// Urdu Nastaliq is activated for isolated letters and lists; words join.
import type { FigureTargetPolicy } from "../figure-target-policy-types.js";

export const urduNastaliqFigureTargetPolicy: FigureTargetPolicy = {
  script: "urdu-nastaliq",
  tracks: ["urdu"],
};
