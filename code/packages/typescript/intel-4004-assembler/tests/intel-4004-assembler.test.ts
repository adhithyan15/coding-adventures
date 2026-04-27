import { describe, expect, it } from "vitest";

import { AssemblerError, assemble, lexLine } from "../src/index.js";

describe("intel-4004-assembler", () => {
  it("lexes a label and operands", () => {
    const parsed = lexLine("loop: JCN 0x4, done ; comment");
    expect(parsed.label).toBe("loop");
    expect(parsed.mnemonic).toBe("JCN");
    expect(parsed.operands).toEqual(["0x4", "done"]);
    expect(parsed.source).toBe("loop: JCN 0x4, done ; comment");
  });

  it("assembles a small program", () => {
    const binary = assemble(`
      ORG 0x000
    _start:
      LDM 5
      XCH R2
      HLT
    `);
    expect(Array.from(binary)).toEqual([0xd5, 0xb2, 0x01]);
  });

  it("resolves labels in jumps", () => {
    const binary = assemble(`
      ORG 0x000
      JUN done
    done:
      HLT
    `);
    expect(Array.from(binary)).toEqual([0x40, 0x02, 0x01]);
  });

  it("fails on undefined labels", () => {
    expect(() => assemble("JUN missing")).toThrow(AssemblerError);
  });
});
