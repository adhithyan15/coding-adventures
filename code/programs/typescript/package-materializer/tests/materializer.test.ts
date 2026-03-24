import assert from "node:assert/strict";
import { existsSync, readFileSync, rmSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { collectPackageClosure, materializePackage } from "../src/materializer.js";

function repoRootFromHere(): string {
  let current = path.dirname(fileURLToPath(import.meta.url));

  while (true) {
    const sharedSourceRoot = path.join(current, "code", "src", "typescript");
    const shellRoot = path.join(current, "code", "packages", "typescript");

    if (existsSync(sharedSourceRoot) && existsSync(shellRoot)) {
      return current;
    }

    const parent = path.dirname(current);
    if (parent === current) {
      throw new Error("Could not locate repository root from materializer tests");
    }
    current = parent;
  }
}

function tempOutputRoot(): string {
  return path.join(os.tmpdir(), `package-materializer-${Date.now()}-${Math.random().toString(16).slice(2)}`);
}

test("collectPackageClosure finds the shared-source dependency chain for typescript-lexer", () => {
  const repoRoot = repoRootFromHere();
  const closure = collectPackageClosure("typescript-lexer", repoRoot);

  assert.deepEqual(closure, [
    "directed-graph",
    "grammar-tools",
    "lexer",
    "state-machine",
    "typescript-lexer",
  ]);
});

test("materializePackage creates a standalone typescript-lexer artifact", () => {
  const repoRoot = repoRootFromHere();
  const outputRoot = tempOutputRoot();

  try {
    const result = materializePackage("typescript-lexer", { repoRoot, outputRoot });
    const packageRoot = result.outputDir;

    assert.ok(existsSync(path.join(packageRoot, "package.json")));
    assert.ok(existsSync(path.join(packageRoot, "README.md")));
    assert.ok(existsSync(path.join(packageRoot, "src", "index.ts")));
    assert.ok(existsSync(path.join(packageRoot, "src", "tokenizer.ts")));
    assert.ok(existsSync(path.join(packageRoot, "src", "tokens", "typescript.tokens")));
    assert.ok(existsSync(path.join(packageRoot, "src", "typescript", "lexer", "index.ts")));
    assert.ok(existsSync(path.join(packageRoot, "src", "typescript", "typescript-lexer", "tokenizer.ts")));

    const wrapper = readFileSync(path.join(packageRoot, "src", "index.ts"), "utf8");
    assert.equal(wrapper, 'export * from "./typescript/typescript-lexer/index.js";\n');

    const packageJson = JSON.parse(readFileSync(path.join(packageRoot, "package.json"), "utf8")) as Record<string, unknown>;
    assert.equal(packageJson.name, "@coding-adventures/typescript-lexer");
    assert.equal("dependencies" in packageJson, false);
    assert.equal("devDependencies" in packageJson, false);

    const manifest = JSON.parse(readFileSync(path.join(packageRoot, "materialization.json"), "utf8")) as {
      shared_packages: string[];
    };
    assert.deepEqual(manifest.shared_packages, [
      "directed-graph",
      "grammar-tools",
      "lexer",
      "state-machine",
      "typescript-lexer",
    ]);
  } finally {
    rmSync(outputRoot, { recursive: true, force: true });
  }
});

test("materializePackage creates wrappers for every top-level source file in lexer", () => {
  const repoRoot = repoRootFromHere();
  const outputRoot = tempOutputRoot();

  try {
    const result = materializePackage("lexer", { repoRoot, outputRoot });
    const packageRoot = result.outputDir;

    assert.ok(existsSync(path.join(packageRoot, "src", "index.ts")));
    assert.ok(existsSync(path.join(packageRoot, "src", "tokenizer.ts")));
    assert.ok(existsSync(path.join(packageRoot, "src", "tokenizer-dfa.ts")));
    assert.ok(existsSync(path.join(packageRoot, "src", "grammar-lexer.ts")));

    const dfaWrapper = readFileSync(path.join(packageRoot, "src", "tokenizer-dfa.ts"), "utf8");
    assert.equal(dfaWrapper, 'export * from "./typescript/lexer/tokenizer-dfa.js";\n');
  } finally {
    rmSync(outputRoot, { recursive: true, force: true });
  }
});

test("materializePackage rejects packages that do not exist in shared source", () => {
  const repoRoot = repoRootFromHere();
  const outputRoot = tempOutputRoot();

  try {
    assert.throws(
      () => materializePackage("definitely-not-a-package", { repoRoot, outputRoot }),
      /Shell package not found/,
    );
  } finally {
    rmSync(outputRoot, { recursive: true, force: true });
  }
});
