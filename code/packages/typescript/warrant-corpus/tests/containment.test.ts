/**
 * Containment checks, exercised against both path flavours on any host.
 *
 * `path.win32` lets Linux CI reproduce the Windows failure: with the repository
 * on `D:` and the corpus on `C:`, `relative` returns an ABSOLUTE path.
 */
import { posix, win32 } from "node:path";
import { describe, expect, it } from "vitest";
import { relativeInside } from "../src/containment.ts";

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
});
