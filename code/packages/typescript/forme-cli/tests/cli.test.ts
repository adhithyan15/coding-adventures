import { access, mkdir, rm, writeFile } from "node:fs/promises";
import { execFile } from "node:child_process";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { mkdtemp } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";
import { afterEach, describe, expect, it } from "vitest";
import { ConfigError, type PipelineConfig } from "@coding-adventures/forme-pipeline-config";
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
    createOrchestrator: () => ({
      buildPipeline: async value => {
        onBuild?.(value);
        return pipeline(value);
      },
      runOnce: async () => result,
      dispose: async () => {},
    } satisfies Orchestrator),
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

    const version = makeIO();
    expect(await run(["--version"], version)).toBe(EXIT_OK);
    expect(version.stdoutText).toBe("0.1.0\n");
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
      loadConfig: async () => config(),
      createOrchestrator: () => ({
        buildPipeline: async () => { throw new ConfigError([
          { path: "stages[0]", code: "BROKEN_ONE", message: "first" },
          { path: "stages[1]", code: "BROKEN_TWO", message: "second" },
        ]); },
        runOnce: async () => successfulResult(),
        dispose: async () => {},
      }),
    };
    expect(await run(["check"], io, {}, custom)).toBe(EXIT_USAGE_OR_CONFIG);
    expect(io.stderrText).toBe(
      "forme: BROKEN_ONE: stages[0]: first\nforme: BROKEN_TWO: stages[1]: second\n",
    );
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
    expect(report).toContain('"path": "index.html"');
    expect(report).toContain('"sha256": "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"');
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
