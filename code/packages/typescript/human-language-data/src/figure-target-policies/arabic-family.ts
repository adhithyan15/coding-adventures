// Arabic-family scripts are activated for isolated letters and lists only.
import type { FigureTargetPolicy } from "../figure-target-policy-types.js";

export const arabicFigureTargetPolicy: FigureTargetPolicy = {
  script: "arabic",
  tracks: ["arabic"],
};

export const persoArabicFigureTargetPolicy: FigureTargetPolicy = {
  script: "perso-arabic",
  tracks: ["persian"],
};

export const urduNastaliqFigureTargetPolicy: FigureTargetPolicy = {
  script: "urdu-nastaliq",
  tracks: ["urdu"],
};
