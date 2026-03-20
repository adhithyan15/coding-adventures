/**
 * # Capability Analyzer — AST Walker
 *
 * This is the heart of the capability analyzer. It uses the TypeScript Compiler API
 * to parse source files into Abstract Syntax Trees (ASTs), then walks those trees
 * to detect OS capability usage.
 *
 * ## What is a "Capability"?
 *
 * A capability is a permission to interact with the operating system. When your code
 * reads a file, opens a network connection, or spawns a child process, it's using
 * OS capabilities. This analyzer detects those usages by examining the code's AST
 * without ever running it — that's what makes it a "static" analyzer.
 *
 * ## Capability Notation
 *
 * We use a three-part notation: `category:action:target`
 *
 * - **category**: The broad area (fs, net, proc, env, ffi)
 * - **action**: What's being done (read, write, delete, exec, connect, etc.)
 * - **target**: The specific resource (a file path, env var name, or * for unknown)
 *
 * Examples:
 * - `fs:read:/etc/passwd` — reading a specific file
 * - `net:connect:*` — connecting to an unspecified network target
 * - `proc:exec:*` — executing a child process
 * - `env:read:HOME` — reading the HOME environment variable
 *
 * ## How Detection Works
 *
 * We detect capabilities at two levels:
 *
 * 1. **Import-level**: When code imports a module like `fs` or `net`, we know it
 *    *intends* to use those capabilities, even before seeing specific calls.
 *
 * 2. **Call-level**: When code calls `fs.readFileSync("x")`, we can determine the
 *    specific action (read) and sometimes the specific target (x).
 *
 * This two-level approach catches both explicit usage and latent capability.
 *
 * ## Banned Constructs
 *
 * Some patterns are inherently unsafe because they allow arbitrary code execution
 * or make static analysis impossible. We flag these separately:
 *
 * - `eval(...)` — executes arbitrary strings as code
 * - `new Function(...)` — creates functions from strings
 * - Dynamic `require()` — loads modules determined at runtime
 * - Dynamic `import()` — same, but for ESM
 * - `Reflect.apply()` / `Reflect.construct()` — meta-programming escape hatches
 */

import ts from "typescript";

// ============================================================================
// Types
// ============================================================================

/**
 * A detected capability represents one place in the source code where an OS
 * capability is used. Think of it like a permission request — "this code at
 * line 42 wants to read from the filesystem."
 */
export interface DetectedCapability {
  /** The broad category: fs, net, proc, env, ffi */
  category: string;
  /** The specific action: read, write, delete, exec, connect, etc. Use * for general */
  action: string;
  /** The specific target (file path, env var, etc). Use * when unknown */
  target: string;
  /** The source file where this capability was detected */
  file: string;
  /** The line number (1-based) where it was detected */
  line: number;
  /** A human-readable snippet showing what triggered detection */
  evidence: string;
}

/**
 * A banned construct is a pattern that makes static analysis impossible or
 * enables arbitrary code execution. These should be flagged as errors, not
 * just capability usage.
 */
export interface BannedConstruct {
  /** What kind of banned pattern: eval, new-function, dynamic-require, dynamic-import, reflect */
  kind: string;
  /** The source file */
  file: string;
  /** The line number (1-based) */
  line: number;
  /** A human-readable description of what was found */
  evidence: string;
}

/**
 * The complete result of analyzing one or more source files.
 */
export interface AnalysisResult {
  /** All detected OS capability usages */
  capabilities: DetectedCapability[];
  /** All banned constructs found */
  banned: BannedConstruct[];
}

// ============================================================================
// Module-to-Capability Mapping
// ============================================================================

/**
 * ## Module Mapping Table
 *
 * This table maps Node.js built-in module names to their capability categories.
 * When we see `import fs from "fs"`, we look up "fs" in this table to determine
 * that it implies filesystem capabilities.
 *
 * Some modules map to a general category (fs:*:*), while importing specific
 * named exports can give us more precision (e.g., `readFileSync` -> fs:read:*).
 *
 * | Module          | Category | Notes                              |
 * |-----------------|----------|------------------------------------|
 * | fs              | fs       | Filesystem operations              |
 * | fs/promises     | fs       | Async filesystem operations        |
 * | path            | fs       | Path manipulation implies fs usage |
 * | net             | net      | Low-level networking               |
 * | http            | net      | HTTP client/server                 |
 * | https           | net      | HTTPS client/server                |
 * | dgram           | net      | UDP sockets                        |
 * | dns             | net      | DNS lookups                        |
 * | tls             | net      | TLS/SSL                            |
 * | child_process   | proc     | Spawning child processes           |
 * | os              | env      | OS information / environment       |
 * | process         | env      | Process environment                |
 * | ffi-napi        | ffi      | Foreign function interface         |
 * | node-ffi        | ffi      | Foreign function interface          |
 */
const MODULE_CATEGORY_MAP: Record<string, string> = {
  fs: "fs",
  "fs/promises": "fs",
  path: "fs",
  net: "net",
  http: "net",
  https: "net",
  dgram: "net",
  dns: "net",
  tls: "net",
  child_process: "proc",
  os: "env",
  process: "env",
  "ffi-napi": "ffi",
  "node-ffi": "ffi",
};

/**
 * ## Named Import Action Mapping
 *
 * When code imports a specific function by name, we can infer the action
 * more precisely than when it imports the whole module.
 *
 * For example:
 * - `import { readFileSync } from "fs"` -> we know it's a read operation
 * - `import fs from "fs"` -> we only know it's fs-related (action: *)
 *
 * This table maps function names to their specific actions.
 */
const NAMED_IMPORT_ACTION_MAP: Record<string, string> = {
  // fs read operations
  readFileSync: "read",
  readFile: "read",
  createReadStream: "read",
  readdir: "list",
  readdirSync: "list",
  // fs write operations
  writeFileSync: "write",
  writeFile: "write",
  appendFileSync: "write",
  appendFile: "write",
  createWriteStream: "write",
  // fs delete operations
  unlinkSync: "delete",
  unlink: "delete",
  rmSync: "delete",
  rm: "delete",
  rmdirSync: "delete",
  rmdir: "delete",
  // fs create operations
  mkdirSync: "create",
  mkdir: "create",
  // process execution
  exec: "exec",
  execSync: "exec",
  spawn: "exec",
  spawnSync: "exec",
  execFile: "exec",
  execFileSync: "exec",
  fork: "exec",
};

/**
 * ## Network Module Action Mapping
 *
 * For network modules (http, https, net), imports generally imply connection
 * capability. We use "connect" as the default action for these.
 */
const NETWORK_MODULES = new Set(["http", "https", "net", "dgram", "dns", "tls"]);

// ============================================================================
// Call Pattern Detection
// ============================================================================

/**
 * ## Method-to-Action Mapping for Call Detection
 *
 * When we see `fs.readFileSync(...)`, we need to map `readFileSync` to
 * the `read` action. This table provides that mapping for property access
 * calls (obj.method(...) patterns).
 *
 * The keys are method names; values are [category, action] pairs.
 * The category here is a hint — we also verify the object matches.
 */
const METHOD_ACTION_MAP: Record<string, [string, string]> = {
  // fs read
  readFileSync: ["fs", "read"],
  readFile: ["fs", "read"],
  createReadStream: ["fs", "read"],
  // fs list
  readdirSync: ["fs", "list"],
  readdir: ["fs", "list"],
  // fs write
  writeFileSync: ["fs", "write"],
  writeFile: ["fs", "write"],
  appendFileSync: ["fs", "write"],
  appendFile: ["fs", "write"],
  createWriteStream: ["fs", "write"],
  // fs delete
  unlinkSync: ["fs", "delete"],
  unlink: ["fs", "delete"],
  rmSync: ["fs", "delete"],
  rm: ["fs", "delete"],
  rmdirSync: ["fs", "delete"],
  rmdir: ["fs", "delete"],
  // fs create
  mkdirSync: ["fs", "create"],
  mkdir: ["fs", "create"],
  // process exec
  exec: ["proc", "exec"],
  execSync: ["proc", "exec"],
  spawn: ["proc", "exec"],
  spawnSync: ["proc", "exec"],
  execFile: ["proc", "exec"],
  execFileSync: ["proc", "exec"],
  fork: ["proc", "exec"],
  // network
  createConnection: ["net", "connect"],
  connect: ["net", "connect"],
  createServer: ["net", "listen"],
  request: ["net", "connect"],
  get: ["net", "connect"],
};

/**
 * Modules whose methods should be checked against METHOD_ACTION_MAP.
 * We track which identifiers were bound to which modules during import
 * analysis, then use those bindings when analyzing call expressions.
 */
const CAPABILITY_MODULES = new Set([
  "fs",
  "fs/promises",
  "net",
  "http",
  "https",
  "child_process",
  "dgram",
  "dns",
  "tls",
]);

// ============================================================================
// AST Analysis Engine
// ============================================================================

/**
 * Tracks which local variable names are bound to which Node.js modules.
 *
 * For example, after `import fs from "fs"` or `const fs = require("fs")`,
 * we record `{ "fs": "fs" }`. After `import * as cp from "child_process"`,
 * we record `{ "cp": "child_process" }`.
 *
 * This lets us resolve calls like `cp.exec(...)` back to `child_process.exec`.
 */
type ModuleBindings = Map<string, string>;

/**
 * ## analyzeSource — The Main Entry Point
 *
 * Analyzes a single source file's text content and returns all detected
 * capabilities and banned constructs.
 *
 * ### How it works, step by step:
 *
 * 1. Parse the source text into an AST using TypeScript's compiler API
 * 2. Walk every node in the AST recursively
 * 3. For each node, check if it's an import, a function call, a property
 *    access, or a banned construct
 * 4. Collect all findings into a result object
 *
 * @param source - The source code text to analyze
 * @param filename - The filename (used for reporting, and to set the parse language)
 * @returns An AnalysisResult with all detected capabilities and banned constructs
 *
 * @example
 * ```ts
 * const result = analyzeSource(
 *   `import fs from "fs"; fs.readFileSync("/etc/passwd");`,
 *   "example.ts"
 * );
 * // result.capabilities will contain:
 * //   { category: "fs", action: "*", target: "*", ... }  (from the import)
 * //   { category: "fs", action: "read", target: "/etc/passwd", ... }  (from the call)
 * ```
 */
export function analyzeSource(source: string, filename: string): AnalysisResult {
  /**
   * Step 1: Parse the source into an AST.
   *
   * `ts.createSourceFile` does the heavy lifting. We pass:
   * - filename: helps TypeScript decide JSX handling, etc.
   * - source: the actual code text
   * - ScriptTarget.Latest: parse with the latest syntax support
   * - true: "setParentNodes" — we need parent references for context
   */
  const sourceFile = ts.createSourceFile(
    filename,
    source,
    ts.ScriptTarget.Latest,
    /* setParentNodes */ true,
  );

  const capabilities: DetectedCapability[] = [];
  const banned: BannedConstruct[] = [];

  /**
   * moduleBindings tracks which local names map to which Node modules.
   * This is populated by import/require analysis and consumed by call analysis.
   */
  const moduleBindings: ModuleBindings = new Map();

  /**
   * Step 2: Walk the AST recursively.
   *
   * TypeScript's `forEachChild` visits each direct child of a node.
   * We call it recursively to visit the entire tree — a depth-first traversal.
   *
   * For each node, we check multiple patterns in sequence. A single node
   * can trigger multiple detections (e.g., `require("fs")` is both an
   * import detection and potentially a banned construct if the argument
   * is non-literal).
   */
  function visit(node: ts.Node): void {
    // Check for import declarations: `import ... from "module"`
    checkImportDeclaration(node, sourceFile, filename, capabilities, moduleBindings);

    // Check for require() calls: `const x = require("module")`
    checkRequireCall(node, sourceFile, filename, capabilities, banned, moduleBindings);

    // Check for capability-using function calls: `fs.readFileSync(...)`
    checkCapabilityCall(node, sourceFile, filename, capabilities, moduleBindings);

    // Check for process.env access: `process.env.KEY`
    checkProcessEnv(node, sourceFile, filename, capabilities);

    // Check for fetch() calls: `fetch(url)`
    checkFetchCall(node, sourceFile, filename, capabilities);

    // Check for banned constructs: eval, new Function, Reflect, dynamic import
    checkBannedConstructs(node, sourceFile, filename, banned);

    // Recurse into children
    ts.forEachChild(node, visit);
  }

  visit(sourceFile);

  return { capabilities, banned };
}

/**
 * ## analyzeFiles — Multi-file Analysis
 *
 * Analyzes multiple source file contents at once, merging all results.
 * This is useful for analyzing an entire project.
 *
 * @param files - An array of { filename, source } objects to analyze
 * @returns A merged AnalysisResult
 */
export function analyzeFiles(
  files: Array<{ filename: string; source: string }>,
): AnalysisResult {
  const allCapabilities: DetectedCapability[] = [];
  const allBanned: BannedConstruct[] = [];

  for (const { filename, source } of files) {
    const result = analyzeSource(source, filename);
    allCapabilities.push(...result.capabilities);
    allBanned.push(...result.banned);
  }

  return { capabilities: allCapabilities, banned: allBanned };
}

// ============================================================================
// Import Detection
// ============================================================================

/**
 * ## Import Declaration Detection
 *
 * Handles ES module imports:
 * - `import fs from "fs"`           — default import
 * - `import * as fs from "fs"`      — namespace import
 * - `import { readFileSync } from "fs"` — named import
 *
 * ### AST Structure for `import fs from "fs"`:
 *
 * ```
 * ImportDeclaration
 *   ├── ImportClause
 *   │   └── Identifier "fs"          (default import)
 *   └── StringLiteral "fs"           (module specifier)
 * ```
 *
 * ### AST Structure for `import { readFileSync } from "fs"`:
 *
 * ```
 * ImportDeclaration
 *   ├── ImportClause
 *   │   └── NamedBindings
 *   │       └── NamedImports
 *   │           └── ImportSpecifier
 *   │               └── Identifier "readFileSync"
 *   └── StringLiteral "fs"
 * ```
 */
function checkImportDeclaration(
  node: ts.Node,
  sourceFile: ts.SourceFile,
  filename: string,
  capabilities: DetectedCapability[],
  moduleBindings: ModuleBindings,
): void {
  if (!ts.isImportDeclaration(node)) return;

  // The module specifier is the string after "from", e.g., "fs" in `import fs from "fs"`
  const moduleSpecifier = node.moduleSpecifier;
  if (!ts.isStringLiteral(moduleSpecifier)) return;

  const moduleName = moduleSpecifier.text;
  const category = MODULE_CATEGORY_MAP[moduleName];

  // If this isn't a module we track, skip it
  if (!category) return;

  const line = sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile)).line + 1;
  const importClause = node.importClause;

  if (!importClause) {
    // Side-effect import: `import "fs"` — rare but possible
    capabilities.push({
      category,
      action: "*",
      target: "*",
      file: filename,
      line,
      evidence: `import "${moduleName}"`,
    });
    return;
  }

  // Default import: `import fs from "fs"`
  if (importClause.name) {
    const localName = importClause.name.text;
    moduleBindings.set(localName, moduleName);

    const action = NETWORK_MODULES.has(moduleName) ? "connect" : "*";
    capabilities.push({
      category,
      action,
      target: "*",
      file: filename,
      line,
      evidence: `import ${localName} from "${moduleName}"`,
    });
  }

  // Namespace or named imports
  const namedBindings = importClause.namedBindings;
  if (namedBindings) {
    if (ts.isNamespaceImport(namedBindings)) {
      // `import * as fs from "fs"`
      const localName = namedBindings.name.text;
      moduleBindings.set(localName, moduleName);

      const action = NETWORK_MODULES.has(moduleName) ? "connect" : "*";
      capabilities.push({
        category,
        action,
        target: "*",
        file: filename,
        line,
        evidence: `import * as ${localName} from "${moduleName}"`,
      });
    } else if (ts.isNamedImports(namedBindings)) {
      // `import { readFileSync, writeFile } from "fs"`
      for (const specifier of namedBindings.elements) {
        const importedName = (specifier.propertyName || specifier.name).text;
        const localName = specifier.name.text;

        // Track the binding so we can detect calls like readFileSync(...)
        moduleBindings.set(localName, moduleName);

        const action = NAMED_IMPORT_ACTION_MAP[importedName] ??
          (NETWORK_MODULES.has(moduleName) ? "connect" : "*");

        capabilities.push({
          category,
          action,
          target: "*",
          file: filename,
          line,
          evidence: `import { ${importedName} } from "${moduleName}"`,
        });
      }
    }
  }
}

// ============================================================================
// Require Detection
// ============================================================================

/**
 * ## Require Call Detection
 *
 * Handles CommonJS requires:
 * - `const fs = require("fs")`
 * - `const { exec } = require("child_process")`
 * - `require("fs")` (standalone)
 *
 * Also detects **dynamic requires** where the argument is not a string literal,
 * which is a banned construct because it makes static analysis impossible.
 *
 * ### AST Structure for `const fs = require("fs")`:
 *
 * ```
 * VariableStatement
 *   └── VariableDeclarationList
 *       └── VariableDeclaration
 *           ├── Identifier "fs"
 *           └── CallExpression
 *               ├── Identifier "require"
 *               └── StringLiteral "fs"
 * ```
 */
function checkRequireCall(
  node: ts.Node,
  sourceFile: ts.SourceFile,
  filename: string,
  capabilities: DetectedCapability[],
  banned: BannedConstruct[],
  moduleBindings: ModuleBindings,
): void {
  if (!ts.isCallExpression(node)) return;
  if (!ts.isIdentifier(node.expression) || node.expression.text !== "require") return;

  const line = sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile)).line + 1;

  // Check if the argument is a string literal
  if (node.arguments.length === 0) return;

  const arg = node.arguments[0];
  if (!ts.isStringLiteral(arg)) {
    // Dynamic require — banned because we can't statically analyze what's being loaded
    banned.push({
      kind: "dynamic-require",
      file: filename,
      line,
      evidence: `require(${arg.getText(sourceFile)})`,
    });
    return;
  }

  const moduleName = arg.text;
  const category = MODULE_CATEGORY_MAP[moduleName];
  if (!category) return;

  // Try to find the variable binding: `const fs = require("fs")`
  // The parent chain is: CallExpression -> VariableDeclaration -> ...
  const parent = node.parent;
  if (parent && ts.isVariableDeclaration(parent)) {
    const binding = parent.name;

    if (ts.isIdentifier(binding)) {
      // Simple binding: `const fs = require("fs")`
      moduleBindings.set(binding.text, moduleName);
    } else if (ts.isObjectBindingPattern(binding)) {
      // Destructured: `const { exec } = require("child_process")`
      for (const element of binding.elements) {
        if (ts.isBindingElement(element) && ts.isIdentifier(element.name)) {
          const importedName = element.propertyName && ts.isIdentifier(element.propertyName)
            ? element.propertyName.text
            : element.name.text;
          moduleBindings.set(element.name.text, moduleName);

          const action = NAMED_IMPORT_ACTION_MAP[importedName] ??
            (NETWORK_MODULES.has(moduleName) ? "connect" : "*");

          capabilities.push({
            category,
            action,
            target: "*",
            file: filename,
            line,
            evidence: `const { ${importedName} } = require("${moduleName}")`,
          });
        }
      }
      return; // Already pushed individual capabilities above
    }
  }

  // Push a general capability for the module import
  const action = NETWORK_MODULES.has(moduleName) ? "connect" : "*";
  capabilities.push({
    category,
    action,
    target: "*",
    file: filename,
    line,
    evidence: `require("${moduleName}")`,
  });
}

// ============================================================================
// Function Call Detection
// ============================================================================

/**
 * ## Capability Call Detection
 *
 * Detects function calls that use OS capabilities:
 * - `fs.readFileSync("x")` — property access call
 * - `readFileSync("x")` — direct call (if imported by name)
 *
 * ### Extracting the Target
 *
 * When a function call has a string literal as its first argument, we can
 * extract it as the "target" of the capability. For example:
 *
 * - `fs.readFileSync("/etc/passwd")` → target is "/etc/passwd"
 * - `fs.readFileSync(variable)` → target is "*" (unknown)
 *
 * This lets security reviewers see exactly which files/resources the code
 * touches.
 */
function checkCapabilityCall(
  node: ts.Node,
  sourceFile: ts.SourceFile,
  filename: string,
  capabilities: DetectedCapability[],
  moduleBindings: ModuleBindings,
): void {
  if (!ts.isCallExpression(node)) return;

  const line = sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile)).line + 1;

  // Pattern 1: `obj.method(...)` where obj is a bound module identifier
  if (ts.isPropertyAccessExpression(node.expression)) {
    const obj = node.expression.expression;
    const method = node.expression.name.text;

    if (ts.isIdentifier(obj)) {
      const moduleName = moduleBindings.get(obj.text);
      if (moduleName && CAPABILITY_MODULES.has(moduleName)) {
        const mapping = METHOD_ACTION_MAP[method];
        if (mapping) {
          const [category, action] = mapping;
          const target = extractFirstStringArg(node, sourceFile);

          capabilities.push({
            category,
            action,
            target,
            file: filename,
            line,
            evidence: `${obj.text}.${method}(${target !== "*" ? `"${target}"` : "..."})`,
          });
        }
      }
    }
  }

  // Pattern 2: Direct call `readFileSync(...)` where readFileSync is a named import
  if (ts.isIdentifier(node.expression)) {
    const funcName = node.expression.text;
    const moduleName = moduleBindings.get(funcName);

    if (moduleName && CAPABILITY_MODULES.has(moduleName)) {
      const mapping = METHOD_ACTION_MAP[funcName];
      if (mapping) {
        const [category, action] = mapping;
        const target = extractFirstStringArg(node, sourceFile);

        capabilities.push({
          category,
          action,
          target,
          file: filename,
          line,
          evidence: `${funcName}(${target !== "*" ? `"${target}"` : "..."})`,
        });
      }
    }
  }
}

/**
 * ## Extracting String Arguments
 *
 * Many capability-using functions take a file path or URL as their first
 * argument. If that argument is a string literal, we can extract it as
 * the "target" of the capability.
 *
 * If the argument is a variable, expression, or template literal, we
 * return "*" to indicate an unknown target.
 */
function extractFirstStringArg(call: ts.CallExpression, sourceFile: ts.SourceFile): string {
  if (call.arguments.length === 0) return "*";

  const firstArg = call.arguments[0];
  if (ts.isStringLiteral(firstArg)) {
    return firstArg.text;
  }
  if (ts.isNoSubstitutionTemplateLiteral(firstArg)) {
    return firstArg.text;
  }

  return "*";
}

// ============================================================================
// process.env Detection
// ============================================================================

/**
 * ## Process Environment Detection
 *
 * Detects access to environment variables via `process.env`:
 *
 * - `process.env.HOME` → env:read:HOME (property access)
 * - `process.env["SECRET_KEY"]` → env:read:SECRET_KEY (element access)
 * - `process.env[variable]` → env:read:* (dynamic access)
 *
 * ### AST Structure for `process.env.HOME`:
 *
 * ```
 * PropertyAccessExpression
 *   ├── PropertyAccessExpression
 *   │   ├── Identifier "process"
 *   │   └── Identifier "env"
 *   └── Identifier "HOME"
 * ```
 *
 * We look for a chain of property accesses where the root is "process"
 * and the second level is "env".
 */
function checkProcessEnv(
  node: ts.Node,
  sourceFile: ts.SourceFile,
  filename: string,
  capabilities: DetectedCapability[],
): void {
  const line = sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile)).line + 1;

  // Pattern 1: process.env.KEY
  if (ts.isPropertyAccessExpression(node)) {
    const inner = node.expression;
    if (
      ts.isPropertyAccessExpression(inner) &&
      ts.isIdentifier(inner.expression) &&
      inner.expression.text === "process" &&
      inner.name.text === "env"
    ) {
      const key = node.name.text;
      capabilities.push({
        category: "env",
        action: "read",
        target: key,
        file: filename,
        line,
        evidence: `process.env.${key}`,
      });
    }
  }

  // Pattern 2: process.env["KEY"] or process.env[variable]
  if (ts.isElementAccessExpression(node)) {
    const inner = node.expression;
    if (
      ts.isPropertyAccessExpression(inner) &&
      ts.isIdentifier(inner.expression) &&
      inner.expression.text === "process" &&
      inner.name.text === "env"
    ) {
      const argExpr = node.argumentExpression;
      const key = ts.isStringLiteral(argExpr) ? argExpr.text : "*";
      capabilities.push({
        category: "env",
        action: "read",
        target: key,
        file: filename,
        line,
        evidence: key !== "*" ? `process.env["${key}"]` : `process.env[${argExpr.getText(sourceFile)}]`,
      });
    }
  }
}

// ============================================================================
// Fetch Detection
// ============================================================================

/**
 * ## Fetch Call Detection
 *
 * The global `fetch()` function (available in Node 18+) is a network capability.
 *
 * - `fetch("https://api.example.com")` → net:connect:https://api.example.com
 * - `fetch(url)` → net:connect:*
 */
function checkFetchCall(
  node: ts.Node,
  sourceFile: ts.SourceFile,
  filename: string,
  capabilities: DetectedCapability[],
): void {
  if (!ts.isCallExpression(node)) return;
  if (!ts.isIdentifier(node.expression) || node.expression.text !== "fetch") return;

  const line = sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile)).line + 1;
  const target = extractFirstStringArg(node, sourceFile);

  capabilities.push({
    category: "net",
    action: "connect",
    target,
    file: filename,
    line,
    evidence: `fetch(${target !== "*" ? `"${target}"` : "..."})`,
  });
}

// ============================================================================
// Banned Construct Detection
// ============================================================================

/**
 * ## Banned Construct Detection
 *
 * Some constructs make static analysis impossible or enable arbitrary code
 * execution. These are flagged separately from capabilities because they
 * represent fundamental security risks.
 *
 * ### Truth Table of Banned Patterns
 *
 * | Pattern                          | Kind            | Why it's banned                    |
 * |----------------------------------|-----------------|------------------------------------|
 * | `eval("code")`                   | eval            | Executes arbitrary code            |
 * | `new Function("code")`           | new-function    | Creates function from string       |
 * | `require(variable)`              | dynamic-require | Unknown module loaded at runtime   |
 * | `import(variable)`               | dynamic-import  | Unknown module loaded at runtime   |
 * | `Reflect.apply(fn, ...)`         | reflect         | Meta-programming escape hatch      |
 * | `Reflect.construct(Cls, ...)`    | reflect         | Meta-programming escape hatch      |
 *
 * Note: `eval` and `new Function` are also caught by ESLint's `no-eval` and
 * `no-new-func` rules. We detect them here for completeness — our analyzer
 * can run without ESLint.
 */
function checkBannedConstructs(
  node: ts.Node,
  sourceFile: ts.SourceFile,
  filename: string,
  banned: BannedConstruct[],
): void {
  if (!ts.isCallExpression(node) && !ts.isNewExpression(node)) return;

  const line = sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile)).line + 1;

  // --- eval(...) ---
  if (ts.isCallExpression(node) && ts.isIdentifier(node.expression) && node.expression.text === "eval") {
    banned.push({
      kind: "eval",
      file: filename,
      line,
      evidence: "eval(...)",
    });
    return;
  }

  // --- new Function(...) ---
  if (ts.isNewExpression(node) && ts.isIdentifier(node.expression) && node.expression.text === "Function") {
    banned.push({
      kind: "new-function",
      file: filename,
      line,
      evidence: "new Function(...)",
    });
    return;
  }

  // --- dynamic import(...) ---
  // In the AST, dynamic import() is a CallExpression with an ImportKeyword token
  if (ts.isCallExpression(node) && node.expression.kind === ts.SyntaxKind.ImportKeyword) {
    if (node.arguments.length > 0 && !ts.isStringLiteral(node.arguments[0])) {
      banned.push({
        kind: "dynamic-import",
        file: filename,
        line,
        evidence: `import(${node.arguments[0].getText(sourceFile)})`,
      });
    }
    return;
  }

  // --- Reflect.apply(...) and Reflect.construct(...) ---
  if (ts.isCallExpression(node) && ts.isPropertyAccessExpression(node.expression)) {
    const obj = node.expression.expression;
    const method = node.expression.name.text;

    if (ts.isIdentifier(obj) && obj.text === "Reflect") {
      if (method === "apply" || method === "construct") {
        banned.push({
          kind: "reflect",
          file: filename,
          line,
          evidence: `Reflect.${method}(...)`,
        });
      }
    }
  }
}
