import { it } from "vitest";
import { expectLanguageContinuity, expectLanguageModality } from "./assert-language-corpus.js";
it("pins Marwadi continuity", () => expectLanguageContinuity("marwadi"));
it("pins Marwadi modality", () => expectLanguageModality("marwadi"));
