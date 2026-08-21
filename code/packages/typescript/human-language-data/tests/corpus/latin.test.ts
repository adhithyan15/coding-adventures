import { it } from "vitest";
import { expectLanguageContinuity, expectLanguageModality } from "./assert-language-corpus.js";
it("pins Latin continuity", () => expectLanguageContinuity("latin"));
it("pins Latin modality", () => expectLanguageModality("latin"));
