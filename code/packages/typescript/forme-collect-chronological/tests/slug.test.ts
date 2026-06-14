/**
 * slug.test.ts — slugify + formatRoute unit tests.
 */

import { describe, it, expect } from "vitest";
import { slugify, formatRoute } from "../src/slug.js";

describe("slugify", () => {
  it("takes the basename and strips .md", () => {
    expect(slugify("posts/hello.md")).toBe("hello");
  });

  it("strips .mdx (case-insensitive)", () => {
    expect(slugify("a/b/My-Post.MDX")).toBe("my-post");
  });

  it("strips .markdown", () => {
    expect(slugify("posts/foo.markdown")).toBe("foo");
  });

  it("handles Windows path separators", () => {
    // Single backslash literal in the path; the regex /[/\\]/ splits
    // on either separator so this still returns "hello".
    expect(slugify("posts\\sub\\Hello.md")).toBe("hello");
  });

  it("does NOT strip unknown extensions", () => {
    expect(slugify("posts/notes.txt")).toBe("notestxt");
  });

  it("lowercases", () => {
    expect(slugify("HELLO.md")).toBe("hello");
  });

  it("replaces whitespace with single dash", () => {
    expect(slugify("posts/hello world here.md")).toBe("hello-world-here");
  });

  it("replaces underscores with dashes", () => {
    expect(slugify("posts/hello_world.md")).toBe("hello-world");
  });

  it("drops punctuation", () => {
    expect(slugify("posts/hello!@#$.md")).toBe("hello");
  });

  it("collapses multiple dashes", () => {
    expect(slugify("posts/hello---world.md")).toBe("hello-world");
  });

  it("trims leading/trailing dashes", () => {
    expect(slugify("posts/-hello-.md")).toBe("hello");
  });

  it("falls back to 'untitled' when result would be empty", () => {
    expect(slugify("posts/@@@.md")).toBe("untitled");
  });

  it("handles files without an extension", () => {
    expect(slugify("a/b/c")).toBe("c");
  });

  it("preserves embedded digits", () => {
    expect(slugify("2026-05-15-release-notes.md")).toBe("2026-05-15-release-notes");
  });
});

describe("formatRoute", () => {
  it("substitutes {slug}", () => {
    expect(formatRoute("/blog/{slug}.html", "hello")).toBe("/blog/hello.html");
  });

  it("substitutes multiple {slug} occurrences", () => {
    expect(formatRoute("/{slug}/{slug}", "x")).toBe("/x/x");
  });

  it("leaves templates without {slug} unchanged", () => {
    expect(formatRoute("/static.html", "ignored")).toBe("/static.html");
  });
});
