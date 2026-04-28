package registervm

// Opcode identifies one bytecode operation in the register VM.
type Opcode int

const (
	// Accumulator loads.
	LdaConstant  Opcode = 0x00
	LdaZero      Opcode = 0x01
	LdaSmi       Opcode = 0x02
	LdaUndefined Opcode = 0x03
	LdaNull      Opcode = 0x04
	LdaTrue      Opcode = 0x05
	LdaFalse     Opcode = 0x06

	// Register moves.
	Ldar Opcode = 0x10
	Star Opcode = 0x11
	Mov  Opcode = 0x12

	// Variable and context access.
	LdaGlobal             Opcode = 0x20
	StaGlobal             Opcode = 0x21
	LdaLocal              Opcode = 0x22
	StaLocal              Opcode = 0x23
	LdaContextSlot        Opcode = 0x24
	StaContextSlot        Opcode = 0x25
	LdaCurrentContextSlot Opcode = 0x26
	StaCurrentContextSlot Opcode = 0x27

	// Arithmetic and bitwise operations.
	Add               Opcode = 0x30
	Sub               Opcode = 0x31
	Mul               Opcode = 0x32
	Div               Opcode = 0x33
	Mod               Opcode = 0x34
	Pow               Opcode = 0x35
	AddSmi            Opcode = 0x36
	SubSmi            Opcode = 0x37
	BitwiseAnd        Opcode = 0x38
	BitwiseOr         Opcode = 0x39
	BitwiseXor        Opcode = 0x3A
	BitwiseNot        Opcode = 0x3B
	ShiftLeft         Opcode = 0x3C
	ShiftRight        Opcode = 0x3D
	ShiftRightLogical Opcode = 0x3E
	Negate            Opcode = 0x3F

	// Comparisons and type tests.
	TestEqual              Opcode = 0x40
	TestNotEqual           Opcode = 0x41
	TestStrictEqual        Opcode = 0x42
	TestStrictNotEqual     Opcode = 0x43
	TestLessThan           Opcode = 0x44
	TestGreaterThan        Opcode = 0x45
	TestLessThanOrEqual    Opcode = 0x46
	TestGreaterThanOrEqual Opcode = 0x47
	TestIn                 Opcode = 0x48
	TestInstanceof         Opcode = 0x49
	TestUndetectable       Opcode = 0x4A
	LogicalNot             Opcode = 0x4B
	Typeof                 Opcode = 0x4C

	// Control flow.
	Jump                  Opcode = 0x50
	JumpIfTrue            Opcode = 0x51
	JumpIfFalse           Opcode = 0x52
	JumpIfNull            Opcode = 0x53
	JumpIfUndefined       Opcode = 0x54
	JumpIfNullOrUndefined Opcode = 0x55
	JumpIfToBooleanTrue   Opcode = 0x56
	JumpIfToBooleanFalse  Opcode = 0x57
	JumpLoop              Opcode = 0x58

	// Calls and function control.
	CallAnyReceiver       Opcode = 0x60
	CallProperty          Opcode = 0x61
	CallUndefinedReceiver Opcode = 0x62
	Construct             Opcode = 0x63
	ConstructWithSpread   Opcode = 0x64
	CallWithSpread        Opcode = 0x65
	Return                Opcode = 0x66
	SuspendGenerator      Opcode = 0x67
	ResumeGenerator       Opcode = 0x68

	// Property access.
	LdaNamedProperty           Opcode = 0x70
	StaNamedProperty           Opcode = 0x71
	LdaKeyedProperty           Opcode = 0x72
	StaKeyedProperty           Opcode = 0x73
	LdaNamedPropertyNoFeedback Opcode = 0x74
	StaNamedPropertyNoFeedback Opcode = 0x75
	DeletePropertyStrict       Opcode = 0x76
	DeletePropertySloppy       Opcode = 0x77

	// Object and closure creation.
	CreateObjectLiteral Opcode = 0x80
	CreateArrayLiteral  Opcode = 0x81
	CreateRegexpLiteral Opcode = 0x82
	CreateClosure       Opcode = 0x83
	CreateContext       Opcode = 0x84
	CloneObject         Opcode = 0x85

	// Iteration.
	GetIterator      Opcode = 0x90
	CallIteratorStep Opcode = 0x91
	GetIteratorDone  Opcode = 0x92
	GetIteratorValue Opcode = 0x93

	// Exceptions.
	Throw   Opcode = 0xA0
	Rethrow Opcode = 0xA1

	// Context and module variable access.
	PushContext       Opcode = 0xB0
	PopContext        Opcode = 0xB1
	LdaModuleVariable Opcode = 0xB4
	StaModuleVariable Opcode = 0xB5

	// VM meta instructions.
	StackCheck Opcode = 0xF0
	Debugger   Opcode = 0xF1
	Halt       Opcode = 0xFF
)

var opcodeNames = map[Opcode]string{
	LdaConstant: "LDA_CONSTANT", LdaZero: "LDA_ZERO", LdaSmi: "LDA_SMI", LdaUndefined: "LDA_UNDEFINED",
	LdaNull: "LDA_NULL", LdaTrue: "LDA_TRUE", LdaFalse: "LDA_FALSE", Ldar: "LDAR", Star: "STAR", Mov: "MOV",
	LdaGlobal: "LDA_GLOBAL", StaGlobal: "STA_GLOBAL", LdaLocal: "LDA_LOCAL", StaLocal: "STA_LOCAL",
	LdaContextSlot: "LDA_CONTEXT_SLOT", StaContextSlot: "STA_CONTEXT_SLOT",
	LdaCurrentContextSlot: "LDA_CURRENT_CONTEXT_SLOT", StaCurrentContextSlot: "STA_CURRENT_CONTEXT_SLOT",
	Add: "ADD", Sub: "SUB", Mul: "MUL", Div: "DIV", Mod: "MOD", Pow: "POW", AddSmi: "ADD_SMI", SubSmi: "SUB_SMI",
	BitwiseAnd: "BITWISE_AND", BitwiseOr: "BITWISE_OR", BitwiseXor: "BITWISE_XOR", BitwiseNot: "BITWISE_NOT",
	ShiftLeft: "SHIFT_LEFT", ShiftRight: "SHIFT_RIGHT", ShiftRightLogical: "SHIFT_RIGHT_LOGICAL", Negate: "NEGATE",
	TestEqual: "TEST_EQUAL", TestNotEqual: "TEST_NOT_EQUAL", TestStrictEqual: "TEST_STRICT_EQUAL",
	TestStrictNotEqual: "TEST_STRICT_NOT_EQUAL", TestLessThan: "TEST_LESS_THAN", TestGreaterThan: "TEST_GREATER_THAN",
	TestLessThanOrEqual: "TEST_LESS_THAN_OR_EQUAL", TestGreaterThanOrEqual: "TEST_GREATER_THAN_OR_EQUAL",
	TestIn: "TEST_IN", TestInstanceof: "TEST_INSTANCEOF", TestUndetectable: "TEST_UNDETECTABLE",
	LogicalNot: "LOGICAL_NOT", Typeof: "TYPEOF", Jump: "JUMP", JumpIfTrue: "JUMP_IF_TRUE",
	JumpIfFalse: "JUMP_IF_FALSE", JumpIfNull: "JUMP_IF_NULL", JumpIfUndefined: "JUMP_IF_UNDEFINED",
	JumpIfNullOrUndefined: "JUMP_IF_NULL_OR_UNDEFINED", JumpIfToBooleanTrue: "JUMP_IF_TO_BOOLEAN_TRUE",
	JumpIfToBooleanFalse: "JUMP_IF_TO_BOOLEAN_FALSE", JumpLoop: "JUMP_LOOP", CallAnyReceiver: "CALL_ANY_RECEIVER",
	CallProperty: "CALL_PROPERTY", CallUndefinedReceiver: "CALL_UNDEFINED_RECEIVER", Construct: "CONSTRUCT",
	ConstructWithSpread: "CONSTRUCT_WITH_SPREAD", CallWithSpread: "CALL_WITH_SPREAD", Return: "RETURN",
	SuspendGenerator: "SUSPEND_GENERATOR", ResumeGenerator: "RESUME_GENERATOR", LdaNamedProperty: "LDA_NAMED_PROPERTY",
	StaNamedProperty: "STA_NAMED_PROPERTY", LdaKeyedProperty: "LDA_KEYED_PROPERTY", StaKeyedProperty: "STA_KEYED_PROPERTY",
	LdaNamedPropertyNoFeedback: "LDA_NAMED_PROPERTY_NO_FEEDBACK",
	StaNamedPropertyNoFeedback: "STA_NAMED_PROPERTY_NO_FEEDBACK", DeletePropertyStrict: "DELETE_PROPERTY_STRICT",
	DeletePropertySloppy: "DELETE_PROPERTY_SLOPPY", CreateObjectLiteral: "CREATE_OBJECT_LITERAL",
	CreateArrayLiteral: "CREATE_ARRAY_LITERAL", CreateRegexpLiteral: "CREATE_REGEXP_LITERAL", CreateClosure: "CREATE_CLOSURE",
	CreateContext: "CREATE_CONTEXT", CloneObject: "CLONE_OBJECT", GetIterator: "GET_ITERATOR",
	CallIteratorStep: "CALL_ITERATOR_STEP", GetIteratorDone: "GET_ITERATOR_DONE", GetIteratorValue: "GET_ITERATOR_VALUE",
	Throw: "THROW", Rethrow: "RETHROW", PushContext: "PUSH_CONTEXT", PopContext: "POP_CONTEXT",
	LdaModuleVariable: "LDA_MODULE_VARIABLE", StaModuleVariable: "STA_MODULE_VARIABLE", StackCheck: "STACK_CHECK",
	Debugger: "DEBUGGER", Halt: "HALT",
}

// OpcodeName returns a stable human-readable name for an opcode.
func OpcodeName(op Opcode) string {
	if name, ok := opcodeNames[op]; ok {
		return name
	}
	return "UNKNOWN"
}

func (op Opcode) String() string {
	return OpcodeName(op)
}
