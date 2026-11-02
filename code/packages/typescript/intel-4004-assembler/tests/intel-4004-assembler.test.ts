import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { AssemblerError, assemble, lexLine } from "../src/index.js";

describe("intel-4004-assembler", () => {
  it("lexes a label and operands", () => {
    const parsed = lexLine("loop: JCN 0x4, done ; comment");
    assert.equal(parsed.label, "loop");
    assert.equal(parsed.mnemonic, "JCN");
    assert.deepEqual(parsed.operands, ["0x4", "done"]);
    assert.equal(parsed.source, "loop: JCN 0x4, done ; comment");
  });

  it("assembles a small program", () => {
    const binary = assemble(`
      ORG 0x000
    _start:
      LDM 5
      XCH R2
      HLT
    `);
    assert.deepEqual(Array.from(binary), [0xd5, 0xb2, 0x01]);
  });

  it("resolves labels in jumps", () => {
    const binary = assemble(`
      ORG 0x000
      JUN done
    done:
      HLT
    `);
    assert.deepEqual(Array.from(binary), [0x40, 0x02, 0x01]);
  });

  it("fails on undefined labels", () => {
    assert.throws(() => assemble("JUN missing"), AssemblerError);
  });
});
