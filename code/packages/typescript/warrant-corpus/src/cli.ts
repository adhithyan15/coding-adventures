/**
 * # The `warrant-corpus` command line (VER04i sections 5.2 and 6.2)
 *
 *     node --experimental-strip-types src/bin.ts <command> --repo-root R
 *         --corpus DIR --binding FILE --package-dir DIR [command options]
 *
 * | command               | what it does                                        |
 * |-----------------------|-----------------------------------------------------|
 * | `run [--out F]`       | full run; writes the manifest on a clean pass       |
 * | `check F`             | full run; fails if the regenerated manifest differs |
 * | `sabotage`            | every control, in a child process, must print `not ok` |
 * | `list-faults`         | `<fault>:<op>` per control                          |
 * | `source-digest D`     | `warrant-src-v1` digest of a source tree            |
 * | `validate-manifest F` | structural validation of a manifest against the corpus |
 *
 * Exit status: `0` everything passed, `1` a verdict failed, `2` the run was
 * refused (bad corpus, bad binding, bad usage). Output is TAP-style:
 * `ok <id>` / `not ok <id> <failure-id>`.
 *
 * The parser is hand-written on purpose: the runner has no dependencies, and a
 * flag it does not know is a usage error, not something to ignore.
 */

import { spawnSync } from "node:child_process";
import { mkdirSync, existsSync, writeFileSync } from "node:fs";
import { dirname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { listControlRows, loadCorpus, type Corpus } from "./corpus.ts";
import { LaneError, UsageError } from "./errors.ts";
import { isObject, MAX_TEXT_BYTES, parseJson, readBytes, renderJson, sha256Hex, decodeUtf8 } from "./jsonio.ts";
import { deriveLabels } from "./labels.ts";
import { sourceDigest, validateManifest } from "./manifest.ts";
import {
  checkFilterRules,
  ENV_FAULT,
  ENV_FILTER,
  ENV_OUT,
  environment,
  execute,
  outcomePassed,
  parseFaultSpec,
  prepare,
  type RunConfig,
} from "./session.ts";

export const LABELS_FILE = "domain-labels.json";
const COMMANDS = ["run", "check", "sabotage", "list-faults", "source-digest", "validate-manifest"] as const;
type Command = (typeof COMMANDS)[number];
const VALUE_FLAGS = ["--repo-root", "--corpus", "--binding", "--package-dir", "--out", "--registry"] as const;

/** Where output goes (a test passes its own). */
export interface Io {
  out(line: string): void;
  err(line: string): void;
  env: NodeJS.ProcessEnv;
}

const processIo: Io = {
  out: (line) => process.stdout.write(`${line}\n`),
  err: (line) => process.stderr.write(`${line}\n`),
  env: process.env,
};

interface Args {
  command: Command;
  flags: Map<string, string>;
  requireActive: boolean;
  positional: string[];
}

function parseArgs(argv: readonly string[]): Args {
  const [command, ...rest] = argv;
  if (command === undefined || !(COMMANDS as readonly string[]).includes(command)) {
    throw new UsageError("usage", `expected one of: ${COMMANDS.join(", ")}`);
  }
  const flags = new Map<string, string>();
  const positional: string[] = [];
  let requireActive = false;
  for (let index = 0; index < rest.length; index += 1) {
    const token = rest[index] as string;
    if (token === "--require-active") {
      requireActive = true;
    } else if ((VALUE_FLAGS as readonly string[]).includes(token)) {
      const value = rest[index + 1];
      if (value === undefined || flags.has(token)) throw new UsageError("usage", `${token} needs one value`);
      flags.set(token, value);
      index += 1;
    } else if (token.startsWith("--")) {
      throw new UsageError("usage", `unknown option ${token}`);
    } else {
      positional.push(token);
    }
  }
  const wantsPositional = command === "check" || command === "source-digest" || command === "validate-manifest";
  if (positional.length !== (wantsPositional ? 1 : 0)) {
    throw new UsageError("usage", `\`${command}\` takes ${wantsPositional ? "exactly one" : "no"} positional argument`);
  }
  if (command !== "source-digest") {
    for (const required of ["--repo-root", "--corpus"]) {
      if (!flags.has(required)) throw new UsageError("usage", `${required} is required`);
    }
  }
  return { command: command as Command, flags, requireActive, positional };
}

function runConfig(args: Args, fault: string | null, faultOp: string | null, filter: string | null): RunConfig {
  const binding = args.flags.get("--binding");
  const packageDir = args.flags.get("--package-dir");
  if (binding === undefined || packageDir === undefined) {
    throw new UsageError("usage", "--binding and --package-dir are required");
  }
  return {
    repoRoot: args.flags.get("--repo-root") as string,
    corpusDir: args.flags.get("--corpus") as string,
    bindingPath: binding,
    packageDir,
    fault,
    faultOp,
    filter,
  };
}

/**
 * The coverage-label sidecar, bound to the manifest by digest.
 *
 * The digest ignores `toolchain.version` (diagnostic, VER04i 8), exactly as
 * `check` does, so a different Node patch release is not label drift.
 */
export function labelsFor(corpus: Corpus, manifest: Record<string, unknown>): Record<string, unknown> {
  const toolchain = manifest["toolchain"] as Record<string, unknown>;
  const stable = { ...manifest, toolchain: { name: toolchain["name"], version: "" } };
  return deriveLabels(
    corpus,
    String(manifest["package"]),
    String(manifest["module"]),
    sha256Hex(new TextEncoder().encode(renderJson(stable))),
  );
}

async function runCommand(args: Args, io: Io): Promise<number> {
  const [faultText, needle, envOut] = environment(io.env);
  checkFilterRules(faultText, needle);
  const [fault, faultOp] = parseFaultSpec(faultText);
  const prepared = await prepare(runConfig(args, fault, faultOp, needle));
  const outcome = execute(prepared, io.out);
  if (!outcomePassed(outcome)) return 1;
  if (args.command === "check") {
    return compare(args.positional[0] as string, outcome.manifest, prepared.corpus, io);
  }
  const out = args.flags.get("--out") ?? envOut;
  if (outcome.manifest === null) {
    io.out("# faulted or filtered run: no manifest written");
  } else if (out !== null) {
    mkdirSync(dirname(out), { recursive: true });
    writeFileSync(out, renderJson(outcome.manifest), "utf-8");
    io.out(`# wrote ${out}`);
    const sidecar = join(dirname(out), LABELS_FILE);
    writeFileSync(sidecar, renderJson(labelsFor(prepared.corpus, outcome.manifest)), "utf-8");
    io.out(`# wrote ${sidecar}`);
  }
  return 0;
}

/**
 * `check`: the regenerated manifest must equal the committed one.
 *
 * `toolchain.version` is diagnostic (VER04i 8), so a different Node patch
 * release is not drift; every other byte is.
 */
function compare(committed: string, fresh: Record<string, unknown> | null, corpus: Corpus, io: Io): number {
  const name = committed.split(/[\\/]/).pop() as string;
  if (fresh === null) {
    io.out("not ok manifest: this run cannot produce a manifest (faulted or filtered)");
    return 1;
  }
  let text: string;
  try {
    text = decodeUtf8(readBytes(committed, MAX_TEXT_BYTES));
  } catch {
    io.out(`not ok manifest: cannot read the committed manifest ${name}`);
    return 1;
  }
  let old: Record<string, unknown>;
  let current: Record<string, unknown>;
  try {
    const parsed = parseJson(new TextEncoder().encode(text));
    const oldToolchain = isObject(parsed) ? parsed["toolchain"] : undefined;
    if (!isObject(parsed) || !isObject(oldToolchain)) throw new TypeError("not a manifest");
    old = parsed;
    current = { ...fresh, toolchain: { name: (fresh["toolchain"] as Record<string, unknown>)["name"], version: oldToolchain["version"] } };
  } catch {
    io.out("not ok manifest: the committed manifest is not a lane manifest");
    return 1;
  }
  if (renderJson(current) !== text) {
    const keys = [...new Set([...Object.keys(current), ...Object.keys(old)])].sort();
    const changed = keys.filter((key) => JSON.stringify(sortedCopy(current[key])) !== JSON.stringify(sortedCopy(old[key])));
    io.out(`not ok manifest: the committed manifest drifted (${changed.join(", ") || "formatting"})`);
    return 1;
  }
  io.out(`ok manifest ${name} matches the regenerated manifest`);
  const expected = renderJson(labelsFor(corpus, current));
  let actual: string;
  try {
    actual = decodeUtf8(readBytes(join(dirname(committed), LABELS_FILE), MAX_TEXT_BYTES));
  } catch {
    io.out(`not ok ${LABELS_FILE}: missing or unreadable next to the manifest`);
    return 1;
  }
  if (actual !== expected) {
    io.out(`not ok ${LABELS_FILE}: differs from the labels derived from the corpus`);
    return 1;
  }
  io.out(`ok ${LABELS_FILE} matches the labels derived from the corpus`);
  return 0;
}

/** A key-sorted deep copy, so two equal values stringify identically. */
function sortedCopy(value: unknown): unknown {
  return JSON.parse(renderJson(value ?? null)) as unknown;
}

/** The environment of one sabotage child: a fault, a suite filter, no output file. */
export function childEnv(base: NodeJS.ProcessEnv, fault: string): NodeJS.ProcessEnv {
  const env = { ...base };
  env[ENV_FAULT] = fault;
  env[ENV_FILTER] = "/exhaustive/";
  delete env[ENV_OUT];
  return env;
}

function commonFlags(args: Args): string[] {
  return ["--repo-root", "--corpus", "--binding", "--package-dir"].flatMap((flag) => [flag, args.flags.get(flag) as string]);
}

/** Re-run `run` once per control with a fault set; each must fail loudly. */
function sabotage(args: Args, io: Io): number {
  runConfig(args, null, null, null); // refuse early if --binding/--package-dir are missing
  const entry = join(dirname(fileURLToPath(import.meta.url)), "bin.ts");
  let failures = 0;
  for (const row of listControlRows(args.flags.get("--corpus") as string)) {
    const done = spawnSync(
      process.execPath,
      ["--experimental-strip-types", "--disable-warning=ExperimentalWarning", entry, "run", ...commonFlags(args)],
      { env: childEnv(io.env, row), encoding: "utf-8", maxBuffer: 256 * 1024 * 1024 },
    );
    const caught = done.status === 1 && done.stdout.split("\n").some((line) => line.startsWith("not ok "));
    io.out(`${caught ? "ok" : "not ok"} sabotage ${row} (exit ${String(done.status)})`);
    if (!caught) failures += 1;
  }
  return failures > 0 ? 1 : 0;
}

function validate(args: Args, io: Io): number {
  const repoRoot = args.flags.get("--repo-root") as string;
  const corpusDir = args.flags.get("--corpus") as string;
  const corpus = loadCorpus(repoRoot, corpusDir);
  const path = relative(resolve(repoRoot), resolve(corpusDir));
  const relativeCorpus = path.split(sep).join("/");
  const registryPath = args.flags.get("--registry");
  const registry = registryPath === undefined ? null : readBytes(registryPath, MAX_TEXT_BYTES);
  const manifestPath = args.positional[0] as string;
  const document = validateManifest(readBytes(manifestPath, MAX_TEXT_BYTES), corpus, relativeCorpus, registry, args.requireActive);
  const sidecar = join(dirname(manifestPath), LABELS_FILE);
  if (existsSync(sidecar)) {
    const expected = renderJson(labelsFor(corpus, document));
    if (decodeUtf8(readBytes(sidecar, MAX_TEXT_BYTES)) !== expected) {
      throw new UsageError("labels", `${LABELS_FILE} differs from the labels derived from the corpus`);
    }
  }
  io.out(`ok ${manifestPath.split(/[\\/]/).pop() as string} is a structurally valid lane manifest`);
  return 0;
}

/** Entry point; resolves to the process exit status. */
export async function main(argv: readonly string[], io: Io = processIo): Promise<number> {
  try {
    const args = parseArgs(argv);
    switch (args.command) {
      case "run":
      case "check":
        return await runCommand(args, io);
      case "sabotage":
        return sabotage(args, io);
      case "list-faults":
        io.out(listControlRows(args.flags.get("--corpus") as string).join("\n"));
        return 0;
      case "source-digest":
        io.out(sourceDigest(args.positional[0] as string));
        return 0;
      case "validate-manifest":
        return validate(args, io);
    }
  } catch (error) {
    if (error instanceof LaneError) {
      io.err(`error: ${error.message}`);
    } else {
      // Anything else is a bug in the runner, not a verdict about the library:
      // refuse (2) rather than exit 1, which is reserved for a failed case.
      io.err(`error: internal: ${error instanceof Error ? error.name : "unknown"}`);
    }
    return 2;
  }
}
