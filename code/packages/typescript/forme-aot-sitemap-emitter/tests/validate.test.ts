/**
 * validate.test.ts — changefreq allowlist + priority clamping.
 */

import { describe, it, expect } from "vitest";
import { validateChangefreq, clampPriority } from "../src/index.js";

describe("validateChangefreq — accepts allowlist", () => {
  const allowed = ["always", "hourly", "daily", "weekly", "monthly", "yearly", "never"];
  for (const v of allowed) {
    it(`'${v}' accepted`, () => {
      expect(validateChangefreq(v)).toBe(v);
    });
  }

  it("case-insensitive: 'Daily' → 'daily'", () => {
    expect(validateChangefreq("Daily")).toBe("daily");
  });

  it("case-insensitive: 'DAILY' → 'daily'", () => {
    expect(validateChangefreq("DAILY")).toBe("daily");
  });
});

describe("validateChangefreq — rejects", () => {
  it("rejects 'often' (not in allowlist)", () => {
    expect(() => validateChangefreq("often"))
      .toThrow(/one of \[always, hourly, daily, weekly, monthly, yearly, never\]/);
  });

  it("rejects empty string", () => {
    expect(() => validateChangefreq("")).toThrow(/one of/);
  });

  it("rejects non-string", () => {
    // @ts-expect-error
    expect(() => validateChangefreq(42)).toThrow(/string/);
  });

  it("rejects null", () => {
    // @ts-expect-error
    expect(() => validateChangefreq(null)).toThrow(/string/);
  });

  it("error message includes the bad value", () => {
    try {
      validateChangefreq("often");
      expect.fail("expected throw");
    } catch (e) {
      expect((e as Error).message).toContain('"often"');
    }
  });
});

describe("clampPriority — clamps to [0.0, 1.0]", () => {
  it("0 → '0.0'", () => expect(clampPriority(0)).toBe("0.0"));
  it("0.5 → '0.5'", () => expect(clampPriority(0.5)).toBe("0.5"));
  it("1 → '1.0'", () => expect(clampPriority(1)).toBe("1.0"));
  it("0.75 → '0.8' (rounded)", () => expect(clampPriority(0.75)).toBe("0.8"));
  it("0.25 → '0.3' (rounded)", () => expect(clampPriority(0.25)).toBe("0.3"));
  it("0.01 → '0.0'", () => expect(clampPriority(0.01)).toBe("0.0"));
});

describe("clampPriority — out-of-range clamps", () => {
  it("negative clamps to '0.0'", () => expect(clampPriority(-0.5)).toBe("0.0"));
  it("very negative clamps to '0.0'", () => expect(clampPriority(-100)).toBe("0.0"));
  it("above 1 clamps to '1.0'", () => expect(clampPriority(1.5)).toBe("1.0"));
  it("very large clamps to '1.0'", () => expect(clampPriority(1e9)).toBe("1.0"));
  it("-Infinity → '0.0'", () => expect(clampPriority(-Infinity)).toBe("0.0"));
  it("+Infinity → '1.0'", () => expect(clampPriority(Infinity)).toBe("1.0"));
});

describe("clampPriority — defensive defaults", () => {
  it("NaN → '0.5' (spec default)", () => expect(clampPriority(NaN)).toBe("0.5"));
  it("non-number → '0.5'", () => {
    // @ts-expect-error
    expect(clampPriority("nope")).toBe("0.5");
  });
});

describe("clampPriority — output always single-decimal", () => {
  it("integer 0 still emits '0.0'", () => {
    expect(clampPriority(0)).toMatch(/^[0-1]\.\d$/);
  });

  it("output always matches /^[0-1]\\.\\d$/", () => {
    for (const v of [0, 0.1, 0.5, 0.9, 1, -1, 2, NaN]) {
      expect(clampPriority(v)).toMatch(/^[0-1]\.\d$/);
    }
  });
});
