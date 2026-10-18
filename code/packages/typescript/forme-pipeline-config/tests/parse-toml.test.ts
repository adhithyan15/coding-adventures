import { describe, expect, it } from "vitest";
import { loadTsConfig, parseTomlConfig, type PipelineConfig } from "../src/index.js";

const COMPLETE = String.raw`
name = "site"

[settings]
storage-root = "./content"
cache-dir = "./.forme/cache"
reproducible-build = true
max-concurrency = 4
log-level = "debug"
best-effort = false
deadline-ms = 2500

[settings.plugin-runtimes.python]
executable = "/opt/python/bin/python3"
root = "/opt/python"

[[stages]]
id = "source"
stage = "@forme/source-fs"
export = "source"
capabilities = ["storage:read"]
config = { glob = "**/*.md", nested = { enabled = true }, weights = [1, 2.5] }

[[stages]]
id = "emit"
stage = "@forme/emit-fs"
config = { out = "./dist" }

[[wires]]
from = { id = "source" }
to = { id = "emit", port = "pages" }

[[outputs]]
from-instance = "emit"
name = "web"
`;

describe("parseTomlConfig", () => {
  it("lowers every declarative surface to the canonical PipelineConfig shape", () => {
    expect(parseTomlConfig(COMPLETE)).toEqual({
      name: "site",
      settings: {
        storageRoot: "./content",
        cacheDir: "./.forme/cache",
        reproducibleBuild: true,
        maxConcurrency: 4,
        logLevel: "debug",
        bestEffort: false,
        deadlineMs: 2500,
        pluginRuntimes: {
          python: { executable: "/opt/python/bin/python3", root: "/opt/python" },
        },
      },
      stages: [
        {
          id: "source",
          stage: { kind: "stage-ref", packageName: "@forme/source-fs", export: "source" },
          capabilities: ["storage:read"],
          config: {
            glob: "**/*.md",
            nested: { enabled: true },
            weights: [1, 2.5],
          },
        },
        {
          id: "emit",
          stage: { kind: "stage-ref", packageName: "@forme/emit-fs" },
          config: { out: "./dist" },
        },
      ],
      wires: [{ from: { id: "source" }, to: { id: "emit", port: "pages" } }],
      outputs: [{ fromInstance: "emit", name: "web" }],
    });
  });

  it("is differential-equivalent to the TypeScript loader", async () => {
    const declarative = parseTomlConfig(COMPLETE);
    const canonical = await loadTsConfig("/project/forme.config.ts", {
      importModule: async () => ({ default: declarative satisfies PipelineConfig }),
    });
    expect(declarative).toEqual(canonical);
  });

  it("fills TOML's omitted nullable settings with canonical nulls", () => {
    const parsed = parseTomlConfig(`name = "minimal"
[settings]
storage-root = "."
cache-dir = "cache"
reproducible-build = false
log-level = "info"
best-effort = false
[[stages]]
stage = "@example/source"
`);
    expect(parsed.settings.maxConcurrency).toBeNull();
    expect(parsed.settings.deadlineMs).toBeNull();
    expect(parsed.wires).toBeUndefined();
    expect(parsed.outputs).toBeUndefined();
  });

  it("supports table-form stage config and quoted/dotted config keys", () => {
    const parsed = parseTomlConfig(`name = "tables"
[settings]
storage-root = "."
cache-dir = "cache"
reproducible-build = false
log-level = "info"
best-effort = false
[[stages]]
stage = "@example/source"
[stages.config]
"content-type" = "text/plain"
nested.answer = 42
`);
    expect(parsed.stages[0]?.config).toEqual({
      "content-type": "text/plain",
      nested: { answer: 42 },
    });
  });

  it.each([
    ['name = "one"\nname = "two"', /already assigned|duplicate/i],
    ['[settings]\n[settings]', /declared more than once|duplicate/i],
    ['settings = "scalar"\n[settings]', /already has|redeclare|descend/i],
  ])("rejects duplicate or conflicting TOML paths", (fragment, message) => {
    expect(() => parseTomlConfig(fragment)).toThrow(message);
  });

  it.each([
    ['extra = true\n', /extra.*not supported/i],
    ['', /settings\.extra.*not supported/i, '[settings]\nextra = true\n'],
    ['', /stages\[0\]\.extra.*not supported/i, '[[stages]]\nstage = "@example/source"\nextra = true\n'],
    ['', /wires\[0\]\.from\.extra.*not supported/i, '[[wires]]\nfrom = { id = "a", extra = "x" }\nto = { id = "b" }\n'],
    ['', /outputs\[0\]\.extra.*not supported/i, '[[outputs]]\nfrom-instance = "a"\nname = "web"\nextra = true\n'],
  ])("rejects unknown keys in closed tables", (rootFragment, message, trailingFragment = "") => {
    const source = `${rootFragment}name = "closed"
[settings]
storage-root = "."
cache-dir = "cache"
reproducible-build = false
log-level = "info"
best-effort = false
[[stages]]
stage = "@example/source"
${trailingFragment}`;
    expect(() => parseTomlConfig(source)).toThrow(message);
  });

  it.each([
    ['name = 7', /name.*string/i],
    [`name = "bad-setting"
[settings]
storage-root = "."
cache-dir = "cache"
reproducible-build = false
max-concurrency = 0
log-level = "info"
best-effort = false
[[stages]]
stage = "@example/source"`, /max-concurrency.*positive safe integer/i],
    [`name = "bad-stage"
[settings]
storage-root = "."
cache-dir = "cache"
reproducible-build = false
log-level = "info"
best-effort = false
[[stages]]
stage = 7`, /stages\[0\]\.stage.*string/i],
    [`name = "bad-capability"
[settings]
storage-root = "."
cache-dir = "cache"
reproducible-build = false
log-level = "info"
best-effort = false
[[stages]]
stage = "@example/source"
capabilities = ["ok", 7]`, /capabilities.*string/i],
    [`name = "bad-wire"
[settings]
storage-root = "."
cache-dir = "cache"
reproducible-build = false
log-level = "info"
best-effort = false
[[stages]]
stage = "@example/source"
[[wires]]
from = { id = "a" }`, /wires\[0\]\.to.*table/i],
  ])("rejects declarative schema mismatches", (fragment, message) => {
    expect(() => parseTomlConfig(fragment)).toThrow(message);
  });

  it("rejects unsafe key segments without changing Object.prototype", () => {
    const before = Object.keys(Object.prototype);
    expect(() => parseTomlConfig('[__proto__]\npolluted = "yes"')).toThrow(/reserved/i);
    expect(Object.keys(Object.prototype)).toEqual(before);
    expect((Object.prototype as { polluted?: unknown }).polluted).toBeUndefined();
  });

  it("enforces source, token, nesting, string, and collection bounds", () => {
    expect(() => parseTomlConfig("#".repeat(1_048_577))).toThrow(/source.*1048576/i);
    expect(() => parseTomlConfig(`name = "${"x".repeat(65_537)}"`)).toThrow(/string.*65536/i);
    expect(() => parseTomlConfig(`name = "deep"\nvalue = ${"[".repeat(65)}0${"]".repeat(65)}`))
      .toThrow(/nesting.*64/i);
    expect(() => parseTomlConfig(`name = "many"\nvalues = [${Array.from({ length: 4097 }, () => "0").join(",")}]`))
      .toThrow(/collection.*4096/i);
  });

  it("rejects non-finite values before they enter stage configuration", () => {
    expect(() => parseTomlConfig(`name = "finite"\nvalue = inf`)).toThrow(/finite/i);
  });
});
