import { describe, expect, it } from "vitest";
import { generatedFigureUrl } from "../src/figures.ts";

describe("generated lesson figures", () => {
  it("bundles the canonical SVG shared with the Spanish book", () => {
    const figureUrl = generatedFigureUrl(
      "spanish",
      "figures/ES-C06-cafe-etymology.svg",
    );

    expect(figureUrl).toMatch(/^(?:data:image\/svg\+xml|\/.*\.svg)/);
  });

  it("rejects traversal, remote URLs, and unknown assets", () => {
    expect(() => generatedFigureUrl("spanish", "../escape.svg")).toThrow(/unsafe/);
    expect(() => generatedFigureUrl("spanish", "https://example.test/a.svg")).toThrow(
      /unsafe/,
    );
    expect(() => generatedFigureUrl("spanish", "figures/missing.svg")).toThrow(/missing/);
  });
});
