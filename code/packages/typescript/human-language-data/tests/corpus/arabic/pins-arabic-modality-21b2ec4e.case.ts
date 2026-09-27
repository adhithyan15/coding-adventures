import { it } from "vitest";
import { expectLanguageModality } from "../assert-language-corpus.js";

it("pins Arabic modality", () => expectLanguageModality("arabic"));
