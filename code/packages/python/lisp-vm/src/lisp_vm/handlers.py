"""Lisp VM Opcode Handlers — The execution semantics for Lisp bytecode.

==========================================================================
Chapter 1: What Opcode Handlers Do
==========================================================================

Each handler is a function that implements one Lisp bytecode instruction.
The GenericVM's eval loop calls the handler whenever it encounters the
corresponding opcode. The handler mutates the VM state (stack, PC, variables)
and optionally returns output text.

All handlers follow the same signature::

    def handle_xxx(vm, instr, code) -> str | None

==========================================================================
Chapter 2: GC Integration
==========================================================================

Unlike Starlark, Lisp needs a garbage collector. Cons cells, closures,
and symbols all live on the GC heap. Handlers that create heap objects
(CONS, MAKE_CLOSURE, MAKE_SYMBOL) use the GC's ``allocate()`` method.
Handlers that read heap objects (CAR, CDR, IS_ATOM) use ``deref()``.

The GC and SymbolTable are injected via closures in the factory function.

==========================================================================
Chapter 3: NIL and Truthiness
==========================================================================

NIL is Lisp's "nothing" — the empty list, the end marker, the false
value. It's a distinct Python object (not None, not 0, not False).

Falsy values in Lisp: NIL, 0, False.
Everything else is truthy.

==========================================================================
Chapter 4: Tail Call Optimization
==========================================================================

The TAIL_CALL opcode reuses the current call frame instead of pushing
a new one. This means tail-recursive functions run in O(1) stack space.

How it works:
1. Pop N args and the callable (same as CALL_FUNCTION)
2. Instead of pushing a new frame, rebind args in the current locals
3. Reset PC to 0 to restart the function
4. Continue executing — no new frame, no stack growth

This is transparent to the caller — the result eventually comes back
via RETURN as if a normal call had been made.
"""

from __future__ import annotations

from typing import Any

from garbage_collector import ConsCell, GarbageCollector, LispClosure, SymbolTable
from virtual_machine import CodeObject, GenericVM, Instruction
from virtual_machine.vm import CallFrame

from lisp_vm.opcodes import LispOp

# =========================================================================
# The NIL sentinel
# =========================================================================
#
# NIL is a unique Python object — not None, not 0, not False. It
# represents Lisp's empty list / false / nothing value.
#
# Why not use Python's None? Because None already has semantics in
# our VM (it means "no value" for missing locals, uninitialized
# variables, etc.). NIL needs to be testable via identity: ``value is NIL``.
# =========================================================================


class _NilType:
    """The type of the NIL sentinel.

    There is exactly one instance of this class: ``NIL``. It is used as
    Lisp's empty list / false / nothing value.
    """

    def __repr__(self) -> str:
        return "NIL"

    def __bool__(self) -> bool:
        return False


NIL = _NilType()
"""The NIL sentinel — Lisp's empty list / false / nothing value."""


def _is_falsy(value: Any) -> bool:
    """Check if a value is falsy in Lisp.

    Falsy values: NIL, 0, False.
    Everything else (including empty strings and empty lists) is truthy.
    """
    if value is NIL:
        return True
    if value is False:
        return True
    if value == 0 and not isinstance(value, bool):
        return True
    return False


# =========================================================================
# LispFunction — wraps a closure for the call protocol
# =========================================================================


class LispFunction:
    """A callable Lisp function (wraps a LispClosure heap address).

    Created by MAKE_CLOSURE, called by CALL_FUNCTION and TAIL_CALL.
    Stores the heap address of the LispClosure and a reference to the GC
    for dereferencing.
    """

    def __init__(self, closure_addr: int, gc: GarbageCollector) -> None:
        self.closure_addr = closure_addr
        self._gc = gc

    @property
    def closure(self) -> LispClosure:
        """Dereference the closure from the heap."""
        obj = self._gc.deref(self.closure_addr)
        assert isinstance(obj, LispClosure)
        return obj

    def __repr__(self) -> str:
        return f"<lisp-function @{self.closure_addr}>"


# =========================================================================
# Stack Handlers (0x01-0x04)
# =========================================================================


def handle_load_const(
    vm: GenericVM, instr: Instruction, code: CodeObject,
) -> str | None:
    """LOAD_CONST — Push a constant from the pool."""
    index = instr.operand
    assert isinstance(index, int)
    vm.push(code.constants[index])
    vm.advance_pc()
    return None


def handle_pop(
    vm: GenericVM, instr: Instruction, code: CodeObject,
) -> str | None:
    """POP — Discard top of stack."""
    vm.pop()
    vm.advance_pc()
    return None


def handle_load_nil(
    vm: GenericVM, instr: Instruction, code: CodeObject,
) -> str | None:
    """LOAD_NIL — Push the NIL sentinel."""
    vm.push(NIL)
    vm.advance_pc()
    return None


def handle_load_true(
    vm: GenericVM, instr: Instruction, code: CodeObject,
) -> str | None:
    """LOAD_TRUE — Push True (Lisp's 't')."""
    vm.push(True)
    vm.advance_pc()
    return None


# =========================================================================
# Variable Handlers (0x10-0x13)
# =========================================================================


def handle_store_name(
    vm: GenericVM, instr: Instruction, code: CodeObject,
) -> str | None:
    """STORE_NAME — Pop and store in a named variable."""
    index = instr.operand
    assert isinstance(index, int)
    name = code.names[index]
    vm.variables[name] = vm.pop()
    vm.advance_pc()
    return None


def handle_load_name(
    vm: GenericVM, instr: Instruction, code: CodeObject,
) -> str | None:
    """LOAD_NAME — Push a named variable's value."""
    index = instr.operand
    assert isinstance(index, int)
    name = code.names[index]
    if name not in vm.variables:
        raise NameError(f"Undefined variable: {name}")
    vm.push(vm.variables[name])
    vm.advance_pc()
    return None


def handle_store_local(
    vm: GenericVM, instr: Instruction, code: CodeObject,
) -> str | None:
    """STORE_LOCAL — Pop and store in a local slot."""
    index = instr.operand
    assert isinstance(index, int)
    while len(vm.locals) <= index:
        vm.locals.append(None)
    vm.locals[index] = vm.pop()
    vm.advance_pc()
    return None


def handle_load_local(
    vm: GenericVM, instr: Instruction, code: CodeObject,
) -> str | None:
    """LOAD_LOCAL — Push a local slot's value."""
    index = instr.operand
    assert isinstance(index, int)
    vm.push(vm.locals[index])
    vm.advance_pc()
    return None


# =========================================================================
# Arithmetic Handlers (0x20-0x23)
# =========================================================================


def handle_add(
    vm: GenericVM, instr: Instruction, code: CodeObject,
) -> str | None:
    """ADD — Add two numbers. a b → (a + b)"""
    b = vm.pop()
    a = vm.pop()
    vm.push(a + b)
    vm.advance_pc()
    return None


def handle_sub(
    vm: GenericVM, instr: Instruction, code: CodeObject,
) -> str | None:
    """SUB — Subtract. a b → (a - b)"""
    b = vm.pop()
    a = vm.pop()
    vm.push(a - b)
    vm.advance_pc()
    return None


def handle_mul(
    vm: GenericVM, instr: Instruction, code: CodeObject,
) -> str | None:
    """MUL — Multiply. a b → (a * b)"""
    b = vm.pop()
    a = vm.pop()
    vm.push(a * b)
    vm.advance_pc()
    return None


def handle_div(
    vm: GenericVM, instr: Instruction, code: CodeObject,
) -> str | None:
    """DIV — Integer divide. a b → (a // b)"""
    b = vm.pop()
    a = vm.pop()
    if b == 0:
        raise ZeroDivisionError("Division by zero")
    vm.push(a // b)
    vm.advance_pc()
    return None


# =========================================================================
# Comparison Handlers (0x30-0x32)
# =========================================================================


def handle_cmp_eq(
    vm: GenericVM, instr: Instruction, code: CodeObject,
) -> str | None:
    """CMP_EQ — Equality check. a b → (1 if equal, 0 if not)

    For Lisp's ``eq``: uses Python's ``==`` for numbers, ``is`` for NIL,
    and address equality for heap objects.
    """
    b = vm.pop()
    a = vm.pop()
    # NIL identity check
    if a is NIL and b is NIL:
        vm.push(1)
    elif a is NIL or b is NIL:
        vm.push(0)
    else:
        vm.push(1 if a == b else 0)
    vm.advance_pc()
    return None


def handle_cmp_lt(
    vm: GenericVM, instr: Instruction, code: CodeObject,
) -> str | None:
    """CMP_LT — Less than. a b → (1 if a < b, else 0)"""
    b = vm.pop()
    a = vm.pop()
    vm.push(1 if a < b else 0)
    vm.advance_pc()
    return None


def handle_cmp_gt(
    vm: GenericVM, instr: Instruction, code: CodeObject,
) -> str | None:
    """CMP_GT — Greater than. a b → (1 if a > b, else 0)"""
    b = vm.pop()
    a = vm.pop()
    vm.push(1 if a > b else 0)
    vm.advance_pc()
    return None


# =========================================================================
# Control Flow Handlers (0x40-0x42)
# =========================================================================


def handle_jump(
    vm: GenericVM, instr: Instruction, code: CodeObject,
) -> str | None:
    """JUMP — Unconditional jump to target PC."""
    target = instr.operand
    assert isinstance(target, int)
    vm.pc = target
    return None


def handle_jump_if_false(
    vm: GenericVM, instr: Instruction, code: CodeObject,
) -> str | None:
    """JUMP_IF_FALSE — Jump if top is falsy, else advance."""
    target = instr.operand
    assert isinstance(target, int)
    value = vm.pop()
    if _is_falsy(value):
        vm.pc = target
    else:
        vm.advance_pc()
    return None


def handle_jump_if_true(
    vm: GenericVM, instr: Instruction, code: CodeObject,
) -> str | None:
    """JUMP_IF_TRUE — Jump if top is truthy, else advance."""
    target = instr.operand
    assert isinstance(target, int)
    value = vm.pop()
    if not _is_falsy(value):
        vm.pc = target
    else:
        vm.advance_pc()
    return None


# =========================================================================
# Function Handlers (0x50-0x53)
# =========================================================================


def make_closure_handler(
    gc: GarbageCollector,
) -> Any:
    """Create a MAKE_CLOSURE handler that captures the GC.

    Returns a handler function closed over the GC instance.
    """
    def handle_make_closure(
        vm: GenericVM, instr: Instruction, code: CodeObject,
    ) -> str | None:
        """MAKE_CLOSURE — Create a closure from a CodeObject.

        Pops a CodeObject from the stack, captures the current variable
        environment, allocates a LispClosure on the GC heap, and pushes
        a LispFunction wrapper.
        """
        func_code = vm.pop()
        assert isinstance(func_code, CodeObject)

        # Capture the current environment (global variables)
        captured_env = dict(vm.variables)

        # The operand tells us how many parameters the function takes
        param_count = instr.operand if instr.operand is not None else 0
        assert isinstance(param_count, int)

        # Extract real parameter names. The compiler stores a tuple of
        # param names as the last constant in the body CodeObject.
        # Fall back to synthetic names for hand-compiled bytecode.
        params = [f"_p{i}" for i in range(param_count)]
        if param_count > 0 and func_code.constants:
            last_const = func_code.constants[-1]
            if (
                isinstance(last_const, (list, tuple))
                and len(last_const) == param_count
                and all(isinstance(s, str) for s in last_const)
            ):
                params = list(last_const)

        # Allocate closure on heap
        closure = LispClosure(
            code=func_code,
            env=captured_env,
            params=params,
        )
        addr = gc.allocate(closure)

        # Push a LispFunction wrapper
        vm.push(LispFunction(addr, gc))
        vm.advance_pc()
        return None

    return handle_make_closure


def call_function_handler(
    gc: GarbageCollector,
) -> Any:
    """Create a CALL_FUNCTION handler that captures the GC.

    Returns a handler function closed over the GC instance.
    """
    def handle_call_function(
        vm: GenericVM, instr: Instruction, code: CodeObject,
    ) -> str | None:
        """CALL_FUNCTION — Call a function with N arguments.

        Stack layout: [arg1, arg2, ..., argN, func]
        Operand: N (number of arguments)
        """
        argc = instr.operand or 0
        assert isinstance(argc, int)

        # Pop the callable first (it's on top after args are pushed)
        func = vm.pop()

        # Pop arguments (they were pushed left-to-right, so pop in reverse)
        args = []
        for _ in range(argc):
            args.insert(0, vm.pop())

        if isinstance(func, LispFunction):
            _execute_lisp_function(vm, func, args, gc)
        else:
            raise TypeError(f"Cannot call {type(func).__name__}")

        return None

    return handle_call_function


def tail_call_handler(
    gc: GarbageCollector,
) -> Any:
    """Create a TAIL_CALL handler that captures the GC.

    Returns a handler function closed over the GC instance.
    """
    def handle_tail_call(
        vm: GenericVM, instr: Instruction, code: CodeObject,
    ) -> str | None:
        """TAIL_CALL — Tail call optimization.

        Like CALL_FUNCTION but signals the execution loop to reuse the
        current frame. We do this by setting a special attribute on the VM.
        """
        argc = instr.operand or 0
        assert isinstance(argc, int)

        # Pop callable and args (same as CALL_FUNCTION)
        func = vm.pop()
        args = []
        for _ in range(argc):
            args.insert(0, vm.pop())

        if isinstance(func, LispFunction):
            # Signal tail call: store the function and args for the
            # execution loop to pick up
            vm._tail_call_func = func  # type: ignore[attr-defined]
            vm._tail_call_args = args  # type: ignore[attr-defined]
            vm._tail_call_pending = True  # type: ignore[attr-defined]
            # Don't advance PC — the execution loop handles the rest
        else:
            raise TypeError(f"Cannot tail-call {type(func).__name__}")

        return None

    return handle_tail_call


def handle_return(
    vm: GenericVM, instr: Instruction, code: CodeObject,
) -> str | None:
    """RETURN — Signal that we should return from the current function.

    The actual return mechanics are handled by _execute_lisp_function.
    This handler just sets a flag.
    """
    # The execution loop in _execute_lisp_function checks for RETURN
    # directly, so this handler just advances (it may not be called directly).
    vm.advance_pc()
    return None


def _execute_lisp_function(
    vm: GenericVM,
    func: LispFunction,
    args: list[Any],
    gc: GarbageCollector,
) -> None:
    """Execute a Lisp function by running its closure's CodeObject.

    This creates a mini-execution context: save the current state, run
    the function's bytecode, then restore the caller's state.

    Tail calls are handled by looping: if the function body executes a
    TAIL_CALL, we rebind args and restart instead of recursing.
    """
    closure = func.closure
    func_code = closure.code
    assert isinstance(func_code, CodeObject)

    # Save current execution state
    saved_pc = vm.pc
    saved_halted = vm.halted
    saved_vars = dict(vm.variables)
    saved_locals = list(vm.locals)

    # Restore captured environment
    vm.variables.update(closure.env)

    # Initialize tail call state
    vm._tail_call_pending = False  # type: ignore[attr-defined]
    current_func = func
    current_args = args
    current_code = func_code

    # TCO loop: if a tail call happens, we restart here with new args
    while True:
        # Set up function context
        vm.locals = list(current_args)
        vm.pc = 0
        vm.halted = False
        vm._tail_call_pending = False  # type: ignore[attr-defined]

        # Also bind parameters in vm.variables so that inner closures
        # can capture them. The closure's params list has the names.
        current_closure = current_func.closure
        for i, param_name in enumerate(current_closure.params):
            if i < len(current_args):
                vm.variables[param_name] = current_args[i]

        return_value = NIL  # default return value

        # Execute the function's bytecode
        while not vm.halted and vm.pc < len(current_code.instructions):
            instruction = current_code.instructions[vm.pc]

            # Check for RETURN — pop and break
            if instruction.opcode == LispOp.RETURN:
                return_value = vm.pop() if vm.stack else NIL
                break

            # Check for HALT
            if instruction.opcode == LispOp.HALT:
                break

            # Dispatch to handler
            handler = vm._handlers.get(instruction.opcode)
            if handler is None:
                raise RuntimeError(
                    f"Unknown opcode in function: {instruction.opcode:#04x}"
                )
            handler(vm, instruction, current_code)

            # Check if a tail call was requested
            if getattr(vm, '_tail_call_pending', False):
                break

        # If a tail call was requested, restart with new function/args
        if getattr(vm, '_tail_call_pending', False):
            vm._tail_call_pending = False  # type: ignore[attr-defined]
            new_func = vm._tail_call_func  # type: ignore[attr-defined]
            new_args = vm._tail_call_args  # type: ignore[attr-defined]

            # Update for next iteration
            current_func = new_func
            current_args = new_args
            new_closure = new_func.closure
            current_code = new_closure.code
            assert isinstance(current_code, CodeObject)

            # Restore the new closure's environment
            vm.variables = dict(saved_vars)
            vm.variables.update(new_closure.env)
            continue

        # Normal return — break the TCO loop
        break

    # Restore caller's state
    vm.pc = saved_pc
    vm.halted = saved_halted
    vm.variables = saved_vars
    vm.locals = saved_locals

    # Push return value
    vm.push(return_value)
    vm.advance_pc()


# =========================================================================
# Lisp-Specific Handlers (0x70-0x75)
# =========================================================================


def cons_handler(gc: GarbageCollector) -> Any:
    """Create a CONS handler that captures the GC."""
    def handle_cons(
        vm: GenericVM, instr: Instruction, code: CodeObject,
    ) -> str | None:
        """CONS — Create a cons cell. car (top), cdr (below) → address"""
        car = vm.pop()
        cdr = vm.pop()
        addr = gc.allocate(ConsCell(car=car, cdr=cdr))
        vm.push(addr)
        vm.advance_pc()
        return None

    return handle_cons


def car_handler(gc: GarbageCollector) -> Any:
    """Create a CAR handler that captures the GC."""
    def handle_car(
        vm: GenericVM, instr: Instruction, code: CodeObject,
    ) -> str | None:
        """CAR — Get the first element of a cons cell."""
        addr = vm.pop()
        assert isinstance(addr, int), f"CAR expects a heap address, got {type(addr)}"
        obj = gc.deref(addr)
        assert isinstance(obj, ConsCell), f"CAR expects a cons cell, got {type(obj)}"
        vm.push(obj.car)
        vm.advance_pc()
        return None

    return handle_car


def cdr_handler(gc: GarbageCollector) -> Any:
    """Create a CDR handler that captures the GC."""
    def handle_cdr(
        vm: GenericVM, instr: Instruction, code: CodeObject,
    ) -> str | None:
        """CDR — Get the second element of a cons cell."""
        addr = vm.pop()
        assert isinstance(addr, int), f"CDR expects a heap address, got {type(addr)}"
        obj = gc.deref(addr)
        assert isinstance(obj, ConsCell), f"CDR expects a cons cell, got {type(obj)}"
        vm.push(obj.cdr)
        vm.advance_pc()
        return None

    return handle_cdr


def make_symbol_handler(
    gc: GarbageCollector, symbol_table: SymbolTable,
) -> Any:
    """Create a MAKE_SYMBOL handler that captures the GC and symbol table."""
    def handle_make_symbol(
        vm: GenericVM, instr: Instruction, code: CodeObject,
    ) -> str | None:
        """MAKE_SYMBOL — Intern a symbol and push its address."""
        index = instr.operand
        assert isinstance(index, int)
        name = code.constants[index]
        assert isinstance(name, str)
        addr = symbol_table.intern(name)
        vm.push(addr)
        vm.advance_pc()
        return None

    return handle_make_symbol


def is_atom_handler(gc: GarbageCollector) -> Any:
    """Create an IS_ATOM handler that captures the GC."""
    def handle_is_atom(
        vm: GenericVM, instr: Instruction, code: CodeObject,
    ) -> str | None:
        """IS_ATOM — Check if a value is an atom (not a cons cell).

        Numbers, symbols, strings, NIL are atoms. Cons cells are not.
        We check: if the value is an int AND it's a valid heap address
        pointing to a ConsCell, it's not an atom.
        """
        value = vm.pop()
        if value is NIL:
            vm.push(1)  # NIL is an atom
        elif isinstance(value, int) and gc.is_valid_address(value):
            obj = gc.deref(value)
            if isinstance(obj, ConsCell):
                vm.push(0)  # Cons cell is NOT an atom
            else:
                vm.push(1)  # Symbol on heap is an atom
        else:
            vm.push(1)  # Numbers, strings, etc. are atoms
        vm.advance_pc()
        return None

    return handle_is_atom


def handle_is_nil(
    vm: GenericVM, instr: Instruction, code: CodeObject,
) -> str | None:
    """IS_NIL — Check if the value is NIL."""
    value = vm.pop()
    vm.push(1 if value is NIL else 0)
    vm.advance_pc()
    return None


# =========================================================================
# I/O Handler (0xA0)
# =========================================================================


def print_handler(gc: GarbageCollector) -> Any:
    """Create a PRINT handler that can pretty-print heap objects."""
    def handle_print(
        vm: GenericVM, instr: Instruction, code: CodeObject,
    ) -> str | None:
        """PRINT — Print the top of stack."""
        value = vm.pop()
        # If the value is a valid heap address, tell _format_value to
        # treat it as a pointer so it dereferences cons cells / symbols.
        is_ptr = isinstance(value, int) and gc.is_valid_address(value)
        text = _format_value(value, gc, is_ptr=is_ptr)
        vm.advance_pc()
        return text

    return handle_print


def _format_value(value: Any, gc: GarbageCollector, *, is_ptr: bool = False, visited: set[int] | None = None) -> str:
    """Format a Lisp value for display.

    Args:
        value: The value to format.
        gc: The garbage collector for heap lookups.
        is_ptr: If True, this value is known to be a heap address.
        visited: Set of already-visited addresses (cycle detection).
    """
    if value is NIL:
        return "nil"
    if value is True:
        return "t"
    if value is False:
        return "nil"
    if is_ptr and isinstance(value, int) and gc.is_valid_address(value):
        if visited is None:
            visited = set()
        if value in visited:
            return "..."
        obj = gc.deref(value)
        if isinstance(obj, ConsCell):
            return _format_cons(value, gc, visited)
        from garbage_collector import Symbol
        if isinstance(obj, Symbol):
            return obj.name
    if isinstance(value, LispFunction):
        return repr(value)
    return str(value)


def _format_cons(addr: int, gc: GarbageCollector, visited: set[int]) -> str:
    """Format a cons cell as a Lisp list or dotted pair.

    Uses iterative traversal of the cdr chain to avoid stack overflow,
    with visited tracking to detect cycles.
    """
    parts = []
    current = addr
    while isinstance(current, int) and gc.is_valid_address(current):
        if current in visited:
            parts.append("...")
            break
        visited.add(current)
        obj = gc.deref(current)
        if not isinstance(obj, ConsCell):
            break
        # Format car — it's a pointer only if it's an int pointing to a heap object
        car_is_ptr = isinstance(obj.car, int) and gc.is_valid_address(obj.car)
        parts.append(_format_value(obj.car, gc, is_ptr=car_is_ptr, visited=visited))
        if obj.cdr is NIL:
            return "(" + " ".join(parts) + ")"
        # Check if cdr is a cons cell (continue list) or something else (dotted pair)
        if isinstance(obj.cdr, int) and gc.is_valid_address(obj.cdr):
            cdr_obj = gc.deref(obj.cdr)
            if isinstance(cdr_obj, ConsCell):
                current = obj.cdr
                continue
            # cdr points to a non-cons heap object
            cdr_str = _format_value(obj.cdr, gc, is_ptr=True, visited=visited)
            return "(" + " ".join(parts) + " . " + cdr_str + ")"
        # cdr is a plain value (number, string, etc.)
        return "(" + " ".join(parts) + " . " + str(obj.cdr) + ")"
    return "(" + " ".join(parts) + ")"


# =========================================================================
# Halt Handler (0xFF)
# =========================================================================


def handle_halt(
    vm: GenericVM, instr: Instruction, code: CodeObject,
) -> str | None:
    """HALT — Stop execution."""
    vm.halted = True
    return None
