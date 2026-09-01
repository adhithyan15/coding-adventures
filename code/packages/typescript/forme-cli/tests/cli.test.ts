import { access, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { execFile } from "node:child_process";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { mkdtemp } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";
import { afterEach, describe, expect, it } from "vitest";
import { ConfigError, type PipelineConfig } from "@coding-adventures/forme-pipeline-config";
import { createCancellationTokenSource } from "@coding-adventures/forme-stage";
import type {
  Orchestrator,
  Pipeline,
  RunResult,
} from "@coding-adventures/forme-orchestrator";
import {
  EXIT_BUILD_FAILED,
  EXIT_OK,
  EXIT_USAGE_OR_CONFIG,
  run,
  type CliIO,
  type CliServices,
} from "../src/index.js";

interface MockIO extends CliIO {
  stdoutText: string;
  stderrText: string;
  readonly removed: string[];
  readonly written: Map<string, string>;
}

const roots: string[] = [];
const execFileAsync = promisify(execFile);

afterEach(async () => {
  for (const root of roots.splice(0)) await rm(root, { recursive: true, force: true });
});

function config(overrides: Partial<PipelineConfig> = {}): PipelineConfig {
  return {
    name: "fixture",
    settings: {
      storageRoot: ".",
      cacheDir: null,
      reproducibleBuild: false,
      maxConcurrency: null,
      logLevel: "info",
      bestEffort: false,
      deadlineMs: null,
    },
    stages: [],
    ...overrides,
  };
}

function makeIO(cwd = "/project", existing = new Set(["/project/forme.config.ts"])): MockIO {
  let current = cwd;
  const io: MockIO = {
    stdoutText: "",
    stderrText: "",
    removed: [],
    written: new Map(),
    stdout: { write(value) { io.stdoutText += value; } },
    stderr: { write(value) { io.stderrText += value; } },
    cwd: () => current,
    chdir: path => { current = path; },
    access: async path => {
      if (!existing.has(path)) throw new Error(`ENOENT: ${path}`);
    },
    remove: async path => { io.removed.push(path); },
    writeFile: async (path, value) => { io.written.set(path, value); },
  };
  return io;
}

function services(
  loaded: PipelineConfig,
  result: RunResult = successfulResult(),
  onBuild?: (value: PipelineConfig) => void,
  onCreate?: (cacheRoot: string | null) => void,
): CliServices {
  const pipeline = (value: PipelineConfig): Pipeline => ({
    config: value,
    dag: {
      instances: new Map(),
      topoOrder: value.stages.map((_, index) => `stage-${index}`),
      sinks: [],
      sources: [],
    },
  }) as never;
  return {
    loadConfig: async () => loaded,
    createOrchestrator: cacheRoot => {
      onCreate?.(cacheRoot);
      return {
        buildPipeline: async value => {
          onBuild?.(value);
          return pipeline(value);
        },
        runOnce: async () => result,
        watch: () => { throw new Error("watch not expected"); },
        dispose: async () => {},
      } satisfies Orchestrator;
    },
    startDevServer: async () => { throw new Error("dev server not expected"); },
    watchProject: () => { throw new Error("project watcher not expected"); },
  };
}

function successfulResult(): RunResult {
  return {
    outcome: "success",
    stages: [],
    outputs: { site: {} },
    errors: [],
    elapsedMs: 1,
    buildId: "blake2b:fixture" as never,
  };
}

describe("argument and diagnostic contracts", () => {
  it("prints help and version with stable success exits", async () => {
    const help = makeIO();
    expect(await run([], help)).toBe(EXIT_OK);
    expect(help.stdoutText).toContain("USAGE\n  forme [OPTIONS] [COMMAND]");
    expect(help.stdoutText).toContain("COMMANDS\n  build");
    expect(help.stdoutText).toContain("--config <PATH>");

    const buildHelp = makeIO();
    expect(await run(["build", "--help"], buildHelp)).toBe(EXIT_OK);
    expect(buildHelp.stdoutText).toContain("forme build [OPTIONS]");
    expect(buildHelp.stdoutText).toContain("--reproducible");
    expect(buildHelp.stdoutText).toContain("--report <PATH>");

    const watchHelp = makeIO();
    expect(await run(["watch", "--help"], watchHelp)).toBe(EXIT_OK);
    expect(watchHelp.stdoutText).toContain("forme watch [OPTIONS]");
    expect(watchHelp.stdoutText).toContain("--port <PORT>");
    expect(watchHelp.stdoutText).toContain("--debounce <MS>");

    const version = makeIO();
    expect(await run(["--version"], version)).toBe(EXIT_OK);
    expect(version.stdoutText).toBe("0.3.0\n");
  });

  it("rejects unknown commands, missing flag values, and invalid clean options", async () => {
    for (const argv of [["deploy"], ["build", "--config"], ["clean", "--reproducible"]]) {
      const io = makeIO();
      expect(await run(argv, io)).toBe(EXIT_USAGE_OR_CONFIG);
      expect(io.stderrText).toMatch(/^forme: E_USAGE:/);
    }
  });

  it("uses CLI Builder duplicate checks and fuzzy flag suggestions", async () => {
    const duplicate = makeIO();
    expect(await run(["build", "--report", "one.json", "--report", "two.json"], duplicate))
      .toBe(EXIT_USAGE_OR_CONFIG);
    expect(duplicate.stderrText).toContain("--report specified more than once");

    const flagTypo = makeIO();
    expect(await run(["build", "--reproducibl"], flagTypo)).toBe(EXIT_USAGE_OR_CONFIG);
    expect(flagTypo.stderrText).toContain("Did you mean '--reproducible'");
  });

  it("formats every ConfigError entry with its machine-readable code", async () => {
    const io = makeIO();
    const custom: CliServices = {
      ...services(config()),
      loadConfig: async () => config(),
      createOrchestrator: () => ({
        buildPipeline: async () => { throw new ConfigError([
          { path: "stages[0]", code: "BROKEN_ONE", message: "first" },
          { path: "stages[1]", code: "BROKEN_TWO", message: "second" },
        ]); },
        runOnce: async () => successfulResult(),
        watch: () => { throw new Error("watch not expected"); },
        dispose: async () => {},
      }),
    };
    expect(await run(["check"], io, {}, custom)).toBe(EXIT_USAGE_OR_CONFIG);
    expect(io.stderrText).toBe(
      "forme: BROKEN_ONE: stages[0]: first\nforme: BROKEN_TWO: stages[1]: second\n",
    );
  });
});

describe("watch preview", () => {
  it("serves successful artifacts, forwards options, and closes cleanly on SIGINT", async () => {
    const io = makeIO();
    const cancellation = createCancellationTokenSource();
    const bytes = new TextEncoder().encode("<!doctype html><body>preview</body>");
    const built: RunResult = {
      ...successfulResult(),
      outputs: {
        site: {
          variant: { kind: "dist-tree" },
          files: { "index.html": bytes },
          manifest: { routes: [], assets: [], buildTime: "fixed", buildId: "blake2b:preview" },
        },
      },
    };
    let publishedBuild: string | null = null;
    let serverClosed = false;
    let sessionStopped = false;
    let observedDebounce: number | undefined;
    let ignoredPaths: readonly string[] = [];
    let releasePublished: (() => void) | undefined;
    const published = new Promise<void>(resolve => { releasePublished = resolve; });
    const base = services(config());
    const custom: CliServices = {
      ...base,
      createOrchestrator: () => ({
        buildPipeline: async value => ({
          config: value,
          dag: { instances: new Map(), topoOrder: [], sinks: [], sources: [] },
        }) as never,
        runOnce: async () => built,
        watch: (_pipeline, options) => {
          observedDebounce = options.debounceMs;
          return {
            results: async function* () {
              yield built;
              await new Promise<void>(resolve => options.cancellation?.onCancel(resolve));
            },
            rebuild: async () => built,
            stop: async () => { sessionStopped = true; },
          };
        },
        dispose: async () => {},
      }),
      startDevServer: async options => {
        expect(options).toEqual({ port: 4321 });
        return {
          address: { host: "127.0.0.1", port: 4321, url: "http://127.0.0.1:4321" },
          publish: snapshot => {
            publishedBuild = snapshot.buildId;
            releasePublished?.();
          },
          publishFailure: () => {},
          close: async () => { serverClosed = true; },
        };
      },
      watchProject: (_root, ignored) => {
        ignoredPaths = ignored;
        return { async *[Symbol.asyncIterator]() { /* host mock stays idle */ } };
      },
    };

    const running = run(
      ["watch", "--port", "4321", "--debounce", "10"],
      io,
      { cancellation: cancellation.token },
      custom,
    );
    await published;
    cancellation.cancel("test SIGINT");
    expect(await running).toBe(130);
    expect(publishedBuild).toBe("blake2b:fixture");
    expect(observedDebounce).toBe(10);
    expect(ignoredPaths).toContain("/project/.git");
    expect(ignoredPaths).toContain("/project/node_modules");
    expect(serverClosed).toBe(true);
    expect(sessionStopped).toBe(true);
    expect(io.stdoutText).toContain("forme watch: http://127.0.0.1:4321");
    expect(io.stdoutText).toContain("forme watch: fixture ready");
  });

  it("validates bounded port and debounce values through the shared parser result", async () => {
    for (const argv of [
      ["watch", "--port", "0"],
      ["watch", "--port", "65536"],
      ["watch", "--debounce", "-1"],
      ["watch", "--debounce", "60001"],
    ]) {
      const io = makeIO();
      expect(await run(argv, io, {}, services(config()))).toBe(EXIT_USAGE_OR_CONFIG);
      expect(io.stderrText).toMatch(/^forme: E_USAGE:/);
    }
  });

  it("reports failed rebuilds, continues watching, and publishes the next good build", async () => {
    const io = makeIO();
    const cancellation = createCancellationTokenSource();
    const failed: RunResult = {
      ...successfulResult(),
      outcome: "failed",
      errors: [{
        stageName: "@fixture/parse",
        instanceId: "parse",
        code: "PARSE_BAD",
        message: "draft is invalid",
        recoverable: false,
        fields: {},
      }],
    };
    const good: RunResult = {
      ...successfulResult(),
      outputs: {
        site: {
          variant: { kind: "dist-tree" },
          files: { "index.html": new TextEncoder().encode("good") },
          manifest: { routes: [], assets: [], buildTime: "fixed", buildId: "blake2b:good" },
        },
      },
    };
    let publishedFailure = "";
    let publishedGood = false;
    let releaseGood: (() => void) | undefined;
    const goodPublished = new Promise<void>(resolve => { releaseGood = resolve; });
    const base = services(config());
    const custom: CliServices = {
      ...base,
      createOrchestrator: () => ({
        buildPipeline: async value => ({
          config: value,
          dag: { instances: new Map(), topoOrder: [], sinks: [], sources: [] },
        }) as never,
        runOnce: async () => good,
        watch: (_pipeline, options) => ({
          results: async function* () {
            yield failed;
            yield good;
            await new Promise<void>(resolve => options.cancellation?.onCancel(resolve));
          },
          rebuild: async () => good,
          stop: async () => {},
        }),
        dispose: async () => {},
      }),
      startDevServer: async () => ({
        address: { host: "127.0.0.1", port: 3000, url: "http://127.0.0.1:3000" },
        publish: () => {
          publishedGood = true;
          releaseGood?.();
        },
        publishFailure: failure => { publishedFailure = failure.message; },
        close: async () => {},
      }),
      watchProject: () => ({ async *[Symbol.asyncIterator]() { /* idle */ } }),
    };

    const running = run(["watch"], io, { cancellation: cancellation.token }, custom);
    await goodPublished;
    cancellation.cancel("test complete");
    expect(await running).toBe(130);
    expect(publishedFailure).toContain("PARSE_BAD: draft is invalid");
    expect(publishedGood).toBe(true);
    expect(io.stderrText).toContain("forme: PARSE_BAD: parse (@fixture/parse): draft is invalid");
  });
});

describe("build and check", () => {
  it("checks the typed DAG without running stages", async () => {
    const io = makeIO();
    const loaded = config({ stages: [{} as never, {} as never] });
    expect(await run(["check"], io, {}, services(loaded))).toBe(EXIT_OK);
    expect(io.stdoutText).toBe("forme check: fixture is valid (2 stages)\n");
  });

  it("overrides reproducible mode without mutating the loaded config", async () => {
    const io = makeIO();
    const loaded = config();
    let observed: PipelineConfig | undefined;
    expect(await run(["build", "--reproducible"], io, {}, services(
      loaded,
      successfulResult(),
      value => { observed = value; },
    ))).toBe(EXIT_OK);
    expect(observed?.settings.reproducibleBuild).toBe(true);
    expect(loaded.settings.reproducibleBuild).toBe(false);
    expect(io.stdoutText).toContain("forme build: fixture success");
  });

  it("accepts the FM03 forme run spelling as a build alias", async () => {
    const io = makeIO();
    expect(await run(["run"], io, {}, services(config()))).toBe(EXIT_OK);
    expect(io.stdoutText).toContain("forme build: fixture success");
  });

  it("writes a deterministic manifest and file-hash report", async () => {
    const io = makeIO();
    const bytes = new TextEncoder().encode("hello");
    const reported: RunResult = {
      ...successfulResult(),
      stages: [{
        instanceId: "render",
        stageName: "@fixture/render",
        itemsConsumed: 1,
        itemsProduced: 1,
        elapsedMs: 5,
        cacheHits: 1,
        cacheMisses: 0,
        outcome: "success",
        errorCount: 0,
      }],
      outputs: {
        site: {
          variant: { kind: "dist-tree" },
          files: { "index.html": bytes },
          manifest: { routes: [], assets: [], buildTime: "fixed", buildId: "blake2b:site" },
        },
      },
    };
    expect(await run(
      ["build", "--report", "dist/report.json"],
      io,
      {},
      services(config(), reported),
    )).toBe(EXIT_OK);
    const report = io.written.get("/project/dist/report.json");
    expect(report).toContain('"schemaVersion": 1');
    expect(report).toContain('"cacheHits": 1');
    expect(report).not.toContain('"elapsedMs"');
    expect(report).toContain('"path": "index.html"');
    expect(report).toContain('"sha256": "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"');
  });

  it("resolves configured cache roots beneath the project and preserves null", async () => {
    const roots: Array<string | null> = [];
    expect(await run(["check"], makeIO(), {}, services(
      config({ settings: { ...config().settings, cacheDir: ".forme/cache" } }),
      successfulResult(),
      undefined,
      value => roots.push(value),
    ))).toBe(EXIT_OK);
    expect(await run(["check"], makeIO(), {}, services(
      config(),
      successfulResult(),
      undefined,
      value => roots.push(value),
    ))).toBe(EXIT_OK);
    expect(roots).toEqual(["/project/.forme/cache", null]);
  });

  it("refuses an outside-project cache before constructing the orchestrator", async () => {
    const io = makeIO();
    let constructed = false;
    expect(await run(["build"], io, {}, services(
      config({ settings: { ...config().settings, cacheDir: "../escape" } }),
      successfulResult(),
      undefined,
      () => { constructed = true; },
    ))).toBe(EXIT_USAGE_OR_CONFIG);
    expect(constructed).toBe(false);
    expect(io.stderrText).toContain("refusing to use unsafe cache path");
  });

  it("returns the build-failure exit and stable stage diagnostics", async () => {
    const io = makeIO();
    const failed: RunResult = {
      ...successfulResult(),
      outcome: "failed",
      errors: [{
        stageName: "@fixture/render",
        instanceId: "render",
        code: "RENDER_BAD",
        message: "could not render",
        recoverable: false,
        fields: {},
      }],
    };
    expect(await run(["build"], io, {}, services(config(), failed))).toBe(EXIT_BUILD_FAILED);
    expect(io.stderrText).toBe(
      "forme: RENDER_BAD: render (@fixture/render): could not render\n",
    );
  });

  it("uses exit 130 for a cooperatively cancelled run", async () => {
    const io = makeIO();
    const cancelled: RunResult = {
      ...successfulResult(),
      outcome: "cancelled",
      errors: [{
        stageName: "@fixture/source",
        instanceId: "source",
        code: "CANCELLED",
        message: "interrupted by SIGINT",
        recoverable: false,
        fields: {},
      }],
    };
    expect(await run(["build"], io, {}, services(config(), cancelled))).toBe(130);
    expect(io.stderrText).toContain("forme: CANCELLED:");
  });
});

describe("clean", () => {
  it("removes only cache and DeployArtifact outDir paths beneath the project", async () => {
    const io = makeIO();
    const deployStage = {
      produces: { name: "DeployArtifact", version: "1.0" },
    };
    const transformStage = {
      produces: { name: "RenderedPage", version: "1.1" },
    };
    const loaded = config({
      settings: { ...config().settings, cacheDir: ".forme/cache" },
      stages: [
        { stage: deployStage, config: { outDir: "dist" } } as never,
        { stage: transformStage, config: { outDir: "not-output" } } as never,
        { stage: deployStage, config: { outDir: "dist" } } as never,
      ],
    });
    expect(await run(["clean"], io, {}, services(loaded))).toBe(EXIT_OK);
    expect(io.removed).toEqual(["/project/.forme/cache", "/project/dist"]);
    expect(io.stdoutText).toBe("forme clean: removed 2 configured paths\n");
  });

  it("refuses project-root and outside-project deletion targets", async () => {
    const deployStage = { produces: { name: "DeployArtifact", version: "1.0" } };
    for (const outDir of [".", "../other"]) {
      const io = makeIO();
      const loaded = config({
        stages: [{ stage: deployStage, config: { outDir } } as never],
      });
      expect(await run(["clean"], io, {}, services(loaded))).toBe(EXIT_USAGE_OR_CONFIG);
      expect(io.removed).toEqual([]);
      expect(io.stderrText).toContain("refusing to clean unsafe path");
    }
  });
});

describe("installed-project shape", () => {
  it("loads and builds a self-contained config outside the repository tree", async () => {
    const root = await mkdtemp(join(tmpdir(), "forme-cli-external-"));
    roots.push(root);
    const configPath = join(root, "forme.config.mjs");
    await writeFile(configPath, externalConfigSource(), "utf8");

    let current = root;
    const io: MockIO = {
      stdoutText: "",
      stderrText: "",
      removed: [],
      written: new Map(),
      stdout: { write(value) { io.stdoutText += value; } },
      stderr: { write(value) { io.stderrText += value; } },
      cwd: () => current,
      chdir: path => { current = path; },
      access,
      remove: async path => { io.removed.push(path); await rm(path, { recursive: true, force: true }); },
      writeFile: (path, value) => writeFile(path, value, "utf8"),
    };

    expect(await run(["check"], io), io.stderrText).toBe(EXIT_OK);
    expect(await run(["build", "--reproducible"], io), io.stderrText).toBe(EXIT_OK);
    expect(io.stdoutText).toContain("external-project is valid");
    expect(io.stdoutText).toContain("external-project success");

    await mkdir(join(root, "dist"));
    await writeFile(join(root, "dist", "stale.txt"), "stale");
    expect(await run(["clean"], io)).toBe(EXIT_OK);
    await expect(access(join(root, "dist"))).rejects.toThrow();
    expect(current).toBe(root);
  });

  it("executes the npm bin launcher with an external working directory", async () => {
    const root = await mkdtemp(join(tmpdir(), "forme-cli-bin-external-"));
    roots.push(root);
    await writeFile(join(root, "forme.config.mjs"), externalConfigSource(), "utf8");
    const bin = fileURLToPath(new URL("../bin/forme.mjs", import.meta.url));

    const checked = await execFileAsync(process.execPath, [bin, "check"], { cwd: root });
    expect(checked.stderr).toBe("");
    expect(checked.stdout).toContain("external-project is valid (1 stage)");

    const built = await execFileAsync(process.execPath, [bin, "build", "--reproducible"], { cwd: root });
    expect(built.stderr).toBe("");
    expect(built.stdout).toContain("external-project success");
  });

  it("reuses pure stages across CLI processes and clean invalidates the project cache", async () => {
    const root = await mkdtemp(join(tmpdir(), "forme-cli-cache-"));
    roots.push(root);
    const configPath = join(root, "forme.config.mjs");
    const bin = fileURLToPath(new URL("../bin/forme.mjs", import.meta.url));
    const build = async (report: string) => {
      const result = await execFileAsync(process.execPath, [bin, "build", "--report", report], { cwd: root });
      expect(result.stderr).toBe("");
      return JSON.parse(await readFile(join(root, report), "utf8")) as {
        stages: Array<{ instanceId: string; cacheHits: number; cacheMisses: number }>;
      };
    };
    const stage = (report: Awaited<ReturnType<typeof build>>, id: string) =>
      report.stages.find(value => value.instanceId === id)!;

    await writeFile(configPath, persistentCacheConfigSource("hello"), "utf8");
    const first = await build("first.json");
    expect(stage(first, "transform")).toMatchObject({ cacheHits: 0, cacheMisses: 1 });
    expect(stage(first, "emit")).toMatchObject({ cacheHits: 0, cacheMisses: 1 });
    await access(join(root, ".forme/cache"));

    const second = await build("second.json");
    expect(stage(second, "transform")).toMatchObject({ cacheHits: 1, cacheMisses: 0 });
    expect(stage(second, "emit")).toMatchObject({ cacheHits: 1, cacheMisses: 0 });

    await writeFile(configPath, persistentCacheConfigSource("changed"), "utf8");
    const changed = await build("changed.json");
    expect(stage(changed, "transform")).toMatchObject({ cacheHits: 0, cacheMisses: 1 });
    expect(stage(changed, "emit")).toMatchObject({ cacheHits: 0, cacheMisses: 1 });

    const cleaned = await execFileAsync(process.execPath, [bin, "clean"], { cwd: root });
    expect(cleaned.stderr).toBe("");
    await expect(access(join(root, ".forme/cache"))).rejects.toThrow();
    const afterClean = await build("after-clean.json");
    expect(stage(afterClean, "transform")).toMatchObject({ cacheHits: 0, cacheMisses: 1 });
    expect(stage(afterClean, "emit")).toMatchObject({ cacheHits: 0, cacheMisses: 1 });
  });
});

function externalConfigSource(): string {
  return `
const emit = {
  name: "@fixture/emit",
  version: "0.1.0",
  apiVersion: 1,
  description: "self-contained external fixture",
  consumes: { name: "Void", version: "1.0" },
  produces: { name: "DeployArtifact", version: "1.0" },
  capabilities: [],
  configSchema: {
    type: "object",
    required: ["outDir"],
    properties: { outDir: { type: "string" } },
  },
  async run() {
    return {
      variant: { kind: "dist-tree" },
      files: {},
      manifest: {
        routes: [], assets: [], buildTime: "2026-08-28T00:00:00.000Z", buildId: "blake2b:fixture",
      },
    };
  },
};
export default {
  name: "external-project",
  settings: {
    storageRoot: ".", cacheDir: null, reproducibleBuild: false,
    maxConcurrency: null, logLevel: "info", bestEffort: false, deadlineMs: null,
  },
  stages: [{ id: "emit", stage: emit, config: { outDir: "dist" } }],
  outputs: [{ fromInstance: "emit", name: "site" }],
};
`;
}

function persistentCacheConfigSource(value: string): string {
  return `
const contentSource = { name: "ContentSource", version: "1.0" };
const source = {
  name: "@fixture/cache-source", version: "0.1.0", apiVersion: 1,
  description: "external cache source", consumes: { name: "Void", version: "1.0" },
  produces: { name: "Stream", version: "1.0", inner: contentSource },
  capabilities: [], configSchema: null,
  async *run() {
    const value = ${JSON.stringify(value)};
    yield {
      path: "page.txt", bytes: new TextEncoder().encode(value), mimeType: "text/plain",
      identity: "01952c0d-7e63-7000-8000-000000000000",
      revision: "blake2b:" + value, providerMeta: {},
    };
  },
};
const transform = {
  name: "@fixture/cache-transform", version: "0.1.0", apiVersion: 1,
  description: "pure cache transform", consumes: contentSource, produces: contentSource,
  capabilities: [], configSchema: null,
  async run(input) { return { ...input, path: input.path.toUpperCase() }; },
};
const emit = {
  name: "@fixture/cache-emit", version: "0.1.0", apiVersion: 1,
  description: "pure in-memory emitter", consumes: contentSource,
  produces: { name: "DeployArtifact", version: "1.0" }, capabilities: [],
  configSchema: { type: "object", required: ["outDir"], properties: { outDir: { type: "string" } } },
  async run(input) {
    return {
      variant: { kind: "dist-tree" }, files: { [input.path]: input.bytes },
      manifest: { routes: [], assets: [], buildTime: "fixed", buildId: "blake2b:fixture" },
    };
  },
};
export default {
  name: "persistent-cache-project",
  settings: {
    storageRoot: ".", cacheDir: ".forme/cache", reproducibleBuild: true,
    maxConcurrency: null, logLevel: "info", bestEffort: false, deadlineMs: null,
  },
  stages: [
    { id: "source", stage: source },
    { id: "transform", stage: transform },
    { id: "emit", stage: emit, config: { outDir: "dist" } },
  ],
  wires: [
    { from: { id: "source" }, to: { id: "transform" } },
    { from: { id: "transform" }, to: { id: "emit" } },
  ],
  outputs: [{ fromInstance: "emit", name: "site" }],
};
`;
}
