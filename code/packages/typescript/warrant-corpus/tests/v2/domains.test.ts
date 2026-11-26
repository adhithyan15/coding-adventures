/** Domains: descriptor trees, size() and nth() (VER04r section 5.1). */
import { describe, expect, it } from "vitest";
import { KitError } from "../../src/v2/errors.ts";
import { buildDomain, describeType } from "../../src/v2/domains.ts";
import { EMPTY_TABLE, type Ty, parseType, readTypeTable } from "../../src/v2/types.ts";
import { toHex, streamBytes } from "../../src/v2/values.ts";

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
const build = (descriptor: unknown, type: string, present = false) => buildDomain(descriptor, t(type), EMPTY_TABLE, present);

describe("leaf descriptors", () => {
  it("enumerates the VER04i 2.6 domains in their order", () => {
    const u8 = build({ d: "u8_all", size: 256 }, "U8");
    expect([u8.nth(0), u8.nth(255)]).toEqual([0, 255]);
    const edge = build({ d: "u64_edge", size: 377 }, "U64");
    expect(edge.nth(255)).toBe(255n);
    expect(edge.nth(256)).toBe(256n);
    expect(edge.nth(311)).toBe(1n << 63n);
    expect(edge.nth(312)).toBe((1n << 64n) - 1n);
    expect(edge.nth(313)).toBe(((1n << 64n) - 1n) ^ 1n);
    expect(edge.nth(376)).toBe(((1n << 64n) - 1n) ^ (1n << 63n));
    const nat = build({ d: "nat_0_64", size: 65 }, "Nat");
    expect([nat.nth(0), nat.nth(64)]).toEqual([0n, 64n]);
    const shift = build({ d: "nat_shift", size: 73 }, "Nat");
    expect([shift.nth(64), shift.nth(65), shift.nth(72)]).toEqual([64n, 65n, 65536n]);
    const count = build({ d: "nat_count", k: 2, size: 3 }, "Nat");
    expect([count.nth(0), count.nth(2)]).toEqual([0n, 2n]);
  });

  it("enumerates bytes_d1 and bytes_n", () => {
    const d1 = build({ d: "bytes_d1", size: 347 }, "Bytes");
    expect(toHex(d1.nth(0) as Uint8Array)).toBe("");
    expect(toHex(d1.nth(1) as Uint8Array)).toBe("00");
    expect(toHex(d1.nth(257) as Uint8Array)).toBe("0000");
    expect(toHex(d1.nth(273) as Uint8Array)).toBe("000000");
    expect(toHex(d1.nth(346) as Uint8Array)).toBe("a5".repeat(8));
    const widths = [[0, 1], [1, 256], [2, 16], [3, 64], [4, 16], [5, 19], [32, 100]];
    for (const [n, size] of widths) expect(build({ d: "bytes_n", n, size }, `Bytes<${n}>`).size, `n=${n}`).toBe(size);
    const three = build({ d: "bytes_n", n: 3, size: 64 }, "Bytes<3>");
    expect(toHex(three.nth(0) as Uint8Array)).toBe("000000");
    expect(toHex(three.nth(63) as Uint8Array)).toBe("ffffff");
    const four = build({ d: "bytes_n", n: 4, size: 16 }, "Bytes<4>");
    expect(toHex(four.nth(0) as Uint8Array)).toBe("00000000");
    expect(toHex(four.nth(3) as Uint8Array)).toBe("ffffffff");
    expect(toHex(four.nth(4) as Uint8Array)).toBe("01000000");
    expect(toHex(four.nth(15) as Uint8Array)).toBe("000000ff");
    // A wide width is generated on demand, never materialised (4,096 x 12,292 bytes would be 50 MB).
    const wide = build({ d: "bytes_n", n: 4096, size: 12292 }, "Bytes<4096>");
    expect((wide.nth(12291) as Uint8Array)[4095]).toBe(255);
    expect(wide.nth(5)).not.toBe(wide.nth(5));
  });

  it("enumerates fx_full and fx_edge", () => {
    const i8 = build({ d: "fx_full", size: 256, t: "I8" }, "I8");
    expect([i8.nth(0), i8.nth(128), i8.nth(255)]).toEqual([-128, 0, 127]);
    const u16 = build({ d: "fx_full", size: 65536, t: "U16" }, "U16");
    expect(u16.nth(65535)).toBe(65535);
    const edge32 = build({ d: "fx_edge", size: 313, t: "U32" }, "U32");
    expect(edge32.nth(0)).toBe(0);
    expect(edge32.nth(256)).toBe(256);
    const sedge = build({ d: "fx_edge", size: 623, t: "I64" }, "I64");
    expect(sedge.nth(0)).toBe(-(1n << 63n));
    expect(sedge.nth(622)).toBe((1n << 63n) - 1n);
    const i16 = build({ d: "fx_edge", size: 527, t: "I16" }, "I16");
    expect(i16.nth(0)).toBe(-32768);
  });

  it("enumerates a result: Ok(x) for each x, then each error case", () => {
    const small = build({ d: "result", ok: { d: "bool", size: 2 }, errs: ["A", "B"], size: 4 }, "Result<Bool,E>");
    expect([0, 1, 2, 3].map((i) => small.nth(i))).toEqual([{ ok: false }, { ok: true }, { err: "A" }, { err: "B" }]);
    // A large payload is generated on demand.
    const large = build({ d: "result", ok: { d: "fx_full", t: "U16", size: 65536 }, errs: ["A"], size: 65537 }, "Result<U16,E>");
    expect(large.nth(5)).toEqual({ ok: 5 });
    expect(large.nth(65536)).toEqual({ err: "A" });
  });
});

describe("refusals", () => {
  const refuse = (descriptor: unknown, type: string): string => code(() => build(descriptor, type));
  it("is `descriptor` for every malformed or mismatched tree", () => {
    expect(refuse(null, "Bool")).toBe("descriptor");
    expect(refuse([], "Bool")).toBe("descriptor");
    expect(refuse({ size: 2 }, "Bool")).toBe("descriptor");
    expect(refuse({ d: 1, size: 2 }, "Bool")).toBe("descriptor");
    expect(refuse({ d: "bool", size: 2.5 }, "Bool")).toBe("descriptor");
    expect(refuse({ d: "bytes_n", n: 1.5, size: 1 }, "Bytes")).toBe("descriptor");
    expect(refuse({ d: "nat_count", k: 1.5, size: 1 }, "Nat")).toBe("descriptor");
    expect(refuse({ d: "nat_count", k: -1, size: 0 }, "Nat")).toBe("descriptor");
    expect(refuse({ d: "fx_full", t: "Nat", size: 1 }, "Nat")).toBe("descriptor");
    expect(refuse({ d: "fx_full", t: 7, size: 1 }, "Nat")).toBe("descriptor");
    expect(refuse({ d: "fx_edge", t: "U64", size: 377 }, "U64")).toBe("descriptor");
    expect(refuse({ d: "result", ok: { d: "bool", size: 2 }, errs: "AB", size: 4 }, "Result<Bool,E>")).toBe("descriptor");
    expect(refuse({ d: "result", ok: { d: "bool", size: 2 }, errs: ["A"], size: 3 }, "Result<Bool,E>")).toBe("none");
    expect(refuse({ d: "result", ok: { d: "bool", size: 2 }, errs: ["A"], size: 3 }, "Result<U8,E>")).toBe("descriptor");
    expect(refuse({ d: "result", ok: { d: "bool", size: 2 }, errs: ["A"], size: 3 }, "U8")).toBe("descriptor");
    expect(refuse({ d: "result", ok: 3, errs: ["A"], size: 3 }, "Result<Bool,E>")).toBe("descriptor");
    expect(refuse({ d: "bytes_d1", size: 347 }, "Bytes<2>")).toBe("descriptor");
    expect(refuse({ d: "nat_shift", size: 73 }, "U8")).toBe("descriptor");
    expect(refuse({ d: "nat_count", k: 1, size: 2 }, "U8")).toBe("descriptor");
    expect(refuse({ d: "fx_full", t: "I8", size: 256 }, "I16")).toBe("descriptor");
    expect(refuse({ d: "fx_edge", t: "I16", size: 527 }, "U16")).toBe("descriptor");
    expect(refuse({ d: "u64_edge", size: 377 }, "U8")).toBe("descriptor");
    expect(refuse({ d: "u8_all", size: 256 }, "U16")).toBe("descriptor");
    expect(refuse({ d: "nat_0_64", size: 65 }, "U8")).toBe("descriptor");
  });

  it("names the level of the descriptors of other levels; the list is the reserved level, never an unknown kind [D63]", () => {
    const level = (descriptor: unknown): string | undefined => {
      try {
        build(descriptor, "Bool");
      } catch (error) {
        return (error as KitError).level;
      }
      return undefined;
    };
    expect(level({ d: "list", of: { d: "bool", size: 2 }, size: 7 })).toBe("ir.list");
    expect(level({ d: "tuple", of: [], size: 1 })).toBe("ir.tuple");
    expect(level({ d: "variant", ctors: [], size: 1 })).toBe("ir.data");
    expect(level({ d: "option", of: { d: "bool", size: 2 }, size: 3 })).toBe("ir.data");
  });

  it("checks errs against the declared cases when a types table is present (R72)", () => {
    const table = readTypeTable({ E: { kind: "error", cases: ["BAD", "WORSE"] } });
    const make = (errs: string[]): unknown => ({ d: "result", ok: { d: "bool", size: 2 }, errs, size: 2 + errs.length });
    expect(buildDomain(make(["BAD", "WORSE"]), t("Result<Bool,E>"), table, true).size).toBe(4);
    for (const errs of [["WORSE", "BAD"], ["BAD"], ["BAD", "OTHER"], ["BAD", "WORSE", "X"]]) {
      expect(code(() => buildDomain(make(errs), t("Result<Bool,E>"), table, true))).toBe("descriptor");
    }
    expect(code(() => buildDomain(make(["BAD"]), t("Result<Bool,Missing>"), table, true))).toBe("descriptor");
    expect(describeType(t("Result<Bytes<2>,E>"))).toBe("Result<Bytes,E>");
  });

  it("stream-encodes the values it enumerates", () => {
    const d = build({ d: "fx_full", size: 256, t: "I8" }, "I8");
    expect(toHex(streamBytes(d.nth(0), t("I8")))).toBe("1180ffffffffffffff");
  });
});
