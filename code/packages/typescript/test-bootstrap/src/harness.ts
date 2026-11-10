import { Verdict, pass, isRegisteredFailureId, fail } from "./verdict.js";
import { Value } from "./types.js";
import { deepEqual as coreDeepEqual, errorIs as coreErrorIs } from "./verdict.js";
import { subSeed, shuffleOrder } from "./prng.js";

export const LANE = "typescript";
export const USAGE_EXIT = 2;

export interface Options {
  seed: bigint;
  sabotage: boolean;
  silent?: boolean;
}

export interface Summary {
  total: number;
  passed: number;
  failed: number;
  bailed?: boolean;
  exitCode: number;
  tapOutput: string[];
}

export class Ctx {
  readonly seed: bigint;
  readonly sabotage: boolean;

  constructor(seed: bigint, sabotage: boolean) {
    this.seed = seed;
    this.sabotage = sabotage;
  }

  deepEqual(actual: Value, expected: Value): Verdict {
    if (this.sabotage) {
      return pass();
    }
    return coreDeepEqual(actual, expected);
  }

  errorIs(raised: string | null, expectedId: string): Verdict {
    if (this.sabotage) {
      return pass();
    }
    return coreErrorIs(raised, expectedId);
  }
}

export type TestFn = (ctx: Ctx) => Verdict | Promise<Verdict>;

export interface RegisteredTestCase {
  id: string;
  fn: TestFn;
}

export function parseArgs(args: string[]): Options {
  let seed = 0n;
  let sabotage = false;

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg === "--seed") {
      i++;
      if (i >= args.length) {
        throw new Error("--seed needs a value");
      }
      const val = args[i];
      if (!/^[0-9]+$/.test(val)) {
        throw new Error(`--seed needs a decimal u64, got ${JSON.stringify(val)}`);
      }
      seed = BigInt(val);
      if (seed < 0n || seed > 0xffff_ffff_ffff_ffffn) {
        throw new Error(`--seed ${JSON.stringify(val)} does not fit in a u64`);
      }
    } else if (arg === "--sabotage") {
      i++;
      if (i >= args.length || args[i] !== "always-pass") {
        throw new Error("--sabotage needs the mode always-pass");
      }
      sabotage = true;
    } else if (arg === "--nocapture") {
      // Ignored
    } else {
      throw new Error(`unknown argument ${JSON.stringify(arg)}`);
    }
  }

  return { seed, sabotage };
}

function isValidTestId(id: string): boolean {
  if (id.length === 0) return false;
  return /^[A-Za-z0-9._/-]+$/.test(id);
}

export class Suite {
  private registrations: RegisteredTestCase[] = [];

  add(id: string, fn: TestFn): void {
    this.registrations.push({ id, fn });
  }

  async run(options: Partial<Options> = {}): Promise<Summary> {
    const seed = options.seed ?? 0n;
    const sabotage = options.sabotage ?? false;
    const silent = options.silent ?? false;

    const tapLines: string[] = [
      "TAP version 14",
      `# tf-lane: ${LANE}`,
      `# tf-seed: ${seed}`,
    ];

    // Validate test IDs and uniqueness before anything runs (TF01 §5.2)
    const seen = new Set<string>();
    for (const test of this.registrations) {
      if (!isValidTestId(test.id)) {
        tapLines.push(`Bail out! invalid test id ${test.id}`);
        if (!silent) {
          for (const line of tapLines) console.log(line);
        }
        return {
          total: this.registrations.length,
          passed: 0,
          failed: 0,
          bailed: true,
          exitCode: 1,
          tapOutput: tapLines,
        };
      }
      if (seen.has(test.id)) {
        tapLines.push(`Bail out! duplicate test id ${test.id}`);
        if (!silent) {
          for (const line of tapLines) console.log(line);
        }
        return {
          total: this.registrations.length,
          passed: 0,
          failed: 0,
          bailed: true,
          exitCode: 1,
          tapOutput: tapLines,
        };
      }
      seen.add(test.id);
    }

    // Sort IDs by byte order (TF01 §5.2 rule 2)
    const sorted = [...this.registrations].sort((a, b) =>
      a.id < b.id ? -1 : a.id > b.id ? 1 : 0
    );

    tapLines.push(`1..${sorted.length}`);

    // Deterministic shuffle of test cases using SplitMix64 (TF01 §5.2 rule 3)
    const sortedIds = sorted.map((t) => t.id);
    const shuffledIds = shuffleOrder(seed, sortedIds);
    const testMap = new Map(sorted.map((t) => [t.id, t.fn]));

    let passed = 0;
    let failed = 0;

    for (let i = 0; i < shuffledIds.length; i++) {
      const testNumber = i + 1;
      const id = shuffledIds[i];
      const fn = testMap.get(id)!;
      const testSeed = subSeed(seed, id);
      const ctx = new Ctx(testSeed, sabotage);

      let verdict: Verdict;
      try {
        verdict = await fn(ctx);
      } catch (err) {
        if (!silent) {
          console.error(err);
        }
        // Never leak escaping exception message into TAP output (VER00 §7.3 & TF01 §5.2)
        verdict = fail("panic", "the test panicked");
      }

      if (verdict.status === "pass") {
        passed++;
        tapLines.push(`ok ${testNumber} - ${id}`);
      } else {
        // Closed vocabulary check at TAP serialization boundary (TF01 §5.3)
        if (!isRegisteredFailureId(verdict.failureId)) {
          tapLines.push("Bail out! invalid failure id");
          if (!silent) {
            for (const line of tapLines) console.log(line);
          }
          return {
            total: sorted.length,
            passed,
            failed,
            bailed: true,
            exitCode: 1,
            tapOutput: tapLines,
          };
        }
        failed++;
        tapLines.push(`not ok ${testNumber} - ${id}`);
        tapLines.push("  ---");
        tapLines.push(`  failure_id: ${JSON.stringify(verdict.failureId)}`);
        tapLines.push(`  message: ${JSON.stringify(verdict.message)}`);
        tapLines.push("  ...");
      }
    }

    if (!silent) {
      for (const line of tapLines) {
        console.log(line);
      }
    }

    return {
      total: sorted.length,
      passed,
      failed,
      exitCode: failed === 0 ? 0 : 1,
      tapOutput: tapLines,
    };
  }
}
