import { afterAll, describe, expect, it } from "vitest";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { execFileSync } from "node:child_process";

import { getChangedFiles, mapFilesToPackages } from "../src/gitdiff.js";

// Helper: create a tiny git repo on disk with a couple of commits so that
// `git diff` actually has something to talk about.  We return the repo
// root path; the caller is responsible for cleaning it up.
function makeRepo(): string {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "gitdiff-test-"));
  execFileSync("git", ["init", "-q", "-b", "main"], { cwd: root });
  execFileSync("git", ["config", "user.email", "a@b"], { cwd: root });
  execFileSync("git", ["config", "user.name", "x"], { cwd: root });
  execFileSync("git", ["config", "core.autocrlf", "false"], { cwd: root });
  fs.writeFileSync(path.join(root, "base.txt"), "base\n");
  execFileSync("git", ["add", "base.txt"], { cwd: root });
  execFileSync("git", ["commit", "-q", "-m", "base"], { cwd: root });
  // Tag the base commit so we have a stable ref to diff against.
  execFileSync("git", ["tag", "base"], { cwd: root });
  fs.writeFileSync(path.join(root, "changed.txt"), "changed\n");
  fs.mkdirSync(path.join(root, "code/packages/python/foo/src"), { recursive: true });
  fs.writeFileSync(path.join(root, "code/packages/python/foo/src/gates.py"), "x = 1\n");
  fs.writeFileSync(path.join(root, "code/packages/python/foo/README.md"), "# foo\n");
  execFileSync("git", ["add", "-A"], { cwd: root });
  execFileSync("git", ["commit", "-q", "-m", "changes"], { cwd: root });
  return root;
}

describe("getChangedFiles", () => {
  it("lists files changed between base and HEAD via three-dot diff", () => {
    const root = makeRepo();
    try {
      const files = getChangedFiles(root, "base");
      expect(files).toContain("changed.txt");
      expect(files).toContain("code/packages/python/foo/src/gates.py");
      expect(files).toContain("code/packages/python/foo/README.md");
    } finally {
      fs.rmSync(root, { recursive: true, force: true });
    }
  });

  it("returns [] when the diff base does not exist (both attempts fail)", () => {
    const root = makeRepo();
    try {
      expect(getChangedFiles(root, "definitely-not-a-ref-here")).toEqual([]);
    } finally {
      fs.rmSync(root, { recursive: true, force: true });
    }
  });

  it("returns [] when the repo root is not a git repo", () => {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "gitdiff-test-"));
    try {
      expect(getChangedFiles(tmp, "origin/main")).toEqual([]);
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });
});

describe("mapFilesToPackages", () => {
  // Common setup used by the variants below.
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), "gitdiff-map-test-"));
  const pkgPaths = new Map([
    ["python/foo", path.join(repoRoot, "code", "packages", "python", "foo")],
    ["python/bar", path.join(repoRoot, "code", "packages", "python", "bar")],
  ]);

  afterAll(() => {
    fs.rmSync(repoRoot, { recursive: true, force: true });
  });

  it("maps a file under a package directory to that package", () => {
    const out = mapFilesToPackages(
      ["code/packages/python/foo/src/gates.py"],
      pkgPaths,
      repoRoot,
    );
    expect(out).toEqual(new Set(["python/foo"]));
  });

  it("ignores files that aren't under any package", () => {
    expect(mapFilesToPackages(["README.md"], pkgPaths, repoRoot).size).toBe(0);
  });

  it("handles a file equal to the package path (no trailing slash)", () => {
    // Equality-only branch on line 155.
    const out = mapFilesToPackages(
      ["code/packages/python/foo"],
      pkgPaths,
      repoRoot,
    );
    expect(out).toEqual(new Set(["python/foo"]));
  });

  it("applies declared-srcs filter and rejects non-matching files", () => {
    const declared = new Map([["python/foo", ["src/**/*.py"]]]);
    const out = mapFilesToPackages(
      ["code/packages/python/foo/README.md"],
      pkgPaths,
      repoRoot,
      declared,
    );
    expect(out.size).toBe(0); // README doesn't match `src/**/*.py`.
  });

  it("accepts matching files when declared srcs are present", () => {
    const declared = new Map([["python/foo", ["src/**/*.py"]]]);
    const out = mapFilesToPackages(
      ["code/packages/python/foo/src/gates.py"],
      pkgPaths,
      repoRoot,
      declared,
    );
    expect(out).toEqual(new Set(["python/foo"]));
  });

  it("treats BUILD files as always-counted regardless of declared srcs", () => {
    const declared = new Map([["python/foo", ["src/**/*.py"]]]);
    for (const buildName of ["BUILD", "BUILD_mac", "BUILD_linux", "BUILD_windows", "BUILD_mac_and_linux"]) {
      const out = mapFilesToPackages(
        [`code/packages/python/foo/${buildName}`],
        pkgPaths,
        repoRoot,
        declared,
      );
      expect(out).toEqual(new Set(["python/foo"]));
    }
  });

  it("respects empty declaredSrcs (falls through to default behaviour)", () => {
    // patterns?.length === 0 → skips the strict check, defaults to under-path inclusion.
    const declared = new Map([["python/foo", [] as string[]]]);
    const out = mapFilesToPackages(
      ["code/packages/python/foo/anything.txt"],
      pkgPaths,
      repoRoot,
      declared,
    );
    expect(out).toEqual(new Set(["python/foo"]));
  });
});
