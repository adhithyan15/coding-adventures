/**
 * Core static analysis engine inspecting AST for ambient capabilities and VER15 CWEs.
 */

import * as fs from "node:fs";
import * as path from "node:path";
import ts from "typescript";

import type {
  AmbientCapability,
  CWEId,
  DetectedCapability,
  DetectedWeakness,
  AnalysisOptions,
  AnalysisResult,
} from "./types.js";
import {
  CAPABILITY_RULES,
  FS_READ_METHODS,
  FS_WRITE_METHODS,
  NET_MODULES,
  PROCESS_METHODS,
  BANNED_PROTO_PROPERTIES,
  isCatastrophicRegex,
} from "./rules.js";
import { loadManifest } from "./manifest.js";
import { discoverSourceFiles } from "./scanner.js";

export const MAX_SOURCE_BYTES = 10_485_760;

export interface FileAnalysisResult {
  readonly capabilities: DetectedCapability[];
  readonly weaknesses: DetectedWeakness[];
}

export function analyzeSource(sourceText: string, filePath: string = "source.ts"): FileAnalysisResult {
  if (typeof sourceText !== "string") {
    throw new TypeError("sourceText must be a string");
  }
  if (typeof filePath !== "string" || filePath.length === 0 || filePath.includes("\0")) {
    throw new TypeError("filePath must be a non-empty string without NUL bytes");
  }
  if (
    sourceText.length > MAX_SOURCE_BYTES ||
    (sourceText.length > Math.floor(MAX_SOURCE_BYTES / 3) &&
      Buffer.byteLength(sourceText, "utf8") > MAX_SOURCE_BYTES)
  ) {
    throw new RangeError(
      `sourceText exceeds maximum size of ${MAX_SOURCE_BYTES} bytes`
    );
  }

  let sourceFile: ts.SourceFile;
  try {
    sourceFile = ts.createSourceFile(
      filePath,
      sourceText,
      ts.ScriptTarget.Latest,
      true
    );
  } catch (err) {
    return {
      capabilities: [],
      weaknesses: [
        {
          cwe: "CWE-674",
          name: "Uncontrolled Recursion / Parser Stack Exhaustion",
          file: filePath,
          line: 1,
          col: 1,
          evidence: "Deeply nested source code",
          description: `Parser failed due to recursion stack exhaustion: ${String(err)}`,
        },
      ],
    };
  }

  const capabilities: DetectedCapability[] = [];
  const weaknesses: DetectedWeakness[] = [];

  // Parse lines to support nolint comments
  const lines = sourceText.split(/\r?\n/);
  const nolintLines = new Map<number, Set<string>>();

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const match = /\/\/\s*nolint(?::([a-zA-Z0-9_-]+))?/.exec(line);
    if (match) {
      const lineNum = i + 1;
      const rawTag = match[1];
      const tag = rawTag && rawTag.length <= 64 ? rawTag.toLowerCase() : "all";
      if (!nolintLines.has(lineNum)) {
        nolintLines.set(lineNum, new Set());
      }
      nolintLines.get(lineNum)!.add(tag);
    }
  }

  function isLineSuppressed(line: number, category: string): boolean {
    const tags = nolintLines.get(line);
    if (!tags) return false;
    return tags.has("all") || tags.has(category.toLowerCase());
  }

  function getLineAndCol(node: ts.Node): { line: number; col: number } {
    const { line, character } = sourceFile.getLineAndCharacterOfPosition(node.getStart());
    return { line: line + 1, col: character + 1 };
  }

  function addCap(cap: AmbientCapability, node: ts.Node, evidence: string): void {
    const { line, col } = getLineAndCol(node);
    if (isLineSuppressed(line, "cap") || isLineSuppressed(line, cap)) {
      return;
    }
    const rule = CAPABILITY_RULES[cap];
    capabilities.push({
      capability: cap,
      category: rule.category,
      action: rule.action,
      file: filePath,
      line,
      col,
      evidence,
    });
  }

  function addWeakness(cwe: CWEId, name: string, node: ts.Node, evidence: string, description: string): void {
    const { line, col } = getLineAndCol(node);
    if (isLineSuppressed(line, cwe) || isLineSuppressed(line, "cwe")) {
      return;
    }
    weaknesses.push({
      cwe,
      name,
      file: filePath,
      line,
      col,
      evidence,
      description,
    });
  }

  // Track function scope stack to identify recursion without depth guards (CWE-674)
  interface FunctionScope {
    name?: string;
    params: string[];
    hasDepthGuard: boolean;
    selfCalls: ts.CallExpression[];
  }
  const scopeStack: FunctionScope[] = [];

  // Track module imports in this file
  const importedModules = new Map<string, string>();

  const MAX_AST_DEPTH = 512;
  const MAX_AST_NODES = 250_000;

  interface WorkItem {
    node: ts.Node;
    depth: number;
    isExit?: boolean;
    pushedScope?: boolean;
  }

  const worklist: WorkItem[] = [{ node: sourceFile, depth: 0 }];
  let nodesVisited = 0;

  while (worklist.length > 0) {
    const item = worklist.pop()!;

    if (item.isExit) {
      if (item.pushedScope) {
        const scope = scopeStack.pop()!;
        if (scope.name && scope.selfCalls.length > 0 && !scope.hasDepthGuard) {
          for (const call of scope.selfCalls) {
            addWeakness(
              "CWE-674",
              "Uncontrolled Recursion",
              call,
              call.getText(sourceFile),
              `Recursive call to '${scope.name}' without explicit depth guard or step budget parameter`
            );
          }
        }
      }
      continue;
    }

    const node = item.node;
    nodesVisited++;
    if (nodesVisited > MAX_AST_NODES) {
      addWeakness(
        "CWE-400",
        "Resource Consumption (AST node quota exceeded)",
        node,
        node.getText(sourceFile),
        `Exceeded maximum AST node inspection quota (${MAX_AST_NODES})`
      );
      break;
    }

    if (item.depth > MAX_AST_DEPTH) {
      addWeakness(
        "CWE-674",
        "Uncontrolled Recursion (AST depth quota exceeded)",
        node,
        node.getText(sourceFile),
        `AST traversal depth (${item.depth}) exceeded maximum allowed limit of ${MAX_AST_DEPTH}`
      );
      continue;
    }
    // ── 1. Import Declarations ──────────────────────────────────────────────
    if (ts.isImportDeclaration(node)) {
      const moduleSpecifier = node.moduleSpecifier;
      if (ts.isStringLiteral(moduleSpecifier)) {
        const modName = moduleSpecifier.text;
        if (node.importClause) {
          if (node.importClause.name) {
            importedModules.set(node.importClause.name.text, modName);
          }
          if (node.importClause.namedBindings) {
            if (ts.isNamespaceImport(node.importClause.namedBindings)) {
              importedModules.set(node.importClause.namedBindings.name.text, modName);
            } else if (ts.isNamedImports(node.importClause.namedBindings)) {
              for (const element of node.importClause.namedBindings.elements) {
                const importedName = element.propertyName ? element.propertyName.text : element.name.text;
                const localName = element.name.text;
                importedModules.set(localName, `${modName}:${importedName}`);
              }
            }
          }
        }
        if (modName === "fs" || modName === "node:fs" || modName.startsWith("node:fs/")) {
          // fs read is ambient baseline for fs imports
          addCap("fs:read", node, `import from '${modName}'`);
        } else if (NET_MODULES.has(modName)) {
          addCap("net:socket", node, `import from '${modName}'`);
        } else if (modName === "child_process" || modName === "node:child_process") {
          addCap("process:spawn", node, `import from '${modName}'`);
        } else if (modName === "ffi-napi" || modName === "koffi" || modName === "bun:ffi") {
          addCap("ffi:unsafe", node, `import native FFI from '${modName}'`);
        }
      }
    }

    // ── 2. Call Expressions ────────────────────────────────────────────────
    if (ts.isCallExpression(node)) {
      const expression = node.expression;
      const text = expression.getText(sourceFile);

      // Check current function self-call for recursion
      const currentScope = scopeStack[scopeStack.length - 1];
      if (currentScope?.name && ts.isIdentifier(expression) && expression.text === currentScope.name) {
        currentScope.selfCalls.push(node);
      }

      // Code Injection (CWE-94)
      if (text === "eval") {
        addWeakness(
          "CWE-94",
          "Code Injection (eval)",
          node,
          node.getText(sourceFile),
          "Dynamic code evaluation via eval() allows arbitrary code execution"
        );
      }

      // Random Entropy (random:entropy)
      if (text === "Math.random" || text.endsWith(".getRandomValues") || text.endsWith(".randomBytes") || text.endsWith(".randomUUID") || text.endsWith(".randomInt")) {
        addCap("random:entropy", node, node.getText(sourceFile));
      }

      // Time (time:clock)
      if (text === "Date.now" || text === "performance.now" || text === "process.hrtime" || text === "process.uptime") {
        addCap("time:clock", node, node.getText(sourceFile));
      }

      // Network calls (net:socket)
      if (text === "fetch" || text.endsWith(".connect") || text.endsWith(".createConnection") || text.endsWith(".createServer")) {
        addCap("net:socket", node, node.getText(sourceFile));
      }

      // Direct identifier calls for fs functions (e.g. readFile, writeFile imported directly)
      if (ts.isIdentifier(expression)) {
        const fnName = expression.text;
        if (FS_WRITE_METHODS.has(fnName)) {
          addCap("fs:write", node, node.getText(sourceFile));
        } else if (FS_READ_METHODS.has(fnName) && fnName !== "open" && fnName !== "read") {
          addCap("fs:read", node, node.getText(sourceFile));
        } else if (
          fnName === "spawn" ||
          fnName === "spawnSync" ||
          fnName === "fork" ||
          fnName === "execFile" ||
          fnName === "execFileSync" ||
          (fnName === "exec" && importedModules.get("exec")?.includes("child_process")) ||
          (fnName === "execSync" && importedModules.get("execSync")?.includes("child_process"))
        ) {
          addCap("process:spawn", node, node.getText(sourceFile));
          if (fnName === "exec" || fnName === "execSync") {
            const firstArg = node.arguments[0];
            if (firstArg && !ts.isStringLiteral(firstArg)) {
              addWeakness(
                "CWE-78",
                "OS Command Injection",
                firstArg,
                firstArg.getText(sourceFile),
                "Unsanitized dynamic command execution in child_process allows command injection"
              );
            }
          }
        }
      }

      // Property access calls (e.g. child_process.exec, fs.readFile, etc.)
      if (ts.isPropertyAccessExpression(expression)) {
        const methodName = expression.name.text;
        const targetObj = expression.expression;
        const targetText = targetObj.getText(sourceFile);
        const isRegexTarget =
          ts.isRegularExpressionLiteral(targetObj) ||
          targetText.endsWith("Regex") ||
          targetText.endsWith("PATTERN") ||
          targetText.endsWith("Pattern") ||
          targetText.startsWith("regex") ||
          targetText.startsWith("re") ||
          targetText === "RegExp";

        // Process spawn (process:spawn & CWE-78)
        if (
          !isRegexTarget &&
          (methodName === "spawn" ||
            methodName === "spawnSync" ||
            methodName === "fork" ||
            methodName === "execFile" ||
            methodName === "execFileSync" ||
            ((methodName === "exec" || methodName === "execSync") &&
              (targetText === "child_process" ||
                targetText === "node:child_process" ||
                targetText === "cp" ||
                targetText === "childProcess" ||
                importedModules.get(targetText) === "child_process" ||
                importedModules.get(targetText) === "node:child_process")))
        ) {
          addCap("process:spawn", node, node.getText(sourceFile));

          // Check for command injection (CWE-78): exec/execSync/spawn with dynamic non-literal command
          if (methodName === "exec" || methodName === "execSync") {
            const firstArg = node.arguments[0];
            if (firstArg && !ts.isStringLiteral(firstArg)) {
              addWeakness(
                "CWE-78",
                "OS Command Injection",
                firstArg,
                firstArg.getText(sourceFile),
                "Unsanitized dynamic command execution in child_process allows command injection"
              );
            }
          }
        }

        // Filesystem reads/writes on property access
        if (FS_WRITE_METHODS.has(methodName)) {
          addCap("fs:write", node, node.getText(sourceFile));
        } else if (FS_READ_METHODS.has(methodName)) {
          if (methodName === "read" || methodName === "open") {
            if (
              targetText === "fs" ||
              targetText === "node:fs" ||
              targetText === "promises" ||
              importedModules.get(targetText)?.includes("fs")
            ) {
              addCap("fs:read", node, node.getText(sourceFile));
            }
          } else {
            addCap("fs:read", node, node.getText(sourceFile));
          }
        }
      }

      // FFI unsafe calls
      if (text === "process.dlopen" || text.includes(".dlopen")) {
        addCap("ffi:unsafe", node, node.getText(sourceFile));
      }
    }

    // ── 3. New Expressions ─────────────────────────────────────────────────
    if (ts.isNewExpression(node)) {
      const text = node.expression.getText(sourceFile);

      // CWE-94: new Function(...)
      if (text === "Function") {
        addWeakness(
          "CWE-94",
          "Code Injection (Function constructor)",
          node,
          node.getText(sourceFile),
          "Dynamic code evaluation via Function constructor"
        );
      }

      // CWE-1333: new RegExp(...)
      if (text === "RegExp") {
        const firstArg = node.arguments?.[0];
        if (firstArg && ts.isStringLiteral(firstArg)) {
          if (isCatastrophicRegex(firstArg.text)) {
            addWeakness(
              "CWE-1333",
              "Inefficient Regular Expression (ReDoS)",
              node,
              node.getText(sourceFile),
              `RegExp pattern '${firstArg.text}' exhibits catastrophic backtracking`
            );
          }
        } else if (firstArg && !ts.isStringLiteral(firstArg)) {
          // Dynamic RegExp from variable / expression
          addWeakness(
            "CWE-1333",
            "Unvalidated Dynamic RegExp (ReDoS)",
            node,
            node.getText(sourceFile),
            "Constructing RegExp from unvalidated dynamic variable can lead to ReDoS"
          );
        }
      }

      // Time clock: new Date()
      if (text === "Date") {
        addCap("time:clock", node, node.getText(sourceFile));
      }

      // Network socket: new WebSocket(...), new EventSource(...)
      if (text === "WebSocket" || text === "EventSource") {
        addCap("net:socket", node, node.getText(sourceFile));
      }
    }

    // ── 4. Regex Literals (CWE-1333) ───────────────────────────────────────
    if (node.kind === ts.SyntaxKind.RegularExpressionLiteral) {
      const regexText = node.getText(sourceFile);
      const patternMatch = /^\/(.*)\/[a-z]*$/.exec(regexText);
      if (patternMatch && isCatastrophicRegex(patternMatch[1])) {
        addWeakness(
          "CWE-1333",
          "Inefficient Regular Expression (ReDoS)",
          node,
          regexText,
          `Regex literal '${regexText}' contains vulnerable nested or overlapping quantifiers`
        );
      }
    }

    // ── 5. Property Access: env:read & CWE-1321 ────────────────────────────
    if (ts.isPropertyAccessExpression(node)) {
      const propName = node.name.text;
      const targetText = node.expression.getText(sourceFile);

      // Environment variables (env:read)
      if (targetText === "process" && propName === "env") {
        addCap("env:read", node, node.getText(sourceFile));
      } else if (targetText === "Deno" && propName === "env") {
        addCap("env:read", node, node.getText(sourceFile));
      } else if (targetText === "Bun" && propName === "env") {
        addCap("env:read", node, node.getText(sourceFile));
      }

      // Prototype Pollution (CWE-1321): access to __proto__, prototype, constructor
      if (BANNED_PROTO_PROPERTIES.has(propName)) {
        // Exclude class declarations or static prototype typing
        if (!ts.isClassDeclaration(node.parent) && !ts.isInterfaceDeclaration(node.parent)) {
          const parent = node.parent;
          if (ts.isBinaryExpression(parent) && parent.left === node && parent.operatorToken.kind === ts.SyntaxKind.EqualsToken) {
            addWeakness(
              "CWE-1321",
              "Prototype Pollution (property assignment)",
              node,
              parent.getText(sourceFile),
              `Direct assignment to prototype property '${propName}'`
            );
          }
        }
      }
    }

    // ── 6. Element Access & Bracket Writes (CWE-1321) ───────────────────────
    if (ts.isElementAccessExpression(node)) {
      const argExpr = node.argumentExpression;
      const targetText = node.expression.getText(sourceFile);

      // Direct bracket access with "__proto__", "constructor", "prototype"
      if (ts.isStringLiteral(argExpr) && BANNED_PROTO_PROPERTIES.has(argExpr.text)) {
        addWeakness(
          "CWE-1321",
          "Prototype Pollution (bracket access)",
          node,
          node.getText(sourceFile),
          `Explicit bracket access to dangerous prototype key '${argExpr.text}'`
        );
      }

      // Dynamic bracket write: obj[dynamicKey] = value
      const parent = node.parent;
      if (
        ts.isBinaryExpression(parent) &&
        parent.left === node &&
        parent.operatorToken.kind === ts.SyntaxKind.EqualsToken
      ) {
        // If the key is dynamic (not string literal, not number literal)
        if (!ts.isStringLiteral(argExpr) && !ts.isNumericLiteral(argExpr)) {
          // If target is not a typed Array or Map
          if (!targetText.endsWith("Array") && !targetText.endsWith("Map")) {
            addWeakness(
              "CWE-1321",
              "Prototype Pollution (dynamic bracket write)",
              parent,
              parent.getText(sourceFile),
              `Dynamic bracket write on '${targetText}[${argExpr.getText(sourceFile)}]' without key validation`
            );
          }
        }
      }
    }

    // ── 7. Function Scopes & Recursion Depth Guard (CWE-674) ───────────────
    let pushedScope = false;
    if (
      ts.isFunctionDeclaration(node) ||
      ts.isFunctionExpression(node) ||
      ts.isArrowFunction(node)
    ) {
      let funcName: string | undefined;
      if (ts.isFunctionDeclaration(node) && node.name) {
        funcName = node.name.text;
      } else if (
        ts.isVariableDeclaration(node.parent) &&
        ts.isIdentifier(node.parent.name)
      ) {
        funcName = node.parent.name.text;
      }

      const params = node.parameters.map((p) => p.name.getText(sourceFile).toLowerCase());
      const hasDepthGuard = params.some(
        (p) =>
          p.includes("depth") ||
          p.includes("budget") ||
          p.includes("limit") ||
          p.includes("quota") ||
          p.includes("step")
      );

      scopeStack.push({
        name: funcName,
        params,
        hasDepthGuard,
        selfCalls: [],
      });
      pushedScope = true;
    }

    if (pushedScope) {
      worklist.push({ node, depth: item.depth, isExit: true, pushedScope: true });
    }

    const children: ts.Node[] = [];
    ts.forEachChild(node, (child) => {
      children.push(child);
    });
    for (let i = children.length - 1; i >= 0; i--) {
      worklist.push({ node: children[i], depth: item.depth + 1 });
    }
  }

  return { capabilities, weaknesses };
}

export function analyzePackage(
  packageDir: string,
  options: AnalysisOptions = {}
): AnalysisResult {
  if (typeof packageDir !== "string" || packageDir.length === 0 || packageDir.includes("\0")) {
    throw new Error("Invalid package directory path");
  }
  if (options !== undefined && (typeof options !== "object" || options === null || Array.isArray(options))) {
    throw new TypeError("options must be an object");
  }
  const manifest = loadManifest(packageDir);
  const files = discoverSourceFiles(packageDir, { includeTests: options.includeTests });

  const allCapabilities: DetectedCapability[] = [];
  const allWeaknesses: DetectedWeakness[] = [];

  for (const file of files) {
    const content = fs.readFileSync(file, "utf-8");
    const res = analyzeSource(content, file);
    allCapabilities.push(...res.capabilities);
    allWeaknesses.push(...res.weaknesses);
  }

  // Cross-reference detected vs declared capabilities
  const undeclared: DetectedCapability[] = [];
  for (const cap of allCapabilities) {
    const isDeclared =
      manifest.declaredCapabilities.has(cap.capability) ||
      manifest.declaredCapabilities.has(`${cap.category}:${cap.action}`);
    if (!isDeclared) {
      undeclared.push(cap);
    }
  }

  // Check CWE exceptions
  const activeWeaknesses: DetectedWeakness[] = [];
  const exemptWeaknesses: DetectedWeakness[] = [];

  for (const weak of allWeaknesses) {
    const exc = manifest.exceptions.get(weak.cwe.toLowerCase());
    if (exc) {
      exemptWeaknesses.push(weak);
    } else {
      activeWeaknesses.push(weak);
    }
  }

  const violations: string[] = [];

  for (const u of undeclared) {
    violations.push(
      `[CAP001] Undeclared capability '${u.capability}' at ${path.relative(packageDir, u.file)}:${u.line}:${u.col} (${u.evidence})`
    );
  }

  if (options.strict) {
    for (const w of activeWeaknesses) {
      violations.push(
        `[${w.cwe}] ${w.name} at ${path.relative(packageDir, w.file)}:${w.line}:${w.col}: ${w.description}`
      );
    }
  }

  return {
    packageDir,
    packageName: manifest.packageName,
    inspectedFiles: files,
    detectedCapabilities: allCapabilities,
    declaredCapabilities: Array.from(manifest.declaredCapabilities),
    undeclaredCapabilities: undeclared,
    detectedWeaknesses: activeWeaknesses,
    exemptWeaknesses,
    violations,
    ok: violations.length === 0,
  };
}
