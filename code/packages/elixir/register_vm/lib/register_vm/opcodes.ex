defmodule CodingAdventures.RegisterVM.Opcodes do
  @moduledoc """
  Opcode constants for the register-based VM.

  ## Design Overview

  Opcodes are grouped into 16-opcode "pages" using the upper nibble:

  | Page  | Range      | Category              |
  |-------|------------|-----------------------|
  | 0x0_  | 0x00–0x06  | Accumulator loads     |
  | 0x1_  | 0x10–0x12  | Register moves        |
  | 0x2_  | 0x20–0x27  | Variable access       |
  | 0x3_  | 0x30–0x3F  | Arithmetic            |
  | 0x4_  | 0x40–0x4C  | Comparisons & logic   |
  | 0x5_  | 0x50–0x58  | Control flow (jumps)  |
  | 0x6_  | 0x60–0x68  | Calls & returns       |
  | 0x7_  | 0x70–0x77  | Property access       |
  | 0x8_  | 0x80–0x85  | Object/array creation |
  | 0x9_  | 0x90–0x93  | Iteration             |
  | 0xA_  | 0xA0–0xA1  | Exceptions            |
  | 0xB_  | 0xB0–0xB5  | Context/scope         |
  | 0xF_  | 0xF0–0xFF  | VM control            |

  This grouping mirrors V8's Ignition bytecode design, where related
  instructions share a common prefix. The lower nibble is the variant
  within each category.

  ## Usage

  All opcodes are exposed as zero-arity public functions so they can be
  used in guard clauses and pattern matches without importing module attributes:

      case instr.opcode do
        op when op == Opcodes.add() -> ...
        op when op == Opcodes.sub() -> ...
      end

  Or with alias:

      alias CodingAdventures.RegisterVM.Opcodes
      Opcodes.lda_constant()  # => 0x00
  """

  # ---------------------------------------------------------------------------
  # 0x0_  Accumulator loads
  # ---------------------------------------------------------------------------
  # These instructions load a value directly into the accumulator.
  # They have no "source register" because the accumulator IS the destination.

  # Load a value from the constants pool at index `operands[0]`
  @lda_constant 0x00
  # Load the integer literal zero — common enough to deserve its own opcode
  @lda_zero 0x01
  # Load a small integer (SMI) embedded directly in the instruction operand
  # V8 calls these "small integers" because they fit in a Smi-tagged pointer
  @lda_smi 0x02
  # Load the special :undefined atom — JavaScript's "not yet assigned" value
  @lda_undefined 0x03
  # Load nil — JavaScript's explicit "no value" (typeof null == "object")
  @lda_null 0x04
  # Load boolean true
  @lda_true 0x05
  # Load boolean false
  @lda_false 0x06

  # ---------------------------------------------------------------------------
  # 0x1_  Register moves
  # ---------------------------------------------------------------------------
  # Transfer values between the accumulator and the register file.

  # Load Accumulator from Register: acc = registers[operands[0]]
  @ldar 0x10
  # Store Accumulator to Register: registers[operands[0]] = acc
  @star 0x11
  # Move between two registers: registers[operands[1]] = registers[operands[0]]
  # Does NOT touch the accumulator
  @mov 0x12

  # ---------------------------------------------------------------------------
  # 0x2_  Variable access
  # ---------------------------------------------------------------------------
  # Read/write named variables in different scopes.
  # The "global" scope is a flat map; "context" is a chain of scope frames.

  # Load a global variable by name: acc = globals[names[operands[0]]]
  @lda_global 0x20
  # Store acc to a global variable: globals[names[operands[0]]] = acc
  @sta_global 0x21
  # Load a local variable from the current scope (convenience alias)
  @lda_local 0x22
  # Store acc to a local variable in the current scope
  @sta_local 0x23
  # Load from a captured variable: walk `operands[0]` links up the context chain,
  # then read slot `operands[1]`
  @lda_context_slot 0x24
  # Store acc to a captured variable at the specified context depth+slot
  @sta_context_slot 0x25
  # Load from slot `operands[0]` in the current (innermost) context
  @lda_current_context_slot 0x26
  # Store acc to slot `operands[0]` in the current context
  @sta_current_context_slot 0x27

  # ---------------------------------------------------------------------------
  # 0x3_  Arithmetic
  # ---------------------------------------------------------------------------
  # Binary operations: always read ONE register as the RIGHT operand.
  # The accumulator provides the LEFT operand and receives the result.
  # Pattern: acc = acc OP registers[operands[0]]
  # Each arithmetic opcode also records type feedback in operands[1].

  # Addition (or string concatenation if either operand is a string)
  @add 0x30
  # Subtraction
  @sub 0x31
  # Multiplication
  @mul 0x32
  # Division (always returns float in Elixir)
  @div 0x33
  # Modulo (remainder)
  @mod 0x34
  # Exponentiation (acc = acc ** registers[r])
  @pow 0x35
  # Add small integer embedded in instruction: acc = acc + operands[0]
  @add_smi 0x36
  # Subtract small integer: acc = acc - operands[0]
  @sub_smi 0x37
  # Bitwise AND: acc = acc &&& registers[r]
  @bitwise_and 0x38
  # Bitwise OR: acc = acc ||| registers[r]
  @bitwise_or 0x39
  # Bitwise XOR: acc = Bitwise.bxor(acc, registers[r])
  @bitwise_xor 0x3A
  # Bitwise NOT (unary): acc = Bitwise.bnot(acc)
  @bitwise_not 0x3B
  # Left shift: acc = acc <<< registers[r]
  @shift_left 0x3C
  # Arithmetic right shift: acc = acc >>> registers[r]
  @shift_right 0x3D
  # Logical right shift (unsigned): in Elixir integers are arbitrary precision
  # so we mask to 32 bits first: acc = (acc >>> r) &&& 0xFFFFFFFF
  @shift_right_logical 0x3E
  # Unary negation: acc = -acc
  @negate 0x3F

  # ---------------------------------------------------------------------------
  # 0x4_  Comparisons and logical tests
  # ---------------------------------------------------------------------------
  # Each comparison stores a boolean result in the accumulator.
  # Binary comparisons read the right operand from a register.

  # Abstract equality (like JS ==): coerces types before comparing
  @test_equal 0x40
  # Abstract inequality
  @test_not_equal 0x41
  # Strict equality (no coercion, like JS ===)
  @test_strict_equal 0x42
  # Strict inequality
  @test_strict_not_equal 0x43
  # acc < registers[r] (numeric comparison)
  @test_less_than 0x44
  # acc > registers[r]
  @test_greater_than 0x45
  # acc <= registers[r]
  @test_less_than_or_equal 0x46
  # acc >= registers[r]
  @test_greater_than_or_equal 0x47
  # Membership test: acc = acc in registers[r] (for lists/maps)
  @test_in 0x48
  # Prototype chain test (simplified: isinstance check)
  @test_instanceof 0x49
  # True if acc is nil or :undefined (undetectable values in JS)
  @test_undetectable 0x4A
  # Logical NOT: acc = !truthy?(acc)
  @logical_not 0x4B
  # Type string: acc = typeof(acc) as a string ("number", "string", etc.)
  @typeof 0x4C

  # ---------------------------------------------------------------------------
  # 0x5_  Control flow
  # ---------------------------------------------------------------------------
  # Jump instructions modify the instruction pointer.
  # Operand is a RELATIVE offset from the instruction AFTER the jump.
  # Positive = forward, negative = backward (for loops).

  # Unconditional jump: ip = ip + 1 + operands[0]
  @jump 0x50
  # Jump if acc is truthy
  @jump_if_true 0x51
  # Jump if acc is falsy
  @jump_if_false 0x52
  # Jump if acc is nil
  @jump_if_null 0x53
  # Jump if acc is :undefined
  @jump_if_undefined 0x54
  # Jump if acc is nil or :undefined
  @jump_if_null_or_undefined 0x55
  # Jump if acc is truthy (after converting to boolean — same as jump_if_true here)
  @jump_if_to_boolean_true 0x56
  # Jump if acc is falsy (after converting to boolean — same as jump_if_false here)
  @jump_if_to_boolean_false 0x57
  # Loop back-edge jump: semantically identical to Jump but signals to a
  # future JIT that this is a loop header (triggers OSR — on-stack replacement)
  @jump_loop 0x58

  # ---------------------------------------------------------------------------
  # 0x6_  Calls and returns
  # ---------------------------------------------------------------------------

  # Call a function with any receiver:
  # operands = [func_reg, first_arg_reg, argc, feedback_slot]
  @call_any_receiver 0x60
  # Call a method on an object (receiver in first_arg_reg):
  # operands = [func_reg, recv_reg, argc, feedback_slot]
  @call_property 0x61
  # Call a function with undefined as receiver
  @call_undefined_receiver 0x62
  # Construct (new): operands = [constructor_reg, first_arg_reg, argc, feedback_slot]
  @construct 0x63
  # Construct with spread argument
  @construct_with_spread 0x64
  # Call with spread argument
  @call_with_spread 0x65
  # Return acc to the caller
  @return 0x66
  # Suspend a generator function (save state, yield value)
  @suspend_generator 0x67
  # Resume a suspended generator
  @resume_generator 0x68

  # ---------------------------------------------------------------------------
  # 0x7_  Property access
  # ---------------------------------------------------------------------------

  # Load named property: acc = registers[operands[0]][names[operands[1]]]
  # operands[2] = feedback_slot for hidden class tracking
  @lda_named_property 0x70
  # Store acc to named property: registers[obj_reg][names[name_idx]] = acc
  @sta_named_property 0x71
  # Load keyed property: key = acc; acc = registers[operands[0]][key]
  @lda_keyed_property 0x72
  # Store keyed property: registers[obj_reg][registers[key_reg]] = acc
  @sta_keyed_property 0x73
  # Like lda_named_property but no feedback slot (e.g., for known-type fast paths)
  @lda_named_property_no_feedback 0x74
  # Like sta_named_property but no feedback slot
  @sta_named_property_no_feedback 0x75
  # Delete a property in strict mode (throws TypeError if not configurable)
  @delete_property_strict 0x76
  # Delete a property in sloppy mode (returns false if not configurable)
  @delete_property_sloppy 0x77

  # ---------------------------------------------------------------------------
  # 0x8_  Object and array creation
  # ---------------------------------------------------------------------------

  # Create an object literal. operands[0] = template constant index (ignored here,
  # we just create %{}), operands[1] = feedback_slot, operands[2] = flags
  @create_object_literal 0x80
  # Create an array literal: creates [] (Elixir list)
  @create_array_literal 0x81
  # Create a regex literal (represented as a string pattern)
  @create_regexp_literal 0x82
  # Create a closure: wraps a CodeObject with the current context
  # Result is a {:function, code, context} tuple
  @create_closure 0x83
  # Push a new context frame for block scoping
  @create_context 0x84
  # Clone an existing object (shallow copy via Map.merge)
  @clone_object 0x85

  # ---------------------------------------------------------------------------
  # 0x9_  Iteration (for-of / for-in)
  # ---------------------------------------------------------------------------

  # Get an iterator from the accumulator (for lists: acc = {list, :iterator})
  @get_iterator 0x90
  # Advance the iterator: acc = next value, or :iterator_done sentinel
  @call_iterator_step 0x91
  # Test if iteration is complete: acc = (iterator.done == true)
  @get_iterator_done 0x92
  # Get the current iterator value: acc = iterator.value
  @get_iterator_value 0x93

  # ---------------------------------------------------------------------------
  # 0xA_  Exceptions
  # ---------------------------------------------------------------------------

  # Throw acc as an exception: creates a VMError with acc as the message
  @throw 0xA0
  # Rethrow the current exception (used in catch blocks)
  @rethrow 0xA1

  # ---------------------------------------------------------------------------
  # 0xB_  Context / scope management
  # ---------------------------------------------------------------------------

  # Push a new scope context (for function entry or block scope)
  @push_context 0xB0
  # Pop the current scope context (restore parent)
  @pop_context 0xB1
  # Load a module-level variable by index
  @lda_module_variable 0xB4
  # Store acc to a module-level variable
  @sta_module_variable 0xB5

  # ---------------------------------------------------------------------------
  # 0xF_  VM control
  # ---------------------------------------------------------------------------

  # Check if the call stack has exceeded the depth limit.
  # Generates a stack overflow error if so.
  @stack_check 0xF0
  # Breakpoint / debugger placeholder. No-op in the interpreter; a debugger
  # could hook here to pause execution.
  @debugger 0xF1
  # Unconditional halt: stop execution and return the current accumulator value.
  # Useful as the final instruction in a top-level script.
  @halt 0xFF

  # ---------------------------------------------------------------------------
  # Public accessor functions
  # ---------------------------------------------------------------------------
  # We expose all opcodes as zero-arity functions rather than raw module
  # attributes. This lets callers use them in pattern matches, guards, and
  # case expressions without needing to know the numeric value.

  def lda_constant, do: @lda_constant
  def lda_zero, do: @lda_zero
  def lda_smi, do: @lda_smi
  def lda_undefined, do: @lda_undefined
  def lda_null, do: @lda_null
  def lda_true, do: @lda_true
  def lda_false, do: @lda_false

  def ldar, do: @ldar
  def star, do: @star
  def mov, do: @mov

  def lda_global, do: @lda_global
  def sta_global, do: @sta_global
  def lda_local, do: @lda_local
  def sta_local, do: @sta_local
  def lda_context_slot, do: @lda_context_slot
  def sta_context_slot, do: @sta_context_slot
  def lda_current_context_slot, do: @lda_current_context_slot
  def sta_current_context_slot, do: @sta_current_context_slot

  def add, do: @add
  def sub, do: @sub
  def mul, do: @mul
  def div, do: @div
  def mod, do: @mod
  def pow, do: @pow
  def add_smi, do: @add_smi
  def sub_smi, do: @sub_smi
  def bitwise_and, do: @bitwise_and
  def bitwise_or, do: @bitwise_or
  def bitwise_xor, do: @bitwise_xor
  def bitwise_not, do: @bitwise_not
  def shift_left, do: @shift_left
  def shift_right, do: @shift_right
  def shift_right_logical, do: @shift_right_logical
  def negate, do: @negate

  def test_equal, do: @test_equal
  def test_not_equal, do: @test_not_equal
  def test_strict_equal, do: @test_strict_equal
  def test_strict_not_equal, do: @test_strict_not_equal
  def test_less_than, do: @test_less_than
  def test_greater_than, do: @test_greater_than
  def test_less_than_or_equal, do: @test_less_than_or_equal
  def test_greater_than_or_equal, do: @test_greater_than_or_equal
  def test_in, do: @test_in
  def test_instanceof, do: @test_instanceof
  def test_undetectable, do: @test_undetectable
  def logical_not, do: @logical_not
  def typeof, do: @typeof

  def jump, do: @jump
  def jump_if_true, do: @jump_if_true
  def jump_if_false, do: @jump_if_false
  def jump_if_null, do: @jump_if_null
  def jump_if_undefined, do: @jump_if_undefined
  def jump_if_null_or_undefined, do: @jump_if_null_or_undefined
  def jump_if_to_boolean_true, do: @jump_if_to_boolean_true
  def jump_if_to_boolean_false, do: @jump_if_to_boolean_false
  def jump_loop, do: @jump_loop

  def call_any_receiver, do: @call_any_receiver
  def call_property, do: @call_property
  def call_undefined_receiver, do: @call_undefined_receiver
  def construct, do: @construct
  def construct_with_spread, do: @construct_with_spread
  def call_with_spread, do: @call_with_spread
  def return, do: @return
  def suspend_generator, do: @suspend_generator
  def resume_generator, do: @resume_generator

  def lda_named_property, do: @lda_named_property
  def sta_named_property, do: @sta_named_property
  def lda_keyed_property, do: @lda_keyed_property
  def sta_keyed_property, do: @sta_keyed_property
  def lda_named_property_no_feedback, do: @lda_named_property_no_feedback
  def sta_named_property_no_feedback, do: @sta_named_property_no_feedback
  def delete_property_strict, do: @delete_property_strict
  def delete_property_sloppy, do: @delete_property_sloppy

  def create_object_literal, do: @create_object_literal
  def create_array_literal, do: @create_array_literal
  def create_regexp_literal, do: @create_regexp_literal
  def create_closure, do: @create_closure
  def create_context, do: @create_context
  def clone_object, do: @clone_object

  def get_iterator, do: @get_iterator
  def call_iterator_step, do: @call_iterator_step
  def get_iterator_done, do: @get_iterator_done
  def get_iterator_value, do: @get_iterator_value

  def throw, do: @throw
  def rethrow, do: @rethrow

  def push_context, do: @push_context
  def pop_context, do: @pop_context
  def lda_module_variable, do: @lda_module_variable
  def sta_module_variable, do: @sta_module_variable

  def stack_check, do: @stack_check
  def debugger, do: @debugger
  def halt, do: @halt

  @doc """
  Returns the human-readable name for an opcode integer.

  ## Examples

      iex> Opcodes.name(0x00)
      "LdaConstant"
      iex> Opcodes.name(0x30)
      "Add"
      iex> Opcodes.name(0xFF)
      "Halt"
      iex> Opcodes.name(0xAB)
      "Unknown(0xAB)"
  """
  def name(opcode) do
    case opcode do
      @lda_constant -> "LdaConstant"
      @lda_zero -> "LdaZero"
      @lda_smi -> "LdaSmi"
      @lda_undefined -> "LdaUndefined"
      @lda_null -> "LdaNull"
      @lda_true -> "LdaTrue"
      @lda_false -> "LdaFalse"
      @ldar -> "Ldar"
      @star -> "Star"
      @mov -> "Mov"
      @lda_global -> "LdaGlobal"
      @sta_global -> "StaGlobal"
      @lda_local -> "LdaLocal"
      @sta_local -> "StaLocal"
      @lda_context_slot -> "LdaContextSlot"
      @sta_context_slot -> "StaContextSlot"
      @lda_current_context_slot -> "LdaCurrentContextSlot"
      @sta_current_context_slot -> "StaCurrentContextSlot"
      @add -> "Add"
      @sub -> "Sub"
      @mul -> "Mul"
      @div -> "Div"
      @mod -> "Mod"
      @pow -> "Pow"
      @add_smi -> "AddSmi"
      @sub_smi -> "SubSmi"
      @bitwise_and -> "BitwiseAnd"
      @bitwise_or -> "BitwiseOr"
      @bitwise_xor -> "BitwiseXor"
      @bitwise_not -> "BitwiseNot"
      @shift_left -> "ShiftLeft"
      @shift_right -> "ShiftRight"
      @shift_right_logical -> "ShiftRightLogical"
      @negate -> "Negate"
      @test_equal -> "TestEqual"
      @test_not_equal -> "TestNotEqual"
      @test_strict_equal -> "TestStrictEqual"
      @test_strict_not_equal -> "TestStrictNotEqual"
      @test_less_than -> "TestLessThan"
      @test_greater_than -> "TestGreaterThan"
      @test_less_than_or_equal -> "TestLessThanOrEqual"
      @test_greater_than_or_equal -> "TestGreaterThanOrEqual"
      @test_in -> "TestIn"
      @test_instanceof -> "TestInstanceOf"
      @test_undetectable -> "TestUndetectable"
      @logical_not -> "LogicalNot"
      @typeof -> "TypeOf"
      @jump -> "Jump"
      @jump_if_true -> "JumpIfTrue"
      @jump_if_false -> "JumpIfFalse"
      @jump_if_null -> "JumpIfNull"
      @jump_if_undefined -> "JumpIfUndefined"
      @jump_if_null_or_undefined -> "JumpIfNullOrUndefined"
      @jump_if_to_boolean_true -> "JumpIfToBooleanTrue"
      @jump_if_to_boolean_false -> "JumpIfToBooleanFalse"
      @jump_loop -> "JumpLoop"
      @call_any_receiver -> "CallAnyReceiver"
      @call_property -> "CallProperty"
      @call_undefined_receiver -> "CallUndefinedReceiver"
      @construct -> "Construct"
      @construct_with_spread -> "ConstructWithSpread"
      @call_with_spread -> "CallWithSpread"
      @return -> "Return"
      @suspend_generator -> "SuspendGenerator"
      @resume_generator -> "ResumeGenerator"
      @lda_named_property -> "LdaNamedProperty"
      @sta_named_property -> "StaNamedProperty"
      @lda_keyed_property -> "LdaKeyedProperty"
      @sta_keyed_property -> "StaKeyedProperty"
      @lda_named_property_no_feedback -> "LdaNamedPropertyNoFeedback"
      @sta_named_property_no_feedback -> "StaNamedPropertyNoFeedback"
      @delete_property_strict -> "DeletePropertyStrict"
      @delete_property_sloppy -> "DeletePropertySloppy"
      @create_object_literal -> "CreateObjectLiteral"
      @create_array_literal -> "CreateArrayLiteral"
      @create_regexp_literal -> "CreateRegExpLiteral"
      @create_closure -> "CreateClosure"
      @create_context -> "CreateContext"
      @clone_object -> "CloneObject"
      @get_iterator -> "GetIterator"
      @call_iterator_step -> "CallIteratorStep"
      @get_iterator_done -> "GetIteratorDone"
      @get_iterator_value -> "GetIteratorValue"
      @throw -> "Throw"
      @rethrow -> "Rethrow"
      @push_context -> "PushContext"
      @pop_context -> "PopContext"
      @lda_module_variable -> "LdaModuleVariable"
      @sta_module_variable -> "StaModuleVariable"
      @stack_check -> "StackCheck"
      @debugger -> "Debugger"
      @halt -> "Halt"
      _ -> "Unknown(0x#{Integer.to_string(opcode, 16)})"
    end
  end
end
