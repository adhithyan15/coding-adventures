import { describe, it, expect } from "@coding-adventures/test-bootstrap";
import * as path from "node:path";
import { fileURLToPath } from "node:url";
import { analyzePackage, analyzeSource } from "../src/index.js";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

describe("Ambient Capability Detection", () => {
  it("detects fs:read from import and readFileSync", () => {
    const code = `
      import fs from "node:fs";
      const data = fs.readFileSync("file.txt", "utf-8");
    `;
    const res = analyzeSource(code, "test.ts");
    const caps = res.capabilities.map((c) => c.capability);
    expect(caps).toContain("fs:read");
  });

  it("detects fs:write from writeFile", () => {
    const code = `
      import { writeFile } from "fs";
      writeFile("out.txt", "content");
    `;
    const res = analyzeSource(code, "test.ts");
    const caps = res.capabilities.map((c) => c.capability);
    expect(caps).toContain("fs:read"); // from import
    expect(caps).toContain("fs:write"); // from writeFile
  });

  it("detects net:socket from fetch and WebSocket", () => {
    const code = `
      const res = await fetch("https://example.com");
      const ws = new WebSocket("wss://echo.websocket.org");
    `;
    const res = analyzeSource(code, "test.ts");
    const caps = res.capabilities.map((c) => c.capability);
    expect(caps).toContain("net:socket");
    expect(res.capabilities.filter((c) => c.capability === "net:socket").length).toBe(2);
  });

  it("detects process:spawn from child_process", () => {
    const code = `
      import { spawn } from "child_process";
      spawn("ls", ["-la"]);
    `;
    const res = analyzeSource(code, "test.ts");
    const caps = res.capabilities.map((c) => c.capability);
    expect(caps).toContain("process:spawn");
  });

  it("detects env:read from process.env", () => {
    const code = `
      const secret = process.env.API_KEY;
    `;
    const res = analyzeSource(code, "test.ts");
    const caps = res.capabilities.map((c) => c.capability);
    expect(caps).toContain("env:read");
  });

  it("detects ffi:unsafe from dlopen", () => {
    const code = `
      process.dlopen(module, "libnative.node");
    `;
    const res = analyzeSource(code, "test.ts");
    const caps = res.capabilities.map((c) => c.capability);
    expect(caps).toContain("ffi:unsafe");
  });

  it("detects time:clock from Date.now and performance.now", () => {
    const code = `
      const t1 = Date.now();
      const t2 = performance.now();
      const d = new Date();
    `;
    const res = analyzeSource(code, "test.ts");
    const caps = res.capabilities.map((c) => c.capability);
    expect(caps).toContain("time:clock");
  });

  it("detects random:entropy from Math.random and crypto", () => {
    const code = `
      const r = Math.random();
      const u = crypto.randomUUID();
    `;
    const res = analyzeSource(code, "test.ts");
    const caps = res.capabilities.map((c) => c.capability);
    expect(caps).toContain("random:entropy");
  });

  it("suppresses capabilities when marked with // nolint:cap", () => {
    const code = `
      const r = Math.random(); // nolint:cap
    `;
    const res = analyzeSource(code, "test.ts");
    expect(res.capabilities.length).toBe(0);
  });
});

describe("VER15 CWE Weakness Detection", () => {
  it("detects CWE-1321 (Prototype Pollution) via __proto__ assignment", () => {
    const code = `
      const obj: any = {};
      obj.__proto__ = { evil: true };
    `;
    const res = analyzeSource(code, "test.ts");
    const cwes = res.weaknesses.map((w) => w.cwe);
    expect(cwes).toContain("CWE-1321");
  });

  it("detects CWE-1321 via dynamic bracket writes on plain objects", () => {
    const code = `
      function merge(target: any, key: string, value: any) {
        target[key] = value;
      }
    `;
    const res = analyzeSource(code, "test.ts");
    const cwes = res.weaknesses.map((w) => w.cwe);
    expect(cwes).toContain("CWE-1321");
  });

  it("detects CWE-1333 (ReDoS) from catastrophic regex literals", () => {
    const code = `
      const pattern = /(a+)+$/;
    `;
    const res = analyzeSource(code, "test.ts");
    const cwes = res.weaknesses.map((w) => w.cwe);
    expect(cwes).toContain("CWE-1333");
  });

  it("detects CWE-1333 from new RegExp with unvalidated dynamic variable", () => {
    const code = `
      function search(userInput: string) {
        return new RegExp(userInput);
      }
    `;
    const res = analyzeSource(code, "test.ts");
    const cwes = res.weaknesses.map((w) => w.cwe);
    expect(cwes).toContain("CWE-1333");
  });

  it("detects CWE-674 (Uncontrolled Recursion) when self-call lacks depth guard", () => {
    const code = `
      function traverseTree(node: any) {
        if (!node) return;
        traverseTree(node.left);
      }
    `;
    const res = analyzeSource(code, "test.ts");
    const cwes = res.weaknesses.map((w) => w.cwe);
    expect(cwes).toContain("CWE-674");
  });

  it("permits recursive functions that carry an explicit depth guard parameter", () => {
    const code = `
      function safeTraverse(node: any, depth: number = 0) {
        if (!node || depth > 64) return;
        safeTraverse(node.left, depth + 1);
      }
    `;
    const res = analyzeSource(code, "test.ts");
    const cwes = res.weaknesses.map((w) => w.cwe);
    expect(cwes).not.toContain("CWE-674");
  });

  it("detects CWE-94 (Code Injection) via eval and new Function", () => {
    const code = `
      eval("2 + 2");
      const fn = new Function("a", "return a * 2");
    `;
    const res = analyzeSource(code, "test.ts");
    const cwes = res.weaknesses.map((w) => w.cwe);
    expect(cwes).toContain("CWE-94");
    expect(res.weaknesses.filter((w) => w.cwe === "CWE-94").length).toBe(2);
  });

  it("detects CWE-78 (OS Command Injection) via dynamic child_process.exec", () => {
    const code = `
      import child_process from "child_process";
      const cmd = "cat " + userFile;
      child_process.exec(cmd);
    `;
    const res = analyzeSource(code, "test.ts");
    const cwes = res.weaknesses.map((w) => w.cwe);
    expect(cwes).toContain("CWE-78");
  });

  it("suppresses CWE findings with // nolint:cwe comment", () => {
    const code = `
      eval("1 + 1"); // nolint:cwe-94
    `;
    const res = analyzeSource(code, "test.ts");
    expect(res.weaknesses.length).toBe(0);
  });

  it("detects namespace imports, FFI imports, Deno/Bun env, and catastrophic regex patterns", () => {
    const code = `
      import * as fsNs from "node:fs";
      import ffi from "ffi-napi";
      const d = Deno.env;
      const b = Bun.env;
      const r1 = new RegExp("(a+)+");
      const r2 = /(a|a)+/;
      const r3 = /.*.*/;
      fsNs.readFileSync("x");
    `;
    const res = analyzeSource(code, "extra.ts");
    const caps = res.capabilities.map((c) => c.capability);
    expect(caps).toContain("fs:read");
    expect(caps).toContain("ffi:unsafe");
    expect(caps).toContain("env:read");
    expect(res.weaknesses.filter((w) => w.cwe === "CWE-1333").length).toBe(3);
  });

  it("permits numeric array and TypedArray bracket writes without CWE-1321 false positives", () => {
    const code = `
      class ComplexBuffer {
        private readonly _data: Float64Array;
        constructor(n: number) {
          this._data = new Float64Array(2 * n);
        }
        setAt(idx: number, re: number, im: number): void {
          this._data[2 * idx] = re;
          this._data[2 * idx + 1] = im;
        }
      }

      export function mixSamples(target: number[], startSample: number, src: readonly number[]): void {
        let outIdx = 0;
        const n: number = src.length;
        const buf = new Float64Array(n);
        const flaggedCheck = (src.length > 0 ? 0 : 1);
        for (let i = 0; i < n; i++) {
          const targetIndex = startSample + i;
          target[targetIndex] = src[i];
          buf[(i)] = Number(src[i]) + (i & 1);
          buf[outIdx++] = -buf[i];
          buf[flaggedCheck] = Math.cos(i);
          buf[(i as number)] = 1.0;
        }
      }
    `;
    const res = analyzeSource(code, "dsp.ts");
    expect(res.weaknesses.filter((w) => w.cwe === "CWE-1321")).toEqual([]);
  });

  it("still detects CWE-1321 when bracket key is non-numeric, reassigned, or from for-of", () => {
    const code = `
      function pollute(arr: number[], keys: string[]): void {
        let idx = 0;
        idx = keys[0] as any;
        arr[idx] = 1;
        for (const k of keys) {
          arr[k as any] = 2;
        }
        const badKey = "constructor";
        const obj: Record<string, unknown> = {};
        obj[badKey] = 3;
      }
    `;
    const res = analyzeSource(code, "pollute.ts");
    expect(res.weaknesses.filter((w) => w.cwe === "CWE-1321").length).toBe(3);
  });

  it("rejects CWE-1321 bypasses via erased casts, for..in, loop back-edges, destructuring, and +=/||=", () => {
    const code = `
      async function evasions(obj: Record<string, unknown>, evil: Record<string, unknown>, key: string) {
        // A: erased 'as unknown as number' cast
        obj[key as unknown as number] = 1;

        // B: for..in string-key overwrite of previously numeric var
        let k1 = 0;
        for (k1 in evil) {
          obj[k1] = evil[k1];
        }

        // C: loop back-edge lexical order (use before mutation)
        let k2 = 0;
        for (let i = 0; i < 2; i++) {
          obj[k2] = 1;
          k2 = "__proto__";
        }

        // D: destructuring reassignment (array & object)
        let k3 = 0;
        let k4 = 0;
        [k3] = ["__proto__"];
        ({ a: k4 } = { a: "constructor" });
        obj[k3] = 1;
        obj[k4] = 2;

        // E: compound += and logical ||= / ??= / &&=
        let k5 = 0;
        k5 += "__proto__";
        obj[k5] = 1;

        let k6 = 0;
        k6 ||= key as any;
        obj[k6] = 1;

        // F: await import("node:fs") alias binding -> fs:read + fs:write
        const f = await import("node:fs");
        const { rm } = await import("node:fs");
        f.rm("x");
        rm("y");
      }
    `;
    const res = analyzeSource(code, "evasions.ts");
    expect(res.weaknesses.filter((w) => w.cwe === "CWE-1321").length).toBe(7);
    const caps = res.capabilities.map((c) => c.capability);
    expect(caps).toContain("fs:read");
    expect(caps).toContain("fs:write");
  });

  it("handles deeply nested numeric bracket expressions without stack overflow", () => {
    const deepExpr = "(".repeat(75) + "1" + ")".repeat(75);
    const code = `
      const obj: any = {};
      obj[${deepExpr}] = 42;
    `;
    const res = analyzeSource(code, "deep_index.ts");
    expect(res.weaknesses.some((w) => w.cwe === "CWE-1321")).toBe(true);
  });

  it("detects CommonJS require, dynamic import(), globalThis eval/Function, and method recursion", () => {
    const code = `
      const fs = require("node:fs");
      const { exec } = require("node:child_process");
      import("node:net");
      async function loadDynamic(modName: string) {
        return await import(modName);
      }
      globalThis.eval("1 + 1");
      const f = new globalThis.Function("return 1;");
      exec("echo " + process.env.USER);
      fs.stat("x");
      const timer = { stat: () => 0 };
      timer.stat();
      const staticRegex = new RegExp(\`^[a-z]+$\`);
      class Tree {
        walk(n: any) {
          if (!n) return;
          this.walk(n.next);
        }
      }
    `;
    const res = analyzeSource(code, "evasion.ts");
    const caps = res.capabilities.map((c) => c.capability);
    expect(caps).toContain("fs:read");
    expect(caps).toContain("process:spawn");
    expect(caps).toContain("net:socket");
    expect(caps).toContain("env:read");
    // timer.stat() must not produce a second fs:read beyond require + fs.stat
    expect(res.capabilities.filter((c) => c.capability === "fs:read").length).toBe(2);

    const cwes = res.weaknesses.map((w) => w.cwe);
    expect(cwes).toContain("CWE-94");
    expect(cwes).toContain("CWE-78");
    expect(cwes).toContain("CWE-674");
    expect(cwes).not.toContain("CWE-1333");
  });

  it("passes real dsp-complex and music-machine-node packages in --strict mode", () => {
    for (const pkgName of ["dsp-complex", "dsp-filters", "music-machine-node"]) {
      const pkgDir = path.resolve(__dirname, "../../..", pkgName);
      const result = analyzePackage(pkgDir, { strict: true });
      expect(result.ok).toBe(true);
      expect(result.violations).toEqual([]);
    }
  });
});
