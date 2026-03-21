"""Starlark VM Opcode Handlers — The execution semantics for Starlark bytecode.

==========================================================================
Chapter 1: What Opcode Handlers Do
==========================================================================

Each handler is a function that implements one Starlark bytecode instruction.
The GenericVM's eval loop calls the handler whenever it encounters the
corresponding opcode. The handler mutates the VM state (stack, PC, variables)
and optionally returns output text.

All handlers follow the same signature::

    def handle_xxx(vm: GenericVM, instr: Instruction, code: CodeObject) -> str | None

- ``vm`` — The VM instance. Use ``vm.push()``, ``vm.pop()``, ``vm.advance_pc()``.
- ``instr`` — The instruction being executed (opcode + optional operand).
- ``code`` — The CodeObject being run (for constant/name pool access).

Returns a string if the handler produces output (e.g., PRINT), else None.

==========================================================================
Chapter 2: Starlark Type Semantics
==========================================================================

Starlark has a small, well-defined type system. Each handler must respect
the type rules:

- **int + int → int**, **float + float → float**, **int + float → float**
- **str + str → str** (concatenation), **str * int → str** (repetition)
- **list + list → list** (concatenation), **list * int → list** (repetition)
- Division always produces **float** (even ``4 / 2 → 2.0``)
- Floor division ``//`` produces **int** (for int operands)
- Truthiness: ``0``, ``0.0``, ``""``, ``[]``, ``{}``, ``()``, ``None``, ``False`` are falsy

==========================================================================
Chapter 3: The Iterator Protocol
==========================================================================

Starlark's for-loops use an iterator protocol (same as Python's):

1. ``GET_ITER`` — Convert an iterable to an iterator object.
2. ``FOR_ITER`` — Get the next value from the iterator, or jump to the
   end of the loop if the iterator is exhausted.

We implement iterators as Python iterators (using ``iter()`` and ``next()``).
The ``StarlarkIterator`` wrapper tracks the underlying Python iterator.
"""

from __future__ import annotations

from typing import Any

from virtual_machine import (
    CodeObject,
    GenericVM,
    Instruction,
    MaxRecursionError,
    VMTypeError,
)
from virtual_machine.vm import (
    CallFrame,
    DivisionByZeroError,
    InvalidOperandError,
    StackUnderflowError,
    UndefinedNameError,
    VMError,
)

from starlark_compiler.opcodes import Op


# =========================================================================
# Iterator wrapper
# =========================================================================


class StarlarkIterator:
    """Wraps a Python iterator for use in the Starlark VM.

    The VM's ``FOR_ITER`` handler calls ``next()`` on this object and
    catches ``StopIteration`` to know when the loop is done.
    """

    def __init__(self, iterable: Any) -> None:
        self._iterator = iter(iterable)

    def __next__(self) -> Any:
        return next(self._iterator)

    def __repr__(self) -> str:
        return "<starlark_iterator>"


# =========================================================================
# Starlark Function Object
# =========================================================================


class StarlarkFunction:
    """A user-defined Starlark function.

    Created by MAKE_FUNCTION, called by CALL_FUNCTION. Contains:
    - The function's compiled code (a CodeObject)
    - Default parameter values
    - The closure environment (captured variables)
    - Metadata (name, parameter count)
    """

    def __init__(
        self,
        code: CodeObject,
        defaults: list[Any] | None = None,
        name: str = "<lambda>",
        param_count: int = 0,
    ) -> None:
        self.code = code
        self.defaults = defaults or []
        self.name = name
        self.param_count = param_count

    def __repr__(self) -> str:
        return f"<function {self.name}>"


# =========================================================================
# Stack Handlers (0x01-0x06)
# =========================================================================


def handle_load_const(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """LOAD_CONST — Push a constant from the pool.

    This is the most common instruction. Every literal value (42, "hello",
    True) in the source code becomes a LOAD_CONST in the bytecode.
    """
    index = instr.operand
    if not isinstance(index, int) or index < 0 or index >= len(code.constants):
        raise InvalidOperandError(
            f"LOAD_CONST operand {index} out of range "
            f"(pool has {len(code.constants)} entries)"
        )
    vm.push(code.constants[index])
    vm.advance_pc()
    return None


def handle_pop(vm: GenericVM, instr: Instruction, code: CodeObject) -> str | None:
    """POP — Discard the top of stack."""
    vm.pop()
    vm.advance_pc()
    return None


def handle_dup(vm: GenericVM, instr: Instruction, code: CodeObject) -> str | None:
    """DUP — Duplicate the top of stack."""
    vm.push(vm.peek())
    vm.advance_pc()
    return None


def handle_load_none(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """LOAD_NONE — Push None."""
    vm.push(None)
    vm.advance_pc()
    return None


def handle_load_true(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """LOAD_TRUE — Push True."""
    vm.push(True)
    vm.advance_pc()
    return None


def handle_load_false(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """LOAD_FALSE — Push False."""
    vm.push(False)
    vm.advance_pc()
    return None


# =========================================================================
# Variable Handlers (0x10-0x15)
# =========================================================================


def handle_store_name(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """STORE_NAME — Pop and store in a named variable."""
    if vm.is_frozen:
        raise VMError("Cannot modify variables — module is frozen")
    index = instr.operand
    if not isinstance(index, int) or index < 0 or index >= len(code.names):
        raise InvalidOperandError(
            f"STORE_NAME operand {index} out of range "
            f"(names pool has {len(code.names)} entries)"
        )
    name = code.names[index]
    value = vm.pop()
    vm.variables[name] = value
    vm.advance_pc()
    return None


def handle_load_name(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """LOAD_NAME — Push the value of a named variable."""
    index = instr.operand
    if not isinstance(index, int) or index < 0 or index >= len(code.names):
        raise InvalidOperandError(
            f"LOAD_NAME operand {index} out of range "
            f"(names pool has {len(code.names)} entries)"
        )
    name = code.names[index]

    # Check variables first, then builtins
    if name in vm.variables:
        vm.push(vm.variables[name])
    elif vm.get_builtin(name) is not None:
        vm.push(vm.get_builtin(name))
    else:
        raise UndefinedNameError(f"Undefined variable: '{name}'")
    vm.advance_pc()
    return None


def handle_store_local(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """STORE_LOCAL — Pop and store in a local variable slot."""
    index = instr.operand
    assert isinstance(index, int)
    while len(vm.locals) <= index:
        vm.locals.append(None)
    vm.locals[index] = vm.pop()
    vm.advance_pc()
    return None


def handle_load_local(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """LOAD_LOCAL — Push a value from a local variable slot."""
    index = instr.operand
    assert isinstance(index, int)
    if index >= len(vm.locals):
        raise UndefinedNameError(f"Local variable slot {index} not yet assigned")
    vm.push(vm.locals[index])
    vm.advance_pc()
    return None


def handle_store_closure(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """STORE_CLOSURE — Store in a closure cell (placeholder)."""
    # Closures are tracked via shared cell objects
    # For now, use the same mechanism as locals
    handle_store_local(vm, instr, code)
    return None


def handle_load_closure(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """LOAD_CLOSURE — Load from a closure cell (placeholder)."""
    handle_load_local(vm, instr, code)
    return None


# =========================================================================
# Arithmetic Handlers (0x20-0x2D)
# =========================================================================


def _is_numeric(value: Any) -> bool:
    """Check if a value is numeric (int, float, or bool)."""
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def handle_add(vm: GenericVM, instr: Instruction, code: CodeObject) -> str | None:
    """ADD — Pop two values, push a + b.

    Supports:
    - int + int → int
    - float + float → float
    - int + float → float
    - str + str → str (concatenation)
    - list + list → list (concatenation)
    - tuple + tuple → tuple (concatenation)
    """
    b = vm.pop()
    a = vm.pop()

    if isinstance(a, str) and isinstance(b, str):
        vm.push(a + b)
    elif isinstance(a, list) and isinstance(b, list):
        vm.push(a + b)
    elif isinstance(a, tuple) and isinstance(b, tuple):
        vm.push(a + b)
    elif _is_numeric(a) and _is_numeric(b):
        vm.push(a + b)
    else:
        raise VMTypeError(
            f"Cannot add {type(a).__name__} and {type(b).__name__}"
        )
    vm.advance_pc()
    return None


def handle_sub(vm: GenericVM, instr: Instruction, code: CodeObject) -> str | None:
    """SUB — Pop two values, push a - b."""
    b = vm.pop()
    a = vm.pop()
    if not (_is_numeric(a) and _is_numeric(b)):
        raise VMTypeError(
            f"Cannot subtract {type(b).__name__} from {type(a).__name__}"
        )
    vm.push(a - b)
    vm.advance_pc()
    return None


def handle_mul(vm: GenericVM, instr: Instruction, code: CodeObject) -> str | None:
    """MUL — Pop two values, push a * b.

    Supports str * int, list * int, and numeric multiplication.
    """
    b = vm.pop()
    a = vm.pop()

    if isinstance(a, str) and isinstance(b, int):
        vm.push(a * b)
    elif isinstance(a, int) and isinstance(b, str):
        vm.push(a * b)
    elif isinstance(a, list) and isinstance(b, int):
        vm.push(a * b)
    elif isinstance(a, int) and isinstance(b, list):
        vm.push(a * b)
    elif _is_numeric(a) and _is_numeric(b):
        vm.push(a * b)
    else:
        raise VMTypeError(
            f"Cannot multiply {type(a).__name__} and {type(b).__name__}"
        )
    vm.advance_pc()
    return None


def handle_div(vm: GenericVM, instr: Instruction, code: CodeObject) -> str | None:
    """DIV — Pop two values, push a / b (always float division)."""
    b = vm.pop()
    a = vm.pop()
    if not (_is_numeric(a) and _is_numeric(b)):
        raise VMTypeError(
            f"Cannot divide {type(a).__name__} by {type(b).__name__}"
        )
    if b == 0:
        raise DivisionByZeroError("Division by zero")
    vm.push(a / b)  # Always float division
    vm.advance_pc()
    return None


def handle_floor_div(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """FLOOR_DIV — Pop two values, push a // b."""
    b = vm.pop()
    a = vm.pop()
    if not (_is_numeric(a) and _is_numeric(b)):
        raise VMTypeError(
            f"Cannot floor-divide {type(a).__name__} by {type(b).__name__}"
        )
    if b == 0:
        raise DivisionByZeroError("Floor division by zero")
    vm.push(a // b)
    vm.advance_pc()
    return None


def handle_mod(vm: GenericVM, instr: Instruction, code: CodeObject) -> str | None:
    """MOD — Pop two values, push a % b.

    Supports numeric modulo and string formatting (str % args).
    """
    b = vm.pop()
    a = vm.pop()
    if isinstance(a, str):
        # String formatting: "Hello, %s" % name
        if isinstance(b, tuple):
            vm.push(a % b)
        else:
            vm.push(a % (b,))
    elif _is_numeric(a) and _is_numeric(b):
        if b == 0:
            raise DivisionByZeroError("Modulo by zero")
        vm.push(a % b)
    else:
        raise VMTypeError(
            f"Cannot compute {type(a).__name__} % {type(b).__name__}"
        )
    vm.advance_pc()
    return None


def handle_power(vm: GenericVM, instr: Instruction, code: CodeObject) -> str | None:
    """POWER — Pop two values, push a ** b."""
    b = vm.pop()
    a = vm.pop()
    if not (_is_numeric(a) and _is_numeric(b)):
        raise VMTypeError(
            f"Cannot compute {type(a).__name__} ** {type(b).__name__}"
        )
    vm.push(a ** b)
    vm.advance_pc()
    return None


def handle_negate(vm: GenericVM, instr: Instruction, code: CodeObject) -> str | None:
    """NEGATE — Pop one value, push -a."""
    a = vm.pop()
    if not _is_numeric(a):
        raise VMTypeError(f"Cannot negate {type(a).__name__}")
    vm.push(-a)
    vm.advance_pc()
    return None


def handle_bit_and(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """BIT_AND — Pop two values, push a & b."""
    b = vm.pop()
    a = vm.pop()
    if not (isinstance(a, int) and isinstance(b, int)):
        raise VMTypeError(
            f"Cannot bitwise AND {type(a).__name__} and {type(b).__name__}"
        )
    vm.push(a & b)
    vm.advance_pc()
    return None


def handle_bit_or(vm: GenericVM, instr: Instruction, code: CodeObject) -> str | None:
    """BIT_OR — Pop two values, push a | b."""
    b = vm.pop()
    a = vm.pop()
    if not (isinstance(a, int) and isinstance(b, int)):
        raise VMTypeError(
            f"Cannot bitwise OR {type(a).__name__} and {type(b).__name__}"
        )
    vm.push(a | b)
    vm.advance_pc()
    return None


def handle_bit_xor(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """BIT_XOR — Pop two values, push a ^ b."""
    b = vm.pop()
    a = vm.pop()
    if not (isinstance(a, int) and isinstance(b, int)):
        raise VMTypeError(
            f"Cannot bitwise XOR {type(a).__name__} and {type(b).__name__}"
        )
    vm.push(a ^ b)
    vm.advance_pc()
    return None


def handle_bit_not(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """BIT_NOT — Pop one value, push ~a."""
    a = vm.pop()
    if not isinstance(a, int):
        raise VMTypeError(f"Cannot bitwise NOT {type(a).__name__}")
    vm.push(~a)
    vm.advance_pc()
    return None


def handle_lshift(vm: GenericVM, instr: Instruction, code: CodeObject) -> str | None:
    """LSHIFT — Pop two values, push a << b."""
    b = vm.pop()
    a = vm.pop()
    if not (isinstance(a, int) and isinstance(b, int)):
        raise VMTypeError(
            f"Cannot left-shift {type(a).__name__} by {type(b).__name__}"
        )
    vm.push(a << b)
    vm.advance_pc()
    return None


def handle_rshift(vm: GenericVM, instr: Instruction, code: CodeObject) -> str | None:
    """RSHIFT — Pop two values, push a >> b."""
    b = vm.pop()
    a = vm.pop()
    if not (isinstance(a, int) and isinstance(b, int)):
        raise VMTypeError(
            f"Cannot right-shift {type(a).__name__} by {type(b).__name__}"
        )
    vm.push(a >> b)
    vm.advance_pc()
    return None


# =========================================================================
# Comparison Handlers (0x30-0x37)
# =========================================================================


def handle_cmp_eq(vm: GenericVM, instr: Instruction, code: CodeObject) -> str | None:
    """CMP_EQ — Pop two values, push a == b."""
    b = vm.pop()
    a = vm.pop()
    vm.push(a == b)
    vm.advance_pc()
    return None


def handle_cmp_ne(vm: GenericVM, instr: Instruction, code: CodeObject) -> str | None:
    """CMP_NE — Pop two values, push a != b."""
    b = vm.pop()
    a = vm.pop()
    vm.push(a != b)
    vm.advance_pc()
    return None


def handle_cmp_lt(vm: GenericVM, instr: Instruction, code: CodeObject) -> str | None:
    """CMP_LT — Pop two values, push a < b."""
    b = vm.pop()
    a = vm.pop()
    vm.push(a < b)
    vm.advance_pc()
    return None


def handle_cmp_gt(vm: GenericVM, instr: Instruction, code: CodeObject) -> str | None:
    """CMP_GT — Pop two values, push a > b."""
    b = vm.pop()
    a = vm.pop()
    vm.push(a > b)
    vm.advance_pc()
    return None


def handle_cmp_le(vm: GenericVM, instr: Instruction, code: CodeObject) -> str | None:
    """CMP_LE — Pop two values, push a <= b."""
    b = vm.pop()
    a = vm.pop()
    vm.push(a <= b)
    vm.advance_pc()
    return None


def handle_cmp_ge(vm: GenericVM, instr: Instruction, code: CodeObject) -> str | None:
    """CMP_GE — Pop two values, push a >= b."""
    b = vm.pop()
    a = vm.pop()
    vm.push(a >= b)
    vm.advance_pc()
    return None


def handle_cmp_in(vm: GenericVM, instr: Instruction, code: CodeObject) -> str | None:
    """CMP_IN — Pop two values, push a in b."""
    b = vm.pop()
    a = vm.pop()
    vm.push(a in b)
    vm.advance_pc()
    return None


def handle_cmp_not_in(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """CMP_NOT_IN — Pop two values, push a not in b."""
    b = vm.pop()
    a = vm.pop()
    vm.push(a not in b)
    vm.advance_pc()
    return None


# =========================================================================
# Boolean Handler (0x38)
# =========================================================================


def _is_truthy(value: Any) -> bool:
    """Determine if a value is truthy in Starlark.

    Starlark truthiness follows Python's rules:
    - None, False, 0, 0.0, "", [], {}, () are falsy
    - Everything else is truthy
    """
    if value is None:
        return False
    if isinstance(value, bool):
        return value
    if isinstance(value, (int, float)):
        return value != 0
    if isinstance(value, str):
        return len(value) > 0
    if isinstance(value, (list, dict, tuple)):
        return len(value) > 0
    return True


def handle_not(vm: GenericVM, instr: Instruction, code: CodeObject) -> str | None:
    """NOT — Pop one value, push logical not."""
    a = vm.pop()
    vm.push(not _is_truthy(a))
    vm.advance_pc()
    return None


# =========================================================================
# Control Flow Handlers (0x40-0x44)
# =========================================================================


def handle_jump(vm: GenericVM, instr: Instruction, code: CodeObject) -> str | None:
    """JUMP — Unconditional jump to target."""
    target = instr.operand
    assert isinstance(target, int)
    vm.jump_to(target)
    return None


def handle_jump_if_false(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """JUMP_IF_FALSE — Pop value, jump if falsy."""
    target = instr.operand
    assert isinstance(target, int)
    value = vm.pop()
    if not _is_truthy(value):
        vm.jump_to(target)
    else:
        vm.advance_pc()
    return None


def handle_jump_if_true(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """JUMP_IF_TRUE — Pop value, jump if truthy."""
    target = instr.operand
    assert isinstance(target, int)
    value = vm.pop()
    if _is_truthy(value):
        vm.jump_to(target)
    else:
        vm.advance_pc()
    return None


def handle_jump_if_false_or_pop(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """JUMP_IF_FALSE_OR_POP — Short-circuit AND.

    If top is falsy → keep it on stack and jump (short-circuit).
    If top is truthy → pop it and fall through (evaluate next operand).
    """
    target = instr.operand
    assert isinstance(target, int)
    value = vm.peek()
    if not _is_truthy(value):
        vm.jump_to(target)  # Keep falsy value on stack
    else:
        vm.pop()  # Discard truthy value, evaluate next operand
        vm.advance_pc()
    return None


def handle_jump_if_true_or_pop(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """JUMP_IF_TRUE_OR_POP — Short-circuit OR.

    If top is truthy → keep it on stack and jump (short-circuit).
    If top is falsy → pop it and fall through (evaluate next operand).
    """
    target = instr.operand
    assert isinstance(target, int)
    value = vm.peek()
    if _is_truthy(value):
        vm.jump_to(target)  # Keep truthy value on stack
    else:
        vm.pop()  # Discard falsy value, evaluate next operand
        vm.advance_pc()
    return None


# =========================================================================
# Function Handlers (0x50-0x53)
# =========================================================================


def handle_make_function(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """MAKE_FUNCTION — Create a function object from a CodeObject.

    The CodeObject is on top of the stack (pushed by LOAD_CONST).
    Default values (if any) are below it.
    """
    flags = instr.operand or 0
    func_code = vm.pop()

    defaults = []
    if flags & 0x01:  # Has defaults
        # Defaults are on the stack below the code object
        # In practice, they were pushed before LOAD_CONST code
        # For simplicity, we'll handle defaults via the function call
        pass

    func = StarlarkFunction(
        code=func_code,
        defaults=defaults,
        param_count=len(func_code.names) if hasattr(func_code, 'names') else 0,
    )
    vm.push(func)
    vm.advance_pc()
    return None


def handle_call_function(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """CALL_FUNCTION — Call a function with N positional arguments.

    Stack layout before call: [func, arg1, arg2, ..., argN]
    Operand: N (number of arguments)
    """
    argc = instr.operand or 0
    assert isinstance(argc, int)

    # Pop arguments (in reverse order)
    args = []
    for _ in range(argc):
        args.insert(0, vm.pop())

    # Pop the callable
    func = vm.pop()

    if isinstance(func, StarlarkFunction):
        # Check recursion limit
        if vm.max_recursion_depth is not None:
            if len(vm.call_stack) >= vm.max_recursion_depth:
                raise MaxRecursionError(
                    f"Maximum recursion depth exceeded "
                    f"(limit: {vm.max_recursion_depth})"
                )

        # Save current state
        frame = CallFrame(
            return_address=vm.pc + 1,
            saved_variables=dict(vm.variables),
            saved_locals=list(vm.locals),
        )
        vm.push_frame(frame)

        # Set up locals for the function
        vm.locals = list(args)
        # Pad with None if not enough args
        while len(vm.locals) < len(func.code.names):
            vm.locals.append(None)

        # Jump to function code
        # We execute the function's CodeObject inline by replacing context
        # For now, we use a simplified approach: execute the function's code
        # as a sub-execution
        _execute_function(vm, func, args)

    elif hasattr(func, 'implementation'):
        # Built-in function
        result = func.implementation(args)
        vm.push(result)
        vm.advance_pc()
    else:
        raise VMTypeError(f"'{type(func).__name__}' object is not callable")

    return None


def _execute_function(
    vm: GenericVM, func: StarlarkFunction, args: list[Any]
) -> None:
    """Execute a StarlarkFunction by running its CodeObject.

    This creates a mini-execution context: save the current state, run
    the function's bytecode, then restore the caller's state.
    """
    # Save current execution state
    saved_pc = vm.pc
    saved_halted = vm.halted
    saved_vars = dict(vm.variables)
    saved_locals = list(vm.locals)

    # Set up function context
    vm.locals = list(args)
    vm.pc = 0
    vm.halted = False

    # Execute the function's code
    func_code = func.code
    while not vm.halted and vm.pc < len(func_code.instructions):
        instruction = func_code.instructions[vm.pc]
        handler = vm._handlers.get(instruction.opcode)
        if handler is None:
            from virtual_machine import InvalidOpcodeError
            raise InvalidOpcodeError(
                f"Unknown opcode in function: {instruction.opcode:#04x}"
            )

        # Check for RETURN
        if instruction.opcode == Op.RETURN:
            return_value = vm.pop() if vm.stack else None
            break
        elif instruction.opcode == Op.HALT:
            return_value = None
            break
        else:
            handler(vm, instruction, func_code)
    else:
        return_value = None

    # Restore caller's state
    vm.pc = saved_pc
    vm.halted = saved_halted
    vm.variables = saved_vars
    vm.locals = saved_locals

    # Pop the call frame we pushed
    if vm.call_stack:
        vm.pop_frame()

    # Push return value
    vm.push(return_value)
    vm.advance_pc()


def handle_call_function_kw(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """CALL_FUNCTION_KW — Call function with keyword arguments.

    For now, treat like CALL_FUNCTION (keyword handling is complex).
    """
    return handle_call_function(vm, instr, code)


def handle_return(vm: GenericVM, instr: Instruction, code: CodeObject) -> str | None:
    """RETURN — Return from a function.

    The return value is on top of the stack.
    """
    # Return is handled specially by _execute_function
    # If we reach here, it means RETURN at the top level
    vm.halted = True
    return None


# =========================================================================
# Collection Handlers (0x60-0x64)
# =========================================================================


def handle_build_list(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """BUILD_LIST — Create a list from N stack items."""
    count = instr.operand or 0
    assert isinstance(count, int)
    items = []
    for _ in range(count):
        items.insert(0, vm.pop())
    vm.push(items)
    vm.advance_pc()
    return None


def handle_build_dict(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """BUILD_DICT — Create a dict from N key-value pairs on the stack."""
    count = instr.operand or 0
    assert isinstance(count, int)
    pairs = []
    for _ in range(count):
        value = vm.pop()
        key = vm.pop()
        pairs.insert(0, (key, value))
    vm.push(dict(pairs))
    vm.advance_pc()
    return None


def handle_build_tuple(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """BUILD_TUPLE — Create a tuple from N stack items."""
    count = instr.operand or 0
    assert isinstance(count, int)
    items = []
    for _ in range(count):
        items.insert(0, vm.pop())
    vm.push(tuple(items))
    vm.advance_pc()
    return None


def handle_list_append(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """LIST_APPEND — Append value to list (for comprehensions).

    Stack: ... list value → ... list
    """
    value = vm.pop()
    lst = vm.peek()  # Don't pop the list — it stays for next append
    if not isinstance(lst, list):
        raise VMTypeError(f"LIST_APPEND requires a list, got {type(lst).__name__}")
    lst.append(value)
    vm.advance_pc()
    return None


def handle_dict_set(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """DICT_SET — Set dict entry (for comprehensions).

    Stack: ... dict key value → ... dict
    """
    value = vm.pop()
    key = vm.pop()
    d = vm.peek()  # Don't pop the dict
    if not isinstance(d, dict):
        raise VMTypeError(f"DICT_SET requires a dict, got {type(d).__name__}")
    d[key] = value
    vm.advance_pc()
    return None


# =========================================================================
# Subscript & Attribute Handlers (0x70-0x74)
# =========================================================================


def handle_load_subscript(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """LOAD_SUBSCRIPT — obj[key]."""
    key = vm.pop()
    obj = vm.pop()
    try:
        vm.push(obj[key])
    except (KeyError, IndexError, TypeError) as e:
        raise VMError(f"Subscript error: {e}") from e
    vm.advance_pc()
    return None


def handle_store_subscript(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """STORE_SUBSCRIPT — obj[key] = value."""
    if vm.is_frozen:
        raise VMError("Cannot modify collections — module is frozen")
    value = vm.pop()
    key = vm.pop()
    obj = vm.pop()
    try:
        obj[key] = value
    except (TypeError, IndexError) as e:
        raise VMError(f"Subscript store error: {e}") from e
    vm.advance_pc()
    return None


def handle_load_attr(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """LOAD_ATTR — obj.attr. Attribute access on Starlark values.

    In Starlark, only a few types have attributes:
    - string methods (upper, lower, split, etc.)
    - dict methods (keys, values, items, etc.)
    - list methods (append, extend, etc.)
    """
    index = instr.operand
    assert isinstance(index, int)
    attr_name = code.names[index]
    obj = vm.pop()

    try:
        result = getattr(obj, attr_name)
        vm.push(result)
    except AttributeError:
        raise VMError(
            f"'{type(obj).__name__}' has no attribute '{attr_name}'"
        ) from None
    vm.advance_pc()
    return None


def handle_store_attr(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """STORE_ATTR — obj.attr = value."""
    raise VMError("Starlark does not support attribute assignment")


def handle_load_slice(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """LOAD_SLICE — obj[start:stop:step].

    Operand flags indicate which parts are present:
    - bit 0: start present
    - bit 1: stop present
    - bit 2: step present
    """
    flags = instr.operand or 0
    assert isinstance(flags, int)

    step = vm.pop() if flags & 0x04 else None
    stop = vm.pop() if flags & 0x02 else None
    start = vm.pop() if flags & 0x01 else None

    # Replace None sentinel values
    if start is None:
        start = None  # slice(None) means from beginning
    if stop is None:
        stop = None
    if step is None:
        step = None

    obj = vm.pop()
    try:
        vm.push(obj[slice(start, stop, step)])
    except (TypeError, IndexError) as e:
        raise VMError(f"Slice error: {e}") from e
    vm.advance_pc()
    return None


# =========================================================================
# Iteration Handlers (0x80-0x82)
# =========================================================================


def handle_get_iter(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """GET_ITER — Convert an iterable to an iterator."""
    iterable = vm.pop()
    vm.push(StarlarkIterator(iterable))
    vm.advance_pc()
    return None


def handle_for_iter(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """FOR_ITER — Get next value from iterator, or jump if exhausted.

    If the iterator has more values: push the next value and advance PC.
    If exhausted: pop the iterator and jump to the target.
    """
    target = instr.operand
    assert isinstance(target, int)

    iterator = vm.peek()
    try:
        value = next(iterator)
        vm.push(value)
        vm.advance_pc()
    except StopIteration:
        vm.pop()  # Pop the exhausted iterator
        vm.jump_to(target)
    return None


def handle_unpack_sequence(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """UNPACK_SEQUENCE — Unpack N items from a sequence.

    Pops a sequence, pushes its elements in reverse order so they can
    be stored in the correct order by subsequent STORE instructions.
    """
    count = instr.operand
    assert isinstance(count, int)

    seq = vm.pop()
    items = list(seq)
    if len(items) != count:
        raise VMError(
            f"Cannot unpack {len(items)} values into {count} variables"
        )

    # Push in reverse so STORE operations work left-to-right
    for item in reversed(items):
        vm.push(item)
    vm.advance_pc()
    return None


# =========================================================================
# Module Handlers (0x90-0x91)
# =========================================================================


def handle_load_module(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """LOAD_MODULE — Load a Starlark module (for load() statement).

    In a full implementation, this would find and execute the module file.
    For now, we push a placeholder that errors on access.
    """
    index = instr.operand
    assert isinstance(index, int)
    module_name = code.names[index]
    # Push a placeholder module dict
    vm.push({"__name__": module_name})
    vm.advance_pc()
    return None


def handle_import_from(
    vm: GenericVM, instr: Instruction, code: CodeObject
) -> str | None:
    """IMPORT_FROM — Extract a symbol from a loaded module."""
    index = instr.operand
    assert isinstance(index, int)
    symbol_name = code.names[index]
    module = vm.peek()  # Don't pop — might need for more imports
    if isinstance(module, dict) and symbol_name in module:
        vm.push(module[symbol_name])
    else:
        raise VMError(
            f"Cannot import '{symbol_name}' from module"
        )
    vm.advance_pc()
    return None


# =========================================================================
# I/O Handler (0xA0)
# =========================================================================


def handle_print(vm: GenericVM, instr: Instruction, code: CodeObject) -> str | None:
    """PRINT — Pop and print a value."""
    value = vm.pop()
    output = _starlark_repr(value)
    vm.output.append(output)
    vm.advance_pc()
    return output


def _starlark_repr(value: Any) -> str:
    """Format a value for Starlark print output.

    Starlark's print representation follows Python conventions:
    - Strings are printed without quotes
    - None → "None"
    - True/False → "True"/"False"
    - Lists: [1, 2, 3]
    - Dicts: {"a": 1}
    """
    if value is None:
        return "None"
    if isinstance(value, bool):
        return "True" if value else "False"
    if isinstance(value, str):
        return value  # print() shows strings without quotes
    return repr(value)


# =========================================================================
# Halt Handler (0xFF)
# =========================================================================


def handle_halt(vm: GenericVM, instr: Instruction, code: CodeObject) -> str | None:
    """HALT — Stop execution."""
    vm.halted = True
    return None
