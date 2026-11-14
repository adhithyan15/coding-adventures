import { describe, expect, it } from "vitest";
import { binderDomain, binderShape, domainShape, domainValues, product } from "../src/domains.ts";
import { CorpusError } from "../src/errors.ts";
import { toHex } from "../src/values.ts";

const hex = (values: unknown[]): string[] => (values as Uint8Array[]).map(toHex);

describe("domainValues", () => {
  it("bool and the integer domains", () => {
    expect(domainValues("bool")).toEqual({ values: [false, true], type: "Bool" });
    const u8 = domainValues("u8_all");
    expect(u8.type).toBe("U8");
    expect(u8.values).toHaveLength(256);
    expect(u8.values[255]).toBe(255n);
    const nat = domainValues("nat_0_64");
    expect(nat.values).toHaveLength(65);
    expect(nat.values[64]).toBe(64n);
  });

  it("u64_edge has 377 values, in the spec's order, none rounded", () => {
    const { values, type } = domainValues("u64_edge");
    expect(type).toBe("U64");
    expect(values).toHaveLength(377);
    expect(values[0]).toBe(0n);
    expect(values[255]).toBe(255n);
    expect(values[256]).toBe(256n);
    expect(values[311]).toBe(1n << 63n);
    expect(values[312]).toBe(2n ** 64n - 1n);
    expect(values[313]).toBe(2n ** 64n - 2n);
    expect(values[376]).toBe(2n ** 64n - 1n - (1n << 63n));
    expect(new Set(values).size).toBe(377);
  });

  it("bytes_d1 has 347 values in the spec's order", () => {
    const { values, type } = domainValues("bytes_d1");
    expect(type).toBe("Bytes");
    const text = hex(values);
    expect(text).toHaveLength(347);
    expect(text[0]).toBe("");
    expect(text[1]).toBe("00");
    expect(text[256]).toBe("ff");
    expect(text.slice(257, 261)).toEqual(["0000", "0001", "0080", "00ff"]);
    expect(text[272]).toBe("ffff");
    expect(text[273]).toBe("000000");
    expect(text[336]).toBe("ffffff");
    expect(text.slice(337)).toEqual([
      "00000000", "a5a5a5a5", "0000000000", "a5a5a5a5a5", "000000000000", "a5a5a5a5a5a5",
      "00000000000000", "a5a5a5a5a5a5a5", "0000000000000000", "a5a5a5a5a5a5a5a5",
    ]); // prettier-ignore
  });

  it("bytes_n(N): small widths are the d1 members, larger ones are fills and single bytes", () => {
    expect(hex(domainValues("bytes_n(0)").values)).toEqual([""]);
    expect(domainValues("bytes_n(1)").values).toHaveLength(256);
    expect(domainValues("bytes_n(2)").values).toHaveLength(16);
    expect(domainValues("bytes_n(3)").values).toHaveLength(64);
    const four = hex(domainValues("bytes_n(4)").values);
    expect(four.slice(0, 4)).toEqual(["00000000", "01010101", "80808080", "ffffffff"]);
    expect(four.slice(4, 7)).toEqual(["01000000", "80000000", "ff000000"]);
    expect(four).toHaveLength(16);
    expect(domainValues("bytes_n(16)")).toMatchObject({ type: "Bytes<16>" });
    expect(domainValues("bytes_n(16)").values).toHaveLength(52);
  });

  it("refuses unknown and out-of-bounds descriptors", () => {
    for (const bad of ["", "bytes", "BOOL", "bytes_n()", "bytes_n(-1)", "bytes_n(4097)", "bytes_n(1.5)", "u64"]) {
      expect(() => domainValues(bad)).toThrow(CorpusError);
    }
    expect(() => domainShape("bytes_n(99999999999999999999)")).toThrow(CorpusError);
  });

  it("shapes agree with the enumerated domains", () => {
    for (const name of ["bool", "u8_all", "u64_edge", "nat_0_64", "bytes_d1", "bytes_n(0)", "bytes_n(1)", "bytes_n(2)", "bytes_n(3)", "bytes_n(4)", "bytes_n(16)", "bytes_n(100)"]) {
      const { values, type } = domainValues(name);
      expect(domainShape(name)).toEqual({ size: values.length, type });
    }
  });
});

describe("binder descriptors", () => {
  it("accepts a matching or omitted type", () => {
    expect(binderDomain({ domain: "bool" }).type).toBe("Bool");
    expect(binderDomain({ domain: "bytes_n(2)", type: "Bytes<2>" }).values).toHaveLength(16);
    expect(binderShape({ domain: "u8_all", type: "U8" })).toEqual({ name: "u8_all", size: 256, type: "U8" });
  });

  it("refuses a disagreeing type, extra keys and non-objects", () => {
    for (const bad of [{ domain: "bool", type: "U8" }, { domain: "bool", extra: 1 }, { type: "Bool" }, { domain: 1 }, "bool", null, [], undefined]) {
      expect(() => binderDomain(bad)).toThrow(CorpusError);
      expect(() => binderShape(bad)).toThrow(CorpusError);
    }
  });
});

describe("product", () => {
  it("visits the Cartesian product with the last binder fastest", () => {
    const seen: string[] = [];
    product([[1, 2], ["a", "b", "c"]], (values) => {
      seen.push(values.join(""));
      return true;
    });
    expect(seen).toEqual(["1a", "1b", "1c", "2a", "2b", "2c"]);
  });

  it("stops early, handles one binder, and visits nothing for an empty domain", () => {
    const seen: unknown[] = [];
    product([[1, 2, 3]], (values) => {
      seen.push(values[0]);
      return values[0] !== 2;
    });
    expect(seen).toEqual([1, 2]);
    let visits = 0;
    product([[1], []], () => {
      visits += 1;
      return true;
    });
    expect(visits).toBe(0);
    product([[1], [2]], () => {
      visits += 1;
      return true;
    });
    expect(visits).toBe(1);
  });
});
