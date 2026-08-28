import { describe, expect, it } from "vitest";
import { validateStyleDocument } from "@coding-adventures/forme-style-ir";
import { classlessTheme } from "../src/index.js";

describe("classlessTheme", () => {
  it("is valid, resolved Style IR with unique rule ids", () => {
    expect(validateStyleDocument(classlessTheme).warnings).toEqual([]);
    expect(classlessTheme.theme).toBeNull();
    expect(new Set(classlessTheme.rules.map((rule) => rule.id)).size)
      .toBe(classlessTheme.rules.length);
  });

  it("includes light tokens and preference-driven dark rules", () => {
    expect(classlessTheme.tokens.colors["surface"]).toEqual({ kind: "named", name: "#ffffff" });
    expect(classlessTheme.contexts).toContain("dark");
    expect(classlessTheme.rules.some((rule) => rule.context === "dark")).toBe(true);
  });
});
