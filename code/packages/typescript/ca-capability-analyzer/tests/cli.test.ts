import { describe, it, expect } from "@coding-adventures/test-bootstrap";
import { formatReportText, runCli } from "../src/index.js";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const PKG_DIR = path.resolve(__dirname, "../..");
const NOTE_FREQ_DIR = path.resolve(__dirname, "../../../note-frequency");
const AUDIO_SINK_DIR = path.resolve(__dirname, "../../../audio-device-sink");

describe("CLI Interface (TF01 / VER08)", () => {
  it("--help exits with code 0", () => {
    const code = runCli(["--help"]);
    expect(code).toBe(0);
  });

  it("--version exits with code 0", () => {
    const code = runCli(["--version"]);
    expect(code).toBe(0);
  });

  it("unknown option exits with code 2", () => {
    const code = runCli(["--unknown-flag-xyz"]);
    expect(code).toBe(2);
  });

  it("analyzes itself successfully in --strict mode", () => {
    const code = runCli(["--dir", PKG_DIR, "--strict"]);
    expect(code).toBe(0);
  });

  it("analyzes note-frequency in --strict mode successfully", () => {
    const code = runCli(["--dir", NOTE_FREQ_DIR, "--strict"]);
    expect(code).toBe(0);
  });

  it("analyzes audio-device-sink in --strict mode successfully", () => {
    const code = runCli(["--dir", AUDIO_SINK_DIR, "--strict"]);
    expect(code).toBe(0);
  });

  it("emits machine-readable JSON with --json flag", () => {
    const originalLog = console.log;
    let captured = "";
    console.log = (msg: string) => {
      captured += msg;
    };
    try {
      const code = runCli(["--dir", PKG_DIR, "--json"]);
      expect(code).toBe(0);
      const parsed = JSON.parse(captured);
      expect(parsed.ok).toBe(true);
      expect(Array.isArray(parsed.inspectedFiles)).toBe(true);
    } finally {
      console.log = originalLog;
    }
  });

  it("supports --dir=path, --verbose, --include-tests, missing --dir arg, and exit codes 1/2", () => {
    const originalLog = console.log;
    const originalError = console.error;
    console.log = () => {};
    console.error = () => {};
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "ca-cli-"));
    try {
      expect(runCli(["--dir"])).toBe(2);
      expect(runCli(["--dir", path.join(tmpDir, "missing")])).toBe(2);
      expect(runCli([`--dir=${PKG_DIR}`, "--verbose"])).toBe(0);
      expect(runCli([`--dir=${NOTE_FREQ_DIR}`, "--verbose", "--include-tests"])).toBe(0);

      fs.writeFileSync(
        path.join(tmpDir, "required_capabilities.json"),
        JSON.stringify({
          capabilities: [],
          banned_construct_exceptions: [
            { construct: "CWE-94", language: "ts", justification: "test" },
          ],
        }),
        "utf-8"
      );
      fs.writeFileSync(
        path.join(tmpDir, "bad.ts"),
        'import fs from "node:fs"; eval("1"); const x: any = {}; x[k] = 1; fs.readFileSync("a");\n',
        "utf-8"
      );

      expect(runCli([tmpDir, "--strict"])).toBe(1);
      const formatted = formatReportText({
        packageDir: tmpDir,
        inspectedFiles: [path.join(tmpDir, "bad.ts")],
        detectedCapabilities: [
          { capability: "fs:read", category: "fs", action: "read", file: "bad.ts", line: 1, col: 1, evidence: "fs" },
        ],
        declaredCapabilities: [],
        undeclaredCapabilities: [
          { capability: "fs:read", category: "fs", action: "read", file: "bad.ts", line: 1, col: 1, evidence: "fs" },
        ],
        detectedWeaknesses: [
          { cwe: "CWE-1321", name: "Prototype Pollution", file: "bad.ts", line: 1, col: 1, evidence: "x[k]=1", description: "bad" },
        ],
        exemptWeaknesses: [
          { cwe: "CWE-94", name: "Code Injection", file: "bad.ts", line: 1, col: 1, evidence: "eval", description: "exempt" },
        ],
        violations: ["[CAP001] Undeclared capability"],
        ok: false,
      });
      expect(formatted).toContain("Detected VER15 Weaknesses");
      expect(formatted).toContain("Exempt Weaknesses");
      expect(formatted).toContain("Violations:");
      expect(formatted).toContain("Verdict: FAIL");
    } finally {
      console.log = originalLog;
      console.error = originalError;
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  });
});
