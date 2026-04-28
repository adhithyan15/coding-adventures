package registervm

import "fmt"

// FeedbackKind is the inline-cache state for one feedback slot.
type FeedbackKind string

const (
	FeedbackUninitialized FeedbackKind = "uninitialized"
	FeedbackMonomorphic   FeedbackKind = "monomorphic"
	FeedbackPolymorphic   FeedbackKind = "polymorphic"
	FeedbackMegamorphic   FeedbackKind = "megamorphic"
)

// TypePair records the pair of observed operand types at an inline-cache site.
type TypePair struct {
	Left  string
	Right string
}

// FeedbackSlot stores the current state of one inline-cache slot.
type FeedbackSlot struct {
	Kind  FeedbackKind
	Types []TypePair
}

var nextHiddenClassID int

// NewHiddenClassID returns a process-local shape id for VMObject instances.
func NewHiddenClassID() int {
	id := nextHiddenClassID
	nextHiddenClassID++
	return id
}

// NewVector creates a feedback vector initialized to uninitialized slots.
func NewVector(size int) []FeedbackSlot {
	if size <= 0 {
		return []FeedbackSlot{}
	}
	vector := make([]FeedbackSlot, size)
	for i := range vector {
		vector[i] = FeedbackSlot{Kind: FeedbackUninitialized}
	}
	return vector
}

// ValueType returns a JavaScript-style type name for a VM value.
func ValueType(v VMValue) string {
	switch v.(type) {
	case bool:
		return "boolean"
	case int, int8, int16, int32, int64, uint, uint8, uint16, uint32, uint64, float32, float64:
		return "number"
	case string:
		return "string"
	case undefinedValue:
		return "undefined"
	case *VMObject, VMObject:
		return "object"
	case []VMValue, []any:
		return "array"
	case *VMFunction, VMFunction, NativeFunction:
		return "function"
	case nil:
		return "null"
	default:
		return "unknown"
	}
}

// RecordBinaryOp records the operand types for a binary operation.
func RecordBinaryOp(vector []FeedbackSlot, slot int, left VMValue, right VMValue) {
	if slot < 0 || slot >= len(vector) {
		return
	}
	vector[slot] = updateSlot(vector[slot], TypePair{Left: ValueType(left), Right: ValueType(right)})
}

// RecordPropertyLoad records object shape feedback for a property access.
func RecordPropertyLoad(vector []FeedbackSlot, slot int, hiddenClassID int) {
	if slot < 0 || slot >= len(vector) {
		return
	}
	pair := TypePair{Left: fmt.Sprintf("object_%d", hiddenClassID), Right: "property"}
	vector[slot] = updateSlot(vector[slot], pair)
}

// RecordCallSite records callee feedback for a call site.
func RecordCallSite(vector []FeedbackSlot, slot int, calleeType string) {
	if slot < 0 || slot >= len(vector) {
		return
	}
	vector[slot] = updateSlot(vector[slot], TypePair{Left: calleeType, Right: "call"})
}

func updateSlot(current FeedbackSlot, pair TypePair) FeedbackSlot {
	switch current.Kind {
	case "", FeedbackUninitialized:
		return FeedbackSlot{Kind: FeedbackMonomorphic, Types: []TypePair{pair}}
	case FeedbackMonomorphic:
		if hasPair(current.Types, pair) {
			return current
		}
		return FeedbackSlot{Kind: FeedbackPolymorphic, Types: append(copyPairs(current.Types), pair)}
	case FeedbackPolymorphic:
		if hasPair(current.Types, pair) {
			return current
		}
		if len(current.Types) >= 4 {
			return FeedbackSlot{Kind: FeedbackMegamorphic}
		}
		return FeedbackSlot{Kind: FeedbackPolymorphic, Types: append(copyPairs(current.Types), pair)}
	case FeedbackMegamorphic:
		return current
	default:
		return FeedbackSlot{Kind: FeedbackMonomorphic, Types: []TypePair{pair}}
	}
}

func hasPair(pairs []TypePair, pair TypePair) bool {
	for _, existing := range pairs {
		if existing == pair {
			return true
		}
	}
	return false
}

func copyPairs(pairs []TypePair) []TypePair {
	copied := make([]TypePair, len(pairs))
	copy(copied, pairs)
	return copied
}
