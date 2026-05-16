import { describe, it, expect } from "vitest";
import { parseManifest, canonicalManifestToml } from "../src/index.js";

const MIN_MANIFEST = `
manifestVersion = 1
[plugin]
name = "@me/x"
version = "1.0.0"
apiVersion = 1
[runtime]
kind = "node"
entry = "./e.js"
[[contributes.stages]]
id = "s"
consumes = "ContentSource"
produces = "ContentNode"
`;

describe("canonicalManifestToml", () => {
  it("is a fixed point under parse + encode", () => {
    const m = parseManifest(MIN_MANIFEST);
    const out1 = canonicalManifestToml(m);
    const m2 = parseManifest(out1);
    const out2 = canonicalManifestToml(m2);
    expect(out1).toBe(out2);
  });

  it("sorts keys lexicographically inside each table", () => {
    const m = parseManifest(`
manifestVersion = 1
[plugin]
version = "1.0.0"
authors = ["A"]
name = "@me/x"
apiVersion = 1
description = "d"
[runtime]
kind = "node"
entry = "./e.js"
[[contributes.stages]]
produces = "ContentNode"
consumes = "ContentSource"
id = "s"
`);
    const out = canonicalManifestToml(m);
    const pluginBlock = out.split("[plugin]")[1]!.split("\n[")[0]!;
    const keys = pluginBlock.split("\n")
      .filter((l) => l.includes("="))
      .map((l) => l.split("=")[0]!.trim());
    const sorted = [...keys].sort();
    expect(keys).toEqual(sorted);
  });

  it("excludes the [signature] block", () => {
    const m = parseManifest(`
manifestVersion = 1
[plugin]
name = "@me/x"
version = "1.0.0"
apiVersion = 1
[runtime]
kind = "node"
entry = "./e.js"
[[contributes.stages]]
id = "s"
consumes = "ContentSource"
produces = "ContentNode"
[signature]
algorithm = "ed25519"
publicKey = "AAA="
signature = "BBB="
signedAt = "2026-05-16T00:00:00Z"
`);
    const out = canonicalManifestToml(m);
    expect(out).not.toContain("[signature]");
    expect(out).not.toContain("publicKey");
  });

  it("uses LF line endings only", () => {
    const m = parseManifest(MIN_MANIFEST);
    const out = canonicalManifestToml(m);
    expect(out).not.toContain("\r");
  });

  it("ends with exactly one newline", () => {
    const m = parseManifest(MIN_MANIFEST);
    const out = canonicalManifestToml(m);
    expect(out.endsWith("\n")).toBe(true);
    expect(out.endsWith("\n\n")).toBe(false);
  });

  it("emits manifestVersion before any section", () => {
    const m = parseManifest(MIN_MANIFEST);
    const out = canonicalManifestToml(m);
    const vIdx = out.indexOf("manifestVersion");
    const sIdx = out.indexOf("[plugin]");
    expect(vIdx).toBeGreaterThan(-1);
    expect(vIdx).toBeLessThan(sIdx);
  });

  it("escapes control characters in strings", () => {
    const m = parseManifest(`
manifestVersion = 1
[plugin]
name = "@me/x"
version = "1.0.0"
apiVersion = 1
description = "tab\\there"
[runtime]
kind = "node"
entry = "./e.js"
[[contributes.stages]]
id = "s"
consumes = "ContentSource"
produces = "ContentNode"
`);
    const out = canonicalManifestToml(m);
    expect(out).toContain('"tab\\there"');
  });

  it("emits each [[array.of.tables]] element as a separate block", () => {
    const m = parseManifest(`
manifestVersion = 1
[plugin]
name = "@me/x"
version = "1.0.0"
apiVersion = 1
[runtime]
kind = "node"
entry = "./e.js"
[[capabilities.required]]
realm = "filesystem"
scope = "read"
reason = "first"
[[capabilities.required]]
realm = "filesystem"
scope = "write"
reason = "second"
[[contributes.stages]]
id = "s"
consumes = "ContentSource"
produces = "ContentNode"
`);
    const out = canonicalManifestToml(m);
    const blocks = out.split("[[capabilities.required]]").length - 1;
    expect(blocks).toBe(2);
  });

  it("emits resources block only when populated", () => {
    const m1 = parseManifest(MIN_MANIFEST);
    const out1 = canonicalManifestToml(m1);
    expect(out1).not.toContain("[resources]");

    const m2 = parseManifest(MIN_MANIFEST + "\n[resources]\nmaxMemoryMb = 256\n");
    const out2 = canonicalManifestToml(m2);
    expect(out2).toContain("[resources]");
    expect(out2).toContain("maxMemoryMb = 256");
  });

  it("emits platform dotted keys when present", () => {
    const m = parseManifest(`
manifestVersion = 1
[plugin]
name = "@me/x"
version = "1.0.0"
apiVersion = 1
[runtime]
kind = "binary"
entry = "./fallback"
platforms.linux-x86_64 = "./bin/l"
platforms.darwin-aarch64 = "./bin/m"
[[contributes.stages]]
id = "s"
consumes = "ContentSource"
produces = "ContentNode"
`);
    const out = canonicalManifestToml(m);
    expect(out).toContain('platforms.darwin-aarch64 = "./bin/m"');
    expect(out).toContain('platforms.linux-x86_64 = "./bin/l"');
  });

  it("emits authors as an inline array", () => {
    const m = parseManifest(`
manifestVersion = 1
[plugin]
name = "@me/x"
version = "1.0.0"
apiVersion = 1
authors = ["A", "B"]
[runtime]
kind = "node"
entry = "./e.js"
[[contributes.stages]]
id = "s"
consumes = "ContentSource"
produces = "ContentNode"
`);
    const out = canonicalManifestToml(m);
    expect(out).toContain('authors = ["A", "B"]');
  });
});
