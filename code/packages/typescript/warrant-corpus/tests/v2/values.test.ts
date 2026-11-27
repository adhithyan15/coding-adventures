/** Values: decode by declared type, the canonical stream, TFV, equality (VER04r sections 2.3 to 2.5, 7.2). */
import { describe, expect, it } from "vitest";
import { KitError } from "../../src/v2/errors.ts";
import { parseTfv } from "../../src/tfv.ts";
import { type Ty, parseType, readTypeTable } from "../../src/v2/types.ts";
import {
  StreamSink, decodeJson, decodeTfv, encodeJson, fixedRange, fromHex, isSmall, makeWriter, parseDecimal, streamBytes,
  toHex, valuesEqual,
} from "../../src/v2/values.ts";
import { createHash } from "node:crypto";

const table = readTypeTable({ E: { kind: "error", cases: ["BAD", "WORSE"] } });
const t = (text: string): Ty => parseType(text);
const code = (run: () => unknown): string => {
  try {
    run();
  } catch (error) {
    if (error instanceof KitError) return error.code;
    throw error;
  }
  return "none";
};

describe("hex and decimal text", () => {
  it("round-trips lowercase hex and refuses the rest", () => {
    expect(toHex(Uint8Array.of(0, 15, 255))).toBe("000fff");
    expect(fromHex("000fff")).toEqual(Uint8Array.of(0, 15, 255));
    expect(fromHex("")).toEqual(new Uint8Array(0));
    for (const bad of ["0", "0F", "zz", "0x00", "00 "]) expect(fromHex(bad), bad).toBeUndefined();
  });

  it("parses canonical decimal only: no +, no leading zero, no -0, at most 20 digits", () => {
    expect(parseDecimal("0")).toBe(0n);
    expect(parseDecimal("-5")).toBe(-5n);
    expect(parseDecimal("18446744073709551615")).toBe(18446744073709551615n);
    expect(parseDecimal("99999999999999999999")).toBe(99999999999999999999n);
    for (const bad of ["", "-", "+1", "01", "00", "-0", "1.0", "1e3", " 1", "1 ", "0x1", "100000000000000000000", "-100000000000000000000"]) {
      expect(parseDecimal(bad), bad).toBeUndefined();
    }
    expect(parseDecimal(5)).toBeUndefined();
    expect(parseDecimal(null)).toBeUndefined();
  });

  it("knows the ranges of fixed-width types and which are held as numbers", () => {
    expect(fixedRange("U", 8)).toEqual([0n, 255n]);
    expect(fixedRange("I", 8)).toEqual([-128n, 127n]);
    expect(fixedRange("I", 64)).toEqual([-(2n ** 63n), 2n ** 63n - 1n]);
    expect(isSmall(t("U32"))).toBe(true);
    expect(isSmall(t("I32"))).toBe(true);
    expect(isSmall(t("U64"))).toBe(false);
    expect(isSmall(t("Nat"))).toBe(false);
    expect(isSmall(t("Bool"))).toBe(false);
  });
});

describe("decodeJson", () => {
  it("decodes each type to its host form", () => {
    expect(decodeJson(true, t("Bool"), table)).toBe(true);
    expect(decodeJson("255", t("U8"), table)).toBe(255);
    expect(decodeJson("-128", t("I8"), table)).toBe(-128);
    expect(decodeJson("4294967295", t("U32"), table)).toBe(4294967295);
    expect(decodeJson("18446744073709551615", t("U64"), table)).toBe(18446744073709551615n);
    expect(decodeJson("-9223372036854775808", t("I64"), table)).toBe(-(2n ** 63n));
    expect(decodeJson("18446744073709551615", t("Nat"), table)).toBe(18446744073709551615n);
    expect(decodeJson("-3", parseType("Int", { allowInt: true }), table)).toBe(-3n);
    expect(decodeJson("0a0b", t("Bytes"), table)).toEqual(Uint8Array.of(10, 11));
    expect(decodeJson("0a0b", t("Bytes<2>"), table)).toEqual(Uint8Array.of(10, 11));
    expect(decodeJson({ ok: "7" }, t("Result<U8,E>"), table)).toEqual({ ok: 7 });
    expect(decodeJson({ err: "WORSE" }, t("Result<U8,E>"), table)).toEqual({ err: "WORSE" });
  });

  it("refuses with `value` everything section 2.3 refuses", () => {
    const refuse = (raw: unknown, ty: string): string => code(() => decodeJson(raw, t(ty), table));
    expect(refuse("1", "Bool")).toBe("value");
    expect(refuse(1, "U8")).toBe("value");
    expect(refuse("256", "U8")).toBe("value");
    expect(refuse("-1", "U8")).toBe("value");
    expect(refuse("128", "I8")).toBe("value");
    expect(refuse("18446744073709551616", "Nat")).toBe("value");
    expect(refuse("-1", "Nat")).toBe("value");
    expect(refuse(0, "Bytes")).toBe("value");
    expect(refuse("0A", "Bytes")).toBe("value");
    expect(refuse("00".repeat(4097), "Bytes")).toBe("value");
    expect(refuse("00", "Bytes<2>")).toBe("value");
    expect(refuse("ok", "Result<U8,E>")).toBe("value");
    expect(refuse([], "Result<U8,E>")).toBe("value");
    expect(refuse(null, "Result<U8,E>")).toBe("value");
    expect(refuse({}, "Result<U8,E>")).toBe("value");
    expect(refuse({ ok: "1", err: "BAD" }, "Result<U8,E>")).toBe("value");
    expect(refuse({ maybe: "1" }, "Result<U8,E>")).toBe("value");
    expect(refuse({ err: "NOPE" }, "Result<U8,E>")).toBe("value");
    expect(refuse({ err: 1 }, "Result<U8,E>")).toBe("value");
    expect(code(() => decodeJson({ ok: "1" }, { k: "Result", ok: null, err: "E" }, table))).toBe("value");
    expect(code(() => decodeJson({ err: "BAD" }, { k: "Result", ok: t("U8"), err: null }, table))).toBe("value");
  });

  it("names the level a type of another level would need", () => {
    // A tuple is held (S10f), so the level a tuple needs is only named when something *inside* it lacks one.
    for (const [ty, level] of [["Option<U8>", "ir.data"], ["List<U8>", "ir.list"], ["Rec", "ir.data"], ["(U8,Option<U8>)", "ir.data"], ["(U8,List<U8>)", "ir.list"]] as const) {
      try {
        decodeJson(ty.startsWith("(") ? ["1", []] : [], t(ty), table);
        expect.unreachable();
      } catch (error) {
        expect(error).toBeInstanceOf(KitError);
        expect((error as KitError).code).toBe("level-missing");
        expect((error as KitError).level).toBe(level);
      }
    }
  });
});

describe("encodeJson and equality", () => {
  it("is the inverse of decodeJson, without range checks", () => {
    expect(encodeJson(true, t("Bool"))).toBe(true);
    expect(encodeJson(7, t("U8"))).toBe("7");
    expect(encodeJson(-7n, t("I64"))).toBe("-7");
    expect(encodeJson(Uint8Array.of(1, 2), t("Bytes"))).toBe("0102");
    expect(encodeJson({ ok: 7 }, t("Result<U8,E>"))).toEqual({ ok: "7" });
    expect(encodeJson({ err: "BAD" }, t("Result<U8,E>"))).toEqual({ err: "BAD" });
    expect(encodeJson({ ok: 1 }, { k: "Result", ok: null, err: "E" })).toEqual({ ok: null });
    expect(encodeJson([7, true], t("(U8,Bool)"))).toEqual(["7", true]);
    expect(encodeJson([[1n, 2n], { ok: [3, 4] }], t("((U64,I64),Result<(U8,U8),E>)"))).toEqual([["1", "2"], { ok: ["3", "4"] }]);
    expect(code(() => encodeJson(null, t("Option<U8>")))).toBe("level-missing");
  });

  it("compares structurally: order matters, errors are nominal, bytes by content", () => {
    expect(valuesEqual(Uint8Array.of(1, 2), Uint8Array.of(1, 2))).toBe(true);
    expect(valuesEqual(Uint8Array.of(1, 2), Uint8Array.of(2, 1))).toBe(false);
    expect(valuesEqual(Uint8Array.of(1), Uint8Array.of(1, 2))).toBe(false);
    expect(valuesEqual(Uint8Array.of(1), 1)).toBe(false);
    expect(valuesEqual({ ok: 1 }, { ok: 1 })).toBe(true);
    expect(valuesEqual({ ok: 1 }, { ok: 2 })).toBe(false);
    expect(valuesEqual({ err: "A" }, { err: "A" })).toBe(true);
    expect(valuesEqual({ err: "A" }, { err: "B" })).toBe(false);
    expect(valuesEqual({ ok: 1 }, { err: "A" })).toBe(false);
    expect(valuesEqual({ err: "A" }, { ok: 1 })).toBe(false);
    expect(valuesEqual({ ok: 1 }, 1)).toBe(false);
    expect(valuesEqual({ ok: 1 }, null)).toBe(false);
    expect(valuesEqual(1n, 1n)).toBe(true);
    expect(valuesEqual(1, 1n)).toBe(false);
    expect(valuesEqual(true, 1)).toBe(false);
  });
});

describe("the canonical stream, version 2 (section 2.4)", () => {
  const hex = (value: unknown, ty: string): string => toHex(streamBytes(value, t(ty)));
  it("writes the section 2.4 rows", () => {
    expect(hex(true, "Bool")).toBe("01");
    expect(hex(false, "Bool")).toBe("00");
    expect(hex(1, "U8")).toBe("100100000000000000");
    expect(hex(0xdeadbeef, "U32")).toBe("10efbeadde00000000");
    expect(hex(-1, "I8")).toBe("11ffffffffffffffff");
    expect(hex(-128, "I8")).toBe("1180ffffffffffffff");
    expect(hex(127, "I8")).toBe("117f00000000000000");
    expect(hex(-2147483648, "I32")).toBe("1100000080ffffffff");
    expect(hex(18446744073709551615n, "U64")).toBe("10ffffffffffffffff");
    expect(hex(-1n, "I64")).toBe("11ffffffffffffffff");
    expect(hex(2n ** 63n - 1n, "I64")).toBe("11ffffffffffffff7f");
    expect(hex(5n, "Nat")).toBe("100500000000000000");
    expect(hex(Uint8Array.of(0xaa, 0xbb), "Bytes")).toBe("2002000000aabb");
    expect(hex({ ok: 1 }, "Result<U8,E>")).toBe("50100100000000000000");
    expect(hex({ err: "BAD" }, "Result<U8,E>")).toBe("5103000000424144");
  });

  it("refuses a Nat outside 0..=2^64-1 and a type of a level it lacks", () => {
    expect(code(() => streamBytes(-1n, t("Nat")))).toBe("value");
    expect(code(() => streamBytes(2n ** 64n, t("Nat")))).toBe("value");
    expect(code(() => makeWriter(t("(U8,Option<U8>)")))).toBe("level-missing");
    expect(code(() => makeWriter(t("Option<U8>")))).toBe("level-missing");
  });

  it("flushes a full buffer and writes a long value straight through", () => {
    const seen: number[] = [];
    const hash = createHash("sha256");
    const sink = new StreamSink((bytes) => {
      seen.push(bytes.length);
      hash.update(bytes);
    });
    const write = makeWriter(t("Bytes"));
    const big = new Uint8Array(70_000).fill(7);
    write(sink, big);
    write(sink, new Uint8Array(20_000).fill(1));
    write(sink, new Uint8Array(20_000).fill(2));
    write(sink, new Uint8Array(20_000).fill(3));
    write(sink, new Uint8Array(0));
    sink.u32(0xffffffff);
    sink.text("hi");
    sink.flush();
    sink.flush();
    expect(seen.reduce((a, b) => a + b, 0)).toBe(5 + 70_000 + 3 * (5 + 20_000) + 5 + 4 + 2);
    expect(seen.length).toBeGreaterThan(2);
    expect(hash.digest("hex")).toHaveLength(64);
  });
});

describe("TFV values (section 2.5)", () => {
  const one = (text: string, ty: string) => {
    const [, fields] = parseTfv(`tfv 1\nx R1 Kv ${text}\n`)[0] as [string, Map<string, never>];
    return decodeTfv(fields.get("v") as never, t(ty), table);
  };
  it("reads booleans, signed integers, bytes and the M form of Result", () => {
    expect(one("T", "Bool")).toBe(true);
    expect(one("F", "Bool")).toBe(false);
    expect(one("I255", "U8")).toBe(255);
    expect(one("I-128", "I8")).toBe(-128);
    expect(one("I18446744073709551615", "U64")).toBe(18446744073709551615n);
    expect(one("I18446744073709551615", "Nat")).toBe(18446744073709551615n);
    expect(one("X0a0b", "Bytes")).toEqual(Uint8Array.of(10, 11));
    expect(one("X0a0b", "Bytes<2>")).toEqual(Uint8Array.of(10, 11));
    expect(one("M1 S6f6b I7", "Result<U8,E>")).toEqual({ ok: 7 });
    expect(one("M1 S657272 S424144", "Result<U8,E>")).toEqual({ err: "BAD" });
  });

  it("refuses with `value` a form that is not the declared type's", () => {
    for (const [text, ty] of [
      ["I1", "Bool"], ["T", "U8"], ["I256", "U8"], ["I-1", "U8"], ["I-1", "Nat"], ["I18446744073709551616", "Nat"],
      ["I1", "Bytes"], ["X00", "Bytes<2>"], ["I1", "Result<U8,E>"], ["M2 S6f6b I1 S657272 S424144", "Result<U8,E>"],
      ["M1 I1 I1", "Result<U8,E>"], ["M1 S6f6b T", "Result<U8,E>"], ["M1 S657272 S4e4f5045", "Result<U8,E>"], ["M1 S657272 I1", "Result<U8,E>"],
      ["M1 S6e6f I1", "Result<U8,E>"],
    ] as const) {
      expect(code(() => one(text, ty)), `${text} as ${ty}`).toBe("value");
    }
    expect(code(() => one("X" + "00".repeat(4097), "Bytes"))).toBe("value");
    expect(code(() => decodeTfv(["int", 1n], { k: "Result", ok: null, err: "E" }, table))).toBe("value");
    expect(code(() => decodeTfv(["map", [[["text", "ok"], ["int", 1n]]]], { k: "Result", ok: null, err: "E" }, table))).toBe("value");
    expect(code(() => decodeTfv(["map", [[["text", "err"], ["text", "BAD"]]]], { k: "Result", ok: t("U8"), err: null }, table))).toBe("value");
    expect(decodeTfv(["int", -3n], parseType("Int", { allowInt: true }), table)).toBe(-3n);
    expect(code(() => one("I1", "(U8,U8)"))).toBe("value");
    expect(code(() => one("L1 I1", "(U8,U8)"))).toBe("value");
    expect(code(() => one("L2 I1 I2", "(U8,Option<U8>)"))).toBe("level-missing");
  });
});
