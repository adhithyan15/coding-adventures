import { expect, it } from "vitest";
import { DUCTUS } from "../../../src/strokes";
import { ductusFor } from "../../../src/ductusview";

const LETTER = DUCTUS["ப"];

it("owns U-BAA view evidence for ப", () => {
  expect(ductusFor("ப")).toBe(LETTER);
});
