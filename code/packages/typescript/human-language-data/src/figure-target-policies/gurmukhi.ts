// Gurmukhi filmstrip policy: bare-letter words share one headline.
import type { FigureTargetPolicy } from "../figure-target-policy-types.js";

export const gurmukhiFigureTargetPolicy: FigureTargetPolicy = {
  script: "gurmukhi",
  tracks: ["punjabi"],
  headlineWord: { letter: /^\p{Script=Gurmukhi}$/u },
};
