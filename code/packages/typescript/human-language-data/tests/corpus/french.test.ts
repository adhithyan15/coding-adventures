import { it } from "vitest";
import { expectLanguageContinuity, expectLanguageModality } from "./assert-language-corpus.js";
it("pins French continuity", () => expectLanguageContinuity("french"));
it("pins French modality", () => expectLanguageModality("french"));
