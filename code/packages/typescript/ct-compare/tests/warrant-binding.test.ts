/**
 * The Warrant binding table resolves against the real exports.
 *
 * The whole corpus run lives in `BUILD` (`tests/warrant-evidence.ts check`);
 * this test only proves, inside Vitest, that `evidence.bindings` is well-formed
 * and names real, callable exports, so a rename here fails fast.
 */

import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { parseBinding, resolveExports } from "@coding-adventures/warrant-corpus";

const PACKAGE = resolve(dirname(fileURLToPath(import.meta.url)), "..");

describe("evidence.bindings", () => {
  it("names the real exports", async () => {
    const binding = parseBinding(new Uint8Array(readFileSync(join(PACKAGE, "evidence.bindings"))));
    expect(binding.package).toBe("typescript/ct-compare");
    const exports = await resolveExports(binding, PACKAGE);
    expect([...exports.keys()].sort()).toEqual(["eq", "eq_fixed", "eq_u64", "select_bytes"]);
    const eq = exports.get("eq") as (a: Uint8Array, b: Uint8Array) => boolean;
    expect(eq(Uint8Array.of(1), Uint8Array.of(1))).toBe(true);
    expect(eq(Uint8Array.of(1), Uint8Array.of(2))).toBe(false);
    const eqU64 = exports.get("eq_u64") as (a: bigint, b: bigint) => boolean;
    expect(eqU64(2n ** 64n - 1n, 2n ** 64n - 2n)).toBe(false);
  });

  it("maps the length mismatch to the one declared error class", async () => {
    const binding = parseBinding(new Uint8Array(readFileSync(join(PACKAGE, "evidence.bindings"))));
    expect(binding.ops.get("select_bytes")?.errors).toEqual([["CT_SELECT_LENGTH_MISMATCH", RangeError]]);
    const exports = await resolveExports(binding, PACKAGE);
    const select = exports.get("select_bytes") as (a: Uint8Array, b: Uint8Array, c: boolean) => Uint8Array;
    expect(() => select(Uint8Array.of(1), Uint8Array.of(1, 2), true)).toThrow(RangeError);
  });
});
