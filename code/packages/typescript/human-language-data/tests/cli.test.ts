import { describe, it, expect, vi } from "vitest";
import { runValidate } from "../src/cli.js";

describe("runValidate", () => {
  it("returns 0 (no errors) on the real curriculum and prints a report", () => {
    const out = vi.spyOn(process.stdout, "write").mockImplementation(() => true);
    const code = runValidate();
    const printed = out.mock.calls.map((c) => String(c[0])).join("");
    out.mockRestore();

    expect(code).toBe(0);
    expect(printed).toMatch(/concepts, \d+ languages/);
    expect(printed).toMatch(/error\(s\)/);
    expect(printed).toContain("spanish");
  });
});
