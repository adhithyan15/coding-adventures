import { expect, it } from "vitest";
import { expectLanguageModality } from "../assert-language-corpus.js";

it("pins Sanskrit modality", () => expectLanguageModality("sanskrit"));
