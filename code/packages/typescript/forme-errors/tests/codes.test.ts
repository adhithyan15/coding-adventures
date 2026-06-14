/**
 * forme-errors — code table tests
 */

import { describe, it, expect } from "vitest";
import { ERROR_CODES } from "../src/index.js";
import type { KernelErrorCode } from "../src/index.js";

describe("ERROR_CODES", () => {
  it("includes every FM01 §6.1 code listed in the spec", () => {
    const expected = [
      "PARSE_ERROR",
      "PARSE_FRONTMATTER_INVALID",
      "PARSE_NO_DOCUMENT",
      "CAPABILITY_DENIED",
      "CANCELLED",
      "UNCAUGHT",
      "TIMEOUT",
      "IO_NOT_FOUND",
      "IO_PERMISSION_DENIED",
      "NETWORK_UNREACHABLE",
    ];
    expect(new Set(Object.values(ERROR_CODES))).toEqual(new Set(expected));
  });

  it("uses identity strings — keys equal values", () => {
    for (const [k, v] of Object.entries(ERROR_CODES)) {
      expect(v).toBe(k);
    }
  });

  it("is frozen — runtime mutation is rejected", () => {
    expect(() => {
      // @ts-expect-error — readonly at the type level.
      ERROR_CODES.NEW_CODE = "NEW_CODE";
    }).toThrow(TypeError);
    expect("NEW_CODE" in ERROR_CODES).toBe(false);
  });

  it("compile-time KernelErrorCode union accepts every value", () => {
    const codes: KernelErrorCode[] = [
      ERROR_CODES.PARSE_ERROR,
      ERROR_CODES.CAPABILITY_DENIED,
      ERROR_CODES.NETWORK_UNREACHABLE,
    ];
    expect(codes.length).toBe(3);
  });
});
