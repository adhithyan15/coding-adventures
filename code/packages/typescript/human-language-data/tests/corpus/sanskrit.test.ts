import { it } from "vitest";
import { expectLanguageContinuity, expectLanguageModality } from "./assert-language-corpus.js";
it("pins Sanskrit continuity", () => expectLanguageContinuity("sanskrit"));
it("pins Sanskrit modality", () => expectLanguageModality("sanskrit"));
