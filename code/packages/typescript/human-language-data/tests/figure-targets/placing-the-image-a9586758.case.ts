import { describe, expect, it } from "vitest";
import type { ScriptFilmstripTarget } from "../../src/figure.js";
import {
  filmstripCandidates,
  filmstripImageMarkdown,
  withFilmstripImages,
  writingLetterOf,
} from "../../src/figure-targets.js";
import { lesson } from "./fixture.js";

describe("placing the image", () => {
  const target = filmstripCandidates([lesson("TA-S1")])[0]!;

  it("puts the filmstrip at the top of the Writing block and nowhere else", () => {
    const [placed] = withFilmstripImages([lesson("TA-S1")], [target]);
    const writing = placed!.blocks.find((block) => block.title.startsWith("Writing"))!;
    expect(writing.markdown.startsWith(filmstripImageMarkdown(target))).toBe(true);
    expect(writing.markdown).toContain("body of Writing: x");
    expect(placed!.blocks.filter((block) => block.markdown.includes("filmstrip")).length).toBe(1);
  });

  it("falls back to the first Script block when a track presents its letter there", () => {
    const scriptOnly = lesson("ZH-S1", { language: "tamil", blocks: ["Warm-up", "Script — the letter", "Wrap-up Recall"] });
    expect(writingLetterOf(scriptOnly)).toBe("அ");
    const [placed] = withFilmstripImages([scriptOnly], filmstripCandidates([scriptOnly]));
    expect(placed!.blocks[1]!.markdown.startsWith("![How அ is written")).toBe(true);
  });

  it("prefers a Writing block over an earlier Script block", () => {
    const both = lesson("TA-S9", { blocks: ["Script — shape", "Writing: strokes", "Wrap-up Recall"] });
    const [placed] = withFilmstripImages([both], filmstripCandidates([both]));
    expect(placed!.blocks[0]!.markdown).not.toContain("filmstrip");
    expect(placed!.blocks[1]!.markdown).toContain("filmstrip");
  });

  it("places a declared target on a word lesson's first script-introducing block", () => {
    // FA-C03-chist teaches چ inside a word lesson: no Writing or Script block,
    // so it is never a derived candidate, but a DECLARED filmstrip still prints.
    const word = lesson("FA-C3", { type: "word", blocks: ["Warm-up", "You'll want to know", "Wrap-up Recall"] });
    word.blocks[1]!.knowledge = { introduces: ["FA-LEX-X", "FA-SCRIPT-CHE"], assesses: [] } as never;
    expect(writingLetterOf(word)).toBeUndefined();
    const declared: ScriptFilmstripTarget = {
      kind: "script-filmstrip",
      lessonId: "FA-C3",
      script: "perso-arabic",
      glyph: "چ",
      output: "persian/book/figures/FA-C3-filmstrip.svg",
    };
    const [placed] = withFilmstripImages([word], [declared]);
    expect(placed!.blocks[1]!.markdown.startsWith("![How چ is written")).toBe(true);
  });

  it("leaves a lesson that already prints its figure by hand alone", () => {
    const authored = lesson("TA-S1");
    authored.blocks[0]!.markdown = "![hand placed](figures/TA-S1-filmstrip.svg)\n";
    const [placed] = withFilmstripImages([authored], [target]);
    expect(placed).toBe(authored);
  });

  it("does not modify the lessons it was given", () => {
    const authored = lesson("TA-S1");
    withFilmstripImages([authored], [target]);
    expect(authored.blocks[1]!.markdown).toBe("body of Writing: x\n");
  });
});
