import { describe, it, expect } from "vitest";
import { slugify, formatRoute } from "../src/slug.js";

describe("slugify", () => {
  it("returns the basename of a POSIX path", () => {
    expect(slugify("posts/hello.md")).toBe("hello");
  });

  it("returns the basename of a Windows path", () => {
    expect(slugify("posts\\hello.md")).toBe("hello");
  });

  it("strips .md extension", () => {
    expect(slugify("a.md")).toBe("a");
  });

  it("strips .mdx extension", () => {
    expect(slugify("a.mdx")).toBe("a");
  });

  it("strips .markdown extension", () => {
    expect(slugify("a.markdown")).toBe("a");
  });

  it("strips case-insensitive markdown extensions", () => {
    expect(slugify("Hello.MD")).toBe("hello");
  });

  it("preserves non-markdown extensions", () => {
    expect(slugify("a.txt")).toBe("atxt");
  });

  it("lowercases", () => {
    expect(slugify("Hello.md")).toBe("hello");
  });

  it("replaces whitespace with hyphens", () => {
    expect(slugify("hello world.md")).toBe("hello-world");
  });

  it("replaces multiple whitespace with single hyphen", () => {
    expect(slugify("hello    world.md")).toBe("hello-world");
  });

  it("replaces underscores with hyphens", () => {
    expect(slugify("hello_world.md")).toBe("hello-world");
  });

  it("drops non-alphanumeric characters", () => {
    expect(slugify("hello!world?.md")).toBe("helloworld");
  });

  it("collapses consecutive hyphens", () => {
    expect(slugify("hello---world.md")).toBe("hello-world");
  });

  it("trims leading and trailing hyphens", () => {
    expect(slugify("---hello---.md")).toBe("hello");
  });

  it("returns 'untitled' when everything is stripped", () => {
    expect(slugify("___.md")).toBe("untitled");
    expect(slugify("?!?.md")).toBe("untitled");
    expect(slugify("---.md")).toBe("untitled");
  });

  it("handles deeply nested paths", () => {
    expect(slugify("a/b/c/d/post.md")).toBe("post");
    expect(slugify("a\\b\\c\\d\\post.md")).toBe("post");
  });

  it("handles empty path with fallback", () => {
    expect(slugify("")).toBe("untitled");
  });

  it("handles unicode by stripping (v0 ASCII-only)", () => {
    expect(slugify("café.md")).toBe("caf");
  });

  it("matches the rules used by forme-collect-chronological", () => {
    // These cases come from collect-chronological's slug.test.ts.
    // Identical output proves the rules are bit-identical.
    expect(slugify("posts/2026-05-15-hello-world.md")).toBe("2026-05-15-hello-world");
    expect(slugify("Posts/  My Post .md")).toBe("my-post");
  });
});

describe("formatRoute", () => {
  it("substitutes {slug}", () => {
    expect(formatRoute("/blog/{slug}.html", "hello")).toBe("/blog/hello.html");
  });

  it("substitutes every occurrence of {slug}", () => {
    expect(formatRoute("/{slug}/post/{slug}.html", "x")).toBe("/x/post/x.html");
  });

  it("returns template unchanged when no placeholder", () => {
    expect(formatRoute("/static/path", "x")).toBe("/static/path");
  });

  it("passes through unknown placeholders unchanged", () => {
    // v0 supports only {slug}; future placeholders pass through.
    expect(formatRoute("/{year}/{slug}", "hello")).toBe("/{year}/hello");
  });

  it("handles empty slug", () => {
    expect(formatRoute("/blog/{slug}.html", "")).toBe("/blog/.html");
  });
});
