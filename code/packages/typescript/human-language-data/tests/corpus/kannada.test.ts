import { it } from "vitest";
import { expectLanguageContinuity, expectLanguageModality } from "./assert-language-corpus.js";
it("pins Kannada continuity", () => expectLanguageContinuity("kannada"));
it("pins Kannada modality", () => expectLanguageModality("kannada"));
