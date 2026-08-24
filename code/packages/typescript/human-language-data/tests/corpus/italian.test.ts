import { it } from "vitest";
import { expectLanguageContinuity, expectLanguageModality } from "./assert-language-corpus.js";
it("pins Italian continuity", () => expectLanguageContinuity("italian"));
it("pins Italian modality", () => expectLanguageModality("italian"));
