import { expect, it } from "vitest";
import { DUCTUS } from "../../../src/strokes";
import { ductusFor } from "../../../src/ductusview";

const LETTER = DUCTUS["ர"];

it("owns U-BB0 view evidence for ர", () => {
  expect(ductusFor("ர")).toBe(LETTER);
});
