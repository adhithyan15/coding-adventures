/**
 * path.test.ts — route → output-path derivation.
 */

import { describe, it, expect } from "vitest";
import { routeToOutputPath } from "../src/index.js";

describe("routeToOutputPath", () => {
  it("/ → index.html", () => { expect(routeToOutputPath("/")).toBe("index.html"); });
  it("/about → about/index.html", () => {
    expect(routeToOutputPath("/about")).toBe("about/index.html");
  });
  it("/posts/x → posts/x/index.html", () => {
    expect(routeToOutputPath("/posts/x")).toBe("posts/x/index.html");
  });
  it("/page.html → page.html", () => {
    expect(routeToOutputPath("/page.html")).toBe("page.html");
  });
  it("/feed.xml → feed.xml", () => {
    expect(routeToOutputPath("/feed.xml")).toBe("feed.xml");
  });
  it("/posts/x.html → posts/x.html", () => {
    expect(routeToOutputPath("/posts/x.html")).toBe("posts/x.html");
  });
  it("/p/x.json → p/x.json", () => {
    expect(routeToOutputPath("/p/x.json")).toBe("p/x.json");
  });
  it("deep path with no extension", () => {
    expect(routeToOutputPath("/a/b/c/d")).toBe("a/b/c/d/index.html");
  });
  it(".hidden does NOT count as extension (dot at index 0)", () => {
    // ".hidden" → no extension (dot at first char) → treated as directory
    expect(routeToOutputPath("/.hidden")).toBe(".hidden/index.html");
  });
  it("compound .a.b.c uses first dot after index 0", () => {
    // /file.tar.gz → file.tar.gz (last segment has `.` at index 4)
    expect(routeToOutputPath("/file.tar.gz")).toBe("file.tar.gz");
  });
});
