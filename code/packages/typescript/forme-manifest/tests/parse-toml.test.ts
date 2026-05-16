import { describe, it, expect } from "vitest";
import { parseManifest, ManifestError } from "../src/index.js";

const FULL_MANIFEST = `
manifestVersion = 1

[plugin]
name        = "@forme/parse-markdown"
version     = "1.4.2"
apiVersion  = 1
description = "Parse CommonMark + GFM into a ContentNode"
license     = "MIT"
authors     = ["Alice <alice@example.com>"]
homepage    = "https://example.com/parse-markdown"
repository  = "https://github.com/alice/parse-markdown"

[runtime]
kind  = "node"
entry = "./entry.js"

[[capabilities.required]]
realm  = "filesystem"
scope  = "read"
detail = "$storageRoot"
reason = "Read source files to parse"

[[capabilities.optional]]
realm  = "system"
scope  = "time"
detail = "wallclock"
reason = "Use file mtime for cache invalidation"

[[contributes.stages]]
id           = "parse-markdown"
consumes     = "ContentSource"
produces     = "ContentNode"
configSchema = "./schemas/config.json"

[[contributes.kinds]]
name      = "ext:youtube-embed"
version   = "1.0"
schema    = "./schemas/youtube-embed.json"
subtypeOf = "ContentNode"

[resources]
maxMemoryMb        = 512
maxWallClockMs     = 30000
maxFileDescriptors = 256
maxConcurrentRpcs  = 64
`;

describe("parseManifest — happy path", () => {
  it("parses a full manifest into the expected shape", () => {
    const m = parseManifest(FULL_MANIFEST);

    expect(m.manifestVersion).toBe(1);
    expect(m.plugin.name).toBe("@forme/parse-markdown");
    expect(m.plugin.version).toBe("1.4.2");
    expect(m.plugin.apiVersion).toBe(1);
    expect(m.plugin.description).toBe("Parse CommonMark + GFM into a ContentNode");
    expect(m.plugin.authors).toEqual(["Alice <alice@example.com>"]);

    expect(m.runtime.kind).toBe("node");
    expect(m.runtime.entry).toBe("./entry.js");

    expect(m.capabilities.required).toHaveLength(1);
    expect(m.capabilities.required[0]!.realm).toBe("filesystem");
    expect(m.capabilities.required[0]!.detail).toBe("$storageRoot");
    expect(m.capabilities.optional).toHaveLength(1);

    expect(m.contributes.stages).toHaveLength(1);
    expect(m.contributes.stages[0]!.id).toBe("parse-markdown");
    expect(m.contributes.kinds).toHaveLength(1);
    expect(m.contributes.kinds[0]!.name).toBe("ext:youtube-embed");

    expect(m.resources?.maxMemoryMb).toBe(512);
  });

  it("parses minimal manifest (no resources, no signature, no optional caps)", () => {
    const m = parseManifest(`
manifestVersion = 1
[plugin]
name       = "x"
version    = "0.1.0"
apiVersion = 1
[runtime]
kind  = "node"
entry = "./e.js"
[[contributes.stages]]
id       = "s"
consumes = "ContentSource"
produces = "ContentNode"
`);
    expect(m.manifestVersion).toBe(1);
    expect(m.resources).toBeUndefined();
    expect(m.signature).toBeUndefined();
    expect(m.capabilities.required).toEqual([]);
    expect(m.capabilities.optional).toEqual([]);
  });

  it("strips comments", () => {
    const m = parseManifest(`
# top-level comment
manifestVersion = 1  # trailing comment
[plugin]              # section comment
name = "x" # value comment
version = "0.1.0"
apiVersion = 1
[runtime]
kind = "node"
entry = "./e.js"
[[contributes.stages]]
id = "s"
consumes = "ContentSource"
produces = "ContentNode"
`);
    expect(m.plugin.name).toBe("x");
  });

  it("handles escape sequences in double-quoted strings", () => {
    const m = parseManifest(`
manifestVersion = 1
[plugin]
name = "x"
version = "0.1.0"
apiVersion = 1
description = "line1\\nline2\\t\\u0041"
[runtime]
kind = "node"
entry = "./e.js"
[[contributes.stages]]
id = "s"
consumes = "ContentSource"
produces = "ContentNode"
`);
    expect(m.plugin.description).toBe("line1\nline2\tA");
  });

  it("treats single-quoted strings as literals (no escape processing)", () => {
    const m = parseManifest(`
manifestVersion = 1
[plugin]
name = 'x'
version = "0.1.0"
apiVersion = 1
description = 'no \\n escape here'
[runtime]
kind = "node"
entry = "./e.js"
[[contributes.stages]]
id = "s"
consumes = "ContentSource"
produces = "ContentNode"
`);
    expect(m.plugin.description).toBe("no \\n escape here");
  });

  it("accepts signed integers", () => {
    const m = parseManifest(`
manifestVersion = 1
[plugin]
name = "x"
version = "0.1.0"
apiVersion = 1
[runtime]
kind = "node"
entry = "./e.js"
[[contributes.stages]]
id = "s"
consumes = "ContentSource"
produces = "ContentNode"
[resources]
maxMemoryMb = +512
`);
    expect(m.resources?.maxMemoryMb).toBe(512);
  });

  it("accepts boolean values", () => {
    // Booleans aren't directly used in plugin.toml's required fields,
    // but the parser must support them as a TOML primitive.
    expect(() => parseManifest(`
manifestVersion = 1
custom = true
other = false
[plugin]
name = "x"
version = "0.1.0"
apiVersion = 1
[runtime]
kind = "node"
entry = "./e.js"
[[contributes.stages]]
id = "s"
consumes = "ContentSource"
produces = "ContentNode"
`)).not.toThrow();
  });

  it("accepts arrays of strings", () => {
    const m = parseManifest(`
manifestVersion = 1
[plugin]
name = "x"
version = "0.1.0"
apiVersion = 1
authors = ["Alice", "Bob", "Carol"]
[runtime]
kind = "node"
entry = "./e.js"
[[contributes.stages]]
id = "s"
consumes = "ContentSource"
produces = "ContentNode"
`);
    expect(m.plugin.authors).toEqual(["Alice", "Bob", "Carol"]);
  });

  it("accepts arrays spanning multiple lines", () => {
    const m = parseManifest(`
manifestVersion = 1
[plugin]
name = "x"
version = "0.1.0"
apiVersion = 1
authors = [
  "Alice",
  "Bob",
]
[runtime]
kind = "node"
entry = "./e.js"
[[contributes.stages]]
id = "s"
consumes = "ContentSource"
produces = "ContentNode"
`);
    expect(m.plugin.authors).toEqual(["Alice", "Bob"]);
  });

  it("accepts platform map via dotted keys", () => {
    const m = parseManifest(`
manifestVersion = 1
[plugin]
name = "x"
version = "0.1.0"
apiVersion = 1
[runtime]
kind = "binary"
entry = "./fallback"
platforms.linux-x86_64  = "./bin/linux"
platforms.darwin-x86_64 = "./bin/macos"
[[contributes.stages]]
id = "s"
consumes = "ContentSource"
produces = "ContentNode"
`);
    expect(m.runtime.platforms).toEqual({
      "linux-x86_64": "./bin/linux",
      "darwin-x86_64": "./bin/macos",
    });
  });

  it("parses [[array.of.tables]] with multiple entries", () => {
    const m = parseManifest(`
manifestVersion = 1
[plugin]
name = "x"
version = "0.1.0"
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
    expect(m.capabilities.required).toHaveLength(2);
    expect(m.capabilities.required[0]!.reason).toBe("first");
    expect(m.capabilities.required[1]!.reason).toBe("second");
  });
});

describe("parseManifest — error paths", () => {
  it("rejects non-string input", () => {
    expect(() => parseManifest(null as unknown as string)).toThrow(ManifestError);
    expect(() => parseManifest(123 as unknown as string)).toThrow(ManifestError);
  });

  it("rejects multi-line strings with a clear error", () => {
    expect(() => parseManifest(`x = """hello"""`)).toThrow(/multi-line strings/);
  });

  it("rejects inline tables", () => {
    expect(() => parseManifest(`x = { a = 1 }`)).toThrow(/inline tables/);
  });

  it("rejects floats and exponents", () => {
    expect(() => parseManifest(`x = 1.5`)).toThrow(/floating-point/);
    expect(() => parseManifest(`x = 1e10`)).toThrow(/floating-point/);
  });

  it("rejects underscore digit separators", () => {
    expect(() => parseManifest(`x = 1_000`)).toThrow(/underscore/);
  });

  it("rejects unterminated strings", () => {
    expect(() => parseManifest(`x = "no closing quote`)).toThrow(/not closed/);
  });

  it("rejects unknown escape sequences", () => {
    expect(() => parseManifest(`x = "bad \\q"`)).toThrow(/unsupported escape/);
  });

  it("rejects malformed unicode escape", () => {
    expect(() => parseManifest(`x = "\\uZZZZ"`)).toThrow(/hexadecimal/);
  });

  it("rejects duplicate keys", () => {
    expect(() => parseManifest(`
x = 1
x = 2
`)).toThrow(/already assigned/);
  });

  it("rejects duplicate sections", () => {
    expect(() => parseManifest(`
[plugin]
name = "x"
[plugin]
version = "0.1.0"
`)).toThrow(/more than once/);
  });

  it("rejects redeclaring a table as array-of-tables", () => {
    expect(() => parseManifest(`
[foo]
x = 1
[[foo]]
y = 2
`)).toThrow(/array.*previously declared as.*table|previously declared as/);
  });

  it("rejects heterogeneous arrays", () => {
    expect(() => parseManifest(`x = ["a", 1]`)).toThrow(/heterogeneous/);
  });

  it("rejects unterminated arrays", () => {
    expect(() => parseManifest(`x = ["a"`)).toThrow(/unterminated array/);
  });

  it("rejects malformed array element separator", () => {
    expect(() => parseManifest(`x = ["a" "b"]`)).toThrow(/expected ',' or ']'/);
  });

  it("reports the line and column on syntax errors", () => {
    try {
      parseManifest(`x = 1\ny = "unterminated\n`);
      expect.fail("should have thrown");
    } catch (err) {
      expect((err as Error).message).toMatch(/line 2/);
    }
  });

  it("rejects bare $ in value position (not a TOML feature)", () => {
    expect(() => parseManifest(`x = $bad`)).toThrow();
  });

  it("rejects empty value", () => {
    expect(() => parseManifest(`x =`)).toThrow();
  });

  it("rejects bare sign with no digits", () => {
    expect(() => parseManifest(`x = -`)).toThrow(/decimal digit/);
  });

  it("rejects descending into a non-table key", () => {
    expect(() => parseManifest(`
x = 1
x.y = 2
`)).toThrow(/cannot descend|already has/);
  });
});

describe("parseManifest — security: prototype pollution defences", () => {
  it("rejects [__proto__] section header", () => {
    expect(() => parseManifest(`
[__proto__]
polluted = "yes"
`)).toThrowError(/reserved|__proto__/);
  });

  it("rejects __proto__ as a key", () => {
    expect(() => parseManifest(`__proto__ = "x"`))
      .toThrowError(/reserved|__proto__/);
  });

  it("rejects __proto__.x dotted key", () => {
    expect(() => parseManifest(`__proto__.polluted = "x"`))
      .toThrowError(/reserved|__proto__/);
  });

  it("rejects constructor key", () => {
    expect(() => parseManifest(`constructor = "x"`))
      .toThrowError(/reserved|constructor/);
  });

  it("rejects prototype key", () => {
    expect(() => parseManifest(`prototype = "x"`))
      .toThrowError(/reserved|prototype/);
  });

  it("rejects [[__proto__]] array-of-tables header", () => {
    expect(() => parseManifest(`
[[__proto__]]
x = 1
`)).toThrowError(/reserved|__proto__/);
  });

  it("parsing does NOT pollute Object.prototype", () => {
    // Defence-in-depth check: even if the denylist were bypassed,
    // every internal table is `Object.create(null)` so pollution
    // is mechanically impossible.  Use a fresh `{}` for the
    // observation — if Object.prototype had been polluted, this
    // would inherit the polluted key.
    const before = Object.keys(Object.prototype).length;
    try {
      parseManifest(`
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
`);
    } catch { /* not relevant */ }
    const after = Object.keys(Object.prototype).length;
    expect(after).toBe(before);
    expect(({} as { polluted?: string }).polluted).toBeUndefined();
  });
});
