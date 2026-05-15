/**
 * forme-types — utility type tests
 *
 * The utility types are erased at runtime, so the tests here are
 * compile-time shape checks: build values that must satisfy the type,
 * and a tiny runtime sanity check that the test file actually executed.
 */

import { describe, it, expect } from "vitest";
import type { JsonValue, ReadonlyRecord } from "../src/index.js";

describe("JsonValue", () => {
  it("accepts every JSON-serialisable form", () => {
    const samples: JsonValue[] = [
      null,
      true,
      false,
      0,
      -1.5,
      "hello",
      "",
      [],
      [1, "two", null, [3, 4], { a: 1 }],
      {},
      { name: "Adhithya", age: 1, tags: ["dev"], nested: { ok: true } },
    ];
    expect(samples.length).toBe(11);
  });

  it("rejects undefined at compile time", () => {
    // @ts-expect-error — undefined is not a valid JsonValue.
    const bad: JsonValue = undefined;
    expect(bad).toBeUndefined();
  });

  it("rejects functions at compile time", () => {
    // @ts-expect-error — functions are not JSON-serialisable.
    const bad: JsonValue = () => 1;
    expect(typeof bad).toBe("function");
  });
});

describe("ReadonlyRecord", () => {
  it("forbids assignment at the type level", () => {
    const r: ReadonlyRecord<string, number> = { a: 1, b: 2 };
    // @ts-expect-error — properties are readonly at the TypeScript level.
    // (Without Object.freeze the assignment still mutates at runtime — this
    // is a compile-time guarantee only.  The @ts-expect-error directive
    // above IS the real assertion; if the type ever became mutable, the
    // expect-error itself would error out.)
    r.a = 99;
    expect(r.a).toBe(99);
  });

  it("becomes runtime-immutable when explicitly frozen", () => {
    const r = Object.freeze({ a: 1, b: 2 } as ReadonlyRecord<string, number>);
    expect(() => {
      // Cast through `unknown` so the compile-time check doesn't shadow
      // the runtime test we're trying to make.
      (r as unknown as { a: number }).a = 99;
    }).toThrow(TypeError);
    expect(r.a).toBe(1);
  });

  it("preserves narrow key unions", () => {
    type ColourKey = "red" | "green" | "blue";
    const palette: ReadonlyRecord<ColourKey, string> = {
      red:   "#ff0000",
      green: "#00ff00",
      blue:  "#0000ff",
    };
    expect(palette.red).toBe("#ff0000");
  });
});
