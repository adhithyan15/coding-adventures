import { it } from "vitest";
import { expectLanguageModality } from "../assert-language-corpus.js";

it("pins German modality", () => expectLanguageModality("german"));
