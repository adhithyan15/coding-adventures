/**
 * opengraph.test.ts — OpenGraph generator.
 */

import { describe, it, expect } from "vitest";
import { generateOpenGraphTags, type OpenGraphMeta } from "../src/index.js";

const MIN: OpenGraphMeta = {
  title: "Hello",
  type: "article",
  image: "https://example.com/og.png",
  url: "https://example.com/hello",
};

describe("generateOpenGraphTags — required fields", () => {
  it("emits the four mandatory tags in conventional order", () => {
    const out = generateOpenGraphTags(MIN);
    const lines = out.split("\n");
    expect(lines[0]).toBe(`<meta property="og:title" content="Hello">`);
    expect(lines[1]).toBe(`<meta property="og:type" content="article">`);
    expect(lines[2]).toBe(`<meta property="og:image" content="https://example.com/og.png">`);
    expect(lines[3]).toBe(`<meta property="og:url" content="https://example.com/hello">`);
    expect(lines.length).toBe(4);
  });
});

describe("generateOpenGraphTags — optional fields", () => {
  it("includes description when supplied", () => {
    expect(generateOpenGraphTags({ ...MIN, description: "A nice page" }))
      .toContain(`<meta property="og:description" content="A nice page">`);
  });

  it("includes site_name (note: kebab→snake) when supplied", () => {
    expect(generateOpenGraphTags({ ...MIN, siteName: "Acme Blog" }))
      .toContain(`<meta property="og:site_name" content="Acme Blog">`);
  });

  it("includes locale when supplied", () => {
    expect(generateOpenGraphTags({ ...MIN, locale: "en_US" }))
      .toContain(`<meta property="og:locale" content="en_US">`);
  });

  it("includes video when supplied (URL-validated)", () => {
    expect(generateOpenGraphTags({ ...MIN, video: "https://example.com/v.mp4" }))
      .toContain(`<meta property="og:video" content="https://example.com/v.mp4">`);
  });

  it("optional tags appear in spec-conventional order after required ones", () => {
    const out = generateOpenGraphTags({
      ...MIN,
      description: "d",
      siteName: "s",
      locale: "l",
      video: "https://example.com/v.mp4",
    });
    const lines = out.split("\n");
    // 4 required + 4 optional
    expect(lines.length).toBe(8);
    expect(lines[4]).toContain("og:description");
    expect(lines[5]).toContain("og:site_name");
    expect(lines[6]).toContain("og:locale");
    expect(lines[7]).toContain("og:video");
  });
});

describe("generateOpenGraphTags — URL validation", () => {
  it("throws on relative og:image", () => {
    expect(() => generateOpenGraphTags({ ...MIN, image: "/img.png" }))
      .toThrow(/og:image.*absolute/);
  });

  it("throws on relative og:url", () => {
    expect(() => generateOpenGraphTags({ ...MIN, url: "/page" }))
      .toThrow(/og:url.*absolute/);
  });

  it("throws on relative og:video", () => {
    expect(() => generateOpenGraphTags({ ...MIN, video: "/v.mp4" }))
      .toThrow(/og:video.*absolute/);
  });

  it("throws on javascript: og:image", () => {
    expect(() => generateOpenGraphTags({ ...MIN, image: "javascript:alert(1)" }))
      .toThrow(/og:image.*absolute/);
  });

  it("throws on data: og:image", () => {
    expect(() => generateOpenGraphTags({ ...MIN, image: "data:image/png;base64,xx" }))
      .toThrow(/og:image.*absolute/);
  });

  it("validates URLs BEFORE emitting any output", () => {
    // If validation happened after emission, this would write partial
    // output.  We assert that the throw is the only observable.
    try {
      generateOpenGraphTags({ ...MIN, image: "/relative" });
      expect.fail("expected throw");
    } catch (e) {
      expect((e as Error).message).toMatch(/og:image/);
    }
  });
});

describe("generateOpenGraphTags — HTML escaping", () => {
  it("escapes special chars in title", () => {
    expect(generateOpenGraphTags({ ...MIN, title: `AT&T "Blog"` }))
      .toContain(`content="AT&amp;T &quot;Blog&quot;"`);
  });

  it("escapes special chars in description", () => {
    expect(generateOpenGraphTags({ ...MIN, description: `<script>x</script>` }))
      .toContain(`content="&lt;script&gt;x&lt;/script&gt;"`);
  });

  it("strips ASCII control bytes from title", () => {
    expect(generateOpenGraphTags({ ...MIN, title: "Hello\x00World" }))
      .toContain(`content="HelloWorld"`);
  });
});

describe("generateOpenGraphTags — reproducibility", () => {
  it("same input → byte-identical output", () => {
    expect(generateOpenGraphTags(MIN)).toBe(generateOpenGraphTags(MIN));
  });
});
