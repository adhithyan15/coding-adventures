import { describe, expect, it } from "vitest";
import { UsageError } from "../src/errors.ts";
import { FAULTS, faultApplies, getFault } from "../src/faults.ts";
import { parseType, type OpSig } from "../src/values.ts";

function sig(params: [string, string][], ret: string): OpSig {
  return {
    name: "op",
    params: params.map(([name, type]) => [name, parseType(type)] as const),
    ret: parseType(ret),
    widthParams: [],
    fresh: false,
  };
}

const EQ = sig([["a", "Bytes"], ["b", "Bytes"]], "Bool");
const SELECT = sig([["a", "Bytes"], ["b", "Bytes"], ["choice", "Bool"]], "Result<Bytes,E>");
const U64 = sig([["a", "U64"], ["b", "U64"]], "Bool");
const ADD = sig([["a", "U64"], ["b", "U64"]], "U64");
const BYTE = sig([["a", "U8"]], "U8");
const COPY = sig([["a", "Bytes"]], "Bytes");

describe("the registry", () => {
  it("has exactly the ten portable faults", () => {
    expect([...FAULTS.keys()].sort()).toEqual(
      ["drop-to-none", "ignore-high-bit", "ignore-last-byte", "invert", "missing-error", "off-by-one", "return-first-arg", "swap-choice", "truncate-last", "xor-lsb"],
    );
  });

  it("refuses an unknown fault", () => {
    expect(() => getFault("nope")).toThrow(UsageError);
    expect(() => faultApplies("nope", EQ)).toThrow(UsageError);
    expect(faultApplies("invert", EQ)).toBe(true);
  });
});

describe("applicability", () => {
  const applies = (name: string, signature: OpSig): boolean => getFault(name).applies(signature);

  it("follows the signature", () => {
    expect(applies("invert", EQ)).toBe(true);
    expect(applies("invert", COPY)).toBe(false);
    expect(applies("ignore-last-byte", EQ)).toBe(true);
    expect(applies("ignore-last-byte", U64)).toBe(false);
    expect(applies("ignore-high-bit", U64)).toBe(true);
    expect(applies("ignore-high-bit", EQ)).toBe(false);
    expect(applies("swap-choice", SELECT)).toBe(true);
    expect(applies("swap-choice", EQ)).toBe(false);
    expect(applies("missing-error", SELECT)).toBe(true);
    expect(applies("missing-error", EQ)).toBe(false);
    expect(applies("xor-lsb", ADD)).toBe(true);
    expect(applies("xor-lsb", EQ)).toBe(false);
    expect(applies("off-by-one", BYTE)).toBe(true);
    expect(applies("return-first-arg", ADD)).toBe(true);
    expect(applies("return-first-arg", BYTE)).toBe(false);
    expect(applies("truncate-last", COPY)).toBe(true);
    expect(applies("truncate-last", EQ)).toBe(false);
    expect(applies("drop-to-none", EQ)).toBe(false);
    expect(applies("drop-to-none", sig([["a", "Bytes"]], "Bytes"))).toBe(false);
  });
});

describe("behaviour", () => {
  const input = (name: string, args: unknown[], signature: OpSig): unknown[] => getFault(name).onInput(args, signature);
  const output = (name: string, result: unknown, args: unknown[], signature: OpSig): unknown =>
    getFault(name).onOutput(result, args, signature);

  it("ignore-last-byte zeroes each Bytes input without touching the original", () => {
    const original = Uint8Array.of(1, 2, 3);
    const [changed, empty] = input("ignore-last-byte", [original, new Uint8Array(0)], EQ) as Uint8Array[];
    expect(Array.from(changed as Uint8Array)).toEqual([1, 2, 0]);
    expect(Array.from(original)).toEqual([1, 2, 3]);
    expect((empty as Uint8Array).length).toBe(0);
    expect(input("ignore-last-byte", ["not bytes", 1n], EQ)).toEqual(["not bytes", 1n]);
  });

  it("ignore-high-bit clears bit 63", () => {
    expect(input("ignore-high-bit", [2n ** 64n - 1n, 5n], U64)).toEqual([2n ** 63n - 1n, 5n]);
    expect(input("ignore-high-bit", [7, 5n], U64)).toEqual([7, 5n]);
  });

  it("swap-choice negates only `choice`", () => {
    expect(input("swap-choice", [Uint8Array.of(1), Uint8Array.of(2), true], SELECT)[2]).toBe(false);
    expect(input("swap-choice", [Uint8Array.of(1), Uint8Array.of(2), false], SELECT)[0]).toEqual(Uint8Array.of(1));
  });

  it("invert, xor-lsb, off-by-one, return-first-arg, truncate-last, drop-to-none", () => {
    expect(output("invert", true, [], EQ)).toBe(false);
    expect(output("xor-lsb", 4n, [], ADD)).toBe(5n);
    expect(output("xor-lsb", 4, [], BYTE)).toBe(5);
    expect(output("xor-lsb", "x", [], ADD)).toBe("x");
    expect(output("off-by-one", 2n ** 64n - 1n, [], ADD)).toBe(0n);
    expect(output("off-by-one", 255, [], BYTE)).toBe(0);
    expect(output("off-by-one", 255n, [], sig([["a", "U8"]], "U8"))).toBe(0n);
    expect(output("off-by-one", "x", [], ADD)).toBe("x");
    expect(output("return-first-arg", 9n, [3n, 4n], ADD)).toBe(3n);
    expect(output("truncate-last", Uint8Array.of(1, 2), [], COPY)).toEqual(Uint8Array.of(1));
    expect(output("truncate-last", Uint8Array.of(), [], COPY)).toEqual(Uint8Array.of());
    expect(output("truncate-last", "x", [], COPY)).toBe("x");
    expect(output("drop-to-none", 1, [], COPY)).toEqual(["none"]);
  });

  it("missing-error turns an Err into an Ok of the first Bytes argument", () => {
    const first = Uint8Array.of(7);
    expect(output("missing-error", ["err", "E"], [first, Uint8Array.of(8), true], SELECT)).toEqual(["ok", first]);
    expect(output("missing-error", ["ok", first], [first], SELECT)).toEqual(["ok", first]);
    expect(output("missing-error", ["err", "E"], [1n], sig([["a", "U64"]], "Result<U64,E>"))).toEqual(["err", "E"]);
    expect(input("missing-error", [first], SELECT)).toEqual([first]);
  });
});
