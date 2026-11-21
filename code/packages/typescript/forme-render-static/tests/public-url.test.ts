import { describe, expect, it } from "vitest";
import { publicUrl } from "../src/index.js";

describe("publicUrl", () => {
  it("strips trailing slashes and normalizes route leading slash", () => {
    expect(publicUrl("https://example.com///", "/posts/hello")).toBe(
      "https://example.com/posts/hello",
    );
    expect(publicUrl("https://example.com", "posts/hello")).toBe(
      "https://example.com/posts/hello",
    );
    expect(publicUrl("///", "/")).toBe("/");
  });

  it("handles many slashes in linear time (ReDoS regression)", () => {
    const start = performance.now();
    const evilPrefix = "https://example.com/" + "/".repeat(50_000) + "a";
    expect(publicUrl(evilPrefix, "/x")).toBe(`${evilPrefix}/x`);
    expect(publicUrl("https://example.com" + "/".repeat(50_000), "x")).toBe(
      "https://example.com/x",
    );
    expect(performance.now() - start).toBeLessThan(1000);
  });
});
