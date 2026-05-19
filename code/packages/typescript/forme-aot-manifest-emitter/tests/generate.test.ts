/**
 * generate.test.ts — end-to-end generateManifest.
 */

import { describe, it, expect } from "vitest";
import { generateManifest } from "../src/index.js";

describe("generateManifest — minimal config", () => {
  it("empty config → empty JSON object", () => {
    expect(generateManifest({})).toBe("{}");
  });

  it("only name → single-field manifest", () => {
    const json = generateManifest({ name: "My App" });
    expect(JSON.parse(json)).toEqual({ name: "My App" });
  });
});

describe("generateManifest — plain string fields", () => {
  it("includes name, short_name, description", () => {
    const json = generateManifest({
      name: "App",
      short_name: "A",
      description: "An app",
    });
    const obj = JSON.parse(json);
    expect(obj.name).toBe("App");
    expect(obj.short_name).toBe("A");
    expect(obj.description).toBe("An app");
  });

  it("includes lang and dir", () => {
    const json = generateManifest({ lang: "en-US", dir: "ltr" });
    expect(JSON.parse(json)).toEqual({ dir: "ltr", lang: "en-US" });
  });

  it("includes orientation", () => {
    const json = generateManifest({ orientation: "portrait" });
    expect(JSON.parse(json).orientation).toBe("portrait");
  });

  it("non-string name throws", () => {
    expect(() => generateManifest({ name: 42 as unknown as string }))
      .toThrow(/name must be a string/);
  });
});

describe("generateManifest — URL fields", () => {
  it("start_url accepted (root-relative)", () => {
    const json = generateManifest({ start_url: "/" });
    expect(JSON.parse(json).start_url).toBe("/");
  });

  it("start_url accepted (absolute)", () => {
    const json = generateManifest({ start_url: "https://example.com/" });
    expect(JSON.parse(json).start_url).toBe("https://example.com/");
  });

  it("scope accepted", () => {
    const json = generateManifest({ scope: "/app" });
    expect(JSON.parse(json).scope).toBe("/app");
  });

  it("start_url javascript: rejected", () => {
    expect(() => generateManifest({ start_url: "javascript:alert(1)" }))
      .toThrow(/http\(s\)/);
  });

  it("scope data: rejected", () => {
    expect(() => generateManifest({ scope: "data:text/html,x" }))
      .toThrow(/http\(s\)/);
  });

  it("start_url empty string rejected", () => {
    expect(() => generateManifest({ start_url: "" })).toThrow(/non-empty/);
  });
});

describe("generateManifest — display allowlist", () => {
  it("'standalone' accepted", () => {
    const json = generateManifest({ display: "standalone" });
    expect(JSON.parse(json).display).toBe("standalone");
  });

  it("'minimal-ui' accepted", () => {
    const json = generateManifest({ display: "minimal-ui" });
    expect(JSON.parse(json).display).toBe("minimal-ui");
  });

  it("'Standalone' (case-insensitive) accepted, lowercased", () => {
    const json = generateManifest({ display: "Standalone" });
    expect(JSON.parse(json).display).toBe("standalone");
  });

  it("'tab' rejected", () => {
    expect(() => generateManifest({ display: "tab" })).toThrow(/one of/);
  });
});

describe("generateManifest — colour fields", () => {
  it("theme_color hex accepted", () => {
    const json = generateManifest({ theme_color: "#0066cc" });
    expect(JSON.parse(json).theme_color).toBe("#0066cc");
  });

  it("background_color hex with alpha accepted", () => {
    const json = generateManifest({ background_color: "#ffffff80" });
    expect(JSON.parse(json).background_color).toBe("#ffffff80");
  });

  it("theme_color 'red' (CSS name) rejected", () => {
    expect(() => generateManifest({ theme_color: "red" })).toThrow(/hex colour/);
  });

  it("background_color rgba() rejected", () => {
    expect(() => generateManifest({ background_color: "rgba(0,0,0,0.5)" }))
      .toThrow(/hex colour/);
  });
});

describe("generateManifest — icons array", () => {
  it("single icon", () => {
    const json = generateManifest({
      icons: [{ src: "/icon-192.png", sizes: "192x192", type: "image/png" }],
    });
    const obj = JSON.parse(json);
    expect(obj.icons).toEqual([
      { src: "/icon-192.png", sizes: "192x192", type: "image/png" },
    ]);
  });

  it("multiple icons", () => {
    const json = generateManifest({
      icons: [
        { src: "/icon-192.png", sizes: "192x192", type: "image/png" },
        { src: "/icon-512.png", sizes: "512x512", type: "image/png" },
      ],
    });
    expect(JSON.parse(json).icons.length).toBe(2);
  });

  it("icon with all fields including purpose", () => {
    const json = generateManifest({
      icons: [{
        src: "/icon.png",
        sizes: "192x192 512x512",
        type: "image/png",
        purpose: "maskable",
      }],
    });
    expect(JSON.parse(json).icons[0]).toEqual({
      src: "/icon.png",
      purpose: "maskable",
      sizes: "192x192 512x512",
      type: "image/png",
    });
  });

  it("icon array preserves caller order", () => {
    const json = generateManifest({
      icons: [
        { src: "/a.png" },
        { src: "/b.png" },
        { src: "/c.png" },
      ],
    });
    const obj = JSON.parse(json);
    expect(obj.icons.map((i: { src: string }) => i.src)).toEqual(["/a.png", "/b.png", "/c.png"]);
  });

  it("icons not an array throws", () => {
    expect(() => generateManifest({
      icons: "not an array" as unknown as never,
    })).toThrow(/icons must be an array/);
  });

  it("null icon throws", () => {
    expect(() => generateManifest({
      icons: [null as unknown as never],
    })).toThrow(/non-null object/);
  });

  it("icon with javascript: src throws", () => {
    expect(() => generateManifest({
      icons: [{ src: "javascript:alert(1)" }],
    })).toThrow(/http\(s\)/);
  });

  it("error message identifies bad icon index", () => {
    try {
      generateManifest({
        icons: [
          { src: "/good.png" },
          { src: "javascript:bad" },
        ],
      });
      expect.fail("expected throw");
    } catch (e) {
      expect((e as Error).message).toContain("icons[1].src");
    }
  });
});

describe("generateManifest — fail-fast (no partial output)", () => {
  it("bad display rejected before icons validated", () => {
    expect(() => generateManifest({
      display: "tab",
      icons: [{ src: "/icon.png" }],
    })).toThrow(/one of/);
  });

  it("bad theme_color rejected", () => {
    expect(() => generateManifest({
      name: "App",
      theme_color: "red",
    })).toThrow(/hex colour/);
  });
});

describe("generateManifest — deterministic key ordering", () => {
  it("top-level keys alphabetically sorted", () => {
    const json = generateManifest({
      icons: [{ src: "/x.png" }],
      name: "App",
      theme_color: "#000000",
      background_color: "#ffffff",
      start_url: "/",
    });
    // Parse and re-stringify to get key order from JSON.parse
    // wouldn't help since both preserve insertion order; instead
    // check the raw output substring positions.
    const bgIdx = json.indexOf(`"background_color"`);
    const iconsIdx = json.indexOf(`"icons"`);
    const nameIdx = json.indexOf(`"name"`);
    const startIdx = json.indexOf(`"start_url"`);
    const themeIdx = json.indexOf(`"theme_color"`);
    // Alphabetical: background_color < icons < name < start_url < theme_color
    expect(bgIdx).toBeLessThan(iconsIdx);
    expect(iconsIdx).toBeLessThan(nameIdx);
    expect(nameIdx).toBeLessThan(startIdx);
    expect(startIdx).toBeLessThan(themeIdx);
  });

  it("same input → byte-identical output", () => {
    const config = {
      name: "App",
      start_url: "/",
      display: "standalone" as const,
      theme_color: "#0066cc",
      icons: [{ src: "/icon-192.png", sizes: "192x192", type: "image/png" }],
    };
    expect(generateManifest(config)).toBe(generateManifest(config));
  });

  it("input key order doesn't affect output", () => {
    const json1 = generateManifest({ name: "A", display: "browser" });
    const json2 = generateManifest({ display: "browser", name: "A" });
    expect(json1).toBe(json2);
  });

  it("icon field order normalised (src first then alphabetical)", () => {
    const json = generateManifest({
      icons: [{ purpose: "maskable", sizes: "192x192", type: "image/png", src: "/icon.png" }],
    });
    // src should come before purpose / sizes / type
    const srcIdx = json.indexOf(`"src"`);
    const purposeIdx = json.indexOf(`"purpose"`);
    const sizesIdx = json.indexOf(`"sizes"`);
    const typeIdx = json.indexOf(`"type"`);
    expect(srcIdx).toBeLessThan(purposeIdx);
    expect(purposeIdx).toBeLessThan(sizesIdx);
    expect(sizesIdx).toBeLessThan(typeIdx);
  });
});

describe("generateManifest — pretty-print", () => {
  it("output uses 2-space indent", () => {
    const json = generateManifest({ name: "App" });
    expect(json).toContain('{\n  "name": "App"\n}');
  });
});

describe("generateManifest — purity", () => {
  it("does not mutate input config", () => {
    const config = {
      name: "App",
      icons: [{ src: "/icon.png", sizes: "192x192" }],
    };
    const before = JSON.stringify(config);
    generateManifest(config);
    expect(JSON.stringify(config)).toBe(before);
  });
});

describe("generateManifest — full real-world example", () => {
  it("matches expected PWA manifest", () => {
    const json = generateManifest({
      name: "Awesome PWA",
      short_name: "PWA",
      description: "An awesome progressive web app",
      start_url: "/",
      scope: "/",
      display: "standalone",
      lang: "en-US",
      dir: "ltr",
      orientation: "portrait",
      theme_color: "#0066cc",
      background_color: "#ffffff",
      icons: [
        { src: "/icon-192.png", sizes: "192x192", type: "image/png" },
        { src: "/icon-512.png", sizes: "512x512", type: "image/png", purpose: "maskable" },
      ],
    });
    const obj = JSON.parse(json);
    expect(obj.name).toBe("Awesome PWA");
    expect(obj.display).toBe("standalone");
    expect(obj.theme_color).toBe("#0066cc");
    expect(obj.icons.length).toBe(2);
    expect(obj.icons[1].purpose).toBe("maskable");
  });
});
