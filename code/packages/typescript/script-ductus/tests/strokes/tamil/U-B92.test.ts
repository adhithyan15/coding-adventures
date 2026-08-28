import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-B92.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ஒ"];
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-B92 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ஒ");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "523e7ea44b1739a937bb8a47188dc531e2422aea02f345712976cdbc40e12f82",
    );
  });

  it("ஒ's two-run order traces to Module 14 and Appendix I Frame 14", () => {
    const source = letter.source;
    expect(source.url).toContain("module-14");
    expect(source.citation).toMatch(
      /Module 14.*ஒ.*Appendix I.*Frame 14.*p\. 195/i,
    );
    expect(source.variation).toMatch(
      /short o.*three movements.*left loop.*large right loop.*joined.*separate lower bowl.*one lift.*two-run.*Noto Sans Tamil.*varies by school/i,
    );
    expect(penLifts(letter)).toBe(1);
  });
});
