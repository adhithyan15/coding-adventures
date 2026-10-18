import { expect, it } from "vitest";
import { expectLanguageContinuity } from "../assert-language-corpus.js";

it("pins Sanskrit continuity", () => expectLanguageContinuity("sanskrit"));
