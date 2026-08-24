// The bug this file exists for.
//
// Six generator CLIs take a path out of a checked-in manifest, join it to the
// curriculum root, and write there. Each inferred containment from
// `path.relative(root, resolved)` — if the answer does not start with `../`,
// it must be inside.
//
// On win32 that inference is wrong. `path.relative()` cannot express a journey
// between two different roots as `..` steps, so it returns the target absolute
// and unchanged, and an upward-escape check waves it through. The traversal
// tests in place only covered `..`, POSIX-absolute, and SAME-drive absolutes —
// all three of which the old check did catch — which is exactly why the gap
// survived. The two shapes below are the ones it did not.
import { describe, it, expect } from "vitest";
import { resolve, relative as pathRelative, normalize } from "node:path";
import { assertRelativeManifestPath } from "../src/manifest-path.js";

const ROOT = resolve("C:/repo/curriculum");

/** The OLD containment rule, kept verbatim so the bypass stays demonstrated. */
function escapesUpward(relative: string): boolean {
  const output = resolve(ROOT, relative);
  const fromRoot = normalize(pathRelative(ROOT, output)).replaceAll("\\", "/");
  return fromRoot === "" || fromRoot === ".." || fromRoot.startsWith("../");
}

describe("assertRelativeManifestPath", () => {
  it("accepts the ordinary manifest values every CLI actually ships", () => {
    for (const ok of [
      "spanish/book/chapters/ch303-reparaciones.tex",
      "core/lesson-modality/spanish.json",
      "progress/spanish.md",
      "spanish/book/figures/fig-01.svg",
      "./spanish/book/chapters/ch01-first-words.tex",
    ]) {
      expect(() => assertRelativeManifestPath(ok, `unsafe '${ok}'`)).not.toThrow();
    }
  });

  it("rejects POSIX-absolute paths", () => {
    expect(() => assertRelativeManifestPath("/absolute/evil.tex", "unsafe")).toThrow(/unsafe/);
  });

  // Division of labour, stated so nobody "tightens" this into a second traversal
  // check and assumes the old one is now redundant. This guard answers one
  // question — is the value shaped like a relative path at all — and dotted
  // traversal stays with the existing containment check downstream, which
  // already handles it correctly on every platform.
  it("leaves dotted traversal to the containment check that already catches it", () => {
    expect(() => assertRelativeManifestPath("../../../evil.tex", "unsafe")).not.toThrow();
    expect(escapesUpward("../../../evil.tex")).toBe(true);
  });

  // The regression. Both of these produce a `fromRoot` that never starts with
  // `../`, so the old rule accepted them and the write landed outside the root:
  // on another local volume, or on an SMB share.
  it("rejects drive-qualified and UNC paths, which the old rule accepted", () => {
    for (const bypass of [
      "D:\\evil.tex",
      "D:/evil.tex",
      "\\\\server\\share\\evil.tex",
      "//server/share/evil.tex",
    ]) {
      // Demonstrate the old rule was fooled...
      expect(escapesUpward(bypass), `${bypass} did not escape upward`).toBe(false);
      // ...and that the new one is not.
      expect(() => assertRelativeManifestPath(bypass, `unsafe '${bypass}'`)).toThrow(/unsafe/);
    }
  });

  // The drive and UNC patterns are applied on EVERY platform, not just win32.
  // A POSIX CI box does not consider `D:\evil.tex` absolute, so a
  // platform-conditional rule would let a poisoned manifest through review on
  // Linux and only bite on a Windows machine later.
  it("applies the same rule regardless of host platform", () => {
    expect(() => assertRelativeManifestPath("D:\\evil.tex", "unsafe")).toThrow();
    expect(() => assertRelativeManifestPath("//server/share/x.json", "unsafe")).toThrow();
  });

  it("preserves the caller's message, so per-CLI error text is unchanged", () => {
    expect(() => assertRelativeManifestPath("D:\\x.tex", "unsafe generated book output 'D:\\x.tex'"))
      .toThrow("unsafe generated book output 'D:\\x.tex'");
  });
});
