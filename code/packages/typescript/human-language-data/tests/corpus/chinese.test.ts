import { it } from "vitest";
import { expectLanguageContinuity, expectLanguageModality } from "./assert-language-corpus.js";
it("pins Chinese continuity", () => expectLanguageContinuity("chinese"));
it("pins Chinese modality", () => expectLanguageModality("chinese"));
