import { expect, it } from "vitest";
import { DUCTUS } from "../../../src/strokes";
import { ductusFor } from "../../../src/ductusview";

const LETTER = DUCTUS["ய"];

it("owns U-BAF view evidence for ய", () => {
  expect(ductusFor("ய")).toBe(LETTER);
});
