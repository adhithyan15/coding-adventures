import { describe, expect, it } from "vitest";
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
} from "../src/index";

describe("Starlark opcodes", () => {
  it("keeps Rust-compatible byte values", () => {
    expect(Op.LoadConst).toBe(0x01);
    expect(Op.Add).toBe(0x20);
    expect(Op.CmpEq).toBe(0x30);
    expect(Op.Jump).toBe(0x40);
    expect(Op.MakeFunction).toBe(0x50);
    expect(Op.BuildList).toBe(0x60);
    expect(Op.LoadSubscript).toBe(0x70);
    expect(Op.GetIter).toBe(0x80);
    expect(Op.LoadModule).toBe(0x90);
    expect(Op.Print).toBe(0xa0);
    expect(Op.Halt).toBe(0xff);
  });

  it("round trips every defined opcode", () => {
    for (const op of ALL_OPS) {
      expect(opFromByte(opByte(op))).toBe(op);
    }
  });

  it("rejects invalid bytes", () => {
    expect(opFromByte(0xee)).toBeUndefined();
    expect(opFromByte(-1)).toBeUndefined();
    expect(opFromByte(256)).toBeUndefined();
    expect(opFromByte(1.5)).toBeUndefined();
  });

  it("classifies opcodes by high nibble", () => {
    expect(opCategory(Op.LoadConst)).toBe("stack");
    expect(opCategory(Op.LoadName)).toBe("variable");
    expect(opCategory(Op.RShift)).toBe("arithmetic");
    expect(opCategory(Op.Not)).toBe("comparison");
    expect(opCategory(Op.JumpIfTrue)).toBe("controlFlow");
    expect(opCategory(Op.Return)).toBe("function");
    expect(opCategory(Op.DictSet)).toBe("collection");
    expect(opCategory(Op.LoadSlice)).toBe("subscriptAttribute");
    expect(opCategory(Op.UnpackSequence)).toBe("iteration");
    expect(opCategory(Op.ImportFrom)).toBe("module");
    expect(opCategory(Op.Print)).toBe("io");
    expect(opCategory(Op.Halt)).toBe("vmControl");
    expect(opCategory(0x07 as Op)).toBeUndefined();
    expect(opCategory(0xb0 as Op)).toBeUndefined();
  });
});

describe("operator maps", () => {
  it("maps binary operators", () => {
    const map = binaryOpMap();
    expect(map.get("+")).toBe(Op.Add);
    expect(map.get("-")).toBe(Op.Sub);
    expect(map.get("*")).toBe(Op.Mul);
    expect(map.get("/")).toBe(Op.Div);
    expect(map.get("//")).toBe(Op.FloorDiv);
    expect(map.get("%")).toBe(Op.Mod);
    expect(map.get("**")).toBe(Op.Power);
    expect(map.get("&")).toBe(Op.BitAnd);
    expect(map.get("|")).toBe(Op.BitOr);
    expect(map.get("^")).toBe(Op.BitXor);
    expect(map.get("<<")).toBe(Op.LShift);
    expect(map.get(">>")).toBe(Op.RShift);
    expect(binaryOpcode("+")).toBe(Op.Add);
    expect(binaryOpcode("???")).toBeUndefined();
    expect(map.size).toBe(12);
  });

  it("maps comparison operators", () => {
    const map = compareOpMap();
    expect(map.get("==")).toBe(Op.CmpEq);
    expect(map.get("!=")).toBe(Op.CmpNe);
    expect(map.get("<")).toBe(Op.CmpLt);
    expect(map.get(">")).toBe(Op.CmpGt);
    expect(map.get("<=")).toBe(Op.CmpLe);
    expect(map.get(">=")).toBe(Op.CmpGe);
    expect(map.get("in")).toBe(Op.CmpIn);
    expect(map.get("not in")).toBe(Op.CmpNotIn);
    expect(compareOpcode("in")).toBe(Op.CmpIn);
    expect(compareOpcode("contains")).toBeUndefined();
    expect(map.size).toBe(8);
  });

  it("maps augmented assignments to arithmetic opcodes", () => {
    const map = augmentedAssignMap();
    expect(map.get("+=")).toBe(Op.Add);
    expect(map.get("-=")).toBe(Op.Sub);
    expect(map.get("*=")).toBe(Op.Mul);
    expect(map.get("/=")).toBe(Op.Div);
    expect(map.get("//=")).toBe(Op.FloorDiv);
    expect(map.get("%=")).toBe(Op.Mod);
    expect(map.get("&=")).toBe(Op.BitAnd);
    expect(map.get("|=")).toBe(Op.BitOr);
    expect(map.get("^=")).toBe(Op.BitXor);
    expect(map.get("<<=")).toBe(Op.LShift);
    expect(map.get(">>=")).toBe(Op.RShift);
    expect(map.get("**=")).toBe(Op.Power);
    expect(augmentedAssignOpcode("**=")).toBe(Op.Power);
    expect(augmentedAssignOpcode("=")).toBeUndefined();
    expect(map.size).toBe(12);
  });

  it("maps unary operators", () => {
    const map = unaryOpMap();
    expect(map.get("-")).toBe(Op.Negate);
    expect(map.get("~")).toBe(Op.BitNot);
    expect(unaryOpcode("~")).toBe(Op.BitNot);
    expect(unaryOpcode("not")).toBeUndefined();
    expect(map.size).toBe(2);
  });

  it("returns mutable map copies", () => {
    const map = binaryOpMap();
    map.set("custom", Op.Halt);
    expect(binaryOpMap().has("custom")).toBe(false);
  });
});
