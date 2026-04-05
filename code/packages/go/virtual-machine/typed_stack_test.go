package virtualmachine

import "testing"

// ════════════════════════════════════════════════════════════════════════
// TYPED STACK TESTS
// ════════════════════════════════════════════════════════════════════════

func TestTypedStackPushPop(t *testing.T) {
	vm := NewGenericVM()

	// Push two typed values.
	vm.PushTyped(TypedVMValue{Type: 0x7F, Value: int32(42)})
	vm.PushTyped(TypedVMValue{Type: 0x7E, Value: int64(100)})

	if len(vm.TypedStack) != 2 {
		t.Fatalf("expected typed stack length 2, got %d", len(vm.TypedStack))
	}

	// Pop in LIFO order.
	v := vm.PopTyped()
	if v.Type != 0x7E || v.Value.(int64) != 100 {
		t.Fatalf("expected i64(100), got type=0x%02x value=%v", v.Type, v.Value)
	}

	v = vm.PopTyped()
	if v.Type != 0x7F || v.Value.(int32) != 42 {
		t.Fatalf("expected i32(42), got type=0x%02x value=%v", v.Type, v.Value)
	}
}

func TestTypedStackPeek(t *testing.T) {
	vm := NewGenericVM()
	vm.PushTyped(TypedVMValue{Type: 0x7F, Value: int32(7)})

	v := vm.PeekTyped()
	if v.Type != 0x7F || v.Value.(int32) != 7 {
		t.Fatalf("PeekTyped returned wrong value")
	}

	// Stack should still have the value.
	if len(vm.TypedStack) != 1 {
		t.Fatalf("PeekTyped should not remove the value")
	}
}

func TestTypedStackUnderflow(t *testing.T) {
	vm := NewGenericVM()
	defer func() {
		r := recover()
		if r == nil {
			t.Fatal("expected panic on empty typed stack pop")
		}
		if r != "TypedStackUnderflowError" {
			t.Fatalf("expected TypedStackUnderflowError, got %v", r)
		}
	}()
	vm.PopTyped()
}

func TestTypedStackPeekUnderflow(t *testing.T) {
	vm := NewGenericVM()
	defer func() {
		r := recover()
		if r == nil {
			t.Fatal("expected panic on empty typed stack peek")
		}
	}()
	vm.PeekTyped()
}

// ════════════════════════════════════════════════════════════════════════
// CONTEXT OPCODE REGISTRATION
// ════════════════════════════════════════════════════════════════════════

func TestRegisterContextOpcode(t *testing.T) {
	vm := NewGenericVM()

	called := false
	vm.RegisterContextOpcode(OpCode(0x41), func(vm *GenericVM, instr Instruction, code CodeObject, ctx interface{}) *string {
		called = true
		vm.AdvancePC()
		return nil
	})

	code := CodeObject{
		Instructions: []Instruction{
			{Opcode: OpCode(0x41), Operand: int32(5)},
		},
	}

	vm.ExecuteWithContext(code, nil)

	if !called {
		t.Fatal("context handler was not called")
	}
}

func TestContextHandlerReceivesContext(t *testing.T) {
	vm := NewGenericVM()

	type TestContext struct {
		Value int
	}

	var receivedCtx *TestContext
	vm.RegisterContextOpcode(OpCode(0x41), func(vm *GenericVM, instr Instruction, code CodeObject, ctx interface{}) *string {
		receivedCtx = ctx.(*TestContext)
		vm.Halted = true
		return nil
	})

	code := CodeObject{
		Instructions: []Instruction{
			{Opcode: OpCode(0x41), Operand: nil},
		},
	}

	testCtx := &TestContext{Value: 99}
	vm.ExecuteWithContext(code, testCtx)

	if receivedCtx == nil || receivedCtx.Value != 99 {
		t.Fatal("context handler did not receive correct context")
	}
}

// ════════════════════════════════════════════════════════════════════════
// HOOKS
// ════════════════════════════════════════════════════════════════════════

func TestPreInstructionHook(t *testing.T) {
	vm := NewGenericVM()

	hookCalled := false
	vm.SetPreInstructionHook(func(vm *GenericVM, instr *Instruction, code CodeObject) {
		hookCalled = true
	})

	vm.RegisterContextOpcode(OpCode(0x01), func(vm *GenericVM, instr Instruction, code CodeObject, ctx interface{}) *string {
		vm.Halted = true
		return nil
	})

	code := CodeObject{
		Instructions: []Instruction{
			{Opcode: OpCode(0x01)},
		},
	}

	vm.ExecuteWithContext(code, nil)

	if !hookCalled {
		t.Fatal("pre-instruction hook was not called")
	}
}

func TestPostInstructionHook(t *testing.T) {
	vm := NewGenericVM()

	hookCalled := false
	vm.SetPostInstructionHook(func(vm *GenericVM, instr Instruction, code CodeObject) {
		hookCalled = true
	})

	vm.RegisterContextOpcode(OpCode(0x01), func(vm *GenericVM, instr Instruction, code CodeObject, ctx interface{}) *string {
		vm.Halted = true
		return nil
	})

	code := CodeObject{
		Instructions: []Instruction{
			{Opcode: OpCode(0x01)},
		},
	}

	vm.ExecuteWithContext(code, nil)

	if !hookCalled {
		t.Fatal("post-instruction hook was not called")
	}
}

// ════════════════════════════════════════════════════════════════════════
// RESET CLEARS TYPED STATE
// ════════════════════════════════════════════════════════════════════════

func TestResetClearsTypedStack(t *testing.T) {
	vm := NewGenericVM()
	vm.PushTyped(TypedVMValue{Type: 0x7F, Value: int32(1)})
	vm.ExecutionContext = "test"

	vm.Reset()

	if len(vm.TypedStack) != 0 {
		t.Fatal("Reset should clear typed stack")
	}
	if vm.ExecutionContext != nil {
		t.Fatal("Reset should clear execution context")
	}
}

// ════════════════════════════════════════════════════════════════════════
// CONTEXT HANDLER FALLBACK TO REGULAR HANDLER
// ════════════════════════════════════════════════════════════════════════

func TestContextFallsBackToRegularHandler(t *testing.T) {
	vm := NewGenericVM()

	regularCalled := false
	vm.RegisterOpcode(OpCode(0x01), func(vm *GenericVM, instr Instruction, code CodeObject) *string {
		regularCalled = true
		vm.Halted = true
		return nil
	})

	code := CodeObject{
		Instructions: []Instruction{
			{Opcode: OpCode(0x01)},
		},
	}

	vm.ExecuteWithContext(code, nil)

	if !regularCalled {
		t.Fatal("should fall back to regular handler when no context handler exists")
	}
}
