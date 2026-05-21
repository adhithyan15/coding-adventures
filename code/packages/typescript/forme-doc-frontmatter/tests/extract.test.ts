/**
 * extract.test.ts — end-to-end extractFrontmatter.
 */

import { describe, it, expect } from "vitest";
import { extractFrontmatter } from "../src/index.js";

describe("extractFrontmatter — no frontmatter", () => {
  it("plain markdown passes through", () => {
    const md = "# Hello\n\nWorld";
    expect(extractFrontmatter(md)).toEqual({
      body: md, frontmatter: null, format: "none",
    });
  });
  it("empty string", () => {
    expect(extractFrontmatter("")).toEqual({
      body: "", frontmatter: null, format: "none",
    });
  });
  it("just delimiters at start, not on their own line, NOT detected", () => {
    const md = "--- not frontmatter\n# Body";
    expect(extractFrontmatter(md).format).toBe("none");
  });
});

describe("extractFrontmatter — YAML", () => {
  it("basic YAML frontmatter", () => {
    const md = `---\ntitle: Hello\n---\n# Body`;
    const r = extractFrontmatter(md);
    expect(r.format).toBe("yaml");
    expect(r.frontmatter).toEqual({ title: "Hello" });
    expect(r.body).toBe("# Body");
  });
  it("YAML with CRLF line endings", () => {
    const md = `---\r\ntitle: Hello\r\n---\r\n# Body`;
    const r = extractFrontmatter(md);
    expect(r.format).toBe("yaml");
    expect(r.frontmatter).toEqual({ title: "Hello" });
    expect(r.body).toBe("# Body");
  });
  it("YAML with multiple keys + array", () => {
    const md = `---\ntitle: Hello\ndate: 2026-05-20\ntags: [a, b]\n---\n\n# Body\n\nMore.`;
    const r = extractFrontmatter(md);
    expect(r.frontmatter).toEqual({
      title: "Hello", date: "2026-05-20", tags: ["a", "b"],
    });
    expect(r.body).toBe("\n# Body\n\nMore.");
  });
  it("empty YAML body — no closing delim — throws", () => {
    expect(() => extractFrontmatter(`---\ntitle: Hi\n# no close`))
      .toThrow(/no matching closing/);
  });
  it("BOM at start is stripped", () => {
    const md = `﻿---\ntitle: Hello\n---\n# Body`;
    const r = extractFrontmatter(md);
    expect(r.format).toBe("yaml");
    expect(r.frontmatter).toEqual({ title: "Hello" });
  });
});

describe("extractFrontmatter — TOML", () => {
  it("basic TOML frontmatter", () => {
    const md = `+++\ntitle = "Hello"\n+++\n# Body`;
    const r = extractFrontmatter(md);
    expect(r.format).toBe("toml");
    expect(r.frontmatter).toEqual({ title: "Hello" });
    expect(r.body).toBe("# Body");
  });
  it("TOML with array + date", () => {
    const md = `+++\ndate = 2026-05-20\ntags = ["a", "b"]\n+++\n\n# Body`;
    const r = extractFrontmatter(md);
    expect(r.frontmatter).toEqual({ date: "2026-05-20", tags: ["a", "b"] });
  });
  it("TOML without closing delim throws", () => {
    expect(() => extractFrontmatter(`+++\ntitle = "x"\n# body`))
      .toThrow(/no matching closing/);
  });
});

describe("extractFrontmatter — input validation", () => {
  it("non-string throws", () => {
    expect(() => extractFrontmatter(42 as unknown as string))
      .toThrow(/source must be a string/);
  });
  it("null throws", () => {
    expect(() => extractFrontmatter(null as unknown as string))
      .toThrow(/source must be a string/);
  });
});

describe("extractFrontmatter — purity / determinism", () => {
  it("same input → identical output", () => {
    const md = `---\ntitle: x\n---\nbody`;
    expect(extractFrontmatter(md)).toEqual(extractFrontmatter(md));
  });
  it("no input mutation", () => {
    const md = `---\ntitle: x\n---\nbody`;
    extractFrontmatter(md);
    expect(md).toBe(`---\ntitle: x\n---\nbody`);
  });
});

describe("extractFrontmatter — malformed frontmatter rejected", () => {
  it("YAML with invalid inner content propagates the error", () => {
    expect(() => extractFrontmatter(`---\n__proto__: bad\n---\nbody`))
      .toThrow(/prototype-pollution/);
  });
  it("TOML with table syntax rejected", () => {
    expect(() => extractFrontmatter(`+++\n[section]\na = 1\n+++\nbody`))
      .toThrow(/tables.*not supported/);
  });
});

describe("extractFrontmatter — real-world examples", () => {
  it("typical Hugo blog post", () => {
    const md = `+++
title = "My Post"
date = 2026-05-20
tags = ["typescript", "docs"]
draft = false
+++

## Introduction

Body here.`;
    const r = extractFrontmatter(md);
    expect(r.format).toBe("toml");
    expect(r.frontmatter).toEqual({
      title: "My Post",
      date: "2026-05-20",
      tags: ["typescript", "docs"],
      draft: false,
    });
    expect(r.body.trim().startsWith("## Introduction")).toBe(true);
  });
  it("typical Jekyll/VuePress post", () => {
    const md = `---
title: My Post
sidebar_position: 3
description: A short description
---

# My Post

Body.`;
    const r = extractFrontmatter(md);
    expect(r.format).toBe("yaml");
    expect(r.frontmatter).toEqual({
      title: "My Post",
      sidebar_position: 3,
      description: "A short description",
    });
  });
});
