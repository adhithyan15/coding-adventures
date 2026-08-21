import { it } from "vitest";
import { expectLanguageContinuity, expectLanguageModality } from "./assert-language-corpus.js";
it("pins Punjabi continuity", () => expectLanguageContinuity("punjabi"));
it("pins Punjabi modality", () => expectLanguageModality("punjabi"));
