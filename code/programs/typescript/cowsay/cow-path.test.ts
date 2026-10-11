/**
 * Tests for cow-file selection — the path-traversal fix for issue #12169.
 *
 * Each test builds a throwaway tree like this:
 *
 *   <tmp>/
 *     secret.cow          ← must NEVER be readable via -f
 *     cows/
 *       default.cow       ← the fallback
 *       tux.cow           ← a legitimate cow
 *       nested/inner.cow  ← exists, but "nested/inner" is not a bare name
 *
 * and asserts that every hostile spelling of "secret" draws the default cow.
 */

import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import * as url from "url";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { isSafeCowName, resolveCowPath } from "./cow-path.js";

const here = path.dirname(url.fileURLToPath(import.meta.url));

let base: string;
let cows: string;

function writeCow(file: string, body: string): void {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, `$the_cow = <<EOC;\n${body}\nEOC\n`);
}

function drawn(cowName: unknown, dir: string = cows): string {
  return fs.readFileSync(resolveCowPath(cowName, dir), "utf-8");
}

beforeEach(() => {
  base = fs.mkdtempSync(path.join(os.tmpdir(), "cowsay-traversal-"));
  cows = path.join(base, "cows");
  writeCow(path.join(cows, "default.cow"), "DEFAULT");
  writeCow(path.join(cows, "tux.cow"), "TUX");
  writeCow(path.join(cows, "nested", "inner.cow"), "NESTED");
  writeCow(path.join(base, "secret.cow"), "SECRET");
});

afterEach(() => {
  fs.rmSync(base, { recursive: true, force: true });
});

describe("isSafeCowName", () => {
  it.each(["default", "tux", "bud-frogs", "three_eyes", "v2", "dragon.and.cow"])(
    "accepts the bare name %j",
    (name) => {
      expect(isSafeCowName(name)).toBe(true);
    },
  );

  it.each([
    "",
    "..",
    "../secret",
    "..\\secret",
    "a/b",
    "a\\b",
    "/etc/passwd",
    "C:secret",
    "C:\\Windows\\win",
    "tux\0",
    "..%2Fsecret",
    "%2e%2e/secret",
    undefined,
    42,
  ])("rejects %j", (name) => {
    expect(isSafeCowName(name)).toBe(false);
  });
});

describe("resolveCowPath", () => {
  it("still loads normal cow names", () => {
    expect(drawn("tux")).toContain("TUX");
    expect(drawn("default")).toContain("DEFAULT");
  });

  it("falls back to default for an unknown cow", () => {
    expect(drawn("does-not-exist")).toContain("DEFAULT");
  });

  it.each(["../secret", "..\\secret", "./../secret", "tux/../../secret"])(
    "refuses relative traversal %j",
    (hostile) => {
      // Sanity: the target really is reachable by naive joining.
      expect(fs.existsSync(path.join(cows, "..", "secret.cow"))).toBe(true);
      expect(drawn(hostile)).toContain("DEFAULT");
    },
  );

  it("refuses an absolute path", () => {
    expect(drawn(path.join(base, "secret"))).toContain("DEFAULT");
  });

  it.each(["nested/inner", "nested\\inner"])(
    "refuses the nested name %j even inside the cows dir",
    (nested) => {
      expect(drawn(nested)).toContain("DEFAULT");
    },
  );

  it.each(["..%2Fsecret", "%2e%2e%2fsecret", "%2E%2E%5Csecret", "tux\0../secret"])(
    "refuses encoded / NUL name %j",
    (hostile) => {
      expect(drawn(hostile)).toContain("DEFAULT");
    },
  );

  // Layer 2 in action: "evil" is syntactically fine, but the file it names is
  // a symlink leading out of the cows directory.
  it.skipIf(process.platform === "win32")(
    "refuses a symlink that escapes the cows dir",
    () => {
      fs.symlinkSync(path.join(base, "secret.cow"), path.join(cows, "evil.cow"));
      expect(drawn("evil")).toContain("DEFAULT");
    },
  );

  it("loads the repository's real cows", () => {
    const repoCows = path.join(here, "..", "..", "..", "specs", "cows");
    expect(drawn("tux", repoCows)).not.toEqual(drawn("default", repoCows));
  });
});
