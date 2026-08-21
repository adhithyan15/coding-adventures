import { it } from "vitest";
import { expectLanguageContinuity, expectLanguageModality } from "./assert-language-corpus.js";
it("pins Urdu continuity", () => expectLanguageContinuity("urdu"));
it("pins Urdu modality", () => expectLanguageModality("urdu"));
