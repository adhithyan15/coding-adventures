import { describe, it, expect } from "vitest";
import { transpileLatticeBrowser } from "./browser-transpiler";

describe("transpileLatticeBrowser", () => {
  it("transpiles a plain CSS rule unchanged", () => {
    const result = transpileLatticeBrowser("h1 { color: red; }");
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.css).toContain("h1");
      expect(result.css).toContain("color: red");
    }
  });

  it("substitutes a variable", () => {
    const result = transpileLatticeBrowser(
      "$brand: #4a90d9;\n.btn { color: $brand; }"
    );
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.css).toContain("#4a90d9");
      expect(result.css).not.toContain("$brand");
    }
  });

  it("expands a mixin", () => {
    const result = transpileLatticeBrowser(
      "@mixin bold { font-weight: bold; }\n.title { @include bold; }"
    );
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.css).toContain("font-weight: bold");
    }
  });

  it("returns an error for an undefined variable", () => {
    const result = transpileLatticeBrowser(".btn { color: $undefined-var; }");
    expect(result.success).toBe(false);
  });

  it("handles minified output", () => {
    const result = transpileLatticeBrowser("h1 { color: red; }", {
      minified: true,
    });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.css).not.toContain("\n  ");
    }
  });

  it("expands a @for loop", () => {
    const result = transpileLatticeBrowser(
      "@for $i from 1 through 3 { .col-$i { flex: $i; } }"
    );
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.css).toContain(".col-1");
      expect(result.css).toContain(".col-2");
      expect(result.css).toContain(".col-3");
    }
  });
});
