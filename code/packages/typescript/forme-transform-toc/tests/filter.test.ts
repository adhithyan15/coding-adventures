/**
 * filter.test.ts — level-range filtering.
 */

import { describe, it, expect } from "vitest";
import type { HeadingSlug } from "@coding-adventures/forme-transform-autolink-headings";
import { filterByLevel } from "../src/index.js";

function s(level: 1|2|3|4|5|6, label = `h${level}`): HeadingSlug {
  return { level, text: label, slug: label, anchorHref: `#${label}` };
}

const SAMPLE: HeadingSlug[] = [
  s(1, "title"),
  s(2, "a"),
  s(3, "a1"),
  s(2, "b"),
  s(4, "b1a"),
];

describe("filterByLevel — defaults [1, 6] keep everything", () => {
  it("returns all 5 entries", () => {
    expect(filterByLevel(SAMPLE, 1, 6)).toEqual(SAMPLE);
  });

  it("preserves input order", () => {
    expect(filterByLevel(SAMPLE, 1, 6).map((x) => x.slug))
      .toEqual(["title", "a", "a1", "b", "b1a"]);
  });
});

describe("filterByLevel — minLevel drops shallow headings", () => {
  it("minLevel: 2 drops the h1", () => {
    const out = filterByLevel(SAMPLE, 2, 6);
    expect(out.map((x) => x.slug)).toEqual(["a", "a1", "b", "b1a"]);
  });

  it("minLevel: 3 drops h1 and h2", () => {
    const out = filterByLevel(SAMPLE, 3, 6);
    expect(out.map((x) => x.slug)).toEqual(["a1", "b1a"]);
  });
});

describe("filterByLevel — maxLevel drops deep headings", () => {
  it("maxLevel: 3 drops the h4", () => {
    const out = filterByLevel(SAMPLE, 1, 3);
    expect(out.map((x) => x.slug)).toEqual(["title", "a", "a1", "b"]);
  });

  it("maxLevel: 2 drops h3+h4", () => {
    const out = filterByLevel(SAMPLE, 1, 2);
    expect(out.map((x) => x.slug)).toEqual(["title", "a", "b"]);
  });
});

describe("filterByLevel — combined min+max", () => {
  it("[2, 3] keeps only h2/h3", () => {
    expect(filterByLevel(SAMPLE, 2, 3).map((x) => x.slug))
      .toEqual(["a", "a1", "b"]);
  });
});

describe("filterByLevel — out-of-range options clamped to [1, 6]", () => {
  it("minLevel < 1 clamps to 1", () => {
    expect(filterByLevel(SAMPLE, 0, 6)).toEqual(SAMPLE);
  });

  it("minLevel negative clamps to 1", () => {
    expect(filterByLevel(SAMPLE, -10, 6)).toEqual(SAMPLE);
  });

  it("maxLevel > 6 clamps to 6", () => {
    expect(filterByLevel(SAMPLE, 1, 99)).toEqual(SAMPLE);
  });

  it("NaN clamps", () => {
    expect(filterByLevel(SAMPLE, NaN, NaN)).toEqual([s(1, "title")]);
  });

  it("Infinity clamps", () => {
    expect(filterByLevel(SAMPLE, -Infinity, Infinity)).toEqual(SAMPLE);
  });

  it("fractional minLevel floors", () => {
    expect(filterByLevel(SAMPLE, 2.7, 6).map((x) => x.slug))
      .toEqual(["a", "a1", "b", "b1a"]);
  });
});

describe("filterByLevel — inverted range", () => {
  it("minLevel > maxLevel → empty array", () => {
    expect(filterByLevel(SAMPLE, 5, 2)).toEqual([]);
  });
});

describe("filterByLevel — purity", () => {
  it("does not mutate input", () => {
    const before = JSON.stringify(SAMPLE);
    filterByLevel(SAMPLE, 2, 4);
    expect(JSON.stringify(SAMPLE)).toBe(before);
  });

  it("returns a fresh array each call", () => {
    const a = filterByLevel(SAMPLE, 1, 6);
    const b = filterByLevel(SAMPLE, 1, 6);
    expect(a).not.toBe(b);
  });

  it("empty input → empty output", () => {
    expect(filterByLevel([], 1, 6)).toEqual([]);
  });
});
