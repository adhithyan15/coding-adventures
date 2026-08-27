import { describe, expect, it } from "vitest";

import type { LetterDuctus } from "../src/strokes";
import { assembleDuctusRegistry, type DuctusOwner } from "../src/strokes/registry";

const letter = (script: string, glyph: string): LetterDuctus => ({
  script,
  glyph,
  strokes: [{ segments: [{ label: "test", path: [{ x: 0, y: 0 }] }] }],
  source: { citation: "test", url: "https://example.test/source" },
});

describe("Script Ductus owner registry", () => {
  it("preserves fixed owner and entry order in an ordinary object", () => {
    const owners: DuctusOwner[] = [
      { owner: "first", entries: [["first:a", letter("first", "a")]] },
      { owner: "second", entries: [
        ["second:b", letter("second", "b")],
        ["second:c", letter("second", "c")],
      ] },
    ];

    const registry = assembleDuctusRegistry(owners);
    expect(Object.keys(registry)).toEqual(["first:a", "second:b", "second:c"]);
    expect(Object.getPrototypeOf(registry)).toBe(Object.prototype);
  });

  it("rejects duplicate keys inside one owner", () => {
    const duplicate: DuctusOwner = {
      owner: "one",
      entries: [
        ["one:a", letter("one", "a")],
        ["one:a", letter("one", "a")],
      ],
    };
    expect(() => assembleDuctusRegistry([duplicate])).toThrow(
      "Script Ductus owner one repeats key one:a",
    );
  });

  it("rejects duplicate keys across owners", () => {
    const first: DuctusOwner = { owner: "first", entries: [["shared:a", letter("first", "a")]] };
    const second: DuctusOwner = { owner: "second", entries: [["shared:a", letter("second", "a")]] };
    expect(() => assembleDuctusRegistry([first, second])).toThrow(
      "Script Ductus owners first and second both claim key shared:a",
    );
  });
});
