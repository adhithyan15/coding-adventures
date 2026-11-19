import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { copyFileSync, mkdtempSync, readFileSync, realpathSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, isAbsolute, join, resolve, sep } from "node:path";
import { pathToFileURL } from "node:url";
import { spawnSync } from "node:child_process";
import test from "node:test";

import ts from "typescript";
import { reduce_frame, type ActionChange, type InputState } from "../src/index.js";

const packageRoot = process.cwd();
const repositoryRoot = resolve(packageRoot, "../../../..");
const rustWorkspace = resolve(repositoryRoot, "code/packages/rust");
const sourcePath = resolve(packageRoot, "src/digital.ts");
const relativeSourcePath = "code/packages/typescript/game-input-frame/src/digital.ts";
const sourceBytes = readFileSync(sourcePath);
const sourceText = sourceBytes.toString("utf8");
const sourceDigest = createHash("sha256").update(sourceBytes).digest("hex");

const mutants = [
  {
    query: "bit_is_set_not_equal",
    before: "!== 0n",
    after: "=== 0n",
    slotBit: 0,
    down: true,
    guard: "selected prior bit is 0",
  },
  {
    query: "set_bit_or",
    before: "return mask |",
    after: "return mask &",
    slotBit: 0,
    down: true,
    guard: "selected prior bit is 0",
  },
  {
    query: "clear_bit_complement",
    before: "mask & ~(1n << BigInt(slot))",
    after: "mask & (1n << BigInt(slot))",
    slotBit: 1,
    down: false,
    guard: "selected prior bit is 1",
  },
] as const;

interface ProofModel {
  mask_decimal: string;
  slot_decimal: string;
  selected_prior_bit: 0 | 1;
  expected: string | boolean;
  predicted: string | boolean;
  guard_evaluated: boolean;
}

function invokeProof(
  digest: string,
  query: string,
  mutatedPath?: string,
): Record<string, unknown> {
  const child = runCli([
    "--source", mutatedPath ?? sourcePath,
    "--expected-path", mutatedPath ?? relativeSourcePath,
    "--sha256", digest,
    "--query", `${query}:Counterexample`,
    "--origin-path", sourcePath,
    "--origin-sha256", sourceDigest,
    "--mutation", query,
  ]);
  assert.equal(child.error, undefined, child.error?.message);
  assert.equal(child.status, 0, child.stderr);
  return JSON.parse(child.stdout) as Record<string, unknown>;
}

function runCli(cliArgs: string[]) {
  return spawnSync(
    "cargo",
    ["run", "--locked", "-q", "-p", "typescript-warrant-ir", "--example", "source-proof", "--", ...cliArgs],
    { cwd: rustWorkspace, encoding: "utf8" },
  );
}

function assertPhysicalPathBoundTo(reportedPath: string, expectedPath: string): void {
  assert.ok(isAbsolute(reportedPath), "reported physical path must be absolute");
  assert.ok(isAbsolute(expectedPath), "expected physical path must be absolute");
  const reportedIdentity = realpathSync.native(reportedPath);
  const expectedIdentity = realpathSync.native(expectedPath);
  assert.equal(reportedIdentity, expectedIdentity);
}

function assertRefused(cliArgs: string[], expectedMessage: RegExp): void {
  const child = runCli(cliArgs);
  assert.equal(child.error, undefined, child.error?.message);
  assert.notEqual(child.status, 0, "a broken binding must fail before proof");
  assert.match(child.stderr, /"status":"refused"/);
  assert.match(child.stderr, expectedMessage);
}

function oracleFrame(
  previous: InputState,
  tick: bigint,
  slot: number,
  down: boolean,
): object {
  const held = Array.from(
    { length: 64 },
    (_, index) => ((previous.held_buttons >> BigInt(index)) & 1n) === 1n,
  );
  const pressed = Array<boolean>(64).fill(false);
  const released = Array<boolean>(64).fill(false);
  const wasDown = held[slot];
  if (down && !wasDown) {
    held[slot] = true;
    pressed[slot] = true;
  } else if (!down && wasDown) {
    held[slot] = false;
    released[slot] = true;
  }
  const mask = (bits: boolean[]): bigint => {
    let result = 0n;
    for (let index = 0; index < 64; index += 1) {
      if (bits[index]) result |= 1n << BigInt(index);
    }
    return result;
  };
  return {
    player_id: previous.player_id,
    tick,
    held_buttons: mask(held),
    pressed_buttons: mask(pressed),
    released_buttons: mask(released),
    axes: [...previous.axes],
  };
}

function encodeResult(result: ReturnType<typeof reduce_frame>): object {
  if (!result.ok) return { error: result.error };
  return {
    player_id: result.value.player_id,
    tick: result.value.tick,
    held_buttons: result.value.held_buttons,
    pressed_buttons: result.value.pressed_buttons,
    released_buttons: result.value.released_buttons,
    axes: [...result.value.axes],
  };
}

async function runMutatedPublicReducer(
  mutatedSource: string,
  previous: InputState,
  tick: bigint,
  changes: ActionChange[],
): Promise<{ ok: boolean; encoded: object }> {
  const temporaryDirectory = mkdtempSync(join(tmpdir(), "gbl050a-mutant-"));
  try {
    writeFileSync(join(temporaryDirectory, "package.json"), '{"type":"module"}\n');
    const transpiled = ts.transpileModule(mutatedSource, {
      compilerOptions: { module: ts.ModuleKind.ES2022, target: ts.ScriptTarget.ES2022 },
    }).outputText;
    writeFileSync(join(temporaryDirectory, "digital.js"), transpiled);
    copyFileSync(
      resolve(packageRoot, "dist-test/src/game_input_frame.js"),
      join(temporaryDirectory, "game_input_frame.js"),
    );
    const imported = await import(pathToFileURL(join(temporaryDirectory, "game_input_frame.js")).href) as {
      reduce_frame: typeof reduce_frame;
    };
    const result = imported.reduce_frame(previous, tick, changes);
    return { ok: result.ok, encoded: encodeResult(result) };
  } finally {
    rmSync(temporaryDirectory, { recursive: true });
  }
}

test("the healthy WIR query binds exact production digital.ts bytes", () => {
  const child = spawnSync(
    "cargo",
    [
      "run", "--locked", "-q", "-p", "typescript-warrant-ir", "--example", "source-proof", "--",
      "--source", sourcePath, "--expected-path", relativeSourcePath,
      "--sha256", sourceDigest, "--query", "healthy:Proved",
    ],
    { cwd: rustWorkspace, encoding: "utf8" },
  );
  assert.equal(child.error, undefined, child.error?.message);
  assert.equal(child.status, 0, child.stderr);
  const report = JSON.parse(child.stdout) as {
    schema: string;
    status: string;
    source: { path: string; physical_path: string; bytes: number; sha256: string };
    profile: { parser: string; lowerer: string };
    solver: string;
    inputs: { mask_bits: number; slot_bits: number; slot_domain: [number, number] };
    queries: Array<{ helper: string; expected_verdict: string; verdict: string }>;
  };
  assert.equal(report.schema, "typescript-warrant-ir/source-proof/v1");
  assert.equal(report.status, "proved");
  assert.equal(report.source.path, relativeSourcePath);
  assertPhysicalPathBoundTo(report.source.physical_path, sourcePath);
  assert.equal(report.source.sha256, sourceDigest);
  assert.equal(report.profile.parser, "ts5.8");
  assert.equal(report.profile.lowerer, "typescript-warrant-ir");
  assert.match(report.solver, /coding_adventures_warrant_ir::Prover using cdcl-sat/);
  assert.match(report.solver, /DRAT-checked/);
  assert.deepEqual(report.inputs, { mask_bits: 64, slot_bits: 6, slot_domain: [0, 63] });
  assert.deepEqual(
    report.queries.map(({ helper, expected_verdict, verdict }) => ({ helper, expected_verdict, verdict })),
    [
      { helper: "bit_is_set", expected_verdict: "Proved", verdict: "UNSAT" },
      { helper: "set_bit", expected_verdict: "Proved", verdict: "UNSAT" },
      { helper: "clear_bit", expected_verdict: "Proved", verdict: "UNSAT" },
    ],
  );
});

test("source-proof admission refuses digest/path/verdict and healthy-provenance mismatches", () => {
  const prefix = [
    "--source", sourcePath,
    "--expected-path", relativeSourcePath,
    "--sha256", sourceDigest,
  ];
  assertRefused(
    ["--source", sourcePath, "--expected-path", relativeSourcePath, "--sha256", "0".repeat(64), "--query", "healthy:Proved"],
    /SHA-256 mismatch/,
  );
  assertRefused(
    ["--source", sourcePath, "--expected-path", "code/packages/typescript/game-input-frame/src/wrong.ts", "--sha256", sourceDigest, "--query", "healthy:Proved"],
    /source path mismatch/,
  );
  assertRefused(
    [...prefix, "--query", "healthy:Counterexample"],
    /unexpected query\/verdict pair/,
  );
  assertRefused(
    [...prefix, "--query", "healthy:Proved", "--origin-path", sourcePath, "--origin-sha256", sourceDigest, "--mutation", "healthy"],
    /healthy query does not accept origin or mutation flags/,
  );
  assertRefused(
    [
      ...prefix, "--query", "bit_is_set_not_equal:Counterexample",
      "--origin-path", sourcePath, "--origin-sha256", "0".repeat(64),
      "--mutation", "bit_is_set_not_equal",
    ],
    /origin source SHA-256 mismatch/,
  );
  assertRefused(
    [
      ...prefix, "--query", "bit_is_set_not_equal:Counterexample",
      "--origin-path", resolve(sourcePath, "../missing-origin.ts"),
      "--origin-sha256", sourceDigest,
      "--mutation", "bit_is_set_not_equal",
    ],
    /origin path:/,
  );
  assertRefused(
    [
      ...prefix, "--query", "bit_is_set_not_equal:Counterexample",
      "--origin-path", sourcePath, "--origin-sha256", sourceDigest,
      "--mutation", "bit_is_set_not_equal",
    ],
    /actual source bytes are not the single admitted/,
  );
});

test("production binding rejects an identical source file at another physical path", () => {
  const temporaryDirectory = mkdtempSync(join(tmpdir(), "gbl050a-same-bytes-"));
  const copiedSourcePath = join(temporaryDirectory, basename(sourcePath));
  try {
    copyFileSync(sourcePath, copiedSourcePath);
    assertRefused(
      [
        "--source", copiedSourcePath,
        "--expected-path", sourcePath,
        "--sha256", sourceDigest,
        "--query", "healthy:Proved",
      ],
      /source path mismatch/,
    );
    const child = runCli([
      "--source", copiedSourcePath,
      "--expected-path", realpathSync.native(copiedSourcePath),
      "--sha256", sourceDigest,
      "--query", "healthy:Proved",
    ]);
    assert.equal(child.error, undefined, child.error?.message);
    assert.equal(child.status, 0, child.stderr);
    const report = JSON.parse(child.stdout) as {
      status: string;
      source: { physical_path: string; sha256: string };
    };
    assert.equal(report.status, "proved");
    assert.equal(report.source.sha256, sourceDigest);
    assert.throws(
      () => assertPhysicalPathBoundTo(report.source.physical_path, sourcePath),
      assert.AssertionError,
    );
    assert.throws(
      () => assertPhysicalPathBoundTo(relativeSourcePath, sourcePath),
      assert.AssertionError,
    );
  } finally {
    rmSync(temporaryDirectory, { recursive: true });
  }
});

test("guarded source-mutant SAT witnesses replay through the public reducer", async (t) => {
  for (const mutant of mutants) {
    const mutantText = sourceText.replace(mutant.before, mutant.after);
    assert.notEqual(mutantText, sourceText, `${mutant.query} mutation must apply`);
    const mutantBytes = Buffer.from(mutantText, "utf8");
    const mutantDigest = createHash("sha256").update(mutantBytes).digest("hex");
    const temporaryDirectory = mkdtempSync(join(tmpdir(), "gbl050a-proof-source-"));
    const mutantPath = join(temporaryDirectory, basename(sourcePath));
    try {
      writeFileSync(mutantPath, mutantBytes);
      const report = invokeProof(mutantDigest, mutant.query, mutantPath) as {
        schema: string;
        status: string;
        source: { path: string | null; identity: string; physical_path: string; sha256: string };
        provenance: {
          origin: { path: string | null; physical_path: string; sha256: string };
          mutation: string;
        };
        solver: string;
        queries: Array<{ helper: string; expected_verdict: string; guard: string; verdict: string; model: ProofModel }>;
      };
      assert.equal(report.schema, "typescript-warrant-ir/source-proof/v1");
      assert.equal(report.status, "proved");
      assert.equal(report.source.path, null, "temporary mutant is outside the repository tree");
      assertPhysicalPathBoundTo(report.source.physical_path, mutantPath);
      assert.equal(
        report.source.identity,
        report.source.physical_path.replaceAll("\\", "/"),
      );
      assert.equal(report.source.sha256, mutantDigest);
      assert.equal(report.provenance.origin.path, relativeSourcePath);
      assertPhysicalPathBoundTo(report.provenance.origin.physical_path, sourcePath);
      assert.equal(report.provenance.origin.sha256, sourceDigest);
      assert.equal(report.provenance.mutation, mutant.query);
      assert.match(report.solver, /coding_adventures_warrant_ir::Prover using cdcl-sat/);
      assert.equal(report.queries.length, 1);
      const query = report.queries[0];
      assert.equal(query.expected_verdict, "Counterexample");
      assert.equal(query.verdict, "SAT");
      assert.match(query.guard, new RegExp(mutant.guard));
      assert.equal(query.model.guard_evaluated, true);
      assert.notDeepEqual(query.model.expected, query.model.predicted);

      const held = BigInt(query.model.mask_decimal);
      const slot = Number(query.model.slot_decimal);
      assert.ok(Number.isInteger(slot) && slot >= 0 && slot < 64);
      assert.equal((held >> BigInt(slot)) & 1n, BigInt(mutant.slotBit));
      assert.equal(query.model.selected_prior_bit, mutant.slotBit);

      const previous: InputState = {
        player_id: 0xffff_ffff,
        held_buttons: held,
        axes: Array<number>(16).fill(0),
      };
      const changes: ActionChange[] = [{ kind: "button", slot, down: mutant.down }];
      const tick = 0xffff_ffff_ffff_ffffn;
      const healthy = reduce_frame(previous, tick, changes);
      assert.equal(healthy.ok, true, "witness must reach the healthy public reducer");
      const expected = oracleFrame(previous, tick, slot, mutant.down);
      assert.deepEqual(encodeResult(healthy), expected);

      const mutatedPublic = await runMutatedPublicReducer(mutantText, previous, tick, changes);
      assert.equal(mutatedPublic.ok, true, "a reducer refusal cannot count as a mutation kill");
      assert.notDeepEqual(mutatedPublic.encoded, expected, `${mutant.query} must alter the full public result`);
      t.diagnostic(`${mutant.query}: SAT witness mask=${held} slot=${slot} prior_bit=${mutant.slotBit} down=${mutant.down}; public mutant executed and full result differs from Boolean oracle; tick=u64::MAX, player=u32::MAX, axes=16 zeroes`);
    } finally {
      rmSync(temporaryDirectory, { recursive: true });
    }
  }
});
