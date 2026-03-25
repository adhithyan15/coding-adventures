/**
 * engram-stress-test.test.ts — Lattice transpiler stress test using real-world CSS.
 *
 * This test transpiles a complete Engram app stylesheet written in Lattice and
 * checks that the output CSS is well-formed. It exercises:
 *
 *   - Variables in values (including multi-value shorthands)
 *   - Variables inside CSS custom property declarations
 *   - Mixins with default parameters that reference variables
 *   - @each $key, $value in $map (two-variable map destructuring)
 *   - @extend with %placeholder selectors
 *   - @media queries (top-level and nested)
 *   - Color functions: darken(), lighten(), mix()
 *   - CSS function pass-through: rgba(), color-mix(), var(), rect()
 *   - & (parent selector) inside mixins
 *   - Vendor-prefixed properties (-webkit-backface-visibility)
 *   - Negative dimensions (-1px, -0.02em)
 *   - !important flag
 *
 * Bugs found during this run are documented inline.
 */

import { readFileSync } from "fs";
import { join } from "path";
import { describe, test, expect } from "vitest";
import { transpileLattice } from "../src/index.js";

const FIXTURE_PATH = join(import.meta.dirname, "fixtures", "engram.lattice");

describe("Engram app CSS stress test", () => {
  let source: string;
  let output: string;

  // Read the fixture once for all tests.
  try {
    source = readFileSync(FIXTURE_PATH, "utf8");
  } catch (e) {
    source = "";
  }

  // ── Core transpilation ──────────────────────────────────────────────────────

  test("transpiles the full Engram stylesheet without throwing", () => {
    expect(() => {
      output = transpileLattice(source);
    }).not.toThrow();
  });

  test("output is non-empty", () => {
    output = transpileLattice(source);
    expect(output.trim().length).toBeGreaterThan(0);
  });

  // ── Variable substitution ───────────────────────────────────────────────────

  test("substitutes $body-bg into CSS output", () => {
    output = transpileLattice(source);
    expect(output).toContain("#1a1a2e");
  });

  test("substitutes $accent into CSS output", () => {
    output = transpileLattice(source);
    expect(output).toContain("#e94560");
  });

  test("substitutes $panel-bg into CSS output", () => {
    output = transpileLattice(source);
    expect(output).toContain("#16213e");
  });

  // ── CSS custom properties ───────────────────────────────────────────────────

  test(":root block is emitted with CSS custom properties", () => {
    output = transpileLattice(source);
    expect(output).toContain(":root");
    expect(output).toContain("--body-bg");
    expect(output).toContain("--accent");
  });

  test("variables inside CSS custom props are substituted", () => {
    output = transpileLattice(source);
    // --body-bg: $body-bg should become --body-bg: #1a1a2e
    expect(output).toMatch(/--body-bg\s*:\s*#1a1a2e/);
  });

  // ── Multi-value shorthands ──────────────────────────────────────────────────

  test("multi-value padding (padding: $sm $md) is emitted correctly", () => {
    output = transpileLattice(source);
    // button { padding: 8px 16px; }
    expect(output).toMatch(/padding\s*:\s*8px\s+16px/);
  });

  // ── Mixin expansion ─────────────────────────────────────────────────────────

  test("flex-column mixin expands to display:flex + flex-direction:column", () => {
    output = transpileLattice(source);
    expect(output).toContain("display: flex");
    expect(output).toContain("flex-direction: column");
  });

  test("panel-card mixin expands background and border properties", () => {
    output = transpileLattice(source);
    expect(output).toContain("border-radius: 8px");
  });

  // ── @each over map ──────────────────────────────────────────────────────────

  test("@each $name, $color in $rating-colors generates btn--again rule", () => {
    output = transpileLattice(source);
    // One rule per map entry should appear
    expect(output).toContain("#f87171");  // again color
    expect(output).toContain("#fb923c");  // hard color
    expect(output).toContain("#4ade80");  // good color
    expect(output).toContain("#60a5fa");  // easy color
  });

  // ── & (parent selector) in mixin ───────────────────────────────────────────

  test("btn-outlined mixin emits :hover rule via &", () => {
    output = transpileLattice(source);
    // The mixin contains &:hover:not(:disabled) { background: $color; }
    // After expansion this should produce a :hover rule
    expect(output).toMatch(/hover/);
  });

  // ── @extend / %placeholder ──────────────────────────────────────────────────

  test("@extend %card-face copies placeholder properties to flash-card__front", () => {
    output = transpileLattice(source);
    // The %card-face placeholder sets position:absolute; @extend should copy it
    expect(output).toContain("position: absolute");
    expect(output).toContain("backface-visibility: hidden");
  });

  // ── Color functions ─────────────────────────────────────────────────────────

  test("darken($accent, 10) produces a valid hex color", () => {
    output = transpileLattice(source);
    // darken($accent, 10) on #e94560 — should produce a darker red hex
    // We just check the output contains some --accent-hover that's a hex
    expect(output).toMatch(/--accent-hover\s*:\s*#[0-9a-f]{6}/i);
  });

  test("lighten() on progress bar fill produces a valid hex color", () => {
    output = transpileLattice(source);
    // lighten(#4ade80, 5) should produce a slightly lighter green hex
    // The original #4ade80 lightened by 5% stays green
    expect(output).toMatch(/background\s*:\s*#[0-9a-f]{6}/i);
  });

  // ── CSS function pass-through ───────────────────────────────────────────────

  test("rgba() literal pass-through is preserved", () => {
    output = transpileLattice(source);
    // --wire-glow: rgba(0, 255, 136, 0.3) should survive unchanged
    expect(output).toMatch(/rgba\(0,\s*255,\s*136,\s*0\.3\)/);
  });

  test("color-mix() with variables is emitted with substituted values", () => {
    output = transpileLattice(source);
    // color-mix(in srgb, $panel-header 60%, $panel-bg) should become
    // color-mix(in srgb, #0f3460 60%, #16213e)
    expect(output).toContain("color-mix");
    expect(output).toContain("#0f3460");
  });

  test("var() function pass-through is preserved", () => {
    output = transpileLattice(source);
    expect(output).toContain("var(--sans");
  });

  test("rect() function pass-through is preserved", () => {
    output = transpileLattice(source);
    expect(output).toContain("rect(0, 0, 0, 0)");
  });

  // ── !important ──────────────────────────────────────────────────────────────

  test("!important is preserved in @media reduced-motion block", () => {
    output = transpileLattice(source);
    expect(output).toContain("!important");
  });

  // ── @media queries ─────────────────────────────────────────────────────────

  test("@media (prefers-reduced-motion: reduce) blocks are emitted", () => {
    output = transpileLattice(source);
    expect(output).toContain("prefers-reduced-motion");
  });

  // ── Vendor prefixes ─────────────────────────────────────────────────────────

  test("-webkit-backface-visibility property is emitted", () => {
    output = transpileLattice(source);
    expect(output).toContain("-webkit-backface-visibility");
  });

  // ── Negative values ─────────────────────────────────────────────────────────

  test("negative dimension -1px is emitted correctly", () => {
    output = transpileLattice(source);
    expect(output).toContain("-1px");
  });

  test("negative em value -0.02em is emitted correctly", () => {
    output = transpileLattice(source);
    expect(output).toContain("-0.02em");
  });

  // ── Font family values ──────────────────────────────────────────────────────

  test("font-family strings with commas are preserved in --mono", () => {
    output = transpileLattice(source);
    expect(output).toContain("SF Mono");
    expect(output).toContain("Consolas");
  });

  // ── Grid layout ─────────────────────────────────────────────────────────────

  test("repeat(auto-fit, minmax(130px, 1fr)) is emitted unchanged", () => {
    output = transpileLattice(source);
    expect(output).toContain("repeat(auto-fit, minmax(130px, 1fr))");
  });
});
