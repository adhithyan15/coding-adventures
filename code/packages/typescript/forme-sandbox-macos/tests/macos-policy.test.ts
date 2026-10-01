import { describe, expect, it } from "vitest";
import { createMacosSandboxFactory, macosSeatbeltProfile } from "../src/index.js";

describe("macOS sandbox policy", () => {
  it("generates a deny-default profile scoped to staged and runtime paths", () => {
    const profile = macosSeatbeltProfile({
      workingDirectory: "/private/tmp/forme-plugin-123",
      runtimeReadPaths: ["/usr/local/bin/node", "/usr/lib/libSystem.B.dylib"],
    });
    expect(profile).toContain("(deny default)");
    expect(profile).toContain('(subpath "/private/tmp/forme-plugin-123")');
    expect(profile).toContain('(literal "/usr/local/bin/node")');
    expect(profile).toContain("(deny network*)");
    expect(profile).toContain("(deny process-fork)");
    expect(profile).not.toContain("(allow default)");
  });

  it("escapes Seatbelt literals and rejects control characters", () => {
    expect(macosSeatbeltProfile({
      workingDirectory: '/tmp/a"b',
      runtimeReadPaths: ["/usr/bin/node"],
    })).toContain('/tmp/a\\"b');
    expect(() => macosSeatbeltProfile({
      workingDirectory: "/tmp/bad\npath",
      runtimeReadPaths: ["/usr/bin/node"],
    })).toThrow(/control/);
    expect(() => macosSeatbeltProfile({
      workingDirectory: "/tmp/forme",
      runtimeReadPaths: [],
    })).toThrow(/runtime read path/);
  });

  it("constructs only the versioned macOS factory", () => {
    const factory = createMacosSandboxFactory({ launcherPath: "/trusted/forme-sandbox-macos" });
    expect(typeof factory.launch).toBe("function");
    expect(typeof createMacosSandboxFactory().launch).toBe("function");
    expect(() => createMacosSandboxFactory({ runtimeReadPaths: ["relative"] })).toThrow(/runtime read paths/);
    expect(() => createMacosSandboxFactory({ runtimeReadPaths: Array(33).fill("/trusted") })).toThrow(/runtime read paths/);
  });
});
