import { createHash } from "node:crypto";
import { access, mkdir, rm, writeFile } from "node:fs/promises";
import { dirname, isAbsolute, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  ParseErrors,
  Parser,
  SpecError,
  type ParseResult,
} from "@coding-adventures/cli-builder";
import {
  snapshotFromOutputs,
  startDevServer,
  type DevServer,
} from "@coding-adventures/forme-dev-server";
import {
  createOrchestrator,
  type Orchestrator,
  type Pipeline,
  type RunResult,
} from "@coding-adventures/forme-orchestrator";
import {
  ConfigError,
  isStageRef,
  loadTsConfig,
  type PipelineConfig,
} from "@coding-adventures/forme-pipeline-config";
import {
  silentLogger,
  type CancellationToken,
} from "@coding-adventures/forme-stage";
import { watchProject } from "./project-watcher.js";

export const EXIT_OK = 0;
export const EXIT_BUILD_FAILED = 1;
export const EXIT_USAGE_OR_CONFIG = 2;
export const EXIT_CANCELLED = 130;

const DEFAULT_CONFIG_NAMES = [
  "forme.config.ts",
  "forme.config.mts",
  "forme.config.js",
  "forme.config.mjs",
] as const;

const CLI_SPEC_PATH = fileURLToPath(new URL("../forme.cli.json", import.meta.url));

export interface CliIO {
  readonly stdout: { write(value: string): unknown };
  readonly stderr: { write(value: string): unknown };
  cwd(): string;
  chdir(path: string): void;
  access(path: string): Promise<void>;
  remove(path: string): Promise<void>;
  writeFile(path: string, value: string): Promise<void>;
}

export interface CliServices {
  loadConfig(path: string): Promise<PipelineConfig>;
  createOrchestrator(): Orchestrator;
  startDevServer(options: { readonly port: number }): Promise<DevServer>;
  watchProject(root: string, ignoredPaths: readonly string[]): AsyncIterable<unknown>;
}

export interface RunCliOptions {
  readonly cancellation?: CancellationToken;
}

interface ParsedArgs {
  readonly command: "build" | "check" | "clean" | "watch";
  readonly configPath: string | null;
  readonly reproducible: boolean;
  readonly reportPath: string | null;
  readonly port: number;
  readonly debounceMs: number;
}

const defaultIO: CliIO = {
  stdout: process.stdout,
  stderr: process.stderr,
  cwd: () => process.cwd(),
  chdir: path => process.chdir(path),
  access,
  remove: path => rm(path, { recursive: true, force: true }),
  writeFile: async (path, value) => {
    await mkdir(dirname(path), { recursive: true });
    await writeFile(path, value, "utf8");
  },
};

const defaultServices: CliServices = {
  loadConfig: path => loadTsConfig(path),
  createOrchestrator: () => createOrchestrator({ logger: silentLogger() }),
  startDevServer: options => startDevServer(options),
  watchProject: (root, ignoredPaths) => watchProject(root, ignoredPaths),
};

export async function run(
  argv: readonly string[],
  io: CliIO = defaultIO,
  options: RunCliOptions = {},
  services: CliServices = defaultServices,
): Promise<number> {
  let parsed: ReturnType<Parser["parse"]>;
  try {
    const effectiveArgv = argv.length === 0 ? ["--help"] : [...argv];
    parsed = new Parser(CLI_SPEC_PATH, ["forme", ...effectiveArgv]).parse();
  } catch (error) {
    if (error instanceof ParseErrors) {
      for (const entry of error.errors) diagnostic(io, "E_USAGE", entry.message);
    } else if (error instanceof SpecError) {
      diagnostic(io, "E_CLI_SPEC", error.message);
    } else {
      diagnostic(io, "E_CLI_SPEC", message(error));
    }
    return EXIT_USAGE_OR_CONFIG;
  }

  if ("text" in parsed) {
    io.stdout.write(`${parsed.text}\n`);
    return EXIT_OK;
  }
  if ("version" in parsed) {
    io.stdout.write(`${parsed.version}\n`);
    return EXIT_OK;
  }

  let args: ParsedArgs;
  try {
    args = invocation(parsed);
  } catch (error) {
    diagnostic(io, "E_USAGE", message(error));
    return EXIT_USAGE_OR_CONFIG;
  }

  const originalCwd = io.cwd();
  let orchestrator: Orchestrator | null = null;
  try {
    const configPath = await resolveConfigPath(args.configPath, originalCwd, io);
    const projectRoot = dirname(configPath);
    io.chdir(projectRoot);
    let config = await services.loadConfig(configPath);
    if (args.reproducible) {
      config = {
        ...config,
        settings: { ...config.settings, reproducibleBuild: true },
      };
    }

    orchestrator = services.createOrchestrator();
    const pipeline = await orchestrator.buildPipeline(config);

    if (args.command === "watch") {
      return await runWatch(
        config,
        pipeline,
        orchestrator,
        projectRoot,
        args,
        io,
        options,
        services,
      );
    }

    if (args.command === "check") {
      const count = pipeline.dag.topoOrder.length;
      io.stdout.write(
        `forme check: ${config.name} is valid (${count} stage${count === 1 ? "" : "s"}${args.reproducible ? ", reproducible" : ""})\n`,
      );
      return EXIT_OK;
    }
    if (args.command === "clean") {
      const targets = cleanTargets(config, projectRoot);
      for (const target of targets) await io.remove(target);
      io.stdout.write(`forme clean: removed ${targets.length} configured path${targets.length === 1 ? "" : "s"}\n`);
      return EXIT_OK;
    }

    const result = await orchestrator.runOnce(pipeline, {
      cancellation: options.cancellation,
    });
    if (result.outcome === "success") {
      const outputs = Object.keys(result.outputs).sort().join(", ") || "none";
      const count = result.stages.length;
      if (args.reportPath !== null) {
        const reportPath = isAbsolute(args.reportPath)
          ? args.reportPath
          : resolve(projectRoot, args.reportPath);
        await io.writeFile(reportPath, buildReport(config, result));
      }
      io.stdout.write(
        `forme build: ${config.name} success (${count} stage${count === 1 ? "" : "s"}; outputs: ${outputs}; build: ${result.buildId})\n`,
      );
      return EXIT_OK;
    }
    for (const error of result.errors) {
      diagnostic(io, error.code, `${error.instanceId} (${error.stageName}): ${error.message}`);
    }
    return result.outcome === "cancelled" ? EXIT_CANCELLED : EXIT_BUILD_FAILED;
  } catch (error) {
    const entries = configErrorEntries(error);
    if (entries !== null) {
      for (const entry of entries) {
        diagnostic(io, entry.code, `${entry.path}: ${entry.message}`);
      }
    } else {
      diagnostic(io, "E_CONFIG", message(error));
    }
    return EXIT_USAGE_OR_CONFIG;
  } finally {
    try {
      if (orchestrator !== null) await orchestrator.dispose();
    } finally {
      io.chdir(originalCwd);
    }
  }
}

function invocation(parsed: ParseResult): ParsedArgs {
  const command = parsed.commandPath[1];
  if (command !== "build" && command !== "check" && command !== "clean" && command !== "watch") {
    throw new Error("a build, check, clean, or watch command is required");
  }
  const config = parsed.flags["config"];
  const report = parsed.flags["report"];
  const port = parsed.flags["port"] ?? 3000;
  const debounceMs = parsed.flags["debounce"] ?? 200;
  if (typeof port !== "number" || !Number.isInteger(port) || port < 1 || port > 65535) {
    throw new Error("--port must be an integer from 1 through 65535");
  }
  if (typeof debounceMs !== "number" || !Number.isInteger(debounceMs) || debounceMs < 0 || debounceMs > 60_000) {
    throw new Error("--debounce must be an integer from 0 through 60000");
  }
  return {
    command,
    configPath: typeof config === "string" ? config : null,
    reproducible: parsed.flags["reproducible"] === true,
    reportPath: typeof report === "string" ? report : null,
    port,
    debounceMs,
  };
}

async function runWatch(
  config: PipelineConfig,
  pipeline: Pipeline,
  orchestrator: Orchestrator,
  projectRoot: string,
  args: ParsedArgs,
  io: CliIO,
  options: RunCliOptions,
  services: CliServices,
): Promise<number> {
  let server: DevServer | null = null;
  let session: ReturnType<Orchestrator["watch"]> | null = null;
  try {
    server = await services.startDevServer({ port: args.port });
    const ignored = [
      resolve(projectRoot, ".git"),
      resolve(projectRoot, "node_modules"),
      ...cleanTargets(config, projectRoot),
    ];
    session = orchestrator.watch(pipeline, {
      changes: services.watchProject(projectRoot, ignored),
      debounceMs: args.debounceMs,
      cancellation: options.cancellation,
    });
    io.stdout.write(`forme watch: ${server.address.url} (${config.name}; debounce: ${args.debounceMs}ms)\n`);

    for await (const result of session.results()) {
      if (result.outcome === "success") {
        try {
          const snapshot = snapshotFromOutputs(String(result.buildId), result.outputs);
          server.publish(snapshot);
          io.stdout.write(`forme watch: ${config.name} ready (build: ${result.buildId})\n`);
        } catch (error) {
          const detail = message(error);
          server.publishFailure({ message: detail });
          diagnostic(io, "E_PREVIEW", detail);
        }
        continue;
      }
      if (result.outcome === "cancelled" && options.cancellation?.cancelled) break;
      const detail = watchFailure(result);
      server.publishFailure({ message: detail });
      for (const error of result.errors) {
        diagnostic(io, error.code, `${error.instanceId} (${error.stageName}): ${error.message}`);
      }
    }
    return options.cancellation?.cancelled ? EXIT_CANCELLED : EXIT_OK;
  } finally {
    if (session !== null) await session.stop();
    if (server !== null) await server.close();
  }
}

function watchFailure(result: RunResult): string {
  if (result.errors.length === 0) return `build ${result.outcome}`;
  return result.errors.map(error => `${error.code}: ${error.message}`).join("\n");
}

function buildReport(config: PipelineConfig, result: Awaited<ReturnType<Orchestrator["runOnce"]>>): string {
  const outputs: Record<string, unknown> = {};
  for (const name of Object.keys(result.outputs).sort()) {
    outputs[name] = summarizeOutput(result.outputs[name]);
  }
  return `${JSON.stringify({
    schemaVersion: 1,
    pipeline: config.name,
    outcome: result.outcome,
    buildId: result.buildId,
    reproducible: config.settings.reproducibleBuild,
    outputs,
  }, null, 2)}\n`;
}

function summarizeOutput(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(summarizeOutput);
  if (typeof value !== "object" || value === null) return { kind: typeof value };
  const candidate = value as {
    variant?: unknown;
    files?: unknown;
    manifest?: unknown;
  };
  if (
    typeof candidate.files !== "object" || candidate.files === null ||
    typeof candidate.manifest !== "object" || candidate.manifest === null
  ) {
    return { kind: "object" };
  }
  const files = candidate.files as Record<string, unknown>;
  return {
    kind: "DeployArtifact",
    variant: candidate.variant ?? null,
    manifest: candidate.manifest,
    files: Object.keys(files).sort().map(path => {
      const bytes = files[path];
      if (!(bytes instanceof Uint8Array)) return { path, byteLength: null, sha256: null };
      return {
        path,
        byteLength: bytes.byteLength,
        sha256: createHash("sha256").update(bytes).digest("hex"),
      };
    }),
  };
}

async function resolveConfigPath(
  explicit: string | null,
  cwd: string,
  io: CliIO,
): Promise<string> {
  if (explicit !== null) {
    const path = isAbsolute(explicit) ? explicit : resolve(cwd, explicit);
    await io.access(path);
    return path;
  }
  for (const name of DEFAULT_CONFIG_NAMES) {
    const candidate = resolve(cwd, name);
    try {
      await io.access(candidate);
      return candidate;
    } catch (error) {
      if (isMissingPath(error)) continue;
      throw error;
    }
  }
  throw new Error(`no Forme config found in ${cwd}`);
}

function cleanTargets(config: PipelineConfig, projectRoot: string): readonly string[] {
  const candidates: string[] = [];
  if (config.settings.cacheDir !== null) candidates.push(config.settings.cacheDir);

  for (const spec of config.stages) {
    if (isStageRef(spec.stage) || spec.stage.produces.name !== "DeployArtifact") continue;
    if (typeof spec.config !== "object" || spec.config === null) continue;
    const outDir = (spec.config as { outDir?: unknown }).outDir;
    if (typeof outDir === "string" && outDir.length !== 0) candidates.push(outDir);
  }

  const unique = new Set<string>();
  for (const candidate of candidates) {
    const target = resolve(projectRoot, candidate);
    const rel = relative(projectRoot, target);
    if (
      rel === "" || rel === "." || rel === ".." ||
      rel.startsWith("../") || rel.startsWith("..\\") || isAbsolute(rel)
    ) {
      throw new Error(`refusing to clean unsafe path ${JSON.stringify(candidate)} outside the project`);
    }
    unique.add(target);
  }
  return [...unique].sort();
}

function diagnostic(io: CliIO, code: string, value: string): void {
  io.stderr.write(`forme: ${code}: ${value}\n`);
}

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function isMissingPath(error: unknown): boolean {
  return typeof error === "object" && error !== null &&
    (error as { code?: unknown }).code === "ENOENT";
}

function configErrorEntries(error: unknown): ConfigError["errors"] | null {
  if (error instanceof ConfigError) return error.errors;
  if (typeof error !== "object" || error === null || !Array.isArray((error as { errors?: unknown }).errors)) {
    return null;
  }
  const entries = (error as { errors: unknown[] }).errors;
  if (!entries.every(entry =>
    typeof entry === "object" && entry !== null &&
    typeof (entry as { path?: unknown }).path === "string" &&
    typeof (entry as { code?: unknown }).code === "string" &&
    typeof (entry as { message?: unknown }).message === "string")) {
    return null;
  }
  return entries as unknown as ConfigError["errors"];
}
