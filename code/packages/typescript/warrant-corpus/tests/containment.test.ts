/**
 * Containment checks, exercised against both path flavours on any host.
 *
 * `path.win32` lets Linux CI reproduce the Windows failure: with the repository
 * on `D:` and the corpus on `C:`, `relative` returns an ABSOLUTE path.
 */
import { posix, win32 } from "node:path";
import { describe, expect, it } from "vitest";
import { asciiFold, refusedComponent, relativeInside } from "../src/containment.ts";

describe("relativeInside", () => {
  it("shows the hazard: win32 relative across drives is absolute", () => {
    expect(win32.relative("D:\\repo", "C:\\tmp\\x")).toBe("C:\\tmp\\x");
  });

  it("returns null across Windows drives", () => {
    expect(relativeInside("D:\\repo", "C:\\tmp\\x", win32)).toBeNull();
  });

  it("accepts a win32 child and reports it relative", () => {
    expect(relativeInside("D:\\repo", "D:\\repo\\specs\\x", win32)).toBe("specs\\x");
    expect(relativeInside("D:\\repo", "D:\\repo", win32)).toBe("");
  });

  it("rejects win32 parents and siblings", () => {
    expect(relativeInside("D:\\repo", "D:\\other", win32)).toBeNull();
    expect(relativeInside("D:\\repo\\a", "D:\\repo", win32)).toBeNull();
  });

  it("handles posix paths the same way", () => {
    expect(relativeInside("/repo", "/repo/a/b", posix)).toBe("a/b");
    expect(relativeInside("/repo", "/repo", posix)).toBe("");
    expect(relativeInside("/repo", "/elsewhere", posix)).toBeNull();
    expect(relativeInside("/repo/a", "/repo", posix)).toBeNull();
  });

  it("does not mistake a `..foo` sibling name for an escape", () => {
    expect(relativeInside("/repo", "/repo/..foo/x", posix)).toBe("..foo/x");
    expect(relativeInside("D:\\repo", "D:\\repo\\..foo", win32)).toBe("..foo");
  });

  it("defaults to the host path module", () => {
    expect(relativeInside("/", "/")).toBe("");
  });

  it("treats a device-namespace path as never inside, on either side (WD-100)", () => {
    expect(relativeInside("D:\\repo", "\\\\?\\D:\\repo\\x", win32)).toBeNull();
    expect(relativeInside("\\\\.\\D:\\repo", "D:\\repo\\x", win32)).toBeNull();
    expect(relativeInside("/r", "/r/\\\\?\\x", posix)).toBe("\\\\?\\x");
  });

  it("refuses exactly two leading slashes on posix and collapses three (review of WD-100)", () => {
    expect(relativeInside("/", "//x", posix)).toBeNull();
    expect(relativeInside("//", "/r", posix)).toBeNull();
    expect(relativeInside("/", "///x", posix)).toBe("x");
    expect(relativeInside("/r", "/r//x", posix)).toBe("x");
  });
});

describe("WD-100b: reserved names and ASCII-only folding", () => {
  it("refuses reserved device names and trailing dots or spaces", () => {
    for (const name of ["CON", "con", "CON.txt", "NUL.", "nul ", "COM1", "com9.log", "LPT1", "com1.tar.gz", "x."]) {
      expect(refusedComponent(name), name).toBe(true);
    }
    for (const name of ["console.txt", "COM10", "LPT0", "COM0", "CONx", "xCON", ".hidden", "a.CON.b", "\u017fon"]) {
      expect(refusedComponent(name), name).toBe(false);
    }
  });

  it("folds A..Z only", () => {
    expect(asciiFold("Repo-\u00c9")).toBe("repo-\u00c9");
    for (const text of ["\u017f", "\u212a", "\u0130", "\u03a3", "\u03c2"]) expect(asciiFold(text)).toBe(text);
  });

  it("applies both rules to a win32 query", () => {
    expect(relativeInside("D:\\repo", "d:\\REPO\\x", win32)).toBe("x");
    expect(relativeInside("D:\\repo", "D:\\repo\\console.txt", win32)).toBe("console.txt");
    for (const child of ["D:\\repo\\NUL", "D:\\repo\\con.txt", "D:\\repo\\AUX\\x", "D:\\repo\\a."]) {
      expect(relativeInside("D:\\repo", child, win32), child).toBeNull();
    }
    expect(relativeInside("D:\\\u212a", "D:\\k\\x", win32)).toBeNull();
    expect(relativeInside("/\u212a", "/k/x", posix)).toBeNull();
  });
});
