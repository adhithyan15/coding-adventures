import { expect, it } from "vitest";
import { languageWritingStages } from "../assert-language-corpus.js";

it(
  "pins Arabic's complete pre-A1 writing ramp",
  () => {
    const arabic = languageWritingStages("arabic");
    expect(arabic.defects).toEqual([]);
    expect(arabic.levels[0]).toMatchObject({ level: "pre-A1", complete: true, missingStages: [] });
  },
  // This ratchet parses and renders the complete Arabic corpus, which can exceed Vitest's default under load.
  60_000,
);
