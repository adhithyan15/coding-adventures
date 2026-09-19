import { describe, it, expect } from "vitest";
import { splitFrontmatter } from "../src/frontmatter.js";

describe("splitFrontmatter", () => {
  it("parses scalars, lists, and the body", () => {
    const src = [
      "---",
      "id: ES-C01-dia",
      "chapter: 1",
      "type: word",
      "headword: día",
      "prerequisites: [ES-C01-el-la, ES-C01-hola]",
      "reviews_of: []",
      "---",
      "# Body starts here",
      "text",
    ].join("\n");
    const { frontmatter, body } = splitFrontmatter(src);
    expect(frontmatter).toMatchObject({
      id: "ES-C01-dia",
      chapter: "1",
      type: "word",
      headword: "día",
      prerequisites: ["ES-C01-el-la", "ES-C01-hola"],
      reviews_of: [],
    });
    expect(body.trim().startsWith("# Body")).toBe(true);
  });

  it("strips single and double quotes from values", () => {
    const { frontmatter } = splitFrontmatter(
      ['---', 'etymology_hook: "día ← dies"', "gloss: 'day'", "---", ""].join("\n"),
    );
    expect(frontmatter?.etymology_hook).toBe("día ← dies");
    expect(frontmatter?.gloss).toBe("day");
  });

  it.each(["headword", "romanization", "gloss", "concept_tag"])(
    "rejects escapes in a double-quoted %s instead of silently preserving them",
    (field) => {
      const escaped = ["---", `${field}: "\\\" \\\""`, "---", ""].join("\n");
      expect(() => splitFrontmatter(escaped)).toThrow(
        new RegExp(
          `${field} cannot use a double-quoted frontmatter scalar containing backslashes; use single quotes`,
        ),
      );
    },
  );

  it("allows literal backslashes in single-quoted scalars", () => {
    const { frontmatter } = splitFrontmatter(
      ["---", "headword: '\\\" \\\"'", "---", ""].join("\n"),
    );
    expect(frontmatter?.headword).toBe('\\\" \\\"');
  });

  it("preserves backslashes in prose-only fields in the repository's tiny YAML dialect", () => {
    const { frontmatter } = splitFrontmatter(
      ['---', 'etymology_hook: "Proto-Indo-European \\*dwo-"', "---", ""].join("\n"),
    );
    expect(frontmatter?.etymology_hook).toBe("Proto-Indo-European \\*dwo-");
  });

  it("ignores blank lines and comment lines inside the block", () => {
    const { frontmatter } = splitFrontmatter(
      ["---", "# a comment", "", "id: X", "---", ""].join("\n"),
    );
    expect(frontmatter).toEqual({ id: "X" });
  });

  it("flattens one schema-v2 nested map level to dotted keys", () => {
    const { frontmatter } = splitFrontmatter(
      [
        "---",
        "duration:",
        "  max_seconds: 240",
        "requires:",
        "  knowledge: [ES-LEX-HOLA, ES-SOUND-H-SILENT]",
        "register: neutral",
        "---",
        "",
      ].join("\n"),
    );
    expect(frontmatter).toEqual({
      "duration.max_seconds": "240",
      "requires.knowledge": ["ES-LEX-HOLA", "ES-SOUND-H-SILENT"],
      register: "neutral",
    });
  });

  it("returns null frontmatter when there is no fence", () => {
    const { frontmatter, body } = splitFrontmatter("no frontmatter here");
    expect(frontmatter).toBeNull();
    expect(body).toBe("no frontmatter here");
  });

  it("tolerates a leading BOM and CRLF line endings", () => {
    const { frontmatter } = splitFrontmatter("﻿---\r\nid: X\r\n---\r\nbody");
    expect(frontmatter).toEqual({ id: "X" });
  });
});
