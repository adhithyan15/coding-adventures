import { it } from "vitest";
import { expectLanguageContinuity, expectLanguageModality } from "./assert-language-corpus.js";
it("pins Tamil continuity", () => expectLanguageContinuity("tamil"));
it("pins Tamil modality", () => expectLanguageModality("tamil"));
