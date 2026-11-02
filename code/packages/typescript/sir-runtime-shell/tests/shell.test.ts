import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { backtick } from "../src/index.js";

/**
 * Commands are built with `node -e "..."` (via `process.execPath`, the running
 * Node binary) rather than the shell built-in `echo` so they behave identically
 * on Windows `cmd.exe` and POSIX `/bin/sh` — `echo`'s quoting and flag handling
 * differ across those shells, whereas a Node one-liner is deterministic.
 */
function node(code: string): string {
  // Quote the interpreter path (it may contain spaces) and wrap the code in
  // double quotes; the code itself uses only single quotes.
  return `"${process.execPath}" -e "${code}"`;
}

describe("backtick — Ruby `cmd` semantics", () => {
  it("returns the command's captured stdout", () => {
    const out = backtick(node("process.stdout.write('123')"));
    assert.match(out, /123/);
  });

  it("returns exactly what was written to stdout", () => {
    const out = backtick(node("process.stdout.write('exact')"));
    assert.equal(out, "exact");
  });

  it("returns a string", () => {
    const out = backtick(node("process.stdout.write('hi')"));
    assert.equal(typeof out, "string");
  });

  it("preserves multi-line output in order", () => {
    const out = backtick(node("process.stdout.write('line1\\nline2')"));
    assert.match(out, /line1/);
    assert.match(out, /line2/);
    assert.ok(out.indexOf("line1") < out.indexOf("line2"));
  });

  it("returns stdout even when the command exits non-zero", () => {
    // execSync throws on non-zero exit; backtick must recover the stdout, like
    // Ruby returning stdout regardless of $?.
    const out = backtick(
      node("process.stdout.write('partial'); process.exit(3)"),
    );
    assert.equal(out, "partial");
  });

  it("returns empty string for a non-zero exit with no stdout", () => {
    const out = backtick(node("process.exit(5)"));
    assert.equal(out, "");
  });

  it("does not include stderr in the result", () => {
    const out = backtick(
      node("process.stderr.write('ERR'); process.stdout.write('OUT')"),
    );
    assert.equal(out, "OUT");
    assert.doesNotMatch(out, /ERR/);
  });

  it("returns empty string when the command cannot be spawned at all", () => {
    // A command the shell cannot run produces a thrown error whose `stdout` may
    // be null/undefined (no child stdout was ever captured). The `?? ""`
    // fallback must turn that into the empty string, never throwing.
    const out = backtick(
      "this-command-definitely-does-not-exist-xyzzy-42 --nope",
    );
    assert.equal(out, "");
  });
});
