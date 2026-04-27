package registervm

import (
	"fmt"
	"math"
	"reflect"
	"strconv"
)

const defaultMaxSteps = 100000

// RegisterVM executes register bytecode.
type RegisterVM struct {
	Globals  map[string]VMValue
	Context  *Context
	MaxSteps int
}

// NewRegisterVM creates a VM with an empty global scope.
func NewRegisterVM() *RegisterVM {
	return &RegisterVM{
		Globals:  map[string]VMValue{},
		MaxSteps: defaultMaxSteps,
	}
}

// Execute runs bytecode and returns the final accumulator value.
func (vm *RegisterVM) Execute(code CodeObject) (VMResult, error) {
	return vm.execute(code, false)
}

// ExecuteWithTrace runs bytecode and records one TraceStep per instruction.
func (vm *RegisterVM) ExecuteWithTrace(code CodeObject) (VMResult, error) {
	return vm.execute(code, true)
}

func (vm *RegisterVM) execute(code CodeObject, wantTrace bool) (VMResult, error) {
	if vm.Globals == nil {
		vm.Globals = map[string]VMValue{}
	}
	maxSteps := vm.MaxSteps
	if maxSteps <= 0 {
		maxSteps = defaultMaxSteps
	}

	registers := make([]VMValue, code.RegisterCount)
	for i := range registers {
		registers[i] = Undefined
	}
	feedback := NewVector(code.FeedbackSlotCount)
	context := vm.Context
	accumulator := Undefined
	ip := 0
	steps := 0
	trace := []TraceStep{}

	for ip >= 0 && ip < len(code.Instructions) {
		if steps >= maxSteps {
			return VMResult{}, &VMError{Message: "maximum instruction count exceeded", InstructionIndex: ip, Opcode: code.Instructions[ip].Opcode}
		}
		steps++

		instr := code.Instructions[ip]
		currentIP := ip
		accBefore := accumulator
		regsBefore := cloneValues(registers)
		feedbackBefore := cloneFeedback(feedback)
		ip++

		if err := vm.step(code, instr, currentIP, &ip, &accumulator, registers, feedback, &context); err != nil {
			if vmErr, ok := err.(*VMError); ok {
				return VMResult{}, vmErr
			}
			return VMResult{}, &VMError{Message: err.Error(), InstructionIndex: currentIP, Opcode: instr.Opcode}
		}

		if wantTrace {
			trace = append(trace, TraceStep{
				IP:                currentIP,
				Instruction:       instr,
				AccumulatorBefore: accBefore,
				AccumulatorAfter:  accumulator,
				RegistersBefore:   regsBefore,
				RegistersAfter:    cloneValues(registers),
				FeedbackBefore:    feedbackBefore,
				FeedbackAfter:     cloneFeedback(feedback),
			})
		}

		if instr.Opcode == Halt || instr.Opcode == Return {
			vm.Context = context
			return VMResult{Value: accumulator, Globals: cloneGlobals(vm.Globals), FeedbackVector: cloneFeedback(feedback), Trace: trace}, nil
		}
	}

	vm.Context = context
	return VMResult{Value: accumulator, Globals: cloneGlobals(vm.Globals), FeedbackVector: cloneFeedback(feedback), Trace: trace}, nil
}

func (vm *RegisterVM) step(code CodeObject, instr RegisterInstruction, currentIP int, ip *int, accumulator *VMValue, registers []VMValue, feedback []FeedbackSlot, context **Context) error {
	operands := instr.Operands
	op := instr.Opcode

	switch op {
	case LdaConstant:
		idx, err := operand(operands, 0)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		value, err := constantAt(code, idx)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		*accumulator = value
	case LdaZero:
		*accumulator = 0
	case LdaSmi:
		value, err := operand(operands, 0)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		*accumulator = value
	case LdaUndefined:
		*accumulator = Undefined
	case LdaNull:
		*accumulator = nil
	case LdaTrue:
		*accumulator = true
	case LdaFalse:
		*accumulator = false
	case Ldar, LdaLocal:
		idx, err := operand(operands, 0)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		value, err := registerAt(registers, idx)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		*accumulator = value
	case Star, StaLocal:
		idx, err := operand(operands, 0)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		if err := setRegister(registers, idx, *accumulator); err != nil {
			return vmError(err.Error(), currentIP, op)
		}
	case Mov:
		src, err := operand(operands, 0)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		dst, err := operand(operands, 1)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		value, err := registerAt(registers, src)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		if err := setRegister(registers, dst, value); err != nil {
			return vmError(err.Error(), currentIP, op)
		}
	case LdaGlobal, LdaModuleVariable:
		name, err := nameAt(code, operands, 0)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		value, ok := vm.Globals[name]
		if !ok {
			if op == LdaModuleVariable {
				*accumulator = Undefined
				return nil
			}
			return vmError(fmt.Sprintf("global %q is not defined", name), currentIP, op)
		}
		*accumulator = value
	case StaGlobal, StaModuleVariable:
		name, err := nameAt(code, operands, 0)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		vm.Globals[name] = *accumulator
	case LdaContextSlot:
		depth, idx, err := depthIndex(operands)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		value, err := GetSlot(*context, depth, idx)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		*accumulator = value
	case StaContextSlot:
		depth, idx, err := depthIndex(operands)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		if err := SetSlot(*context, depth, idx, *accumulator); err != nil {
			return vmError(err.Error(), currentIP, op)
		}
	case LdaCurrentContextSlot:
		idx, err := operand(operands, 0)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		value, err := GetSlot(*context, 0, idx)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		*accumulator = value
	case StaCurrentContextSlot:
		idx, err := operand(operands, 0)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		if err := SetSlot(*context, 0, idx, *accumulator); err != nil {
			return vmError(err.Error(), currentIP, op)
		}
	case Add, Sub, Mul, Div, Mod, Pow, BitwiseAnd, BitwiseOr, BitwiseXor, ShiftLeft, ShiftRight, ShiftRightLogical:
		idx, err := operand(operands, 0)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		right, err := registerAt(registers, idx)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		fbSlot := feedbackIndex(instr, 1)
		RecordBinaryOp(feedback, fbSlot, *accumulator, right)
		result, err := binaryOp(op, *accumulator, right)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		*accumulator = result
	case AddSmi, SubSmi:
		value, err := operand(operands, 0)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		right := VMValue(value)
		RecordBinaryOp(feedback, feedbackIndex(instr, 1), *accumulator, right)
		if op == AddSmi {
			result, err := binaryOp(Add, *accumulator, right)
			if err != nil {
				return vmError(err.Error(), currentIP, op)
			}
			*accumulator = result
		} else {
			result, err := binaryOp(Sub, *accumulator, right)
			if err != nil {
				return vmError(err.Error(), currentIP, op)
			}
			*accumulator = result
		}
	case BitwiseNot:
		value, ok := toInt(*accumulator)
		if !ok {
			return vmError("bitwise not requires an integer", currentIP, op)
		}
		*accumulator = ^value
	case Negate:
		value, ok := toNumber(*accumulator)
		if !ok {
			return vmError("negate requires a number", currentIP, op)
		}
		if integral, ok := toInt(*accumulator); ok {
			*accumulator = -integral
		} else {
			*accumulator = -value
		}
	case TestEqual, TestNotEqual, TestStrictEqual, TestStrictNotEqual, TestLessThan, TestGreaterThan, TestLessThanOrEqual, TestGreaterThanOrEqual, TestIn, TestInstanceof:
		idx, err := operand(operands, 0)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		right, err := registerAt(registers, idx)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		result, err := compareOp(op, *accumulator, right)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		*accumulator = result
	case TestUndetectable:
		*accumulator = *accumulator == nil || IsUndefined(*accumulator)
	case LogicalNot:
		*accumulator = !truthy(*accumulator)
	case Typeof:
		*accumulator = ValueType(*accumulator)
	case Jump, JumpLoop:
		offset, err := operand(operands, 0)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		*ip += offset
	case JumpIfTrue, JumpIfToBooleanTrue:
		offset, err := operand(operands, 0)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		if truthy(*accumulator) {
			*ip += offset
		}
	case JumpIfFalse, JumpIfToBooleanFalse:
		offset, err := operand(operands, 0)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		if !truthy(*accumulator) {
			*ip += offset
		}
	case JumpIfNull:
		offset, err := operand(operands, 0)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		if *accumulator == nil {
			*ip += offset
		}
	case JumpIfUndefined:
		offset, err := operand(operands, 0)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		if IsUndefined(*accumulator) {
			*ip += offset
		}
	case JumpIfNullOrUndefined:
		offset, err := operand(operands, 0)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		if *accumulator == nil || IsUndefined(*accumulator) {
			*ip += offset
		}
	case CallAnyReceiver, CallUndefinedReceiver:
		if err := vm.callNative(code, instr, currentIP, accumulator, registers, feedback); err != nil {
			return err
		}
	case Return:
		return nil
	case LdaNamedProperty, LdaNamedPropertyNoFeedback:
		obj, name, err := objectAndName(code, registers, operands)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		if op == LdaNamedProperty {
			if receiver, ok := obj.(*VMObject); ok {
				RecordPropertyLoad(feedback, feedbackIndex(instr, 2), receiver.HiddenClassID)
			}
		}
		value, err := loadNamedProperty(obj, name)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		*accumulator = value
	case StaNamedProperty, StaNamedPropertyNoFeedback:
		obj, name, err := objectAndName(code, registers, operands)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		if op == StaNamedProperty {
			if receiver, ok := obj.(*VMObject); ok {
				RecordPropertyLoad(feedback, feedbackIndex(instr, 2), receiver.HiddenClassID)
			}
		}
		if err := storeNamedProperty(obj, name, *accumulator); err != nil {
			return vmError(err.Error(), currentIP, op)
		}
	case LdaKeyedProperty:
		obj, key, err := objectAndKey(registers, operands)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		value, err := loadKeyedProperty(obj, key)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		*accumulator = value
	case StaKeyedProperty:
		objReg, err := operand(operands, 0)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		obj, key, err := objectAndKey(registers, operands)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		updated, err := storeKeyedProperty(obj, key, *accumulator)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		if updated != nil {
			registers[objReg] = updated
		}
	case DeletePropertyStrict, DeletePropertySloppy:
		obj, key, err := objectAndKey(registers, operands)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		*accumulator = deleteProperty(obj, key)
	case CreateObjectLiteral:
		*accumulator = NewObject()
	case CreateArrayLiteral:
		*accumulator = []VMValue{}
	case CreateRegexpLiteral:
		if len(operands) == 0 {
			*accumulator = ""
			return nil
		}
		value, err := constantAt(code, operands[0])
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		*accumulator = value
	case CreateClosure:
		idx, err := operand(operands, 0)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		value, err := constantAt(code, idx)
		if err != nil {
			return vmError(err.Error(), currentIP, op)
		}
		inner, ok := value.(CodeObject)
		if !ok {
			return vmError("CREATE_CLOSURE constant is not a CodeObject", currentIP, op)
		}
		*accumulator = &VMFunction{Code: inner, Context: *context}
	case CreateContext, PushContext:
		slotCount := 0
		if len(operands) > 0 {
			slotCount = operands[0]
		}
		*context = NewContext(*context, slotCount)
	case CloneObject:
		*accumulator = cloneValue(*accumulator)
	case PopContext:
		if *context != nil && (*context).Parent != nil {
			*context = (*context).Parent
		}
	case Throw:
		return vmError(fmt.Sprint(*accumulator), currentIP, op)
	case Rethrow:
		return vmError("RETHROW is not implemented", currentIP, op)
	case StackCheck, Debugger, Halt:
		return nil
	case CallProperty, Construct, ConstructWithSpread, CallWithSpread, SuspendGenerator, ResumeGenerator, GetIterator, CallIteratorStep, GetIteratorDone, GetIteratorValue:
		return vmError(fmt.Sprintf("%s is not implemented in the Go core VM yet", OpcodeName(op)), currentIP, op)
	default:
		return vmError(fmt.Sprintf("unknown opcode 0x%02X", int(op)), currentIP, op)
	}
	return nil
}

func (vm *RegisterVM) callNative(code CodeObject, instr RegisterInstruction, currentIP int, accumulator *VMValue, registers []VMValue, feedback []FeedbackSlot) error {
	calleeReg, err := operand(instr.Operands, 0)
	if err != nil {
		return vmError(err.Error(), currentIP, instr.Opcode)
	}
	argStart := 1
	argCount := 0
	if len(instr.Operands) > 1 {
		argStart = instr.Operands[1]
	}
	if len(instr.Operands) > 2 {
		argCount = instr.Operands[2]
	}
	callee, err := registerAt(registers, calleeReg)
	if err != nil {
		return vmError(err.Error(), currentIP, instr.Opcode)
	}
	args := make([]VMValue, argCount)
	for i := range args {
		args[i], err = registerAt(registers, argStart+i)
		if err != nil {
			return vmError(err.Error(), currentIP, instr.Opcode)
		}
	}

	var native NativeFunction
	switch fn := callee.(type) {
	case NativeFunction:
		native = fn
	case *VMFunction:
		native = fn.Native
	case VMFunction:
		native = fn.Native
	default:
		return vmError(fmt.Sprintf("cannot call %s", ValueType(callee)), currentIP, instr.Opcode)
	}
	if native == nil {
		return vmError("bytecode function calls are not implemented in the Go core VM yet", currentIP, instr.Opcode)
	}
	RecordCallSite(feedback, feedbackIndex(instr, 3), ValueType(callee))
	result, err := native(args)
	if err != nil {
		return vmError(err.Error(), currentIP, instr.Opcode)
	}
	*accumulator = result
	_ = code
	return nil
}

func operand(operands []int, idx int) (int, error) {
	if idx < 0 || idx >= len(operands) {
		return 0, fmt.Errorf("missing operand %d", idx)
	}
	return operands[idx], nil
}

func depthIndex(operands []int) (int, int, error) {
	depth, err := operand(operands, 0)
	if err != nil {
		return 0, 0, err
	}
	idx, err := operand(operands, 1)
	return depth, idx, err
}

func constantAt(code CodeObject, idx int) (VMValue, error) {
	if idx < 0 || idx >= len(code.Constants) {
		return nil, fmt.Errorf("constant index %d out of range", idx)
	}
	return code.Constants[idx], nil
}

func nameAt(code CodeObject, operands []int, operandIndex int) (string, error) {
	idx, err := operand(operands, operandIndex)
	if err != nil {
		return "", err
	}
	if idx < 0 || idx >= len(code.Names) {
		return "", fmt.Errorf("name index %d out of range", idx)
	}
	return code.Names[idx], nil
}

func registerAt(registers []VMValue, idx int) (VMValue, error) {
	if idx < 0 || idx >= len(registers) {
		return nil, fmt.Errorf("register index %d out of range", idx)
	}
	return registers[idx], nil
}

func setRegister(registers []VMValue, idx int, value VMValue) error {
	if idx < 0 || idx >= len(registers) {
		return fmt.Errorf("register index %d out of range", idx)
	}
	registers[idx] = value
	return nil
}

func feedbackIndex(instr RegisterInstruction, operandIndex int) int {
	if instr.HasFeedbackSlot {
		return instr.FeedbackSlot
	}
	if operandIndex >= 0 && operandIndex < len(instr.Operands) {
		return instr.Operands[operandIndex]
	}
	return -1
}

func binaryOp(op Opcode, left VMValue, right VMValue) (VMValue, error) {
	if op == Add {
		if _, ok := left.(string); ok {
			return stringify(left) + stringify(right), nil
		}
		if _, ok := right.(string); ok {
			return stringify(left) + stringify(right), nil
		}
	}

	leftInt, leftIsInt := toInt(left)
	rightInt, rightIsInt := toInt(right)
	leftNum, leftIsNum := toNumber(left)
	rightNum, rightIsNum := toNumber(right)
	if !leftIsNum || !rightIsNum {
		return nil, fmt.Errorf("%s requires numeric operands", OpcodeName(op))
	}

	switch op {
	case Add:
		if leftIsInt && rightIsInt {
			return leftInt + rightInt, nil
		}
		return leftNum + rightNum, nil
	case Sub:
		if leftIsInt && rightIsInt {
			return leftInt - rightInt, nil
		}
		return leftNum - rightNum, nil
	case Mul:
		if leftIsInt && rightIsInt {
			return leftInt * rightInt, nil
		}
		return leftNum * rightNum, nil
	case Div:
		if rightNum == 0 {
			return nil, fmt.Errorf("division by zero")
		}
		return leftNum / rightNum, nil
	case Mod:
		if rightNum == 0 {
			return nil, fmt.Errorf("modulo by zero")
		}
		if leftIsInt && rightIsInt {
			return leftInt % rightInt, nil
		}
		return math.Mod(leftNum, rightNum), nil
	case Pow:
		result := math.Pow(leftNum, rightNum)
		if leftIsInt && rightIsInt && result == math.Trunc(result) {
			return int(result), nil
		}
		return result, nil
	case BitwiseAnd:
		return leftInt & rightInt, nil
	case BitwiseOr:
		return leftInt | rightInt, nil
	case BitwiseXor:
		return leftInt ^ rightInt, nil
	case ShiftLeft:
		return leftInt << rightInt, nil
	case ShiftRight:
		return leftInt >> rightInt, nil
	case ShiftRightLogical:
		return int(uint32(leftInt) >> uint(rightInt)), nil
	default:
		return nil, fmt.Errorf("unsupported binary operation %s", OpcodeName(op))
	}
}

func compareOp(op Opcode, left VMValue, right VMValue) (bool, error) {
	switch op {
	case TestEqual:
		return looseEqual(left, right), nil
	case TestNotEqual:
		return !looseEqual(left, right), nil
	case TestStrictEqual:
		return strictEqual(left, right), nil
	case TestStrictNotEqual:
		return !strictEqual(left, right), nil
	case TestLessThan, TestGreaterThan, TestLessThanOrEqual, TestGreaterThanOrEqual:
		leftNum, lok := toNumber(left)
		rightNum, rok := toNumber(right)
		if lok && rok {
			switch op {
			case TestLessThan:
				return leftNum < rightNum, nil
			case TestGreaterThan:
				return leftNum > rightNum, nil
			case TestLessThanOrEqual:
				return leftNum <= rightNum, nil
			case TestGreaterThanOrEqual:
				return leftNum >= rightNum, nil
			}
		}
		leftStr, lok := left.(string)
		rightStr, rok := right.(string)
		if lok && rok {
			switch op {
			case TestLessThan:
				return leftStr < rightStr, nil
			case TestGreaterThan:
				return leftStr > rightStr, nil
			case TestLessThanOrEqual:
				return leftStr <= rightStr, nil
			case TestGreaterThanOrEqual:
				return leftStr >= rightStr, nil
			}
		}
		return false, fmt.Errorf("%s requires comparable operands", OpcodeName(op))
	case TestIn:
		key := stringify(left)
		switch obj := right.(type) {
		case *VMObject:
			_, ok := obj.Properties[key]
			return ok, nil
		case VMObject:
			_, ok := obj.Properties[key]
			return ok, nil
		case []VMValue:
			idx, err := strconv.Atoi(key)
			return err == nil && idx >= 0 && idx < len(obj), nil
		case string:
			return contains(obj, key), nil
		default:
			return false, fmt.Errorf("right operand is not searchable")
		}
	case TestInstanceof:
		if right == nil || left == nil {
			return false, nil
		}
		return reflect.TypeOf(left) == reflect.TypeOf(right), nil
	default:
		return false, fmt.Errorf("unsupported comparison %s", OpcodeName(op))
	}
}

func looseEqual(left VMValue, right VMValue) bool {
	if strictEqual(left, right) {
		return true
	}
	leftNum, lok := toNumber(left)
	rightNum, rok := toNumber(right)
	if lok && rok {
		return leftNum == rightNum
	}
	return stringify(left) == stringify(right)
}

func strictEqual(left VMValue, right VMValue) bool {
	if IsUndefined(left) || IsUndefined(right) {
		return IsUndefined(left) && IsUndefined(right)
	}
	if left == nil || right == nil {
		return left == nil && right == nil
	}
	if reflect.TypeOf(left) != reflect.TypeOf(right) {
		return false
	}
	return reflect.DeepEqual(left, right)
}

func toNumber(value VMValue) (float64, bool) {
	switch v := value.(type) {
	case int:
		return float64(v), true
	case int8:
		return float64(v), true
	case int16:
		return float64(v), true
	case int32:
		return float64(v), true
	case int64:
		return float64(v), true
	case uint:
		return float64(v), true
	case uint8:
		return float64(v), true
	case uint16:
		return float64(v), true
	case uint32:
		return float64(v), true
	case uint64:
		return float64(v), true
	case float32:
		return float64(v), true
	case float64:
		return v, true
	default:
		return 0, false
	}
}

func toInt(value VMValue) (int, bool) {
	switch v := value.(type) {
	case int:
		return v, true
	case int8:
		return int(v), true
	case int16:
		return int(v), true
	case int32:
		return int(v), true
	case int64:
		return int(v), true
	case uint:
		return int(v), true
	case uint8:
		return int(v), true
	case uint16:
		return int(v), true
	case uint32:
		return int(v), true
	case uint64:
		return int(v), true
	default:
		return 0, false
	}
}

func truthy(value VMValue) bool {
	switch v := value.(type) {
	case nil:
		return false
	case undefinedValue:
		return false
	case bool:
		return v
	case int:
		return v != 0
	case int8:
		return v != 0
	case int16:
		return v != 0
	case int32:
		return v != 0
	case int64:
		return v != 0
	case uint:
		return v != 0
	case uint8:
		return v != 0
	case uint16:
		return v != 0
	case uint32:
		return v != 0
	case uint64:
		return v != 0
	case float32:
		return v != 0
	case float64:
		return v != 0
	case string:
		return v != ""
	default:
		return true
	}
}

func stringify(value VMValue) string {
	if IsUndefined(value) {
		return "undefined"
	}
	if value == nil {
		return "null"
	}
	return fmt.Sprint(value)
}

func objectAndName(code CodeObject, registers []VMValue, operands []int) (VMValue, string, error) {
	objReg, err := operand(operands, 0)
	if err != nil {
		return nil, "", err
	}
	obj, err := registerAt(registers, objReg)
	if err != nil {
		return nil, "", err
	}
	name, err := nameAt(code, operands, 1)
	return obj, name, err
}

func objectAndKey(registers []VMValue, operands []int) (VMValue, VMValue, error) {
	objReg, err := operand(operands, 0)
	if err != nil {
		return nil, nil, err
	}
	keyReg, err := operand(operands, 1)
	if err != nil {
		return nil, nil, err
	}
	obj, err := registerAt(registers, objReg)
	if err != nil {
		return nil, nil, err
	}
	key, err := registerAt(registers, keyReg)
	return obj, key, err
}

func loadNamedProperty(obj VMValue, name string) (VMValue, error) {
	switch receiver := obj.(type) {
	case *VMObject:
		if value, ok := receiver.Properties[name]; ok {
			return value, nil
		}
		return Undefined, nil
	case VMObject:
		if value, ok := receiver.Properties[name]; ok {
			return value, nil
		}
		return Undefined, nil
	case []VMValue:
		if name == "length" {
			return len(receiver), nil
		}
	}
	return nil, fmt.Errorf("cannot load property %q from %s", name, ValueType(obj))
}

func storeNamedProperty(obj VMValue, name string, value VMValue) error {
	switch receiver := obj.(type) {
	case *VMObject:
		receiver.Properties[name] = value
		return nil
	case VMObject:
		receiver.Properties[name] = value
		return nil
	default:
		return fmt.Errorf("cannot store property %q on %s", name, ValueType(obj))
	}
}

func loadKeyedProperty(obj VMValue, key VMValue) (VMValue, error) {
	switch receiver := obj.(type) {
	case *VMObject:
		if value, ok := receiver.Properties[stringify(key)]; ok {
			return value, nil
		}
		return Undefined, nil
	case VMObject:
		if value, ok := receiver.Properties[stringify(key)]; ok {
			return value, nil
		}
		return Undefined, nil
	case []VMValue:
		idx, ok := toInt(key)
		if ok && idx >= 0 && idx < len(receiver) {
			return receiver[idx], nil
		}
		return Undefined, nil
	default:
		return nil, fmt.Errorf("cannot keyed-load from %s", ValueType(obj))
	}
}

func storeKeyedProperty(obj VMValue, key VMValue, value VMValue) (VMValue, error) {
	switch receiver := obj.(type) {
	case *VMObject:
		receiver.Properties[stringify(key)] = value
		return nil, nil
	case VMObject:
		receiver.Properties[stringify(key)] = value
		return receiver, nil
	case []VMValue:
		idx, ok := toInt(key)
		if !ok || idx < 0 {
			return nil, fmt.Errorf("array key must be a non-negative integer")
		}
		for len(receiver) <= idx {
			receiver = append(receiver, Undefined)
		}
		receiver[idx] = value
		return receiver, nil
	default:
		return nil, fmt.Errorf("cannot keyed-store on %s", ValueType(obj))
	}
}

func deleteProperty(obj VMValue, key VMValue) bool {
	switch receiver := obj.(type) {
	case *VMObject:
		delete(receiver.Properties, stringify(key))
		return true
	case VMObject:
		delete(receiver.Properties, stringify(key))
		return true
	default:
		return false
	}
}

func cloneValue(value VMValue) VMValue {
	switch v := value.(type) {
	case *VMObject:
		cloned := NewObject()
		for key, prop := range v.Properties {
			cloned.Properties[key] = prop
		}
		return cloned
	case VMObject:
		cloned := NewObject()
		for key, prop := range v.Properties {
			cloned.Properties[key] = prop
		}
		return cloned
	case []VMValue:
		return cloneValues(v)
	default:
		return value
	}
}

func cloneValues(values []VMValue) []VMValue {
	copied := make([]VMValue, len(values))
	copy(copied, values)
	return copied
}

func cloneFeedback(slots []FeedbackSlot) []FeedbackSlot {
	copied := make([]FeedbackSlot, len(slots))
	for i, slot := range slots {
		copied[i] = FeedbackSlot{Kind: slot.Kind, Types: copyPairs(slot.Types)}
	}
	return copied
}

func cloneGlobals(globals map[string]VMValue) map[string]VMValue {
	copied := make(map[string]VMValue, len(globals))
	for key, value := range globals {
		copied[key] = value
	}
	return copied
}

func contains(haystack string, needle string) bool {
	if needle == "" {
		return true
	}
	for i := 0; i+len(needle) <= len(haystack); i++ {
		if haystack[i:i+len(needle)] == needle {
			return true
		}
	}
	return false
}

func vmError(message string, instructionIndex int, opcode Opcode) *VMError {
	return &VMError{Message: message, InstructionIndex: instructionIndex, Opcode: opcode}
}
