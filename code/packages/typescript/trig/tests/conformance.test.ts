// Bit-exact conformance against the shared TRIG-REF-1 vectors.
//
// code/specs/fixtures/trig-reference-v1 holds every vector as a row
//
//     op <TAB> x <TAB> y <TAB> cr <TAB> flags
//
// with x and y as IEEE binary64 bit patterns (16 lowercase hex digits).  We decode
// x by a bit cast, call the public function, encode the result and compare the two
// integers (PHY00a 6.5) -- never printed numbers.  A NaN result matches any NaN;
// `error` means sqrt of a negative, which throws.  Tests run from the package
// directory (see package.json), so the fixture is three levels up.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, it } from "node:test";
import {
  C,
  DEG_PER_RAD,
  INV_PIO2,
  LIMIT,
  MAGIC,
  PI,
  PIO2,
  RAD_PER_DEG,
  S,
  TINY,
  atan,
  atan2,
  cos,
  degrees,
  fromBits,
  radians,
  sin,
  sqrt,
  tan,
  toBits,
} from "../src/trig.js";

const FIXTURE = resolve(process.cwd(), "../../../specs/fixtures/trig-reference-v1");
const QNAN = 0x7ff8000000000000n;

function bits(hex: string): number {
  return fromBits(BigInt("0x" + hex));
}

function call(op: string, x: string): number {
  if (op === "atan2") {
    const [ys, xs] = x.split(",") as [string, string];
    return atan2(bits(ys), bits(xs));
  }
  const v = bits(x);
  switch (op) {
    case "sin":
      return sin(v);
    case "cos":
      return cos(v);
    case "tan":
      return tan(v);
    case "radians":
      return radians(v);
    case "degrees":
      return degrees(v);
    case "sqrt":
      return sqrt(v);
    case "atan":
      return atan(v);
    default:
      throw new Error(`unknown op ${op}`);
  }
}

function checkGroup(group: string): void {
  const text = readFileSync(resolve(FIXTURE, `${group}.tsv`), "utf8");
  assert.ok(text.endsWith("\n"));
  const failures: string[] = [];
  let rows = 0;
  for (const line of text.split("\n").slice(0, -1)) {
    const f = line.split("\t");
    assert.equal(f.length, 5, line);
    const [op, x, y] = f as [string, string, string];
    rows++;
    if (y === "error") {
      let threw = false;
      try {
        call(op, x);
      } catch {
        threw = true;
      }
      if (!threw) failures.push(`${op} ${x}: no error`);
      continue;
    }
    const want = BigInt("0x" + y);
    const got = call(op, x);
    const ok = want === QNAN ? Number.isNaN(got) : toBits(got) === want;
    if (!ok) failures.push(`${op} ${x}: want ${y}, got ${toBits(got).toString(16)}`);
  }
  assert.ok(rows > 0);
  assert.deepEqual(failures.slice(0, 8), [], `${failures.length} failures`);
}

describe("trig-reference-v1 vectors, bit for bit", () => {
  for (const group of ["edge", "sweep-binade", "sweep-cw", "sweep-prng"]) {
    it(group, () => checkGroup(group));
  }
});

describe("constants are the pinned bit patterns", () => {
  it("embedded literals", () => {
    assert.equal(toBits(PI), 0x400921fb54442d18n);
    assert.equal(toBits(1e-15), 0x3cd203af9ee75616n);
    assert.equal(toBits(1.0e308), 0x7fe1ccf385ebc8a0n);
    assert.equal(toBits(1e-17), 0x3c670ef54646d497n);
    assert.equal(toBits(1e-300), 0x01a56e1fc2f8f359n);
    assert.equal(toBits(7.450580596923828e-9), toBits(TINY));
  });
  it("derived constants", () => {
    assert.equal(RAD_PER_DEG, PI / 180.0);
    assert.equal(DEG_PER_RAD, 180.0 / PI);
    assert.equal(toBits(INV_PIO2), 0x3fe45f306dc9c883n);
    assert.equal(toBits(MAGIC), 0x4338000000000000n);
    assert.equal(toBits(LIMIT), 0x41c0000000000000n);
    assert.equal(S.length, 10);
    assert.equal(C.length, 10);
    assert.equal(PIO2.length, 5);
    assert.equal(toBits(S[9] as number), 0xbc62f49b46814157n);
    assert.equal(toBits(C[9] as number), 0xbca6827863b97d97n);
  });
});

describe("contract", () => {
  it("sin(-0) is -0, tiny arguments are returned unchanged", () => {
    assert.equal(toBits(sin(-0)), 0x8000000000000000n);
    assert.equal(sin(5e-324), 5e-324);
    assert.equal(cos(-0), 1);
  });
  it("outside the domain is NaN", () => {
    for (const x of [NaN, Infinity, -Infinity, 1e20, 536870913]) {
      assert.ok(Number.isNaN(sin(x)) && Number.isNaN(cos(x)) && Number.isNaN(tan(x)));
    }
    assert.equal(toBits(sin(536870912)), 0x3fd4e67c0e2622dfn);
  });
  it("atan2 of NaN is NaN and the pinned conventions hold", () => {
    assert.ok(Number.isNaN(atan2(NaN, 1)));
    assert.ok(Number.isNaN(atan2(1, NaN)));
    assert.equal(atan2(-0, -1), PI);
    assert.equal(radians(180), PI);
    assert.equal(degrees(PI), 180);
  });
});
