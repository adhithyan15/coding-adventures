/**
 * # Analyzer Tests
 *
 * Comprehensive tests for the capability analyzer's AST walking and detection.
 * Organized by detection category:
 *
 * 1. Import detection (ESM and CJS)
 * 2. Function call detection
 * 3. process.env detection
 * 4. fetch() detection
 * 5. Banned construct detection
 * 6. Pure code (no capabilities)
 */

import { describe, it, expect } from "vitest";
import { analyzeSource, analyzeFiles } from "../src/analyzer.js";

// ============================================================================
// Helper
// ============================================================================

/**
 * Shorthand: analyze a snippet and return capabilities.
 * Uses "test.ts" as the filename.
 */
function caps(source: string) {
  return analyzeSource(source, "test.ts").capabilities;
}

/** Shorthand: analyze a snippet and return banned constructs. */
function bans(source: string) {
  return analyzeSource(source, "test.ts").banned;
}

// ============================================================================
// Import Detection — ESM
// ============================================================================

describe("ESM import detection", () => {
  it("detects default import of fs", () => {
    const result = caps(`import fs from "fs";`);
    expect(result).toHaveLength(1);
    expect(result[0].category).toBe("fs");
    expect(result[0].action).toBe("*");
    expect(result[0].target).toBe("*");
  });

  it("detects namespace import of fs", () => {
    const result = caps(`import * as fs from "fs";`);
    expect(result).toHaveLength(1);
    expect(result[0].category).toBe("fs");
    expect(result[0].action).toBe("*");
    expect(result[0].evidence).toContain("import * as fs");
  });

  it("detects named import of readFileSync from fs", () => {
    const result = caps(`import { readFileSync } from "fs";`);
    expect(result).toHaveLength(1);
    expect(result[0].category).toBe("fs");
    expect(result[0].action).toBe("read");
  });

  it("detects named import of writeFileSync from fs", () => {
    const result = caps(`import { writeFileSync } from "fs";`);
    expect(result).toHaveLength(1);
    expect(result[0].action).toBe("write");
  });

  it("detects named import of unlinkSync from fs", () => {
    const result = caps(`import { unlinkSync } from "fs";`);
    expect(result).toHaveLength(1);
    expect(result[0].action).toBe("delete");
  });

  it("detects named import of mkdirSync from fs", () => {
    const result = caps(`import { mkdirSync } from "fs";`);
    expect(result).toHaveLength(1);
    expect(result[0].action).toBe("create");
  });

  it("detects named import of readdirSync from fs", () => {
    const result = caps(`import { readdirSync } from "fs";`);
    expect(result).toHaveLength(1);
    expect(result[0].action).toBe("list");
  });

  it("detects multiple named imports", () => {
    const result = caps(`import { readFileSync, writeFileSync } from "fs";`);
    expect(result).toHaveLength(2);
    expect(result[0].action).toBe("read");
    expect(result[1].action).toBe("write");
  });

  it("detects import of path module", () => {
    const result = caps(`import path from "path";`);
    expect(result).toHaveLength(1);
    expect(result[0].category).toBe("fs");
  });

  it("detects import of net module", () => {
    const result = caps(`import net from "net";`);
    expect(result).toHaveLength(1);
    expect(result[0].category).toBe("net");
    expect(result[0].action).toBe("connect");
  });

  it("detects import of http module", () => {
    const result = caps(`import http from "http";`);
    expect(result).toHaveLength(1);
    expect(result[0].category).toBe("net");
    expect(result[0].action).toBe("connect");
  });

  it("detects import of https module", () => {
    const result = caps(`import https from "https";`);
    expect(result).toHaveLength(1);
    expect(result[0].category).toBe("net");
    expect(result[0].action).toBe("connect");
  });

  it("detects import of child_process", () => {
    const result = caps(`import { exec } from "child_process";`);
    expect(result).toHaveLength(1);
    expect(result[0].category).toBe("proc");
    expect(result[0].action).toBe("exec");
  });

  it("detects import of os module", () => {
    const result = caps(`import os from "os";`);
    expect(result).toHaveLength(1);
    expect(result[0].category).toBe("env");
  });

  it("detects import of ffi-napi", () => {
    const result = caps(`import ffi from "ffi-napi";`);
    expect(result).toHaveLength(1);
    expect(result[0].category).toBe("ffi");
  });

  it("detects import of fs/promises", () => {
    const result = caps(`import fsp from "fs/promises";`);
    expect(result).toHaveLength(1);
    expect(result[0].category).toBe("fs");
  });

  it("ignores non-capability imports", () => {
    const result = caps(`import express from "express";`);
    expect(result).toHaveLength(0);
  });

  it("detects side-effect import", () => {
    const result = caps(`import "fs";`);
    expect(result).toHaveLength(1);
    expect(result[0].category).toBe("fs");
  });
});

// ============================================================================
// Import Detection — CJS require()
// ============================================================================

describe("CJS require detection", () => {
  it("detects simple require of fs", () => {
    const result = caps(`const fs = require("fs");`);
    expect(result).toHaveLength(1);
    expect(result[0].category).toBe("fs");
    expect(result[0].action).toBe("*");
  });

  it("detects destructured require of child_process", () => {
    const result = caps(`const { exec } = require("child_process");`);
    expect(result).toHaveLength(1);
    expect(result[0].category).toBe("proc");
    expect(result[0].action).toBe("exec");
  });

  it("detects require of http module", () => {
    const result = caps(`const http = require("http");`);
    expect(result).toHaveLength(1);
    expect(result[0].category).toBe("net");
    expect(result[0].action).toBe("connect");
  });

  it("ignores non-capability require", () => {
    const result = caps(`const lodash = require("lodash");`);
    expect(result).toHaveLength(0);
  });

  it("detects require of os module", () => {
    const result = caps(`const os = require("os");`);
    expect(result).toHaveLength(1);
    expect(result[0].category).toBe("env");
  });
});

// ============================================================================
// Function Call Detection
// ============================================================================

describe("function call detection", () => {
  it("detects fs.readFileSync with string arg", () => {
    const source = `
      import fs from "fs";
      fs.readFileSync("/etc/passwd");
    `;
    const result = caps(source);
    // One from the import, one from the call
    const calls = result.filter((c) => c.action === "read" && c.target === "/etc/passwd");
    expect(calls).toHaveLength(1);
    expect(calls[0].evidence).toContain("readFileSync");
  });

  it("detects fs.writeFileSync with string arg", () => {
    const source = `
      import fs from "fs";
      fs.writeFileSync("/tmp/output.txt", "data");
    `;
    const result = caps(source);
    const writes = result.filter((c) => c.action === "write");
    expect(writes).toHaveLength(1);
    expect(writes[0].target).toBe("/tmp/output.txt");
  });

  it("detects fs.unlinkSync", () => {
    const source = `
      import fs from "fs";
      fs.unlinkSync("/tmp/trash.txt");
    `;
    const result = caps(source);
    const deletes = result.filter((c) => c.action === "delete");
    expect(deletes).toHaveLength(1);
    expect(deletes[0].target).toBe("/tmp/trash.txt");
  });

  it("detects fs.mkdirSync", () => {
    const source = `
      import fs from "fs";
      fs.mkdirSync("/tmp/newdir");
    `;
    const result = caps(source);
    const creates = result.filter((c) => c.action === "create");
    expect(creates).toHaveLength(1);
    expect(creates[0].target).toBe("/tmp/newdir");
  });

  it("detects fs.readdirSync", () => {
    const source = `
      import fs from "fs";
      fs.readdirSync("/home");
    `;
    const result = caps(source);
    const lists = result.filter((c) => c.action === "list");
    expect(lists).toHaveLength(1);
    expect(lists[0].target).toBe("/home");
  });

  it("uses * target for non-literal argument", () => {
    const source = `
      import fs from "fs";
      const file = getPath();
      fs.readFileSync(file);
    `;
    const result = caps(source);
    const reads = result.filter((c) => c.action === "read");
    expect(reads).toHaveLength(1);
    expect(reads[0].target).toBe("*");
  });

  it("detects child_process.exec", () => {
    const source = `
      import cp from "child_process";
      cp.exec("ls -la");
    `;
    const result = caps(source);
    const execs = result.filter((c) => c.category === "proc" && c.action === "exec");
    expect(execs).toHaveLength(1);
  });

  it("detects child_process.spawn", () => {
    const source = `
      import cp from "child_process";
      cp.spawn("node", ["server.js"]);
    `;
    const result = caps(source);
    const execs = result.filter((c) => c.action === "exec" && c.evidence.includes("spawn"));
    expect(execs).toHaveLength(1);
  });

  it("detects http.request", () => {
    const source = `
      import http from "http";
      http.request("http://example.com");
    `;
    const result = caps(source);
    const connects = result.filter((c) => c.action === "connect" && c.evidence.includes("request"));
    expect(connects).toHaveLength(1);
  });

  it("detects net.createConnection", () => {
    const source = `
      import net from "net";
      net.createConnection(8080, "localhost");
    `;
    const result = caps(source);
    const connects = result.filter((c) => c.action === "connect" && c.evidence.includes("createConnection"));
    expect(connects).toHaveLength(1);
  });

  it("detects direct call of named import", () => {
    const source = `
      import { readFileSync } from "fs";
      readFileSync("/data/config.json");
    `;
    const result = caps(source);
    const reads = result.filter((c) => c.action === "read" && c.target === "/data/config.json");
    expect(reads).toHaveLength(1);
  });

  it("detects direct call of exec from named import", () => {
    const source = `
      import { exec } from "child_process";
      exec("whoami");
    `;
    const result = caps(source);
    const execs = result.filter(
      (c) => c.category === "proc" && c.action === "exec" && c.evidence.includes('exec("whoami")'),
    );
    expect(execs).toHaveLength(1);
  });

  it("detects require + method call", () => {
    const source = `
      const fs = require("fs");
      fs.readFileSync("/etc/hosts");
    `;
    const result = caps(source);
    const reads = result.filter((c) => c.action === "read" && c.target === "/etc/hosts");
    expect(reads).toHaveLength(1);
  });
});

// ============================================================================
// process.env Detection
// ============================================================================

describe("process.env detection", () => {
  it("detects process.env.HOME", () => {
    const result = caps(`const home = process.env.HOME;`);
    expect(result).toHaveLength(1);
    expect(result[0].category).toBe("env");
    expect(result[0].action).toBe("read");
    expect(result[0].target).toBe("HOME");
  });

  it("detects process.env['SECRET_KEY']", () => {
    const result = caps(`const key = process.env["SECRET_KEY"];`);
    expect(result).toHaveLength(1);
    expect(result[0].category).toBe("env");
    expect(result[0].target).toBe("SECRET_KEY");
  });

  it("detects process.env[variable] with * target", () => {
    const result = caps(`const val = process.env[varName];`);
    expect(result).toHaveLength(1);
    expect(result[0].target).toBe("*");
  });

  it("detects multiple env accesses", () => {
    const source = `
      const a = process.env.NODE_ENV;
      const b = process.env.PORT;
    `;
    const result = caps(source);
    expect(result).toHaveLength(2);
    expect(result.map((c) => c.target).sort()).toEqual(["NODE_ENV", "PORT"]);
  });
});

// ============================================================================
// fetch() Detection
// ============================================================================

describe("fetch detection", () => {
  it("detects fetch with URL string", () => {
    const result = caps(`fetch("https://api.example.com/data");`);
    expect(result).toHaveLength(1);
    expect(result[0].category).toBe("net");
    expect(result[0].action).toBe("connect");
    expect(result[0].target).toBe("https://api.example.com/data");
  });

  it("detects fetch with variable (target *)", () => {
    const result = caps(`fetch(apiUrl);`);
    expect(result).toHaveLength(1);
    expect(result[0].target).toBe("*");
  });

  it("detects fetch in async context", () => {
    const source = `
      async function getData() {
        const res = await fetch("https://httpbin.org/get");
        return res.json();
      }
    `;
    const result = caps(source);
    expect(result).toHaveLength(1);
    expect(result[0].target).toBe("https://httpbin.org/get");
  });
});

// ============================================================================
// Banned Construct Detection
// ============================================================================

describe("banned construct detection", () => {
  it("detects eval()", () => {
    const result = bans(`eval("alert(1)");`);
    expect(result).toHaveLength(1);
    expect(result[0].kind).toBe("eval");
  });

  it("detects new Function()", () => {
    const result = bans(`const fn = new Function("return 42");`);
    expect(result).toHaveLength(1);
    expect(result[0].kind).toBe("new-function");
  });

  it("detects dynamic require", () => {
    const result = bans(`const mod = require(moduleName);`);
    expect(result).toHaveLength(1);
    expect(result[0].kind).toBe("dynamic-require");
  });

  it("detects dynamic import()", () => {
    const result = bans(`const mod = await import(moduleName);`);
    expect(result).toHaveLength(1);
    expect(result[0].kind).toBe("dynamic-import");
  });

  it("does NOT flag static import()", () => {
    const result = bans(`const mod = await import("./module.js");`);
    expect(result).toHaveLength(0);
  });

  it("detects Reflect.apply", () => {
    const result = bans(`Reflect.apply(fn, null, []);`);
    expect(result).toHaveLength(1);
    expect(result[0].kind).toBe("reflect");
  });

  it("detects Reflect.construct", () => {
    const result = bans(`Reflect.construct(Cls, []);`);
    expect(result).toHaveLength(1);
    expect(result[0].kind).toBe("reflect");
  });

  it("does NOT flag Reflect.ownKeys (not banned)", () => {
    const result = bans(`Reflect.ownKeys(obj);`);
    expect(result).toHaveLength(0);
  });

  it("does NOT flag string literal require", () => {
    const result = bans(`const fs = require("fs");`);
    expect(result).toHaveLength(0);
  });
});

// ============================================================================
// Pure Code (No Capabilities)
// ============================================================================

describe("pure code detection", () => {
  it("finds no capabilities in pure math", () => {
    const source = `
      function add(a: number, b: number): number {
        return a + b;
      }
      const result = add(1, 2);
    `;
    const result = analyzeSource(source, "math.ts");
    expect(result.capabilities).toHaveLength(0);
    expect(result.banned).toHaveLength(0);
  });

  it("finds no capabilities in class definition", () => {
    const source = `
      class Point {
        constructor(public x: number, public y: number) {}
        distanceTo(other: Point): number {
          return Math.sqrt((this.x - other.x) ** 2 + (this.y - other.y) ** 2);
        }
      }
    `;
    const result = analyzeSource(source, "point.ts");
    expect(result.capabilities).toHaveLength(0);
    expect(result.banned).toHaveLength(0);
  });

  it("finds no capabilities in array operations", () => {
    const source = `
      const nums = [1, 2, 3, 4, 5];
      const doubled = nums.map(n => n * 2);
      const sum = nums.reduce((a, b) => a + b, 0);
    `;
    const result = analyzeSource(source, "arrays.ts");
    expect(result.capabilities).toHaveLength(0);
    expect(result.banned).toHaveLength(0);
  });

  it("finds no capabilities in Promise usage", () => {
    const source = `
      async function delay(ms: number): Promise<void> {
        return new Promise(resolve => setTimeout(resolve, ms));
      }
    `;
    const result = analyzeSource(source, "delay.ts");
    expect(result.capabilities).toHaveLength(0);
    expect(result.banned).toHaveLength(0);
  });

  it("ignores non-tracked module imports", () => {
    const source = `
      import React from "react";
      import { useState } from "react";
      import lodash from "lodash";
    `;
    const result = analyzeSource(source, "app.tsx");
    expect(result.capabilities).toHaveLength(0);
    expect(result.banned).toHaveLength(0);
  });
});

// ============================================================================
// Multi-file Analysis
// ============================================================================

describe("multi-file analysis", () => {
  it("merges results from multiple files", () => {
    const result = analyzeFiles([
      { filename: "a.ts", source: `import fs from "fs";` },
      { filename: "b.ts", source: `import http from "http";` },
    ]);
    expect(result.capabilities).toHaveLength(2);
    expect(result.capabilities[0].file).toBe("a.ts");
    expect(result.capabilities[1].file).toBe("b.ts");
  });

  it("merges banned from multiple files", () => {
    const result = analyzeFiles([
      { filename: "a.ts", source: `eval("x");` },
      { filename: "b.ts", source: `new Function("x");` },
    ]);
    expect(result.banned).toHaveLength(2);
  });
});

// ============================================================================
// Edge Cases
// ============================================================================

describe("edge cases", () => {
  it("handles empty source", () => {
    const result = analyzeSource("", "empty.ts");
    expect(result.capabilities).toHaveLength(0);
    expect(result.banned).toHaveLength(0);
  });

  it("handles source with syntax errors gracefully", () => {
    // TypeScript parser is lenient — it will still produce a partial AST
    const result = analyzeSource(`import fs from "fs"; fs.readFileSync(`, "broken.ts");
    // Should at least detect the import
    expect(result.capabilities.length).toBeGreaterThanOrEqual(1);
  });

  it("reports correct line numbers", () => {
    const source = `
const x = 1;
const y = 2;
import fs from "fs";
`;
    const result = caps(source);
    expect(result[0].line).toBe(4);
  });

  it("handles aliased named imports", () => {
    const source = `import { readFileSync as rfs } from "fs";`;
    const result = caps(source);
    expect(result).toHaveLength(1);
    expect(result[0].action).toBe("read");
    expect(result[0].evidence).toContain("readFileSync");
  });

  it("detects namespace import call pattern", () => {
    const source = `
      import * as childProcess from "child_process";
      childProcess.exec("ls");
    `;
    const result = caps(source);
    const execs = result.filter((c) => c.category === "proc" && c.action === "exec");
    expect(execs).toHaveLength(1);
  });

  it("handles template literal in readFileSync", () => {
    const source = `
      import fs from "fs";
      fs.readFileSync(\`/etc/hosts\`);
    `;
    const result = caps(source);
    const reads = result.filter((c) => c.action === "read");
    expect(reads).toHaveLength(1);
    expect(reads[0].target).toBe("/etc/hosts");
  });
});
