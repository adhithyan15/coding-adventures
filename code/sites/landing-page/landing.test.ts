import { readFile } from "node:fs/promises";
import { describe, expect, it } from "vitest";
import { parseLandingModel } from "./model.ts";
import { renderDocument } from "./render-landing.ts";

const fixture = JSON.parse(await readFile(new URL("./data/index.landing", import.meta.url), "utf8"));

describe("landing model", () => {
  it("validates the complete declarative source", () => {
    const model = parseLandingModel(fixture);
    expect(model.paths).toHaveLength(3);
    expect(model.labs).toHaveLength(16);
    expect(model.forme.pipeline).toHaveLength(6);
    expect(model.site.ogImage).toBe("assets/og.jpg");
  });

  it("rejects unsafe links and asset paths", () => {
    expect(() => parseLandingModel({
      ...fixture,
      site: { ...fixture.site, ogImage: "../og.jpg" },
    })).toThrow(/portable relative path/);
    expect(() => parseLandingModel({
      ...fixture,
      labs: [{ ...fixture.labs[0], href: "javascript:alert(1)" }],
    })).toThrow(/root-relative path/);
    expect(() => parseLandingModel({
      ...fixture,
      labs: [{ ...fixture.labs[0], href: "//example.com/escape" }],
    })).toThrow(/root-relative path/);
  });
});

describe("landing renderer", () => {
  it("produces the approved semantic surface without client JavaScript", () => {
    const model = parseLandingModel(fixture);
    const output = renderDocument(model, "forme-asset:preview", "body { color: #15231f; }");
    expect(output).toContain('<meta name="generator" content="Forme">');
    expect(output).toContain('class="lab-card featured"');
    expect(output).toContain('id="forme"');
    expect(output).toContain("forme-asset:preview");
    expect(output).not.toContain("<script");
    expect(output.match(/class="lab-card/g)).toHaveLength(16);
  });
});
