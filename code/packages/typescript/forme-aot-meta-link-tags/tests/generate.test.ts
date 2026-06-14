/**
 * generate.test.ts — end-to-end generateMetaLinkTags.
 */

import { describe, it, expect } from "vitest";
import { generateMetaLinkTags } from "../src/index.js";

describe("generateMetaLinkTags — empty / shape", () => {
  it("empty config → empty string", () => {
    expect(generateMetaLinkTags({})).toBe("");
  });
  it("null config throws", () => {
    expect(() => generateMetaLinkTags(null as unknown as never))
      .toThrow(/config must be a non-null object/);
  });
  it("string config throws", () => {
    expect(() => generateMetaLinkTags("nope" as unknown as never))
      .toThrow(/config must be a non-null object/);
  });
});

describe("generateMetaLinkTags — canonical / prev / next", () => {
  it("canonical only", () => {
    expect(generateMetaLinkTags({ canonical: "https://example.com/x" }))
      .toBe(`<link rel="canonical" href="https://example.com/x">`);
  });
  it("prev only", () => {
    expect(generateMetaLinkTags({ prev: "/page/1" }))
      .toBe(`<link rel="prev" href="/page/1">`);
  });
  it("next only", () => {
    expect(generateMetaLinkTags({ next: "/page/3" }))
      .toBe(`<link rel="next" href="/page/3">`);
  });
  it("all three in canonical → prev → next order", () => {
    const out = generateMetaLinkTags({
      next: "/page/3",     // intentionally reversed input order
      prev: "/page/1",
      canonical: "https://example.com/page/2",
    });
    expect(out).toBe([
      `<link rel="canonical" href="https://example.com/page/2">`,
      `<link rel="prev" href="/page/1">`,
      `<link rel="next" href="/page/3">`,
    ].join("\n"));
  });
  it("canonical rejects javascript:", () => {
    expect(() => generateMetaLinkTags({ canonical: "javascript:x" }))
      .toThrow(/canonical must be http\(s\)/);
  });
  it("prev rejects bad URL", () => {
    expect(() => generateMetaLinkTags({ prev: "//evil" }))
      .toThrow(/prev must be http\(s\)/);
  });
  it("next rejects bad URL", () => {
    expect(() => generateMetaLinkTags({ next: "data:bad" }))
      .toThrow(/next must be http\(s\)/);
  });
});

describe("generateMetaLinkTags — meta tags", () => {
  it("name + content", () => {
    expect(generateMetaLinkTags({ meta: [{ name: "description", content: "Hi" }] }))
      .toBe(`<meta name="description" content="Hi">`);
  });
  it("http-equiv + content", () => {
    expect(generateMetaLinkTags({ meta: [{ httpEquiv: "content-security-policy", content: "default-src 'self'" }] }))
      .toBe(`<meta http-equiv="content-security-policy" content="default-src &#39;self&#39;">`);
  });
  it("multiple meta tags in caller's order", () => {
    const out = generateMetaLinkTags({
      meta: [
        { name: "viewport", content: "width=device-width" },
        { name: "description", content: "Hello" },
        { name: "robots", content: "index,follow" },
      ],
    });
    expect(out).toBe([
      `<meta name="viewport" content="width=device-width">`,
      `<meta name="description" content="Hello">`,
      `<meta name="robots" content="index,follow">`,
    ].join("\n"));
  });
  it("rejects entry with both name AND httpEquiv", () => {
    expect(() => generateMetaLinkTags({
      meta: [{ name: "x", httpEquiv: "y", content: "z" }],
    })).toThrow(/exactly one of name\/httpEquiv/);
  });
  it("rejects entry with NEITHER name nor httpEquiv", () => {
    expect(() => generateMetaLinkTags({
      meta: [{ content: "z" } as unknown as never],
    })).toThrow(/exactly one of name\/httpEquiv/);
  });
  it("rejects empty name", () => {
    expect(() => generateMetaLinkTags({ meta: [{ name: "", content: "x" }] }))
      .toThrow(/must be non-empty/);
  });
  it("rejects empty httpEquiv", () => {
    expect(() => generateMetaLinkTags({ meta: [{ httpEquiv: "", content: "x" }] }))
      .toThrow(/must be non-empty/);
  });
  it("rejects null meta entry", () => {
    expect(() => generateMetaLinkTags({ meta: [null as unknown as never] }))
      .toThrow(/meta\[0\] must be a non-null object/);
  });
  it("rejects non-string content", () => {
    expect(() => generateMetaLinkTags({
      meta: [{ name: "x", content: 42 as unknown as string }],
    })).toThrow(/content must be a string/);
  });
  it("rejects non-string name", () => {
    expect(() => generateMetaLinkTags({
      meta: [{ name: 42 as unknown as string, content: "x" }],
    })).toThrow(/name must be a string/);
  });
  it("rejects non-string httpEquiv", () => {
    expect(() => generateMetaLinkTags({
      meta: [{ httpEquiv: true as unknown as string, content: "x" }],
    })).toThrow(/httpEquiv must be a string/);
  });
  it("rejects non-array meta", () => {
    expect(() => generateMetaLinkTags({ meta: "x" as unknown as never }))
      .toThrow(/meta must be an array/);
  });
  it("escapes XSS attempt in content", () => {
    const out = generateMetaLinkTags({
      meta: [{ name: "description", content: `<script>alert("x")</script>` }],
    });
    expect(out).toContain(`content="&lt;script&gt;alert(&quot;x&quot;)&lt;/script&gt;"`);
    expect(out).not.toContain("<script>alert");
  });
  it("escapes special chars in name", () => {
    const out = generateMetaLinkTags({
      meta: [{ name: `evil"x`, content: "y" }],
    });
    expect(out).toContain(`name="evil&quot;x"`);
  });
});

describe("generateMetaLinkTags — icons", () => {
  it("single icon defaults rel to 'icon'", () => {
    expect(generateMetaLinkTags({ icons: [{ href: "/favicon.png" }] }))
      .toBe(`<link rel="icon" href="/favicon.png">`);
  });
  it("icon with type + sizes", () => {
    expect(generateMetaLinkTags({
      icons: [{ href: "/favicon.png", type: "image/png", sizes: "32x32" }],
    }))
      .toBe(`<link rel="icon" type="image/png" sizes="32x32" href="/favicon.png">`);
  });
  it("apple-touch-icon rel", () => {
    expect(generateMetaLinkTags({
      icons: [{ href: "/apple-touch-icon.png", rel: "apple-touch-icon", sizes: "180x180" }],
    }))
      .toBe(`<link rel="apple-touch-icon" sizes="180x180" href="/apple-touch-icon.png">`);
  });
  it("multiple icons in caller's order", () => {
    const out = generateMetaLinkTags({
      icons: [
        { href: "/favicon-32.png", type: "image/png", sizes: "32x32" },
        { href: "/favicon-16.png", type: "image/png", sizes: "16x16" },
      ],
    });
    expect(out).toBe([
      `<link rel="icon" type="image/png" sizes="32x32" href="/favicon-32.png">`,
      `<link rel="icon" type="image/png" sizes="16x16" href="/favicon-16.png">`,
    ].join("\n"));
  });
  it("rejects bad icon rel", () => {
    expect(() => generateMetaLinkTags({
      icons: [{ href: "/f.png", rel: "manifest" as unknown as never }],
    })).toThrow(/icons\[0\]\.rel must be one of/);
  });
  it("rejects javascript: href", () => {
    expect(() => generateMetaLinkTags({
      icons: [{ href: "javascript:x" }],
    })).toThrow(/icons\[0\]\.href must be http\(s\)/);
  });
  it("rejects null icon", () => {
    expect(() => generateMetaLinkTags({ icons: [null as unknown as never] }))
      .toThrow(/icons\[0\] must be a non-null object/);
  });
  it("rejects non-array icons", () => {
    expect(() => generateMetaLinkTags({ icons: "x" as unknown as never }))
      .toThrow(/icons must be an array/);
  });
  it("escapes special chars in sizes", () => {
    const out = generateMetaLinkTags({
      icons: [{ href: "/f.png", sizes: `32x32"` }],
    });
    expect(out).toContain(`sizes="32x32&quot;"`);
  });
});

describe("generateMetaLinkTags — resource hints", () => {
  it("preload requires as", () => {
    expect(() => generateMetaLinkTags({
      preload: [{ href: "/main.js", rel: "preload" }],
    })).toThrow(/preload\[0\]\.as is required when rel="preload"/);
  });
  it("modulepreload requires as", () => {
    expect(() => generateMetaLinkTags({
      preload: [{ href: "/m.js", rel: "modulepreload" }],
    })).toThrow(/required when rel="modulepreload"/);
  });
  it("preload script", () => {
    expect(generateMetaLinkTags({
      preload: [{ href: "/main.js", rel: "preload", as: "script" }],
    }))
      .toBe(`<link rel="preload" as="script" href="/main.js">`);
  });
  it("preload font with type + crossorigin", () => {
    expect(generateMetaLinkTags({
      preload: [{ href: "/inter.woff2", rel: "preload", as: "font", type: "font/woff2", crossorigin: "anonymous" }],
    }))
      .toBe(`<link rel="preload" as="font" type="font/woff2" crossorigin="anonymous" href="/inter.woff2">`);
  });
  it("preconnect (no as)", () => {
    expect(generateMetaLinkTags({
      preload: [{ href: "https://fonts.example.com", rel: "preconnect" }],
    }))
      .toBe(`<link rel="preconnect" href="https://fonts.example.com">`);
  });
  it("preconnect with as gets `as` dropped (not valid HTML)", () => {
    const out = generateMetaLinkTags({
      preload: [{ href: "https://fonts.example.com", rel: "preconnect", as: "font" }],
    });
    expect(out).toBe(`<link rel="preconnect" href="https://fonts.example.com">`);
    expect(out).not.toContain("as=");
  });
  it("dns-prefetch", () => {
    expect(generateMetaLinkTags({
      preload: [{ href: "https://cdn.example.com", rel: "dns-prefetch" }],
    }))
      .toBe(`<link rel="dns-prefetch" href="https://cdn.example.com">`);
  });
  it("prefetch", () => {
    expect(generateMetaLinkTags({
      preload: [{ href: "/next-page.html", rel: "prefetch" }],
    }))
      .toBe(`<link rel="prefetch" href="/next-page.html">`);
  });
  it("modulepreload with as=script", () => {
    expect(generateMetaLinkTags({
      preload: [{ href: "/m.js", rel: "modulepreload", as: "script" }],
    }))
      .toBe(`<link rel="modulepreload" as="script" href="/m.js">`);
  });
  it("preconnect with crossorigin use-credentials", () => {
    expect(generateMetaLinkTags({
      preload: [{ href: "https://api.example.com", rel: "preconnect", crossorigin: "use-credentials" }],
    }))
      .toBe(`<link rel="preconnect" crossorigin="use-credentials" href="https://api.example.com">`);
  });
  it("rejects bad rel", () => {
    expect(() => generateMetaLinkTags({
      preload: [{ href: "/x", rel: "yolo" as unknown as never }],
    })).toThrow(/preload\[0\]\.rel must be one of/);
  });
  it("rejects bad as on preload", () => {
    expect(() => generateMetaLinkTags({
      preload: [{ href: "/x", rel: "preload", as: "iframe" as unknown as never }],
    })).toThrow(/preload\[0\]\.as must be one of/);
  });
  it("rejects bad crossorigin", () => {
    expect(() => generateMetaLinkTags({
      preload: [{ href: "/x", rel: "preconnect", crossorigin: "true" as unknown as never }],
    })).toThrow(/preload\[0\]\.crossorigin must be one of/);
  });
  it("rejects javascript: href", () => {
    expect(() => generateMetaLinkTags({
      preload: [{ href: "javascript:x", rel: "preload", as: "script" }],
    })).toThrow(/preload\[0\]\.href must be http\(s\)/);
  });
  it("rejects null hint", () => {
    expect(() => generateMetaLinkTags({ preload: [null as unknown as never] }))
      .toThrow(/preload\[0\] must be a non-null object/);
  });
  it("rejects non-array preload", () => {
    expect(() => generateMetaLinkTags({ preload: "x" as unknown as never }))
      .toThrow(/preload must be an array/);
  });
  it("rejects non-string type", () => {
    expect(() => generateMetaLinkTags({
      preload: [{ href: "/x", rel: "preload", as: "font", type: 1 as unknown as string }],
    })).toThrow(/preload\[0\]\.type must be a string/);
  });
});

describe("generateMetaLinkTags — output order", () => {
  it("meta → canonical → prev → next → icons → hints", () => {
    const out = generateMetaLinkTags({
      preload:    [{ href: "/main.js", rel: "preload", as: "script" }],
      icons:      [{ href: "/favicon.png" }],
      next:       "/next",
      prev:       "/prev",
      canonical:  "https://example.com",
      meta:       [{ name: "viewport", content: "width=device-width" }],
    });
    const lines = out.split("\n");
    expect(lines).toEqual([
      `<meta name="viewport" content="width=device-width">`,
      `<link rel="canonical" href="https://example.com">`,
      `<link rel="prev" href="/prev">`,
      `<link rel="next" href="/next">`,
      `<link rel="icon" href="/favicon.png">`,
      `<link rel="preload" as="script" href="/main.js">`,
    ]);
  });
});

describe("generateMetaLinkTags — fail-fast (no partial output)", () => {
  it("bad icon URL in mid-array throws — no XML emitted", () => {
    try {
      generateMetaLinkTags({
        icons: [
          { href: "/ok.png" },
          { href: "javascript:bad" },
          { href: "/also-ok.png" },
        ],
      });
      expect.fail("expected throw");
    } catch (e) {
      expect((e as Error).message).toMatch(/icons\[1\]\.href must be http\(s\)/);
    }
  });
  it("bad meta entry throws before canonical is emitted", () => {
    expect(() => generateMetaLinkTags({
      meta: [{ name: "x", content: "y" }, { content: "z" } as unknown as never],
      canonical: "https://example.com",
    })).toThrow(/meta\[1\] must have exactly one/);
  });
});

describe("generateMetaLinkTags — HTML escaping security", () => {
  it("escapes ampersand in canonical URL", () => {
    expect(generateMetaLinkTags({ canonical: "https://example.com/?a=1&b=2" }))
      .toBe(`<link rel="canonical" href="https://example.com/?a=1&amp;b=2">`);
  });
  it("escapes quotes in meta content", () => {
    expect(generateMetaLinkTags({ meta: [{ name: "x", content: `say "hi"` }] }))
      .toContain(`content="say &quot;hi&quot;"`);
  });
  it("strips control bytes from meta content", () => {
    expect(generateMetaLinkTags({ meta: [{ name: "x", content: "ab\x00cd" }] }))
      .toContain(`content="abcd"`);
  });
});

describe("generateMetaLinkTags — purity / determinism", () => {
  it("same input → byte-identical output", () => {
    const cfg = {
      canonical: "https://example.com",
      meta: [{ name: "viewport", content: "x" }],
      icons: [{ href: "/f.png", sizes: "32x32" }],
    };
    expect(generateMetaLinkTags(cfg)).toBe(generateMetaLinkTags(cfg));
  });
  it("does not mutate input", () => {
    const cfg = {
      canonical: "https://example.com",
      icons: [{ href: "/f.png" }],
      preload: [{ href: "/m.js", rel: "preload" as const, as: "script" as const }],
      meta: [{ name: "viewport", content: "x" }],
    };
    const before = JSON.stringify(cfg);
    generateMetaLinkTags(cfg);
    expect(JSON.stringify(cfg)).toBe(before);
  });
});

describe("generateMetaLinkTags — full real-world example", () => {
  it("blog-post head with everything", () => {
    const out = generateMetaLinkTags({
      meta: [
        { name: "viewport", content: "width=device-width, initial-scale=1" },
        { name: "description", content: "A blog post about feeds." },
        { httpEquiv: "x-ua-compatible", content: "IE=edge" },
      ],
      canonical: "https://example.com/blog/post-1",
      prev: "/blog/page/0",
      next: "/blog/page/2",
      icons: [
        { href: "/favicon.svg", type: "image/svg+xml" },
        { href: "/apple-touch-icon.png", rel: "apple-touch-icon", sizes: "180x180" },
      ],
      preload: [
        { href: "/main.js", rel: "preload", as: "script" },
        { href: "/inter.woff2", rel: "preload", as: "font", type: "font/woff2", crossorigin: "anonymous" },
        { href: "https://fonts.example.com", rel: "preconnect", crossorigin: "anonymous" },
      ],
    });
    expect(out).toBe([
      `<meta name="viewport" content="width=device-width, initial-scale=1">`,
      `<meta name="description" content="A blog post about feeds.">`,
      `<meta http-equiv="x-ua-compatible" content="IE=edge">`,
      `<link rel="canonical" href="https://example.com/blog/post-1">`,
      `<link rel="prev" href="/blog/page/0">`,
      `<link rel="next" href="/blog/page/2">`,
      `<link rel="icon" type="image/svg+xml" href="/favicon.svg">`,
      `<link rel="apple-touch-icon" sizes="180x180" href="/apple-touch-icon.png">`,
      `<link rel="preload" as="script" href="/main.js">`,
      `<link rel="preload" as="font" type="font/woff2" crossorigin="anonymous" href="/inter.woff2">`,
      `<link rel="preconnect" crossorigin="anonymous" href="https://fonts.example.com">`,
    ].join("\n"));
  });
});
