import { it } from "vitest";
import { expectLanguageContinuity, expectLanguageModality } from "./assert-language-corpus.js";
it("pins Arabic continuity", () => expectLanguageContinuity("arabic"));
it("pins Arabic modality", () => expectLanguageModality("arabic"));
