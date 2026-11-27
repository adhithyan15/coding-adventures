/**
 * Falsification first [D42]: deliberately wrong evaluators and codecs.
 *
 * A conformance kit is only worth anything if a *wrong* implementation fails
 * it.  Each mutant below copies this package's `src/` tree to a scratch
 * directory, applies one **textual** edit that breaks one rule of VER04r (a
 * wrap that does not wrap, an `and` that evaluates both sides, a chunk walked
 * first-binder-fastest ...), loads the mutated library, runs the kit through it
 * and demands that the kit fail **by case id**, in each category the mutant is
 * named for.
 *
 * Mutating the text rather than threading test hooks through the library keeps
 * the production code free of switches, and each edit must match the source
 * exactly once: if the source moves and an edit no longer applies, the test
 * fails loudly instead of passing vacuously.
 *
 * The three wrong evaluators section 16.13 [D42] names for the lane runners --
 * no wrap, eager `and`, first-binder-fastest enumeration -- are the first
 * three below.
 */
import { cpSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { pathToFileURL } from "node:url";
import { describe, expect, it } from "vitest";
import { KIT_DIR, tempDir } from "./support.ts";

const SRC = join(dirname(new URL(import.meta.url).pathname), "..", "..", "src");

interface Mutant {
  readonly name: string;
  /** values | arithmetic | ir | domains | chunks | faults */
  readonly area: string;
  readonly file: string;
  readonly edits: readonly (readonly [string, string])[];
  /** The kit categories that must show a failure. */
  readonly kills: readonly string[];
  readonly rule: string;
}

const MUTANTS: readonly Mutant[] = [
  // ---- the three wrong evaluators of [D42] ----
  {
    name: "u8-add-does-not-wrap", area: "arithmetic", file: "fixed.ts", kills: ["v2-arith", "v2-ir", "v2-fault"],
    rule: "every fixed-width + - * wraps (section 3.2)",
    edits: [["if (t.w === 8) return (x) => x & 0xff;", "if (t.w === 8) return (x) => x;"]],
  },
  {
    name: "and-evaluates-both-sides", area: "ir", file: "ir.ts", kills: ["v2-ir"],
    rule: "and short-circuits and an untaken subject call is not counted (section 4.2 rule 2)",
    edits: [["return (env) => a(env) === true && b(env) === true;", "return (env) => { const l = a(env); const r = b(env); return l === true && r === true; };"]],
  },
  {
    name: "first-binder-fastest", area: "chunks", file: "chunks.ts", kills: ["v2-chunk"],
    rule: "the product is row-major, last binder fastest (section 5.2)",
    edits: [
      ["for (let k = count - 1; k >= 0; k -= 1) {\n    idx[k] = rest", "for (let k = 0; k < count; k += 1) {\n    idx[k] = rest"],
      ["let k = count - 1;\n      while (k >= 0) {", "let k = 0;\n      while (k < count) {"],
      ["        idx[k] = 0;\n        env[k] = (domains[k] as Domain).nth(0);\n        k -= 1;", "        idx[k] = 0;\n        env[k] = (domains[k] as Domain).nth(0);\n        k += 1;"],
    ],
  },
  // ---- values and codecs ----
  {
    name: "decimal-text-accepts-a-leading-zero", area: "values", file: "values.ts", kills: ["v2-value"],
    rule: "decimal text has no leading zero (section 2.3)",
    edits: [["const DECIMAL = /^-?(?:0|[1-9][0-9]*)$/;", "const DECIMAL = /^-?[0-9]+$/;"]],
  },
  {
    name: "signed-stream-value-not-sign-extended", area: "values", file: "values.ts", kills: ["v2-value", "v2-domain"],
    rule: "I8..I64 are two's complement sign-extended to 64 bits (section 2.4)",
    edits: [["const fill = signed && v < 0 ? 255 : 0;", "const fill = 0;"]],
  },
  {
    name: "u32-written-big-endian", area: "values", file: "values.ts", kills: ["v2-value", "v2-domain"],
    rule: "stream integers are 8 bytes little-endian (section 2.4)",
    edits: [
      ["b[p + 1] = v & 255;", "b[p + 1] = (v >>> 24) & 255;"],
      ["b[p + 4] = (v >>> 24) & 255;", "b[p + 4] = v & 255;"],
    ],
  },
  {
    name: "bytes-n-length-unchecked", area: "values", file: "values.ts", kills: ["v2-value"],
    rule: "a Bytes<N> value has exactly N bytes (section 2.3)",
    edits: [['if (typeof ty.n === "number" && data.length !== ty.n) return bad("a Bytes<N> value has exactly N bytes");', ""]],
  },
  // ---- arithmetic ----
  {
    name: "u8-add-saturates", area: "arithmetic", file: "ir.ts", kills: ["v2-arith"],
    rule: "overflow wraps, it never saturates (section 3.2)",
    edits: [['if (op === "add") return (env) => wrap((a(env) as number) + (b(env) as number));', 'if (op === "add") return (env) => wrap(Math.min((a(env) as number) + (b(env) as number), (ty as { w: number }).w === 8 ? 255 : 65535));']],
  },
  {
    name: "signed-order-on-the-bit-pattern", area: "arithmetic", file: "ir.ts", kills: ["v2-arith", "v2-ir"],
    rule: "a signed comparison orders by signed value (section 3.2)",
    edits: [["return (env) => (a(env) as number) < (b(env) as number);", "return (env) => ((a(env) as number) >>> 0) < ((b(env) as number) >>> 0);"]],
  },
  {
    name: "nat-product-unchecked-past-2-127", area: "arithmetic", file: "ir.ts", kills: ["v2-arith", "v2-ir"],
    rule: "a Nat or Int node outside -(2^127-1)..=2^127-1 is ir-overflow (section 3.2)",
    edits: [["const r = op === \"add\" ? x + y : op === \"sub\" ? x - y : x * y;\n        if (r > INT_WINDOW || r < -INT_WINDOW) overflow();", "const r = op === \"add\" ? x + y : op === \"sub\" ? x - y : x * y;"]],
  },
  // ---- IR ----
  {
    name: "if-evaluates-both-branches", area: "ir", file: "ir.ts", kills: ["v2-ir"],
    rule: "if evaluates exactly one branch (section 4.2 rule 2)",
    edits: [["return (env) => (c(env) === true ? a(env) : b(env));", "return (env) => { const l = a(env); const r = b(env); return c(env) === true ? l : r; };"]],
  },
  {
    name: "take-does-not-clamp", area: "ir", file: "ir.ts", kills: ["v2-ir"],
    rule: "take and drop are total: a negative count is 0, a count past the end clamps (Appendix A)",
    edits: [["const count = wanted <= 0n ? 0 : wanted >= BigInt(data.length) ? data.length : Number(wanted);", "const count = Number(wanted);"]],
  },
  {
    name: "overflow-class-ignores-the-subject-beneath", area: "ir", file: "ir.ts", kills: ["v2-ir"],
    rule: "a node with a subject_op beneath it overflows as a case failure (R71)",
    edits: [['overflowClass: own ? "case" : "corpus"', 'overflowClass: ctx ? "case" : "corpus"']],
  },
  {
    name: "model-chain-counted-from-the-caller", area: "ir", file: "ir.ts", kills: ["v2-ir"],
    rule: "the 64-frame limit covers every chain, called or not: a chain of 65 models is refused (R77)",
    edits: [['if (info.chain > MAX_DEPTH) fail("depth"', 'if (info.chain > MAX_DEPTH + 1) fail("depth"']],
  },
  {
    name: "static-widths-ignored", area: "ir", file: "ir.ts", kills: ["v2-ir"],
    rule: "a subject_op's static widths are substituted into the op's signature (R70)",
    edits: [["const wanted = substituteWidths((op.params[i] as Param).ty, op.widthParams, widths);", "const wanted = substituteWidths((op.params[i] as Param).ty, op.widthParams, widths.map(() => 0));"]],
  },
  {
    name: "cost-takes-the-cheaper-branch", area: "ir", file: "ir.ts", kills: ["v2-ir"],
    rule: "the cost of an if is 1 + cond + max(then, else) (section 4.2 rule 6)",
    edits: [["cost: 1 + cond.cost + Math.max(then.cost, otherwise.cost)", "cost: 1 + cond.cost + Math.min(then.cost, otherwise.cost)"]],
  },
  {
    name: "bytes-literal-forgets-its-width", area: "ir", file: "ir.ts", kills: ["v2-ir"],
    rule: "a bytes literal has the static type Bytes<len> (R73, R51)",
    edits: [["return { tag, ty: plain ? BYTES : { k: \"Bytes\", n: data.length }, kids: [], sub: false, cost: 1, value: data };", "return { tag, ty: BYTES, kids: [], sub: false, cost: 1, value: data };"]],
  },
  {
    name: "fixed-width-take-count-accepted", area: "ir", file: "ir.ts", kills: ["v2-ir"],
    rule: "a fixed-width take/drop count is ill-typed (R74)",
    edits: [["if (!isExact(count.ty)) fail(\"ill-typed\", `a ${tag} count is a Nat or an Int`);", "if (!isInteger(count.ty)) fail(\"ill-typed\", `a ${tag} count is a Nat or an Int`);"]],
  },
  {
    name: "concat-checked-after-the-allocation", area: "ir", file: "ir.ts", kills: ["v2-ir"],
    rule: "a bytes value over MAX_IR_VALUE is refused before it is allocated (section 4.2)",
    edits: [["if (left.length + right.length > MAX_IR_VALUE) overflow();", "if (left.length + right.length > MAX_IR_VALUE * 2) overflow();"]],
  },
  // ---- domains ----
  {
    name: "signed-fx-edge-in-unsigned-order", area: "domains", file: "domains.ts", kills: ["v2-domain"],
    rule: "a signed fx_edge is in ascending numeric order [D21]",
    edits: [["result = [...set].sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));", "result = [...set];"]],
  },
  {
    name: "errs-duplicates-allowed", area: "domains", file: "domains.ts", kills: ["v2-domain"],
    rule: "errs holds unique names (R72)",
    edits: [["!isName(name) || seen.has(name)", "!isName(name)"]],
  },
  {
    name: "u64-edge-tail-in-the-wrong-order", area: "domains", file: "domains.ts", kills: ["v2-domain"],
    rule: "u64_edge ends with MAX ^ (1<<i) for i = 0..=63 (section 5.1)",
    edits: [["for (let s = 0n; s < 64n; s += 1n) out.push(max ^ (1n << s));", "for (let s = 63n; s >= 0n; s -= 1n) out.push(max ^ (1n << s));"]],
  },
  // ---- chunks ----
  {
    name: "chunk-index-missing-from-the-stream-prefix", area: "chunks", file: "chunks.ts", kills: ["v2-chunk"],
    rule: "each chunk's prefix ends with its u32-LE index (section 5.4)",
    edits: [["new DataView(prefix.buffer).setUint32(context.length, chunk, true);", "new DataView(prefix.buffer).setUint32(context.length, 0, true);"]],
  },
  {
    name: "first-admitted-counted-on-product-ordinals", area: "chunks", file: "chunks.ts", kills: ["v2-chunk"],
    rule: "first_admitted is the running prefix sum of admitted cases (section 5.5)",
    edits: [["    if (Number(first) !== running) bad(\"first_admitted is not the running prefix sum\");\n    running += Number(cases);", "    if (Number(first) !== running) bad(\"first_admitted is not the running prefix sum\");\n    running += suite.chunkSize;"]],
  },
  {
    name: "suite-digest-omits-the-chunk-count", area: "chunks", file: "chunks.ts", kills: ["v2-chunk"],
    rule: "the suite digest frames the chunk count before the digests (section 5.4)",
    edits: [['new TextEncoder().encode("warrant-chunks-v1\\u0000"), u32(hexDigests.length), ...raw', 'new TextEncoder().encode("warrant-chunks-v1\\u0000"), ...raw']],
  },
  {
    name: "slim-chunks-may-omit-the-last-chunk", area: "chunks", file: "chunks.ts", kills: ["v2-chunk"],
    rule: "slim_chunks always holds chunk 0 and chunk m-1 (section 5.6)",
    edits: [["if (raw[0] !== 0 || raw[raw.length - 1] !== chunks - 1)", "if (raw[0] !== 0)"]],
  },
  // ---- faults ----
  {
    name: "pre-rewrites-applied-sequentially", area: "faults", file: "faults.ts", kills: ["v2-fault"],
    rule: "pre rewrites are applied simultaneously (section 8.1)",
    edits: [["for (const rewrite of compiled) out[rewrite.slot] = rewrite.run(args.slice());", "for (const rewrite of compiled) out[rewrite.slot] = rewrite.run(out);"]],
  },
  // ---- ir.fixed and ir.tuple (S10f) ----
  // Two more were tried and SURVIVE the kit, which is a gap in it and not in the lane (reported with S10f): an `Option` anywhere but
  // the first operand of `unwrap_or` is `level-missing` (no case pins it; tests/v2/fixed.test.ts does), and the R98 comparison of an
  // `ok` payload with the declared one (every kit case that would need it is also caught by the type of the whole expression).
  {
    name: "shr-is-arithmetic", area: "arithmetic", file: "fixed.ts", kills: ["v2-ir"],
    rule: "shr is logical on the pattern, then re-read: shr(-1i8, 1) = 127 (Appendix A.1)",
    edits: [["(n >= W ? 0 : read(pat(a) >>> Number(n)))", "(n >= W ? 0 : read(a >> Number(n)))"]],
  },
  {
    name: "sar-is-logical", area: "arithmetic", file: "fixed.ts", kills: ["v2-ir"],
    rule: "sar copies the sign bit (Appendix A.1)",
    edits: [["(n >= W ? (a < 0 ? -1 : 0) : a >> Number(n))", "(n >= W ? (a < 0 ? -1 : 0) : a >>> Number(n))"]],
  },
  {
    name: "shl-count-not-compared-with-the-width", area: "arithmetic", file: "fixed.ts", kills: ["v2-ir"],
    rule: "a count is compared with the width before any host shift: shl(1u32, 32) is 0, not 1 (Appendix A.1, Notes)",
    edits: [["(n >= W ? 0 : wrap(a << Number(n)))", "wrap(a << Number(n % W))"]],
  },
  {
    name: "from-be-reads-little-endian", area: "arithmetic", file: "fixed.ts", kills: ["v2-ir"],
    rule: "from_be reads the bytes most significant first (Appendix A.1)",
    edits: [["const little = tag === \"from_le\";", "const little = tag === \"from_be\";"]],
  },
  {
    name: "div-truncates-instead-of-flooring", area: "arithmetic", file: "fixed.ts", kills: ["v2-ir"],
    rule: "div and rem on Int and Nat are floor, and rem has the divisor's sign (Appendix A.1)",
    edits: [["if (r !== 0n && r < 0n !== b < 0n) {", "if (false as boolean) {"]],
  },
  {
    name: "clz-of-zero-is-zero", area: "arithmetic", file: "fixed.ts", kills: ["v2-ir"],
    rule: "clz and ctz of 0 are the width (Appendix A.1)",
    edits: [["return ((a: number) => BigInt(Math.clz32(pat(a)) - (32 - w))) as (a: never) => Raw;", "return ((a: number) => BigInt(pat(a) === 0 ? 0 : Math.clz32(pat(a)) - (32 - w))) as (a: never) => Raw;"]],
  },
  {
    name: "widen-accepts-a-narrowing", area: "ir", file: "ir.ts", kills: ["v2-ir"],
    rule: "widen needs the operand's range inside the target's (Appendix A.1)",
    edits: [["if (lo < tlo || hi > thi) fail(\"ill-typed\", \"widen needs a range inside the target\");", "if (lo < tlo && hi > thi) fail(\"ill-typed\", \"widen needs a range inside the target\");"]],
  },
  {
    name: "match-on-a-non-result-accepted", area: "ir", file: "ir.ts", kills: ["v2-ir"],
    rule: "a match whose scrutinee is not a Result is ir.data (R76)",
    edits: [["if (scrutinee.ty.k !== \"Result\") throw new KitError(\"level-missing\", \"a match on anything but a Result needs ir.data\", { level: \"ir.data\" });", "if (scrutinee.ty.k === \"Option\") throw new KitError(\"level-missing\", \"x\", { level: \"ir.data\" });"]],
  },
  {
    name: "match-runs-the-last-matching-arm", area: "ir", file: "ir.ts", kills: ["v2-ir"],
    rule: "the patterns are tried in order and the first match wins (Appendix A.1)",
    edits: [["for (let i = 0; i < count; i += 1) if ((matchers[i] as PatternRun)(value, env)) return i;", "for (let i = count - 1; i >= 0; i -= 1) if ((matchers[i] as PatternRun)(value, env)) return i;"]],
  },
  {
    name: "unwrap-or-short-circuits", area: "ir", file: "ir.ts", kills: ["v2-ir"],
    rule: "unwrap_or evaluates both operands: its cost is the sum and an overflow in the default is an overflow (Appendix A.1)",
    edits: [["const fallback = b(env);\n          return (option === undefined ? fallback : option) as Value;", "return (option === undefined ? b(env) : option) as Value;"]],
  },
  {
    name: "tuple-stream-tag-is-60", area: "values", file: "values.ts", kills: ["v2-value", "v2-domain"],
    rule: "a tuple is written `64` and a u32-LE component count (section 2.4)",
    edits: [["sink.buf[sink.pos] = 0x64;", "sink.buf[sink.pos] = 0x60;"]],
  },
  {
    name: "tuple-descriptor-first-component-fastest", area: "domains", file: "domains.ts", kills: ["v2-domain"],
    rule: "a tuple descriptor is the product with the last component varying fastest (section 5.1)",
    edits: [["for (let k = count - 1; k >= 0; k -= 1) {\n            out[k] = (parts[k] as Domain).nth(rest % (sizes[k] as number));", "for (let k = 0; k < count; k += 1) {\n            out[k] = (parts[k] as Domain).nth(rest % (sizes[k] as number));"]],
  },
  {
    name: "fault-tuple-components-unify-like-the-top", area: "faults", file: "types.ts", kills: ["v2-fault"],
    rule: "under a tuple two Bytes widths do not unify: a one-byte literal in a Bytes<2> component is refused (R96, R67)",
    edits: [["if (a.k === \"Bytes\" && b.k === \"Bytes\" && narrow) return sameType(a, b) ? a : null;", ""]],
  },
  {
    name: "fault-flow-demanded-instead-of-a-common-type", area: "faults", file: "faults.ts", kills: ["v2-fault"],
    rule: "a fault expression has a common type with its result, it need not flow (R94)",
    edits: [['if (faultCommon(typed.ty, ret) === null) throw new KitError("ill-typed", "the expression shares no type with the op\'s result");', 'if (!flows(typed.ty, ret)) throw new KitError("ill-typed", "the expression shares no type with the op\'s result");']],
  },
];

/** The scratch copy and the mutated kit runner of one mutant. */
async function load(mutant: Mutant): Promise<{ runKit: (dir: string, options: { sequential: boolean }) => { categories: { category: string; failed: { id: string; note: string }[] }[]; failed: number } }> {
  const root = tempDir(`warrant-mutant-${mutant.name.slice(0, 20)}-`);
  cpSync(SRC, join(root, "src"), { recursive: true });
  const path = join(root, "src", "v2", mutant.file);
  let text = readFileSync(path, "utf8");
  for (const [find, replace] of mutant.edits) {
    expect(text.split(find).length - 1, `${mutant.name}: the edit must match exactly once`).toBe(1);
    text = text.replace(find, () => replace);
  }
  if (mutant.file === "faults.ts" && text.includes("flows(typed.ty")) text = text.replace('import { eraseNamedWidths, faultCommon, typeLevels }', 'import { eraseNamedWidths, faultCommon, flows, typeLevels }');
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, text);
  return (await import(pathToFileURL(join(root, "src", "v2", "kit.ts")).href)) as never;
}

describe("falsification: every mutant makes the kit fail", () => {
  it("covers every area the spec names, with at least 12 mutants", () => {
    expect(MUTANTS.length).toBeGreaterThanOrEqual(12);
    for (const area of ["values", "arithmetic", "ir", "domains", "chunks", "faults"]) {
      expect(MUTANTS.filter((m) => m.area === area).length, area).toBeGreaterThanOrEqual(1);
    }
    expect(new Set(MUTANTS.map((m) => m.name)).size).toBe(MUTANTS.length);
  });

  it("passes the unmodified copy, so a failure below is the mutation's", async () => {
    const control = await load({ name: "control", area: "ir", file: "levels.ts", edits: [], kills: [], rule: "" });
    expect(control.runKit(KIT_DIR, { sequential: false }).failed).toBe(0);
  });

  for (const mutant of MUTANTS) {
    it(`${mutant.name} (${mutant.area}): ${mutant.rule}`, async () => {
      const library = await load(mutant);
      const report = library.runKit(KIT_DIR, { sequential: false });
      expect(report.failed, "the kit must fail").toBeGreaterThan(0);
      for (const category of mutant.kills) {
        const row = report.categories.find((r) => r.category === category);
        expect(row?.failed.length, `${mutant.name} must fail a ${category} case`).toBeGreaterThan(0);
        expect(row?.failed[0]?.id.startsWith(`${category}/`)).toBe(true);
        // N9: a mutant is killed by an *outcome that differs*, never only by a harness crash, which any bug would cause.
        expect(
          row?.failed.some((f) => f.note.startsWith("outcome differs")),
          `${mutant.name} must change an outcome in ${category}, not only crash the harness`,
        ).toBe(true);
      }
    }, 120_000);
  }
});
