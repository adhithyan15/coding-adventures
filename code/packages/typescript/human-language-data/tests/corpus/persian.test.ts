import { it } from "vitest";
import { expectLanguageContinuity, expectLanguageModality } from "./assert-language-corpus.js";
it("pins Persian continuity", () => expectLanguageContinuity("persian"));
it("pins Persian modality", () => expectLanguageModality("persian"));
