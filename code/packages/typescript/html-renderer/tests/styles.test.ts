/**
 * Tests for the CSS styles module.
 * ==================================
 *
 * The styles are embedded as a string in the HTML output. These
 * tests verify that the CSS string contains the essential rules
 * and doesn't have obvious issues.
 */

import { describe, it, expect } from "vitest";
import { getStyles } from "../src/styles.js";

describe("getStyles", () => {
  it("should return a non-empty CSS string", () => {
    const css = getStyles();
    expect(css.length).toBeGreaterThan(0);
  });

  it("should include body styling", () => {
    const css = getStyles();
    expect(css).toContain("body");
    expect(css).toContain("background-color");
  });

  it("should include dark theme colors", () => {
    const css = getStyles();
    expect(css).toContain("#1e1e2e"); // base background
    expect(css).toContain("#cdd6f4"); // text color
    expect(css).toContain("#313244"); // surface color
  });

  it("should include token badge styles", () => {
    const css = getStyles();
    expect(css).toContain(".token-list");
    expect(css).toContain(".token-name");
    expect(css).toContain(".token-number");
    expect(css).toContain(".token-operator");
    expect(css).toContain(".token-keyword");
    expect(css).toContain(".token-string");
    expect(css).toContain(".token-default");
  });

  it("should include table styles", () => {
    const css = getStyles();
    expect(css).toContain("table");
    expect(css).toContain("th");
    expect(css).toContain("td");
  });

  it("should include stack visualization styles", () => {
    const css = getStyles();
    expect(css).toContain(".stack");
    expect(css).toContain(".stack-item");
  });

  it("should include encoding/bit-field styles", () => {
    const css = getStyles();
    expect(css).toContain(".encoding");
    expect(css).toContain(".bit-field");
  });

  it("should include gate styles", () => {
    const css = getStyles();
    expect(css).toContain(".gate-group");
    expect(css).toContain(".gate-name");
    expect(css).toContain(".gate-row");
  });

  it("should include responsive media query", () => {
    const css = getStyles();
    expect(css).toContain("@media");
    expect(css).toContain("768px");
  });

  it("should include monospace font declarations", () => {
    const css = getStyles();
    expect(css).toContain("monospace");
  });
});
