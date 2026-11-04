import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  ALL_OPS,
  Op,
  augmentedAssignMap,
  augmentedAssignOpcode,
  binaryOpMap,
  binaryOpcode,
  compareOpMap,
  compareOpcode,
  opByte,
  opCategory,
  opFromByte,
  unaryOpMap,
  unaryOpcode,
} from "../src/index.js";

describe("Starlark opcodes", () => {
  it("keeps Rust-compatible byte values", () => {
    assert.equal(Op.LoadConst, 0x01);
    assert.equal(Op.Add, 0x20);
    assert.equal(Op.CmpEq, 0x30);
    assert.equal(Op.Jump, 0x40);
    assert.equal(Op.MakeFunction, 0x50);
    assert.equal(Op.BuildList, 0x60);
    assert.equal(Op.LoadSubscript, 0x70);
    assert.equal(Op.GetIter, 0x80);
    assert.equal(Op.LoadModule, 0x90);
    assert.equal(Op.Print, 0xa0);
    assert.equal(Op.Halt, 0xff);
  });

  it("round trips every defined opcode", () => {
    for (const op of ALL_OPS) {
      assert.equal(opFromByte(opByte(op)), op);
    }
  });

  it("rejects invalid bytes", () => {
    assert.equal(opFromByte(0xee), undefined);
    assert.equal(opFromByte(-1), undefined);
    assert.equal(opFromByte(256), undefined);
    assert.equal(opFromByte(1.5), undefined);
  });

  it("classifies opcodes by high nibble", () => {
    assert.equal(opCategory(Op.LoadConst), "stack");
    assert.equal(opCategory(Op.LoadName), "variable");
    assert.equal(opCategory(Op.RShift), "arithmetic");
    assert.equal(opCategory(Op.Not), "comparison");
    assert.equal(opCategory(Op.JumpIfTrue), "controlFlow");
    assert.equal(opCategory(Op.Return), "function");
    assert.equal(opCategory(Op.DictSet), "collection");
    assert.equal(opCategory(Op.LoadSlice), "subscriptAttribute");
    assert.equal(opCategory(Op.UnpackSequence), "iteration");
    assert.equal(opCategory(Op.ImportFrom), "module");
    assert.equal(opCategory(Op.Print), "io");
    assert.equal(opCategory(Op.Halt), "vmControl");
    assert.equal(opCategory(0x07 as Op), undefined);
    assert.equal(opCategory(0xb0 as Op), undefined);
  });
});

describe("operator maps", () => {
  it("maps binary operators", () => {
    const map = binaryOpMap();
    assert.equal(map.get("+"), Op.Add);
    assert.equal(map.get("-"), Op.Sub);
    assert.equal(map.get("*"), Op.Mul);
    assert.equal(map.get("/"), Op.Div);
    assert.equal(map.get("//"), Op.FloorDiv);
    assert.equal(map.get("%"), Op.Mod);
    assert.equal(map.get("**"), Op.Power);
    assert.equal(map.get("&"), Op.BitAnd);
    assert.equal(map.get("|"), Op.BitOr);
    assert.equal(map.get("^"), Op.BitXor);
    assert.equal(map.get("<<"), Op.LShift);
    assert.equal(map.get(">>"), Op.RShift);
    assert.equal(binaryOpcode("+"), Op.Add);
    assert.equal(binaryOpcode("???"), undefined);
    assert.equal(map.size, 12);
  });

  it("maps comparison operators", () => {
    const map = compareOpMap();
    assert.equal(map.get("=="), Op.CmpEq);
    assert.equal(map.get("!="), Op.CmpNe);
    assert.equal(map.get("<"), Op.CmpLt);
    assert.equal(map.get(">"), Op.CmpGt);
    assert.equal(map.get("<="), Op.CmpLe);
    assert.equal(map.get(">="), Op.CmpGe);
    assert.equal(map.get("in"), Op.CmpIn);
    assert.equal(map.get("not in"), Op.CmpNotIn);
    assert.equal(compareOpcode("in"), Op.CmpIn);
    assert.equal(compareOpcode("contains"), undefined);
    assert.equal(map.size, 8);
  });

  it("maps augmented assignments to arithmetic opcodes", () => {
    const map = augmentedAssignMap();
    assert.equal(map.get("+="), Op.Add);
    assert.equal(map.get("-="), Op.Sub);
    assert.equal(map.get("*="), Op.Mul);
    assert.equal(map.get("/="), Op.Div);
    assert.equal(map.get("//="), Op.FloorDiv);
    assert.equal(map.get("%="), Op.Mod);
    assert.equal(map.get("&="), Op.BitAnd);
    assert.equal(map.get("|="), Op.BitOr);
    assert.equal(map.get("^="), Op.BitXor);
    assert.equal(map.get("<<="), Op.LShift);
    assert.equal(map.get(">>="), Op.RShift);
    assert.equal(map.get("**="), Op.Power);
    assert.equal(augmentedAssignOpcode("**="), Op.Power);
    assert.equal(augmentedAssignOpcode("="), undefined);
    assert.equal(map.size, 12);
  });

  it("maps unary operators", () => {
    const map = unaryOpMap();
    assert.equal(map.get("-"), Op.Negate);
    assert.equal(map.get("~"), Op.BitNot);
    assert.equal(unaryOpcode("~"), Op.BitNot);
    assert.equal(unaryOpcode("not"), undefined);
    assert.equal(map.size, 2);
  });

  it("returns mutable map copies", () => {
    const map = binaryOpMap();
    map.set("custom", Op.Halt);
    assert.equal(binaryOpMap().has("custom"), false);
  });
});
