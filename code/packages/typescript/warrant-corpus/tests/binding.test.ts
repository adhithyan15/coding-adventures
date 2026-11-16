import { mkdirSync, symlinkSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { Subject, SubjectFailure, parseBinding, resolveExports, type Exports } from "../src/binding.ts";
import { BindingError, CorpusError } from "../src/errors.ts";
import { FAULTS } from "../src/faults.ts";
import { BINDING_TEXT, newLibrary, realCorpus, tempDir } from "./helpers.ts";

const corpus = realCorpus();
const GOOD = new TextEncoder().encode(BINDING_TEXT);
const bytes = (text: string): Uint8Array => new TextEncoder().encode(text);
const replace = (from: string, to: string): Uint8Array => {
  expect(BINDING_TEXT).toContain(from);
  return bytes(BINDING_TEXT.replace(from, to));
};

describe("parseBinding", () => {
  it("parses the directive file", () => {
    const binding = parseBinding(bytes(`# comment\n\n${BINDING_TEXT}`));
    expect(binding.lane).toBe("typescript");
    expect(binding.module).toBe("ct_compare");
    expect(binding.importPath).toBe("src/index.ts");
    expect(binding.seed).toBe(0x5741_5252_414e_5421n);
    expect(binding.ops.get("eq_fixed")?.width).toBe("implicit");
    expect(binding.ops.get("eq")?.width).toBe("none");
    expect(binding.ops.get("select_bytes")?.errors[0]?.[0]).toBe("CT_SELECT_LENGTH_MISMATCH");
    expect(binding.ops.get("select_bytes")?.errors[0]?.[1]).toBe(RangeError);
    expect(binding.values.get("U64")).toBe("bigint");
    expect(binding.raw).toEqual(bytes(`# comment\n\n${BINDING_TEXT}`));
  });

  it("accepts CRLF line ends, a decimal seed and a runtime width", () => {
    const text = BINDING_TEXT.replaceAll("\n", "\r\n").replace("0x5741_5252_414e_5421", "1_000").replace("width=implicit", "width=runtime");
    const binding = parseBinding(bytes(text));
    expect(binding.seed).toBe(1000n);
    expect(binding.ops.get("eq_fixed")?.width).toBe("runtime");
  });

  it("refuses malformed files", () => {
    for (const data of [
      Uint8Array.of(0xff, 0xfe),
      new Uint8Array(70_000),
      bytes("bogus directive\n"),
      bytes(`${BINDING_TEXT}lane typescript\n`),
      replace("lane typescript", "lane"),
      replace("lane typescript\n", ""),
      bytes(BINDING_TEXT.split("\n").filter((line) => !line.startsWith("op ")).join("\n")),
    ]) {
      expect(() => parseBinding(data)).toThrow(BindingError);
    }
  });

  it("refuses malformed directives", () => {
    const values = "values Bool=boolean Bytes=Uint8Array Nat=bigint U64=bigint";
    const select = "errors=CT_SELECT_LENGTH_MISMATCH:RangeError";
    for (const [from, to] of [
      ["seed 0x5741_5252_414e_5421", "seed banana"],
      ["seed 0x5741_5252_414e_5421", "seed 0x"],
      ["seed 0x5741_5252_414e_5421", "seed 0x1_0000_0000_0000_0000"],
      ["import src/index.ts", "import ../outside.ts"],
      ["import src/index.ts", "import /abs/index.ts"],
      ["import src/index.ts", "import src/./index.ts"],
      ["import src/index.ts", "import src//index.ts"],
      ["import src/index.ts", "import a\\b.ts"],
      ["source src", "source .."],
      [values, "values U64=number"],
      [values, "values U64=int"],
      [values, "values Bytes=Buffer"],
      [values, "values U64"],
      [values, "values U64=bigint U64=bigint"],
      [values, "values toString=function"],
      ["op eq export=ctEq\n", "op eq\n"],
      ["op eq export=ctEq\n", "op Eq export=ctEq\n"],
      ["op eq export=ctEq\n", "op eq export=ctEq export=ctEq\n"],
      ["op eq export=ctEq\n", "op eq export=ctEq mystery=1\n"],
      ["op eq export=ctEq\n", "op eq mystery=1\n"],
      ["op eq export=ctEq\n", "op eq export=not-an-identifier\n"],
      ["op eq export=ctEq\n", "op eq export=ctEq width=static\n"],
      ["op eq export=ctEq\n", "op eq export=ctEq width\n"],
      ["op eq export=ctEq\n", "op eq export=ctEq\nop eq export=ctEq\n"],
      [select, "errors=lower:RangeError"],
      [select, "errors=CT_SELECT_LENGTH_MISMATCH"],
      [select, "errors=CT_SELECT_LENGTH_MISMATCH:Error"],
      [select, "errors=CT_SELECT_LENGTH_MISMATCH:Object"],
      [select, "errors=CT_SELECT_LENGTH_MISMATCH:NoSuchError"],
      [select, "errors=CT_SELECT_LENGTH_MISMATCH:toString"],
    ] as const) {
      expect(() => parseBinding(replace(from, to)), `${from} -> ${to}`).toThrow(BindingError);
    }
  });

  it("accepts every built-in error class but Error", () => {
    for (const name of ["RangeError", "TypeError", "SyntaxError", "ReferenceError", "EvalError", "URIError"]) {
      const binding = parseBinding(replace("CT_SELECT_LENGTH_MISMATCH:RangeError", `CT_SELECT_LENGTH_MISMATCH:${name}`));
      expect(binding.ops.get("select_bytes")?.errors[0]?.[1]?.name).toBe(name);
    }
  });
});

describe("resolveExports", () => {
  it("imports the library and finds every export", async () => {
    const lib = newLibrary();
    const exports = await resolveExports(parseBinding(GOOD), lib.packageDir);
    expect([...exports.keys()].sort()).toEqual(["eq", "eq_fixed", "eq_u64", "select_bytes"]);
  });

  it("refuses a missing file, a failing import and a missing export", async () => {
    const lib = newLibrary();
    await expect(resolveExports(parseBinding(replace("src/index.ts", "src/other.ts")), lib.packageDir)).rejects.toThrow(/cannot resolve/);
    writeFileSync(join(lib.packageDir, "src", "broken.ts"), "export const x: number = ;\n");
    await expect(resolveExports(parseBinding(replace("src/index.ts", "src/broken.ts")), lib.packageDir)).rejects.toThrow(/cannot import/);
    writeFileSync(join(lib.packageDir, "src", "partial.ts"), "export const ctEq = 1;\n");
    await expect(resolveExports(parseBinding(replace("src/index.ts", "src/partial.ts")), lib.packageDir)).rejects.toThrow(/missing or not callable/);
  });

  it("refuses a module outside the declared source root", async () => {
    const lib = newLibrary();
    mkdirSync(join(lib.packageDir, "elsewhere"));
    writeFileSync(join(lib.packageDir, "elsewhere", "index.ts"), Object.values(
      { eq: "export const ctEq = () => true;", f: "export const ctEqFixed = () => true;", s: "export const ctSelectBytes = () => 1;", u: "export const ctEqU64 = () => true;" },
    ).join("\n"));
    await expect(resolveExports(parseBinding(replace("import src/index.ts", "import elsewhere/index.ts")), lib.packageDir)).rejects.toThrow(/source-mismatch/);
  });

  // A symlink needs a privilege Windows grants only to administrators or to
  // developer mode, so creating one there would fail for the wrong reason.
  it.skipIf(process.platform === "win32")("resolves a symlink inside src to its real path before judging it", async () => {
    const lib = newLibrary();
    const outside = join(tempDir(), "real.ts");
    writeFileSync(outside, "export const ctEq = () => true;\n");
    symlinkSync(outside, join(lib.packageDir, "src", "link.ts"));
    await expect(resolveExports(parseBinding(replace("src/index.ts", "src/link.ts")), lib.packageDir)).rejects.toThrow(/source-mismatch/);
  });
});

async function subjectFor(overrides: Record<string, string>, fault: string | null = null, faultOp: string | null = null): Promise<Subject> {
  const lib = newLibrary(overrides);
  const binding = parseBinding(GOOD);
  const exports: Exports = await resolveExports(binding, lib.packageDir);
  return new Subject(binding, exports, corpus.ops, corpus.errors, fault === null ? null : (FAULTS.get(fault) ?? null), faultOp);
}

const A = Uint8Array.of(1, 2);
const B = Uint8Array.of(3, 4);
const failure = (call: () => unknown): string => {
  try {
    call();
  } catch (error) {
    if (error instanceof SubjectFailure) return error.failureId;
    throw error;
  }
  return "none";
};

describe("Subject", () => {
  it("calls the library, counts calls and returns canonical values", async () => {
    const subject = await subjectFor({});
    expect(subject.call("eq", [A, A], undefined)).toBe(true);
    expect(subject.call("eq_u64", [1n, 2n], undefined)).toBe(false);
    expect(subject.call("eq_fixed", [A, B], 2)).toBe(false);
    expect(subject.call("select_bytes", [A, B, true], undefined)).toEqual(["ok", A]);
    expect(subject.call("select_bytes", [A, Uint8Array.of(1), true], undefined)).toEqual(["err", "CT_SELECT_LENGTH_MISMATCH"]);
    expect(subject.totalCalls(["eq", "eq_u64"])).toBe(2);
    expect(subject.callsTo("select_bytes")).toBe(2);
    expect(subject.callsTo("ghost")).toBe(0);
    expect(subject.totalCalls(["ghost"])).toBe(0);
  });

  it("gives the library its own copy of every Bytes argument", async () => {
    const subject = await subjectFor({ ctEq: "export function ctEq(a: Uint8Array, b: Uint8Array): boolean { a.fill(9); b.fill(9); return true; }" });
    const left = Uint8Array.of(1, 2);
    subject.call("eq", [left, left], undefined);
    expect(Array.from(left)).toEqual([1, 2]);
  });

  it("maps exactly the declared class to the declared Err", async () => {
    const exact = await subjectFor({ ctSelectBytes: "export function ctSelectBytes(): Uint8Array { throw new RangeError('x'); }" });
    expect(exact.call("select_bytes", [A, B, true], undefined)).toEqual(["err", "CT_SELECT_LENGTH_MISMATCH"]);
  });

  it("treats a subclass, another class, a non-Error and null as unexpected-error", async () => {
    for (const thrown of ["new (class Mine extends RangeError {})('x')", "new TypeError('x')", "'a string'", "null", "undefined", "{ name: 'RangeError' }"]) {
      const subject = await subjectFor({ ctSelectBytes: `export function ctSelectBytes(): Uint8Array { throw ${thrown}; }` });
      expect(failure(() => subject.call("select_bytes", [A, B, true], undefined)), thrown).toBe("unexpected-error");
    }
  });

  it("treats an error from an op that declares none as unexpected-error", async () => {
    const subject = await subjectFor({ ctEq: "export function ctEq(): boolean { throw new RangeError('x'); }" });
    expect(failure(() => subject.call("eq", [A, A], undefined))).toBe("unexpected-error");
  });

  it("refuses results of the wrong exact type", async () => {
    const cases: [string, string, unknown[]][] = [
      ["ctEq", "export const ctEq = () => 1;", [A, A]],
      ["ctEq", "export const ctEq = () => 'true';", [A, A]],
      ["ctEq", "export const ctEq = async () => true;", [A, A]],
      ["ctEqU64", "export const ctEqU64 = () => 1;", [1n, 1n]],
      ["ctSelectBytes", "export const ctSelectBytes = () => Buffer.alloc(1);", [A, B, true]],
      ["ctSelectBytes", "export const ctSelectBytes = () => [1, 2];", [A, B, true]],
      ["ctSelectBytes", "export const ctSelectBytes = () => ['err', 'CT_SELECT_LENGTH_MISMATCH'];", [A, B, true]],
      ["ctSelectBytes", "export const ctSelectBytes = () => undefined;", [A, B, true]],
    ];
    for (const [name, source, args] of cases) {
      const subject = await subjectFor({ [name]: source });
      const op = name === "ctEq" ? "eq" : name === "ctEqU64" ? "eq_u64" : "select_bytes";
      expect(failure(() => subject.call(op, args, undefined)), source).toBe("type-mismatch");
    }
  });

  it("refuses a fresh result that aliases an argument (rule 4)", async () => {
    const first = await subjectFor({ ctSelectBytes: "export const ctSelectBytes = (a: Uint8Array) => a;" });
    expect(failure(() => first.call("select_bytes", [A, B, true], undefined))).toBe("type-mismatch");
    const view = await subjectFor({ ctSelectBytes: "export const ctSelectBytes = (a: Uint8Array) => new Uint8Array(a.buffer);" });
    expect(failure(() => view.call("select_bytes", [A, B, true], undefined))).toBe("type-mismatch");
    // An op that is not `fresh` is not held to rule 4, and an Err has no buffer.
    const plain = await subjectFor({});
    expect(plain.call("select_bytes", [A, Uint8Array.of(1), true], undefined)).toEqual(["err", "CT_SELECT_LENGTH_MISMATCH"]);
  });

  it("asserts widths the corpus states", async () => {
    const subject = await subjectFor({});
    expect(() => subject.call("eq_fixed", [A, Uint8Array.of(1)], 2)).toThrow(CorpusError);
    expect(() => subject.call("eq_fixed", [A, A], 3)).toThrow(/length N/);
  });

  it("asserts widths the signature states (`Bytes<4>`)", async () => {
    const fixed = new Map(corpus.ops);
    const eq = fixed.get("eq");
    if (!eq) throw new Error("eq op missing");
    fixed.set("eq", { ...eq, params: [["a", { kind: "Bytes", text: "Bytes<4>", width: 4 }], ["b", { kind: "Bytes", text: "Bytes<4>", width: 4 }]] });
    const lib = newLibrary();
    const binding = parseBinding(GOOD);
    const subject = new Subject(binding, await resolveExports(binding, lib.packageDir), fixed, corpus.errors);
    expect(() => subject.call("eq", [A, A], undefined)).toThrow(/declared length/);
    expect(() => subject.call("eq", [1n, A], undefined)).toThrow(/declared length/);
    expect(subject.call("eq", [Uint8Array.of(1, 2, 3, 4), Uint8Array.of(1, 2, 3, 4)], undefined)).toBe(true);
  });

  it("passes a runtime width as the last argument", async () => {
    const lib = newLibrary({ ctEqFixed: "export const ctEqFixed = (a: Uint8Array, b: Uint8Array, n: number) => n === 2 && a.length === n && b.length === n;" });
    const binding = parseBinding(replace("width=implicit", "width=runtime"));
    const subject = new Subject(binding, await resolveExports(binding, lib.packageDir), corpus.ops, corpus.errors);
    expect(subject.call("eq_fixed", [A, B], 2)).toBe(true);
  });

  it("applies a fault only to its op, and only where it applies", async () => {
    const inverted = await subjectFor({}, "invert", "eq");
    expect(inverted.call("eq", [A, A], undefined)).toBe(false);
    expect(inverted.call("eq_u64", [1n, 1n], undefined)).toBe(true);
    const everywhere = await subjectFor({}, "invert", null);
    expect(everywhere.call("eq", [A, A], undefined)).toBe(false);
    expect(everywhere.call("eq_u64", [1n, 1n], undefined)).toBe(false);
    expect(everywhere.call("select_bytes", [A, B, true], undefined)).toEqual(["ok", A]);
    const masked = await subjectFor({}, "ignore-high-bit", "eq_u64");
    expect(masked.call("eq_u64", [2n ** 64n - 1n, 2n ** 63n - 1n], undefined)).toBe(true);
  });

  it("does not let a promise's rejection escape", async () => {
    const subject = await subjectFor({ ctEq: "export const ctEq = () => Promise.reject(new Error('late'));" });
    expect(failure(() => subject.call("eq", [A, A], undefined))).toBe("type-mismatch");
    await new Promise((resolve) => setTimeout(resolve, 5));
  });
});
