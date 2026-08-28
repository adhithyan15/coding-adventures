import { expect, it } from "vitest";
import { DUCTUS } from "../../../src/strokes";
import { ductusFor } from "../../../src/ductusview";

const LETTER = DUCTUS["ள"];

it("owns U-BB3 view evidence for ள", () => {
  expect(ductusFor("ள")).toBe(LETTER);
});
