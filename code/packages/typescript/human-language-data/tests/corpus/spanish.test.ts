import { it } from "vitest";
import { expectLanguageContinuity, expectLanguageModality } from "./assert-language-corpus.js";
it("pins Spanish continuity", () => expectLanguageContinuity("spanish"));
it("pins Spanish modality", () => expectLanguageModality("spanish"));
