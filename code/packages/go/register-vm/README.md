# Register VM (Go)

A register-based bytecode interpreter modeled after the V8 Ignition-style VM used by the other language packages in this repository.

The Go package currently focuses on the portable core: opcodes, bytecode data structures, inline-cache feedback, lexical contexts, property/object helpers, trace capture, and deterministic execution for arithmetic, control-flow, globals, context slots, and object operations. More advanced closure, spread call, generator, and native-extension behavior can be layered on top in later parity batches where Go can either host native implementations or call into Rust-backed packages.

## Example

```go
code := registervm.CodeObject{
	Instructions: []registervm.RegisterInstruction{
		{Opcode: registervm.LdaConstant, Operands: []int{0}},
		{Opcode: registervm.Star, Operands: []int{0}},
		{Opcode: registervm.LdaConstant, Operands: []int{1}},
		{Opcode: registervm.Add, Operands: []int{0, 0}},
		{Opcode: registervm.Halt},
	},
	Constants:         []registervm.VMValue{40, 2},
	RegisterCount:     1,
	FeedbackSlotCount: 1,
}

result, err := registervm.NewRegisterVM().Execute(code)
// result.Value == 42
```
