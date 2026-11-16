import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { CorpusError } from "../src/errors.ts";
import {
  StreamHasher,
  checkValue,
  decodeJson,
  decodeTfv,
  fromHex,
  lift,
  lower,
  parseType,
  streamPrefix,
  toHex,
  valuesEqual,
  type ErrorTypes,
  type TfvValue,
} from "../src/values.ts";

const ERRORS: ErrorTypes = new Map([["SelectError", ["CT_SELECT_LENGTH_MISMATCH"]]]);
const RESULT = parseType("Result<Bytes,SelectError>");
const tagged = (type: string, value: unknown): unknown => ({ type, value });

describe("parseType", () => {
  it("parses the closed profile", () => {
    expect(parseType("Bool").kind).toBe("Bool");
    expect(parseType("U8").kind).toBe("U8");
    expect(parseType("Bytes")).toEqual({ kind: "Bytes", text: "Bytes" });
    expect(parseType("Bytes<16>").width).toBe(16);
    expect(parseType("Bytes<N>").width).toBe("N");
    expect(RESULT.ok?.kind).toBe("Bytes");
    expect(RESULT.error).toBe("SelectError");
    expect(parseType("Result<Result<Bool,E>,F>").ok?.kind).toBe("Result");
  });

  it("refuses anything else", () => {
    for (const bad of ["", "I32", "Bytes<4097>", "Bytes<>", "List<U8>", "Result<Bool>", "Result<Bool,e>", "Result<Bool,A,B>x"]) {
      expect(() => parseType(bad)).toThrow(CorpusError);
    }
    expect(parseType("Bytes<4096>").width).toBe(4096);
  });
});

describe("hex and equality", () => {
  it("round-trips hex", () => {
    expect(toHex(fromHex("00ff10"))).toBe("00ff10");
    expect(fromHex("")).toEqual(new Uint8Array(0));
  });

  it("compares strictly", () => {
    expect(valuesEqual(true, true)).toBe(true);
    expect(valuesEqual(1n, true)).toBe(false);
    expect(valuesEqual(1n, 1)).toBe(false);
    expect(valuesEqual(Uint8Array.of(1), Uint8Array.of(1))).toBe(true);
    expect(valuesEqual(Uint8Array.of(1), Uint8Array.of(2))).toBe(false);
    expect(valuesEqual(Uint8Array.of(1), Uint8Array.of(1, 1))).toBe(false);
    expect(valuesEqual(Uint8Array.of(1), [1])).toBe(false);
    expect(valuesEqual(["ok", 1n], ["ok", 1n])).toBe(true);
    expect(valuesEqual(["ok", 1n], ["ok", 2n])).toBe(false);
    expect(valuesEqual(["ok", 1n], ["ok"])).toBe(false);
    expect(valuesEqual(["ok"], 1n)).toBe(false);
    expect(valuesEqual({}, {})).toBe(false);
  });
});

describe("decodeJson", () => {
  it("decodes tagged values", () => {
    expect(decodeJson(true, parseType("Bool"), ERRORS)).toBe(true);
    expect(decodeJson(tagged("U64", "18446744073709551615"), parseType("U64"), ERRORS)).toBe(18446744073709551615n);
    expect(decodeJson(tagged("U8", "255"), parseType("U8"), ERRORS)).toBe(255n);
    expect(decodeJson(tagged("Nat", "0"), parseType("Nat"), ERRORS)).toBe(0n);
    expect(decodeJson(tagged("Bytes", "00ff"), parseType("Bytes"), ERRORS)).toEqual(Uint8Array.of(0, 255));
    expect(decodeJson(tagged("Bytes<2>", "00ff"), parseType("Bytes<2>"), ERRORS)).toEqual(Uint8Array.of(0, 255));
  });

  it("decodes a Result", () => {
    const ok = { type: RESULT.text, value: { ok: tagged("Bytes", "01") } };
    const err = { type: RESULT.text, value: { err: "CT_SELECT_LENGTH_MISMATCH" } };
    expect(decodeJson(ok, RESULT, ERRORS)).toEqual(["ok", Uint8Array.of(1)]);
    expect(decodeJson(err, RESULT, ERRORS)).toEqual(["err", "CT_SELECT_LENGTH_MISMATCH"]);
  });

  it("refuses malformed values", () => {
    const refuse = (raw: unknown, type: string): void => {
      expect(() => decodeJson(raw, parseType(type), ERRORS)).toThrow(CorpusError);
    };
    refuse(1, "Bool");
    refuse("true", "Bool");
    refuse(7, "U64");
    refuse(tagged("U64", "7"), "U8".replace("U8", "Nat"));
    refuse(tagged("U64", "01"), "U64");
    refuse(tagged("U64", "-1"), "U64");
    refuse(tagged("U64", "18446744073709551616"), "U64");
    refuse(tagged("U64", 7), "U64");
    refuse(tagged("U8", "256"), "U8");
    refuse(tagged("Bytes", "0"), "Bytes");
    refuse(tagged("Bytes", "0F"), "Bytes");
    refuse(tagged("Bytes<2>", "00"), "Bytes<2>");
    refuse({ type: "Bytes", value: "00", extra: 1 }, "Bytes");
    refuse(null, "Bytes");
    refuse([], "Bytes");
    refuse({ type: RESULT.text, value: { err: "OTHER" } }, RESULT.text);
    refuse({ type: RESULT.text, value: { ok: tagged("Bytes", "00"), err: "X" } }, RESULT.text);
    refuse({ type: RESULT.text, value: [] }, RESULT.text);
    refuse({ type: RESULT.text, value: { err: 1 } }, RESULT.text);
    refuse({ type: "Result<Bytes,Other>", value: { ok: tagged("Bytes", "") } }, RESULT.text);
  });
});

describe("decodeTfv", () => {
  const text = (value: string): TfvValue => ["text", value];
  it("decodes each kind", () => {
    expect(decodeTfv(["bool", false], parseType("Bool"), ERRORS)).toBe(false);
    expect(decodeTfv(["bytes", Uint8Array.of(1)], parseType("Bytes"), ERRORS)).toEqual(Uint8Array.of(1));
    expect(decodeTfv(["int", 9n], parseType("U64"), ERRORS)).toBe(9n);
    expect(decodeTfv(["map", [[text("ok"), ["bytes", Uint8Array.of(2)]]]], RESULT, ERRORS)).toEqual([
      "ok",
      Uint8Array.of(2),
    ]);
    expect(
      decodeTfv(["map", [[text("err"), text("CT_SELECT_LENGTH_MISMATCH")]]], RESULT, ERRORS),
    ).toEqual(["err", "CT_SELECT_LENGTH_MISMATCH"]);
  });

  it("refuses mismatches", () => {
    const refuse = (raw: TfvValue, type: string): void => {
      expect(() => decodeTfv(raw, parseType(type), ERRORS)).toThrow(CorpusError);
    };
    refuse(["int", 1n], "Bool");
    refuse(["bool", true], "Bytes");
    refuse(["bytes", Uint8Array.of(1)], "Bytes<2>");
    refuse(["bool", true], "U64");
    refuse(["int", -1n], "U64");
    refuse(["int", 256n], "U8");
    refuse(["int", 18446744073709551616n], "Nat");
    refuse(["bool", true], RESULT.text);
    refuse(["map", []], RESULT.text);
    refuse(["map", [[["int", 1n], ["bool", true]]]], RESULT.text);
    refuse(["map", [[text("nope"), text("X")]]], RESULT.text);
    refuse(["map", [[text("err"), text("OTHER")]]], RESULT.text);
    refuse(["map", [[text("err"), ["int", 1n]]]], RESULT.text);
  });
});

describe("checkValue is exact", () => {
  const check = (value: unknown, type: string): boolean => checkValue(value, parseType(type), ERRORS);

  it("Bool", () => {
    expect(check(true, "Bool")).toBe(true);
    expect(check(1, "Bool")).toBe(false);
    expect(check(1n, "Bool")).toBe(false);
  });

  it("U64 and Nat are bigint, in range", () => {
    expect(check(0n, "U64")).toBe(true);
    expect(check(18446744073709551615n, "U64")).toBe(true);
    expect(check(18446744073709551616n, "U64")).toBe(false);
    expect(check(-1n, "Nat")).toBe(false);
    expect(check(1, "U64")).toBe(false);
    expect(check(true, "U64")).toBe(false);
  });

  it("U8 is an integral number in range", () => {
    expect(check(255, "U8")).toBe(true);
    expect(check(256, "U8")).toBe(false);
    expect(check(1.5, "U8")).toBe(false);
    expect(check(1n, "U8")).toBe(false);
  });

  it("Bytes is exactly Uint8Array", () => {
    expect(check(new Uint8Array(2), "Bytes")).toBe(true);
    expect(check(Buffer.alloc(2), "Bytes")).toBe(false);
    expect(check(new Uint8Array(2).buffer, "Bytes")).toBe(false);
    expect(check([1, 2], "Bytes")).toBe(false);
    expect(check(new Uint16Array(2), "Bytes")).toBe(false);
    class Sub extends Uint8Array {}
    expect(check(new Sub(2), "Bytes")).toBe(false);
    const detached = new Uint8Array(4);
    structuredClone(detached.buffer, { transfer: [detached.buffer] });
    expect(check(detached, "Bytes")).toBe(false);
  });

  it("Result carries a declared variant", () => {
    expect(check(["ok", new Uint8Array(0)], RESULT.text)).toBe(true);
    expect(check(["err", "CT_SELECT_LENGTH_MISMATCH"], RESULT.text)).toBe(true);
    expect(check(["err", "OTHER"], RESULT.text)).toBe(false);
    expect(check(["err", 1], RESULT.text)).toBe(false);
    expect(check(["ok", 1], RESULT.text)).toBe(false);
    expect(check(["maybe", 1], RESULT.text)).toBe(false);
    expect(check(["ok"], RESULT.text)).toBe(false);
    expect(check(new Uint8Array(0), RESULT.text)).toBe(false);
  });
});

describe("lower and lift", () => {
  it("lowers U8 to number and copies Bytes", () => {
    expect(lower(200n, parseType("U8"))).toBe(200);
    expect(lower(5n, parseType("U64"))).toBe(5n);
    const original = Uint8Array.of(1, 2);
    const copy = lower(original, parseType("Bytes")) as Uint8Array;
    expect(copy).toEqual(original);
    expect(copy.buffer).not.toBe(original.buffer);
  });

  it("lifts a library result to canonical form", () => {
    expect(lift(200, parseType("U8"))).toBe(200n);
    expect(lift(true, parseType("Bool"))).toBe(true);
    expect(lift(["ok", 9], parseType("Result<U8,E>"))).toEqual(["ok", 9n]);
    expect(lift(["err", "V"], RESULT)).toEqual(["err", "V"]);
  });
});

describe("warrant-stream-v1", () => {
  const sha = (...parts: number[][]): string => {
    const hash = createHash("sha256");
    for (const part of parts) hash.update(Uint8Array.from(part));
    return hash.digest("hex");
  };
  const ascii = (text: string): number[] => Array.from(new TextEncoder().encode(text));
  const u32 = (n: number): number[] => [n & 255, (n >> 8) & 255, (n >> 16) & 255, (n >> 24) & 255];

  it("builds the context prefix by hand", () => {
    const prefix = streamPrefix("s", ["Bytes", "Bool"], "Bool");
    expect(Array.from(prefix)).toEqual([
      ...ascii("warrant-stream-v1\0"),
      ...u32(1), ...ascii("s"),
      ...u32(2), ...u32(5), ...ascii("Bytes"), ...u32(4), ...ascii("Bool"),
      ...u32(4), ...ascii("Bool"),
    ]); // prettier-ignore
  });

  it("encodes every value kind as the spec says", () => {
    const prefix = new Uint8Array(0);
    const hasher = new StreamHasher(prefix);
    hasher.write(true, parseType("Bool"));
    hasher.write(false, parseType("Bool"));
    hasher.write(258n, parseType("U64"));
    hasher.write(18446744073709551615n, parseType("U8"));
    hasher.write(Uint8Array.of(7, 8), parseType("Bytes"));
    hasher.write(["ok", Uint8Array.of(9)], RESULT);
    hasher.write(["err", "ABC"], RESULT);
    const f = [255, 255, 255, 255, 255, 255, 255, 255];
    expect(hasher.hexdigest()).toBe(
      sha(
        [1], [0], [0x10, 2, 1, 0, 0, 0, 0, 0, 0], [0x10, ...f],
        [0x20, ...u32(2), 7, 8], [0x50, 0x20, ...u32(1), 9], [0x51, ...u32(3), ...ascii("ABC")],
      ), // prettier-ignore
    );
  });

  it("gives the same digest however the bytes are chunked", () => {
    const one = new StreamHasher(new Uint8Array(0));
    const reference = createHash("sha256");
    const big = new Uint8Array(70_000).fill(3);
    for (let round = 0; round < 40; round += 1) {
      one.write(Uint8Array.of(round), parseType("Bytes"));
      reference.update(Uint8Array.from([0x20, 1, 0, 0, 0, round]));
      one.write(BigInt(round), parseType("Nat"));
      const word = new Uint8Array(9);
      word[0] = 0x10;
      word[1] = round;
      reference.update(word);
    }
    one.write(big, parseType("Bytes"));
    reference.update(Uint8Array.from([0x20, ...u32(70_000)]));
    reference.update(big);
    expect(one.hexdigest()).toBe(reference.digest("hex"));
  });

  it("flushes near the buffer boundary and for a long error name", () => {
    const hasher = new StreamHasher(new Uint8Array(0));
    const reference = createHash("sha256");
    const filler = new Uint8Array(65_530).fill(1);
    hasher.write(filler, parseType("Bytes"));
    reference.update(Uint8Array.from([0x20, ...u32(65_530)]));
    reference.update(filler);
    hasher.write(1n, parseType("U64"));
    reference.update(Uint8Array.from([0x10, 1, 0, 0, 0, 0, 0, 0, 0]));
    const long = "E".repeat(70_000);
    hasher.write(["err", long], RESULT);
    reference.update(Uint8Array.from([0x51, ...u32(70_000)]));
    reference.update(new TextEncoder().encode(long));
    expect(hasher.hexdigest()).toBe(reference.digest("hex"));
  });
});
