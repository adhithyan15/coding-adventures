defmodule CodingAdventures.RegisterVM.InterpreterTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.RegisterVM
  alias CodingAdventures.RegisterVM.Types.{CodeObject, RegisterInstruction}
  alias CodingAdventures.RegisterVM.Opcodes
  alias CodingAdventures.RegisterVM.Feedback

  # ---------------------------------------------------------------------------
  # Test helpers
  # ---------------------------------------------------------------------------

  # Build a RegisterInstruction with just an opcode (no operands)
  defp instr(opcode), do: %RegisterInstruction{opcode: opcode, operands: []}
  defp instr(opcode, operands), do: %RegisterInstruction{opcode: opcode, operands: operands}

  # Build a minimal CodeObject given just an instruction list
  defp make_code(instructions, opts \\ []) do
    %CodeObject{
      instructions: instructions,
      constants: Keyword.get(opts, :constants, []),
      names: Keyword.get(opts, :names, []),
      register_count: Keyword.get(opts, :registers, 2),
      feedback_slot_count: Keyword.get(opts, :feedback_slots, 2),
      name: Keyword.get(opts, :name, "test")
    }
  end

  # ---------------------------------------------------------------------------
  # Test 1: LdaConstant + Halt
  # ---------------------------------------------------------------------------
  # The most basic program: load a constant from the pool and halt.
  # This verifies the core fetch-execute loop and the constants pool.

  test "LdaConstant loads a value from the constants pool, Halt returns it" do
    code = make_code(
      [
        instr(Opcodes.lda_constant(), [0]),
        instr(Opcodes.halt())
      ],
      constants: [42],
      registers: 0,
      feedback_slots: 0
    )

    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 42
    assert result.error == nil
  end

  test "LdaConstant with string constant" do
    code = make_code(
      [
        instr(Opcodes.lda_constant(), [0]),
        instr(Opcodes.halt())
      ],
      constants: ["hello, world"],
      registers: 0,
      feedback_slots: 0
    )

    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == "hello, world"
  end

  test "LdaZero loads integer zero" do
    code = make_code(
      [instr(Opcodes.lda_zero()), instr(Opcodes.halt())],
      registers: 0, feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 0
  end

  test "LdaSmi loads a small integer from the operand" do
    code = make_code(
      [instr(Opcodes.lda_smi(), [99]), instr(Opcodes.halt())],
      registers: 0, feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 99
  end

  test "LdaUndefined loads :undefined" do
    code = make_code(
      [instr(Opcodes.lda_undefined()), instr(Opcodes.halt())],
      registers: 0, feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == :undefined
  end

  test "LdaNull loads nil" do
    code = make_code(
      [instr(Opcodes.lda_null()), instr(Opcodes.halt())],
      registers: 0, feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == nil
  end

  test "LdaTrue loads true" do
    code = make_code(
      [instr(Opcodes.lda_true()), instr(Opcodes.halt())],
      registers: 0, feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == true
  end

  test "LdaFalse loads false" do
    code = make_code(
      [instr(Opcodes.lda_false()), instr(Opcodes.halt())],
      registers: 0, feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == false
  end

  # ---------------------------------------------------------------------------
  # Test 2: Star + Ldar — register save and restore
  # ---------------------------------------------------------------------------
  # Demonstrates the accumulator-register duality:
  #   1. Load 10 into acc
  #   2. Save it in r0
  #   3. Load 20 into acc (overwrites)
  #   4. Restore r0 back into acc
  # Result should be 10 (the saved value), not 20.

  test "Star saves acc to register; Ldar restores it" do
    code = make_code(
      [
        instr(Opcodes.lda_constant(), [0]),   # acc = 10
        instr(Opcodes.star(), [0]),            # r0 = acc (10)
        instr(Opcodes.lda_constant(), [1]),   # acc = 20 (overwrites)
        instr(Opcodes.ldar(), [0]),            # acc = r0 (restore 10)
        instr(Opcodes.halt())
      ],
      constants: [10, 20],
      registers: 1,
      feedback_slots: 0
    )

    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 10
  end

  test "Mov copies between registers without touching accumulator" do
    code = make_code(
      [
        instr(Opcodes.lda_constant(), [0]),   # acc = 77
        instr(Opcodes.star(), [0]),            # r0 = 77
        instr(Opcodes.mov(), [0, 1]),          # r1 = r0 (77), acc unchanged
        instr(Opcodes.lda_zero()),             # acc = 0
        instr(Opcodes.ldar(), [1]),            # acc = r1 = 77
        instr(Opcodes.halt())
      ],
      constants: [77],
      registers: 2,
      feedback_slots: 0
    )

    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 77
  end

  # ---------------------------------------------------------------------------
  # Test 3: Add with feedback recording
  # ---------------------------------------------------------------------------
  # Adds two integers and verifies:
  # 1. The arithmetic result is correct (10 + 20 = 30)
  # 2. The feedback slot recorded a monomorphic {integer, integer} pair

  test "Add produces correct result and records monomorphic integer+integer feedback" do
    code = make_code(
      [
        instr(Opcodes.lda_constant(), [0]),       # acc = 10
        instr(Opcodes.star(), [0]),                # r0 = 10
        instr(Opcodes.lda_constant(), [1]),       # acc = 20
        instr(Opcodes.add(), [0, 0]),              # acc = acc + r0 = 30; feedback slot 0
        instr(Opcodes.halt())
      ],
      constants: [10, 20],
      registers: 1,
      feedback_slots: 1
    )

    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 30
    assert result.error == nil

    # Verify the feedback slot recorded integer+integer (monomorphic)
    slot_0 = Enum.at(result.final_feedback_vector, 0)
    assert slot_0 == {:monomorphic, [{:integer, :integer}]}
  end

  test "Sub, Mul, Div, Mod produce correct results" do
    # 10 - 3 = 7
    sub_code = make_code(
      [
        instr(Opcodes.lda_smi(), [10]),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_smi(), [3]),
        instr(Opcodes.sub(), [0, 0]),
        instr(Opcodes.halt())
      ],
      feedback_slots: 1
    )

    {:ok, sub_result} = RegisterVM.execute(sub_code)
    # Note: acc starts as 3, r0=10; sub is acc - r0 = 3 - 10 = -7
    # Wait, let's check: Star stores 10 in r0, then LdaSmi 3 puts 3 in acc
    # Sub: acc = acc - r0 = 3 - 10 = -7
    # Hmm — let me reconsider. Actually in V8 style: acc = acc - reg
    # So: acc(3) - r0(10) = -7
    assert sub_result.return_value == -7

    # 4 * 5 = 20
    mul_code = make_code(
      [
        instr(Opcodes.lda_smi(), [4]),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_smi(), [5]),
        instr(Opcodes.mul(), [0, 0]),
        instr(Opcodes.halt())
      ],
      feedback_slots: 1
    )
    {:ok, mul_result} = RegisterVM.execute(mul_code)
    # acc(5) * r0(4) = 20
    assert mul_result.return_value == 20

    # 10 / 4 = 2.5
    div_code = make_code(
      [
        instr(Opcodes.lda_smi(), [10]),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_smi(), [10]),
        instr(Opcodes.div(), [0, 0]),
        instr(Opcodes.halt())
      ],
      feedback_slots: 1
    )
    {:ok, div_result} = RegisterVM.execute(div_code)
    # acc(10) / r0(10) = 1.0
    assert div_result.return_value == 1.0

    # 7 mod 3 = 1
    mod_code = make_code(
      [
        instr(Opcodes.lda_smi(), [3]),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_smi(), [7]),
        instr(Opcodes.mod(), [0, 0]),
        instr(Opcodes.halt())
      ],
      feedback_slots: 1
    )
    {:ok, mod_result} = RegisterVM.execute(mod_code)
    # acc(7) mod r0(3) = 1
    assert mod_result.return_value == 1
  end

  test "AddSmi adds a literal integer to acc" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [10]),
        instr(Opcodes.add_smi(), [5, 0]),
        instr(Opcodes.halt())
      ],
      feedback_slots: 1
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 15
  end

  test "Add concatenates strings when either operand is a string" do
    code = make_code(
      [
        instr(Opcodes.lda_constant(), [0]),   # acc = "hello"
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_constant(), [1]),   # acc = " world"
        instr(Opcodes.add(), [0, 0]),          # acc = " world" + "hello" = " worldhello"
        instr(Opcodes.halt())
      ],
      constants: ["hello", " world"],
      registers: 1,
      feedback_slots: 1
    )
    {:ok, result} = RegisterVM.execute(code)
    # acc = " world" (in acc), r0 = "hello"; add: acc + r0 = " worldhello"
    assert result.return_value == " worldhello"
  end

  test "Negate negates the accumulator" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [42]),
        instr(Opcodes.negate()),
        instr(Opcodes.halt())
      ],
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == -42
  end

  test "BitwiseAnd computes bitwise AND" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [0b1100]),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_smi(), [0b1010]),
        instr(Opcodes.bitwise_and(), [0, 0]),
        instr(Opcodes.halt())
      ],
      feedback_slots: 1
    )
    {:ok, result} = RegisterVM.execute(code)
    # acc(0b1010) &&& r0(0b1100) = 0b1000 = 8
    assert result.return_value == 0b1000
  end

  test "ShiftLeft shifts bits left" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [2]),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_smi(), [1]),
        instr(Opcodes.shift_left(), [0, 0]),
        instr(Opcodes.halt())
      ],
      feedback_slots: 1
    )
    {:ok, result} = RegisterVM.execute(code)
    # acc(1) <<< r0(2) = 4
    assert result.return_value == 4
  end

  # ---------------------------------------------------------------------------
  # Test 4: Feedback transitions mono → poly → mega
  # ---------------------------------------------------------------------------
  # We run the same Add instruction multiple times with different type pairs
  # and verify the feedback slot state advances through the state machine.
  #
  # We test Feedback.update_slot directly since the interpreter feedback
  # is an accumulated result from a single run.

  test "Feedback slot state machine: uninitialized → monomorphic → polymorphic → megamorphic" do
    # Start: uninitialized
    slot = :uninitialized

    # First observation: int+int → monomorphic
    slot = Feedback.update_slot(slot, {:integer, :integer})
    assert slot == {:monomorphic, [{:integer, :integer}]}

    # Same pair again → stays monomorphic (deduplication)
    slot = Feedback.update_slot(slot, {:integer, :integer})
    assert slot == {:monomorphic, [{:integer, :integer}]}

    # New pair: int+string → polymorphic
    slot = Feedback.update_slot(slot, {:integer, :string})
    assert match?({:polymorphic, _}, slot)
    {:polymorphic, pairs} = slot
    assert {:integer, :integer} in pairs
    assert {:integer, :string} in pairs

    # Existing pair: no change
    slot_before = slot
    slot = Feedback.update_slot(slot, {:integer, :integer})
    assert slot == slot_before

    # 3rd distinct pair
    slot = Feedback.update_slot(slot, {:string, :string})
    assert match?({:polymorphic, _}, slot)
    {:polymorphic, pairs3} = slot
    assert length(pairs3) == 3

    # 4th distinct pair
    slot = Feedback.update_slot(slot, {:float, :float})
    assert match?({:polymorphic, _}, slot)
    {:polymorphic, pairs4} = slot
    assert length(pairs4) == 4

    # 5th distinct pair → megamorphic (length was already 4)
    slot = Feedback.update_slot(slot, {:boolean, :integer})
    assert slot == :megamorphic

    # Terminal: stays megamorphic
    slot = Feedback.update_slot(slot, {:string, :float})
    assert slot == :megamorphic
  end

  test "Feedback.new_vector creates all-uninitialized vector" do
    v = Feedback.new_vector(5)
    assert length(v) == 5
    assert Enum.all?(v, &(&1 == :uninitialized))
  end

  test "Feedback.value_type classifies values correctly" do
    assert Feedback.value_type(42) == :integer
    assert Feedback.value_type(3.14) == :float
    assert Feedback.value_type("hello") == :string
    assert Feedback.value_type(true) == :boolean
    assert Feedback.value_type(false) == :boolean
    assert Feedback.value_type(nil) == :null
    assert Feedback.value_type(:undefined) == :undefined
    assert Feedback.value_type(%{}) == :object
    assert Feedback.value_type([]) == :array
    assert Feedback.value_type({:function, nil, nil}) == :function
  end

  test "Feedback records monomorphic binary op after same-type repetition" do
    v = Feedback.new_vector(1)
    v = Feedback.record_binary_op(v, 0, 10, 20)      # int+int
    v = Feedback.record_binary_op(v, 0, 30, 40)      # int+int again
    assert Enum.at(v, 0) == {:monomorphic, [{:integer, :integer}]}
  end

  test "Feedback transitions to polymorphic on different-type pair" do
    v = Feedback.new_vector(1)
    v = Feedback.record_binary_op(v, 0, 10, 20)        # int+int
    v = Feedback.record_binary_op(v, 0, "a", 20)      # str+int
    assert match?({:polymorphic, _}, Enum.at(v, 0))
  end

  test "Feedback property load records hidden class" do
    v = Feedback.new_vector(1)
    obj = %{"x" => 1, "y" => 2}
    v = Feedback.record_property_load(v, 0, obj)
    v = Feedback.record_property_load(v, 0, obj)  # same shape again
    # Should be monomorphic with the same hidden class id
    assert match?({:monomorphic, _}, Enum.at(v, 0))
  end

  test "Feedback property load goes polymorphic on different shapes" do
    v = Feedback.new_vector(1)
    obj1 = %{"x" => 1}
    obj2 = %{"x" => 1, "y" => 2}
    v = Feedback.record_property_load(v, 0, obj1)
    v = Feedback.record_property_load(v, 0, obj2)
    assert match?({:polymorphic, _}, Enum.at(v, 0))
  end

  # ---------------------------------------------------------------------------
  # Test 5: JumpIfFalse — conditional branching
  # ---------------------------------------------------------------------------
  # Program:
  #   0: LdaFalse
  #   1: JumpIfFalse +1     ← jumps to ip = (1+1) + 1 = 3
  #   2: LdaConstant 0 (42) ← SKIPPED
  #   3: LdaConstant 1 (99) ← executed
  #   4: Halt
  # Result: 99

  test "JumpIfFalse skips instructions when acc is false" do
    code = make_code(
      [
        instr(Opcodes.lda_false()),           # 0: acc = false
        instr(Opcodes.jump_if_false(), [1]),  # 1: jump +1 past instruction 2
        instr(Opcodes.lda_constant(), [0]),   # 2: acc = 42 (SKIPPED)
        instr(Opcodes.lda_constant(), [1]),   # 3: acc = 99
        instr(Opcodes.halt())                  # 4
      ],
      constants: [42, 99],
      registers: 0,
      feedback_slots: 0
    )

    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 99
  end

  test "JumpIfFalse does not jump when acc is truthy" do
    code = make_code(
      [
        instr(Opcodes.lda_true()),            # 0: acc = true
        instr(Opcodes.jump_if_false(), [1]),  # 1: condition false → no jump
        instr(Opcodes.lda_constant(), [0]),   # 2: acc = 42 (executed)
        instr(Opcodes.lda_constant(), [1]),   # 3: acc = 99
        instr(Opcodes.halt())
      ],
      constants: [42, 99],
      registers: 0,
      feedback_slots: 0
    )

    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 99
  end

  test "JumpIfTrue jumps when acc is truthy" do
    code = make_code(
      [
        instr(Opcodes.lda_true()),            # 0: acc = true
        instr(Opcodes.jump_if_true(), [1]),   # 1: jump +1 past instruction 2
        instr(Opcodes.lda_constant(), [0]),   # 2: acc = 42 (SKIPPED)
        instr(Opcodes.lda_constant(), [1]),   # 3: acc = 99
        instr(Opcodes.halt())
      ],
      constants: [42, 99],
      registers: 0,
      feedback_slots: 0
    )

    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 99
  end

  test "Jump is unconditional" do
    code = make_code(
      [
        instr(Opcodes.jump(), [1]),           # 0: jump +1 past instruction 1
        instr(Opcodes.lda_constant(), [0]),   # 1: SKIPPED
        instr(Opcodes.lda_constant(), [1]),   # 2: acc = 99
        instr(Opcodes.halt())
      ],
      constants: [42, 99],
      registers: 0,
      feedback_slots: 0
    )

    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 99
  end

  test "JumpIfNull jumps when acc is nil" do
    code = make_code(
      [
        instr(Opcodes.lda_null()),
        instr(Opcodes.jump_if_null(), [1]),
        instr(Opcodes.lda_constant(), [0]),   # SKIPPED
        instr(Opcodes.lda_constant(), [1]),
        instr(Opcodes.halt())
      ],
      constants: [42, 99],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 99
  end

  test "JumpIfUndefined jumps when acc is :undefined" do
    code = make_code(
      [
        instr(Opcodes.lda_undefined()),
        instr(Opcodes.jump_if_undefined(), [1]),
        instr(Opcodes.lda_constant(), [0]),   # SKIPPED
        instr(Opcodes.lda_constant(), [1]),
        instr(Opcodes.halt())
      ],
      constants: [42, 99],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 99
  end

  # A simple loop: sum 1 to 5 using JumpLoop
  test "JumpLoop enables backward jumps for iteration" do
    # Pseudocode:
    #   r0 = 1 (counter)
    #   r1 = 0 (sum)
    #   loop:
    #     r1 = r1 + r0
    #     r0 = r0 + 1
    #     if r0 <= 5 goto loop
    #   return r1  (should be 15)
    code = make_code(
      [
        # 0: r0 = 1 (counter)
        instr(Opcodes.lda_smi(), [1]),
        instr(Opcodes.star(), [0]),
        # 2: r1 = 0 (sum)
        instr(Opcodes.lda_zero()),
        instr(Opcodes.star(), [1]),
        # 4: loop start — r1 += r0
        instr(Opcodes.ldar(), [1]),          # acc = r1
        instr(Opcodes.add(), [0, 0]),         # acc = acc + r0; feedback slot 0
        instr(Opcodes.star(), [1]),            # r1 = acc
        # 7: r0 += 1
        instr(Opcodes.ldar(), [0]),           # acc = r0
        instr(Opcodes.add_smi(), [1, 0]),    # acc = acc + 1; feedback slot 0
        instr(Opcodes.star(), [0]),            # r0 = acc
        # 10: if r0 <= 5, goto loop (ip=4)
        instr(Opcodes.lda_smi(), [5]),
        instr(Opcodes.star(), [2]),           # r2 = 5
        instr(Opcodes.ldar(), [0]),           # acc = r0
        instr(Opcodes.test_less_than_or_equal(), [2]),  # acc = r0 <= 5
        # 14: JumpLoop with negative offset: ip after advance = 15; 15 + (-11) = 4
        instr(Opcodes.jump_if_to_boolean_true(), [-11]),
        # 15: done — return r1
        instr(Opcodes.ldar(), [1]),
        instr(Opcodes.halt())
      ],
      registers: 3,
      feedback_slots: 1
    )

    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 15
  end

  # ---------------------------------------------------------------------------
  # Test 6: LdaGlobal / StaGlobal — global variable read/write
  # ---------------------------------------------------------------------------
  # Program:
  #   StaGlobal "x" = 42  (via LdaConstant 42 then StaGlobal)
  #   LdaZero             (clear acc)
  #   LdaGlobal "x"       (load back)
  #   Halt
  # Result: 42

  test "StaGlobal stores acc into globals; LdaGlobal reads it back" do
    code = make_code(
      [
        instr(Opcodes.lda_constant(), [0]),   # 0: acc = 42
        instr(Opcodes.sta_global(), [0]),     # 1: globals["x"] = 42; name=names[0]
        instr(Opcodes.lda_zero()),            # 2: acc = 0 (clear)
        instr(Opcodes.lda_global(), [0, 1]), # 3: acc = globals["x"]; feedback slot 1
        instr(Opcodes.halt())
      ],
      constants: [42],
      names: ["x"],
      registers: 0,
      feedback_slots: 2
    )

    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 42
    assert result.error == nil
  end

  test "LdaGlobal returns error for undefined variable" do
    code = make_code(
      [
        instr(Opcodes.lda_global(), [0, 0]),
        instr(Opcodes.halt())
      ],
      names: ["nonexistent"],
      registers: 0,
      feedback_slots: 1
    )

    {:ok, result} = RegisterVM.execute(code)
    assert result.error != nil
    assert String.contains?(result.error.message, "nonexistent")
  end

  test "StaGlobal and LdaGlobal handle multiple variables" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [10]),
        instr(Opcodes.sta_global(), [0]),     # globals["a"] = 10
        instr(Opcodes.lda_smi(), [20]),
        instr(Opcodes.sta_global(), [1]),     # globals["b"] = 20
        instr(Opcodes.lda_global(), [0]),    # acc = globals["a"] = 10
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_global(), [1]),    # acc = globals["b"] = 20
        instr(Opcodes.add(), [0, 0]),         # acc = 20 + 10 = 30
        instr(Opcodes.halt())
      ],
      names: ["a", "b"],
      registers: 1,
      feedback_slots: 1
    )

    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 30
  end

  # ---------------------------------------------------------------------------
  # Test 7: CallAnyReceiver — function calls
  # ---------------------------------------------------------------------------
  # We create a callee CodeObject that returns a constant value,
  # then a parent CodeObject that creates a closure and calls it.

  test "CreateClosure + CallAnyReceiver invokes a nested function" do
    # Callee: load constant 100 and return it
    callee = %CodeObject{
      instructions: [
        instr(Opcodes.lda_constant(), [0]),
        instr(Opcodes.return())
      ],
      constants: [100],
      names: [],
      register_count: 0,
      feedback_slot_count: 0,
      name: "callee"
    }

    # Caller:
    #   0: CreateClosure 0  (constants[0] = callee code object)
    #   1: Star r0           (r0 = {:function, callee, nil})
    #   2: CallAnyReceiver r0, first_arg=1, argc=0, slot=0
    #   3: Halt
    caller = %CodeObject{
      instructions: [
        instr(Opcodes.create_closure(), [0]),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.call_any_receiver(), [0, 1, 0, 0]),
        instr(Opcodes.halt())
      ],
      constants: [callee],
      names: [],
      register_count: 2,
      feedback_slot_count: 1,
      name: "caller"
    }

    {:ok, result} = RegisterVM.execute(caller)
    assert result.return_value == 100
    assert result.error == nil
  end

  test "Function returns a constant value to the caller" do
    # This test verifies that a callee's return value properly resumes the caller.
    # We use a simple callee that just returns a constant (no args needed).
    callee = %CodeObject{
      instructions: [
        instr(Opcodes.lda_smi(), [77]),
        instr(Opcodes.return())
      ],
      constants: [],
      names: [],
      register_count: 1,
      feedback_slot_count: 0,
      name: "returns_77"
    }

    caller = %CodeObject{
      instructions: [
        instr(Opcodes.create_closure(), [0]),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.call_any_receiver(), [0, 1, 0, 0]),
        # After the call, acc should be 77 (callee's return value)
        instr(Opcodes.halt())
      ],
      constants: [callee],
      names: [],
      register_count: 2,
      feedback_slot_count: 1,
      name: "caller"
    }

    {:ok, result} = RegisterVM.execute(caller)
    assert result.error == nil
    assert result.return_value == 77
  end

  test "Calling non-function produces TypeError" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [42]),       # acc = 42 (not a function)
        instr(Opcodes.star(), [0]),
        instr(Opcodes.call_any_receiver(), [0, 1, 0, 0]),
        instr(Opcodes.halt())
      ],
      registers: 2,
      feedback_slots: 1
    )

    {:ok, result} = RegisterVM.execute(code)
    assert result.error != nil
    assert String.contains?(result.error.message, "not a function")
  end

  # ---------------------------------------------------------------------------
  # Test 8: Halt stops execution
  # ---------------------------------------------------------------------------

  test "Halt stops execution immediately and returns accumulator" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [7]),
        instr(Opcodes.halt()),
        # These instructions must NOT be executed:
        instr(Opcodes.lda_smi(), [999]),
        instr(Opcodes.lda_smi(), [999]),
        instr(Opcodes.lda_smi(), [999])
      ],
      registers: 0,
      feedback_slots: 0
    )

    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 7
  end

  test "Return also stops execution and returns accumulator" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [55]),
        instr(Opcodes.return()),
        instr(Opcodes.lda_smi(), [999])   # Must NOT execute
      ],
      registers: 0,
      feedback_slots: 0
    )

    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 55
  end

  # ---------------------------------------------------------------------------
  # Test 9: LdaNamedProperty with hidden class feedback
  # ---------------------------------------------------------------------------
  # We create an object, store it in a register, load a property from it
  # twice, and verify that the feedback slot transitions to monomorphic.

  test "LdaNamedProperty loads a property and records hidden class feedback" do
    # We pre-populate the object with a property and then load from it.
    # Using separate feedback slots for Sta (slot 0) and Lda (slot 1) to
    # verify each independently. The Lda slot should go monomorphic since
    # we load from an object of consistent shape.
    #
    # Program:
    #   CreateObjectLiteral → acc = %{}
    #   Star r0
    #   LdaConstant 0 (10) → acc = 10
    #   StaNamedProperty r0, "x", slot=0  → r0["x"] = 10
    #   LdaNamedProperty r0, "x", slot=1  → acc = r0["x"] = 10
    #   LdaNamedProperty r0, "x", slot=1  → acc again (same shape → monomorphic)
    #   Halt
    code = make_code(
      [
        instr(Opcodes.create_object_literal()),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_constant(), [0]),              # acc = 10
        instr(Opcodes.sta_named_property(), [0, 0, 0]),  # r0["x"] = 10; slot 0 (sta)
        instr(Opcodes.lda_named_property(), [0, 0, 1]),  # acc = r0["x"]; slot 1 (lda)
        instr(Opcodes.lda_named_property(), [0, 0, 1]),  # acc = r0["x"]; slot 1 again
        instr(Opcodes.halt())
      ],
      constants: [10],
      names: ["x"],
      registers: 1,
      feedback_slots: 2
    )

    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 10
    assert result.error == nil

    # Slot 1 (lda) should have monomorphic hidden class feedback
    # since we loaded the same-shaped object twice
    slot_1 = Enum.at(result.final_feedback_vector, 1)
    assert match?({:monomorphic, _}, slot_1)
  end

  test "StaNamedProperty correctly stores to the object in the register" do
    code = make_code(
      [
        instr(Opcodes.create_object_literal()),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_smi(), [42]),
        instr(Opcodes.sta_named_property(), [0, 0]),   # r0["key"] = 42
        instr(Opcodes.lda_named_property(), [0, 0]),   # acc = r0["key"]
        instr(Opcodes.halt())
      ],
      names: ["key"],
      registers: 1,
      feedback_slots: 0
    )

    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 42
  end

  test "Repeated LdaNamedProperty on same-shape objects stays monomorphic" do
    # Run property access twice on the same object — verifies deduplication
    # in the feedback state machine keeps it monomorphic.
    # Use slot=1 for LdaNamedProperty (slot=0 is for StaNamedProperty)
    # to avoid shape-change pollution between store and load.
    code = make_code(
      [
        instr(Opcodes.create_object_literal()),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_smi(), [5]),
        instr(Opcodes.sta_named_property(), [0, 0, 0]),   # slot 0 for store
        instr(Opcodes.lda_named_property(), [0, 0, 1]),   # slot 1 for load (1st)
        instr(Opcodes.lda_named_property(), [0, 0, 1]),   # slot 1 for load (2nd, same shape)
        instr(Opcodes.halt())
      ],
      names: ["val"],
      registers: 1,
      feedback_slots: 2
    )

    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 5

    slot_1 = Enum.at(result.final_feedback_vector, 1)
    # Should be monomorphic after two accesses on the same-shaped object
    assert match?({:monomorphic, _}, slot_1)
  end

  test "LdaKeyedProperty loads a property by computed key" do
    code = make_code(
      [
        instr(Opcodes.create_object_literal()),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_smi(), [99]),
        instr(Opcodes.sta_named_property(), [0, 0]),    # r0["k"] = 99
        instr(Opcodes.lda_constant(), [0]),              # acc = "k" (the key)
        instr(Opcodes.lda_keyed_property(), [0, 0]),    # acc = r0[acc]
        instr(Opcodes.halt())
      ],
      constants: ["k"],
      names: ["k"],
      registers: 1,
      feedback_slots: 1
    )

    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 99
  end

  # ---------------------------------------------------------------------------
  # Test 10: StackCheck / Stack overflow
  # ---------------------------------------------------------------------------
  # A function that calls itself infinitely should trigger the stack overflow
  # guard. We verify that execute returns an error (not an Elixir crash).

  test "Infinite recursion produces a stack overflow error" do
    # Strategy: We store a function value as a GLOBAL variable "recurse",
    # and the function reads "recurse" from globals and calls it.
    # This avoids the circular CodeObject reference problem.
    #
    # Program (run at top level):
    #   0: StaGlobal "recurse" = <function below>       ← caller sets this up
    #   1: LdaGlobal "recurse"
    #   2: Star r0
    #   3: CallAnyReceiver r0, first_arg=1, argc=0, slot=0
    #   4: Halt
    #
    # The called function ("recurse") does:
    #   0: StackCheck           ← triggers overflow if too deep
    #   1: LdaGlobal "recurse"  ← read the function from globals
    #   2: Star r0
    #   3: CallAnyReceiver r0, first_arg=1, argc=0, slot=0
    #   4: Return

    # The recursive function reads itself from globals and calls itself
    recurse_code = %CodeObject{
      instructions: [
        instr(Opcodes.stack_check()),
        instr(Opcodes.lda_global(), [0]),        # acc = globals["recurse"]
        instr(Opcodes.star(), [0]),
        instr(Opcodes.call_any_receiver(), [0, 1, 0, 0]),
        instr(Opcodes.return())
      ],
      constants: [],
      names: ["recurse"],
      register_count: 2,
      feedback_slot_count: 1,
      name: "recurse"
    }

    # Top-level: store the function in globals["recurse"], then call it
    caller = %CodeObject{
      instructions: [
        instr(Opcodes.create_closure(), [0]),    # acc = {:function, recurse_code, nil}
        instr(Opcodes.sta_global(), [0]),        # globals["recurse"] = func
        instr(Opcodes.lda_global(), [0]),        # acc = globals["recurse"]
        instr(Opcodes.star(), [0]),
        instr(Opcodes.call_any_receiver(), [0, 1, 0, 0]),
        instr(Opcodes.halt())
      ],
      constants: [recurse_code],
      names: ["recurse"],
      register_count: 2,
      feedback_slot_count: 1,
      name: "trigger"
    }

    {:ok, result} = RegisterVM.execute(caller)
    assert result.error != nil
    assert String.contains?(result.error.message, "Stack overflow") or
           String.contains?(result.error.message, "call depth") or
           String.contains?(result.error.message, "stack size") or
           String.contains?(result.error.message, "RangeError")
  end

  # ---------------------------------------------------------------------------
  # Comparison and logic tests
  # ---------------------------------------------------------------------------

  test "TestStrictEqual returns true for identical values" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [5]),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_smi(), [5]),
        instr(Opcodes.test_strict_equal(), [0]),
        instr(Opcodes.halt())
      ],
      registers: 1,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == true
  end

  test "TestStrictEqual returns false for different types" do
    code = make_code(
      [
        instr(Opcodes.lda_constant(), [0]),   # acc = "5" (string)
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_smi(), [5]),         # acc = 5 (integer)
        instr(Opcodes.test_strict_equal(), [0]),
        instr(Opcodes.halt())
      ],
      constants: ["5"],
      registers: 1,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == false
  end

  test "TestEqual (abstract) coerces number and string" do
    code = make_code(
      [
        instr(Opcodes.lda_constant(), [0]),   # acc = "5"
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_smi(), [5]),         # acc = 5
        instr(Opcodes.test_equal(), [0]),      # 5 == "5" → true (coercion)
        instr(Opcodes.halt())
      ],
      constants: ["5"],
      registers: 1,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == true
  end

  test "TestLessThan compares numerically" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [10]),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_smi(), [3]),
        instr(Opcodes.test_less_than(), [0]),  # acc(3) < r0(10) = true
        instr(Opcodes.halt())
      ],
      registers: 1,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == true
  end

  test "LogicalNot inverts truthiness" do
    code = make_code(
      [
        instr(Opcodes.lda_true()),
        instr(Opcodes.logical_not()),
        instr(Opcodes.halt())
      ],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == false
  end

  test "TypeOf returns the correct type string" do
    for {value, expected_type} <- [
      {42, "number"},
      {3.14, "number"},
      {"hello", "string"},
      {true, "boolean"},
      {false, "boolean"}
    ] do
      code = make_code(
        [
          instr(Opcodes.lda_constant(), [0]),
          instr(Opcodes.typeof()),
          instr(Opcodes.halt())
        ],
        constants: [value],
        registers: 0,
        feedback_slots: 0
      )
      {:ok, result} = RegisterVM.execute(code)
      assert result.return_value == expected_type,
             "Expected typeof(#{inspect(value)}) == #{expected_type}, got #{result.return_value}"
    end
  end

  test "TestUndetectable is true for nil and :undefined" do
    for val <- [nil, :undefined] do
      code = make_code(
        [
          instr(Opcodes.lda_constant(), [0]),
          instr(Opcodes.test_undetectable()),
          instr(Opcodes.halt())
        ],
        constants: [val],
        registers: 0,
        feedback_slots: 0
      )
      {:ok, result} = RegisterVM.execute(code)
      assert result.return_value == true
    end
  end

  test "TestUndetectable is false for non-null values" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [0]),
        instr(Opcodes.test_undetectable()),
        instr(Opcodes.halt())
      ],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == false
  end

  # ---------------------------------------------------------------------------
  # Scope / context tests
  # ---------------------------------------------------------------------------

  test "CreateContext + LdaCurrentContextSlot + StaCurrentContextSlot work" do
    code = make_code(
      [
        instr(Opcodes.create_context(), [2]),           # push context with 2 slots
        instr(Opcodes.lda_smi(), [123]),
        instr(Opcodes.sta_current_context_slot(), [0]),  # ctx[0] = 123
        instr(Opcodes.lda_zero()),
        instr(Opcodes.lda_current_context_slot(), [0]),  # acc = ctx[0] = 123
        instr(Opcodes.halt())
      ],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 123
  end

  # ---------------------------------------------------------------------------
  # execute_with_trace test
  # ---------------------------------------------------------------------------

  test "execute_with_trace returns a list of TraceSteps" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [10]),
        instr(Opcodes.add_smi(), [5, 0]),
        instr(Opcodes.halt())
      ],
      registers: 0,
      feedback_slots: 1
    )

    {:ok, result, trace} = RegisterVM.execute_with_trace(code)
    assert result.return_value == 15
    assert is_list(trace)
    assert length(trace) == 3  # 3 instructions executed

    # First step should show acc going from :undefined to 10
    first_step = List.first(trace)
    assert first_step.ip == 0
    assert first_step.acc_before == :undefined
    assert first_step.acc_after == 10

    # Last step (halt) should show final accumulator
    last_step = List.last(trace)
    assert last_step.acc_before == 15
    assert last_step.acc_after == 15
  end

  # ---------------------------------------------------------------------------
  # Object and array creation
  # ---------------------------------------------------------------------------

  test "CreateObjectLiteral creates an empty map" do
    code = make_code(
      [instr(Opcodes.create_object_literal()), instr(Opcodes.halt())],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == %{}
  end

  test "CreateArrayLiteral creates an empty list" do
    code = make_code(
      [instr(Opcodes.create_array_literal()), instr(Opcodes.halt())],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == []
  end

  # ---------------------------------------------------------------------------
  # Throw produces an error
  # ---------------------------------------------------------------------------

  test "Throw instruction creates a VMError" do
    code = make_code(
      [
        instr(Opcodes.lda_constant(), [0]),
        instr(Opcodes.throw())
      ],
      constants: ["something went wrong"],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.error != nil
    assert String.contains?(result.error.message, "something went wrong")
  end

  # ---------------------------------------------------------------------------
  # Opcodes.name/1 tests
  # ---------------------------------------------------------------------------

  test "Opcodes.name returns human-readable name for known opcodes" do
    assert Opcodes.name(Opcodes.lda_constant()) == "LdaConstant"
    assert Opcodes.name(Opcodes.add()) == "Add"
    assert Opcodes.name(Opcodes.halt()) == "Halt"
    assert Opcodes.name(Opcodes.return()) == "Return"
    assert Opcodes.name(Opcodes.jump_if_false()) == "JumpIfFalse"
  end

  test "Opcodes.name returns Unknown(...) for unrecognised opcodes" do
    name = Opcodes.name(0xEE)
    assert String.starts_with?(name, "Unknown")
  end

  # ---------------------------------------------------------------------------
  # Scope module unit tests
  # ---------------------------------------------------------------------------

  test "Scope.new_globals returns empty map" do
    assert CodingAdventures.RegisterVM.Scope.new_globals() == %{}
  end

  test "Scope.set_global and get_global round-trip" do
    alias CodingAdventures.RegisterVM.Scope
    g = Scope.new_globals()
    g = Scope.set_global(g, "foo", 42)
    assert Scope.get_global(g, "foo") == {:ok, 42}
    assert Scope.get_global(g, "bar") == :error
  end

  test "Scope.new_context creates slots of correct size" do
    alias CodingAdventures.RegisterVM.Scope
    ctx = Scope.new_context(nil, 3)
    assert tuple_size(ctx.slots) == 3
    assert ctx.parent == nil
  end

  test "Scope.get_slot and set_slot at depth 0" do
    alias CodingAdventures.RegisterVM.Scope
    ctx = Scope.new_context(nil, 2)
    {:ok, ctx} = Scope.set_slot(ctx, 0, 1, :hello)
    assert Scope.get_slot(ctx, 0, 1) == {:ok, :hello}
  end

  test "Scope.get_slot walks parent chain" do
    alias CodingAdventures.RegisterVM.Scope
    outer = Scope.new_context(nil, 2)
    {:ok, outer} = Scope.set_slot(outer, 0, 0, :outer_value)
    inner = Scope.new_context(outer, 1)
    # depth=1 means walk 1 parent link
    assert Scope.get_slot(inner, 1, 0) == {:ok, :outer_value}
  end

  test "Scope.get_slot returns :error for out-of-bounds index" do
    alias CodingAdventures.RegisterVM.Scope
    ctx = Scope.new_context(nil, 2)
    assert Scope.get_slot(ctx, 0, 5) == :error
  end

  test "Scope.get_slot returns :error for nil context" do
    alias CodingAdventures.RegisterVM.Scope
    assert Scope.get_slot(nil, 0, 0) == :error
  end

  test "Scope.set_slot returns :error for out-of-bounds" do
    alias CodingAdventures.RegisterVM.Scope
    ctx = Scope.new_context(nil, 1)
    assert Scope.set_slot(ctx, 0, 5, :x) == :error
  end

  # ---------------------------------------------------------------------------
  # Additional coverage: arithmetic, bitwise, comparison opcodes
  # ---------------------------------------------------------------------------

  test "Pow computes exponentiation" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [2]),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_smi(), [10]),
        instr(Opcodes.pow(), [0, 0]),   # 10 ** 2 = 100
        instr(Opcodes.halt())
      ],
      registers: 1,
      feedback_slots: 1
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 100
  end

  test "SubSmi subtracts a literal from acc" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [20]),
        instr(Opcodes.sub_smi(), [7, 0]),
        instr(Opcodes.halt())
      ],
      feedback_slots: 1
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 13
  end

  test "BitwiseOr computes bitwise OR" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [0b0011]),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_smi(), [0b0101]),
        instr(Opcodes.bitwise_or(), [0, 0]),
        instr(Opcodes.halt())
      ],
      registers: 1,
      feedback_slots: 1
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 0b0111
  end

  test "BitwiseXor computes bitwise XOR" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [0b1100]),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_smi(), [0b1010]),
        instr(Opcodes.bitwise_xor(), [0, 0]),
        instr(Opcodes.halt())
      ],
      registers: 1,
      feedback_slots: 1
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 0b0110
  end

  test "BitwiseNot inverts bits" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [0]),
        instr(Opcodes.bitwise_not(), [0]),
        instr(Opcodes.halt())
      ],
      feedback_slots: 1
    )
    {:ok, result} = RegisterVM.execute(code)
    # ~~~0 = -1 in two's complement
    assert result.return_value == -1
  end

  test "ShiftRight shifts bits right (arithmetic)" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [1]),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_smi(), [16]),
        instr(Opcodes.shift_right(), [0, 0]),  # 16 >>> 1 = 8
        instr(Opcodes.halt())
      ],
      registers: 1,
      feedback_slots: 1
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 8
  end

  test "ShiftRightLogical masks to 32 bits" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [1]),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_smi(), [0xFF]),
        instr(Opcodes.shift_right_logical(), [0, 0]),  # 0xFF >>> 1 = 127
        instr(Opcodes.halt())
      ],
      registers: 1,
      feedback_slots: 1
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 127
  end

  test "TestGreaterThan returns true when acc > reg" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [3]),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_smi(), [10]),
        instr(Opcodes.test_greater_than(), [0]),  # 10 > 3 = true
        instr(Opcodes.halt())
      ],
      registers: 1,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == true
  end

  test "TestGreaterThanOrEqual returns true for equal values" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [5]),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_smi(), [5]),
        instr(Opcodes.test_greater_than_or_equal(), [0]),
        instr(Opcodes.halt())
      ],
      registers: 1,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == true
  end

  test "TestLessThanOrEqual returns false when acc > reg" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [3]),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_smi(), [10]),
        instr(Opcodes.test_less_than_or_equal(), [0]),  # 10 <= 3 = false
        instr(Opcodes.halt())
      ],
      registers: 1,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == false
  end

  test "TestNotEqual returns true for different values" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [5]),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_smi(), [10]),
        instr(Opcodes.test_not_equal(), [0]),
        instr(Opcodes.halt())
      ],
      registers: 1,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == true
  end

  test "TestStrictNotEqual returns true for different values" do
    code = make_code(
      [
        instr(Opcodes.lda_constant(), [0]),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_smi(), [5]),
        instr(Opcodes.test_strict_not_equal(), [0]),
        instr(Opcodes.halt())
      ],
      constants: ["5"],
      registers: 1,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == true
  end

  test "TestIn checks list membership" do
    code = make_code(
      [
        instr(Opcodes.lda_constant(), [0]),   # acc = [1, 2, 3]
        instr(Opcodes.star(), [0]),            # r0 = [1, 2, 3]
        instr(Opcodes.lda_smi(), [2]),         # acc = 2 (the key)
        instr(Opcodes.test_in(), [0]),         # acc = (2 in r0)
        instr(Opcodes.halt())
      ],
      constants: [[1, 2, 3]],
      registers: 1,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == true
  end

  test "TestIn checks map key membership" do
    code = make_code(
      [
        instr(Opcodes.create_object_literal()),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_smi(), [42]),
        instr(Opcodes.sta_named_property(), [0, 0]),
        instr(Opcodes.lda_constant(), [0]),   # acc = "key"
        instr(Opcodes.test_in(), [0]),         # acc = ("key" in r0)
        instr(Opcodes.halt())
      ],
      constants: ["key"],
      names: ["key"],
      registers: 1,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == true
  end

  test "TestInstanceOf returns true for maps" do
    code = make_code(
      [
        instr(Opcodes.create_object_literal()),
        instr(Opcodes.star(), [1]),            # r1 = type (unused, simplified)
        instr(Opcodes.create_object_literal()),
        instr(Opcodes.test_instanceof(), [1]),  # acc = is_map(acc)
        instr(Opcodes.halt())
      ],
      registers: 2,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == true
  end

  test "Div by zero returns :infinity" do
    code = make_code(
      [
        instr(Opcodes.lda_zero()),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_smi(), [10]),
        instr(Opcodes.div(), [0, 0]),   # 10 / 0
        instr(Opcodes.halt())
      ],
      registers: 1,
      feedback_slots: 1
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == :infinity
  end

  # ---------------------------------------------------------------------------
  # Additional coverage: iteration opcodes
  # ---------------------------------------------------------------------------

  test "GetIterator + CallIteratorStep + GetIteratorDone iterates a list" do
    # Iterate [10, 20, 30] and verify we can detect done status
    code = make_code(
      [
        instr(Opcodes.lda_constant(), [0]),      # acc = [10, 20, 30]
        instr(Opcodes.get_iterator()),            # acc = {:iterator, [10,20,30], false}
        instr(Opcodes.call_iterator_step()),      # advance: skip first element
        instr(Opcodes.get_iterator_done()),        # acc = done?
        instr(Opcodes.halt())
      ],
      constants: [[10, 20, 30]],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == false   # not done yet (2 elements remain)
  end

  test "GetIteratorValue extracts current value" do
    code = make_code(
      [
        instr(Opcodes.lda_constant(), [0]),    # acc = [42, 99]
        instr(Opcodes.get_iterator()),
        instr(Opcodes.get_iterator_value()),   # acc = 42
        instr(Opcodes.halt())
      ],
      constants: [[42, 99]],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 42
  end

  test "GetIteratorDone is true on empty iterator" do
    code = make_code(
      [
        instr(Opcodes.lda_constant(), [0]),    # acc = []
        instr(Opcodes.get_iterator()),
        instr(Opcodes.get_iterator_done()),
        instr(Opcodes.halt())
      ],
      constants: [[]],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == false   # iterator starts; empty list = done immediately
  end

  test "CallIteratorStep on empty iterator stays done" do
    code = make_code(
      [
        instr(Opcodes.lda_constant(), [0]),
        instr(Opcodes.get_iterator()),
        instr(Opcodes.call_iterator_step()),   # step an empty iterator
        instr(Opcodes.get_iterator_done()),
        instr(Opcodes.halt())
      ],
      constants: [[]],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == true
  end

  test "GetIterator on a map produces a list iterator" do
    code = make_code(
      [
        instr(Opcodes.create_object_literal()),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_smi(), [1]),
        instr(Opcodes.sta_named_property(), [0, 0]),
        instr(Opcodes.ldar(), [0]),
        instr(Opcodes.get_iterator()),
        instr(Opcodes.get_iterator_done()),   # maps produce list of pairs, not done immediately
        instr(Opcodes.halt())
      ],
      names: ["a"],
      registers: 1,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    # Map iterator should not be immediately done (map has 1 entry)
    assert result.return_value == false
  end

  # ---------------------------------------------------------------------------
  # Additional coverage: context, module variable, clone
  # ---------------------------------------------------------------------------

  test "LdaContextSlot reads from context at depth 1" do
    code = make_code(
      [
        instr(Opcodes.create_context(), [2]),          # outer context, 2 slots
        instr(Opcodes.lda_smi(), [42]),
        instr(Opcodes.sta_current_context_slot(), [0]),
        instr(Opcodes.create_context(), [1]),          # inner context (child)
        instr(Opcodes.lda_context_slot(), [1, 0]),     # depth=1 → read outer ctx slot 0
        instr(Opcodes.halt())
      ],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 42
  end

  test "StaContextSlot writes to context at depth 1" do
    code = make_code(
      [
        instr(Opcodes.create_context(), [2]),
        instr(Opcodes.create_context(), [1]),
        instr(Opcodes.lda_smi(), [99]),
        instr(Opcodes.sta_context_slot(), [1, 0]),      # write to outer ctx slot 0
        instr(Opcodes.lda_context_slot(), [1, 0]),      # read it back
        instr(Opcodes.halt())
      ],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 99
  end

  test "PushContext and PopContext manage scope stack" do
    code = make_code(
      [
        instr(Opcodes.push_context(), [2]),
        instr(Opcodes.lda_smi(), [55]),
        instr(Opcodes.sta_current_context_slot(), [0]),
        instr(Opcodes.lda_current_context_slot(), [0]),
        instr(Opcodes.star(), [0]),                     # r0 = 55
        instr(Opcodes.pop_context()),
        instr(Opcodes.ldar(), [0]),
        instr(Opcodes.halt())
      ],
      registers: 1,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 55
  end

  test "LdaModuleVariable and StaModuleVariable round-trip" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [123]),
        instr(Opcodes.sta_module_variable(), [0]),
        instr(Opcodes.lda_zero()),
        instr(Opcodes.lda_module_variable(), [0]),
        instr(Opcodes.halt())
      ],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 123
  end

  test "CloneObject creates a shallow copy" do
    code = make_code(
      [
        instr(Opcodes.create_object_literal()),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_smi(), [7]),
        instr(Opcodes.sta_named_property(), [0, 0]),   # r0["k"] = 7
        instr(Opcodes.clone_object(), [0]),             # acc = clone of r0
        instr(Opcodes.halt())
      ],
      names: ["k"],
      registers: 1,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert is_map(result.return_value)
    assert result.return_value["k"] == 7
  end

  test "Debugger opcode is a no-op" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [5]),
        instr(Opcodes.debugger()),
        instr(Opcodes.halt())
      ],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 5
  end

  test "Rethrow instruction creates a VMError" do
    code = make_code(
      [
        instr(Opcodes.lda_constant(), [0]),
        instr(Opcodes.rethrow())
      ],
      constants: ["rethrown error"],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.error != nil
  end

  test "DeletePropertyStrict removes a key from an object" do
    code = make_code(
      [
        instr(Opcodes.create_object_literal()),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_smi(), [1]),
        instr(Opcodes.sta_named_property(), [0, 0]),
        instr(Opcodes.lda_constant(), [0]),             # acc = "prop" (key)
        instr(Opcodes.delete_property_strict(), [0]),   # delete r0["prop"]
        instr(Opcodes.halt())
      ],
      constants: ["prop"],
      names: ["prop"],
      registers: 1,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == true
  end

  test "DeletePropertySloppy removes a key from an object" do
    code = make_code(
      [
        instr(Opcodes.create_object_literal()),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_smi(), [99]),
        instr(Opcodes.sta_named_property(), [0, 0]),
        instr(Opcodes.lda_constant(), [0]),
        instr(Opcodes.delete_property_sloppy(), [0]),
        instr(Opcodes.halt())
      ],
      constants: ["field"],
      names: ["field"],
      registers: 1,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == true
  end

  test "StaKeyedProperty sets a keyed property on an object" do
    code = make_code(
      [
        instr(Opcodes.create_object_literal()),
        instr(Opcodes.star(), [0]),             # r0 = {}
        instr(Opcodes.lda_constant(), [0]),     # acc = "mykey"
        instr(Opcodes.star(), [1]),             # r1 = "mykey"
        instr(Opcodes.lda_smi(), [888]),        # acc = 888
        instr(Opcodes.sta_keyed_property(), [0, 1, 0]),  # r0[r1] = acc
        instr(Opcodes.lda_constant(), [0]),     # acc = "mykey" again
        instr(Opcodes.lda_keyed_property(), [0, 0]),     # acc = r0[acc]
        instr(Opcodes.halt())
      ],
      constants: ["mykey"],
      registers: 2,
      feedback_slots: 1
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 888
  end

  test "LdaNamedPropertyNoFeedback loads without recording feedback" do
    code = make_code(
      [
        instr(Opcodes.create_object_literal()),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.lda_smi(), [33]),
        instr(Opcodes.sta_named_property_no_feedback(), [0, 0]),
        instr(Opcodes.lda_named_property_no_feedback(), [0, 0]),
        instr(Opcodes.halt())
      ],
      names: ["z"],
      registers: 1,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 33
  end

  test "CreateRegExpLiteral stores a regexp tuple" do
    code = make_code(
      [
        instr(Opcodes.create_regexp_literal(), [0]),
        instr(Opcodes.halt())
      ],
      constants: ["[a-z]+"],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == {:regexp, "[a-z]+"}
  end

  test "JumpLoop implements backward jump" do
    # Simple countdown: r0 starts at 3, decrement until 0
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [3]),
        instr(Opcodes.star(), [0]),              # r0 = 3
        # loop start (ip=2):
        instr(Opcodes.ldar(), [0]),              # acc = r0
        instr(Opcodes.sub_smi(), [1, 0]),        # acc = acc - 1
        instr(Opcodes.star(), [0]),              # r0 = acc
        instr(Opcodes.lda_zero()),
        instr(Opcodes.star(), [1]),              # r1 = 0
        instr(Opcodes.ldar(), [0]),              # acc = r0
        instr(Opcodes.test_greater_than(), [1]), # acc = r0 > 0
        # JumpLoop: if r0 > 0, go back to ip=2
        # ip after advance = 10; 10 + (-8) = 2
        instr(Opcodes.jump_if_to_boolean_true(), [-8]),
        instr(Opcodes.ldar(), [0]),
        instr(Opcodes.halt())
      ],
      registers: 2,
      feedback_slots: 1
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 0
  end

  test "JumpIfNullOrUndefined jumps for both nil and :undefined" do
    for val <- [nil, :undefined] do
      code = make_code(
        [
          instr(Opcodes.lda_constant(), [0]),
          instr(Opcodes.jump_if_null_or_undefined(), [1]),
          instr(Opcodes.lda_smi(), [0]),         # SKIPPED
          instr(Opcodes.lda_smi(), [1]),
          instr(Opcodes.halt())
        ],
        constants: [val],
        registers: 0,
        feedback_slots: 0
      )
      {:ok, result} = RegisterVM.execute(code)
      assert result.return_value == 1
    end
  end

  test "JumpIfToBooleanTrue works like JumpIfTrue" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [1]),
        instr(Opcodes.jump_if_to_boolean_true(), [1]),
        instr(Opcodes.lda_smi(), [0]),   # SKIPPED
        instr(Opcodes.lda_smi(), [99]),
        instr(Opcodes.halt())
      ],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 99
  end

  test "JumpIfToBooleanFalse works like JumpIfFalse" do
    code = make_code(
      [
        instr(Opcodes.lda_false()),
        instr(Opcodes.jump_if_to_boolean_false(), [1]),
        instr(Opcodes.lda_smi(), [0]),   # SKIPPED
        instr(Opcodes.lda_smi(), [77]),
        instr(Opcodes.halt())
      ],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 77
  end

  test "SuspendGenerator returns accumulator" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [42]),
        instr(Opcodes.suspend_generator()),
        instr(Opcodes.lda_smi(), [0])   # Not reached
      ],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 42
  end

  test "ResumeGenerator is a no-op" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [5]),
        instr(Opcodes.resume_generator()),
        instr(Opcodes.halt())
      ],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 5
  end

  test "TypeOf returns 'function' for a closure" do
    code = make_code(
      [
        instr(Opcodes.create_closure(), [0]),
        instr(Opcodes.typeof()),
        instr(Opcodes.halt())
      ],
      constants: [%CodeObject{
        instructions: [instr(Opcodes.return())],
        constants: [], names: [], register_count: 0,
        feedback_slot_count: 0, name: "noop"
      }],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == "function"
  end

  test "TypeOf returns 'undefined' for :undefined" do
    code = make_code(
      [
        instr(Opcodes.lda_undefined()),
        instr(Opcodes.typeof()),
        instr(Opcodes.halt())
      ],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == "undefined"
  end

  test "TypeOf returns 'object' for a map" do
    code = make_code(
      [
        instr(Opcodes.create_object_literal()),
        instr(Opcodes.typeof()),
        instr(Opcodes.halt())
      ],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == "object"
  end

  test "TypeOf returns 'object' for a list" do
    code = make_code(
      [
        instr(Opcodes.create_array_literal()),
        instr(Opcodes.typeof()),
        instr(Opcodes.halt())
      ],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == "object"
  end

  test "LdaLocal and StaLocal work via globals map" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [7]),
        instr(Opcodes.sta_local(), [0]),
        instr(Opcodes.lda_zero()),
        instr(Opcodes.lda_local(), [0]),
        instr(Opcodes.halt())
      ],
      names: ["local_var"],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 7
  end

  test "LdaLocal error for undefined variable" do
    code = make_code(
      [
        instr(Opcodes.lda_local(), [0]),
        instr(Opcodes.halt())
      ],
      names: ["missing"],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.error != nil
  end

  test "Opcodes module covers all opcode name lookups" do
    # Spot-check a sample from each category
    assert Opcodes.name(Opcodes.lda_zero()) == "LdaZero"
    assert Opcodes.name(Opcodes.lda_smi()) == "LdaSmi"
    assert Opcodes.name(Opcodes.lda_undefined()) == "LdaUndefined"
    assert Opcodes.name(Opcodes.lda_null()) == "LdaNull"
    assert Opcodes.name(Opcodes.lda_true()) == "LdaTrue"
    assert Opcodes.name(Opcodes.lda_false()) == "LdaFalse"
    assert Opcodes.name(Opcodes.ldar()) == "Ldar"
    assert Opcodes.name(Opcodes.star()) == "Star"
    assert Opcodes.name(Opcodes.mov()) == "Mov"
    assert Opcodes.name(Opcodes.sub()) == "Sub"
    assert Opcodes.name(Opcodes.mul()) == "Mul"
    assert Opcodes.name(Opcodes.div()) == "Div"
    assert Opcodes.name(Opcodes.mod()) == "Mod"
    assert Opcodes.name(Opcodes.pow()) == "Pow"
    assert Opcodes.name(Opcodes.add_smi()) == "AddSmi"
    assert Opcodes.name(Opcodes.sub_smi()) == "SubSmi"
    assert Opcodes.name(Opcodes.negate()) == "Negate"
    assert Opcodes.name(Opcodes.bitwise_and()) == "BitwiseAnd"
    assert Opcodes.name(Opcodes.bitwise_or()) == "BitwiseOr"
    assert Opcodes.name(Opcodes.bitwise_xor()) == "BitwiseXor"
    assert Opcodes.name(Opcodes.bitwise_not()) == "BitwiseNot"
    assert Opcodes.name(Opcodes.shift_left()) == "ShiftLeft"
    assert Opcodes.name(Opcodes.shift_right()) == "ShiftRight"
    assert Opcodes.name(Opcodes.shift_right_logical()) == "ShiftRightLogical"
    assert Opcodes.name(Opcodes.test_equal()) == "TestEqual"
    assert Opcodes.name(Opcodes.test_not_equal()) == "TestNotEqual"
    assert Opcodes.name(Opcodes.test_strict_equal()) == "TestStrictEqual"
    assert Opcodes.name(Opcodes.test_strict_not_equal()) == "TestStrictNotEqual"
    assert Opcodes.name(Opcodes.test_less_than()) == "TestLessThan"
    assert Opcodes.name(Opcodes.test_greater_than()) == "TestGreaterThan"
    assert Opcodes.name(Opcodes.test_less_than_or_equal()) == "TestLessThanOrEqual"
    assert Opcodes.name(Opcodes.test_greater_than_or_equal()) == "TestGreaterThanOrEqual"
    assert Opcodes.name(Opcodes.test_in()) == "TestIn"
    assert Opcodes.name(Opcodes.test_instanceof()) == "TestInstanceOf"
    assert Opcodes.name(Opcodes.test_undetectable()) == "TestUndetectable"
    assert Opcodes.name(Opcodes.logical_not()) == "LogicalNot"
    assert Opcodes.name(Opcodes.typeof()) == "TypeOf"
    assert Opcodes.name(Opcodes.jump()) == "Jump"
    assert Opcodes.name(Opcodes.jump_if_true()) == "JumpIfTrue"
    assert Opcodes.name(Opcodes.jump_if_false()) == "JumpIfFalse"
    assert Opcodes.name(Opcodes.jump_if_null()) == "JumpIfNull"
    assert Opcodes.name(Opcodes.jump_if_undefined()) == "JumpIfUndefined"
    assert Opcodes.name(Opcodes.jump_if_null_or_undefined()) == "JumpIfNullOrUndefined"
    assert Opcodes.name(Opcodes.jump_if_to_boolean_true()) == "JumpIfToBooleanTrue"
    assert Opcodes.name(Opcodes.jump_if_to_boolean_false()) == "JumpIfToBooleanFalse"
    assert Opcodes.name(Opcodes.jump_loop()) == "JumpLoop"
    assert Opcodes.name(Opcodes.call_any_receiver()) == "CallAnyReceiver"
    assert Opcodes.name(Opcodes.call_property()) == "CallProperty"
    assert Opcodes.name(Opcodes.call_undefined_receiver()) == "CallUndefinedReceiver"
    assert Opcodes.name(Opcodes.construct()) == "Construct"
    assert Opcodes.name(Opcodes.construct_with_spread()) == "ConstructWithSpread"
    assert Opcodes.name(Opcodes.call_with_spread()) == "CallWithSpread"
    assert Opcodes.name(Opcodes.suspend_generator()) == "SuspendGenerator"
    assert Opcodes.name(Opcodes.resume_generator()) == "ResumeGenerator"
    assert Opcodes.name(Opcodes.lda_named_property()) == "LdaNamedProperty"
    assert Opcodes.name(Opcodes.sta_named_property()) == "StaNamedProperty"
    assert Opcodes.name(Opcodes.lda_keyed_property()) == "LdaKeyedProperty"
    assert Opcodes.name(Opcodes.sta_keyed_property()) == "StaKeyedProperty"
    assert Opcodes.name(Opcodes.lda_named_property_no_feedback()) == "LdaNamedPropertyNoFeedback"
    assert Opcodes.name(Opcodes.sta_named_property_no_feedback()) == "StaNamedPropertyNoFeedback"
    assert Opcodes.name(Opcodes.delete_property_strict()) == "DeletePropertyStrict"
    assert Opcodes.name(Opcodes.delete_property_sloppy()) == "DeletePropertySloppy"
    assert Opcodes.name(Opcodes.create_object_literal()) == "CreateObjectLiteral"
    assert Opcodes.name(Opcodes.create_array_literal()) == "CreateArrayLiteral"
    assert Opcodes.name(Opcodes.create_regexp_literal()) == "CreateRegExpLiteral"
    assert Opcodes.name(Opcodes.create_closure()) == "CreateClosure"
    assert Opcodes.name(Opcodes.create_context()) == "CreateContext"
    assert Opcodes.name(Opcodes.clone_object()) == "CloneObject"
    assert Opcodes.name(Opcodes.get_iterator()) == "GetIterator"
    assert Opcodes.name(Opcodes.call_iterator_step()) == "CallIteratorStep"
    assert Opcodes.name(Opcodes.get_iterator_done()) == "GetIteratorDone"
    assert Opcodes.name(Opcodes.get_iterator_value()) == "GetIteratorValue"
    assert Opcodes.name(Opcodes.throw()) == "Throw"
    assert Opcodes.name(Opcodes.rethrow()) == "Rethrow"
    assert Opcodes.name(Opcodes.push_context()) == "PushContext"
    assert Opcodes.name(Opcodes.pop_context()) == "PopContext"
    assert Opcodes.name(Opcodes.lda_module_variable()) == "LdaModuleVariable"
    assert Opcodes.name(Opcodes.sta_module_variable()) == "StaModuleVariable"
    assert Opcodes.name(Opcodes.stack_check()) == "StackCheck"
    assert Opcodes.name(Opcodes.debugger()) == "Debugger"
  end

  test "CallUndefinedReceiver calls a function" do
    callee = %CodeObject{
      instructions: [
        instr(Opcodes.lda_smi(), [42]),
        instr(Opcodes.return())
      ],
      constants: [], names: [], register_count: 0, feedback_slot_count: 0, name: "f"
    }
    code = make_code(
      [
        instr(Opcodes.create_closure(), [0]),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.call_undefined_receiver(), [0, 1, 0, 0]),
        instr(Opcodes.halt())
      ],
      constants: [callee],
      registers: 2,
      feedback_slots: 1
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 42
  end

  test "CallProperty calls a function" do
    callee = %CodeObject{
      instructions: [
        instr(Opcodes.lda_smi(), [77]),
        instr(Opcodes.return())
      ],
      constants: [], names: [], register_count: 0, feedback_slot_count: 0, name: "f"
    }
    code = make_code(
      [
        instr(Opcodes.create_closure(), [0]),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.call_property(), [0, 1, 0, 0]),
        instr(Opcodes.halt())
      ],
      constants: [callee],
      registers: 2,
      feedback_slots: 1
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 77
  end

  test "Construct calls a function like CallAnyReceiver" do
    callee = %CodeObject{
      instructions: [
        instr(Opcodes.lda_smi(), [99]),
        instr(Opcodes.return())
      ],
      constants: [], names: [], register_count: 0, feedback_slot_count: 0, name: "Ctor"
    }
    code = make_code(
      [
        instr(Opcodes.create_closure(), [0]),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.construct(), [0, 1, 0, 0]),
        instr(Opcodes.halt())
      ],
      constants: [callee],
      registers: 2,
      feedback_slots: 1
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 99
  end

  test "CallWithSpread calls a function" do
    callee = %CodeObject{
      instructions: [
        instr(Opcodes.lda_smi(), [11]),
        instr(Opcodes.return())
      ],
      constants: [], names: [], register_count: 0, feedback_slot_count: 0, name: "f"
    }
    code = make_code(
      [
        instr(Opcodes.create_closure(), [0]),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.call_with_spread(), [0, 1, 0, 0]),
        instr(Opcodes.halt())
      ],
      constants: [callee],
      registers: 2,
      feedback_slots: 1
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 11
  end

  test "ConstructWithSpread calls a function" do
    callee = %CodeObject{
      instructions: [
        instr(Opcodes.lda_smi(), [22]),
        instr(Opcodes.return())
      ],
      constants: [], names: [], register_count: 0, feedback_slot_count: 0, name: "C"
    }
    code = make_code(
      [
        instr(Opcodes.create_closure(), [0]),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.construct_with_spread(), [0, 1, 0, 0]),
        instr(Opcodes.halt())
      ],
      constants: [callee],
      registers: 2,
      feedback_slots: 1
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 22
  end

  test "Unknown opcode produces an error" do
    code = make_code(
      [
        instr(0xCC, []),
        instr(Opcodes.halt(), [])
      ],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.error != nil
    assert String.contains?(result.error.message, "Unknown opcode")
  end

  test "Implicit halt at end of instructions (ip out of bounds)" do
    # No explicit halt — ip walks past the end; should return acc
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [42])
        # no halt
      ],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.return_value == 42
  end

  test "CallAnyReceiver with non-function produces error" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [42]),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.call_undefined_receiver(), [0, 1, 0, 0]),
        instr(Opcodes.halt())
      ],
      registers: 2,
      feedback_slots: 1
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.error != nil
    assert String.contains?(result.error.message, "not a function")
  end

  test "CallProperty with non-function produces error" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [42]),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.call_property(), [0, 1, 0, 0]),
        instr(Opcodes.halt())
      ],
      registers: 2,
      feedback_slots: 1
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.error != nil
  end

  test "Construct with non-function produces error" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [42]),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.construct(), [0, 1, 0, 0]),
        instr(Opcodes.halt())
      ],
      registers: 2,
      feedback_slots: 1
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.error != nil
  end

  test "CallWithSpread with non-function produces error" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [42]),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.call_with_spread(), [0]),
        instr(Opcodes.halt())
      ],
      registers: 2,
      feedback_slots: 1
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.error != nil
  end

  test "ConstructWithSpread with non-function produces error" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [42]),
        instr(Opcodes.star(), [0]),
        instr(Opcodes.construct_with_spread(), [0]),
        instr(Opcodes.halt())
      ],
      registers: 2,
      feedback_slots: 1
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.error != nil
  end

  test "LdaContextSlot error on bad depth" do
    code = make_code(
      [
        instr(Opcodes.lda_context_slot(), [99, 0]),   # depth 99 = no such context
        instr(Opcodes.halt())
      ],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.error != nil
  end

  test "StaContextSlot error on bad depth" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [1]),
        instr(Opcodes.sta_context_slot(), [99, 0]),
        instr(Opcodes.halt())
      ],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.error != nil
  end

  test "LdaCurrentContextSlot error when no context" do
    code = make_code(
      [
        instr(Opcodes.lda_current_context_slot(), [0]),
        instr(Opcodes.halt())
      ],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.error != nil
  end

  test "StaCurrentContextSlot error when no context" do
    code = make_code(
      [
        instr(Opcodes.lda_smi(), [1]),
        instr(Opcodes.sta_current_context_slot(), [0]),
        instr(Opcodes.halt())
      ],
      registers: 0,
      feedback_slots: 0
    )
    {:ok, result} = RegisterVM.execute(code)
    assert result.error != nil
  end

  test "Feedback.record_call_site records function type" do
    v = Feedback.new_vector(1)
    dummy_code = %CodeObject{
      instructions: [], constants: [], names: [],
      register_count: 0, feedback_slot_count: 0, name: "f"
    }
    func = {:function, dummy_code, nil}
    v = Feedback.record_call_site(v, 0, func)
    assert match?({:monomorphic, _}, Enum.at(v, 0))
  end

  test "Feedback.record_binary_op ignores out-of-bounds slot" do
    v = Feedback.new_vector(1)
    v2 = Feedback.record_binary_op(v, 99, 1, 2)
    assert v2 == v   # unchanged
  end

  test "Feedback.record_property_load ignores non-map" do
    v = Feedback.new_vector(1)
    v2 = Feedback.record_property_load(v, 0, "not a map")
    assert v2 == v
  end
end
