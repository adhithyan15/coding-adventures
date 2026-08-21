import { it } from "vitest";
import { expectLanguageContinuity, expectLanguageModality } from "./assert-language-corpus.js";
it("pins Telugu continuity", () => expectLanguageContinuity("telugu"));
it("pins Telugu modality", () => expectLanguageModality("telugu"));
