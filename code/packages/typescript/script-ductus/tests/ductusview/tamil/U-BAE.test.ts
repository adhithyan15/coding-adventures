import { expect, it } from "vitest";
import { DUCTUS } from "../../../src/strokes";
import { ductusFor } from "../../../src/ductusview";

const LETTER = DUCTUS["ம"];

it("owns U-BAE view evidence for ம", () => {
  expect(ductusFor("ம")).toBe(LETTER);
});
