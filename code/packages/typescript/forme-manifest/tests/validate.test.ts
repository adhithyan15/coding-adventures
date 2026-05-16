import { describe, it, expect } from "vitest";
import {
  parseManifest,
  validateManifest,
  ManifestError,
  PLUGIN_NAME_REGEX,
  RESERVED_KIND_NAMES,
  type Manifest,
} from "../src/index.js";

function valid(): Manifest {
  return parseManifest(`
manifestVersion = 1
[plugin]
name       = "@me/test-plugin"
version    = "1.0.0"
apiVersion = 1
[runtime]
kind  = "node"
entry = "./e.js"
[[capabilities.required]]
realm  = "filesystem"
scope  = "read"
detail = "$storageRoot"
reason = "needs to read source files"
[[contributes.stages]]
id       = "my-stage"
consumes = "ContentSource"
produces = "ContentNode"
`);
}

describe("validateManifest — happy paths", () => {
  it("accepts a valid manifest", () => {
    expect(() => validateManifest(valid())).not.toThrow();
  });

  it("accepts a binary runtime with platforms", () => {
    const m = parseManifest(`
manifestVersion = 1
[plugin]
name = "@me/p"
version = "1.0.0"
apiVersion = 1
[runtime]
kind = "binary"
entry = "./ignored"
platforms.linux-x86_64 = "./bin/linux"
[[contributes.stages]]
id = "s"
consumes = "ContentSource"
produces = "ContentNode"
`);
    expect(() => validateManifest(m)).not.toThrow();
  });

  it("accepts ext: kinds", () => {
    const m = parseManifest(`
manifestVersion = 1
[plugin]
name = "@me/p"
version = "1.0.0"
apiVersion = 1
[runtime]
kind = "node"
entry = "./e.js"
[[contributes.kinds]]
name = "ext:youtube-embed"
version = "1.0"
subtypeOf = "ContentNode"
`);
    expect(() => validateManifest(m)).not.toThrow();
  });
});

describe("validateManifest — rejections (one per FM02 §3.3 rule)", () => {
  it("rejects unsupported manifestVersion", () => {
    const m = { ...valid(), manifestVersion: 99 };
    expect(() => validateManifest(m)).toThrowError(/manifestVersion.*not supported/);
  });

  it("rejects missing plugin.name", () => {
    const m: Manifest = { ...valid(), plugin: { ...valid().plugin, name: "" } };
    expect(() => validateManifest(m)).toThrow();
  });

  it("rejects plugin.name with uppercase", () => {
    const m: Manifest = { ...valid(), plugin: { ...valid().plugin, name: "Bad-Name" } };
    try {
      validateManifest(m);
      expect.fail("should throw");
    } catch (err) {
      expect((err as ManifestError).errors[0]!.code).toBe("PLUGIN_NAME_INVALID");
    }
  });

  it("PLUGIN_NAME_REGEX accepts valid names", () => {
    expect(PLUGIN_NAME_REGEX.test("@me/foo-bar")).toBe(true);
    expect(PLUGIN_NAME_REGEX.test("simple")).toBe(true);
    expect(PLUGIN_NAME_REGEX.test("-bad")).toBe(false);
    expect(PLUGIN_NAME_REGEX.test("@/foo")).toBe(false);
  });

  it("rejects invalid plugin.version", () => {
    const m: Manifest = { ...valid(), plugin: { ...valid().plugin, version: "not.a.semver" } };
    expect(() => validateManifest(m)).toThrowError(/PLUGIN_VERSION_INVALID|semver/);
  });

  it("rejects invalid apiVersion", () => {
    const m: Manifest = { ...valid(), plugin: { ...valid().plugin, apiVersion: 99 } };
    expect(() => validateManifest(m)).toThrowError(/apiVersion 99/);
  });

  it("rejects unrecognised runtime.kind", () => {
    const v = valid();
    const m: Manifest = { ...v, runtime: { ...v.runtime, kind: "ruby" as never } };
    expect(() => validateManifest(m)).toThrowError(/RUNTIME_KIND_INVALID|must be one of/);
  });

  it("rejects binary runtime without platforms", () => {
    const v = valid();
    const m: Manifest = { ...v, runtime: { kind: "binary", entry: "./e" } };
    expect(() => validateManifest(m)).toThrowError(/platforms|RUNTIME_PLATFORMS_MISSING/);
  });

  it("rejects non-binary runtime without entry", () => {
    const v = valid();
    const m: Manifest = { ...v, runtime: { kind: "node", entry: "" } };
    expect(() => validateManifest(m)).toThrowError(/RUNTIME_ENTRY_MISSING|required for non-binary/);
  });

  it("rejects malformed capability strings", () => {
    const v = valid();
    const m: Manifest = {
      ...v,
      capabilities: {
        required: [{ realm: "bad realm", scope: "read", reason: "x" }],
        optional: [],
      },
    };
    expect(() => validateManifest(m)).toThrowError(/CAPABILITY_MALFORMED|failed to parse/);
  });

  it("rejects FIRST_PARTY_ONLY required capabilities", () => {
    const v = valid();
    const m: Manifest = {
      ...v,
      capabilities: {
        required: [{ realm: "system", scope: "shell", reason: "would be bad" }],
        optional: [],
      },
    };
    expect(() => validateManifest(m)).toThrowError(/CAPABILITY_FIRST_PARTY_ONLY|reserved for first-party/);
  });

  it("rejects capability missing reason", () => {
    const v = valid();
    const m: Manifest = {
      ...v,
      capabilities: {
        required: [{ realm: "filesystem", scope: "read", reason: "" }],
        optional: [],
      },
    };
    expect(() => validateManifest(m)).toThrowError(/reason/);
  });

  it("rejects contributes with no stages and no kinds", () => {
    const v = valid();
    const m: Manifest = { ...v, contributes: { stages: [], kinds: [] } };
    expect(() => validateManifest(m)).toThrowError(/at least one stage or kind/);
  });

  it("rejects duplicate stage ids", () => {
    const v = valid();
    const m: Manifest = {
      ...v,
      contributes: {
        stages: [
          { id: "dup", consumes: "ContentSource", produces: "ContentNode" },
          { id: "dup", consumes: "ContentNode", produces: "DeployArtifact" },
        ],
        kinds: [],
      },
    };
    expect(() => validateManifest(m)).toThrowError(/STAGE_ID_DUPLICATE|more than once/);
  });

  it("rejects invalid stage ids", () => {
    const v = valid();
    const m: Manifest = {
      ...v,
      contributes: {
        stages: [{ id: "Bad ID", consumes: "ContentSource", produces: "ContentNode" }],
        kinds: [],
      },
    };
    expect(() => validateManifest(m)).toThrowError(/STAGE_ID_INVALID/);
  });

  it("rejects unknown kernel kind in stage", () => {
    const v = valid();
    const m: Manifest = {
      ...v,
      contributes: {
        stages: [{ id: "s", consumes: "UnknownKind", produces: "ContentNode" }],
        kinds: [],
      },
    };
    expect(() => validateManifest(m)).toThrowError(/unknown kind/);
  });

  it("rejects ext: kind not matching the format", () => {
    const v = valid();
    const m: Manifest = {
      ...v,
      contributes: {
        stages: [],
        kinds: [{ name: "no-prefix", version: "1.0" }],
      },
    };
    expect(() => validateManifest(m)).toThrowError(/begin with "ext:"|KIND_NAME_INVALID/);
  });

  it("rejects ext: kind with capital letters", () => {
    const v = valid();
    const m: Manifest = {
      ...v,
      contributes: {
        stages: [],
        kinds: [{ name: "ext:WithCaps", version: "1.0" }],
      },
    };
    expect(() => validateManifest(m)).toThrow();
  });

  it("rejects kind version that isn't MAJOR.MINOR", () => {
    const v = valid();
    const m: Manifest = {
      ...v,
      contributes: {
        stages: [],
        kinds: [{ name: "ext:foo", version: "1" }],
      },
    };
    expect(() => validateManifest(m)).toThrowError(/MAJOR.MINOR/);
  });

  it("rejects kind subtypeOf pointing at neither kernel nor ext:", () => {
    const v = valid();
    const m: Manifest = {
      ...v,
      contributes: {
        stages: [],
        kinds: [{ name: "ext:foo", version: "1.0", subtypeOf: "Whatever" }],
      },
    };
    expect(() => validateManifest(m)).toThrowError(/subtypeOf/);
  });

  it("rejects negative resource values", () => {
    const v = valid();
    const m: Manifest = { ...v, resources: { maxMemoryMb: -1 } };
    expect(() => validateManifest(m)).toThrowError(/positive integer/);
  });

  it("rejects resource values above the ceiling", () => {
    const v = valid();
    const m: Manifest = { ...v, resources: { maxMemoryMb: 999_999 } };
    expect(() => validateManifest(m)).toThrowError(/ceiling/);
  });

  it("rejects unknown signature algorithm", () => {
    const v = valid();
    const m: Manifest = {
      ...v,
      signature: {
        algorithm: "rsa",
        publicKey: "A".repeat(40),
        signature: "A".repeat(80),
        signedAt: "2026-05-16T00:00:00Z",
      },
    };
    expect(() => validateManifest(m)).toThrowError(/SIGNATURE_ALGORITHM_INVALID/);
  });

  it("rejects signature with missing publicKey", () => {
    const v = valid();
    const m: Manifest = {
      ...v,
      signature: { algorithm: "ed25519", publicKey: "", signature: "x", signedAt: "2026-05-16T00:00:00Z" },
    };
    expect(() => validateManifest(m)).toThrowError(/publicKey/);
  });

  it("rejects signedAt not RFC 3339", () => {
    const v = valid();
    const m: Manifest = {
      ...v,
      signature: { algorithm: "ed25519", publicKey: "x", signature: "y", signedAt: "yesterday" },
    };
    expect(() => validateManifest(m)).toThrowError(/RFC 3339/);
  });

  it("aggregates multiple errors", () => {
    const m: Manifest = {
      manifestVersion: 99,
      plugin: { name: "BAD", version: "x", apiVersion: 99 },
      runtime: { kind: "rust" as never, entry: "" },
      capabilities: { required: [], optional: [] },
      contributes: { stages: [], kinds: [] },
    };
    try {
      validateManifest(m);
      expect.fail("should throw");
    } catch (err) {
      expect(err).toBeInstanceOf(ManifestError);
      const e = err as ManifestError;
      // At minimum: manifestVersion, plugin.name, plugin.version,
      // plugin.apiVersion, runtime.kind, contributes
      expect(e.errors.length).toBeGreaterThanOrEqual(5);
    }
  });

  it("RESERVED_KIND_NAMES contains all 13 kernel kinds", () => {
    expect(RESERVED_KIND_NAMES.has("ContentSource")).toBe(true);
    expect(RESERVED_KIND_NAMES.has("Stream")).toBe(true);
    expect(RESERVED_KIND_NAMES.has("Void")).toBe(true);
    expect(RESERVED_KIND_NAMES.has("Bogus")).toBe(false);
  });
});
