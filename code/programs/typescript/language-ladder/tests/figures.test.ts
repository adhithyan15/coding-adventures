import { describe, expect, it } from "vitest";
import { filmstripCaption, generatedFigureUrl, generatedFilmstripUrl } from "../src/figures.ts";

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

  it("loads a cited writing filmstrip through the lazy source map", async () => {
    await expect(generatedFilmstripUrl("tamil", "TA-S01-va")).resolves.toMatch(
      /^(?:data:image\/svg\+xml|\/.*\.svg)/,
    );
    await expect(generatedFilmstripUrl("tamil", "TA-W-without-ductus")).resolves.toBeNull();
  });

  it("rejects unsafe filmstrip lookup keys before loading the source map", async () => {
    await expect(generatedFilmstripUrl("../tamil", "TA-S01-va")).rejects.toThrow(/unsafe/);
    await expect(generatedFilmstripUrl("tamil", "../TA-S01-va")).rejects.toThrow(/unsafe/);
  });

  it("captions a filmstrip as a letter, a word, or a list of letters", () => {
    expect(filmstripCaption("ক")).toBe("How ক is written, stroke by stroke");
    expect(filmstripCaption("はい")).toBe("How はい is written, stroke by stroke");
    expect(filmstripCaption("ক — ণ — শ")).toBe(
      "How these letters are written, stroke by stroke: ক, ণ, শ",
    );
    expect(filmstripCaption("ن، ت، ث")).toBe(
      "How these letters are written, stroke by stroke: ن, ت, ث",
    );
  });
});
