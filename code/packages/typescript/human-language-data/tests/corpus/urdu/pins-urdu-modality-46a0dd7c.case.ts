import { it } from "vitest";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
} from "../assert-language-corpus.js";

it("pins Urdu modality", () => expectLanguageModality("urdu"));
