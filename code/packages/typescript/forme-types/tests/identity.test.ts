/**
 * forme-types — identity branded-type tests
 *
 * The branded LogicalId / RevisionId types exist only at compile time
 * (they're plain strings at runtime).  These tests verify the brand
 * actually keeps the two types apart.
 */

import { describe, it, expect } from "vitest";
import type { LogicalId, RevisionId } from "../src/index.js";

describe("LogicalId / RevisionId branding", () => {
  it("forbids assigning a plain string to a LogicalId", () => {
    // @ts-expect-error — branded types reject raw strings.
    const id: LogicalId = "01952c0d-7e63-7000-8000-000000000000";
    expect(typeof id).toBe("string");
  });

  it("forbids cross-assignment between the two ID types", () => {
    const logical = "01952c0d-7e63-7000-8000-000000000000" as LogicalId;
    // @ts-expect-error — RevisionId and LogicalId are different brands.
    const rev: RevisionId = logical;
    expect(typeof rev).toBe("string");
  });

  it("permits explicit branding via type assertion", () => {
    const logical = "01952c0d-7e63-7000-8000-000000000000" as LogicalId;
    const rev = "blake2b:cafebabe" as RevisionId;
    expect(logical).toContain("01952c0d");
    expect(rev).toContain("blake2b:");
  });
});
