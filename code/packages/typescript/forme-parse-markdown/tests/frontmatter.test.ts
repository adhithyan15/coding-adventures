/**
 * frontmatter.ts — edge-case coverage.
 *
 * The grammar is intentionally tiny so the test surface is too — but
 * the *failure modes* are where bugs hide, so we lean hardest on
 * malformed input (no closing fence, missing colon, empty key, etc.)
 * to lock the "silently fall back to no-frontmatter" contract in.
 */

import { describe, it, expect } from "vitest";
import { splitFrontmatter } from "../src/frontmatter.js";

describe("splitFrontmatter — happy paths", () => {
  it("parses a single-key block", () => {
    const r = splitFrontmatter("---\ntitle: hi\n---\nhello");
    expect(r.data).toEqual({ title: "hi" });
    expect(r.body).toBe("hello");
  });

  it("parses multiple keys", () => {
    const r = splitFrontmatter(
      "---\ntitle: hi\ndate: 2026-05-15\nauthor: adhithya\n---\n# Hello",
    );
    expect(r.data).toEqual({
      title: "hi",
      date: "2026-05-15",
      author: "adhithya",
    });
    expect(r.body).toBe("# Hello");
  });

  it("trims whitespace around keys and values", () => {
    const r = splitFrontmatter("---\n  title  :   hello world   \n---\nx");
    expect(r.data).toEqual({ title: "hello world" });
  });

  it("preserves colons inside values", () => {
    const r = splitFrontmatter("---\nurl: https://example.com/x\n---\nbody");
    expect(r.data).toEqual({ url: "https://example.com/x" });
  });

  it("allows blank lines inside the block", () => {
    const r = splitFrontmatter("---\ntitle: hi\n\nauthor: me\n---\nbody");
    expect(r.data).toEqual({ title: "hi", author: "me" });
  });

  it("handles CRLF line endings on the fence", () => {
    const r = splitFrontmatter("---\r\ntitle: hi\r\n---\r\nbody");
    expect(r.data).toEqual({ title: "hi" });
    // body is normalised to LF
    expect(r.body).toBe("body");
  });

  it("body retains its original line breaks (LF after normalisation)", () => {
    const r = splitFrontmatter("---\nk: v\n---\nline1\nline2\nline3");
    expect(r.body).toBe("line1\nline2\nline3");
  });

  it("empty body after fence is empty string", () => {
    const r = splitFrontmatter("---\nk: v\n---\n");
    expect(r.body).toBe("");
  });
});

describe("splitFrontmatter — malformed input falls back gracefully", () => {
  it("no frontmatter at all → body is source verbatim", () => {
    const src = "# Just a heading\n\nSome text.";
    const r = splitFrontmatter(src);
    expect(r.data).toEqual({});
    expect(r.body).toBe(src);
  });

  it("opening fence not at byte 0 → no frontmatter", () => {
    const src = "\n---\ntitle: hi\n---\nbody";
    const r = splitFrontmatter(src);
    expect(r.data).toEqual({});
    expect(r.body).toBe(src);
  });

  it("opening fence with trailing text → no frontmatter", () => {
    const src = "---something\nfoo\n---\nbody";
    const r = splitFrontmatter(src);
    expect(r.data).toEqual({});
    expect(r.body).toBe(src);
  });

  it("no closing fence → no frontmatter, body is source verbatim", () => {
    const src = "---\ntitle: hi\nbody continues here";
    const r = splitFrontmatter(src);
    expect(r.data).toEqual({});
    expect(r.body).toBe(src);
  });

  it("interior line missing colon → whole block invalidated", () => {
    const src = "---\ntitle: hi\nbroken line\n---\nbody";
    const r = splitFrontmatter(src);
    expect(r.data).toEqual({});
    expect(r.body).toBe(src);
  });

  it("interior line with empty key → whole block invalidated", () => {
    const src = "---\n: value\n---\nbody";
    const r = splitFrontmatter(src);
    expect(r.data).toEqual({});
    expect(r.body).toBe(src);
  });

  it("source is just the fence string with no newline → no frontmatter", () => {
    const r = splitFrontmatter("---");
    expect(r.data).toEqual({});
    expect(r.body).toBe("---");
  });

  it("empty source → no frontmatter, empty body", () => {
    const r = splitFrontmatter("");
    expect(r.data).toEqual({});
    expect(r.body).toBe("");
  });
});
