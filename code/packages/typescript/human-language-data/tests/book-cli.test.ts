import {
  existsSync,
  mkdtempSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { afterEach, describe, expect, it, vi } from "vitest";
import { generatedBookOutputs, runBookGeneration } from "../src/book-cli.js";

const roots: string[] = [];

function fixture(output = "test/book/chapters/ch01-first.tex"): string {
  const root = mkdtempSync(join(tmpdir(), "human-language-book-"));
  roots.push(root);
  mkdirSync(join(root, "core"), { recursive: true });
  mkdirSync(join(root, "test", "lessons"), { recursive: true });
  writeFileSync(
    join(root, "core", "book-generation.json"),
    `${JSON.stringify({
      version: 1,
      targets: [{ language: "test", chapter: 1, title: "Hello", label: "ch:hello", output }],
    })}\n`,
  );
  writeFileSync(
    join(root, "test", "lessons", "hello.md"),
    `---
schema_version: 2
id: TEST-C01-hello
spine_node: HELLO
sequence: 10
chapter: 1
type: word
headword: hello
gloss: hello
concept_tag: GREETING-HELLO
duration:
  max_seconds: 120
---

# hello

## Warm-up

Say hello.

## Wrap-up Recall

Say hello again.
`,
  );
  return root;
}

afterEach(() => {
  vi.restoreAllMocks();
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true });
});

describe("canonical book generator filesystem shell", () => {
  it("writes and checks the generated chapter plus source-hash manifest", () => {
    const root = fixture();
    vi.spyOn(process.stdout, "write").mockImplementation(() => true);
    vi.spyOn(process.stderr, "write").mockImplementation(() => true);

    expect(runBookGeneration(["--write"], root)).toBe(0);
    const chapter = join(root, "test", "book", "chapters", "ch01-first.tex");
    const manifest = join(root, "core", "generated-book-hashes.json");
    expect(existsSync(chapter)).toBe(true);
    expect(readFileSync(manifest, "utf8")).toContain('"algorithm": "fnv1a64"');
    expect(runBookGeneration(["--check"], root)).toBe(0);

    writeFileSync(chapter, "stale\n");
    expect(runBookGeneration(["--check"], root)).toBe(1);
    expect(process.stderr.write).toHaveBeenCalledWith(
      "test/book/chapters/ch01-first.tex: generated output is missing or stale\n",
    );
  });

  it("rejects outputs outside the curriculum root", () => {
    const root = fixture("../../escape.tex");
    expect(() => generatedBookOutputs(root)).toThrow(/unsafe generated book output/);
  });

  it("rejects an empty generation config and unsupported CLI modes", () => {
    const root = fixture();
    writeFileSync(
      join(root, "core", "book-generation.json"),
      `${JSON.stringify({ version: 1, targets: [] })}\n`,
    );
    expect(() => generatedBookOutputs(root)).toThrow(/at least one target/);

    vi.spyOn(process.stderr, "write").mockImplementation(() => true);
    expect(runBookGeneration([], root)).toBe(2);
    expect(process.stderr.write).toHaveBeenCalledWith("usage: book-cli (--check | --write)\n");
  });
});
