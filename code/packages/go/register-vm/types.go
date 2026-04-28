package registervm

import "fmt"

// VMValue is the dynamic value type carried by registers and the accumulator.
type VMValue any

type undefinedValue struct{}

// Undefined represents JavaScript's undefined sentinel, distinct from nil/null.
var Undefined VMValue = undefinedValue{}

// IsUndefined reports whether v is the VM undefined sentinel.
func IsUndefined(v VMValue) bool {
	_, ok := v.(undefinedValue)
	return ok
}

func (undefinedValue) String() string {
	return "undefined"
}

// VMObject is a heap object with a shape identifier and string-keyed storage.
type VMObject struct {
	HiddenClassID int
	Properties    map[string]VMValue
}

// NewObject creates an object with a fresh hidden class id.
func NewObject() *VMObject {
	return ObjectWithHiddenClass(NewHiddenClassID())
}

// ObjectWithHiddenClass creates an object using an explicit hidden class id.
func ObjectWithHiddenClass(id int) *VMObject {
	return &VMObject{HiddenClassID: id, Properties: map[string]VMValue{}}
}

// NativeFunction is a host function callable by the VM's simple call opcodes.
type NativeFunction func(args []VMValue) (VMValue, error)

// VMFunction is a closure value made from bytecode plus a captured context.
type VMFunction struct {
	Code    CodeObject
	Context *Context
	Native  NativeFunction
}

// CodeObject is one compiled bytecode unit.
type CodeObject struct {
	Instructions      []RegisterInstruction
	Constants         []VMValue
	Names             []string
	RegisterCount     int
	FeedbackSlotCount int
	ParameterCount    int
	Name              string
}

// RegisterInstruction is one opcode plus integer operands.
type RegisterInstruction struct {
	Opcode          Opcode
	Operands        []int
	FeedbackSlot    int
	HasFeedbackSlot bool
}

func (i RegisterInstruction) String() string {
	if len(i.Operands) == 0 {
		return OpcodeName(i.Opcode)
	}
	return fmt.Sprintf("%s %v", OpcodeName(i.Opcode), i.Operands)
}

// Context stores lexical closure slots and a parent link.
type Context struct {
	Slots  []VMValue
	Parent *Context
}

// TraceStep captures the state transition for one executed instruction.
type TraceStep struct {
	IP                int
	Instruction       RegisterInstruction
	AccumulatorBefore VMValue
	AccumulatorAfter  VMValue
	RegistersBefore   []VMValue
	RegistersAfter    []VMValue
	FeedbackBefore    []FeedbackSlot
	FeedbackAfter     []FeedbackSlot
}

// VMResult is returned after successful execution.
type VMResult struct {
	Value          VMValue
	Globals        map[string]VMValue
	FeedbackVector []FeedbackSlot
	Trace          []TraceStep
}

// VMError describes a bytecode execution failure.
type VMError struct {
	Message          string
	InstructionIndex int
	Opcode           Opcode
}

func (e *VMError) Error() string {
	if e == nil {
		return ""
	}
	if e.Opcode != 0 || e.InstructionIndex >= 0 {
		return fmt.Sprintf("%s at instruction %d (%s)", e.Message, e.InstructionIndex, OpcodeName(e.Opcode))
	}
	return e.Message
}
