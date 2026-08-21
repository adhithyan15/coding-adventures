import { it } from "vitest";
import { expectLanguageContinuity, expectLanguageModality } from "./assert-language-corpus.js";
it("pins Marathi continuity", () => expectLanguageContinuity("marathi"));
it("pins Marathi modality", () => expectLanguageModality("marathi"));
