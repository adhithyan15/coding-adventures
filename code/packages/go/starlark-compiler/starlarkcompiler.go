package starlarkcompiler

type Op byte

const (
	LoadConst Op = 0x01
	Pop       Op = 0x02
	Dup       Op = 0x03
	LoadNone  Op = 0x04
	LoadTrue  Op = 0x05
	LoadFalse Op = 0x06

	StoreName    Op = 0x10
	LoadName     Op = 0x11
	StoreLocal   Op = 0x12
	LoadLocal    Op = 0x13
	StoreClosure Op = 0x14
	LoadClosure  Op = 0x15

	Add      Op = 0x20
	Sub      Op = 0x21
	Mul      Op = 0x22
	Div      Op = 0x23
	FloorDiv Op = 0x24
	Mod      Op = 0x25
	Power    Op = 0x26
	Negate   Op = 0x27
	BitAnd   Op = 0x28
	BitOr    Op = 0x29
	BitXor   Op = 0x2A
	BitNot   Op = 0x2B
	LShift   Op = 0x2C
	RShift   Op = 0x2D

	CmpEq    Op = 0x30
	CmpNe    Op = 0x31
	CmpLt    Op = 0x32
	CmpGt    Op = 0x33
	CmpLe    Op = 0x34
	CmpGe    Op = 0x35
	CmpIn    Op = 0x36
	CmpNotIn Op = 0x37
	Not      Op = 0x38

	Jump             Op = 0x40
	JumpIfFalse      Op = 0x41
	JumpIfTrue       Op = 0x42
	JumpIfFalseOrPop Op = 0x43
	JumpIfTrueOrPop  Op = 0x44

	MakeFunction   Op = 0x50
	CallFunction   Op = 0x51
	CallFunctionKw Op = 0x52
	Return         Op = 0x53

	BuildList  Op = 0x60
	BuildDict  Op = 0x61
	BuildTuple Op = 0x62
	ListAppend Op = 0x63
	DictSet    Op = 0x64

	LoadSubscript  Op = 0x70
	StoreSubscript Op = 0x71
	LoadAttr       Op = 0x72
	StoreAttr      Op = 0x73
	LoadSlice      Op = 0x74

	GetIter        Op = 0x80
	ForIter        Op = 0x81
	UnpackSequence Op = 0x82

	LoadModule Op = 0x90
	ImportFrom Op = 0x91

	Print Op = 0xA0

	Halt Op = 0xFF
)

type Category string

const (
	CategoryStack              Category = "stack"
	CategoryVariable           Category = "variable"
	CategoryArithmetic         Category = "arithmetic"
	CategoryComparison         Category = "comparison"
	CategoryControlFlow        Category = "control_flow"
	CategoryFunction           Category = "function"
	CategoryCollection         Category = "collection"
	CategorySubscriptAttribute Category = "subscript_attribute"
	CategoryIteration          Category = "iteration"
	CategoryModule             Category = "module"
	CategoryIO                 Category = "io"
	CategoryVMControl          Category = "vm_control"
)

var allOps = []Op{
	LoadConst, Pop, Dup, LoadNone, LoadTrue, LoadFalse,
	StoreName, LoadName, StoreLocal, LoadLocal, StoreClosure, LoadClosure,
	Add, Sub, Mul, Div, FloorDiv, Mod, Power, Negate, BitAnd, BitOr, BitXor, BitNot, LShift, RShift,
	CmpEq, CmpNe, CmpLt, CmpGt, CmpLe, CmpGe, CmpIn, CmpNotIn, Not,
	Jump, JumpIfFalse, JumpIfTrue, JumpIfFalseOrPop, JumpIfTrueOrPop,
	MakeFunction, CallFunction, CallFunctionKw, Return,
	BuildList, BuildDict, BuildTuple, ListAppend, DictSet,
	LoadSubscript, StoreSubscript, LoadAttr, StoreAttr, LoadSlice,
	GetIter, ForIter, UnpackSequence,
	LoadModule, ImportFrom,
	Print,
	Halt,
}

var opNames = map[Op]string{
	LoadConst: "LoadConst", Pop: "Pop", Dup: "Dup", LoadNone: "LoadNone", LoadTrue: "LoadTrue", LoadFalse: "LoadFalse",
	StoreName: "StoreName", LoadName: "LoadName", StoreLocal: "StoreLocal", LoadLocal: "LoadLocal", StoreClosure: "StoreClosure", LoadClosure: "LoadClosure",
	Add: "Add", Sub: "Sub", Mul: "Mul", Div: "Div", FloorDiv: "FloorDiv", Mod: "Mod", Power: "Power", Negate: "Negate", BitAnd: "BitAnd", BitOr: "BitOr", BitXor: "BitXor", BitNot: "BitNot", LShift: "LShift", RShift: "RShift",
	CmpEq: "CmpEq", CmpNe: "CmpNe", CmpLt: "CmpLt", CmpGt: "CmpGt", CmpLe: "CmpLe", CmpGe: "CmpGe", CmpIn: "CmpIn", CmpNotIn: "CmpNotIn", Not: "Not",
	Jump: "Jump", JumpIfFalse: "JumpIfFalse", JumpIfTrue: "JumpIfTrue", JumpIfFalseOrPop: "JumpIfFalseOrPop", JumpIfTrueOrPop: "JumpIfTrueOrPop",
	MakeFunction: "MakeFunction", CallFunction: "CallFunction", CallFunctionKw: "CallFunctionKw", Return: "Return",
	BuildList: "BuildList", BuildDict: "BuildDict", BuildTuple: "BuildTuple", ListAppend: "ListAppend", DictSet: "DictSet",
	LoadSubscript: "LoadSubscript", StoreSubscript: "StoreSubscript", LoadAttr: "LoadAttr", StoreAttr: "StoreAttr", LoadSlice: "LoadSlice",
	GetIter: "GetIter", ForIter: "ForIter", UnpackSequence: "UnpackSequence",
	LoadModule: "LoadModule", ImportFrom: "ImportFrom",
	Print: "Print",
	Halt:  "Halt",
}

func AllOps() []Op {
	ops := make([]Op, len(allOps))
	copy(ops, allOps)
	return ops
}

func OpFromByte(value byte) (Op, bool) {
	switch value {
	case 0x01:
		return LoadConst, true
	case 0x02:
		return Pop, true
	case 0x03:
		return Dup, true
	case 0x04:
		return LoadNone, true
	case 0x05:
		return LoadTrue, true
	case 0x06:
		return LoadFalse, true
	case 0x10:
		return StoreName, true
	case 0x11:
		return LoadName, true
	case 0x12:
		return StoreLocal, true
	case 0x13:
		return LoadLocal, true
	case 0x14:
		return StoreClosure, true
	case 0x15:
		return LoadClosure, true
	case 0x20:
		return Add, true
	case 0x21:
		return Sub, true
	case 0x22:
		return Mul, true
	case 0x23:
		return Div, true
	case 0x24:
		return FloorDiv, true
	case 0x25:
		return Mod, true
	case 0x26:
		return Power, true
	case 0x27:
		return Negate, true
	case 0x28:
		return BitAnd, true
	case 0x29:
		return BitOr, true
	case 0x2A:
		return BitXor, true
	case 0x2B:
		return BitNot, true
	case 0x2C:
		return LShift, true
	case 0x2D:
		return RShift, true
	case 0x30:
		return CmpEq, true
	case 0x31:
		return CmpNe, true
	case 0x32:
		return CmpLt, true
	case 0x33:
		return CmpGt, true
	case 0x34:
		return CmpLe, true
	case 0x35:
		return CmpGe, true
	case 0x36:
		return CmpIn, true
	case 0x37:
		return CmpNotIn, true
	case 0x38:
		return Not, true
	case 0x40:
		return Jump, true
	case 0x41:
		return JumpIfFalse, true
	case 0x42:
		return JumpIfTrue, true
	case 0x43:
		return JumpIfFalseOrPop, true
	case 0x44:
		return JumpIfTrueOrPop, true
	case 0x50:
		return MakeFunction, true
	case 0x51:
		return CallFunction, true
	case 0x52:
		return CallFunctionKw, true
	case 0x53:
		return Return, true
	case 0x60:
		return BuildList, true
	case 0x61:
		return BuildDict, true
	case 0x62:
		return BuildTuple, true
	case 0x63:
		return ListAppend, true
	case 0x64:
		return DictSet, true
	case 0x70:
		return LoadSubscript, true
	case 0x71:
		return StoreSubscript, true
	case 0x72:
		return LoadAttr, true
	case 0x73:
		return StoreAttr, true
	case 0x74:
		return LoadSlice, true
	case 0x80:
		return GetIter, true
	case 0x81:
		return ForIter, true
	case 0x82:
		return UnpackSequence, true
	case 0x90:
		return LoadModule, true
	case 0x91:
		return ImportFrom, true
	case 0xA0:
		return Print, true
	case 0xFF:
		return Halt, true
	default:
		return 0, false
	}
}

func (op Op) Byte() byte {
	return byte(op)
}

func (op Op) Category() (Category, bool) {
	if _, ok := OpFromByte(byte(op)); !ok {
		return "", false
	}

	switch (byte(op) >> 4) & 0x0F {
	case 0x0:
		return CategoryStack, true
	case 0x1:
		return CategoryVariable, true
	case 0x2:
		return CategoryArithmetic, true
	case 0x3:
		return CategoryComparison, true
	case 0x4:
		return CategoryControlFlow, true
	case 0x5:
		return CategoryFunction, true
	case 0x6:
		return CategoryCollection, true
	case 0x7:
		return CategorySubscriptAttribute, true
	case 0x8:
		return CategoryIteration, true
	case 0x9:
		return CategoryModule, true
	case 0xA:
		return CategoryIO, true
	case 0xF:
		return CategoryVMControl, true
	default:
		return "", false
	}
}

func (op Op) String() string {
	if name, ok := opNames[op]; ok {
		return name
	}
	return "Op(unknown)"
}

func BinaryOpMap() map[string]Op {
	return map[string]Op{
		"+": Add, "-": Sub, "*": Mul, "/": Div, "//": FloorDiv, "%": Mod,
		"**": Power, "&": BitAnd, "|": BitOr, "^": BitXor, "<<": LShift, ">>": RShift,
	}
}

func CompareOpMap() map[string]Op {
	return map[string]Op{
		"==": CmpEq, "!=": CmpNe, "<": CmpLt, ">": CmpGt,
		"<=": CmpLe, ">=": CmpGe, "in": CmpIn, "not in": CmpNotIn,
	}
}

func AugmentedAssignMap() map[string]Op {
	return map[string]Op{
		"+=": Add, "-=": Sub, "*=": Mul, "/=": Div, "//=": FloorDiv, "%=": Mod,
		"&=": BitAnd, "|=": BitOr, "^=": BitXor, "<<=": LShift, ">>=": RShift, "**=": Power,
	}
}

func UnaryOpMap() map[string]Op {
	return map[string]Op{
		"-": Negate,
		"~": BitNot,
	}
}

func BinaryOpcode(operator string) (Op, bool) {
	op, ok := BinaryOpMap()[operator]
	return op, ok
}

func CompareOpcode(operator string) (Op, bool) {
	op, ok := CompareOpMap()[operator]
	return op, ok
}

func AugmentedAssignOpcode(operator string) (Op, bool) {
	op, ok := AugmentedAssignMap()[operator]
	return op, ok
}

func UnaryOpcode(operator string) (Op, bool) {
	op, ok := UnaryOpMap()[operator]
	return op, ok
}
