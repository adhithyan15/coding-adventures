export enum Op {
  LoadConst = 0x01,
  Pop = 0x02,
  Dup = 0x03,
  LoadNone = 0x04,
  LoadTrue = 0x05,
  LoadFalse = 0x06,

  StoreName = 0x10,
  LoadName = 0x11,
  StoreLocal = 0x12,
  LoadLocal = 0x13,
  StoreClosure = 0x14,
  LoadClosure = 0x15,

  Add = 0x20,
  Sub = 0x21,
  Mul = 0x22,
  Div = 0x23,
  FloorDiv = 0x24,
  Mod = 0x25,
  Power = 0x26,
  Negate = 0x27,
  BitAnd = 0x28,
  BitOr = 0x29,
  BitXor = 0x2a,
  BitNot = 0x2b,
  LShift = 0x2c,
  RShift = 0x2d,

  CmpEq = 0x30,
  CmpNe = 0x31,
  CmpLt = 0x32,
  CmpGt = 0x33,
  CmpLe = 0x34,
  CmpGe = 0x35,
  CmpIn = 0x36,
  CmpNotIn = 0x37,
  Not = 0x38,

  Jump = 0x40,
  JumpIfFalse = 0x41,
  JumpIfTrue = 0x42,
  JumpIfFalseOrPop = 0x43,
  JumpIfTrueOrPop = 0x44,

  MakeFunction = 0x50,
  CallFunction = 0x51,
  CallFunctionKw = 0x52,
  Return = 0x53,

  BuildList = 0x60,
  BuildDict = 0x61,
  BuildTuple = 0x62,
  ListAppend = 0x63,
  DictSet = 0x64,

  LoadSubscript = 0x70,
  StoreSubscript = 0x71,
  LoadAttr = 0x72,
  StoreAttr = 0x73,
  LoadSlice = 0x74,

  GetIter = 0x80,
  ForIter = 0x81,
  UnpackSequence = 0x82,

  LoadModule = 0x90,
  ImportFrom = 0x91,

  Print = 0xa0,

  Halt = 0xff,
}

export type OpCategory =
  | "stack"
  | "variable"
  | "arithmetic"
  | "comparison"
  | "controlFlow"
  | "function"
  | "collection"
  | "subscriptAttribute"
  | "iteration"
  | "module"
  | "io"
  | "vmControl";

export const ALL_OPS: readonly Op[] = Object.freeze([
  Op.LoadConst,
  Op.Pop,
  Op.Dup,
  Op.LoadNone,
  Op.LoadTrue,
  Op.LoadFalse,
  Op.StoreName,
  Op.LoadName,
  Op.StoreLocal,
  Op.LoadLocal,
  Op.StoreClosure,
  Op.LoadClosure,
  Op.Add,
  Op.Sub,
  Op.Mul,
  Op.Div,
  Op.FloorDiv,
  Op.Mod,
  Op.Power,
  Op.Negate,
  Op.BitAnd,
  Op.BitOr,
  Op.BitXor,
  Op.BitNot,
  Op.LShift,
  Op.RShift,
  Op.CmpEq,
  Op.CmpNe,
  Op.CmpLt,
  Op.CmpGt,
  Op.CmpLe,
  Op.CmpGe,
  Op.CmpIn,
  Op.CmpNotIn,
  Op.Not,
  Op.Jump,
  Op.JumpIfFalse,
  Op.JumpIfTrue,
  Op.JumpIfFalseOrPop,
  Op.JumpIfTrueOrPop,
  Op.MakeFunction,
  Op.CallFunction,
  Op.CallFunctionKw,
  Op.Return,
  Op.BuildList,
  Op.BuildDict,
  Op.BuildTuple,
  Op.ListAppend,
  Op.DictSet,
  Op.LoadSubscript,
  Op.StoreSubscript,
  Op.LoadAttr,
  Op.StoreAttr,
  Op.LoadSlice,
  Op.GetIter,
  Op.ForIter,
  Op.UnpackSequence,
  Op.LoadModule,
  Op.ImportFrom,
  Op.Print,
  Op.Halt,
]);

const BYTE_TO_OP = new Map<number, Op>(ALL_OPS.map((op) => [op, op]));

const BINARY_OPS: readonly (readonly [string, Op])[] = Object.freeze([
  ["+", Op.Add],
  ["-", Op.Sub],
  ["*", Op.Mul],
  ["/", Op.Div],
  ["//", Op.FloorDiv],
  ["%", Op.Mod],
  ["**", Op.Power],
  ["&", Op.BitAnd],
  ["|", Op.BitOr],
  ["^", Op.BitXor],
  ["<<", Op.LShift],
  [">>", Op.RShift],
]);

const COMPARE_OPS: readonly (readonly [string, Op])[] = Object.freeze([
  ["==", Op.CmpEq],
  ["!=", Op.CmpNe],
  ["<", Op.CmpLt],
  [">", Op.CmpGt],
  ["<=", Op.CmpLe],
  [">=", Op.CmpGe],
  ["in", Op.CmpIn],
  ["not in", Op.CmpNotIn],
]);

const AUGMENTED_ASSIGN_OPS: readonly (readonly [string, Op])[] =
  Object.freeze([
    ["+=", Op.Add],
    ["-=", Op.Sub],
    ["*=", Op.Mul],
    ["/=", Op.Div],
    ["//=", Op.FloorDiv],
    ["%=", Op.Mod],
    ["&=", Op.BitAnd],
    ["|=", Op.BitOr],
    ["^=", Op.BitXor],
    ["<<=", Op.LShift],
    [">>=", Op.RShift],
    ["**=", Op.Power],
  ]);

const UNARY_OPS: readonly (readonly [string, Op])[] = Object.freeze([
  ["-", Op.Negate],
  ["~", Op.BitNot],
]);

export function opFromByte(value: number): Op | undefined {
  if (!Number.isInteger(value) || value < 0 || value > 0xff) {
    return undefined;
  }
  return BYTE_TO_OP.get(value);
}

export function opByte(op: Op): number {
  return op;
}

export function opCategory(op: Op): OpCategory | undefined {
  if (!BYTE_TO_OP.has(op)) {
    return undefined;
  }

  switch ((op >> 4) & 0x0f) {
    case 0x0:
      return "stack";
    case 0x1:
      return "variable";
    case 0x2:
      return "arithmetic";
    case 0x3:
      return "comparison";
    case 0x4:
      return "controlFlow";
    case 0x5:
      return "function";
    case 0x6:
      return "collection";
    case 0x7:
      return "subscriptAttribute";
    case 0x8:
      return "iteration";
    case 0x9:
      return "module";
    case 0xa:
      return "io";
    case 0xf:
      return "vmControl";
    default:
      return undefined;
  }
}

export function binaryOpMap(): Map<string, Op> {
  return new Map(BINARY_OPS);
}

export function compareOpMap(): Map<string, Op> {
  return new Map(COMPARE_OPS);
}

export function augmentedAssignMap(): Map<string, Op> {
  return new Map(AUGMENTED_ASSIGN_OPS);
}

export function unaryOpMap(): Map<string, Op> {
  return new Map(UNARY_OPS);
}

export function binaryOpcode(operator: string): Op | undefined {
  return binaryOpMap().get(operator);
}

export function compareOpcode(operator: string): Op | undefined {
  return compareOpMap().get(operator);
}

export function augmentedAssignOpcode(operator: string): Op | undefined {
  return augmentedAssignMap().get(operator);
}

export function unaryOpcode(operator: string): Op | undefined {
  return unaryOpMap().get(operator);
}
