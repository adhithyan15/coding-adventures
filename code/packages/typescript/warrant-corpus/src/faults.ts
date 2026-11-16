/**
 * # The ten portable fault wrappers (VER04i section 4.2)
 *
 * A *negative control* is how a runner proves it can fail. We deliberately
 * break exactly one operation of the library -- not by editing the library, but
 * by wrapping the callable the runner holds -- and require the suites to notice.
 *
 * ## Why wrap one op at a time
 *
 * Take the claim `mul(a, b) == mul(b, a)`. Break *every* op by XOR-ing the low
 * bit of its result and the claim still holds: `mul(a,b)^1 == mul(b,a)^1`. The
 * suite passes under the fault and looks "insensitive". Per-op scoping removes
 * the ambiguity: a control is the triple (suite kind, op, fault) and perturbs
 * exactly one op.
 *
 * Each fault has two halves, applied around the call, both on **library-form**
 * values (see `values.ts`):
 *
 * - `onInput` rewrites the arguments *before* the library sees them;
 * - `onOutput` rewrites the (already error-normalised) result afterwards.
 *
 * | fault              | applies when                  | behaviour                       |
 * |--------------------|-------------------------------|---------------------------------|
 * | `invert`           | op returns `Bool`             | negate the result               |
 * | `ignore-last-byte` | op has `Bytes` params         | zero each input's last byte     |
 * | `ignore-high-bit`  | op has `U64` params           | clear bit 63 of each input      |
 * | `swap-choice`      | `Bool` param `choice`         | negate it                       |
 * | `missing-error`    | op returns `Result`           | `Err` -> `Ok(first Bytes arg)`  |
 * | `xor-lsb`          | returns U8/U64/Nat            | `result ^ 1`                    |
 * | `off-by-one`       | returns U8/U64/Nat            | `result + 1` (masked)           |
 * | `return-first-arg` | two params of the return type | return the first argument       |
 * | `truncate-last`    | op returns `Bytes`            | drop the final byte             |
 * | `drop-to-none`     | op returns `Option`           | `none`                          |
 *
 * `Option` is outside this profile's value model, so `drop-to-none` never
 * applies to a checked corpus today; it is kept so the registry matches the
 * spec.
 *
 * A wrapper never mutates what it is given: it builds a new value, so a fault
 * cannot corrupt the enumerated domain it is being run against.
 */

import { UsageError } from "./errors.ts";
import { INT_KINDS, U64_MAX, type OpSig } from "./values.ts";

type Args = unknown[];

/** A named transformer with a signature-based applicability test. */
export interface Fault {
  readonly name: string;
  applies(sig: OpSig): boolean;
  onInput(args: Args, sig: OpSig): Args;
  onOutput(result: unknown, args: Args, sig: OpSig): unknown;
}

const sameInput = (args: Args): Args => args;
const sameOutput = (result: unknown): unknown => result;
const hasParam = (sig: OpSig, kind: string): boolean => sig.params.some(([, ty]) => ty.kind === kind);

function mapParams(args: Args, sig: OpSig, kind: string, change: (value: unknown) => unknown): Args {
  return args.map((value, index) => (sig.params[index]?.[1].kind === kind ? change(value) : value));
}

function zeroLast(value: unknown): unknown {
  if (!(value instanceof Uint8Array) || value.length === 0) return value;
  const copy = value.slice();
  copy[copy.length - 1] = 0;
  return copy;
}

function swapChoice(args: Args, sig: OpSig): Args {
  return args.map((value, index) => {
    const param = sig.params[index];
    return param?.[0] === "choice" && param[1].kind === "Bool" ? !value : value;
  });
}

function missingError(result: unknown, args: Args, sig: OpSig): unknown {
  if (Array.isArray(result) && result[0] === "err") {
    const index = sig.params.findIndex(([, ty]) => ty.kind === "Bytes");
    if (index >= 0) return ["ok", args[index]];
  }
  return result;
}

/** `value + 1`, masked to the width of the return type (a `bigint` or a `number`). */
function offByOne(result: unknown, _args: Args, sig: OpSig): unknown {
  if (typeof result === "bigint") return (result + 1n) & (sig.ret.kind === "U8" ? 0xffn : U64_MAX);
  if (typeof result === "number") return (result + 1) & 0xff;
  return result;
}

function xorLsb(result: unknown): unknown {
  if (typeof result === "bigint") return result ^ 1n;
  if (typeof result === "number") return result ^ 1;
  return result;
}

function twoParamsOfReturnType(sig: OpSig): boolean {
  const same = sig.params.filter(([, ty]) => ty.kind === sig.ret.kind);
  return sig.params.length === 2 && same.length === 2;
}

function build(fault: Fault): [string, Fault] {
  return [fault.name, fault];
}

export const FAULTS: ReadonlyMap<string, Fault> = new Map([
  build({
    name: "invert",
    applies: (sig) => sig.ret.kind === "Bool",
    onInput: sameInput,
    onOutput: (result) => !result,
  }),
  build({
    name: "ignore-last-byte",
    applies: (sig) => hasParam(sig, "Bytes"),
    onInput: (args, sig) => mapParams(args, sig, "Bytes", zeroLast),
    onOutput: sameOutput,
  }),
  build({
    name: "ignore-high-bit",
    applies: (sig) => hasParam(sig, "U64"),
    onInput: (args, sig) =>
      mapParams(args, sig, "U64", (value) => (typeof value === "bigint" ? value & (U64_MAX >> 1n) : value)),
    onOutput: sameOutput,
  }),
  build({
    name: "swap-choice",
    applies: (sig) => sig.params.some(([name, ty]) => name === "choice" && ty.kind === "Bool"),
    onInput: swapChoice,
    onOutput: sameOutput,
  }),
  build({
    name: "missing-error",
    applies: (sig) => sig.ret.kind === "Result" && hasParam(sig, "Bytes"),
    onInput: sameInput,
    onOutput: missingError,
  }),
  build({
    name: "xor-lsb",
    applies: (sig) => INT_KINDS.includes(sig.ret.kind),
    onInput: sameInput,
    onOutput: xorLsb,
  }),
  build({
    name: "off-by-one",
    applies: (sig) => INT_KINDS.includes(sig.ret.kind),
    onInput: sameInput,
    onOutput: offByOne,
  }),
  build({
    name: "return-first-arg",
    applies: twoParamsOfReturnType,
    onInput: sameInput,
    onOutput: (_result, args) => args[0],
  }),
  build({
    name: "truncate-last",
    applies: (sig) => sig.ret.kind === "Bytes",
    onInput: sameInput,
    onOutput: (result) => (result instanceof Uint8Array ? result.slice(0, -1) : result),
  }),
  build({
    name: "drop-to-none",
    applies: (sig) => sig.ret.text.startsWith("Option<"),
    onInput: sameInput,
    onOutput: () => ["none"],
  }),
]);

/** Look a fault up by its portable name, or refuse. */
export function getFault(name: string): Fault {
  const fault = FAULTS.get(name);
  if (fault === undefined) throw new UsageError("unknown-fault", `unknown fault \`${name}\``);
  return fault;
}

/** Does the registered `fault` apply to an op with signature `sig`? */
export function faultApplies(fault: string, sig: OpSig): boolean {
  return getFault(fault).applies(sig);
}
