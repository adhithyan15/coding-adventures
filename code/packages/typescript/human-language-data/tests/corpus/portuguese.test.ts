import { it } from "vitest";
import { expectLanguageContinuity, expectLanguageModality } from "./assert-language-corpus.js";
it("pins Portuguese continuity", () => expectLanguageContinuity("portuguese"));
it("pins Portuguese modality", () => expectLanguageModality("portuguese"));
