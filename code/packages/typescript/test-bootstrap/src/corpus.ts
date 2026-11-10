import { Ctx, Suite } from "./harness.js";
import { Record, Value, Verdict } from "./types.js";
import { pass, fail, isRegisteredFailureId } from "./verdict.js";
import { SplitMix64, fnv1a64, shuffleOrder, subSeed } from "./prng.js";
import { parseFile, TfvError } from "./tfv.js";

function getField(record: Record, name: string): Value {
  const v = record.fields.get(name);
  if (v === undefined) {
    throw new Error(`missing field ${JSON.stringify(name)}`);
  }
  return v;
}

function getText(record: Record, name: string): string {
  const v = getField(record, name);
  if (v.kind !== "text") {
    throw new Error(`expected text for field ${JSON.stringify(name)}, got ${v.kind}`);
  }
  return v.value;
}

function getInt(record: Record, name: string): bigint {
  const v = getField(record, name);
  if (v.kind !== "int") {
    throw new Error(`expected int for field ${JSON.stringify(name)}, got ${v.kind}`);
  }
  return BigInt(v.text);
}

function getList(record: Record, name: string): Value[] {
  const v = getField(record, name);
  if (v.kind !== "list") {
    throw new Error(`expected list for field ${JSON.stringify(name)}, got ${v.kind}`);
  }
  return v.items;
}

function getInts(values: Value[]): bigint[] {
  return values.map((v) => {
    if (v.kind !== "int") throw new Error("expected int item in list");
    return BigInt(v.text);
  });
}

function getTexts(values: Value[]): string[] {
  return values.map((v) => {
    if (v.kind !== "text") throw new Error("expected text item in list");
    return v.value;
  });
}

function agree(ok: boolean, detail: () => string): Verdict {
  if (ok) {
    return pass();
  }
  return fail("corpus-mismatch", `disagrees with the corpus: ${detail()}`);
}

function expectVerdict(record: Record, got: Verdict): Verdict {
  const want = getText(record, "verdict");
  let matches = false;
  if (got.status === "pass" && want === "pass") {
    matches = true;
  } else if (got.status === "fail" && want === "fail") {
    matches = got.failureId === getText(record, "failure_id");
  }
  return agree(matches, () => {
    const said =
      got.status === "pass" ? "pass" : `fail (${got.failureId}: ${got.message})`;
    return `corpus expects ${want}, the core said ${said}`;
  });
}

export function checkRecord(ctx: Ctx, record: Record): Verdict {
  const op = getText(record, "op");
  switch (op) {
    case "deep_equal": {
      const actual = getField(record, "actual");
      const expected = getField(record, "expected");
      const got = ctx.deepEqual(actual, expected);
      return expectVerdict(record, got);
    }
    case "error_is": {
      const raisedVal = getField(record, "raised");
      let raised: string | null = null;
      if (raisedVal.kind === "null") {
        raised = null;
      } else if (raisedVal.kind === "text") {
        raised = raisedVal.value;
      } else {
        throw new Error("raised must be null or text");
      }
      const expectedId = getText(record, "expected");
      const got = ctx.errorIs(raised, expectedId);
      return expectVerdict(record, got);
    }
    case "failure_id_valid": {
      const input = getText(record, "input");
      const validVal = getField(record, "valid");
      if (validVal.kind !== "bool") {
        throw new Error("valid must be bool");
      }
      const got = isRegisteredFailureId(input);
      return agree(got === validVal.value, () => `failure_id ${input} valid=${got}`);
    }
    case "splitmix64": {
      const seed = getInt(record, "seed");
      const rng = new SplitMix64(seed);
      const want = getInts(getList(record, "outputs"));
      const got: bigint[] = [];
      for (let i = 0; i < want.length; i++) {
        got.push(rng.nextU64());
      }
      const matches =
        got.length === want.length && got.every((g, idx) => g === want[idx]);
      return agree(matches, () => `outputs mismatch`);
    }
    case "fnv1a64": {
      const inputVal = getField(record, "input");
      if (inputVal.kind !== "bytes") {
        throw new Error("input must be bytes");
      }
      const got = fnv1a64(inputVal.value);
      const want = getInt(record, "hash");
      return agree(got === want, () => `got ${got.toString(16)}, want ${want.toString(16)}`);
    }
    case "shuffle": {
      const seed = getInt(record, "seed");
      const ids = getTexts(getList(record, "ids"));
      const got = shuffleOrder(seed, ids);
      const want = getTexts(getList(record, "order"));
      const matches =
        got.length === want.length && got.every((g, idx) => g === want[idx]);
      return agree(matches, () => `order mismatch: got ${JSON.stringify(got)}`);
    }
    case "sub_seed": {
      const seed = getInt(record, "seed");
      const testId = getText(record, "test_id");
      const got = subSeed(seed, testId);
      const want = getInt(record, "sub_seed");
      return agree(got === want, () => `got ${got}, want ${want}`);
    }
    case "tfv_reject": {
      const text = getText(record, "text");
      try {
        parseFile(text);
        return fail("corpus-mismatch", "the parser accepted a text it must reject");
      } catch (err) {
        if (err instanceof TfvError) {
          return pass();
        }
        return fail("corpus-mismatch", `rejected with non-TfvError: ${String(err)}`);
      }
    }
    default:
      throw new Error(`unknown op ${JSON.stringify(op)}`);
  }
}

export function buildSelfCorpusSuite(records: Record[]): Suite {
  const suite = new Suite();
  for (const record of records) {
    suite.add(record.id, (ctx) => {
      try {
        return checkRecord(ctx, record);
      } catch (err) {
        return fail("corpus-malformed", err instanceof Error ? err.message : String(err));
      }
    });
  }
  return suite;
}
