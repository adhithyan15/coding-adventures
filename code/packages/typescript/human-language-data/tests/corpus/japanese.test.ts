import { it } from "vitest";
import { expectLanguageContinuity, expectLanguageModality } from "./assert-language-corpus.js";
it("pins Japanese continuity", () => expectLanguageContinuity("japanese"));
it("pins Japanese modality", () => expectLanguageModality("japanese"));
