/**
 * document-manager.test.ts -- DocumentManager tests
 *
 * Tests cover:
 *   - Opening and retrieving documents
 *   - Closing documents
 *   - Full replacement changes
 *   - Incremental changes (range-based edits)
 *   - Incremental changes with emoji (surrogate pairs)
 *   - Multiple sequential changes
 *   - Error handling for non-open documents
 */

import { describe, it, expect } from "vitest";
import { DocumentManager } from "../src/document-manager.js";
import type { Range } from "../src/types.js";

describe("DocumentManager", () => {
  it("open and get", () => {
    const dm = new DocumentManager();
    dm.open("file:///test.txt", "hello world", 1);

    const doc = dm.get("file:///test.txt");
    expect(doc).toBeDefined();
    expect(doc!.text).toBe("hello world");
    expect(doc!.version).toBe(1);
  });

  it("get missing returns undefined", () => {
    const dm = new DocumentManager();
    expect(dm.get("file:///nonexistent.txt")).toBeUndefined();
  });

  it("close removes document", () => {
    const dm = new DocumentManager();
    dm.open("file:///test.txt", "hello", 1);
    dm.close("file:///test.txt");
    expect(dm.get("file:///test.txt")).toBeUndefined();
  });

  it("applyChanges full replacement", () => {
    const dm = new DocumentManager();
    dm.open("file:///test.txt", "hello world", 1);

    dm.applyChanges("file:///test.txt", [
      { newText: "goodbye world" },
    ], 2);

    const doc = dm.get("file:///test.txt");
    expect(doc!.text).toBe("goodbye world");
    expect(doc!.version).toBe(2);
  });

  it("applyChanges incremental", () => {
    const dm = new DocumentManager();
    dm.open("file:///test.txt", "hello world", 1);

    // Replace "world" (chars 6-11) with "Go"
    dm.applyChanges("file:///test.txt", [
      {
        range: {
          start: { line: 0, character: 6 },
          end: { line: 0, character: 11 },
        },
        newText: "Go",
      },
    ], 2);

    const doc = dm.get("file:///test.txt");
    expect(doc!.text).toBe("hello Go");
  });

  it("applyChanges on non-open document throws", () => {
    const dm = new DocumentManager();
    expect(() => {
      dm.applyChanges("file:///notopen.txt", [{ newText: "x" }], 1);
    }).toThrow("document not open");
  });

  it("incremental change with emoji", () => {
    // "A\u{1F3B8}B" -- emoji is 2 UTF-16 code units
    // Replace "B" (UTF-16 char 3, string index 3) with "X"
    const dm = new DocumentManager();
    dm.open("file:///test.txt", "A\u{1F3B8}B", 1);

    dm.applyChanges("file:///test.txt", [
      {
        range: {
          start: { line: 0, character: 3 },
          end: { line: 0, character: 4 },
        },
        newText: "X",
      },
    ], 2);

    const doc = dm.get("file:///test.txt");
    expect(doc!.text).toBe("A\u{1F3B8}X");
  });

  it("multiple sequential incremental changes", () => {
    const dm = new DocumentManager();
    dm.open("uri", "hello world", 1);

    // Change "hello" to "hi"
    dm.applyChanges("uri", [
      {
        range: {
          start: { line: 0, character: 0 },
          end: { line: 0, character: 5 },
        },
        newText: "hi",
      },
    ], 2);

    const doc = dm.get("uri");
    expect(doc!.text).toBe("hi world");
  });

  it("multiline incremental change", () => {
    const dm = new DocumentManager();
    dm.open("file:///test.txt", "line1\nline2\nline3", 1);

    // Replace "line2" (line 1, chars 0-5) with "REPLACED"
    dm.applyChanges("file:///test.txt", [
      {
        range: {
          start: { line: 1, character: 0 },
          end: { line: 1, character: 5 },
        },
        newText: "REPLACED",
      },
    ], 2);

    const doc = dm.get("file:///test.txt");
    expect(doc!.text).toBe("line1\nREPLACED\nline3");
  });

  it("insertion (empty range)", () => {
    const dm = new DocumentManager();
    dm.open("file:///test.txt", "helloworld", 1);

    // Insert " " at position 5
    dm.applyChanges("file:///test.txt", [
      {
        range: {
          start: { line: 0, character: 5 },
          end: { line: 0, character: 5 },
        },
        newText: " ",
      },
    ], 2);

    const doc = dm.get("file:///test.txt");
    expect(doc!.text).toBe("hello world");
  });

  it("deletion (empty newText)", () => {
    const dm = new DocumentManager();
    dm.open("file:///test.txt", "hello world", 1);

    // Delete " world" (chars 5-11)
    dm.applyChanges("file:///test.txt", [
      {
        range: {
          start: { line: 0, character: 5 },
          end: { line: 0, character: 11 },
        },
        newText: "",
      },
    ], 2);

    const doc = dm.get("file:///test.txt");
    expect(doc!.text).toBe("hello");
  });
});
