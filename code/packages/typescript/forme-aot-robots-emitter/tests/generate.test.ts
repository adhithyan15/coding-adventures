/**
 * generate.test.ts — end-to-end generateRobots.
 */

import { describe, it, expect } from "vitest";
import { generateRobots } from "../src/index.js";

describe("generateRobots — minimal", () => {
  it("empty config → empty string", () => {
    expect(generateRobots({ rules: [] })).toBe("");
  });

  it("single allow-all rule", () => {
    const txt = generateRobots({
      rules: [{ userAgent: "*", allow: ["/"] }],
    });
    expect(txt).toContain("User-agent: *");
    expect(txt).toContain("Allow: /");
    expect(txt.endsWith("\n")).toBe(true);
  });

  it("single disallow rule", () => {
    const txt = generateRobots({
      rules: [{ userAgent: "*", disallow: ["/admin"] }],
    });
    expect(txt).toContain("User-agent: *");
    expect(txt).toContain("Disallow: /admin");
  });
});

describe("generateRobots — multiple rule blocks", () => {
  it("two blocks separated by blank line", () => {
    const txt = generateRobots({
      rules: [
        { userAgent: "*", disallow: ["/admin"] },
        { userAgent: "Googlebot", allow: ["/"] },
      ],
    });
    expect(txt).toContain("User-agent: *\nDisallow: /admin\n\nUser-agent: Googlebot\nAllow: /");
  });

  it("preserves caller's rule order", () => {
    const txt = generateRobots({
      rules: [
        { userAgent: "C" },
        { userAgent: "A" },
        { userAgent: "B" },
      ],
    });
    const cIdx = txt.indexOf("User-agent: C");
    const aIdx = txt.indexOf("User-agent: A");
    const bIdx = txt.indexOf("User-agent: B");
    expect(cIdx).toBeLessThan(aIdx);
    expect(aIdx).toBeLessThan(bIdx);
  });
});

describe("generateRobots — userAgent array", () => {
  it("array of UAs emits one User-agent line per element", () => {
    const txt = generateRobots({
      rules: [{ userAgent: ["Googlebot", "Bingbot"], disallow: ["/private"] }],
    });
    expect(txt).toContain("User-agent: Googlebot\nUser-agent: Bingbot\nDisallow: /private");
  });

  it("empty userAgent array throws", () => {
    expect(() => generateRobots({
      rules: [{ userAgent: [], disallow: ["/x"] }],
    })).toThrow(/at least one value/);
  });

  it("userAgent not string or array throws", () => {
    expect(() => generateRobots({
      // @ts-expect-error
      rules: [{ userAgent: 42, disallow: ["/x"] }],
    })).toThrow(/string or string\[\]/);
  });
});

describe("generateRobots — allow + disallow ordering", () => {
  it("Allow lines emitted before Disallow lines", () => {
    const txt = generateRobots({
      rules: [{
        userAgent: "*",
        allow: ["/public"],
        disallow: ["/private"],
      }],
    });
    const allowIdx = txt.indexOf("Allow: /public");
    const disallowIdx = txt.indexOf("Disallow: /private");
    expect(allowIdx).toBeLessThan(disallowIdx);
  });

  it("multiple paths each get their own line", () => {
    const txt = generateRobots({
      rules: [{
        userAgent: "*",
        disallow: ["/a", "/b", "/c"],
      }],
    });
    expect(txt).toContain("Disallow: /a\nDisallow: /b\nDisallow: /c");
  });
});

describe("generateRobots — crawlDelay", () => {
  it("emitted after disallow lines", () => {
    const txt = generateRobots({
      rules: [{
        userAgent: "Slurp",
        disallow: ["/heavy"],
        crawlDelay: 10,
      }],
    });
    expect(txt).toContain("Disallow: /heavy\nCrawl-delay: 10");
  });

  it("zero crawlDelay permitted", () => {
    const txt = generateRobots({
      rules: [{ userAgent: "*", crawlDelay: 0 }],
    });
    expect(txt).toContain("Crawl-delay: 0");
  });

  it("undefined crawlDelay omits line", () => {
    const txt = generateRobots({
      rules: [{ userAgent: "*" }],
    });
    expect(txt).not.toContain("Crawl-delay");
  });

  it("negative crawlDelay throws", () => {
    expect(() => generateRobots({
      rules: [{ userAgent: "*", crawlDelay: -1 }],
    })).toThrow(/non-negative/);
  });

  it("fractional crawlDelay throws", () => {
    expect(() => generateRobots({
      rules: [{ userAgent: "*", crawlDelay: 1.5 }],
    })).toThrow(/integer/);
  });

  it("NaN crawlDelay throws", () => {
    expect(() => generateRobots({
      rules: [{ userAgent: "*", crawlDelay: NaN }],
    })).toThrow(/finite/);
  });
});

describe("generateRobots — sitemap", () => {
  it("single sitemap URL", () => {
    const txt = generateRobots({
      rules: [{ userAgent: "*", allow: ["/"] }],
      sitemap: "https://example.com/sitemap.xml",
    });
    expect(txt).toContain("Sitemap: https://example.com/sitemap.xml");
  });

  it("array of sitemaps", () => {
    const txt = generateRobots({
      rules: [{ userAgent: "*" }],
      sitemap: [
        "https://example.com/sitemap-1.xml",
        "https://example.com/sitemap-2.xml",
      ],
    });
    expect(txt).toContain("Sitemap: https://example.com/sitemap-1.xml");
    expect(txt).toContain("Sitemap: https://example.com/sitemap-2.xml");
  });

  it("sitemap appears AFTER rule blocks", () => {
    const txt = generateRobots({
      rules: [{ userAgent: "*", disallow: ["/admin"] }],
      sitemap: "https://example.com/sitemap.xml",
    });
    const ruleIdx = txt.indexOf("Disallow: /admin");
    const smIdx = txt.indexOf("Sitemap:");
    expect(ruleIdx).toBeLessThan(smIdx);
  });

  it("rejects javascript: sitemap", () => {
    expect(() => generateRobots({
      rules: [{ userAgent: "*" }],
      sitemap: "javascript:alert(1)",
    })).toThrow(/http\(s\)/);
  });

  it("rejects root-relative sitemap (absolute required)", () => {
    expect(() => generateRobots({
      rules: [{ userAgent: "*" }],
      sitemap: "/sitemap.xml",
    })).toThrow(/http\(s\)/);
  });

  it("sitemap with newline rejected", () => {
    expect(() => generateRobots({
      rules: [{ userAgent: "*" }],
      sitemap: "https://example.com/x\nDisallow: /evil",
    })).toThrow(/forbidden control character/);
  });

  it("non-string sitemap throws", () => {
    expect(() => generateRobots({
      rules: [{ userAgent: "*" }],
      // @ts-expect-error
      sitemap: 42,
    })).toThrow(/string, string\[\], or undefined/);
  });
});

describe("generateRobots — host", () => {
  it("host emitted at end", () => {
    const txt = generateRobots({
      rules: [{ userAgent: "*" }],
      host: "example.com",
    });
    expect(txt).toContain("Host: example.com");
  });

  it("host appears AFTER sitemap when both supplied", () => {
    const txt = generateRobots({
      rules: [{ userAgent: "*" }],
      sitemap: "https://example.com/sitemap.xml",
      host: "example.com",
    });
    const smIdx = txt.indexOf("Sitemap:");
    const hostIdx = txt.indexOf("Host:");
    expect(smIdx).toBeLessThan(hostIdx);
  });

  it("rejects host containing scheme", () => {
    expect(() => generateRobots({
      rules: [{ userAgent: "*" }],
      host: "https://example.com",
    })).toThrow(/not a URL/);
  });

  it("rejects host with path", () => {
    expect(() => generateRobots({
      rules: [{ userAgent: "*" }],
      host: "example.com/path",
    })).toThrow(/no path/);
  });
});

describe("generateRobots — header-injection defence", () => {
  it("userAgent with newline rejected", () => {
    expect(() => generateRobots({
      rules: [{ userAgent: "Goodbot\nUser-agent: Evilbot", disallow: ["/"] }],
    })).toThrow(/forbidden control character/);
  });

  it("disallow path with CR rejected", () => {
    expect(() => generateRobots({
      rules: [{ userAgent: "*", disallow: ["/admin\rAllow: /admin"] }],
    })).toThrow(/forbidden control character/);
  });

  it("allow path with CRLF rejected", () => {
    expect(() => generateRobots({
      rules: [{ userAgent: "*", allow: ["/good\r\nDisallow: /good"] }],
    })).toThrow(/forbidden control character/);
  });

  it("userAgent with NUL rejected", () => {
    expect(() => generateRobots({
      rules: [{ userAgent: "Bot\x00name" }],
    })).toThrow(/forbidden control character/);
  });

  it("error message identifies the bad field", () => {
    try {
      generateRobots({
        rules: [
          { userAgent: "*" },
          { userAgent: "*", disallow: ["/a", "/b\nbad"] },
        ],
      });
      expect.fail("expected throw");
    } catch (e) {
      expect((e as Error).message).toContain("rules[1].disallow[1]");
    }
  });
});

describe("generateRobots — fail-fast (no partial output)", () => {
  it("bad rule in mid-array throws without partial emission", () => {
    expect(() => generateRobots({
      rules: [
        { userAgent: "*", allow: ["/"] },
        { userAgent: "Bot\nInjection" },
        { userAgent: "Googlebot" },
      ],
    })).toThrow(/forbidden control character/);
  });

  it("bad sitemap throws even when rules are valid", () => {
    expect(() => generateRobots({
      rules: [{ userAgent: "*" }],
      sitemap: "javascript:bad",
    })).toThrow(/http\(s\)/);
  });
});

describe("generateRobots — input shape validation", () => {
  it("null rule throws", () => {
    expect(() => generateRobots({
      // @ts-expect-error
      rules: [null],
    })).toThrow(/non-null object/);
  });

  it("non-array allow throws", () => {
    expect(() => generateRobots({
      // @ts-expect-error
      rules: [{ userAgent: "*", allow: "not an array" }],
    })).toThrow(/must be an array/);
  });

  it("non-array disallow throws", () => {
    expect(() => generateRobots({
      // @ts-expect-error
      rules: [{ userAgent: "*", disallow: "not an array" }],
    })).toThrow(/must be an array/);
  });
});

describe("generateRobots — purity / determinism", () => {
  it("does not mutate input rules array", () => {
    const config = {
      rules: [{ userAgent: "*", disallow: ["/admin"] }],
    };
    const before = JSON.stringify(config);
    generateRobots(config);
    expect(JSON.stringify(config)).toBe(before);
  });

  it("same input → byte-identical output", () => {
    const config = {
      rules: [
        { userAgent: "*", allow: ["/"], disallow: ["/admin"], crawlDelay: 5 },
        { userAgent: "Googlebot", allow: ["/"] },
      ],
      sitemap: "https://example.com/sitemap.xml",
      host: "example.com",
    };
    expect(generateRobots(config)).toBe(generateRobots(config));
  });

  it("output ends with single newline", () => {
    const txt = generateRobots({
      rules: [{ userAgent: "*" }],
    });
    expect(txt.endsWith("\n")).toBe(true);
    expect(txt.endsWith("\n\n")).toBe(false);
  });
});

describe("generateRobots — full real-world example", () => {
  it("blog with admin block + googlebot exception + sitemap", () => {
    const txt = generateRobots({
      rules: [
        { userAgent: "*", disallow: ["/admin", "/private"], crawlDelay: 10 },
        { userAgent: "Googlebot", allow: ["/"] },
        { userAgent: ["Bingbot", "Slurp"], crawlDelay: 5 },
      ],
      sitemap: [
        "https://example.com/sitemap.xml",
        "https://example.com/sitemap-news.xml",
      ],
      host: "example.com",
    });
    expect(txt).toBe([
      "User-agent: *",
      "Disallow: /admin",
      "Disallow: /private",
      "Crawl-delay: 10",
      "",
      "User-agent: Googlebot",
      "Allow: /",
      "",
      "User-agent: Bingbot",
      "User-agent: Slurp",
      "Crawl-delay: 5",
      "",
      "Sitemap: https://example.com/sitemap.xml",
      "Sitemap: https://example.com/sitemap-news.xml",
      "Host: example.com",
    ].join("\n") + "\n");
  });
});
