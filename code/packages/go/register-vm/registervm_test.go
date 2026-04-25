package registervm

import (
	"errors"
	"testing"
)

func TestLoadsArithmeticAndFeedback(t *testing.T) {
	code := CodeObject{
		Instructions: []RegisterInstruction{
			{Opcode: LdaConstant, Operands: []int{0}},
			{Opcode: Star, Operands: []int{0}},
			{Opcode: LdaConstant, Operands: []int{1}},
			{Opcode: Add, Operands: []int{0, 0}},
			{Opcode: Halt},
		},
		Constants:         []VMValue{40, 2},
		RegisterCount:     1,
		FeedbackSlotCount: 1,
	}

	result, err := NewRegisterVM().Execute(code)
	if err != nil {
		t.Fatalf("execute failed: %v", err)
	}
	if result.Value != 42 {
		t.Fatalf("expected 42, got %#v", result.Value)
	}
	if result.FeedbackVector[0].Kind != FeedbackMonomorphic {
		t.Fatalf("expected monomorphic feedback, got %#v", result.FeedbackVector[0])
	}
}

func TestRegisterMovesAndSmallIntegers(t *testing.T) {
	code := CodeObject{
		Instructions: []RegisterInstruction{
			{Opcode: LdaSmi, Operands: []int{10}},
			{Opcode: Star, Operands: []int{0}},
			{Opcode: Mov, Operands: []int{0, 1}},
			{Opcode: Ldar, Operands: []int{1}},
			{Opcode: AddSmi, Operands: []int{5}},
			{Opcode: SubSmi, Operands: []int{3}},
			{Opcode: Halt},
		},
		RegisterCount: 2,
	}

	result, err := NewRegisterVM().Execute(code)
	if err != nil {
		t.Fatalf("execute failed: %v", err)
	}
	if result.Value != 12 {
		t.Fatalf("expected 12, got %#v", result.Value)
	}
}

func TestStringConcatenationAndTypeof(t *testing.T) {
	code := CodeObject{
		Instructions: []RegisterInstruction{
			{Opcode: LdaConstant, Operands: []int{0}},
			{Opcode: Star, Operands: []int{0}},
			{Opcode: LdaConstant, Operands: []int{1}},
			{Opcode: Add, Operands: []int{0}},
			{Opcode: Typeof},
			{Opcode: Halt},
		},
		Constants:     []VMValue{"world", "hello "},
		RegisterCount: 1,
	}

	result, err := NewRegisterVM().Execute(code)
	if err != nil {
		t.Fatalf("execute failed: %v", err)
	}
	if result.Value != "string" {
		t.Fatalf("expected typeof string, got %#v", result.Value)
	}
}

func TestControlFlow(t *testing.T) {
	code := CodeObject{
		Instructions: []RegisterInstruction{
			{Opcode: LdaFalse},
			{Opcode: JumpIfFalse, Operands: []int{2}},
			{Opcode: LdaConstant, Operands: []int{0}},
			{Opcode: Jump, Operands: []int{1}},
			{Opcode: LdaConstant, Operands: []int{1}},
			{Opcode: Halt},
		},
		Constants: []VMValue{"wrong", "right"},
	}

	result, err := NewRegisterVM().Execute(code)
	if err != nil {
		t.Fatalf("execute failed: %v", err)
	}
	if result.Value != "right" {
		t.Fatalf("expected right branch, got %#v", result.Value)
	}
}

func TestGlobalsAndModuleVariables(t *testing.T) {
	vm := NewRegisterVM()
	code := CodeObject{
		Instructions: []RegisterInstruction{
			{Opcode: LdaConstant, Operands: []int{0}},
			{Opcode: StaGlobal, Operands: []int{0}},
			{Opcode: LdaGlobal, Operands: []int{0}},
			{Opcode: StaModuleVariable, Operands: []int{1}},
			{Opcode: LdaModuleVariable, Operands: []int{1}},
			{Opcode: Halt},
		},
		Constants: []VMValue{99},
		Names:     []string{"answer", "exportedAnswer"},
	}

	result, err := vm.Execute(code)
	if err != nil {
		t.Fatalf("execute failed: %v", err)
	}
	if result.Value != 99 || result.Globals["answer"] != 99 || result.Globals["exportedAnswer"] != 99 {
		t.Fatalf("unexpected globals/result: %#v %#v", result.Value, result.Globals)
	}
}

func TestContextSlots(t *testing.T) {
	vm := NewRegisterVM()
	vm.Context = NewContext(nil, 2)
	code := CodeObject{
		Instructions: []RegisterInstruction{
			{Opcode: LdaConstant, Operands: []int{0}},
			{Opcode: StaCurrentContextSlot, Operands: []int{1}},
			{Opcode: LdaCurrentContextSlot, Operands: []int{1}},
			{Opcode: Halt},
		},
		Constants: []VMValue{"closed"},
	}

	result, err := vm.Execute(code)
	if err != nil {
		t.Fatalf("execute failed: %v", err)
	}
	if result.Value != "closed" {
		t.Fatalf("expected closed-over value, got %#v", result.Value)
	}
}

func TestNamedAndKeyedProperties(t *testing.T) {
	obj := NewObject()
	code := CodeObject{
		Instructions: []RegisterInstruction{
			{Opcode: LdaConstant, Operands: []int{0}},
			{Opcode: Star, Operands: []int{0}},
			{Opcode: LdaConstant, Operands: []int{1}},
			{Opcode: StaNamedProperty, Operands: []int{0, 0, 0}},
			{Opcode: LdaNamedProperty, Operands: []int{0, 0, 0}},
			{Opcode: Star, Operands: []int{1}},
			{Opcode: LdaConstant, Operands: []int{2}},
			{Opcode: Star, Operands: []int{2}},
			{Opcode: LdaConstant, Operands: []int{3}},
			{Opcode: StaKeyedProperty, Operands: []int{0, 2}},
			{Opcode: LdaKeyedProperty, Operands: []int{0, 2}},
			{Opcode: Halt},
		},
		Constants:         []VMValue{obj, 42, "dynamic", "value"},
		Names:             []string{"answer"},
		RegisterCount:     3,
		FeedbackSlotCount: 1,
	}

	result, err := NewRegisterVM().Execute(code)
	if err != nil {
		t.Fatalf("execute failed: %v", err)
	}
	if result.Value != "value" || obj.Properties["answer"] != 42 {
		t.Fatalf("unexpected object properties: %#v", obj.Properties)
	}
	if result.FeedbackVector[0].Kind != FeedbackMonomorphic {
		t.Fatalf("expected property feedback, got %#v", result.FeedbackVector[0])
	}
}

func TestArrayLengthAndDelete(t *testing.T) {
	obj := NewObject()
	obj.Properties["gone"] = true
	code := CodeObject{
		Instructions: []RegisterInstruction{
			{Opcode: LdaConstant, Operands: []int{0}},
			{Opcode: Star, Operands: []int{0}},
			{Opcode: LdaConstant, Operands: []int{1}},
			{Opcode: Star, Operands: []int{1}},
			{Opcode: DeletePropertySloppy, Operands: []int{0, 1}},
			{Opcode: Halt},
		},
		Constants:     []VMValue{obj, "gone"},
		RegisterCount: 2,
	}

	result, err := NewRegisterVM().Execute(code)
	if err != nil {
		t.Fatalf("execute failed: %v", err)
	}
	if result.Value != true {
		t.Fatalf("expected delete success, got %#v", result.Value)
	}
	if _, ok := obj.Properties["gone"]; ok {
		t.Fatal("expected property to be deleted")
	}

	arrayCode := CodeObject{
		Instructions: []RegisterInstruction{
			{Opcode: LdaConstant, Operands: []int{0}},
			{Opcode: Star, Operands: []int{0}},
			{Opcode: LdaNamedPropertyNoFeedback, Operands: []int{0, 0}},
			{Opcode: Halt},
		},
		Constants:     []VMValue{[]VMValue{1, 2, 3}},
		Names:         []string{"length"},
		RegisterCount: 1,
	}
	arrayResult, err := NewRegisterVM().Execute(arrayCode)
	if err != nil {
		t.Fatalf("array execute failed: %v", err)
	}
	if arrayResult.Value != 3 {
		t.Fatalf("expected length 3, got %#v", arrayResult.Value)
	}
}

func TestComparisonsBitwiseAndLogicalNot(t *testing.T) {
	code := CodeObject{
		Instructions: []RegisterInstruction{
			{Opcode: LdaConstant, Operands: []int{0}},
			{Opcode: Star, Operands: []int{0}},
			{Opcode: LdaConstant, Operands: []int{1}},
			{Opcode: TestGreaterThan, Operands: []int{0}},
			{Opcode: LogicalNot},
			{Opcode: Halt},
		},
		Constants:     []VMValue{5, 10},
		RegisterCount: 1,
	}

	result, err := NewRegisterVM().Execute(code)
	if err != nil {
		t.Fatalf("execute failed: %v", err)
	}
	if result.Value != false {
		t.Fatalf("expected logical negation of true, got %#v", result.Value)
	}

	bitwise := CodeObject{
		Instructions: []RegisterInstruction{
			{Opcode: LdaConstant, Operands: []int{0}},
			{Opcode: Star, Operands: []int{0}},
			{Opcode: LdaConstant, Operands: []int{1}},
			{Opcode: BitwiseOr, Operands: []int{0}},
			{Opcode: Halt},
		},
		Constants:     []VMValue{2, 4},
		RegisterCount: 1,
	}
	bitwiseResult, err := NewRegisterVM().Execute(bitwise)
	if err != nil {
		t.Fatalf("bitwise execute failed: %v", err)
	}
	if bitwiseResult.Value != 6 {
		t.Fatalf("expected 6, got %#v", bitwiseResult.Value)
	}
}

func TestArithmeticVariants(t *testing.T) {
	tests := []struct {
		name     string
		opcode   Opcode
		right    VMValue
		left     VMValue
		expected VMValue
	}{
		{name: "sub", opcode: Sub, right: 2, left: 10, expected: 8},
		{name: "mul", opcode: Mul, right: 6, left: 7, expected: 42},
		{name: "div", opcode: Div, right: 2, left: 9, expected: 4.5},
		{name: "mod", opcode: Mod, right: 3, left: 10, expected: 1},
		{name: "pow", opcode: Pow, right: 3, left: 2, expected: 8},
		{name: "and", opcode: BitwiseAnd, right: 0b0110, left: 0b1010, expected: 0b0010},
		{name: "xor", opcode: BitwiseXor, right: 0b0110, left: 0b1010, expected: 0b1100},
		{name: "shift left", opcode: ShiftLeft, right: 2, left: 3, expected: 12},
		{name: "shift right", opcode: ShiftRight, right: 1, left: 8, expected: 4},
		{name: "logical shift right", opcode: ShiftRightLogical, right: 1, left: -2, expected: 2147483647},
	}

	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			result, err := NewRegisterVM().Execute(CodeObject{
				Instructions: []RegisterInstruction{
					{Opcode: LdaConstant, Operands: []int{0}},
					{Opcode: Star, Operands: []int{0}},
					{Opcode: LdaConstant, Operands: []int{1}},
					{Opcode: test.opcode, Operands: []int{0}},
					{Opcode: Halt},
				},
				Constants:     []VMValue{test.right, test.left},
				RegisterCount: 1,
			})
			if err != nil {
				t.Fatalf("execute failed: %v", err)
			}
			if result.Value != test.expected {
				t.Fatalf("expected %#v, got %#v", test.expected, result.Value)
			}
		})
	}

	unary := CodeObject{
		Instructions: []RegisterInstruction{
			{Opcode: LdaConstant, Operands: []int{0}},
			{Opcode: BitwiseNot},
			{Opcode: Star, Operands: []int{0}},
			{Opcode: LdaConstant, Operands: []int{1}},
			{Opcode: Negate},
			{Opcode: Add, Operands: []int{0}},
			{Opcode: Halt},
		},
		Constants:     []VMValue{0, 5},
		RegisterCount: 1,
	}
	result, err := NewRegisterVM().Execute(unary)
	if err != nil {
		t.Fatalf("unary execute failed: %v", err)
	}
	if result.Value != -6 {
		t.Fatalf("expected -6, got %#v", result.Value)
	}
}

func TestComparisonAndMembershipVariants(t *testing.T) {
	obj := NewObject()
	obj.Properties["answer"] = 42

	tests := []struct {
		name     string
		opcode   Opcode
		right    VMValue
		left     VMValue
		expected VMValue
	}{
		{name: "loose equal", opcode: TestEqual, right: 7, left: "7", expected: true},
		{name: "not equal", opcode: TestNotEqual, right: 7, left: 8, expected: true},
		{name: "strict equal", opcode: TestStrictEqual, right: 7, left: 7, expected: true},
		{name: "strict not equal", opcode: TestStrictNotEqual, right: 7, left: "7", expected: true},
		{name: "less than", opcode: TestLessThan, right: 10, left: 5, expected: true},
		{name: "less than or equal", opcode: TestLessThanOrEqual, right: 5, left: 5, expected: true},
		{name: "greater than or equal", opcode: TestGreaterThanOrEqual, right: 5, left: 6, expected: true},
		{name: "in object", opcode: TestIn, right: obj, left: "answer", expected: true},
		{name: "in string", opcode: TestIn, right: "abcdef", left: "bcd", expected: true},
		{name: "instanceof", opcode: TestInstanceof, right: obj, left: NewObject(), expected: true},
	}

	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			result, err := NewRegisterVM().Execute(CodeObject{
				Instructions: []RegisterInstruction{
					{Opcode: LdaConstant, Operands: []int{0}},
					{Opcode: Star, Operands: []int{0}},
					{Opcode: LdaConstant, Operands: []int{1}},
					{Opcode: test.opcode, Operands: []int{0}},
					{Opcode: Halt},
				},
				Constants:     []VMValue{test.right, test.left},
				RegisterCount: 1,
			})
			if err != nil {
				t.Fatalf("execute failed: %v", err)
			}
			if result.Value != test.expected {
				t.Fatalf("expected %#v, got %#v", test.expected, result.Value)
			}
		})
	}

	result, err := NewRegisterVM().Execute(CodeObject{
		Instructions: []RegisterInstruction{
			{Opcode: LdaUndefined},
			{Opcode: TestUndetectable},
			{Opcode: Halt},
		},
	})
	if err != nil {
		t.Fatalf("undetectable execute failed: %v", err)
	}
	if result.Value != true {
		t.Fatalf("expected undefined to be undetectable, got %#v", result.Value)
	}
}

func TestJumpVariants(t *testing.T) {
	tests := []struct {
		name   string
		load   Opcode
		jump   Opcode
		offset int
	}{
		{name: "true", load: LdaTrue, jump: JumpIfTrue, offset: 1},
		{name: "to boolean true", load: LdaTrue, jump: JumpIfToBooleanTrue, offset: 1},
		{name: "to boolean false", load: LdaZero, jump: JumpIfToBooleanFalse, offset: 1},
		{name: "null", load: LdaNull, jump: JumpIfNull, offset: 1},
		{name: "undefined", load: LdaUndefined, jump: JumpIfUndefined, offset: 1},
		{name: "null or undefined", load: LdaNull, jump: JumpIfNullOrUndefined, offset: 1},
	}

	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			result, err := NewRegisterVM().Execute(CodeObject{
				Instructions: []RegisterInstruction{
					{Opcode: test.load},
					{Opcode: test.jump, Operands: []int{test.offset}},
					{Opcode: LdaConstant, Operands: []int{0}},
					{Opcode: LdaConstant, Operands: []int{1}},
					{Opcode: Halt},
				},
				Constants: []VMValue{"wrong", "right"},
			})
			if err != nil {
				t.Fatalf("execute failed: %v", err)
			}
			if result.Value != "right" {
				t.Fatalf("expected jump to right branch, got %#v", result.Value)
			}
		})
	}
}

func TestContextDepthAndCreationOpcodes(t *testing.T) {
	code := CodeObject{
		Instructions: []RegisterInstruction{
			{Opcode: CreateContext, Operands: []int{1}},
			{Opcode: LdaConstant, Operands: []int{0}},
			{Opcode: StaCurrentContextSlot, Operands: []int{0}},
			{Opcode: PushContext, Operands: []int{1}},
			{Opcode: LdaConstant, Operands: []int{1}},
			{Opcode: StaCurrentContextSlot, Operands: []int{0}},
			{Opcode: LdaContextSlot, Operands: []int{1, 0}},
			{Opcode: PopContext},
			{Opcode: Halt},
		},
		Constants: []VMValue{"outer", "inner"},
	}

	result, err := NewRegisterVM().Execute(code)
	if err != nil {
		t.Fatalf("execute failed: %v", err)
	}
	if result.Value != "outer" {
		t.Fatalf("expected outer context value, got %#v", result.Value)
	}
}

func TestCreationCloneRegexpAndArrayKeyedStore(t *testing.T) {
	obj := NewObject()
	obj.Properties["answer"] = 42

	cloneResult, err := NewRegisterVM().Execute(CodeObject{
		Instructions: []RegisterInstruction{
			{Opcode: LdaConstant, Operands: []int{0}},
			{Opcode: CloneObject},
			{Opcode: Star, Operands: []int{0}},
			{Opcode: LdaNamedPropertyNoFeedback, Operands: []int{0, 0}},
			{Opcode: Halt},
		},
		Constants:     []VMValue{obj},
		Names:         []string{"answer"},
		RegisterCount: 1,
	})
	if err != nil {
		t.Fatalf("clone execute failed: %v", err)
	}
	if cloneResult.Value != 42 {
		t.Fatalf("expected cloned property, got %#v", cloneResult.Value)
	}

	arrayResult, err := NewRegisterVM().Execute(CodeObject{
		Instructions: []RegisterInstruction{
			{Opcode: CreateArrayLiteral},
			{Opcode: Star, Operands: []int{0}},
			{Opcode: LdaSmi, Operands: []int{2}},
			{Opcode: Star, Operands: []int{1}},
			{Opcode: LdaConstant, Operands: []int{0}},
			{Opcode: StaKeyedProperty, Operands: []int{0, 1}},
			{Opcode: LdaNamedPropertyNoFeedback, Operands: []int{0, 0}},
			{Opcode: Halt},
		},
		Constants:     []VMValue{"third"},
		Names:         []string{"length"},
		RegisterCount: 2,
	})
	if err != nil {
		t.Fatalf("array execute failed: %v", err)
	}
	if arrayResult.Value != 3 {
		t.Fatalf("expected extended array length, got %#v", arrayResult.Value)
	}

	regexpResult, err := NewRegisterVM().Execute(CodeObject{
		Instructions: []RegisterInstruction{
			{Opcode: CreateRegexpLiteral, Operands: []int{0}},
			{Opcode: Halt},
		},
		Constants: []VMValue{"[a-z]+"},
	})
	if err != nil {
		t.Fatalf("regexp execute failed: %v", err)
	}
	if regexpResult.Value != "[a-z]+" {
		t.Fatalf("expected regexp placeholder, got %#v", regexpResult.Value)
	}

	closureResult, err := NewRegisterVM().Execute(CodeObject{
		Instructions: []RegisterInstruction{
			{Opcode: CreateClosure, Operands: []int{0}},
			{Opcode: Typeof},
			{Opcode: Halt},
		},
		Constants: []VMValue{CodeObject{Name: "inner"}},
	})
	if err != nil {
		t.Fatalf("closure execute failed: %v", err)
	}
	if closureResult.Value != "function" {
		t.Fatalf("expected closure typeof function, got %#v", closureResult.Value)
	}
}

func TestNativeCallAndTrace(t *testing.T) {
	native := NativeFunction(func(args []VMValue) (VMValue, error) {
		if len(args) != 2 {
			return nil, errors.New("expected two args")
		}
		return args[0].(int) + args[1].(int), nil
	})
	code := CodeObject{
		Instructions: []RegisterInstruction{
			{Opcode: LdaConstant, Operands: []int{0}},
			{Opcode: Star, Operands: []int{0}},
			{Opcode: LdaConstant, Operands: []int{1}},
			{Opcode: Star, Operands: []int{1}},
			{Opcode: LdaConstant, Operands: []int{2}},
			{Opcode: Star, Operands: []int{2}},
			{Opcode: CallUndefinedReceiver, Operands: []int{0, 1, 2, 0}},
			{Opcode: Halt},
		},
		Constants:         []VMValue{native, 20, 22},
		RegisterCount:     3,
		FeedbackSlotCount: 1,
	}

	result, err := NewRegisterVM().ExecuteWithTrace(code)
	if err != nil {
		t.Fatalf("execute failed: %v", err)
	}
	if result.Value != 42 {
		t.Fatalf("expected native call result, got %#v", result.Value)
	}
	if len(result.Trace) != len(code.Instructions) {
		t.Fatalf("expected trace for every instruction, got %d", len(result.Trace))
	}
	if result.FeedbackVector[0].Kind != FeedbackMonomorphic {
		t.Fatalf("expected call feedback, got %#v", result.FeedbackVector[0])
	}
}

func TestHelpers(t *testing.T) {
	if OpcodeName(Add) != "ADD" || OpcodeName(Opcode(0xDE)) != "UNKNOWN" || Add.String() != "ADD" {
		t.Fatalf("unexpected opcode names")
	}
	if (RegisterInstruction{Opcode: Add, Operands: []int{0}}).String() != "ADD [0]" {
		t.Fatalf("unexpected instruction string")
	}
	if Undefined.(undefinedValue).String() != "undefined" {
		t.Fatalf("unexpected undefined string")
	}
	if (&VMError{Message: "bad", InstructionIndex: 3, Opcode: Add}).Error() != "bad at instruction 3 (ADD)" {
		t.Fatalf("unexpected VMError string")
	}

	ctx := NewContext(nil, 1)
	if err := SetSlot(ctx, 0, 0, "value"); err != nil {
		t.Fatalf("set slot failed: %v", err)
	}
	got, err := GetSlot(ctx, 0, 0)
	if err != nil {
		t.Fatalf("get slot failed: %v", err)
	}
	if got != "value" {
		t.Fatalf("unexpected slot value %#v", got)
	}

	vector := NewVector(1)
	RecordBinaryOp(vector, 0, 1, 2)
	RecordBinaryOp(vector, 0, "a", "b")
	RecordBinaryOp(vector, 0, true, false)
	RecordBinaryOp(vector, 0, nil, Undefined)
	RecordBinaryOp(vector, 0, NewObject(), NewObject())
	if vector[0].Kind != FeedbackMegamorphic {
		t.Fatalf("expected megamorphic feedback, got %#v", vector[0])
	}

	empty := NewVector(0)
	RecordBinaryOp(empty, 0, 1, 2)
	RecordPropertyLoad(empty, 0, 1)
	RecordCallSite(empty, 0, "function")
}

func TestErrors(t *testing.T) {
	_, err := NewRegisterVM().Execute(CodeObject{
		Instructions: []RegisterInstruction{{Opcode: Opcode(0xEE)}},
	})
	if err == nil {
		t.Fatal("expected unknown opcode error")
	}

	_, err = NewRegisterVM().Execute(CodeObject{
		Instructions: []RegisterInstruction{
			{Opcode: LdaConstant, Operands: []int{0}},
			{Opcode: Throw},
		},
		Constants: []VMValue{"boom"},
	})
	if err == nil {
		t.Fatal("expected throw error")
	}

	_, err = (&RegisterVM{MaxSteps: 3}).Execute(CodeObject{
		Instructions: []RegisterInstruction{
			{Opcode: JumpLoop, Operands: []int{-1}},
		},
	})
	if err == nil {
		t.Fatal("expected max steps error")
	}

	_, err = NewRegisterVM().Execute(CodeObject{
		Instructions: []RegisterInstruction{{Opcode: LdaConstant, Operands: []int{99}}},
	})
	if err == nil {
		t.Fatal("expected constant bounds error")
	}

	_, err = NewRegisterVM().Execute(CodeObject{
		Instructions: []RegisterInstruction{{Opcode: CallWithSpread}},
	})
	if err == nil {
		t.Fatal("expected unsupported opcode error")
	}
}
