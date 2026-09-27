import { it } from "vitest";
import { expectLanguageContinuity } from "../assert-language-corpus.js";

it("pins German continuity", () => expectLanguageContinuity("german"));
