import { describe, it, expect } from "@coding-adventures/test-bootstrap";
import { analyzeSource } from "../src/index.js";

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
});
