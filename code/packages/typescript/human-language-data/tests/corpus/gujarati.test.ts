import { it } from "vitest";
import { expectLanguageContinuity, expectLanguageModality } from "./assert-language-corpus.js";
it("pins Gujarati continuity", () => expectLanguageContinuity("gujarati"));
it("pins Gujarati modality", () => expectLanguageModality("gujarati"));
