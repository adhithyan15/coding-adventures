import { it } from "vitest";
import { expectLanguageContinuity, expectLanguageModality } from "./assert-language-corpus.js";
it("pins Hindi continuity", () => expectLanguageContinuity("hindi"));
it("pins Hindi modality", () => expectLanguageModality("hindi"));
