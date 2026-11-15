import { describe, it, expect } from "../../test-bootstrap/dist/index.js";
import { runCli } from "../dist/index.js";
import * as path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const PKG_DIR = path.resolve(__dirname, "..");
const NOTE_FREQ_DIR = path.resolve(__dirname, "../../note-frequency");
const AUDIO_SINK_DIR = path.resolve(__dirname, "../../audio-device-sink");

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
});
