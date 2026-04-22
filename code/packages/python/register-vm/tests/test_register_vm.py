"""Comprehensive tests for the register-based virtual machine.

Each test builds a ``CodeObject`` manually from ``RegisterInstruction`` and
``Opcode`` values, then executes it through ``RegisterVM`` (or the module-level
``execute`` / ``execute_with_trace`` helpers).

Testing philosophy
------------------
We test at the *bytecode* level (not through a compiler), which:

1. Keeps tests focused on the VM semantics, not compiler correctness.
2. Exercises edge cases precisely (e.g. exact jump offsets).
3. Is how you would write VM conformance tests for a new back-end.

Each test group corresponds to a major opcode category and is named
``test_<category>_<specific_behaviour>``.
"""

from dataclasses import dataclass

import pytest

from register_vm import (
    UNDEFINED,
    CodeObject,
    Opcode,
    RegisterInstruction,
    RegisterVM,
    SlotMegamorphic,
    SlotMonomorphic,
    SlotPolymorphic,
    VMFunction,
    VMObject,
    execute,
    execute_with_trace,
)
from register_vm.feedback import (
    new_vector,
    record_binary_op,
    record_call_site,
    record_property_load,
    value_type,
)
from register_vm.generic_vm import (
    GenericRegisterVM,
    GenericTrace,
    GenericVMError,
    RegisterFrame,
)
from register_vm.scope import get_slot, new_context, set_slot

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def simple_code(
    instructions: list[RegisterInstruction],
    constants: list = None,
    names: list[str] = None,
    register_count: int = 4,
    feedback_slot_count: int = 4,
) -> CodeObject:
    """Build a ``CodeObject`` with sensible defaults for tests.

    Provides 4 registers and 4 feedback slots unless overridden.
    """
    return CodeObject(
        instructions=instructions,
        constants=constants or [],
        names=names or [],
        register_count=register_count,
        feedback_slot_count=feedback_slot_count,
    )


# ===========================================================================
# 1. Accumulator loads and basic arithmetic
# ===========================================================================

class TestAccumulatorLoads:
    """Tests for opcodes that put literal values into the accumulator."""

    def test_lda_constant_integer(self) -> None:
        """LDA_CONSTANT should load a constant-pool integer into the accumulator."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_CONSTANT, [0]),
                RegisterInstruction(Opcode.RETURN),
            ],
            constants=[99],
        )
        result = execute(code)
        assert result.error is None
        assert result.return_value == 99

    def test_lda_smi(self) -> None:
        """LDA_SMI should embed the integer directly in the instruction operand."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_SMI, [42]),
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        result = execute(code)
        assert result.return_value == 42

    def test_lda_zero(self) -> None:
        """LDA_ZERO is a special opcode that loads 0 without a constant pool entry."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_ZERO),
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        result = execute(code)
        assert result.return_value == 0

    def test_lda_true_false(self) -> None:
        """LDA_TRUE and LDA_FALSE should load Python booleans."""
        for opcode, expected in [(Opcode.LDA_TRUE, True), (Opcode.LDA_FALSE, False)]:
            code = simple_code(
                instructions=[
                    RegisterInstruction(opcode),
                    RegisterInstruction(Opcode.RETURN),
                ],
            )
            result = execute(code)
            assert result.return_value is expected

    def test_lda_null_and_undefined(self) -> None:
        """LDA_NULL returns None; LDA_UNDEFINED returns the UNDEFINED sentinel."""
        code_null = simple_code(
            instructions=[RegisterInstruction(Opcode.LDA_NULL), RegisterInstruction(Opcode.RETURN)],
        )
        code_undef = simple_code(
            instructions=[RegisterInstruction(Opcode.LDA_UNDEFINED), RegisterInstruction(Opcode.RETURN)],
        )
        assert execute(code_null).return_value is None
        assert execute(code_undef).return_value is UNDEFINED


# ===========================================================================
# 2. Register moves (STAR / LDAR / MOV)
# ===========================================================================

class TestRegisterMoves:
    """Tests for register file read/write operations."""

    def test_star_ldar_roundtrip(self) -> None:
        """Storing to a register and loading from it should give the same value."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_SMI, [123]),
                RegisterInstruction(Opcode.STAR, [2]),       # r2 = 123
                RegisterInstruction(Opcode.LDA_ZERO),        # acc = 0
                RegisterInstruction(Opcode.LDAR, [2]),       # acc = r2
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        result = execute(code)
        assert result.return_value == 123

    def test_mov_copies_between_registers(self) -> None:
        """MOV should copy one register to another without touching the accumulator."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_SMI, [55]),
                RegisterInstruction(Opcode.STAR, [0]),       # r0 = 55
                RegisterInstruction(Opcode.LDA_ZERO),        # acc = 0 (distractor)
                RegisterInstruction(Opcode.MOV, [0, 1]),     # r1 = r0
                RegisterInstruction(Opcode.LDAR, [1]),       # acc = r1
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        result = execute(code)
        assert result.return_value == 55


# ===========================================================================
# 3. Arithmetic operations
# ===========================================================================

class TestArithmetic:
    """Tests for ADD, SUB, MUL, DIV, MOD, POW, ADD_SMI, NEGATE, and bitwise ops."""

    def test_add_integers(self) -> None:
        """ADD should sum two integers stored in the accumulator and a register."""
        # Compute 10 + 32 = 42.
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_SMI, [10]),
                RegisterInstruction(Opcode.STAR, [0]),
                RegisterInstruction(Opcode.LDA_SMI, [32]),
                RegisterInstruction(Opcode.ADD, [0]),        # acc = acc + r0
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        result = execute(code)
        assert result.return_value == 42

    def test_add_strings_concatenation(self) -> None:
        """ADD with string operands should concatenate them.

        ADD computes ``acc + register``.  To get "hello world":
          r0 = " world"  (loaded first, stored as right operand)
          acc = "hello"  (loaded second, left operand)
          ADD r0         → acc = "hello" + " world" = "hello world"
        """
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_CONSTANT, [1]),  # acc = " world"
                RegisterInstruction(Opcode.STAR, [0]),          # r0 = " world"
                RegisterInstruction(Opcode.LDA_CONSTANT, [0]),  # acc = "hello"
                RegisterInstruction(Opcode.ADD, [0]),           # acc = "hello" + " world"
                RegisterInstruction(Opcode.RETURN),
            ],
            constants=["hello", " world"],
        )
        result = execute(code)
        assert result.return_value == "hello world"

    def test_sub_mul_div(self) -> None:
        """SUB, MUL, and DIV should perform basic numeric operations.

        Convention: ``run(acc_val, reg_val, op)`` computes ``acc_val OP reg_val``.
          r0 = reg_val   (loaded first)
          acc = acc_val  (loaded second, is left operand)
          OP r0          → acc = acc_val OP reg_val
        """
        def run(acc_val: int, reg_val: int, op: Opcode) -> object:
            code = simple_code(
                instructions=[
                    RegisterInstruction(Opcode.LDA_SMI, [reg_val]),
                    RegisterInstruction(Opcode.STAR, [0]),
                    RegisterInstruction(Opcode.LDA_SMI, [acc_val]),
                    RegisterInstruction(op, [0]),
                    RegisterInstruction(Opcode.RETURN),
                ],
            )
            return execute(code).return_value

        assert run(10, 3, Opcode.SUB) == 7    # 10 - 3 = 7
        assert run(4, 5, Opcode.MUL) == 20    # 4 * 5 = 20
        assert run(10, 4, Opcode.DIV) == 2.5  # 10 / 4 = 2.5

    def test_div_by_zero_raises(self) -> None:
        """DIV by zero should produce a VMError.

        Convention: r0 = 0 (the divisor), acc = 10 (the dividend).
        DIV r0 computes acc / r0 = 10 / 0 → VMError.
        """
        code2 = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_ZERO),
                RegisterInstruction(Opcode.STAR, [0]),          # r0 = 0
                RegisterInstruction(Opcode.LDA_SMI, [10]),      # acc = 10
                RegisterInstruction(Opcode.DIV, [0]),           # acc / r0 = 10 / 0
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        result = execute(code2)
        assert result.error is not None
        assert "zero" in result.error.message.lower()

    def test_mod_and_pow(self) -> None:
        """MOD and POW should work correctly.

        Convention: ``run2(acc_val, reg_val, op)`` computes ``acc_val OP reg_val``.
        """
        def run2(acc_val: int, reg_val: int, op: Opcode) -> object:
            code = simple_code(
                instructions=[
                    RegisterInstruction(Opcode.LDA_SMI, [reg_val]),
                    RegisterInstruction(Opcode.STAR, [0]),
                    RegisterInstruction(Opcode.LDA_SMI, [acc_val]),
                    RegisterInstruction(op, [0]),
                    RegisterInstruction(Opcode.RETURN),
                ],
            )
            return execute(code).return_value

        assert run2(10, 3, Opcode.MOD) == 1     # 10 % 3 = 1
        assert run2(2, 10, Opcode.POW) == 1024  # 2 ** 10 = 1024

    def test_add_smi(self) -> None:
        """ADD_SMI should add a literal integer to the accumulator."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_SMI, [100]),
                RegisterInstruction(Opcode.ADD_SMI, [5]),
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        assert execute(code).return_value == 105

    def test_negate(self) -> None:
        """NEGATE should flip the sign of the accumulator."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_SMI, [7]),
                RegisterInstruction(Opcode.NEGATE),
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        assert execute(code).return_value == -7

    def test_bitwise_and_or_xor_not(self) -> None:
        """Bitwise AND, OR, XOR, and NOT should perform integer bit operations."""
        def bitwise(a: int, b: int, op: Opcode) -> object:
            code = simple_code(
                instructions=[
                    RegisterInstruction(Opcode.LDA_SMI, [b]),
                    RegisterInstruction(Opcode.STAR, [0]),
                    RegisterInstruction(Opcode.LDA_SMI, [a]),
                    RegisterInstruction(op, [0]),
                    RegisterInstruction(Opcode.RETURN),
                ],
            )
            return execute(code).return_value

        assert bitwise(0b1100, 0b1010, Opcode.BITWISE_AND) == 0b1000
        assert bitwise(0b1100, 0b1010, Opcode.BITWISE_OR)  == 0b1110
        assert bitwise(0b1100, 0b1010, Opcode.BITWISE_XOR) == 0b0110

        code_not = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_SMI, [5]),
                RegisterInstruction(Opcode.BITWISE_NOT),
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        assert execute(code_not).return_value == ~5


# ===========================================================================
# 4. Comparisons
# ===========================================================================

class TestComparisons:
    """Tests for TEST_EQUAL, TEST_LESS_THAN, TEST_STRICT_EQUAL, LOGICAL_NOT, TYPEOF."""

    def test_test_equal(self) -> None:
        """TEST_EQUAL should return True when both operands are equal."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_SMI, [5]),
                RegisterInstruction(Opcode.STAR, [0]),
                RegisterInstruction(Opcode.LDA_SMI, [5]),
                RegisterInstruction(Opcode.TEST_EQUAL, [0]),
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        assert execute(code).return_value is True

    def test_test_less_than(self) -> None:
        """TEST_LESS_THAN should compare accumulator < register."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_SMI, [10]),
                RegisterInstruction(Opcode.STAR, [0]),     # r0 = 10
                RegisterInstruction(Opcode.LDA_SMI, [3]),  # acc = 3
                RegisterInstruction(Opcode.TEST_LESS_THAN, [0]),  # 3 < 10 → True
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        assert execute(code).return_value is True

    def test_typeof(self) -> None:
        """TYPEOF should return the JavaScript-style type name string."""
        expectations = [
            (RegisterInstruction(Opcode.LDA_SMI, [1]), "number"),
            (RegisterInstruction(Opcode.LDA_CONSTANT, [0]), "string"),
            (RegisterInstruction(Opcode.LDA_TRUE), "boolean"),
            (RegisterInstruction(Opcode.LDA_NULL), "null"),
            (RegisterInstruction(Opcode.LDA_UNDEFINED), "undefined"),
        ]
        for load_instr, expected_type in expectations:
            code = simple_code(
                instructions=[
                    load_instr,
                    RegisterInstruction(Opcode.TYPEOF),
                    RegisterInstruction(Opcode.RETURN),
                ],
                constants=["hello"],
            )
            result = execute(code)
            assert result.return_value == expected_type, (
                f"Expected typeof to be '{expected_type}', got {result.return_value!r}"
            )

    def test_logical_not(self) -> None:
        """LOGICAL_NOT should invert the truthiness of the accumulator."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_ZERO),
                RegisterInstruction(Opcode.LOGICAL_NOT),
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        assert execute(code).return_value is True


# ===========================================================================
# 5. Control flow — jumps and loops
# ===========================================================================

class TestControlFlow:
    """Tests for JUMP, JUMP_IF_TRUE, JUMP_IF_FALSE, and JUMP_LOOP."""

    def test_unconditional_jump(self) -> None:
        """JUMP should skip the next instruction unconditionally.

        Bytecode layout:
            ip=0  LDA_SMI 100
            ip=1  JUMP +1       ← jump skips ip=2
            ip=2  LDA_SMI 999   ← never executed
            ip=3  RETURN
        """
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_SMI, [100]),
                RegisterInstruction(Opcode.JUMP, [1]),
                RegisterInstruction(Opcode.LDA_SMI, [999]),  # skipped
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        result = execute(code)
        assert result.return_value == 100

    def test_jump_if_false(self) -> None:
        """JUMP_IF_FALSE should branch when accumulator is falsy.

        Simulates:
            if (false) { acc = 1 } else { acc = 2 }
        """
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_FALSE),         # acc = False
                RegisterInstruction(Opcode.JUMP_IF_FALSE, [2]), # false → skip 2 instr
                RegisterInstruction(Opcode.LDA_SMI, [1]),       # skipped (true branch)
                RegisterInstruction(Opcode.JUMP, [1]),           # skipped
                RegisterInstruction(Opcode.LDA_SMI, [2]),       # false branch
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        result = execute(code)
        assert result.return_value == 2

    def test_loop_with_jump_loop(self) -> None:
        """JUMP_LOOP should create a backward loop.

        Simulates the equivalent of:
            i = 0
            while i < 3:
                i += 1
            return i

        Register layout:
            r0 = loop counter i
            r1 = limit (3)

        Bytecode and jump-offset arithmetic:
            ip=0  LDA_ZERO
            ip=1  STAR 0              ; r0 = 0
            ip=2  LDA_SMI 3
            ip=3  STAR 1              ; r1 = 3
            -- loop condition (target of backward jump) --
            ip=4  LDAR 0              ; acc = r0
            ip=5  TEST_LESS_THAN 1   ; acc = (r0 < r1)
            ip=6  JUMP_IF_FALSE +4   ; pre-inc→ip=7, taken→ip=7+4=11 (exit)
            -- loop body --
            ip=7  LDAR 0
            ip=8  ADD_SMI 1          ; acc = r0 + 1
            ip=9  STAR 0             ; r0 = acc
            ip=10 JUMP_LOOP -7       ; pre-inc→ip=11, then ip=11+(-7)=4 ✓
            -- after loop --
            ip=11 LDAR 0
            ip=12 RETURN
        """
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_ZERO),            # ip=0
                RegisterInstruction(Opcode.STAR, [0]),           # ip=1  r0 = 0
                RegisterInstruction(Opcode.LDA_SMI, [3]),        # ip=2  acc = 3
                RegisterInstruction(Opcode.STAR, [1]),           # ip=3  r1 = 3
                # Loop condition
                RegisterInstruction(Opcode.LDAR, [0]),           # ip=4  acc = r0
                RegisterInstruction(Opcode.TEST_LESS_THAN, [1]), # ip=5  acc < r1?
                RegisterInstruction(Opcode.JUMP_IF_FALSE, [4]),  # ip=6  exit if not less
                # Loop body
                RegisterInstruction(Opcode.LDAR, [0]),           # ip=7  acc = r0
                RegisterInstruction(Opcode.ADD_SMI, [1]),        # ip=8  acc += 1
                RegisterInstruction(Opcode.STAR, [0]),           # ip=9  r0 = acc
                RegisterInstruction(Opcode.JUMP_LOOP, [-7]),     # ip=10 → ip=11-7=4
                # After loop
                RegisterInstruction(Opcode.LDAR, [0]),           # ip=11
                RegisterInstruction(Opcode.RETURN),              # ip=12
            ],
        )
        result = execute(code)
        assert result.error is None, f"Unexpected error: {result.error}"
        assert result.return_value == 3


# ===========================================================================
# 6. Variable access — globals and locals
# ===========================================================================

class TestVariableAccess:
    """Tests for LDA_GLOBAL, STA_GLOBAL, LDA_LOCAL, STA_LOCAL."""

    def test_global_store_and_load(self) -> None:
        """STA_GLOBAL followed by LDA_GLOBAL should round-trip a value."""
        vm = RegisterVM()

        store_code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_SMI, [77]),
                RegisterInstruction(Opcode.STA_GLOBAL, [0]),
                RegisterInstruction(Opcode.HALT),
            ],
            names=["myVar"],
        )
        vm.execute(store_code)

        load_code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_GLOBAL, [0]),
                RegisterInstruction(Opcode.RETURN),
            ],
            names=["myVar"],
        )
        result = vm.execute(load_code)
        assert result.return_value == 77

    def test_undefined_global_raises(self) -> None:
        """Loading an undefined global should produce a VMError."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_GLOBAL, [0]),
                RegisterInstruction(Opcode.RETURN),
            ],
            names=["doesNotExist"],
        )
        result = execute(code)
        assert result.error is not None
        assert "doesNotExist" in result.error.message

    def test_local_alias(self) -> None:
        """LDA_LOCAL and STA_LOCAL are aliases for LDAR and STAR."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_SMI, [9]),
                RegisterInstruction(Opcode.STA_LOCAL, [3]),   # r3 = 9
                RegisterInstruction(Opcode.LDA_ZERO),
                RegisterInstruction(Opcode.LDA_LOCAL, [3]),   # acc = r3
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        assert execute(code).return_value == 9


# ===========================================================================
# 7. Object and property access
# ===========================================================================

class TestObjects:
    """Tests for CREATE_OBJECT_LITERAL, LDA/STA_NAMED_PROPERTY, keyed access."""

    def test_create_and_read_named_property(self) -> None:
        """Creating an object and reading a property should return the stored value."""
        # Simulate: obj = {}; obj.x = 42; return obj.x
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.CREATE_OBJECT_LITERAL),
                RegisterInstruction(Opcode.STAR, [0]),               # r0 = obj
                RegisterInstruction(Opcode.LDA_SMI, [42]),           # acc = 42
                RegisterInstruction(Opcode.STA_NAMED_PROPERTY, [0, 0, 0]),  # obj.x = 42
                RegisterInstruction(Opcode.LDA_NAMED_PROPERTY, [0, 0, 1]),  # acc = obj.x
                RegisterInstruction(Opcode.RETURN),
            ],
            names=["x"],
        )
        result = execute(code)
        assert result.error is None
        assert result.return_value == 42

    def test_keyed_property_access(self) -> None:
        """Keyed property store and load should work on VMObject."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.CREATE_OBJECT_LITERAL),
                RegisterInstruction(Opcode.STAR, [0]),               # r0 = obj
                RegisterInstruction(Opcode.LDA_CONSTANT, [0]),       # acc = "key"
                RegisterInstruction(Opcode.STAR, [1]),               # r1 = "key"
                RegisterInstruction(Opcode.LDA_SMI, [99]),           # acc = 99
                RegisterInstruction(Opcode.STA_KEYED_PROPERTY, [0, 1, 0]),  # obj["key"] = 99
                RegisterInstruction(Opcode.LDA_KEYED_PROPERTY, [0, 1, 0]),  # acc = obj["key"]
                RegisterInstruction(Opcode.RETURN),
            ],
            constants=["key"],
        )
        result = execute(code)
        assert result.error is None
        assert result.return_value == 99

    def test_array_length_property(self) -> None:
        """Reading the .length property of an array should return its size."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.CREATE_ARRAY_LITERAL),
                RegisterInstruction(Opcode.STAR, [0]),   # r0 = []
                # Push three elements via keyed store.
                RegisterInstruction(Opcode.LDA_SMI, [0]),
                RegisterInstruction(Opcode.STAR, [1]),   # r1 = 0 (key)
                RegisterInstruction(Opcode.LDA_SMI, [10]),
                RegisterInstruction(Opcode.STA_KEYED_PROPERTY, [0, 1, 0]),  # arr[0] = 10
                RegisterInstruction(Opcode.LDA_SMI, [1]),
                RegisterInstruction(Opcode.STAR, [1]),
                RegisterInstruction(Opcode.LDA_SMI, [20]),
                RegisterInstruction(Opcode.STA_KEYED_PROPERTY, [0, 1, 0]),  # arr[1] = 20
                RegisterInstruction(Opcode.LDA_NAMED_PROPERTY, [0, 0, 0]),  # acc = arr.length
                RegisterInstruction(Opcode.RETURN),
            ],
            names=["length"],
        )
        result = execute(code)
        assert result.error is None
        assert result.return_value == 2


# ===========================================================================
# 8. Function calls (closures and first-class functions)
# ===========================================================================

class TestFunctionCalls:
    """Tests for CREATE_CLOSURE, CALL_ANY_RECEIVER, and RETURN."""

    def test_create_and_call_closure(self) -> None:
        """A closure created with CREATE_CLOSURE should be callable.

        Inner function: return 7 (no parameters, no captured variables).
        Outer function: create closure, call it, return result.
        """
        inner = CodeObject(
            instructions=[
                RegisterInstruction(Opcode.LDA_SMI, [7]),
                RegisterInstruction(Opcode.RETURN),
            ],
            constants=[],
            names=[],
            register_count=0,
            feedback_slot_count=0,
            name="inner",
        )

        outer = simple_code(
            instructions=[
                RegisterInstruction(Opcode.CREATE_CLOSURE, [0]),  # acc = VMFunction(inner)
                RegisterInstruction(Opcode.STAR, [0]),            # r0 = closure
                # Call r0 with 0 arguments.
                RegisterInstruction(Opcode.CALL_ANY_RECEIVER, [0, 1, 0, 0]),
                RegisterInstruction(Opcode.RETURN),
            ],
            constants=[inner],
        )
        result = execute(outer)
        assert result.error is None, f"Unexpected error: {result.error}"
        assert result.return_value == 7

    def test_builtin_print_captures_output(self) -> None:
        """Calling the built-in ``print`` global should append to result.output."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_GLOBAL, [0]),     # acc = print
                RegisterInstruction(Opcode.STAR, [0]),            # r0 = print
                RegisterInstruction(Opcode.LDA_CONSTANT, [0]),   # acc = "hello"
                RegisterInstruction(Opcode.STAR, [1]),            # r1 = "hello"
                # Call print with 1 arg.
                RegisterInstruction(Opcode.CALL_ANY_RECEIVER, [0, 1, 1, 0]),
                RegisterInstruction(Opcode.HALT),
            ],
            names=["print"],
            constants=["hello"],
        )
        result = execute(code)
        assert result.error is None
        assert result.output == ["hello"]

    def test_stack_overflow_raises_vm_error(self) -> None:
        """Deeply recursive calls should raise a stack-overflow VMError."""
        # Create a function that calls itself forever.
        # It uses STACK_CHECK to detect overflow.
        recurse = CodeObject(
            instructions=[
                RegisterInstruction(Opcode.STACK_CHECK),
                # Load self from global and call.
                RegisterInstruction(Opcode.LDA_GLOBAL, [0]),
                RegisterInstruction(Opcode.STAR, [0]),
                RegisterInstruction(Opcode.CALL_ANY_RECEIVER, [0, 1, 0, 0]),
                RegisterInstruction(Opcode.RETURN),
            ],
            constants=[],
            names=["recurse"],
            register_count=2,
            feedback_slot_count=1,
            name="recurse",
        )

        vm = RegisterVM(max_depth=10)
        # First, store the function in the global named "recurse".
        vm._globals["recurse"] = VMFunction(code=recurse, context=None)

        call_code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_GLOBAL, [0]),
                RegisterInstruction(Opcode.STAR, [0]),
                RegisterInstruction(Opcode.CALL_ANY_RECEIVER, [0, 1, 0, 0]),
                RegisterInstruction(Opcode.RETURN),
            ],
            names=["recurse"],
        )
        result = vm.execute(call_code)
        assert result.error is not None
        assert "depth" in result.error.message.lower() or "stack" in result.error.message.lower()


# ===========================================================================
# 9. Feedback vectors and inline caches
# ===========================================================================

class TestFeedbackVectors:
    """Tests for the type-feedback recording machinery."""

    def test_binary_op_feedback_monomorphic(self) -> None:
        """Adding two integers should transition the feedback slot to Monomorphic."""
        vec = new_vector(2)
        record_binary_op(vec, 0, 5, 3)
        assert isinstance(vec[0], SlotMonomorphic)
        assert vec[0].types == [("number", "number")]

    def test_binary_op_feedback_polymorphic(self) -> None:
        """Seeing two different type pairs should transition to Polymorphic."""
        vec = new_vector(1)
        record_binary_op(vec, 0, 5, 3)          # number + number
        record_binary_op(vec, 0, "hi", " there") # string + string
        assert isinstance(vec[0], SlotPolymorphic)
        assert len(vec[0].types) == 2

    def test_binary_op_feedback_megamorphic(self) -> None:
        """Seeing 5+ distinct type pairs should transition to Megamorphic.

        We need 5 *distinct* (lhs_type, rhs_type) pairs.  Pairs that map to
        the same type names are deduplicated and don't advance the state.

        Distinct pairs used:
            1. ("number", "number")      — 1 + 2
            2. ("string", "string")      — "a" + "b"
            3. ("boolean", "boolean")    — True + False
            4. ("number", "string")      — 1 + "x"
            5. ("string", "number")      — "y" + 2
        """
        vec = new_vector(1)
        pairs = [
            (1, 2),           # ("number", "number")
            ("a", "b"),       # ("string", "string")
            (True, False),    # ("boolean", "boolean")
            (1, "x"),         # ("number", "string")
            ("y", 2),         # ("string", "number")
        ]
        for left, right in pairs:
            record_binary_op(vec, 0, left, right)
        assert isinstance(vec[0], SlotMegamorphic)

    def test_property_load_feedback(self) -> None:
        """Property load should record the hidden class ID in the feedback slot."""
        vec = new_vector(1)
        record_property_load(vec, 0, hidden_class_id=42)
        assert isinstance(vec[0], SlotMonomorphic)
        assert vec[0].types[0] == ("object_42", "property")

    def test_value_type_mapping(self) -> None:
        """value_type() should return correct JS-style type names."""
        assert value_type(0) == "number"
        assert value_type(3.14) == "number"
        assert value_type("s") == "string"
        assert value_type(True) == "boolean"
        assert value_type(None) == "null"
        assert value_type(UNDEFINED) == "undefined"
        assert value_type(VMObject(0)) == "object"
        assert value_type([]) == "array"


# ===========================================================================
# 10. Execution tracing
# ===========================================================================

class TestTracing:
    """Tests for execute_with_trace — verifies the trace structure."""

    def test_trace_has_correct_length(self) -> None:
        """Trace should have one entry per executed instruction."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_SMI, [1]),
                RegisterInstruction(Opcode.ADD_SMI, [2]),
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        result, trace = execute_with_trace(code)
        assert result.error is None
        # Expect 3 trace steps (one per instruction).
        assert len(trace) == 3

    def test_trace_accumulator_progression(self) -> None:
        """Each TraceStep should capture acc_before and acc_after correctly."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_SMI, [10]),
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        _, trace = execute_with_trace(code)
        # First step: LDA_SMI — acc goes from UNDEFINED to 10.
        assert trace[0].acc_before is UNDEFINED
        assert trace[0].acc_after == 10
        # Second step: RETURN — acc stays 10.
        assert trace[1].acc_before == 10
        assert trace[1].acc_after == 10

    def test_trace_frame_depth(self) -> None:
        """Top-level code should have frame_depth=0 in all trace steps."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_ZERO),
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        _, trace = execute_with_trace(code)
        for step in trace:
            assert step.frame_depth == 0


# ===========================================================================
# 11. Scope / context
# ===========================================================================

class TestScope:
    """Tests for the lexical context chain (new_context, get_slot, set_slot)."""

    def test_context_read_write_depth_zero(self) -> None:
        """Reading and writing depth-0 slots should work correctly."""
        ctx = new_context(None, 3)
        set_slot(ctx, 0, 1, "hello")
        assert get_slot(ctx, 0, 1) == "hello"

    def test_context_parent_chain(self) -> None:
        """get_slot with depth=1 should read from the parent context."""
        outer = new_context(None, 2)
        outer.slots[0] = 99
        inner = new_context(outer, 1)
        assert get_slot(inner, 1, 0) == 99

    def test_context_depth_out_of_range_raises(self) -> None:
        """Walking past the end of the context chain should raise IndexError."""
        ctx = new_context(None, 1)
        with pytest.raises(IndexError):
            get_slot(ctx, 5, 0)

    def test_create_and_pop_context_in_vm(self) -> None:
        """CREATE_CONTEXT and STA/LDA_CURRENT_CONTEXT_SLOT should store/retrieve values."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.CREATE_CONTEXT, [2]),     # push ctx with 2 slots
                RegisterInstruction(Opcode.LDA_SMI, [42]),
                RegisterInstruction(Opcode.STA_CURRENT_CONTEXT_SLOT, [0]),  # ctx[0] = 42
                RegisterInstruction(Opcode.LDA_ZERO),                # clear acc
                RegisterInstruction(Opcode.LDA_CURRENT_CONTEXT_SLOT, [0]), # acc = ctx[0]
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        result = execute(code)
        assert result.error is None
        assert result.return_value == 42


# ===========================================================================
# 12. Error handling — THROW and unknown opcodes
# ===========================================================================

class TestErrors:
    """Tests for THROW, unknown opcodes, and type errors."""

    def test_throw_raises_vm_error(self) -> None:
        """THROW should create a VMError with the accumulator's string value."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_CONSTANT, [0]),
                RegisterInstruction(Opcode.THROW),
            ],
            constants=["something went wrong"],
        )
        result = execute(code)
        assert result.error is not None
        assert "something went wrong" in result.error.message

    def test_unknown_opcode_raises(self) -> None:
        """An unrecognised opcode byte should raise a VMError."""
        code = simple_code(
            instructions=[
                RegisterInstruction(0xCC),  # not a valid opcode
            ],
        )
        result = execute(code)
        assert result.error is not None

    def test_add_incompatible_types_raises(self) -> None:
        """ADD with None and an integer should raise a VMError."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_SMI, [5]),
                RegisterInstruction(Opcode.STAR, [0]),
                RegisterInstruction(Opcode.LDA_NULL),       # acc = None
                RegisterInstruction(Opcode.ADD, [0]),        # None + 5 → error
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        result = execute(code)
        assert result.error is not None

    def test_halt_terminates_execution(self) -> None:
        """HALT should stop execution and return the current accumulator."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_SMI, [50]),
                RegisterInstruction(Opcode.HALT),
                RegisterInstruction(Opcode.LDA_SMI, [999]),  # never reached
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        result = execute(code)
        assert result.return_value == 50


# ===========================================================================
# 13. Additional coverage — comparisons, shifts, context slots, closures
# ===========================================================================

class TestAdditionalCoverage:
    """Extra tests designed to push coverage of vm.py and feedback.py above 80%."""

    # ------------------------------------------------------------------
    # More comparison opcodes
    # ------------------------------------------------------------------

    def test_test_not_equal(self) -> None:
        """TEST_NOT_EQUAL should return True when values differ."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_SMI, [3]),
                RegisterInstruction(Opcode.STAR, [0]),
                RegisterInstruction(Opcode.LDA_SMI, [5]),
                RegisterInstruction(Opcode.TEST_NOT_EQUAL, [0]),
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        assert execute(code).return_value is True

    def test_test_strict_equal(self) -> None:
        """TEST_STRICT_EQUAL should be True only if same type and value."""
        # 0 strictly equals 0 (both int).
        code_eq = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_ZERO),
                RegisterInstruction(Opcode.STAR, [0]),
                RegisterInstruction(Opcode.LDA_ZERO),
                RegisterInstruction(Opcode.TEST_STRICT_EQUAL, [0]),
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        assert execute(code_eq).return_value is True

    def test_test_strict_not_equal(self) -> None:
        """TEST_STRICT_NOT_EQUAL should be True when types differ."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_SMI, [1]),
                RegisterInstruction(Opcode.STAR, [0]),
                RegisterInstruction(Opcode.LDA_SMI, [2]),
                RegisterInstruction(Opcode.TEST_STRICT_NOT_EQUAL, [0]),
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        assert execute(code).return_value is True

    def test_test_greater_than(self) -> None:
        """TEST_GREATER_THAN: acc > reg."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_SMI, [2]),
                RegisterInstruction(Opcode.STAR, [0]),     # r0 = 2
                RegisterInstruction(Opcode.LDA_SMI, [5]),  # acc = 5
                RegisterInstruction(Opcode.TEST_GREATER_THAN, [0]),
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        assert execute(code).return_value is True

    def test_test_lte_and_gte(self) -> None:
        """TEST_LESS_THAN_OR_EQUAL and TEST_GREATER_THAN_OR_EQUAL."""
        def compare(acc_val: int, reg_val: int, op: Opcode) -> object:
            code = simple_code(
                instructions=[
                    RegisterInstruction(Opcode.LDA_SMI, [reg_val]),
                    RegisterInstruction(Opcode.STAR, [0]),
                    RegisterInstruction(Opcode.LDA_SMI, [acc_val]),
                    RegisterInstruction(op, [0]),
                    RegisterInstruction(Opcode.RETURN),
                ],
            )
            return execute(code).return_value

        assert compare(3, 3, Opcode.TEST_LESS_THAN_OR_EQUAL) is True
        assert compare(3, 3, Opcode.TEST_GREATER_THAN_OR_EQUAL) is True
        assert compare(2, 3, Opcode.TEST_LESS_THAN_OR_EQUAL) is True
        assert compare(4, 3, Opcode.TEST_GREATER_THAN_OR_EQUAL) is True

    def test_test_in_list(self) -> None:
        """TEST_IN should check membership in a list."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.CREATE_ARRAY_LITERAL),
                RegisterInstruction(Opcode.STAR, [0]),          # r0 = []
                RegisterInstruction(Opcode.LDA_SMI, [0]),
                RegisterInstruction(Opcode.STAR, [1]),          # r1 = index 0
                RegisterInstruction(Opcode.LDA_SMI, [42]),
                RegisterInstruction(Opcode.STA_KEYED_PROPERTY, [0, 1, 0]),  # arr[0] = 42
                RegisterInstruction(Opcode.LDA_SMI, [42]),      # acc = 42
                RegisterInstruction(Opcode.TEST_IN, [0]),       # 42 in arr?
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        result = execute(code)
        assert result.error is None
        assert result.return_value is True

    def test_test_undetectable(self) -> None:
        """TEST_UNDETECTABLE should be True for None and UNDEFINED."""
        code_null = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_NULL),
                RegisterInstruction(Opcode.TEST_UNDETECTABLE),
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        code_undef = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_UNDEFINED),
                RegisterInstruction(Opcode.TEST_UNDETECTABLE),
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        assert execute(code_null).return_value is True
        assert execute(code_undef).return_value is True

    def test_test_instanceof(self) -> None:
        """TEST_INSTANCEOF: True when types match."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_SMI, [1]),
                RegisterInstruction(Opcode.STAR, [0]),
                RegisterInstruction(Opcode.LDA_SMI, [2]),
                RegisterInstruction(Opcode.TEST_INSTANCEOF, [0]),
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        assert execute(code).return_value is True

    # ------------------------------------------------------------------
    # Shift operations
    # ------------------------------------------------------------------

    def test_shift_left_and_right(self) -> None:
        """SHIFT_LEFT, SHIFT_RIGHT, and SHIFT_RIGHT_LOGICAL."""
        def shift(val: int, amount: int, op: Opcode) -> object:
            code = simple_code(
                instructions=[
                    RegisterInstruction(Opcode.LDA_SMI, [amount]),
                    RegisterInstruction(Opcode.STAR, [0]),
                    RegisterInstruction(Opcode.LDA_SMI, [val]),
                    RegisterInstruction(op, [0]),
                    RegisterInstruction(Opcode.RETURN),
                ],
            )
            return execute(code).return_value

        assert shift(4, 2, Opcode.SHIFT_LEFT) == 16     # 4 << 2 = 16
        assert shift(16, 2, Opcode.SHIFT_RIGHT) == 4    # 16 >> 2 = 4
        assert shift(16, 2, Opcode.SHIFT_RIGHT_LOGICAL) == 4  # unsigned

    def test_sub_smi(self) -> None:
        """SUB_SMI should subtract a literal from the accumulator."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_SMI, [10]),
                RegisterInstruction(Opcode.SUB_SMI, [3]),
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        assert execute(code).return_value == 7

    # ------------------------------------------------------------------
    # Context slot opcodes
    # ------------------------------------------------------------------

    def test_lda_sta_context_slot(self) -> None:
        """LDA_CONTEXT_SLOT and STA_CONTEXT_SLOT at depth=0."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.CREATE_CONTEXT, [3]),
                RegisterInstruction(Opcode.LDA_SMI, [77]),
                RegisterInstruction(Opcode.STA_CONTEXT_SLOT, [0, 1]),   # ctx[depth=0][slot=1] = 77
                RegisterInstruction(Opcode.LDA_SMI, [0]),
                RegisterInstruction(Opcode.LDA_CONTEXT_SLOT, [0, 1]),   # acc = ctx[0][1]
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        result = execute(code)
        assert result.error is None
        assert result.return_value == 77

    def test_pop_context(self) -> None:
        """POP_CONTEXT should restore the parent context."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.CREATE_CONTEXT, [1]),       # push outer ctx
                RegisterInstruction(Opcode.LDA_SMI, [11]),
                RegisterInstruction(Opcode.STA_CURRENT_CONTEXT_SLOT, [0]),  # outer[0] = 11
                RegisterInstruction(Opcode.CREATE_CONTEXT, [1]),       # push inner ctx
                RegisterInstruction(Opcode.LDA_SMI, [22]),
                RegisterInstruction(Opcode.STA_CURRENT_CONTEXT_SLOT, [0]),  # inner[0] = 22
                RegisterInstruction(Opcode.POP_CONTEXT),               # pop inner
                RegisterInstruction(Opcode.LDA_CURRENT_CONTEXT_SLOT, [0]),  # now outer[0]
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        result = execute(code)
        assert result.error is None
        assert result.return_value == 11

    # ------------------------------------------------------------------
    # Module variables
    # ------------------------------------------------------------------

    def test_lda_sta_module_variable(self) -> None:
        """LDA/STA_MODULE_VARIABLE should alias globals."""
        vm = RegisterVM()
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_SMI, [55]),
                RegisterInstruction(Opcode.STA_MODULE_VARIABLE, [0]),
                RegisterInstruction(Opcode.LDA_ZERO),
                RegisterInstruction(Opcode.LDA_MODULE_VARIABLE, [0]),
                RegisterInstruction(Opcode.RETURN),
            ],
            names=["modVar"],
        )
        result = vm.execute(code)
        assert result.error is None
        assert result.return_value == 55

    # ------------------------------------------------------------------
    # Jump variants
    # ------------------------------------------------------------------

    def test_jump_if_true(self) -> None:
        """JUMP_IF_TRUE should branch when accumulator is truthy."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_TRUE),
                RegisterInstruction(Opcode.JUMP_IF_TRUE, [1]),   # skip next
                RegisterInstruction(Opcode.LDA_SMI, [0]),        # skipped
                RegisterInstruction(Opcode.LDA_SMI, [99]),
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        assert execute(code).return_value == 99

    def test_jump_if_null(self) -> None:
        """JUMP_IF_NULL should branch when accumulator is None."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_NULL),
                RegisterInstruction(Opcode.JUMP_IF_NULL, [1]),
                RegisterInstruction(Opcode.LDA_SMI, [0]),   # skipped
                RegisterInstruction(Opcode.LDA_SMI, [7]),
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        assert execute(code).return_value == 7

    def test_jump_if_undefined(self) -> None:
        """JUMP_IF_UNDEFINED should branch when accumulator is UNDEFINED."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_UNDEFINED),
                RegisterInstruction(Opcode.JUMP_IF_UNDEFINED, [1]),
                RegisterInstruction(Opcode.LDA_SMI, [0]),   # skipped
                RegisterInstruction(Opcode.LDA_SMI, [8]),
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        assert execute(code).return_value == 8

    def test_jump_if_null_or_undefined(self) -> None:
        """JUMP_IF_NULL_OR_UNDEFINED should branch for both null and undefined."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_NULL),
                RegisterInstruction(Opcode.JUMP_IF_NULL_OR_UNDEFINED, [1]),
                RegisterInstruction(Opcode.LDA_SMI, [0]),   # skipped
                RegisterInstruction(Opcode.LDA_SMI, [9]),
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        assert execute(code).return_value == 9

    def test_jump_if_to_boolean_true_false(self) -> None:
        """JUMP_IF_TO_BOOLEAN_TRUE/FALSE mirror JUMP_IF_TRUE/FALSE."""
        code_t = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_SMI, [1]),
                RegisterInstruction(Opcode.JUMP_IF_TO_BOOLEAN_TRUE, [1]),
                RegisterInstruction(Opcode.LDA_SMI, [0]),  # skipped
                RegisterInstruction(Opcode.LDA_SMI, [5]),
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        assert execute(code_t).return_value == 5

        code_f = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_ZERO),
                RegisterInstruction(Opcode.JUMP_IF_TO_BOOLEAN_FALSE, [1]),
                RegisterInstruction(Opcode.LDA_SMI, [0]),  # skipped
                RegisterInstruction(Opcode.LDA_SMI, [6]),
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        assert execute(code_f).return_value == 6

    # ------------------------------------------------------------------
    # Property access edge cases
    # ------------------------------------------------------------------

    def test_named_property_no_feedback(self) -> None:
        """LDA/STA_NAMED_PROPERTY_NO_FEEDBACK should work without IC slots."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.CREATE_OBJECT_LITERAL),
                RegisterInstruction(Opcode.STAR, [0]),
                RegisterInstruction(Opcode.LDA_SMI, [123]),
                RegisterInstruction(Opcode.STA_NAMED_PROPERTY_NO_FEEDBACK, [0, 0]),
                RegisterInstruction(Opcode.LDA_ZERO),
                RegisterInstruction(Opcode.LDA_NAMED_PROPERTY_NO_FEEDBACK, [0, 0]),
                RegisterInstruction(Opcode.RETURN),
            ],
            names=["val"],
        )
        result = execute(code)
        assert result.error is None
        assert result.return_value == 123

    def test_delete_property(self) -> None:
        """DELETE_PROPERTY_STRICT and DELETE_PROPERTY_SLOPPY should remove properties."""
        for opcode in (Opcode.DELETE_PROPERTY_STRICT, Opcode.DELETE_PROPERTY_SLOPPY):
            code = simple_code(
                instructions=[
                    RegisterInstruction(Opcode.CREATE_OBJECT_LITERAL),
                    RegisterInstruction(Opcode.STAR, [0]),
                    RegisterInstruction(Opcode.LDA_CONSTANT, [0]),  # "x"
                    RegisterInstruction(Opcode.STAR, [1]),
                    RegisterInstruction(Opcode.LDA_SMI, [1]),
                    RegisterInstruction(Opcode.STA_KEYED_PROPERTY, [0, 1, 0]),
                    RegisterInstruction(opcode, [0, 1]),             # delete obj["x"]
                    RegisterInstruction(Opcode.RETURN),
                ],
                constants=["x"],
            )
            result = execute(code)
            assert result.error is None

    def test_clone_object(self) -> None:
        """CLONE_OBJECT should produce an independent copy of a VMObject."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.CREATE_OBJECT_LITERAL),
                RegisterInstruction(Opcode.STAR, [0]),           # r0 = original
                RegisterInstruction(Opcode.LDA_SMI, [42]),
                RegisterInstruction(Opcode.STA_NAMED_PROPERTY, [0, 0, 0]),  # original.val = 42
                RegisterInstruction(Opcode.LDAR, [0]),           # acc = original
                RegisterInstruction(Opcode.CLONE_OBJECT),        # acc = clone
                RegisterInstruction(Opcode.RETURN),
            ],
            names=["val"],
        )
        result = execute(code)
        assert result.error is None
        assert isinstance(result.return_value, VMObject)
        assert result.return_value.properties.get("val") == 42

    # ------------------------------------------------------------------
    # Object creation misc
    # ------------------------------------------------------------------

    def test_create_regexp_literal(self) -> None:
        """CREATE_REGEXP_LITERAL should return the pattern from the constant pool."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.CREATE_REGEXP_LITERAL, [0]),
                RegisterInstruction(Opcode.RETURN),
            ],
            constants=["[a-z]+"],
        )
        assert execute(code).return_value == "[a-z]+"

    def test_create_closure_and_call(self) -> None:
        """CREATE_CLOSURE should wrap a CodeObject in a VMFunction."""
        inner = CodeObject(
            instructions=[
                RegisterInstruction(Opcode.LDA_SMI, [42]),
                RegisterInstruction(Opcode.RETURN),
            ],
            constants=[], names=[], register_count=0, feedback_slot_count=0,
        )
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.CREATE_CLOSURE, [0]),
                RegisterInstruction(Opcode.RETURN),
            ],
            constants=[inner],
        )
        result = execute(code)
        assert result.error is None
        assert isinstance(result.return_value, VMFunction)

    # ------------------------------------------------------------------
    # Feedback helpers — call site and edge cases
    # ------------------------------------------------------------------

    def test_record_call_site(self) -> None:
        """record_call_site should record the callee type in the feedback slot."""
        vec = new_vector(2)
        record_call_site(vec, 0, "function")
        assert isinstance(vec[0], SlotMonomorphic)
        assert vec[0].types[0] == ("function", "call")

    def test_feedback_slot_out_of_range_ignored(self) -> None:
        """Recording feedback to a slot outside the vector range should be a no-op."""
        vec = new_vector(2)
        record_binary_op(vec, 99, 1, 2)   # slot 99 doesn't exist — no crash
        record_binary_op(vec, -1, 1, 2)   # negative slot — no crash

    def test_feedback_megamorphic_stays(self) -> None:
        """A megamorphic slot should remain megamorphic on further updates."""
        vec = new_vector(1)
        # Push past the 4-pair threshold.
        record_binary_op(vec, 0, 1, 2)
        record_binary_op(vec, 0, "a", "b")
        record_binary_op(vec, 0, True, False)
        record_binary_op(vec, 0, 1, "x")
        record_binary_op(vec, 0, "y", 2)
        assert isinstance(vec[0], SlotMegamorphic)
        # One more update — should stay megamorphic.
        record_binary_op(vec, 0, None, None)
        assert isinstance(vec[0], SlotMegamorphic)

    def test_monomorphic_deduplication(self) -> None:
        """Seeing the same type pair twice should not advance the slot state."""
        vec = new_vector(1)
        record_binary_op(vec, 0, 1, 2)   # number + number
        record_binary_op(vec, 0, 3, 4)   # number + number (same pair)
        assert isinstance(vec[0], SlotMonomorphic)

    # ------------------------------------------------------------------
    # STACK_CHECK opcode
    # ------------------------------------------------------------------

    def test_stack_check_passes_within_depth(self) -> None:
        """STACK_CHECK should not raise when call depth is within limit."""
        vm = RegisterVM(max_depth=100)
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.STACK_CHECK),
                RegisterInstruction(Opcode.LDA_SMI, [1]),
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        result = vm.execute(code)
        assert result.error is None
        assert result.return_value == 1

    # ------------------------------------------------------------------
    # DEBUGGER opcode (no-op)
    # ------------------------------------------------------------------

    def test_debugger_is_noop(self) -> None:
        """DEBUGGER should not affect the accumulator or registers."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_SMI, [77]),
                RegisterInstruction(Opcode.DEBUGGER),
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        assert execute(code).return_value == 77

    # ------------------------------------------------------------------
    # Implicit HALT at end of instructions
    # ------------------------------------------------------------------

    def test_implicit_halt(self) -> None:
        """Falling off the end of instructions should return the accumulator."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_SMI, [33]),
                # No RETURN or HALT — should return 33 implicitly.
            ],
        )
        result = execute(code)
        assert result.error is None
        assert result.return_value == 33

    # ------------------------------------------------------------------
    # Construct opcode
    # ------------------------------------------------------------------

    def test_construct_creates_object(self) -> None:
        """CONSTRUCT should produce a new VMObject as the result."""
        # A constructor that doesn't explicitly return an object.
        constructor = CodeObject(
            instructions=[
                RegisterInstruction(Opcode.LDA_SMI, [1]),
                RegisterInstruction(Opcode.RETURN),   # returns int — VM uses new_obj
            ],
            constants=[], names=[], register_count=2, feedback_slot_count=0,
        )
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.CREATE_CLOSURE, [0]),
                RegisterInstruction(Opcode.STAR, [0]),              # r0 = constructor fn
                # CONSTRUCT [callable_reg=0, first_arg_reg=1, argc=0]
                RegisterInstruction(Opcode.CONSTRUCT, [0, 1, 0]),
                RegisterInstruction(Opcode.RETURN),
            ],
            constants=[constructor],
        )
        result = execute(code)
        assert result.error is None
        assert isinstance(result.return_value, VMObject)

    # ------------------------------------------------------------------
    # Error: unsupported spread / generator / rethrow
    # ------------------------------------------------------------------

    def test_spread_call_raises(self) -> None:
        """CALL_WITH_SPREAD should raise a VMError."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.CALL_WITH_SPREAD, [0, 0, 0]),
            ],
        )
        assert execute(code).error is not None

    def test_suspend_generator_raises(self) -> None:
        """SUSPEND_GENERATOR should raise a VMError."""
        code = simple_code(
            instructions=[RegisterInstruction(Opcode.SUSPEND_GENERATOR)],
        )
        assert execute(code).error is not None

    def test_rethrow_raises(self) -> None:
        """RETHROW should raise a VMError."""
        code = simple_code(
            instructions=[RegisterInstruction(Opcode.RETHROW)],
        )
        assert execute(code).error is not None

    # ------------------------------------------------------------------
    # Iteration protocol
    # ------------------------------------------------------------------

    def test_get_iterator_and_step(self) -> None:
        """GET_ITERATOR and CALL_ITERATOR_STEP should iterate over a list."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.CREATE_ARRAY_LITERAL),
                RegisterInstruction(Opcode.STAR, [0]),          # r0 = []
                RegisterInstruction(Opcode.LDA_SMI, [0]),
                RegisterInstruction(Opcode.STAR, [1]),
                RegisterInstruction(Opcode.LDA_SMI, [10]),
                RegisterInstruction(Opcode.STA_KEYED_PROPERTY, [0, 1, 0]),  # arr[0]=10
                RegisterInstruction(Opcode.LDAR, [0]),
                RegisterInstruction(Opcode.GET_ITERATOR),       # acc = iter(arr)
                RegisterInstruction(Opcode.CALL_ITERATOR_STEP), # acc = {done, value}
                RegisterInstruction(Opcode.GET_ITERATOR_VALUE), # acc = step.value
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        result = execute(code)
        assert result.error is None
        assert result.return_value == 10

    def test_get_iterator_done(self) -> None:
        """GET_ITERATOR_DONE on an exhausted iterator should return True."""
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.CREATE_ARRAY_LITERAL),  # empty list
                RegisterInstruction(Opcode.GET_ITERATOR),           # iter([])
                RegisterInstruction(Opcode.CALL_ITERATOR_STEP),     # first step → done=True
                RegisterInstruction(Opcode.GET_ITERATOR_DONE),      # acc = True
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        result = execute(code)
        assert result.error is None
        assert result.return_value is True

    # ------------------------------------------------------------------
    # CALL_PROPERTY
    # ------------------------------------------------------------------

    def test_call_property(self) -> None:
        """CALL_PROPERTY should invoke a callable from a register."""
        fn = CodeObject(
            instructions=[
                RegisterInstruction(Opcode.LDA_SMI, [7]),
                RegisterInstruction(Opcode.RETURN),
            ],
            constants=[], names=[], register_count=0, feedback_slot_count=0,
        )
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.CREATE_CLOSURE, [0]),
                RegisterInstruction(Opcode.STAR, [0]),  # r0 = fn
                RegisterInstruction(Opcode.LDA_ZERO),
                RegisterInstruction(Opcode.STAR, [1]),  # r1 = receiver (not used)
                # CALL_PROPERTY [callable_reg=0, receiver_reg=1, first_arg_reg=2, argc=0, slot=0]
                RegisterInstruction(Opcode.CALL_PROPERTY, [0, 1, 2, 0, 0]),
                RegisterInstruction(Opcode.RETURN),
            ],
            constants=[fn],
            register_count=4,
        )
        result = execute(code)
        assert result.error is None
        assert result.return_value == 7

    # ------------------------------------------------------------------
    # vm.py _strict_equal and _is_truthy helpers via VM instructions
    # ------------------------------------------------------------------

    def test_strict_equal_different_types(self) -> None:
        """Strict equality should be False when types differ (bool vs int)."""
        # In Python, 0 == False is True but _strict_equal(0, False) should be False.
        code = simple_code(
            instructions=[
                RegisterInstruction(Opcode.LDA_FALSE),
                RegisterInstruction(Opcode.STAR, [0]),   # r0 = False (bool)
                RegisterInstruction(Opcode.LDA_ZERO),    # acc = 0 (int)
                RegisterInstruction(Opcode.TEST_STRICT_EQUAL, [0]),
                RegisterInstruction(Opcode.RETURN),
            ],
        )
        # 0 (int) strictly != False (bool) — different Python types.
        assert execute(code).return_value is False


# ===========================================================================
# GenericRegisterVM tests
# ===========================================================================
"""Tests for the pluggable GenericRegisterVM chassis (generic_vm.py).

These tests verify the dispatch loop, trace hook, halt/ret signals,
and the user_data extension point independently of any language backend.
"""


# ---------------------------------------------------------------------------
# Minimal instruction type for tests
# ---------------------------------------------------------------------------

@dataclass
class Instr:
    """Minimal instruction: just opcode + operands."""
    opcode: int
    operands: list[int]


# Opcode constants for test mini-language
OP_LOAD = 0x01   # acc = operands[0]
OP_ADD  = 0x02   # acc = acc + registers[operands[0]]
OP_SAVE = 0x03   # registers[operands[0]] = acc
OP_HALT = 0xFF   # stop
OP_RET  = 0xFE   # ret(acc)
OP_INC  = 0x04   # acc += 1 (no operands)
OP_JMP  = 0x05   # ip += operands[0] (relative)
OP_SET_UD = 0x06  # frame.user_data["x"] = operands[0]


def _build_grvm() -> GenericRegisterVM:
    """Build a tiny test VM with the 8 opcodes above."""
    g = GenericRegisterVM()
    g.register_handler(OP_LOAD,   lambda grv, f, i: setattr(f, "acc", i.operands[0]))
    g.register_handler(OP_ADD,    lambda grv, f, i: setattr(f, "acc", f.acc + f.registers[i.operands[0]]))
    g.register_handler(OP_SAVE,   lambda grv, f, i: f.registers.__setitem__(i.operands[0], f.acc))
    g.register_handler(OP_HALT,   lambda grv, f, i: grv.halt())
    g.register_handler(OP_RET,    lambda grv, f, i: grv.ret(f.acc))
    g.register_handler(OP_INC,    lambda grv, f, i: setattr(f, "acc", f.acc + 1))
    g.register_handler(OP_JMP,    lambda grv, f, i: setattr(f, "ip", f.ip + i.operands[0]))
    g.register_handler(OP_SET_UD, lambda grv, f, i: f.user_data.update({"x": i.operands[0]}))
    return g


def _frame(*instrs: Instr, depth: int = 0) -> RegisterFrame:
    return RegisterFrame(
        instructions=list(instrs),
        ip=0,
        acc=0,
        registers=[0] * 8,
        depth=depth,
    )


class TestRegisterFrameDefaults:
    def test_default_acc(self) -> None:
        f = RegisterFrame(instructions=[])
        assert f.acc is None

    def test_default_registers(self) -> None:
        f = RegisterFrame(instructions=[])
        assert len(f.registers) == 8
        assert all(r is None for r in f.registers)

    def test_default_user_data(self) -> None:
        f = RegisterFrame(instructions=[])
        assert f.user_data == {}

    def test_default_depth(self) -> None:
        f = RegisterFrame(instructions=[])
        assert f.depth == 0

    def test_caller_frame_default_none(self) -> None:
        f = RegisterFrame(instructions=[])
        assert f.caller_frame is None


class TestGenericRegisterVMBasics:
    def test_halt_returns_acc(self) -> None:
        g = _build_grvm()
        frame = _frame(Instr(OP_LOAD, [42]), Instr(OP_HALT, []))
        assert g.run(frame) == 42

    def test_load_and_add(self) -> None:
        g = _build_grvm()
        frame = _frame(
            Instr(OP_LOAD, [10]),
            Instr(OP_SAVE, [0]),   # R0 = 10
            Instr(OP_LOAD, [5]),
            Instr(OP_ADD, [0]),    # acc = 5 + 10 = 15
            Instr(OP_HALT, []),
        )
        assert g.run(frame) == 15

    def test_ret_signal_returns_value(self) -> None:
        g = _build_grvm()
        frame = _frame(Instr(OP_LOAD, [7]), Instr(OP_RET, []))
        assert g.run(frame) == 7

    def test_accumulator_starts_at_given_value(self) -> None:
        g = _build_grvm()
        frame = RegisterFrame(instructions=[Instr(OP_HALT, [])], acc=99, registers=[0]*8)
        assert g.run(frame) == 99

    def test_register_handler_overwrites(self) -> None:
        g = _build_grvm()
        # Override OP_LOAD to always return 0.
        g.register_handler(OP_LOAD, lambda grv, f, i: setattr(f, "acc", 0))
        frame = _frame(Instr(OP_LOAD, [42]), Instr(OP_HALT, []))
        assert g.run(frame) == 0

    def test_unregistered_opcode_raises_generic_vm_error(self) -> None:
        g = _build_grvm()
        frame = _frame(Instr(0xAA, []))
        with pytest.raises(GenericVMError, match="no handler registered"):
            g.run(frame)

    def test_ip_advances_sequentially(self) -> None:
        steps: list[int] = []
        g = _build_grvm()
        g.register_handler(OP_INC, lambda grv, f, i: (setattr(f, "acc", f.acc + 1), steps.append(f.ip - 1)))
        frame = _frame(
            Instr(OP_INC, []),  # ip=0
            Instr(OP_INC, []),  # ip=1
            Instr(OP_HALT, []),
        )
        g.run(frame)
        assert steps == [0, 1]

    def test_user_data_preserved(self) -> None:
        g = _build_grvm()
        frame = _frame(Instr(OP_SET_UD, [123]), Instr(OP_HALT, []))
        g.run(frame)
        assert frame.user_data["x"] == 123

    def test_jump_skips_instruction(self) -> None:
        g = _build_grvm()
        frame = _frame(
            Instr(OP_LOAD, [1]),
            Instr(OP_JMP, [1]),     # skip next
            Instr(OP_LOAD, [99]),   # skipped
            Instr(OP_HALT, []),
        )
        assert g.run(frame) == 1


class TestGenericRegisterVMTracing:
    def test_run_traced_returns_tuple(self) -> None:
        g = _build_grvm()
        frame = _frame(Instr(OP_LOAD, [5]), Instr(OP_HALT, []))
        result, traces = g.run_traced(frame)
        assert result == 5
        assert isinstance(traces, list)

    def test_trace_length(self) -> None:
        g = _build_grvm()
        frame = _frame(Instr(OP_LOAD, [1]), Instr(OP_INC, []), Instr(OP_HALT, []))
        _, traces = g.run_traced(frame)
        assert len(traces) == 3

    def test_trace_fields(self) -> None:
        g = _build_grvm()
        frame = _frame(Instr(OP_LOAD, [42]), Instr(OP_HALT, []))
        _, traces = g.run_traced(frame)
        first = traces[0]
        assert isinstance(first, GenericTrace)
        assert first.opcode == OP_LOAD
        assert first.operands == [42]
        assert first.ip == 0
        assert first.frame_depth == 0
        assert first.acc_before == 0   # frame started at acc=0
        assert first.acc_after == 42

    def test_trace_acc_before_after(self) -> None:
        g = _build_grvm()
        frame = _frame(Instr(OP_LOAD, [10]), Instr(OP_INC, []), Instr(OP_HALT, []))
        _, traces = g.run_traced(frame)
        inc_trace = traces[1]
        assert inc_trace.acc_before == 10
        assert inc_trace.acc_after == 11

    def test_trace_registers_snapshot(self) -> None:
        g = _build_grvm()
        frame = _frame(Instr(OP_LOAD, [7]), Instr(OP_SAVE, [0]), Instr(OP_HALT, []))
        _, traces = g.run_traced(frame)
        save_trace = traces[1]
        # Before SAVE: R0 was still 0.
        assert save_trace.registers_before[0] == 0
        # After SAVE: R0 is 7.
        assert save_trace.registers_after[0] == 7

    def test_trace_builder_called_on_halt(self) -> None:
        g = _build_grvm()
        frame = _frame(Instr(OP_HALT, []))
        _, traces = g.run_traced(frame)
        assert traces[-1].opcode == OP_HALT

    def test_trace_builder_called_on_ret(self) -> None:
        g = _build_grvm()
        frame = _frame(Instr(OP_LOAD, [5]), Instr(OP_RET, []))
        _, traces = g.run_traced(frame)
        assert traces[-1].opcode == OP_RET

    def test_trace_builder_restored_after_run(self) -> None:
        # Use a real callable so run_traced doesn't crash when chaining it.
        g = _build_grvm()
        calls: list[int] = []
        original = lambda f, i, ip, ab, rb: calls.append(i.opcode)  # noqa: E731
        g.trace_builder = original
        frame = _frame(Instr(OP_HALT, []))
        g.run_traced(frame)
        assert g.trace_builder is original

    def test_both_trace_builders_run(self) -> None:
        """If trace_builder is already set, run_traced chains it."""
        side_effects: list[int] = []
        g = _build_grvm()
        g.trace_builder = lambda f, i, ip, ab, rb: side_effects.append(i.opcode)
        frame = _frame(Instr(OP_LOAD, [1]), Instr(OP_HALT, []))
        _, generic_traces = g.run_traced(frame)
        # generic trace captured both
        assert len(generic_traces) == 2
        # original builder also called
        assert len(side_effects) == 2


class TestGenericRegisterVMNestedFrames:
    def test_recursive_run_frame_with_ret(self) -> None:
        """CALL handler pattern: recursively invoke _run_frame for callee."""
        g = _build_grvm()

        def _h_call(grv: GenericRegisterVM, frame: RegisterFrame, instr: Instr) -> None:
            callee = RegisterFrame(
                instructions=[Instr(OP_LOAD, [99]), Instr(OP_RET, [])],
                acc=0,
                registers=[0] * 8,
                depth=frame.depth + 1,
                caller_frame=frame,
            )
            result = grv._run_frame(callee)  # noqa: SLF001
            frame.acc = result

        OP_CALL = 0xC0
        g.register_handler(OP_CALL, _h_call)

        frame = _frame(Instr(OP_CALL, []), Instr(OP_HALT, []))
        assert g.run(frame) == 99

    def test_nested_frame_trace_appears_in_run_traced(self) -> None:
        """Traces from nested _run_frame calls appear in the run_traced output."""
        g = _build_grvm()

        def _h_call(grv: GenericRegisterVM, frame: RegisterFrame, instr: Instr) -> None:
            callee = RegisterFrame(
                instructions=[Instr(OP_LOAD, [55]), Instr(OP_RET, [])],
                acc=0,
                registers=[0] * 8,
                depth=frame.depth + 1,
                caller_frame=frame,
            )
            frame.acc = grv._run_frame(callee)  # noqa: SLF001

        OP_CALL = 0xC1
        g.register_handler(OP_CALL, _h_call)

        frame = _frame(Instr(OP_CALL, []), Instr(OP_HALT, []))
        _, traces = g.run_traced(frame)

        # Should have: CALL trace (depth 0), LOAD+RET from callee (depth 1), HALT (depth 0)
        depths = [t.frame_depth for t in traces]
        assert 1 in depths   # callee instructions were traced
        callee_traces = [t for t in traces if t.frame_depth == 1]
        assert len(callee_traces) == 2   # LOAD + RET


class TestGenericVMError:
    def test_error_message_contains_opcode(self) -> None:
        g = GenericRegisterVM()
        frame = _frame(Instr(0xBB, []))
        with pytest.raises(GenericVMError) as exc_info:
            g.run(frame)
        assert "0xBB" in str(exc_info.value)

    def test_error_contains_depth_and_ip(self) -> None:
        g = GenericRegisterVM()
        frame = RegisterFrame(
            instructions=[Instr(0xCC, [])],
            ip=0,
            acc=0,
            registers=[0] * 8,
            depth=2,
        )
        with pytest.raises(GenericVMError) as exc_info:
            g.run(frame)
        msg = str(exc_info.value)
        assert "depth=2" in msg
        assert "ip=0" in msg
