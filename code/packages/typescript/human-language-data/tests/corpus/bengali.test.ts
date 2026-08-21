import { it } from "vitest";
import { expectLanguageContinuity, expectLanguageModality } from "./assert-language-corpus.js";
it("pins Bengali continuity", () => expectLanguageContinuity("bengali"));
it("pins Bengali modality", () => expectLanguageModality("bengali"));
