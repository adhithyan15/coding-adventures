import type { EtymologyGapPinOwner } from "../etymology-gap-pin-owners.js";

export const etymologyGapPinOwner = {
  language: "italian",
  withoutHook: 0,
  withoutVisible: 0,
  rationale: [
    "The 1,126 / 1,135 baseline first fell to 745 / 745 and then to 0 / 0. The first 401 gap lessons through IT-C110-pepe gained sourced origins, and the remaining 745 from IT-C111-infermiera through IT-C259-storia followed.",
    "The review used EtymDB 2.1 (Fourrier and Sagot 2020), CC BY-SA 4.0, Lewis and Short for textbook Latin forms EtymDB lacks, and sister-word tracing where EtymDB has no row. Well-established origins may be stated without a source tag; genuinely uncertain lessons say so and why.",
  ],
} as const satisfies EtymologyGapPinOwner;
