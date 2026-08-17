// HL-C90 -- corpus-derived strings must not be able to edit the report that
// judges them.
//
// Found by the security review of HL-C80 and deliberately filed whole rather
// than fixed halfway inside an unrelated PR, because the pattern is package-wide:
// every gate interpolates author-written ids into lines written to stdout.
//
// Control characters are constructed with String.fromCharCode rather than
// written as literals. Two reasons, both learned the hard way in this session: a
// literal ESC in a source file is invisible to a reviewer, and the tooling that
// wrote these files has silently mangled non-ASCII literals before.

import { describe, expect, it } from "vitest";
import { stripControlCharacters } from "../src/constants.js";
import { renderStrandSummary, summarizeStrands } from "../src/strands.js";
import { renderRootLedger, buildRootLedger } from "../src/root-ledger.js";
import { renderInfoDump, measureInfoDump } from "../src/info-dump.js";
import { renderMetalanguage, measureMetalanguage } from "../src/metalanguage.js";
import { loadMetalanguage } from "../src/loader.js";
import { parseLesson } from "../src/parse.js";
import { CURRICULUM_STRANDS, type CurriculumSpine, type SpineNode } from "../src/types.js";

const ESC = String.fromCharCode(27);
const CR = String.fromCharCode(13);
const NUL = String.fromCharCode(0);
/** An id that erases its own report line in a terminal. */
const LINE_EATER = `EVIL${ESC}[2K${CR}innocent`;

describe("stripControlCharacters", () => {
  it("removes an ANSI escape sequence's control bytes", () => {
    expect(stripControlCharacters(LINE_EATER)).toBe("EVIL[2Kinnocent");
  });

  it("removes NUL", () => {
    expect(stripControlCharacters(`a${NUL}b`)).toBe("ab");
  });

  it("removes the C1 range, which some terminals also act on", () => {
    expect(stripControlCharacters(`a${String.fromCharCode(0x9b)}b`)).toBe("ab");
  });

  it("keeps tab and newline, which are ordinary layout", () => {
    expect(stripControlCharacters("a\tb\nc")).toBe("a\tb\nc");
  });

  it("keeps ordinary text and accented Spanish untouched", () => {
    expect(stripControlCharacters("ES-C01-adiós · grātia")).toBe("ES-C01-adiós · grātia");
  });

  it("is a no-op on a clean string, so it can be applied everywhere", () => {
    const clean = "ES-C05-adios: 3 payoffs";
    expect(stripControlCharacters(clean)).toBe(clean);
  });
});

describe("the render helpers cannot be edited by their own subject", () => {
  function containsControl(text: string): boolean {
    for (const ch of text) {
      const code = ch.codePointAt(0)!;
      if (ch === "\t" || ch === "\n") continue;
      if (code <= 0x1f || (code >= 0x7f && code <= 0x9f)) return true;
    }
    return false;
  }

  it("strand summary", () => {
    const spine: CurriculumSpine = {
      version: 1,
      stages: ["A1"],
      strands: [...CURRICULUM_STRANDS],
      nodes: [
        {
          id: LINE_EATER,
          stage: "A1",
          strand: "FUNCTION",
          canDo: "x",
          prerequisites: [],
          core: true,
          concepts: Array.from({ length: 42 }, (_, i) => `c${i}`),
        } as SpineNode,
      ],
    };
    const text = renderStrandSummary(summarizeStrands(spine, 12)).join("\n");
    expect(text).toContain("EVIL");
    expect(containsControl(text)).toBe(false);
  });

  it("root ledger", () => {
    // `parseLesson` refuses a hostile id since HL-C211, so it can no longer be
    // laundered in through the frontmatter. The RENDER HELPER is this file's
    // subject and must still hold on its own, so the id is set on the parsed
    // object directly -- defence in depth, tested at the depth it defends.
    const lesson = parseLesson(
      `---\nschema_version: 2\nid: ES-C01-clean\nsequence: 10\nchapter: 1\ntype: vocabulary\nheadword: x\ngloss: x\nroots: [${LINE_EATER}]\n---\n\n# x\n`,
      "spanish",
    );
    lesson.realization.lessonId = LINE_EATER;
    const text = renderRootLedger(buildRootLedger([lesson], 3)).join("\n");
    expect(text).toContain("EVIL");
    expect(containsControl(text)).toBe(false);
  });

  it("metalanguage", () => {
    // The site deliberately deferred when this fix was written: metalanguage.ts
    // landed in a different PR, so it was picked up on rebase rather than left
    // as the one unguarded render helper.
    const inventory = {
      version: 1,
      terms: [
        {
          id: "META-EVIL",
          term: LINE_EATER,
          stage: "A1",
          order: 1,
          introduceAfter: "x",
          plainAlternative: "y",
          technical: true,
        },
      ],
    };
    const lesson = parseLesson(
      `---\nschema_version: 2\nid: L1\nsequence: 10\nchapter: 1\ntype: vocabulary\nheadword: x\ngloss: x\n---\n\n# x\n\nThe ${LINE_EATER} matters.\n`,
      "spanish",
    );
    const text = renderMetalanguage(measureMetalanguage([lesson], inventory as never)).join("\n");
    expect(containsControl(text)).toBe(false);
  });

  it("every committed metalanguage term is already control-free", () => {
    for (const term of loadMetalanguage().terms) {
      expect(containsControl(term.term)).toBe(false);
      expect(containsControl(term.plainAlternative)).toBe(false);
    }
  });

  it("info dump", () => {
    const grid = [
      "| person | form |",
      "|---|---|",
      "| yo | a |",
      "| tú | b |",
      "| él | c |",
      "| nosotros | d |",
      "| vosotros | e |",
    ].join("\n");
    // Same as the root ledger above: the id is set after parsing, because the
    // parser now refuses it and the render helper is still what is under test.
    // `info-dump.ts` reads the RAW frontmatter id rather than the validated
    // `realization.lessonId`, so that is the field to poison here -- and the
    // difference is why this helper still needs its own guard.
    const lesson = parseLesson(
      `---\nschema_version: 2\nid: ES-C01-clean\nsequence: 10\nchapter: 1\ntype: vocabulary\nheadword: x\ngloss: x\n---\n\n# x\n\n${grid}\n`,
      "spanish",
    );
    (lesson.frontmatter as Record<string, unknown>).id = LINE_EATER;
    const text = renderInfoDump(measureInfoDump([lesson], 1)).join("\n");
    expect(text).toContain("EVIL");
    expect(containsControl(text)).toBe(false);
  });
});
