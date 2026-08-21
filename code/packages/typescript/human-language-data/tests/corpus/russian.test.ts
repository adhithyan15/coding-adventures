import { it } from "vitest";
import { expectLanguageContinuity, expectLanguageModality } from "./assert-language-corpus.js";
it("pins Russian continuity", () => expectLanguageContinuity("russian"));
it("pins Russian modality", () => expectLanguageModality("russian"));
