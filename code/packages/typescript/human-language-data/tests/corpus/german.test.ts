import { it } from "vitest";
import { expectLanguageContinuity, expectLanguageModality } from "./assert-language-corpus.js";
it("pins German continuity", () => expectLanguageContinuity("german"));
it("pins German modality", () => expectLanguageModality("german"));
