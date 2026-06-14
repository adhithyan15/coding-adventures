/**
 * filename.test.ts — filename-hint extraction tests.
 */

import { describe, it, expect } from "vitest";
import { extractFilenameHint } from "../src/index.js";

describe("extractFilenameHint — six comment styles", () => {
  it("// (line comment)", () => {
    const r = extractFilenameHint("// file: src/auth.ts\nexport function login() {}\n");
    expect(r.filename).toBe("src/auth.ts");
    expect(r.strippedValue).toBe("export function login() {}\n");
  });
  it("# (hash comment)", () => {
    const r = extractFilenameHint("# file: app.py\ndef main(): pass\n");
    expect(r.filename).toBe("app.py");
    expect(r.strippedValue).toBe("def main(): pass\n");
  });
  it("-- (SQL/Haskell/Lua)", () => {
    const r = extractFilenameHint("-- file: schema.sql\nCREATE TABLE foo (id INT);\n");
    expect(r.filename).toBe("schema.sql");
    expect(r.strippedValue).toBe("CREATE TABLE foo (id INT);\n");
  });
  it("% (LaTeX)", () => {
    const r = extractFilenameHint("% file: paper.tex\n\\documentclass{article}\n");
    expect(r.filename).toBe("paper.tex");
    expect(r.strippedValue).toBe("\\documentclass{article}\n");
  });
  it("<!-- … --> (HTML)", () => {
    const r = extractFilenameHint("<!-- file: index.html -->\n<html><body>hi</body></html>\n");
    expect(r.filename).toBe("index.html");
    expect(r.strippedValue).toBe("<html><body>hi</body></html>\n");
  });
  it("/* … */ (C block)", () => {
    const r = extractFilenameHint("/* file: theme.css */\nbody { color: red; }\n");
    expect(r.filename).toBe("theme.css");
    expect(r.strippedValue).toBe("body { color: red; }\n");
  });
});

describe("extractFilenameHint — variations", () => {
  it("case-insensitive keyword: File:", () => {
    const r = extractFilenameHint("// File: foo.ts\nbar\n");
    expect(r.filename).toBe("foo.ts");
  });
  it("case-insensitive keyword: FILE:", () => {
    const r = extractFilenameHint("// FILE: foo.ts\nbar\n");
    expect(r.filename).toBe("foo.ts");
  });
  it("extra whitespace around keyword and colon", () => {
    const r = extractFilenameHint("//   file   :   foo.ts\nbar\n");
    expect(r.filename).toBe("foo.ts");
  });
  it("leading whitespace before comment marker", () => {
    const r = extractFilenameHint("    // file: foo.ts\nbar\n");
    expect(r.filename).toBe("foo.ts");
  });
  it("trailing content after filename (line comment) is discarded", () => {
    const r = extractFilenameHint("// file: foo.ts — auth module\nbar\n");
    expect(r.filename).toBe("foo.ts");
    expect(r.strippedValue).toBe("bar\n");
  });
  it("absolute paths are preserved verbatim", () => {
    const r = extractFilenameHint("# file: /etc/nginx/sites-enabled/default\nfoo\n");
    expect(r.filename).toBe("/etc/nginx/sites-enabled/default");
  });
  it("paths with dots and dashes are preserved", () => {
    const r = extractFilenameHint("// file: ../../../node_modules/.bin/vitest.js\nbar\n");
    expect(r.filename).toBe("../../../node_modules/.bin/vitest.js");
  });
  it("CRLF line endings (defensive — parser normalises but we still handle)", () => {
    const r = extractFilenameHint("// file: foo.ts\r\nbar\r\n");
    expect(r.filename).toBe("foo.ts");
    expect(r.strippedValue).toBe("bar\r\n");
  });
});

describe("extractFilenameHint — no hint present", () => {
  it("plain code returns null filename, value untouched", () => {
    const r = extractFilenameHint("export function f() {}\n");
    expect(r.filename).toBeNull();
    expect(r.strippedValue).toBe("export function f() {}\n");
  });
  it("comment on line 1 but not a filename hint", () => {
    const r = extractFilenameHint("// just a comment\nfoo()\n");
    expect(r.filename).toBeNull();
    expect(r.strippedValue).toBe("// just a comment\nfoo()\n");
  });
  it("filename hint on line 2 is NOT extracted (rule: line 1 only)", () => {
    const r = extractFilenameHint("foo()\n// file: foo.ts\nbar()\n");
    expect(r.filename).toBeNull();
    expect(r.strippedValue).toBe("foo()\n// file: foo.ts\nbar()\n");
  });
  it("empty value", () => {
    const r = extractFilenameHint("");
    expect(r.filename).toBeNull();
    expect(r.strippedValue).toBe("");
  });
});

describe("extractFilenameHint — edge cases", () => {
  it("single-line code block that IS a filename hint strips to empty", () => {
    const r = extractFilenameHint("// file: foo.ts");
    expect(r.filename).toBe("foo.ts");
    expect(r.strippedValue).toBe("");
  });
  it("malformed HTML comment (no closing) is NOT matched as HTML; falls to line-comment regex which also fails (no //, /*, #, --, %)", () => {
    const r = extractFilenameHint("<!-- file: foo.ts\ncode\n");
    expect(r.filename).toBeNull();
  });
  it("hint with extra trailing junk after */ does NOT match C-block style", () => {
    const r = extractFilenameHint("/* file: foo.css */ garbage\nrest\n");
    expect(r.filename).toBeNull();
  });
});
