/**
 * generate.test.ts — end-to-end generateStyleTags.
 */

import { describe, it, expect } from "vitest";
import { generateStyleTags } from "../src/index.js";

const SHA384_B64 = "A".repeat(64);

describe("generateStyleTags — empty / shape", () => {
  it("empty config → empty string", () => {
    expect(generateStyleTags({})).toBe("");
  });
  it("null config throws", () => {
    expect(() => generateStyleTags(null as unknown as never))
      .toThrow(/config must be a non-null object/);
  });
  it("string config throws", () => {
    expect(() => generateStyleTags("x" as unknown as never))
      .toThrow(/config must be a non-null object/);
  });
  it("empty arrays → empty string", () => {
    expect(generateStyleTags({ stylesheets: [], inline: [] })).toBe("");
  });
});

describe("generateStyleTags — external stylesheets", () => {
  it("minimal href-only", () => {
    expect(generateStyleTags({ stylesheets: [{ href: "/main.css" }] }))
      .toBe(`<link rel="stylesheet" href="/main.css">`);
  });
  it("with media", () => {
    expect(generateStyleTags({ stylesheets: [{ href: "/print.css", media: "print" }] }))
      .toBe(`<link rel="stylesheet" href="/print.css" media="print">`);
  });
  it("with media query", () => {
    expect(generateStyleTags({ stylesheets: [{ href: "/m.css", media: "(max-width: 600px)" }] }))
      .toBe(`<link rel="stylesheet" href="/m.css" media="(max-width: 600px)">`);
  });
  it("with SRI + crossorigin", () => {
    expect(generateStyleTags({
      stylesheets: [{ href: "https://cdn.example.com/x.css", integrity: `sha384-${SHA384_B64}`, crossorigin: "anonymous" }],
    }))
      .toBe(`<link rel="stylesheet" href="https://cdn.example.com/x.css" integrity="sha384-${SHA384_B64}" crossorigin="anonymous">`);
  });
  it("disabled boolean", () => {
    expect(generateStyleTags({ stylesheets: [{ href: "/x.css", disabled: true }] }))
      .toBe(`<link rel="stylesheet" href="/x.css" disabled>`);
  });
  it("disabled=false omits attr", () => {
    expect(generateStyleTags({ stylesheets: [{ href: "/x.css", disabled: false }] }))
      .toBe(`<link rel="stylesheet" href="/x.css">`);
  });
  it("attribute order: rel → href → media → integrity → crossorigin → disabled", () => {
    expect(generateStyleTags({
      stylesheets: [{
        disabled: true,
        crossorigin: "anonymous",
        integrity: `sha384-${SHA384_B64}`,
        media: "screen",
        href: "https://cdn.example.com/x.css",
      }],
    })).toBe(
      `<link rel="stylesheet" href="https://cdn.example.com/x.css" media="screen" integrity="sha384-${SHA384_B64}" crossorigin="anonymous" disabled>`,
    );
  });
  it("multiple stylesheets in caller's order", () => {
    expect(generateStyleTags({
      stylesheets: [{ href: "/a.css" }, { href: "/b.css" }, { href: "/c.css" }],
    })).toBe([
      `<link rel="stylesheet" href="/a.css">`,
      `<link rel="stylesheet" href="/b.css">`,
      `<link rel="stylesheet" href="/c.css">`,
    ].join("\n"));
  });
  it("rejects bad href", () => {
    expect(() => generateStyleTags({ stylesheets: [{ href: "javascript:x" }] }))
      .toThrow(/stylesheets\[0\]\.href must be http\(s\)/);
  });
  it("rejects bad integrity", () => {
    expect(() => generateStyleTags({ stylesheets: [{ href: "/x.css", integrity: "md5-abc" }] }))
      .toThrow(/stylesheets\[0\]\.integrity algo must be one of/);
  });
  it("rejects bad crossorigin", () => {
    expect(() => generateStyleTags({ stylesheets: [{ href: "/x.css", crossorigin: "true" as unknown as never }] }))
      .toThrow(/stylesheets\[0\]\.crossorigin must be one of/);
  });
  it("rejects non-bool disabled", () => {
    expect(() => generateStyleTags({ stylesheets: [{ href: "/x.css", disabled: 1 as unknown as boolean }] }))
      .toThrow(/disabled must be a boolean/);
  });
  it("rejects non-string media", () => {
    expect(() => generateStyleTags({ stylesheets: [{ href: "/x.css", media: 1 as unknown as string }] }))
      .toThrow(/media must be a string/);
  });
  it("rejects null stylesheet entry", () => {
    expect(() => generateStyleTags({ stylesheets: [null as unknown as never] }))
      .toThrow(/stylesheets\[0\] must be a non-null object/);
  });
  it("rejects non-array stylesheets", () => {
    expect(() => generateStyleTags({ stylesheets: "x" as unknown as never }))
      .toThrow(/stylesheets must be an array/);
  });
});

describe("generateStyleTags — inline styles", () => {
  it("minimal inline", () => {
    expect(generateStyleTags({ inline: [{ css: ":root { --c: blue; }" }] }))
      .toBe(`<style>:root { --c: blue; }</style>`);
  });
  it("empty CSS renders empty <style>", () => {
    expect(generateStyleTags({ inline: [{ css: "" }] }))
      .toBe(`<style></style>`);
  });
  it("inline with media", () => {
    expect(generateStyleTags({
      inline: [{ media: "(prefers-color-scheme: dark)", css: "body{background:#000}" }],
    })).toBe(
      `<style media="(prefers-color-scheme: dark)">body{background:#000}</style>`,
    );
  });
  it("CSS body NOT escaped (verbatim between <style> and </style>)", () => {
    // Browsers parse <style> contents in a special raw-text state;
    // escaping `<`/`>`/`&` would corrupt CSS selectors.
    const css = "a > b { color: red; }";
    expect(generateStyleTags({ inline: [{ css }] }))
      .toBe(`<style>${css}</style>`);
  });
  it("multiple inline blocks in order", () => {
    expect(generateStyleTags({
      inline: [
        { css: ".a {}" },
        { css: ".b {}" },
        { css: ".c {}" },
      ],
    })).toBe([
      `<style>.a {}</style>`,
      `<style>.b {}</style>`,
      `<style>.c {}</style>`,
    ].join("\n"));
  });
  it("rejects literal </style> in CSS", () => {
    expect(() => generateStyleTags({
      inline: [{ css: "body {} </style><script>alert(1)</script>" }],
    })).toThrow(/literal <\/style> sequence/);
  });
  it("rejects </STYLE> case-variant", () => {
    expect(() => generateStyleTags({ inline: [{ css: "body{} </STYLE> x" }] }))
      .toThrow(/literal <\/style>/);
  });
  it("rejects non-string CSS", () => {
    expect(() => generateStyleTags({ inline: [{ css: 42 as unknown as string }] }))
      .toThrow(/inline\[0\]\.css must be a string/);
  });
  it("rejects null inline entry", () => {
    expect(() => generateStyleTags({ inline: [null as unknown as never] }))
      .toThrow(/inline\[0\] must be a non-null object/);
  });
  it("rejects non-array inline", () => {
    expect(() => generateStyleTags({ inline: "x" as unknown as never }))
      .toThrow(/inline must be an array/);
  });
  it("media gets HTML-escaped (defensive)", () => {
    const out = generateStyleTags({ inline: [{ media: `screen"x`, css: "" }] });
    expect(out).toContain(`media="screen&quot;x"`);
  });
});

describe("generateStyleTags — output order", () => {
  it("stylesheets → inline", () => {
    const out = generateStyleTags({
      inline: [{ css: ".inline {}" }],
      stylesheets: [{ href: "/main.css" }],
    });
    expect(out).toBe([
      `<link rel="stylesheet" href="/main.css">`,
      `<style>.inline {}</style>`,
    ].join("\n"));
  });
});

describe("generateStyleTags — fail-fast", () => {
  it("bad stylesheet mid-array, no inline emitted", () => {
    expect(() => generateStyleTags({
      stylesheets: [{ href: "/a.css" }, { href: "javascript:bad" }],
      inline: [{ css: ".x {}" }],
    })).toThrow(/stylesheets\[1\]\.href/);
  });
  it("bad inline after good stylesheets, no output", () => {
    expect(() => generateStyleTags({
      stylesheets: [{ href: "/a.css" }],
      inline: [{ css: ".x {}" }, { css: "</style>" }],
    })).toThrow(/inline\[1\]\.css/);
  });
});

describe("generateStyleTags — HTML escaping (security)", () => {
  it("escapes ampersand in href", () => {
    expect(generateStyleTags({ stylesheets: [{ href: "https://example.com/?a=1&b=2" }] }))
      .toContain(`href="https://example.com/?a=1&amp;b=2"`);
  });
  it("escapes quote in media", () => {
    expect(generateStyleTags({ stylesheets: [{ href: "/x.css", media: `(max-width:600px) "x"` }] }))
      .toContain(`media="(max-width:600px) &quot;x&quot;"`);
  });
});

describe("generateStyleTags — purity / determinism", () => {
  it("same input → byte-identical output", () => {
    const cfg = { stylesheets: [{ href: "/main.css", media: "screen" }] };
    expect(generateStyleTags(cfg)).toBe(generateStyleTags(cfg));
  });
  it("does not mutate input", () => {
    const cfg = {
      stylesheets: [{ href: "/x.css", integrity: `sha384-${SHA384_B64}`, disabled: true }],
      inline: [{ css: ".x {}" }],
    };
    const before = JSON.stringify(cfg);
    generateStyleTags(cfg);
    expect(JSON.stringify(cfg)).toBe(before);
  });
});

describe("generateStyleTags — full real-world example", () => {
  it("multi-sheet + dark-mode override", () => {
    const out = generateStyleTags({
      stylesheets: [
        { href: "/reset.css" },
        { href: "/main.css", integrity: `sha384-${SHA384_B64}`, crossorigin: "anonymous" },
        { href: "/print.css", media: "print" },
      ],
      inline: [
        { css: ":root { --c: #0a0a0a; }" },
        { media: "(prefers-color-scheme: dark)", css: ":root { --c: #fafafa; }" },
      ],
    });
    expect(out).toBe([
      `<link rel="stylesheet" href="/reset.css">`,
      `<link rel="stylesheet" href="/main.css" integrity="sha384-${SHA384_B64}" crossorigin="anonymous">`,
      `<link rel="stylesheet" href="/print.css" media="print">`,
      `<style>:root { --c: #0a0a0a; }</style>`,
      `<style media="(prefers-color-scheme: dark)">:root { --c: #fafafa; }</style>`,
    ].join("\n"));
  });
});
