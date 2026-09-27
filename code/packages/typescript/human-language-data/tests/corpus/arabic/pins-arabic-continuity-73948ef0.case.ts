import { it } from "vitest";
import { expectLanguageContinuity } from "../assert-language-corpus.js";

it("pins Arabic continuity", () => expectLanguageContinuity("arabic"));
