import { describe, expect, it } from "vitest";
import {
  collectElementIdCounts,
  prepareInteractivity,
} from "../src/interactivity.js";

const ASSET_ID = "01952c0d-7e63-7000-8000-000000000090";
const SCRIPT_SHA256 = "a".repeat(64);

describe("element ID collection", () => {
  it("counts quoted, unquoted, case-insensitive, and numeric-entity IDs", () => {
    const ids = collectElementIdCounts([
      '<DIV ID="alpha"></DIV>',
      "<p id=beta></p>",
      "<i id='&#x67;amma'></i>",
      "<b id=alpha></b>",
    ].join(""));
    expect([...ids]).toEqual([["alpha", 2], ["beta", 1], ["gamma", 1]]);
  });

  it("ignores comments, raw-text bodies, and detached template content", () => {
    const ids = collectElementIdCounts([
      '<!-- <div id="comment"></div> -->',
      '<script>"<div id=script></div>"</script>',
      '<style>/* <i id=style> */</style>',
      '<textarea><b id=textarea></b></textarea>',
      '<title><b id=title></b></title>',
      '<template><b id=template></b></template>',
      '<main id="real"></main>',
    ].join(""));
    expect([...ids]).toEqual([["real", 1]]);
  });

  it("terminates conservatively on malformed comments and tags", () => {
    expect([...collectElementIdCounts("plain < text")]).toEqual([]);
    expect([...collectElementIdCounts("<!-- never closes <b id=x>")]).toEqual([]);
    expect([...collectElementIdCounts("<!doctype html><main id=x>")]).toEqual([["x", 1]]);
  });

  it("uses browser duplicate-attribute, semicolonless-reference, and tree-construction rules", () => {
    const ids = collectElementIdCounts([
      '<div id="first" id="second"></div>',
      '<i id="&#115;econd"></i>',
      '<plaintext><b id="not-markup"></b>',
    ].join(""));
    expect([...ids]).toEqual([["first", 1], ["second", 1]]);
  });

  // The depth is the point: 10,000 levels would overflow a recursive walker.
  // The cost is in parse5, not the walk. Its open-element checks make parsing
  // quadratic in nesting depth (measured idle: 5k -> 169ms, 10k -> 640ms,
  // 20k -> 2.6s). So this test took ~0.7s idle and 5.8s on a CI runner under
  // a full-repository rebuild, over vitest's 5s default. It states its own
  // budget instead of shrinking the depth that makes it meaningful.
  it("walks deeply nested HTML iteratively", () => {
    const depth = 10_000;
    const html = `${"<div>".repeat(depth)}<button id=deep></button>${"</div>".repeat(depth)}`;
    expect([...collectElementIdCounts(html)]).toEqual([["deep", 1]]);
  }, 30_000);
});

describe("interactivity composition config", () => {
  const emptyDocument = {
    kind: "Interactivity",
    version: 1,
    state: [],
    bindings: [],
    handlers: [],
    islands: [],
  };

  it("accepts empty defaults and snapshots validated route documents", () => {
    expect(prepareInteractivity(undefined, undefined).byRoute.size).toBe(0);
    const prepared = prepareInteractivity(
      [{ route: "/a", document: emptyDocument }],
      [{ packageName: "@example/island", export: "enhance", assetId: ASSET_ID, sha256: SCRIPT_SHA256 }],
    );
    expect(prepared.byRoute.get("/a")).toEqual(emptyDocument);
    expect(prepared.modules.size).toBe(1);
  });

  it("rejects hostile array and object shapes without invoking accessors", () => {
    expect(() => prepareInteractivity(new Proxy([], {}), [])).toThrow(/must be an array/);
    const sparse = new Array(1);
    expect(() => prepareInteractivity(sparse, [])).toThrow(/sparse/);
    const decorated: unknown[] = [];
    Object.defineProperty(decorated, "hidden", { value: true });
    expect(() => prepareInteractivity(decorated, [])).toThrow(/hidden is unknown/);
    const accessor = { route: "/a", document: emptyDocument };
    Object.defineProperty(accessor, "route", { get: () => "/trap", enumerable: true });
    expect(() => prepareInteractivity([accessor], [])).toThrow(/must not be an accessor/);
    const symbol = { route: "/a", document: emptyDocument, [Symbol("x")]: true };
    expect(() => prepareInteractivity([symbol], [])).toThrow(/symbol keys/);
    expect(() => prepareInteractivity([new (class Entry {})()], [])).toThrow(/plain object/);
    expect(() => prepareInteractivity([{ route: "/a", document: emptyDocument, extra: true }], []))
      .toThrow(/extra is unknown/);
    const hidden = { route: "/a", document: emptyDocument };
    Object.defineProperty(hidden, "extra", { value: true });
    expect(() => prepareInteractivity([hidden], [])).toThrow(/extra is unknown/);
  });

  it("rejects ambiguous routes and malformed module identities", () => {
    expect(() => prepareInteractivity([
      { route: "/a", document: emptyDocument },
      { route: "/a", document: emptyDocument },
    ], [])).toThrow(/duplicate interactivity route/);
    expect(() => prepareInteractivity([{ route: "relative", document: emptyDocument }], []))
      .toThrow(/root-relative route/);
    expect(() => prepareInteractivity([], [{ packageName: "Bad Name", export: "enhance", assetId: ASSET_ID, sha256: SCRIPT_SHA256 }]))
      .toThrow(/packageName is invalid/);
    expect(() => prepareInteractivity([], [{ packageName: "good", export: "bad-export", assetId: ASSET_ID, sha256: SCRIPT_SHA256 }]))
      .toThrow(/export is invalid/);
    expect(() => prepareInteractivity([], [{ packageName: "good", export: "enhance", assetId: "bad", sha256: SCRIPT_SHA256 }]))
      .toThrow(/UUIDv7/);
    expect(() => prepareInteractivity([], [{ packageName: "good", export: "enhance", assetId: ASSET_ID, sha256: "bad" }]))
      .toThrow(/sha256/);
  });
});
