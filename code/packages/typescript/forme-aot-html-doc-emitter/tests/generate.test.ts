/**
 * generate.test.ts — end-to-end generateHtmlDocument.
 */

import { describe, it, expect } from "vitest";
import { generateHtmlDocument } from "../src/index.js";

describe("generateHtmlDocument — shape", () => {
  it("null config throws", () => {
    expect(() => generateHtmlDocument(null as unknown as never))
      .toThrow(/config must be a non-null object/);
  });
  it("string config throws", () => {
    expect(() => generateHtmlDocument("x" as unknown as never))
      .toThrow(/config must be a non-null object/);
  });
  it("missing head throws", () => {
    expect(() => generateHtmlDocument({ body: "x" } as unknown as never))
      .toThrow(/head must be a string/);
  });
  it("missing body throws", () => {
    expect(() => generateHtmlDocument({ head: "x" } as unknown as never))
      .toThrow(/body must be a string/);
  });
  it("non-string head throws", () => {
    expect(() => generateHtmlDocument({ head: 42 as unknown as string, body: "" }))
      .toThrow(/head must be a string/);
  });
});

describe("generateHtmlDocument — minimal", () => {
  it("empty head + empty body", () => {
    expect(generateHtmlDocument({ head: "", body: "" })).toBe([
      "<!doctype html>",
      "<html>",
      "<head>",
      "",
      "</head>",
      "<body>",
      "",
      "</body>",
      "</html>",
    ].join("\n"));
  });
  it("populated head + body", () => {
    expect(generateHtmlDocument({
      head: "<title>Hi</title>",
      body: "<h1>Hi</h1>",
    })).toBe([
      "<!doctype html>",
      "<html>",
      "<head>",
      "<title>Hi</title>",
      "</head>",
      "<body>",
      "<h1>Hi</h1>",
      "</body>",
      "</html>",
    ].join("\n"));
  });
});

describe("generateHtmlDocument — lang + dir", () => {
  it("lang only", () => {
    const out = generateHtmlDocument({ lang: "en", head: "", body: "" });
    expect(out).toContain(`<html lang="en">`);
  });
  it("dir only", () => {
    const out = generateHtmlDocument({ dir: "rtl", head: "", body: "" });
    expect(out).toContain(`<html dir="rtl">`);
  });
  it("lang + dir, lang first", () => {
    const out = generateHtmlDocument({ dir: "ltr", lang: "en-US", head: "", body: "" });
    expect(out).toContain(`<html lang="en-US" dir="ltr">`);
  });
  it("bad lang throws", () => {
    expect(() => generateHtmlDocument({ lang: "123", head: "", body: "" }))
      .toThrow(/BCP-47/);
  });
  it("bad dir throws", () => {
    expect(() => generateHtmlDocument({ dir: "sideways" as unknown as never, head: "", body: "" }))
      .toThrow(/one of/);
  });
});

describe("generateHtmlDocument — htmlAttrs", () => {
  it("data-* attribute", () => {
    const out = generateHtmlDocument({
      head: "", body: "",
      htmlAttrs: { "data-theme": "dark" },
    });
    expect(out).toContain(`<html data-theme="dark">`);
  });
  it("multiple attrs in insertion order", () => {
    const out = generateHtmlDocument({
      head: "", body: "",
      htmlAttrs: { "data-theme": "dark", "data-page": "home" },
    });
    expect(out).toContain(`<html data-theme="dark" data-page="home">`);
  });
  it("attrs after lang + dir", () => {
    const out = generateHtmlDocument({
      lang: "en", dir: "ltr",
      head: "", body: "",
      htmlAttrs: { "data-theme": "dark" },
    });
    expect(out).toContain(`<html lang="en" dir="ltr" data-theme="dark">`);
  });
  it("rejects 'lang' key (reserved)", () => {
    expect(() => generateHtmlDocument({
      head: "", body: "",
      htmlAttrs: { lang: "x" },
    })).toThrow(/reserved/);
  });
  it("rejects 'dir' key (reserved)", () => {
    expect(() => generateHtmlDocument({
      head: "", body: "",
      htmlAttrs: { dir: "ltr" },
    })).toThrow(/reserved/);
  });
  it("rejects 'xmlns' key (reserved)", () => {
    expect(() => generateHtmlDocument({
      head: "", body: "",
      htmlAttrs: { xmlns: "x" },
    })).toThrow(/reserved/);
  });
  it("rejects 'onload' (event handler)", () => {
    expect(() => generateHtmlDocument({
      head: "", body: "",
      htmlAttrs: { onload: "alert(1)" },
    })).toThrow(/event-handler/);
  });
  it("rejects 'onmouseover' (event handler)", () => {
    expect(() => generateHtmlDocument({
      head: "", body: "",
      htmlAttrs: { onmouseover: "alert(1)" },
    })).toThrow(/event-handler/);
  });
  it("rejects attr-injection-attempt as key", () => {
    expect(() => generateHtmlDocument({
      head: "", body: "",
      htmlAttrs: { 'x" onclick="alert(1)': "y" },
    })).toThrow(/lowercase ASCII/);
  });
  it("rejects array as attrs", () => {
    expect(() => generateHtmlDocument({
      head: "", body: "",
      htmlAttrs: [] as unknown as never,
    })).toThrow(/htmlAttrs must be an object/);
  });
  it("rejects null as attrs", () => {
    expect(() => generateHtmlDocument({
      head: "", body: "",
      htmlAttrs: null as unknown as never,
    })).toThrow(/htmlAttrs must be an object/);
  });
  it("rejects string as attrs (error reports type)", () => {
    expect(() => generateHtmlDocument({
      head: "", body: "",
      htmlAttrs: "nope" as unknown as never,
    })).toThrow(/htmlAttrs must be an object; got string/);
  });
  it("rejects number as attrs", () => {
    expect(() => generateHtmlDocument({
      head: "", body: "",
      htmlAttrs: 42 as unknown as never,
    })).toThrow(/htmlAttrs must be an object; got number/);
  });
  it("rejects non-string value", () => {
    expect(() => generateHtmlDocument({
      head: "", body: "",
      htmlAttrs: { "data-x": 42 as unknown as string },
    })).toThrow(/attribute value must be a string/);
  });
  it("rejects NUL in value", () => {
    expect(() => generateHtmlDocument({
      head: "", body: "",
      htmlAttrs: { "data-x": "a\x00b" },
    })).toThrow(/control bytes/);
  });
  it("HTML-escapes attribute value", () => {
    const out = generateHtmlDocument({
      head: "", body: "",
      htmlAttrs: { "data-x": `she said "hi" & <bye>` },
    });
    expect(out).toContain(`data-x="she said &quot;hi&quot; &amp; &lt;bye&gt;"`);
  });
});

describe("generateHtmlDocument — bodyAttrs", () => {
  it("class attr on body", () => {
    const out = generateHtmlDocument({
      head: "", body: "",
      bodyAttrs: { class: "page-home" },
    });
    expect(out).toContain(`<body class="page-home">`);
  });
  it("multiple body attrs", () => {
    const out = generateHtmlDocument({
      head: "", body: "",
      bodyAttrs: { class: "p", id: "main" },
    });
    expect(out).toContain(`<body class="p" id="main">`);
  });
  it("rejects onload on body", () => {
    expect(() => generateHtmlDocument({
      head: "", body: "",
      bodyAttrs: { onload: "alert(1)" },
    })).toThrow(/event-handler/);
  });
  it("rejects onunload on body", () => {
    expect(() => generateHtmlDocument({
      head: "", body: "",
      bodyAttrs: { onunload: "x" },
    })).toThrow(/event-handler/);
  });
  it("HTML-escapes bodyAttrs value", () => {
    const out = generateHtmlDocument({
      head: "", body: "",
      bodyAttrs: { class: `"><script>alert(1)</script>` },
    });
    expect(out).toContain(`class="&quot;&gt;&lt;script&gt;alert(1)&lt;/script&gt;"`);
    expect(out).not.toContain("<script>alert");
  });
});

describe("generateHtmlDocument — head/body passthrough (NOT escaped)", () => {
  it("head with full HTML structure passes through", () => {
    const head = `<meta charset="utf-8">\n<title>Test</title>\n<link rel="stylesheet" href="/x.css">`;
    const out = generateHtmlDocument({ head, body: "" });
    expect(out).toContain(head);
  });
  it("body with raw HTML passes through", () => {
    const body = `<div class="container">\n<p>Hello & welcome</p>\n</div>`;
    const out = generateHtmlDocument({ head: "", body });
    expect(out).toContain(body);
  });
  it("multi-line head", () => {
    const head = "<title>A</title>\n<title>B</title>";
    const out = generateHtmlDocument({ head, body: "" });
    expect(out.split("\n").filter((l) => l.includes("title"))).toHaveLength(2);
  });
});

describe("generateHtmlDocument — purity / determinism", () => {
  it("same input → byte-identical output", () => {
    const cfg = {
      lang: "en", dir: "ltr" as const,
      head: "<title>X</title>", body: "<p>x</p>",
      htmlAttrs: { "data-theme": "dark" },
      bodyAttrs: { class: "p" },
    };
    expect(generateHtmlDocument(cfg)).toBe(generateHtmlDocument(cfg));
  });
  it("does not mutate input", () => {
    const cfg = {
      lang: "en",
      head: "<title>X</title>",
      body: "<p>x</p>",
      htmlAttrs: { "data-theme": "dark" },
    };
    const before = JSON.stringify(cfg);
    generateHtmlDocument(cfg);
    expect(JSON.stringify(cfg)).toBe(before);
  });
});

describe("generateHtmlDocument — fail-fast", () => {
  it("bad attr key throws before doc is built", () => {
    expect(() => generateHtmlDocument({
      head: "X", body: "Y",
      htmlAttrs: { onload: "z" },
    })).toThrow(/event-handler/);
  });
  it("bad lang throws before attrs validated", () => {
    expect(() => generateHtmlDocument({
      lang: "bad lang",
      head: "", body: "",
      htmlAttrs: { onload: "z" }, // would also fail, but lang is checked first
    })).toThrow(/BCP-47/);
  });
});

describe("generateHtmlDocument — full real-world example", () => {
  it("complete page", () => {
    const out = generateHtmlDocument({
      lang: "en-US",
      dir: "ltr",
      head: [
        `<meta charset="utf-8">`,
        `<title>Hello</title>`,
        `<link rel="stylesheet" href="/main.css">`,
      ].join("\n"),
      body: [
        `<header><h1>Hello</h1></header>`,
        `<main><p>World</p></main>`,
      ].join("\n"),
      htmlAttrs: { "data-theme": "dark" },
      bodyAttrs: { class: "page", id: "home" },
    });
    expect(out).toBe([
      "<!doctype html>",
      `<html lang="en-US" dir="ltr" data-theme="dark">`,
      "<head>",
      `<meta charset="utf-8">`,
      `<title>Hello</title>`,
      `<link rel="stylesheet" href="/main.css">`,
      "</head>",
      `<body class="page" id="home">`,
      `<header><h1>Hello</h1></header>`,
      `<main><p>World</p></main>`,
      "</body>",
      "</html>",
    ].join("\n"));
  });
});
