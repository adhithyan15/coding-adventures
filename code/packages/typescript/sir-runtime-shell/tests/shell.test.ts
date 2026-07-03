import { describe, it, expect } from "vitest";
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
    expect(out).toContain("123");
  });

  it("returns exactly what was written to stdout", () => {
    const out = backtick(node("process.stdout.write('exact')"));
    expect(out).toBe("exact");
  });

  it("returns a string", () => {
    const out = backtick(node("process.stdout.write('hi')"));
    expect(typeof out).toBe("string");
  });

  it("preserves multi-line output in order", () => {
    const out = backtick(node("process.stdout.write('line1\\nline2')"));
    expect(out).toContain("line1");
    expect(out).toContain("line2");
    expect(out.indexOf("line1")).toBeLessThan(out.indexOf("line2"));
  });

  it("returns stdout even when the command exits non-zero", () => {
    // execSync throws on non-zero exit; backtick must recover the stdout, like
    // Ruby returning stdout regardless of $?.
    const out = backtick(
      node("process.stdout.write('partial'); process.exit(3)"),
    );
    expect(out).toBe("partial");
  });

  it("returns empty string for a non-zero exit with no stdout", () => {
    const out = backtick(node("process.exit(5)"));
    expect(out).toBe("");
  });

  it("does not include stderr in the result", () => {
    const out = backtick(
      node("process.stderr.write('ERR'); process.stdout.write('OUT')"),
    );
    expect(out).toBe("OUT");
    expect(out).not.toContain("ERR");
  });

  it("returns empty string when the command cannot be spawned at all", () => {
    // A command the shell cannot run produces a thrown error whose `stdout` may
    // be null/undefined (no child stdout was ever captured). The `?? ""`
    // fallback must turn that into the empty string, never throwing.
    const out = backtick(
      "this-command-definitely-does-not-exist-xyzzy-42 --nope",
    );
    expect(out).toBe("");
  });
});
