import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { LABELS_FILE, childEnv, labelsFor, main, type Io } from "../src/cli.ts";
import { loadCorpus } from "../src/corpus.ts";
import { ENV_FAULT, ENV_FILTER, ENV_OUT } from "../src/session.ts";
import { CORPUS_DIR, REGISTRY_PATH, REPO_ROOT, miniCorpus, newLibrary, tempDir, type Library } from "./helpers.ts";

interface Captured {
  io: Io;
  out: string[];
  err: string[];
}
function capture(env: NodeJS.ProcessEnv = {}): Captured {
  const out: string[] = [];
  const err: string[] = [];
  return { out, err, io: { out: (line) => out.push(line), err: (line) => err.push(line), env } };
}

function flags(lib: Library, root: string, corpusDir: string): string[] {
  return ["--repo-root", root, "--corpus", corpusDir, "--binding", lib.binding, "--package-dir", lib.packageDir];
}

async function exec(argv: string[], env: NodeJS.ProcessEnv = {}): Promise<{ status: number } & Captured> {
  const captured = capture(env);
  const status = await main(argv, captured.io);
  return { status, ...captured };
}

/** A mini corpus, a correct library, and a manifest written by a real run. */
async function written(): Promise<{ lib: Library; root: string; corpusDir: string; manifest: string }> {
  const lib = newLibrary();
  const { root, corpusDir } = miniCorpus();
  const manifest = join(tempDir(), "evidence", "test-manifest.json");
  const result = await exec(["run", ...flags(lib, root, corpusDir), "--out", manifest]);
  expect(result.status).toBe(0);
  return { lib, root, corpusDir, manifest };
}

describe("simple commands", () => {
  it("list-faults prints <fault>:<op> per control", async () => {
    const result = await exec(["list-faults", "--repo-root", REPO_ROOT, "--corpus", CORPUS_DIR]);
    expect(result.status).toBe(0);
    const rows = (result.out[0] as string).split("\n");
    expect(rows).toHaveLength(8);
    expect(rows[0]).toBe("ignore-last-byte:eq");
  });

  it("source-digest prints the warrant-src-v1 digest", async () => {
    const dir = tempDir();
    writeFileSync(join(dir, "a.ts"), "x");
    const result = await exec(["source-digest", dir]);
    expect(result.status).toBe(0);
    expect(result.out[0]).toMatch(/^[0-9a-f]{64}$/);
  });
});

describe("run and check", () => {
  it("run writes a manifest and the label sidecar; check accepts them", async () => {
    const { lib, root, corpusDir, manifest } = await written();
    const text = readFileSync(manifest, "utf-8");
    expect(text.endsWith("}\n")).toBe(true);
    expect(text.includes("\r")).toBe(false);
    const sidecar = join(dirname(manifest), LABELS_FILE);
    expect(existsSync(sidecar)).toBe(true);
    const labels = JSON.parse(readFileSync(sidecar, "utf-8")) as Record<string, unknown>;
    expect(labels["schema"]).toBe("warrant-lane-domain-labels-v1");
    const result = await exec(["check", ...flags(lib, root, corpusDir), manifest]);
    expect(result.status).toBe(0);
    expect(result.out.some((line) => line.startsWith("ok manifest "))).toBe(true);
    expect(result.out.some((line) => line.startsWith(`ok ${LABELS_FILE}`))).toBe(true);
  });

  it("the manifest path can come from WARRANT_EVIDENCE_OUT", async () => {
    const lib = newLibrary();
    const { root, corpusDir } = miniCorpus();
    const out = join(tempDir(), "viaenv", "m.json");
    const result = await exec(["run", ...flags(lib, root, corpusDir)], { [ENV_OUT]: out });
    expect(result.status).toBe(0);
    expect(existsSync(out)).toBe(true);
    const bare = await exec(["run", ...flags(lib, root, corpusDir)]);
    expect(bare.status).toBe(0);
  });

  it("check ignores toolchain.version but reports every other drift", async () => {
    const { lib, root, corpusDir, manifest } = await written();
    const original = readFileSync(manifest, "utf-8");
    const check = (): Promise<{ status: number } & Captured> => exec(["check", ...flags(lib, root, corpusDir), manifest]);

    writeFileSync(manifest, original.replace(/"version": "[^"]*"/, '"version": "0.0.1"'));
    expect((await check()).status).toBe(0);

    writeFileSync(manifest, original.replace(`"faulted": false`, `"faulted":false`));
    let result = await check();
    expect(result.status).toBe(1);
    expect(result.out.join("\n")).toMatch(/drifted \(formatting\)/);

    writeFileSync(manifest, original.replace(/"source_sha256": "[0-9a-f]{2}/, '"source_sha256": "ff'));
    result = await check();
    expect(result.status).toBe(1);
    expect(result.out.join("\n")).toMatch(/drifted \(source_sha256\)/);

    writeFileSync(manifest, original.replace('"name": "node"', '"name": "bun"'));
    result = await check();
    expect(result.status).toBe(1);

    writeFileSync(manifest, "[]");
    result = await check();
    expect(result.out.join("\n")).toMatch(/not a lane manifest/);
    writeFileSync(manifest, "{");
    expect((await check()).status).toBe(1);
    writeFileSync(manifest, Buffer.from([0xff]));
    expect((await check()).out.join("\n")).toMatch(/cannot read the committed manifest/);
    writeFileSync(manifest, original.replace(/"toolchain": \{/, '"toolchain": 5, "x": {'));
    expect((await check()).status).toBe(1);

    writeFileSync(manifest, original);
    const missing = await exec(["check", ...flags(lib, root, corpusDir), join(dirname(manifest), "gone.json")]);
    expect(missing.status).toBe(1);
  });

  it("check guards the label sidecar", async () => {
    const { lib, root, corpusDir, manifest } = await written();
    const sidecar = join(dirname(manifest), LABELS_FILE);
    const original = readFileSync(sidecar, "utf-8");
    writeFileSync(sidecar, original.replace("full", "sample"));
    const changed = await exec(["check", ...flags(lib, root, corpusDir), manifest]);
    expect(changed.status).toBe(1);
    expect(changed.out.join("\n")).toMatch(/differs from the labels/);
    writeFileSync(sidecar, Buffer.from([0xff]));
    expect((await exec(["check", ...flags(lib, root, corpusDir), manifest])).out.join("\n")).toMatch(/missing or unreadable/);
  });

  it("a failing library exits 1 and writes nothing", async () => {
    const lib = newLibrary({ ctEqFixed: "export const ctEqFixed = () => true;" });
    const { root, corpusDir } = miniCorpus();
    const out = join(tempDir(), "never.json");
    const result = await exec(["run", ...flags(lib, root, corpusDir), "--out", out]);
    expect(result.status).toBe(1);
    expect(existsSync(out)).toBe(false);
    expect(result.out.some((line) => line.startsWith("not ok "))).toBe(true);
  });

  it("a faulted run prints not ok, exits 1 and never writes a manifest", async () => {
    const lib = newLibrary();
    const { root, corpusDir } = miniCorpus();
    const out = join(tempDir(), "never.json");
    const result = await exec(["run", ...flags(lib, root, corpusDir), "--out", out], { [ENV_FAULT]: "invert:eq_fixed", [ENV_FILTER]: "/exhaustive/" });
    expect(result.status).toBe(1);
    expect(result.out.some((line) => line.startsWith("not ok "))).toBe(true);
    expect(existsSync(out)).toBe(false);
  });

  it("a faulted run that passes still writes nothing, and `check` cannot use it", async () => {
    const lib = newLibrary();
    const { root, corpusDir } = miniCorpus();
    const out = join(tempDir(), "never.json");
    const env = { [ENV_FAULT]: "invert:eq_u64", [ENV_FILTER]: "/exhaustive/" };
    const run = await exec(["run", ...flags(lib, root, corpusDir), "--out", out], env);
    expect(run.status).toBe(0);
    expect(run.out).toContain("# faulted or filtered run: no manifest written");
    expect(existsSync(out)).toBe(false);
    const check = await exec(["check", ...flags(lib, root, corpusDir), out], env);
    expect(check.status).toBe(1);
    expect(check.out.join("\n")).toMatch(/cannot produce a manifest/);
  });

  it("refuses unsafe environments with exit 2", async () => {
    const lib = newLibrary();
    const { root, corpusDir } = miniCorpus();
    for (const env of [{ [ENV_FILTER]: "x" }, { [ENV_FAULT]: "none", [ENV_FILTER]: "x" }, { [ENV_FAULT]: "bogus" }, { [ENV_FAULT]: "invert", [ENV_FILTER]: "no such id" }]) {
      const result = await exec(["run", ...flags(lib, root, corpusDir)], env);
      expect(result.status).toBe(2);
      expect(result.err[0]).toMatch(/^error: /);
    }
  });
});

describe("usage errors exit 2", () => {
  const usage = async (argv: string[]): Promise<void> => {
    const result = await exec(argv);
    expect(result.status, argv.join(" ")).toBe(2);
    expect(result.err.length).toBeGreaterThan(0);
  };

  it("refuses bad command lines", async () => {
    const base = ["--repo-root", REPO_ROOT, "--corpus", CORPUS_DIR];
    await usage([]);
    await usage(["bogus"]);
    await usage(["run"]);
    await usage(["run", "--repo-root", REPO_ROOT]);
    await usage(["run", ...base]); // no --binding/--package-dir
    await usage(["sabotage", ...base]);
    await usage(["run", ...base, "--mystery"]);
    await usage(["run", ...base, "--out"]);
    await usage(["run", ...base, "--repo-root", REPO_ROOT]);
    await usage(["run", ...base, "stray"]);
    await usage(["check", ...base]);
    await usage(["check", ...base, "a", "b"]);
    await usage(["source-digest"]);
    await usage(["validate-manifest", ...base]);
  });

  it("reports a bug in the runner as a refusal, not a verdict", async () => {
    const captured = capture();
    captured.io.out = () => {
      throw new Error("sink broke");
    };
    expect(await main(["list-faults", "--repo-root", REPO_ROOT, "--corpus", CORPUS_DIR], captured.io)).toBe(2);
    expect(captured.err[0]).toBe("error: internal: Error");
    captured.io.out = () => {
      throw "not an error";
    };
    expect(await main(["list-faults", "--repo-root", REPO_ROOT, "--corpus", CORPUS_DIR], captured.io)).toBe(2);
    expect(captured.err[1]).toBe("error: internal: unknown");
  });
});

describe("validate-manifest", () => {
  it("accepts a real manifest, with and without a registry", async () => {
    const { root, corpusDir, manifest } = await written();
    const result = await exec(["validate-manifest", "--repo-root", root, "--corpus", corpusDir, manifest]);
    expect(result.status).toBe(0);
    expect(result.out[0]).toBe("ok test-manifest.json is a structurally valid lane manifest");
    // The checked-in registry lists only the Python lane, so a TypeScript manifest
    // is correctly refused against it until the row exists.
    const refused = await exec(["validate-manifest", "--repo-root", root, "--corpus", corpusDir, "--registry", REGISTRY_PATH, manifest]);
    expect(refused.status).toBe(2);
    expect(refused.err[0]).toMatch(/registered exactly once/);
  });

  it("accepts a registry that lists the lane, and --require-active", async () => {
    const { root, corpusDir, manifest } = await written();
    const registry = join(tempDir(), "consumers.json");
    const row = { lane: "typescript", package: "typescript/ct-compare", module: "ct_compare", status: "planned" };
    writeFileSync(registry, JSON.stringify({ consumers: [row] }));
    const planned = await exec(["validate-manifest", "--repo-root", root, "--corpus", corpusDir, "--registry", registry, manifest]);
    expect(planned.status).toBe(0);
    const required = await exec(["validate-manifest", "--repo-root", root, "--corpus", corpusDir, "--registry", registry, "--require-active", manifest]);
    expect(required.status).toBe(2);
  });

  it("refuses a stale label sidecar and a bad manifest", async () => {
    const { root, corpusDir, manifest } = await written();
    const sidecar = join(dirname(manifest), LABELS_FILE);
    writeFileSync(sidecar, readFileSync(sidecar, "utf-8").replace("full", "sample"));
    expect((await exec(["validate-manifest", "--repo-root", root, "--corpus", corpusDir, manifest])).status).toBe(2);
    writeFileSync(manifest, "{}");
    expect((await exec(["validate-manifest", "--repo-root", root, "--corpus", corpusDir, manifest])).status).toBe(2);
  });

  it("labelsFor binds the sidecar to the manifest, ignoring toolchain.version", () => {
    const corpus = loadCorpus(REPO_ROOT, CORPUS_DIR);
    const base = { package: "p/q", module: "m", toolchain: { name: "node", version: "1" } };
    const a = labelsFor(corpus, base);
    expect(labelsFor(corpus, { ...base, toolchain: { name: "node", version: "2" } })).toEqual(a);
    expect(labelsFor(corpus, { ...base, toolchain: { name: "other", version: "1" } })).not.toEqual(a);
  });
});

describe("sabotage", () => {
  it("re-runs every control in a child process and each one fails loudly", async () => {
    const lib = newLibrary();
    const { root, corpusDir } = miniCorpus();
    const result = await exec(["sabotage", ...flags(lib, root, corpusDir)]);
    expect(result.status).toBe(0);
    expect(result.out).toEqual([
      "ok sabotage ignore-last-byte:eq_fixed (exit 1)",
      "ok sabotage invert:eq_fixed (exit 1)",
    ]);
  }, 120_000);

  it("fails when a child cannot demonstrate the fault", async () => {
    const lib = newLibrary();
    const { root, corpusDir } = miniCorpus();
    const broken: Library = { packageDir: lib.packageDir, binding: join(tempDir(), "missing.bindings") };
    const result = await exec(["sabotage", ...flags(broken, root, corpusDir)]);
    expect(result.status).toBe(1);
    expect(result.out.every((line) => line.startsWith("not ok sabotage "))).toBe(true);
  }, 120_000);

  it("gives each child a fault, a filter and no output file", () => {
    const env = childEnv({ A: "1", [ENV_OUT]: "x", [ENV_FILTER]: "old" }, "invert:eq");
    expect(env).toEqual({ A: "1", [ENV_FAULT]: "invert:eq", [ENV_FILTER]: "/exhaustive/" });
  });
});

describe("the process entry", () => {
  it("exits with the command's status", () => {
    const entry = join(dirname(fileURLToPath(import.meta.url)), "..", "src", "bin.ts");
    const run = (args: string[]) =>
      spawnSync(process.execPath, ["--experimental-strip-types", "--disable-warning=ExperimentalWarning", entry, ...args], { encoding: "utf-8" });
    const good = run(["list-faults", "--repo-root", REPO_ROOT, "--corpus", CORPUS_DIR]);
    expect(good.status).toBe(0);
    expect(good.stdout.split("\n")[0]).toBe("ignore-last-byte:eq");
    const bad = run(["nope"]);
    expect(bad.status).toBe(2);
    expect(bad.stderr).toMatch(/^error: usage/);
    mkdirSync(tempDir(), { recursive: true });
  });
});
