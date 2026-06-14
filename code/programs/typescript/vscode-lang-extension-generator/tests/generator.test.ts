/**
 * Tests for vscode-lang-extension-generator.
 *
 * These tests pin down the contract documented in
 * `code/specs/LS04-vscode-extension-generator.md`:
 *
 * 1. Validation: bad inputs are rejected with clear errors.
 * 2. File layout: only the right files are emitted for the requested
 *    capability set (LSP-only, DAP-only, both, with/without grammar).
 * 3. Content correctness: emitted package.json carries the right
 *    activationEvents, contributes.* sections, and dependencies that
 *    match the wired capabilities.
 * 4. Determinism: same inputs produce byte-identical output.
 *
 * Tests run the generator's `generate()` library function — no spawning
 * of the CLI. The CLI wrapper around `generate()` is exercised separately.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { execSync } from "node:child_process";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import {
  GeneratorOptions,
  escapeRegex,
  generate,
  loadLanguageSpec,
  validateOptions,
  ValidationError,
  buildPackageJson,
  buildExtensionTs,
  buildLanguageConfiguration,
  buildTextMateGrammar,
  buildLspTs,
  buildDapTs,
  main,
} from "../src/index.js";

// ----------------------------------------------------------------------
// Test fixtures
// ----------------------------------------------------------------------

/** A fully-loaded option set: LSP+DAP+keywords+comments. */
function fullOpts(overrides: Partial<GeneratorOptions> = {}): GeneratorOptions {
  return {
    languageId: "twig",
    languageName: "Twig",
    fileExtensions: [".twig", ".tw"],
    lspBinary: "twig-lsp-server",
    dapBinary: "twig-dap",
    outputDir: "/tmp/ignored",
    lineComment: ";",
    blockCommentStart: "",
    blockCommentEnd: "",
    keywords: ["define", "if", "let", "lambda"],
    description: "Twig language support for VS Code",
    extensionVersion: "0.1.0",
    ...overrides,
  };
}

/** Minimal option set: DAP-only, no syntax highlighting. */
function dapOnlyOpts(overrides: Partial<GeneratorOptions> = {}): GeneratorOptions {
  return {
    languageId: "nib",
    languageName: "Nib",
    fileExtensions: [".nib"],
    lspBinary: "",
    dapBinary: "nib-dap",
    outputDir: "/tmp/ignored",
    lineComment: "",
    blockCommentStart: "",
    blockCommentEnd: "",
    keywords: [],
    description: "",
    extensionVersion: "0.1.0",
    ...overrides,
  };
}

/** LSP-only. */
function lspOnlyOpts(overrides: Partial<GeneratorOptions> = {}): GeneratorOptions {
  return {
    languageId: "smol",
    languageName: "Smol",
    fileExtensions: [".smol"],
    lspBinary: "smol-lsp-server",
    dapBinary: "",
    outputDir: "/tmp/ignored",
    lineComment: "",
    blockCommentStart: "",
    blockCommentEnd: "",
    keywords: [],
    description: "",
    extensionVersion: "0.1.0",
    ...overrides,
  };
}

// ----------------------------------------------------------------------
// validateOptions
// ----------------------------------------------------------------------

describe("validateOptions", () => {
  it("accepts a fully-populated options object", () => {
    expect(() => validateOptions(fullOpts())).not.toThrow();
  });

  it("rejects empty languageId", () => {
    expect(() => validateOptions(fullOpts({ languageId: "" }))).toThrow(ValidationError);
  });

  it("rejects languageId with uppercase", () => {
    expect(() => validateOptions(fullOpts({ languageId: "Twig" }))).toThrow(ValidationError);
  });

  it("rejects languageId starting with digit", () => {
    expect(() => validateOptions(fullOpts({ languageId: "1twig" }))).toThrow(ValidationError);
  });

  it("rejects languageId with spaces", () => {
    expect(() => validateOptions(fullOpts({ languageId: "my lang" }))).toThrow(ValidationError);
  });

  it("accepts languageId with internal hyphens and digits", () => {
    expect(() => validateOptions(fullOpts({ languageId: "my-lang-2" }))).not.toThrow();
  });

  it("rejects empty languageName", () => {
    expect(() => validateOptions(fullOpts({ languageName: "" }))).toThrow(ValidationError);
  });

  it("rejects empty fileExtensions", () => {
    expect(() => validateOptions(fullOpts({ fileExtensions: [] }))).toThrow(ValidationError);
  });

  it("rejects fileExtension that does not start with a dot", () => {
    expect(() => validateOptions(fullOpts({ fileExtensions: ["twig"] }))).toThrow(ValidationError);
  });

  it("rejects fileExtension containing whitespace", () => {
    expect(() => validateOptions(fullOpts({ fileExtensions: [".tw ig"] }))).toThrow(ValidationError);
  });

  // -----------------------------------------------------------------
  // Path-traversal hardening — the spec-driven flow makes the
  // language-spec JSON an untrusted input.  fileExtensions[0] flows
  // into `path.join(outputDir, "examples/sample" + ext)`, so any
  // path separator or `..` segment must be rejected at validation.
  // -----------------------------------------------------------------

  it("rejects fileExtension containing a forward slash", () => {
    expect(() => validateOptions(fullOpts({ fileExtensions: ["./evil"] }))).toThrow(
      ValidationError,
    );
  });

  it("rejects fileExtension containing a backslash", () => {
    expect(() => validateOptions(fullOpts({ fileExtensions: [".\\evil"] }))).toThrow(
      ValidationError,
    );
  });

  it("rejects fileExtension that is just a dot", () => {
    expect(() => validateOptions(fullOpts({ fileExtensions: ["."] }))).toThrow(
      ValidationError,
    );
  });

  it("rejects fileExtension with a parent-directory segment", () => {
    expect(() =>
      validateOptions(fullOpts({ fileExtensions: ["./../../etc"] })),
    ).toThrow(ValidationError);
  });

  it("rejects all-dot fileExtensions like '..' or '...'", () => {
    expect(() => validateOptions(fullOpts({ fileExtensions: [".."] }))).toThrow(
      ValidationError,
    );
    expect(() =>
      validateOptions(fullOpts({ fileExtensions: ["..."] })),
    ).toThrow(ValidationError);
  });

  it("accepts realistic file extensions", () => {
    expect(() =>
      validateOptions(fullOpts({ fileExtensions: [".twig", ".tw"] })),
    ).not.toThrow();
    expect(() =>
      validateOptions(fullOpts({ fileExtensions: [".cpp", ".c++", ".h"] })),
    ).not.toThrow();
    expect(() =>
      validateOptions(fullOpts({ fileExtensions: [".tar.gz"] })),
    ).not.toThrow();
  });

  it("rejects when neither lspBinary nor dapBinary is given", () => {
    expect(() =>
      validateOptions(fullOpts({ lspBinary: "", dapBinary: "" })),
    ).toThrow(ValidationError);
  });

  it("accepts LSP-only", () => {
    expect(() => validateOptions(lspOnlyOpts())).not.toThrow();
  });

  it("accepts DAP-only", () => {
    expect(() => validateOptions(dapOnlyOpts())).not.toThrow();
  });

  it("rejects empty extensionVersion", () => {
    expect(() => validateOptions(fullOpts({ extensionVersion: "" }))).toThrow(ValidationError);
  });

  // -----------------------------------------------------------------
  // Injection-defence: lspBinary, dapBinary, languageName, description
  // all flow into source-code string literals.  The validator must
  // reject characters that could break out of the literal.
  // -----------------------------------------------------------------

  it("rejects lspBinary containing a quote", () => {
    expect(() =>
      validateOptions(fullOpts({ lspBinary: '";process.exit(1);//' })),
    ).toThrow(ValidationError);
  });

  it("rejects lspBinary containing a backtick", () => {
    expect(() => validateOptions(fullOpts({ lspBinary: "evil`" }))).toThrow(ValidationError);
  });

  it("rejects lspBinary containing whitespace", () => {
    expect(() => validateOptions(fullOpts({ lspBinary: "two words" }))).toThrow(ValidationError);
  });

  it("rejects dapBinary containing a semicolon and parens", () => {
    expect(() =>
      validateOptions(fullOpts({ dapBinary: "ok;require('x')" })),
    ).toThrow(ValidationError);
  });

  it("rejects languageName containing a double-quote", () => {
    expect(() =>
      validateOptions(fullOpts({ languageName: 'Twig"; bad();//' })),
    ).toThrow(ValidationError);
  });

  it("rejects languageName containing a template-substitution marker", () => {
    expect(() =>
      validateOptions(fullOpts({ languageName: "Twig${process.cwd()}" })),
    ).toThrow(ValidationError);
  });

  it("rejects languageName containing a backslash", () => {
    expect(() =>
      validateOptions(fullOpts({ languageName: "Twig\\nevil" })),
    ).toThrow(ValidationError);
  });

  it("rejects description containing a backtick", () => {
    expect(() =>
      validateOptions(fullOpts({ description: "x`y" })),
    ).toThrow(ValidationError);
  });

  it("accepts safe binary names with dots, slashes, hyphens, and underscores", () => {
    expect(() =>
      validateOptions(fullOpts({ lspBinary: "./bin/twig-lsp_server.v2" })),
    ).not.toThrow();
  });
});

// ----------------------------------------------------------------------
// Injection defence — generated source must be safe even if a
// malicious value somehow bypasses validateOptions (defense in depth).
// ----------------------------------------------------------------------

describe("injection defence: builders use JSON.stringify for user input", () => {
  /**
   * Construct a malicious-but-validation-passing option set is not
   * possible because the validator catches all problematic
   * characters.  But to prove the JSON.stringify path is used, we
   * spy on what the builders emit when given the *most permissive*
   * legal binary name and verify the result has the binary inside
   * a properly-quoted JS string literal — not an unescaped raw
   * embed that could be coaxed into code execution if validation
   * regressed.
   */
  it("buildLspTs embeds the binary as a JSON-encoded string literal", () => {
    const text = buildLspTs(fullOpts({ lspBinary: "twig-lsp-server" }));
    expect(text).toContain('"twig-lsp-server"');
    // Should NOT contain a bare ${...} interpolation marker, which
    // would suggest template-string interpolation of user input.
    expect(text).not.toMatch(/\$\{opts\./);
  });

  it("buildDapTs embeds the binary as a JSON-encoded string literal", () => {
    const text = buildDapTs(fullOpts({ dapBinary: "twig-dap" }));
    expect(text).toContain('"twig-dap"');
    expect(text).not.toMatch(/\$\{opts\./);
  });

  it("buildLspTs embeds the language name as a JSON-encoded string literal", () => {
    const text = buildLspTs(fullOpts({ languageName: "My Language" }));
    expect(text).toContain('"My Language Language Server"');
  });
});

// ----------------------------------------------------------------------
// buildPackageJson
// ----------------------------------------------------------------------

describe("buildPackageJson", () => {
  it("produces parseable JSON", () => {
    const text = buildPackageJson(fullOpts());
    expect(() => JSON.parse(text)).not.toThrow();
  });

  it("uses <id>-vscode as the package name", () => {
    const pkg = JSON.parse(buildPackageJson(fullOpts()));
    expect(pkg.name).toBe("twig-vscode");
  });

  it("uses the language name as displayName", () => {
    const pkg = JSON.parse(buildPackageJson(fullOpts()));
    expect(pkg.displayName).toBe("Twig");
  });

  it("uses the explicit description when provided", () => {
    const pkg = JSON.parse(buildPackageJson(fullOpts({ description: "custom" })));
    expect(pkg.description).toBe("custom");
  });

  it("falls back to an autogenerated description when none provided", () => {
    const pkg = JSON.parse(buildPackageJson(fullOpts({ description: "" })));
    expect(typeof pkg.description).toBe("string");
    expect(pkg.description.length).toBeGreaterThan(0);
  });

  it("emits onLanguage activationEvent", () => {
    const pkg = JSON.parse(buildPackageJson(fullOpts()));
    expect(pkg.activationEvents).toContain("onLanguage:twig");
  });

  it("emits onDebug activationEvent when DAP wired", () => {
    const pkg = JSON.parse(buildPackageJson(fullOpts()));
    expect(pkg.activationEvents).toContain("onDebug");
  });

  it("omits onDebug activationEvent when DAP not wired", () => {
    const pkg = JSON.parse(buildPackageJson(lspOnlyOpts()));
    expect(pkg.activationEvents).not.toContain("onDebug");
  });

  it("registers the language with all extensions", () => {
    const pkg = JSON.parse(buildPackageJson(fullOpts()));
    expect(pkg.contributes.languages).toHaveLength(1);
    expect(pkg.contributes.languages[0].id).toBe("twig");
    expect(pkg.contributes.languages[0].extensions).toEqual([".twig", ".tw"]);
    expect(pkg.contributes.languages[0].aliases).toEqual(["Twig"]);
  });

  it("emits contributes.grammars only when keywords are provided", () => {
    const withKw = JSON.parse(buildPackageJson(fullOpts()));
    const withoutKw = JSON.parse(buildPackageJson(fullOpts({ keywords: [] })));
    expect(withKw.contributes.grammars).toBeDefined();
    expect(withoutKw.contributes.grammars).toBeUndefined();
  });

  it("emits contributes.debuggers only when DAP wired", () => {
    const withDap = JSON.parse(buildPackageJson(fullOpts()));
    const withoutDap = JSON.parse(buildPackageJson(lspOnlyOpts()));
    expect(withDap.contributes.debuggers).toBeDefined();
    expect(withDap.contributes.debuggers[0].type).toBe("twig");
    expect(withoutDap.contributes.debuggers).toBeUndefined();
  });

  it("declares vscode-languageclient only when LSP wired", () => {
    const withLsp = JSON.parse(buildPackageJson(fullOpts()));
    const withoutLsp = JSON.parse(buildPackageJson(dapOnlyOpts()));
    expect(withLsp.dependencies?.["vscode-languageclient"]).toBeDefined();
    expect(withoutLsp.dependencies?.["vscode-languageclient"]).toBeUndefined();
  });

  it("emits serverPath setting only when LSP wired", () => {
    const withLsp = JSON.parse(buildPackageJson(fullOpts()));
    const withoutLsp = JSON.parse(buildPackageJson(dapOnlyOpts()));
    expect(withLsp.contributes.configuration.properties["twig.serverPath"]).toBeDefined();
    expect(withoutLsp.contributes.configuration.properties["nib.serverPath"]).toBeUndefined();
  });

  it("emits adapterPath setting only when DAP wired", () => {
    const withDap = JSON.parse(buildPackageJson(fullOpts()));
    const withoutDap = JSON.parse(buildPackageJson(lspOnlyOpts()));
    expect(withDap.contributes.configuration.properties["twig.adapterPath"]).toBeDefined();
    expect(withoutDap.contributes.configuration.properties["smol.adapterPath"]).toBeUndefined();
  });

  it("uses a deterministic version", () => {
    const pkg = JSON.parse(buildPackageJson(fullOpts({ extensionVersion: "1.2.3" })));
    expect(pkg.version).toBe("1.2.3");
  });

  // Regression: VS Code rejects extensions without a `publisher` field
  // when installed locally (the directory layout is
  // `<publisher>.<name>-<version>/`).  Always emit one.
  it("emits a non-empty publisher field", () => {
    const pkg = JSON.parse(buildPackageJson(fullOpts()));
    expect(typeof pkg.publisher).toBe("string");
    expect(pkg.publisher.length).toBeGreaterThan(0);
  });

  // Regression: the engines.vscode constraint must be permissive enough
  // to load on widely-deployed VS Code versions.  1.82 is the floor
  // imposed by vscode-languageclient@9.  Pinning to 1.85 silently
  // bricks installs on slightly older VS Code (1.84 was the production
  // version when this generator first shipped).  Don't tighten without
  // also bumping the dep bound and documenting why.
  it("declares engines.vscode no tighter than ^1.82.0", () => {
    const pkg = JSON.parse(buildPackageJson(fullOpts()));
    expect(pkg.engines.vscode).toBe("^1.82.0");
  });
});

// ----------------------------------------------------------------------
// buildExtensionTs / buildLspTs / buildDapTs
// ----------------------------------------------------------------------

describe("buildExtensionTs", () => {
  it("imports lsp helpers when LSP wired", () => {
    const text = buildExtensionTs(fullOpts());
    expect(text).toMatch(/from ["']\.\/lsp["']/);
  });

  it("does not import lsp helpers when LSP not wired", () => {
    const text = buildExtensionTs(dapOnlyOpts());
    expect(text).not.toMatch(/from ["']\.\/lsp["']/);
  });

  it("imports dap helpers when DAP wired", () => {
    const text = buildExtensionTs(fullOpts());
    expect(text).toMatch(/from ["']\.\/dap["']/);
  });

  it("does not import dap helpers when DAP not wired", () => {
    const text = buildExtensionTs(lspOnlyOpts());
    expect(text).not.toMatch(/from ["']\.\/dap["']/);
  });

  it("exports activate and deactivate", () => {
    const text = buildExtensionTs(fullOpts());
    expect(text).toMatch(/export\s+(async\s+)?function\s+activate/);
    expect(text).toMatch(/export\s+(async\s+)?function\s+deactivate/);
  });
});

describe("buildLspTs", () => {
  it("references the lsp binary name", () => {
    const text = buildLspTs(fullOpts());
    expect(text).toContain("twig-lsp-server");
  });

  it("references the configuration key", () => {
    const text = buildLspTs(fullOpts());
    expect(text).toContain("twig.serverPath");
  });

  it("registers the language id as document selector", () => {
    const text = buildLspTs(fullOpts());
    expect(text).toContain('"twig"');
  });
});

describe("buildDapTs", () => {
  it("references the dap binary name", () => {
    const text = buildDapTs(fullOpts());
    expect(text).toContain("twig-dap");
  });

  it("references the adapter configuration key", () => {
    const text = buildDapTs(fullOpts());
    expect(text).toContain("twig.adapterPath");
  });

  it("registers the language id as the debug type", () => {
    const text = buildDapTs(fullOpts());
    expect(text).toMatch(/registerDebugAdapterDescriptorFactory\(\s*["']twig["']/);
  });
});

// ----------------------------------------------------------------------
// buildLanguageConfiguration
// ----------------------------------------------------------------------

describe("buildLanguageConfiguration", () => {
  it("emits parseable JSON", () => {
    const text = buildLanguageConfiguration(fullOpts());
    expect(() => JSON.parse(text)).not.toThrow();
  });

  it("includes line comment when provided", () => {
    const cfg = JSON.parse(buildLanguageConfiguration(fullOpts({ lineComment: ";" })));
    expect(cfg.comments.lineComment).toBe(";");
  });

  it("omits line comment when not provided", () => {
    const cfg = JSON.parse(buildLanguageConfiguration(fullOpts({ lineComment: "" })));
    expect(cfg.comments?.lineComment).toBeUndefined();
  });

  it("includes block comment when both endpoints provided", () => {
    const cfg = JSON.parse(
      buildLanguageConfiguration(
        fullOpts({ blockCommentStart: "/*", blockCommentEnd: "*/" }),
      ),
    );
    expect(cfg.comments.blockComment).toEqual(["/*", "*/"]);
  });

  it("omits block comment when only one endpoint provided", () => {
    const cfg = JSON.parse(
      buildLanguageConfiguration(
        fullOpts({ blockCommentStart: "/*", blockCommentEnd: "" }),
      ),
    );
    expect(cfg.comments?.blockComment).toBeUndefined();
  });

  it("always includes brackets, autoClosingPairs, surroundingPairs", () => {
    const cfg = JSON.parse(buildLanguageConfiguration(fullOpts()));
    expect(Array.isArray(cfg.brackets)).toBe(true);
    expect(Array.isArray(cfg.autoClosingPairs)).toBe(true);
    expect(Array.isArray(cfg.surroundingPairs)).toBe(true);
  });
});

// ----------------------------------------------------------------------
// buildTextMateGrammar
// ----------------------------------------------------------------------

describe("buildTextMateGrammar", () => {
  it("returns null when keywords list is empty", () => {
    expect(buildTextMateGrammar(fullOpts({ keywords: [] }))).toBeNull();
  });

  it("emits parseable JSON when keywords given", () => {
    const text = buildTextMateGrammar(fullOpts())!;
    expect(() => JSON.parse(text)).not.toThrow();
  });

  it("uses the language id as scopeName suffix", () => {
    const grammar = JSON.parse(buildTextMateGrammar(fullOpts())!);
    expect(grammar.scopeName).toBe("source.twig");
  });

  it("includes all keywords in the regex", () => {
    const grammar = JSON.parse(buildTextMateGrammar(fullOpts({ keywords: ["foo", "bar"] }))!);
    const keywordPattern = grammar.patterns.find(
      (p: { name?: string }) => p.name === "keyword.control.twig",
    );
    expect(keywordPattern.match).toContain("foo");
    expect(keywordPattern.match).toContain("bar");
  });

  it("escapes regex metacharacters in keywords", () => {
    const grammar = JSON.parse(buildTextMateGrammar(fullOpts({ keywords: ["a.b", "c+d"] }))!);
    const keywordPattern = grammar.patterns.find(
      (p: { name?: string }) => p.name === "keyword.control.twig",
    );
    // Dots and plusses must be escaped or they will match other characters.
    expect(keywordPattern.match).toMatch(/a\\\.b/);
    expect(keywordPattern.match).toMatch(/c\\\+d/);
  });
});

// ----------------------------------------------------------------------
// generate (full filesystem run)
// ----------------------------------------------------------------------

describe("generate", () => {
  let tmpRoot: string;

  beforeEach(() => {
    tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "vscode-ext-gen-"));
  });

  afterEach(() => {
    fs.rmSync(tmpRoot, { recursive: true, force: true });
  });

  it("writes the canonical file tree for LSP+DAP+keywords", () => {
    const out = path.join(tmpRoot, "twig-vscode");
    generate(fullOpts({ outputDir: out }));

    const expected = [
      "package.json",
      "tsconfig.json",
      ".vscodeignore",
      ".gitignore",
      "README.md",
      "CHANGELOG.md",
      "BUILD",
      "BUILD_windows",
      "language-configuration.json",
      "src/extension.ts",
      "src/lsp.ts",
      "src/dap.ts",
      "syntaxes/twig.tmLanguage.json",
      "examples/sample.twig",
    ];
    for (const rel of expected) {
      expect(fs.existsSync(path.join(out, rel)), rel).toBe(true);
    }
  });

  it("does not write src/lsp.ts when LSP not wired", () => {
    const out = path.join(tmpRoot, "nib-vscode");
    generate(dapOnlyOpts({ outputDir: out }));
    expect(fs.existsSync(path.join(out, "src/lsp.ts"))).toBe(false);
    expect(fs.existsSync(path.join(out, "src/dap.ts"))).toBe(true);
  });

  it("does not write src/dap.ts when DAP not wired", () => {
    const out = path.join(tmpRoot, "smol-vscode");
    generate(lspOnlyOpts({ outputDir: out }));
    expect(fs.existsSync(path.join(out, "src/dap.ts"))).toBe(false);
    expect(fs.existsSync(path.join(out, "src/lsp.ts"))).toBe(true);
  });

  it("does not write syntaxes/ when no keywords", () => {
    const out = path.join(tmpRoot, "nib-vscode");
    generate(dapOnlyOpts({ outputDir: out }));
    expect(fs.existsSync(path.join(out, "syntaxes"))).toBe(false);
  });

  it("refuses to write into a non-empty directory", () => {
    const out = path.join(tmpRoot, "occupied");
    fs.mkdirSync(out, { recursive: true });
    fs.writeFileSync(path.join(out, "leftover.txt"), "hello");
    expect(() => generate(fullOpts({ outputDir: out }))).toThrow();
  });

  it("creates parent directories that don't exist", () => {
    const out = path.join(tmpRoot, "deep", "nested", "twig-vscode");
    expect(() => generate(fullOpts({ outputDir: out }))).not.toThrow();
    expect(fs.existsSync(path.join(out, "package.json"))).toBe(true);
  });

  it("is deterministic — running twice into different dirs yields identical content", () => {
    const a = path.join(tmpRoot, "a");
    const b = path.join(tmpRoot, "b");
    generate(fullOpts({ outputDir: a }));
    generate(fullOpts({ outputDir: b }));
    const filesA = listAll(a);
    const filesB = listAll(b);
    expect(filesA).toEqual(filesB);
    for (const rel of filesA) {
      const ca = fs.readFileSync(path.join(a, rel), "utf-8");
      const cb = fs.readFileSync(path.join(b, rel), "utf-8");
      expect(ca, rel).toBe(cb);
    }
  });

  it("emits a sample file with the first extension", () => {
    const out = path.join(tmpRoot, "twig-vscode");
    generate(fullOpts({ outputDir: out }));
    expect(fs.existsSync(path.join(out, "examples/sample.twig"))).toBe(true);
  });

  it("emits valid JSON for package.json from disk", () => {
    const out = path.join(tmpRoot, "twig-vscode");
    generate(fullOpts({ outputDir: out }));
    const text = fs.readFileSync(path.join(out, "package.json"), "utf-8");
    expect(() => JSON.parse(text)).not.toThrow();
  });

  // Regression: CI runs BUILD via `sh` on Linux (dash), which rejects
  // bash-isms like `-o pipefail`.  Lock down the generated BUILD to
  // POSIX-only shell so it works under both bash and dash.
  it("generated BUILD does not use bash-specific syntax", () => {
    const out = path.join(tmpRoot, "twig-vscode");
    generate(fullOpts({ outputDir: out }));
    const buildText = fs.readFileSync(path.join(out, "BUILD"), "utf-8");
    expect(buildText).not.toMatch(/pipefail/);
    expect(buildText).not.toMatch(/^#!\s*\/.*bash/m);
    // `set -u` (treat unset vars as error) is also bash-flavoured in
    // some dashes; drop it too.
    expect(buildText).not.toMatch(/^set\s+-[a-z]*u/m);
  });
});

// ----------------------------------------------------------------------
// escapeRegex
// ----------------------------------------------------------------------

describe("escapeRegex", () => {
  it("escapes regex metacharacters", () => {
    expect(escapeRegex("a.b")).toBe("a\\.b");
    expect(escapeRegex("c+d")).toBe("c\\+d");
    expect(escapeRegex("(x)")).toBe("\\(x\\)");
    expect(escapeRegex("[a]")).toBe("\\[a\\]");
    expect(escapeRegex("$0^")).toBe("\\$0\\^");
  });

  it("leaves non-special characters alone", () => {
    expect(escapeRegex("abc123")).toBe("abc123");
  });
});

// ----------------------------------------------------------------------
// loadLanguageSpec
// ----------------------------------------------------------------------

describe("loadLanguageSpec", () => {
  let tmpDir: string;
  let specPath: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "vscode-ext-gen-spec-"));
    specPath = path.join(tmpDir, "twig.spec.json");
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  /** Mirror of what `twig-spec-dump` actually produces. */
  function writeRealisticSpec(): void {
    fs.writeFileSync(
      specPath,
      JSON.stringify(
        {
          $schemaVersion: 1,
          languageId: "twig",
          languageName: "Twig",
          fileExtensions: ["twig", "tw"],
          keywords: ["define", "lambda", "let", "if"],
          reservedKeywords: [],
          contextKeywords: [],
          lineComment: ";",
          blockComment: null,
          brackets: [["(", ")"]],
          rules: ["program", "expression"],
          declarationRules: ["define", "module_form"],
          caseSensitive: true,
        },
        null,
        2,
      ),
    );
  }

  it("extracts languageId/Name/extensions/keywords/lineComment", () => {
    writeRealisticSpec();
    const patch = loadLanguageSpec(specPath);
    expect(patch.languageId).toBe("twig");
    expect(patch.languageName).toBe("Twig");
    expect(patch.fileExtensions).toEqual([".twig", ".tw"]);  // dots re-added
    expect(patch.keywords).toEqual(["define", "lambda", "let", "if"]);
    expect(patch.lineComment).toBe(";");
  });

  it("re-adds leading dots to file extensions if absent", () => {
    fs.writeFileSync(
      specPath,
      JSON.stringify({ $schemaVersion: 1, fileExtensions: ["twig", "tw"] }),
    );
    const patch = loadLanguageSpec(specPath);
    expect(patch.fileExtensions).toEqual([".twig", ".tw"]);
  });

  it("preserves leading dots if already present", () => {
    fs.writeFileSync(
      specPath,
      JSON.stringify({ $schemaVersion: 1, fileExtensions: [".twig"] }),
    );
    const patch = loadLanguageSpec(specPath);
    expect(patch.fileExtensions).toEqual([".twig"]);
  });

  it("populates blockComment when both endpoints present", () => {
    fs.writeFileSync(
      specPath,
      JSON.stringify({
        $schemaVersion: 1,
        blockComment: ["/*", "*/"],
      }),
    );
    const patch = loadLanguageSpec(specPath);
    expect(patch.blockCommentStart).toBe("/*");
    expect(patch.blockCommentEnd).toBe("*/");
  });

  it("leaves blockComment unset when null", () => {
    fs.writeFileSync(
      specPath,
      JSON.stringify({ $schemaVersion: 1, blockComment: null }),
    );
    const patch = loadLanguageSpec(specPath);
    expect(patch.blockCommentStart).toBeUndefined();
    expect(patch.blockCommentEnd).toBeUndefined();
  });

  it("throws on missing file", () => {
    expect(() => loadLanguageSpec(path.join(tmpDir, "nope.json"))).toThrow(
      /failed to read/,
    );
  });

  it("throws on invalid JSON", () => {
    fs.writeFileSync(specPath, "{ this is not json");
    expect(() => loadLanguageSpec(specPath)).toThrow(/not valid JSON/);
  });

  it("rejects unsupported $schemaVersion", () => {
    fs.writeFileSync(
      specPath,
      JSON.stringify({ $schemaVersion: 99, languageId: "x" }),
    );
    expect(() => loadLanguageSpec(specPath)).toThrow(/unsupported \$schemaVersion/);
  });

  it("accepts a spec without $schemaVersion (forward-compat)", () => {
    fs.writeFileSync(specPath, JSON.stringify({ languageId: "x" }));
    expect(() => loadLanguageSpec(specPath)).not.toThrow();
  });

  it("ignores unknown fields without complaint", () => {
    fs.writeFileSync(
      specPath,
      JSON.stringify({
        $schemaVersion: 1,
        languageId: "twig",
        unknownFutureField: { nested: "thing" },
      }),
    );
    const patch = loadLanguageSpec(specPath);
    expect(patch.languageId).toBe("twig");
  });
});

// ----------------------------------------------------------------------
// CLI main()
// ----------------------------------------------------------------------

describe("main (CLI entry point)", () => {
  let tmpRoot: string;
  let stdoutSpy: ReturnType<typeof vi.spyOn>;
  let stderrSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "vscode-ext-gen-cli-"));
    stdoutSpy = vi.spyOn(process.stdout, "write").mockImplementation(() => true);
    stderrSpy = vi.spyOn(process.stderr, "write").mockImplementation(() => true);
  });

  afterEach(() => {
    fs.rmSync(tmpRoot, { recursive: true, force: true });
    stdoutSpy.mockRestore();
    stderrSpy.mockRestore();
  });

  it("returns 0 on a successful generation", async () => {
    const out = path.join(tmpRoot, "twig-vscode");
    const code = await main([
      "--language-id", "twig",
      "--language-name", "Twig",
      "--file-extensions", ".twig",
      "--lsp-binary", "twig-lsp-server",
      "--dap-binary", "twig-dap",
      "--output-dir", out,
      "--keywords", "define,if,let",
    ]);
    expect(code).toBe(0);
    expect(fs.existsSync(path.join(out, "package.json"))).toBe(true);
  });

  it("prints help and returns 0 on --help", async () => {
    const code = await main(["--help"]);
    expect(code).toBe(0);
    expect(stdoutSpy).toHaveBeenCalled();
  });

  it("returns 1 when --output-dir is missing", async () => {
    const code = await main([
      "--language-id", "twig",
      "--language-name", "Twig",
      "--file-extensions", ".twig",
      "--lsp-binary", "twig-lsp-server",
    ]);
    expect(code).toBe(1);
    expect(stderrSpy).toHaveBeenCalled();
  });

  it("returns 1 when validation fails (missing capability)", async () => {
    const out = path.join(tmpRoot, "x");
    const code = await main([
      "--language-id", "twig",
      "--language-name", "Twig",
      "--file-extensions", ".twig",
      "--output-dir", out,
    ]);
    expect(code).toBe(1);
    expect(stderrSpy).toHaveBeenCalled();
  });

  it("returns 1 on a parse error (unknown flag)", async () => {
    const code = await main(["--nonexistent-flag", "x"]);
    expect(code).toBe(1);
    expect(stderrSpy).toHaveBeenCalled();
  });

  it("accepts --language-spec and uses it in place of explicit flags", async () => {
    const specPath = path.join(tmpRoot, "twig.spec.json");
    fs.writeFileSync(
      specPath,
      JSON.stringify({
        $schemaVersion: 1,
        languageId: "twig",
        languageName: "Twig",
        fileExtensions: ["twig", "tw"],
        keywords: ["define", "if", "let"],
        lineComment: ";",
      }),
    );
    const out = path.join(tmpRoot, "twig-vscode");
    const code = await main([
      "--language-spec", specPath,
      "--lsp-binary", "twig-lsp-server",
      "--dap-binary", "twig-dap",
      "--output-dir", out,
    ]);
    expect(code).toBe(0);

    // The generator must have used the keywords from the spec — verify
    // by checking the emitted TextMate grammar includes them.
    const grammarPath = path.join(out, "syntaxes/twig.tmLanguage.json");
    expect(fs.existsSync(grammarPath)).toBe(true);
    const grammar = JSON.parse(fs.readFileSync(grammarPath, "utf-8"));
    const kwPattern = grammar.patterns.find(
      (p: { name?: string }) => p.name === "keyword.control.twig",
    );
    expect(kwPattern.match).toContain("define");
    expect(kwPattern.match).toContain("if");
    expect(kwPattern.match).toContain("let");
  });

  it("CLI flags override the language-spec when both are provided", async () => {
    const specPath = path.join(tmpRoot, "twig.spec.json");
    fs.writeFileSync(
      specPath,
      JSON.stringify({
        $schemaVersion: 1,
        languageId: "twig",
        languageName: "Twig",
        fileExtensions: ["twig"],
        keywords: ["define"],
      }),
    );
    const out = path.join(tmpRoot, "override-vscode");
    const code = await main([
      "--language-spec", specPath,
      "--language-name", "OverriddenName",
      "--keywords", "alpha,beta",
      "--lsp-binary", "twig-lsp-server",
      "--output-dir", out,
    ]);
    expect(code).toBe(0);
    const pkg = JSON.parse(fs.readFileSync(path.join(out, "package.json"), "utf-8"));
    expect(pkg.displayName).toBe("OverriddenName");
    const grammar = JSON.parse(
      fs.readFileSync(path.join(out, "syntaxes/twig.tmLanguage.json"), "utf-8"),
    );
    const kwPattern = grammar.patterns.find(
      (p: { name?: string }) => p.name === "keyword.control.twig",
    );
    expect(kwPattern.match).toContain("alpha");
    expect(kwPattern.match).toContain("beta");
    expect(kwPattern.match).not.toContain("define");
  });

  it("returns 1 with a clear error when --language-spec points at missing file", async () => {
    const out = path.join(tmpRoot, "missing-spec");
    const code = await main([
      "--language-spec", path.join(tmpRoot, "nope.json"),
      "--lsp-binary", "twig-lsp-server",
      "--output-dir", out,
    ]);
    expect(code).toBe(1);
    expect(stderrSpy).toHaveBeenCalled();
  });
});

// ----------------------------------------------------------------------
// Smoke: generated extension's TypeScript actually compiles
// ----------------------------------------------------------------------

/**
 * The generator's most important guarantee is that what it emits
 * compiles cleanly via tsc.  This test runs `tsc --noEmit` against
 * the generated source files (without installing vscode-languageclient
 * to keep the test fast — strict TS type errors will still surface).
 *
 * We skip the test if tsc isn't available locally — CI installs it,
 * but we don't want to fail on a fresh clone before npm install.
 */
describe("smoke: generated extension TypeScript syntax check", () => {
  let tmpRoot: string;

  beforeEach(() => {
    tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "vscode-ext-gen-smoke-"));
  });

  afterEach(() => {
    fs.rmSync(tmpRoot, { recursive: true, force: true });
  });

  it("generated extension.ts and dap.ts parse with --noEmit and noResolve", () => {
    const out = path.join(tmpRoot, "twig-vscode");
    generate({
      languageId: "twig",
      languageName: "Twig",
      fileExtensions: [".twig"],
      lspBinary: "",  // skip lsp.ts to avoid the vscode-languageclient import
      dapBinary: "twig-dap",
      outputDir: out,
      lineComment: ";",
      blockCommentStart: "",
      blockCommentEnd: "",
      keywords: [],
      description: "",
      extensionVersion: "0.1.0",
    });

    // Use a stub for the `vscode` import so type-checking doesn't need
    // @types/vscode pulled into the test environment.
    fs.mkdirSync(path.join(out, "node_modules", "vscode"), { recursive: true });
    fs.writeFileSync(
      path.join(out, "node_modules", "vscode", "index.d.ts"),
      `
declare module "vscode" {
  export interface ExtensionContext { subscriptions: { push(item: unknown): void }; }
  export interface DebugSession {}
  export type ProviderResult<T> = T | undefined;
  export interface DebugAdapterDescriptor {}
  export class DebugAdapterExecutable implements DebugAdapterDescriptor {
    constructor(command: string, args: string[]);
  }
  export interface DebugAdapterDescriptorFactory {
    createDebugAdapterDescriptor(session: DebugSession): ProviderResult<DebugAdapterDescriptor>;
  }
  export const debug: {
    registerDebugAdapterDescriptorFactory(type: string, factory: DebugAdapterDescriptorFactory): unknown;
  };
  export const workspace: {
    getConfiguration(section: string): { get<T>(key: string, def: T): T };
  };
}
`,
    );
    fs.writeFileSync(
      path.join(out, "node_modules", "vscode", "package.json"),
      JSON.stringify({ name: "vscode", types: "index.d.ts" }),
    );

    // Try to compile with the project tsconfig.  If tsc isn't installed
    // locally we skip — the project root build pipeline will catch it.
    const tsc = path.resolve(__dirname, "..", "node_modules", ".bin", "tsc");
    if (!fs.existsSync(tsc)) {
      return; // skip silently
    }
    try {
      execSync(`"${tsc}" --noEmit -p .`, { cwd: out, stdio: "pipe" });
    } catch (err) {
      const e = err as { stdout?: Buffer; stderr?: Buffer };
      const out = (e.stdout?.toString() ?? "") + (e.stderr?.toString() ?? "");
      // We only fail on real syntax/type errors in our generated code.
      // Errors about missing "vscode-languageclient" types only occur if
      // someone forgot the lspBinary skip; this test guards against that.
      throw new Error(`tsc rejected generated source:\n${out}`);
    }
  });
});

/** Recursively list all files in a directory, returning paths relative to root, sorted. */
function listAll(root: string): string[] {
  const out: string[] = [];
  function walk(dir: string, rel: string): void {
    for (const entry of fs.readdirSync(dir).sort()) {
      const full = path.join(dir, entry);
      const r = path.posix.join(rel, entry);
      const stat = fs.statSync(full);
      if (stat.isDirectory()) {
        walk(full, r);
      } else {
        out.push(r);
      }
    }
  }
  walk(root, "");
  return out;
}
