/**
 * generate.test.ts — end-to-end generateScriptTags.
 */

import { describe, it, expect } from "vitest";
import { generateScriptTags } from "../src/index.js";

const SHA256_B64 = "A".repeat(43) + "=";
const SHA384_B64 = "A".repeat(64);

describe("generateScriptTags — single tag", () => {
  it("minimal src-only emits classic script", () => {
    expect(generateScriptTags({ src: "/main.js" }))
      .toBe(`<script src="/main.js"></script>`);
  });
  it("type module", () => {
    expect(generateScriptTags({ src: "/main.js", type: "module" }))
      .toBe(`<script type="module" src="/main.js"></script>`);
  });
  it("type importmap", () => {
    expect(generateScriptTags({ src: "/im.json", type: "importmap" }))
      .toBe(`<script type="importmap" src="/im.json"></script>`);
  });
  it("with SRI sha384 + crossorigin", () => {
    const out = generateScriptTags({
      src: "https://cdn.example.com/x.js",
      integrity: `sha384-${SHA384_B64}`,
      crossorigin: "anonymous",
    });
    expect(out).toBe(
      `<script src="https://cdn.example.com/x.js" integrity="sha384-${SHA384_B64}" crossorigin="anonymous"></script>`,
    );
  });
  it("async boolean", () => {
    expect(generateScriptTags({ src: "/app.js", async: true }))
      .toBe(`<script src="/app.js" async></script>`);
  });
  it("defer boolean", () => {
    expect(generateScriptTags({ src: "/app.js", defer: true }))
      .toBe(`<script src="/app.js" defer></script>`);
  });
  it("nomodule boolean", () => {
    expect(generateScriptTags({ src: "/legacy.js", nomodule: true }))
      .toBe(`<script src="/legacy.js" nomodule></script>`);
  });
  it("referrerpolicy", () => {
    expect(generateScriptTags({ src: "/x.js", referrerpolicy: "no-referrer" }))
      .toBe(`<script src="/x.js" referrerpolicy="no-referrer"></script>`);
  });
  it("absolute https URL passes through", () => {
    expect(generateScriptTags({ src: "https://example.com/x.js" }))
      .toBe(`<script src="https://example.com/x.js"></script>`);
  });
  it("false booleans are not emitted", () => {
    expect(generateScriptTags({ src: "/a.js", async: false, defer: false, nomodule: false }))
      .toBe(`<script src="/a.js"></script>`);
  });
});

describe("generateScriptTags — attribute order", () => {
  it("type → src → integrity → crossorigin → referrerpolicy → async → defer → nomodule", () => {
    const out = generateScriptTags({
      nomodule: true,
      async: true,
      referrerpolicy: "no-referrer",
      crossorigin: "anonymous",
      integrity: `sha384-${SHA384_B64}`,
      src: "https://cdn.example.com/x.js",
      type: "module",
    });
    // async + defer can't coexist, so defer omitted from this test.
    expect(out).toBe(
      `<script type="module" src="https://cdn.example.com/x.js" integrity="sha384-${SHA384_B64}" crossorigin="anonymous" referrerpolicy="no-referrer" async nomodule></script>`,
    );
  });
  it("defer ordered before nomodule", () => {
    const out = generateScriptTags({ src: "/x.js", defer: true, nomodule: true });
    expect(out).toBe(`<script src="/x.js" defer nomodule></script>`);
  });
});

describe("generateScriptTags — array of tags", () => {
  it("multiple tags joined by newline", () => {
    const out = generateScriptTags([
      { src: "/a.js" },
      { src: "/b.js", type: "module" },
      { src: "/c.js", defer: true },
    ]);
    expect(out).toBe([
      `<script src="/a.js"></script>`,
      `<script type="module" src="/b.js"></script>`,
      `<script src="/c.js" defer></script>`,
    ].join("\n"));
  });
  it("empty array → empty string", () => {
    expect(generateScriptTags([])).toBe("");
  });
  it("single-element array same as single object", () => {
    const single = generateScriptTags({ src: "/x.js" });
    const arr    = generateScriptTags([{ src: "/x.js" }]);
    expect(arr).toBe(single);
  });
  it("preserves caller's order", () => {
    const out = generateScriptTags([
      { src: "/c.js" }, { src: "/a.js" }, { src: "/b.js" },
    ]);
    const cIdx = out.indexOf("/c.js");
    const aIdx = out.indexOf("/a.js");
    const bIdx = out.indexOf("/b.js");
    expect(cIdx).toBeLessThan(aIdx);
    expect(aIdx).toBeLessThan(bIdx);
  });
});

describe("generateScriptTags — URL validation", () => {
  it("javascript: throws", () => {
    expect(() => generateScriptTags({ src: "javascript:alert(1)" })).toThrow(/src must be http\(s\)/);
  });
  it("data: throws", () => {
    expect(() => generateScriptTags({ src: "data:text/x,1" })).toThrow(/http\(s\)/);
  });
  it("file: throws", () => {
    expect(() => generateScriptTags({ src: "file:///etc/passwd" })).toThrow(/http\(s\)/);
  });
  it("protocol-relative throws", () => {
    expect(() => generateScriptTags({ src: "//evil.com/x.js" })).toThrow(/http\(s\)/);
  });
  it("backslash-variant throws", () => {
    expect(() => generateScriptTags({ src: "/\\evil.com/x.js" })).toThrow(/http\(s\)/);
  });
  it("bare relative throws", () => {
    expect(() => generateScriptTags({ src: "x.js" })).toThrow(/http\(s\)/);
  });
});

describe("generateScriptTags — SRI integrity", () => {
  it("valid sha384 emitted verbatim", () => {
    expect(generateScriptTags({ src: "/x.js", integrity: `sha384-${SHA384_B64}` }))
      .toContain(`integrity="sha384-${SHA384_B64}"`);
  });
  it("two-algo SRI emitted verbatim", () => {
    const sri = `sha256-${SHA256_B64} sha384-${SHA384_B64}`;
    expect(generateScriptTags({ src: "/x.js", integrity: sri }))
      .toContain(`integrity="${sri}"`);
  });
  it("md5 algo rejected", () => {
    expect(() => generateScriptTags({ src: "/x.js", integrity: "md5-abc" }))
      .toThrow(/algo must be one of/);
  });
  it("wrong-length base64 rejected", () => {
    expect(() => generateScriptTags({ src: "/x.js", integrity: "sha256-abc" }))
      .toThrow(/sha256 expects 44/);
  });
});

describe("generateScriptTags — allowlist validation", () => {
  it("type 'text/javascript' rejected", () => {
    expect(() => generateScriptTags({ src: "/x.js", type: "text/javascript" as unknown as never }))
      .toThrow(/type must be one of/);
  });
  it("crossorigin 'true' rejected", () => {
    expect(() => generateScriptTags({ src: "/x.js", crossorigin: "true" as unknown as never }))
      .toThrow(/crossorigin must be one of/);
  });
  it("referrerpolicy 'never' rejected", () => {
    expect(() => generateScriptTags({ src: "/x.js", referrerpolicy: "never" as unknown as never }))
      .toThrow(/referrerpolicy must be one of/);
  });
});

describe("generateScriptTags — async + defer conflict", () => {
  it("both true rejected", () => {
    expect(() => generateScriptTags({ src: "/x.js", async: true, defer: true }))
      .toThrow(/cannot set both async and defer/);
  });
  it("async true + defer false ok", () => {
    expect(generateScriptTags({ src: "/x.js", async: true, defer: false }))
      .toBe(`<script src="/x.js" async></script>`);
  });
  it("async false + defer true ok", () => {
    expect(generateScriptTags({ src: "/x.js", async: false, defer: true }))
      .toBe(`<script src="/x.js" defer></script>`);
  });
});

describe("generateScriptTags — boolean type-checking", () => {
  it("non-boolean async rejected", () => {
    expect(() => generateScriptTags({ src: "/x.js", async: 1 as unknown as boolean }))
      .toThrow(/async must be a boolean/);
  });
  it("non-boolean defer rejected", () => {
    expect(() => generateScriptTags({ src: "/x.js", defer: "yes" as unknown as boolean }))
      .toThrow(/defer must be a boolean/);
  });
  it("non-boolean nomodule rejected", () => {
    expect(() => generateScriptTags({ src: "/x.js", nomodule: null as unknown as boolean }))
      .toThrow(/nomodule must be a boolean/);
  });
});

describe("generateScriptTags — input shape validation", () => {
  it("null entry in array throws with index", () => {
    expect(() => generateScriptTags([null as unknown as never]))
      .toThrow(/input\[0\] must be a non-null object/);
  });
  it("non-object entry throws", () => {
    expect(() => generateScriptTags(["x" as unknown as never]))
      .toThrow(/input\[0\] must be a non-null object/);
  });
  it("error in second entry shows index 1", () => {
    expect(() => generateScriptTags([
      { src: "/ok.js" },
      { src: "javascript:bad" },
    ])).toThrow(/src must be http\(s\)/);
  });
});

describe("generateScriptTags — HTML escaping", () => {
  it("escapes ampersand in src", () => {
    expect(generateScriptTags({ src: "https://example.com/?a=1&b=2" }))
      .toContain(`src="https://example.com/?a=1&amp;b=2"`);
  });
  it("rejects control byte in src (security: silent strip would redirect to different URL)", () => {
    expect(() => generateScriptTags({ src: "/main.js\x00" }))
      .toThrow(/must not contain ASCII control bytes/);
  });
  it("rejects tab in src (otherwise /\\tevil would become /evil after escape)", () => {
    expect(() => generateScriptTags({ src: "/\tevil" }))
      .toThrow(/must not contain ASCII control bytes/);
  });
});

describe("generateScriptTags — fail-fast (no partial output)", () => {
  it("bad entry mid-array throws — no output", () => {
    try {
      generateScriptTags([
        { src: "/a.js" },
        { src: "/b.js", integrity: "bad-format" },
        { src: "/c.js" },
      ]);
      expect.fail("expected throw");
    } catch (e) {
      expect((e as Error).message).toMatch(/algo must be one of/);
    }
  });
});

describe("generateScriptTags — purity / determinism", () => {
  it("same input → byte-identical output", () => {
    const cfg = { src: "/main.js", type: "module" as const, defer: true };
    expect(generateScriptTags(cfg)).toBe(generateScriptTags(cfg));
  });
  it("does not mutate input", () => {
    const cfg = { src: "/x.js", integrity: `sha384-${SHA384_B64}`, async: true };
    const before = JSON.stringify(cfg);
    generateScriptTags(cfg);
    expect(JSON.stringify(cfg)).toBe(before);
  });
  it("does not mutate array input", () => {
    const arr = [{ src: "/a.js" }, { src: "/b.js" }];
    const before = JSON.stringify(arr);
    generateScriptTags(arr);
    expect(JSON.stringify(arr)).toBe(before);
  });
});

describe("generateScriptTags — full real-world example", () => {
  it("app + analytics + legacy fallback", () => {
    const out = generateScriptTags([
      { src: "/main.js", type: "module", integrity: `sha384-${SHA384_B64}`, crossorigin: "anonymous" },
      { src: "/legacy.js", nomodule: true, defer: true },
      { src: "https://analytics.example.com/a.js", async: true, referrerpolicy: "no-referrer" },
    ]);
    expect(out).toBe([
      `<script type="module" src="/main.js" integrity="sha384-${SHA384_B64}" crossorigin="anonymous"></script>`,
      `<script src="/legacy.js" defer nomodule></script>`,
      `<script src="https://analytics.example.com/a.js" referrerpolicy="no-referrer" async></script>`,
    ].join("\n"));
  });
});
