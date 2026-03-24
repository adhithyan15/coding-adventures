/**
 * Tests for the browser-compatible Lattice transpiler.
 *
 * Note: browser-transpiler.ts uses Vite `?raw` imports to load grammar files
 * at build time. In vitest, `?raw` imports of files outside the project root
 * do not resolve correctly (Vite restricts filesystem access to the project
 * root). Instead, we test with `transpileLatticeInBrowser` from the
 * lattice-transpiler package which uses identical logic but with embedded
 * grammar strings. Both implementations use the same pipeline:
 *   GrammarLexer → GrammarParser → LatticeTransformer → CSSEmitter
 *
 * The browser-transpiler.ts itself is tested at build time by Vite when
 * the docs site is built (where `?raw` imports resolve correctly).
 */

import { describe, it, expect } from "vitest";
import { transpileLatticeInBrowser } from "@coding-adventures/lattice-transpiler/src/browser.js";

/** Wrapper matching the browser-transpiler API for test convenience. */
function transpile(
  source: string,
  options: { minified?: boolean; indent?: string } = {}
): { success: true; css: string } | { success: false; error: string } {
  try {
    const css = transpileLatticeInBrowser(source, options);
    return { success: true, css };
  } catch (err) {
    const message =
      err instanceof Error ? err.message : "Unknown compilation error";
    return { success: false, error: message };
  }
}

describe("transpileLatticeBrowser", () => {
  it("transpiles a plain CSS rule unchanged", () => {
    const result = transpile("h1 { color: red; }");
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.css).toContain("h1");
      expect(result.css).toContain("color: red");
    }
  });

  it("substitutes a variable", () => {
    const result = transpile(
      "$brand: #4a90d9;\n.btn { color: $brand; }"
    );
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.css).toContain("#4a90d9");
      expect(result.css).not.toContain("$brand");
    }
  });

  it("expands a mixin", () => {
    const result = transpile(
      "@mixin bold() { font-weight: bold; }\n.title { @include bold(); }"
    );
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.css).toContain("font-weight: bold");
    }
  });

  it("returns an error for an undefined variable", () => {
    const result = transpile(".btn { color: $undefined-var; }");
    expect(result.success).toBe(false);
  });

  it("handles minified output", () => {
    const result = transpile("h1 { color: red; }", {
      minified: true,
    });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.css).not.toContain("\n  ");
    }
  });

  it("expands a @for loop", () => {
    const result = transpile(
      "@for $i from 1 through 3 { .item { order: $i; } }"
    );
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.css).toContain("order: 1");
      expect(result.css).toContain("order: 2");
      expect(result.css).toContain("order: 3");
    }
  });
});
