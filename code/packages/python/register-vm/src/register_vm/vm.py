"""Register-based virtual machine — execution engine.

This is the core interpreter.  It follows the V8 Ignition model:

* **Accumulator** — a single implicit register that is the source/destination
  for most operations.  This reduces the number of operand bytes needed in
  the instruction encoding.
* **Register file** — a fixed-size array of ``VMValue`` slots per call frame.
  Registers are addressed by integer index in operands.
* **Feedback vectors** — per-call-frame inline-cache state, updated as
  operations execute (see ``feedback.py``).

Dispatch loop
--------------
Python 3.12's ``match/case`` statement is used for opcode dispatch.  It
compiles to a jump table on CPython 3.12+ (via ``MATCH_SEQUENCE``), giving
O(1) dispatch instead of a chain of ``if/elif`` comparisons.

Each iteration of the inner loop::

    1. Fetch instruction at ``frame.ip``.
    2. Advance ``frame.ip`` by 1 (pre-increment, like most real ISAs).
    3. Dispatch on opcode, reading operands.
    4. Write result to accumulator (or registers, or return).

Relative jumps
--------------
All jump operands are *relative to the instruction after the jump* (i.e.
after the pre-increment in step 2).  So ``JUMP 0`` is a no-op, ``JUMP 1``
skips one instruction, and ``JUMP -2`` is a two-instruction backward loop.

This matches the encoding used in CPython's bytecode and in most modern
interpreters.

Call convention
---------------
Function calls are implemented by recursive Python calls to ``_run_frame``.
This keeps the implementation simple but means the Python call stack grows
with JS call depth.  A production interpreter would use an explicit stack
(continuation-passing or manual frame management) to avoid Python's
1000-frame recursion limit.  We guard against this with ``max_depth``.

Global ``print``
-----------------
The module-level ``_globals`` dict in each ``RegisterVM`` instance is
pre-seeded with a ``"print"`` entry that appends to ``self._output`` instead
of writing to ``sys.stdout``.  This makes tests deterministic and avoids
polluting the terminal.
"""

from __future__ import annotations

import copy
from typing import Any

from register_vm.feedback import (
    new_hidden_class_id,
    new_vector,
    record_binary_op,
    record_call_site,
    record_property_load,
    value_type,
)
from register_vm.opcodes import Opcode
from register_vm.scope import get_slot, new_context, set_slot
from register_vm.types import (
    UNDEFINED,
    CallFrame,
    CodeObject,
    Context,
    TraceStep,
    VMError,
    VMFunction,
    VMObject,
    VMResult,
    VMValue,
)

# ---------------------------------------------------------------------------
# Module-level hidden class counter for objects created directly by the VM
# (not via user bytecode).
# ---------------------------------------------------------------------------

_next_hidden_class_id: int = 0


def _new_object() -> VMObject:
    """Create a fresh ``VMObject`` with a unique hidden class ID.

    Each call to this function produces an object with a new, unique
    ``hidden_class_id``.  Objects created sequentially will have
    different IDs even if they end up with identical properties, because
    we don't yet track shape transitions.

    Returns:
        A new ``VMObject`` with an empty properties dict.

    Example::

        a = _new_object()
        b = _new_object()
        assert a.hidden_class_id != b.hidden_class_id
        assert a.properties == {}
    """
    global _next_hidden_class_id
    obj = VMObject(hidden_class_id=_next_hidden_class_id)
    _next_hidden_class_id += 1
    return obj


# ---------------------------------------------------------------------------
# Arithmetic helpers
# ---------------------------------------------------------------------------

def _do_add(a: VMValue, b: VMValue) -> VMValue:
    """Add two VM values following JavaScript-style coercion rules.

    Coercion rules (in priority order):

    1. If *either* operand is a ``str`` → coerce both to ``str``, concatenate.
    2. If both are numeric (``int`` or ``float``) → standard numeric addition.
    3. Otherwise → raise ``TypeError`` (will be caught by the VM dispatch).

    Note that ``bool`` values fall through to numeric addition because
    ``bool`` is a subclass of ``int`` in Python.  So ``True + True == 2``.

    Args:
        a: Left operand (accumulator value).
        b: Right operand (register value).

    Returns:
        The sum or concatenation.

    Raises:
        TypeError: If the operands cannot be added under these rules.

    Examples::

        _do_add(1, 2)         # 3
        _do_add(1.5, 0.5)     # 2.0
        _do_add("hi", "!")    # "hi!"
        _do_add("hi", 2)      # "hi2"
        _do_add(None, 1)      # TypeError
    """
    if isinstance(a, str) or isinstance(b, str):
        return str(a) + str(b)
    if isinstance(a, (int, float)) and isinstance(b, (int, float)):
        return a + b
    raise TypeError(f"Cannot add {type(a).__name__} and {type(b).__name__}")


def _is_truthy(v: VMValue) -> bool:
    """Convert a VM value to a boolean following JavaScript semantics.

    Falsy values: ``UNDEFINED``, ``None``, ``False``, ``0``, ``0.0``, ``""``.
    Everything else (including empty objects and empty lists) is truthy,
    matching Python's default ``bool()`` semantics for those types.

    Args:
        v: Any VM value.

    Returns:
        ``True`` if the value is truthy, ``False`` otherwise.

    Examples::

        _is_truthy(0)         # False
        _is_truthy("")        # False
        _is_truthy(None)      # False
        _is_truthy(UNDEFINED) # False
        _is_truthy(1)         # True
        _is_truthy("hi")      # True
        _is_truthy([])        # True  (JS semantics; Python would be False)
    """
    if v is UNDEFINED or v is None:
        return False
    if isinstance(v, bool):
        return v
    if isinstance(v, (int, float)):
        return v != 0
    if isinstance(v, str):
        return len(v) > 0
    # Objects, lists, VMFunction are always truthy (like JS).
    return True


def _strict_equal(a: VMValue, b: VMValue) -> bool:
    """JavaScript strict equality (``===``).

    Two values are strictly equal iff they are the same type AND the same
    value.  This differs from Python's ``==`` in the ``bool``/``int`` edge
    case: ``0 === false`` is ``False`` in JS, but ``0 == False`` is ``True``
    in Python.

    Args:
        a: Left operand.
        b: Right operand.

    Returns:
        ``True`` if strictly equal.

    Examples::

        _strict_equal(0, False)   # False  (different JS types)
        _strict_equal(0, 0)       # True
        _strict_equal(None, None) # True
    """
    if a is b:
        return True
    if type(a) is not type(b):
        return False
    return a == b


# ---------------------------------------------------------------------------
# VM class
# ---------------------------------------------------------------------------

class RegisterVM:
    """A register-based bytecode interpreter with accumulator and feedback vectors.

    Instantiate once per script / session.  Globals accumulate across
    multiple ``execute`` calls, simulating a REPL-like environment where
    names defined in one snippet are available in the next.

    Attributes:
        _globals:    Mutable global variable store.  Pre-seeded with ``print``.
        _output:     Lines collected from the VM's ``print`` built-in.
        _call_depth: Current recursive call depth (reset per top-level call).
        _max_depth:  Maximum allowed call depth before ``VMError`` is raised.

    Example usage::

        from register_vm import RegisterVM, CodeObject, RegisterInstruction, Opcode

        code = CodeObject(
            instructions=[
                RegisterInstruction(Opcode.LDA_SMI, [7]),
                RegisterInstruction(Opcode.RETURN),
            ],
            constants=[],
            names=[],
            register_count=0,
            feedback_slot_count=0,
        )
        vm = RegisterVM()
        result = vm.execute(code)
        assert result.return_value == 7
    """

    def __init__(self, max_depth: int = 500) -> None:
        """Initialize the VM with an empty global environment.

        Args:
            max_depth: Maximum call stack depth.  Defaults to 500.
        """
        self._globals: dict[str, VMValue] = {}
        self._output: list[str] = []
        self._call_depth: int = 0
        self._max_depth = max_depth

        # Seed the ``print`` global — captures output for testing.
        def _print_fn(*args: Any) -> None:
            self._output.append(" ".join(str(a) for a in args))

        self._globals["print"] = _print_fn  # type: ignore[assignment]

    def execute(self, code: CodeObject) -> VMResult:
        """Execute a ``CodeObject`` and return the result.

        Resets the call depth and output buffer before execution so that
        successive calls to ``execute`` don't bleed state into each other.

        Args:
            code: The compiled code to execute.

        Returns:
            A ``VMResult`` with the return value, captured output lines,
            and any ``VMError`` (or ``None`` on success).

        Example::

            result = vm.execute(code)
            if result.error:
                print(f"Error: {result.error}")
            else:
                print(f"Result: {result.return_value}")
        """
        self._output = []
        self._call_depth = 0
        try:
            frame = self._new_frame(code, caller_frame=None)
            return_value = self._run_frame(frame)
            return VMResult(
                return_value=return_value,
                output=list(self._output),
                error=None,
            )
        except VMError as e:
            return VMResult(
                return_value=UNDEFINED,
                output=list(self._output),
                error=e,
            )

    def execute_with_trace(
        self, code: CodeObject
    ) -> tuple[VMResult, list[TraceStep]]:
        """Execute a ``CodeObject`` and return both the result and an execution trace.

        The trace is a list of ``TraceStep`` objects, one per instruction
        executed.  Each step records the instruction pointer, the instruction
        itself, and the accumulator/register state before and after execution.

        This is more expensive than ``execute`` because it takes a shallow copy
        of the register file at each step.

        Args:
            code: The compiled code to execute.

        Returns:
            A ``(VMResult, list[TraceStep])`` tuple.

        Example::

            result, trace = vm.execute_with_trace(code)
            for step in trace:
                print(f"  ip={step.ip} {Opcode(step.instruction.opcode).name} "
                      f"acc: {step.acc_before} → {step.acc_after}")
        """
        self._output = []
        self._call_depth = 0
        trace: list[TraceStep] = []
        try:
            frame = self._new_frame(code, caller_frame=None)
            return_value = self._run_frame(frame, trace=trace, frame_depth=0)
            return (
                VMResult(return_value=return_value, output=list(self._output), error=None),
                trace,
            )
        except VMError as e:
            return (
                VMResult(return_value=UNDEFINED, output=list(self._output), error=e),
                trace,
            )

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    def _new_frame(
        self,
        code: CodeObject,
        caller_frame: CallFrame | None,
        args: list[VMValue] | None = None,
        context: Context | None = None,
    ) -> CallFrame:
        """Allocate and initialise a new call frame for ``code``.

        Registers are pre-populated with function arguments (if any);
        remaining registers are initialised to ``UNDEFINED``.

        Args:
            code:         The ``CodeObject`` to execute.
            caller_frame: The enclosing frame, or ``None`` at the top level.
            args:         Argument values to load into the first ``len(args)``
                          registers.
            context:      The lexical context to use (for closures).

        Returns:
            A fresh, ready-to-execute ``CallFrame``.
        """
        registers: list[VMValue] = [UNDEFINED] * code.register_count
        if args:
            for i, arg in enumerate(args):
                if i < code.register_count:
                    registers[i] = arg
        feedback_vector = new_vector(code.feedback_slot_count)
        return CallFrame(
            code=code,
            ip=0,
            accumulator=UNDEFINED,
            registers=registers,
            feedback_vector=feedback_vector,
            context=context,
            caller_frame=caller_frame,
        )

    def _run_frame(
        self,
        frame: CallFrame,
        trace: list[TraceStep] | None = None,
        frame_depth: int = 0,
    ) -> VMValue:
        """Execute instructions in ``frame`` until RETURN or HALT.

        This is the main interpreter loop.  It uses Python 3.12's
        ``match/case`` for O(1) dispatch on the opcode integer value.

        Args:
            frame:       The call frame to execute.
            trace:       Optional list to append ``TraceStep`` entries to.
            frame_depth: The call-stack depth (0 = outermost).

        Returns:
            The accumulator value when the frame terminates.

        Raises:
            VMError: On any runtime error (undefined variable, type error,
                     stack overflow, etc.).
        """
        instructions = frame.code.instructions
        n = len(instructions)

        while frame.ip < n:
            instr = instructions[frame.ip]
            op = instr.opcode
            operands = instr.operands

            # Record state before execution (for tracing).
            if trace is not None:
                acc_before = frame.accumulator
                regs_before = list(frame.registers)

            # Advance IP *before* executing so jumps can override it.
            frame.ip += 1

            try:
                match op:
                    # --------------------------------------------------
                    # 0x0_ Accumulator loads
                    # --------------------------------------------------

                    case Opcode.LDA_CONSTANT:
                        # Load constants[operands[0]] into the accumulator.
                        frame.accumulator = frame.code.constants[operands[0]]

                    case Opcode.LDA_ZERO:
                        # Optimized literal 0 — no constant pool lookup.
                        frame.accumulator = 0

                    case Opcode.LDA_SMI:
                        # Small integer encoded directly in the instruction.
                        frame.accumulator = operands[0]

                    case Opcode.LDA_UNDEFINED:
                        frame.accumulator = UNDEFINED

                    case Opcode.LDA_NULL:
                        frame.accumulator = None

                    case Opcode.LDA_TRUE:
                        frame.accumulator = True

                    case Opcode.LDA_FALSE:
                        frame.accumulator = False

                    # --------------------------------------------------
                    # 0x1_ Register moves
                    # --------------------------------------------------

                    case Opcode.LDAR:
                        # Load a register into the accumulator.
                        frame.accumulator = frame.registers[operands[0]]

                    case Opcode.STAR:
                        # Store accumulator into a register.
                        frame.registers[operands[0]] = frame.accumulator

                    case Opcode.MOV:
                        # Copy one register to another (acc unchanged).
                        frame.registers[operands[1]] = frame.registers[operands[0]]

                    # --------------------------------------------------
                    # 0x2_ Variable access
                    # --------------------------------------------------

                    case Opcode.LDA_GLOBAL:
                        name = frame.code.names[operands[0]]
                        if name not in self._globals:
                            raise VMError(
                                message=f"Undefined global '{name}'",
                                instruction_index=frame.ip - 1,
                                opcode=op,
                            )
                        val = self._globals[name]
                        # If it's a callable Python function, wrap it.
                        frame.accumulator = val  # type: ignore[assignment]

                    case Opcode.STA_GLOBAL:
                        name = frame.code.names[operands[0]]
                        self._globals[name] = frame.accumulator

                    case Opcode.LDA_LOCAL:
                        # Alias for LDAR for compiler readability.
                        frame.accumulator = frame.registers[operands[0]]

                    case Opcode.STA_LOCAL:
                        # Alias for STAR for compiler readability.
                        frame.registers[operands[0]] = frame.accumulator

                    case Opcode.LDA_CONTEXT_SLOT:
                        if frame.context is None:
                            raise VMError(
                                message="No context available",
                                instruction_index=frame.ip - 1,
                                opcode=op,
                            )
                        depth, idx = operands[0], operands[1]
                        frame.accumulator = get_slot(frame.context, depth, idx)

                    case Opcode.STA_CONTEXT_SLOT:
                        if frame.context is None:
                            raise VMError(
                                message="No context available",
                                instruction_index=frame.ip - 1,
                                opcode=op,
                            )
                        depth, idx = operands[0], operands[1]
                        set_slot(frame.context, depth, idx, frame.accumulator)

                    case Opcode.LDA_CURRENT_CONTEXT_SLOT:
                        if frame.context is None:
                            raise VMError(
                                message="No context available",
                                instruction_index=frame.ip - 1,
                                opcode=op,
                            )
                        frame.accumulator = get_slot(frame.context, 0, operands[0])

                    case Opcode.STA_CURRENT_CONTEXT_SLOT:
                        if frame.context is None:
                            raise VMError(
                                message="No context available",
                                instruction_index=frame.ip - 1,
                                opcode=op,
                            )
                        set_slot(frame.context, 0, operands[0], frame.accumulator)

                    # --------------------------------------------------
                    # 0x3_ Arithmetic and bitwise operations
                    # --------------------------------------------------

                    case Opcode.ADD:
                        right = frame.registers[operands[0]]
                        slot_idx = operands[1] if len(operands) > 1 else -1
                        record_binary_op(
                            frame.feedback_vector, slot_idx,
                            frame.accumulator, right,
                        )
                        try:
                            frame.accumulator = _do_add(frame.accumulator, right)
                        except TypeError as exc:
                            raise VMError(
                                message=str(exc),
                                instruction_index=frame.ip - 1,
                                opcode=op,
                            ) from exc

                    case Opcode.SUB:
                        right = frame.registers[operands[0]]
                        slot_idx = operands[1] if len(operands) > 1 else -1
                        record_binary_op(
                            frame.feedback_vector, slot_idx,
                            frame.accumulator, right,
                        )
                        frame.accumulator = frame.accumulator - right  # type: ignore[operator]

                    case Opcode.MUL:
                        right = frame.registers[operands[0]]
                        slot_idx = operands[1] if len(operands) > 1 else -1
                        record_binary_op(
                            frame.feedback_vector, slot_idx,
                            frame.accumulator, right,
                        )
                        frame.accumulator = frame.accumulator * right  # type: ignore[operator]

                    case Opcode.DIV:
                        right = frame.registers[operands[0]]
                        if right == 0:
                            raise VMError(
                                message="Division by zero",
                                instruction_index=frame.ip - 1,
                                opcode=op,
                            )
                        slot_idx = operands[1] if len(operands) > 1 else -1
                        record_binary_op(
                            frame.feedback_vector, slot_idx,
                            frame.accumulator, right,
                        )
                        frame.accumulator = frame.accumulator / right  # type: ignore[operator]

                    case Opcode.MOD:
                        right = frame.registers[operands[0]]
                        if right == 0:
                            raise VMError(
                                message="Modulo by zero",
                                instruction_index=frame.ip - 1,
                                opcode=op,
                            )
                        frame.accumulator = frame.accumulator % right  # type: ignore[operator]

                    case Opcode.POW:
                        right = frame.registers[operands[0]]
                        frame.accumulator = frame.accumulator ** right  # type: ignore[operator]

                    case Opcode.ADD_SMI:
                        # Add a small integer literal directly (no register lookup).
                        frame.accumulator = frame.accumulator + operands[0]  # type: ignore[operator]

                    case Opcode.SUB_SMI:
                        frame.accumulator = frame.accumulator - operands[0]  # type: ignore[operator]

                    case Opcode.BITWISE_AND:
                        right = frame.registers[operands[0]]
                        frame.accumulator = int(frame.accumulator) & int(right)  # type: ignore[arg-type]

                    case Opcode.BITWISE_OR:
                        right = frame.registers[operands[0]]
                        frame.accumulator = int(frame.accumulator) | int(right)  # type: ignore[arg-type]

                    case Opcode.BITWISE_XOR:
                        right = frame.registers[operands[0]]
                        frame.accumulator = int(frame.accumulator) ^ int(right)  # type: ignore[arg-type]

                    case Opcode.BITWISE_NOT:
                        frame.accumulator = ~int(frame.accumulator)  # type: ignore[arg-type]

                    case Opcode.SHIFT_LEFT:
                        right = frame.registers[operands[0]]
                        frame.accumulator = int(frame.accumulator) << int(right)  # type: ignore[arg-type]

                    case Opcode.SHIFT_RIGHT:
                        right = frame.registers[operands[0]]
                        frame.accumulator = int(frame.accumulator) >> int(right)  # type: ignore[arg-type]

                    case Opcode.SHIFT_RIGHT_LOGICAL:
                        # Unsigned 32-bit right shift (mask to 32 bits first).
                        right = frame.registers[operands[0]]
                        frame.accumulator = (int(frame.accumulator) & 0xFFFFFFFF) >> int(right)  # type: ignore[arg-type]

                    case Opcode.NEGATE:
                        frame.accumulator = -frame.accumulator  # type: ignore[operator]

                    # --------------------------------------------------
                    # 0x4_ Comparisons
                    # --------------------------------------------------

                    case Opcode.TEST_EQUAL:
                        right = frame.registers[operands[0]]
                        frame.accumulator = frame.accumulator == right

                    case Opcode.TEST_NOT_EQUAL:
                        right = frame.registers[operands[0]]
                        frame.accumulator = frame.accumulator != right

                    case Opcode.TEST_STRICT_EQUAL:
                        right = frame.registers[operands[0]]
                        frame.accumulator = _strict_equal(frame.accumulator, right)

                    case Opcode.TEST_STRICT_NOT_EQUAL:
                        right = frame.registers[operands[0]]
                        frame.accumulator = not _strict_equal(frame.accumulator, right)

                    case Opcode.TEST_LESS_THAN:
                        right = frame.registers[operands[0]]
                        frame.accumulator = frame.accumulator < right  # type: ignore[operator]

                    case Opcode.TEST_GREATER_THAN:
                        right = frame.registers[operands[0]]
                        frame.accumulator = frame.accumulator > right  # type: ignore[operator]

                    case Opcode.TEST_LESS_THAN_OR_EQUAL:
                        right = frame.registers[operands[0]]
                        frame.accumulator = frame.accumulator <= right  # type: ignore[operator]

                    case Opcode.TEST_GREATER_THAN_OR_EQUAL:
                        right = frame.registers[operands[0]]
                        frame.accumulator = frame.accumulator >= right  # type: ignore[operator]

                    case Opcode.TEST_IN:
                        right = frame.registers[operands[0]]
                        if isinstance(right, VMObject):
                            frame.accumulator = frame.accumulator in right.properties
                        elif isinstance(right, (list, str, dict)):
                            frame.accumulator = frame.accumulator in right
                        else:
                            raise VMError(
                                message=f"Cannot use 'in' with {type(right).__name__}",
                                instruction_index=frame.ip - 1,
                                opcode=op,
                            )

                    case Opcode.TEST_INSTANCEOF:
                        right = frame.registers[operands[0]]
                        frame.accumulator = type(frame.accumulator) is type(right)

                    case Opcode.TEST_UNDETECTABLE:
                        frame.accumulator = (
                            frame.accumulator is None or frame.accumulator is UNDEFINED
                        )

                    case Opcode.LOGICAL_NOT:
                        frame.accumulator = not _is_truthy(frame.accumulator)

                    case Opcode.TYPEOF:
                        frame.accumulator = value_type(frame.accumulator)

                    # --------------------------------------------------
                    # 0x5_ Control flow
                    # --------------------------------------------------

                    case Opcode.JUMP:
                        # Unconditional relative jump (ip already advanced by 1).
                        frame.ip += operands[0]

                    case Opcode.JUMP_IF_TRUE:
                        if _is_truthy(frame.accumulator):
                            frame.ip += operands[0]

                    case Opcode.JUMP_IF_FALSE:
                        if not _is_truthy(frame.accumulator):
                            frame.ip += operands[0]

                    case Opcode.JUMP_IF_NULL:
                        if frame.accumulator is None:
                            frame.ip += operands[0]

                    case Opcode.JUMP_IF_UNDEFINED:
                        if frame.accumulator is UNDEFINED:
                            frame.ip += operands[0]

                    case Opcode.JUMP_IF_NULL_OR_UNDEFINED:
                        if frame.accumulator is None or frame.accumulator is UNDEFINED:
                            frame.ip += operands[0]

                    case Opcode.JUMP_IF_TO_BOOLEAN_TRUE:
                        if _is_truthy(frame.accumulator):
                            frame.ip += operands[0]

                    case Opcode.JUMP_IF_TO_BOOLEAN_FALSE:
                        if not _is_truthy(frame.accumulator):
                            frame.ip += operands[0]

                    case Opcode.JUMP_LOOP:
                        # Backward jump (negative offset) for loop bodies.
                        frame.ip += operands[0]

                    # --------------------------------------------------
                    # 0x6_ Calls
                    # --------------------------------------------------

                    case Opcode.CALL_ANY_RECEIVER | Opcode.CALL_UNDEFINED_RECEIVER:
                        # operands = [callable_reg, first_arg_reg, argc, feedback_slot]
                        callee = frame.registers[operands[0]]
                        first_arg_reg = operands[1]
                        argc = operands[2]
                        fb_slot = operands[3] if len(operands) > 3 else -1
                        args = [
                            frame.registers[first_arg_reg + i]
                            for i in range(argc)
                        ]
                        frame.accumulator = self._call_value(
                            callee, args, frame, fb_slot, frame.ip - 1, op
                        )

                    case Opcode.CALL_PROPERTY:
                        # operands = [callable_reg, receiver_reg, first_arg_reg, argc, feedback_slot]
                        callee = frame.registers[operands[0]]
                        first_arg_reg = operands[2]
                        argc = operands[3]
                        fb_slot = operands[4] if len(operands) > 4 else -1
                        args = [
                            frame.registers[first_arg_reg + i]
                            for i in range(argc)
                        ]
                        frame.accumulator = self._call_value(
                            callee, args, frame, fb_slot, frame.ip - 1, op
                        )

                    case Opcode.CONSTRUCT:
                        # Create a new object, call the constructor, return the object.
                        callee = frame.registers[operands[0]]
                        first_arg_reg = operands[1]
                        argc = operands[2]
                        args = [
                            frame.registers[first_arg_reg + i]
                            for i in range(argc)
                        ]
                        new_obj = _new_object()
                        if isinstance(callee, VMFunction):
                            new_frame = self._new_frame(
                                callee.code,
                                caller_frame=frame,
                                args=args,
                                context=callee.context,
                            )
                            # Pass new_obj as register 0 (the "this" receiver).
                            if new_frame.code.register_count > 0:
                                new_frame.registers[0] = new_obj
                            result = self._run_frame(new_frame, frame_depth=frame_depth + 1)
                            # If constructor returns an object, use it; else use new_obj.
                            if isinstance(result, VMObject):
                                frame.accumulator = result
                            else:
                                frame.accumulator = new_obj
                        else:
                            frame.accumulator = new_obj

                    case Opcode.CONSTRUCT_WITH_SPREAD | Opcode.CALL_WITH_SPREAD:
                        raise VMError(
                            message="Spread calls/constructs not implemented",
                            instruction_index=frame.ip - 1,
                            opcode=op,
                        )

                    case Opcode.RETURN:
                        # Return the accumulator to the caller.
                        if trace is not None:
                            acc_after = frame.accumulator
                            trace.append(TraceStep(
                                frame_depth=frame_depth,
                                ip=frame.ip - 1,
                                instruction=instr,
                                acc_before=acc_before,  # type: ignore[possibly-undefined]
                                acc_after=acc_after,
                                registers_before=regs_before,  # type: ignore[possibly-undefined]
                                registers_after=list(frame.registers),
                            ))
                        return frame.accumulator

                    case Opcode.SUSPEND_GENERATOR | Opcode.RESUME_GENERATOR:
                        raise VMError(
                            message="Generators not implemented",
                            instruction_index=frame.ip - 1,
                            opcode=op,
                        )

                    # --------------------------------------------------
                    # 0x7_ Property access
                    # --------------------------------------------------

                    case Opcode.LDA_NAMED_PROPERTY:
                        # operands = [obj_reg, name_idx, feedback_slot]
                        obj = frame.registers[operands[0]]
                        name = frame.code.names[operands[1]]
                        fb_slot = operands[2] if len(operands) > 2 else -1
                        if isinstance(obj, VMObject):
                            record_property_load(
                                frame.feedback_vector, fb_slot, obj.hidden_class_id
                            )
                            frame.accumulator = obj.properties.get(name, UNDEFINED)
                        elif isinstance(obj, list) and name == "length":
                            frame.accumulator = len(obj)
                        else:
                            raise VMError(
                                message=f"Cannot load property '{name}' from {type(obj).__name__}",
                                instruction_index=frame.ip - 1,
                                opcode=op,
                            )

                    case Opcode.STA_NAMED_PROPERTY:
                        # operands = [obj_reg, name_idx, feedback_slot]
                        obj = frame.registers[operands[0]]
                        name = frame.code.names[operands[1]]
                        fb_slot = operands[2] if len(operands) > 2 else -1
                        if isinstance(obj, VMObject):
                            record_property_load(
                                frame.feedback_vector, fb_slot, obj.hidden_class_id
                            )
                            obj.properties[name] = frame.accumulator
                        else:
                            raise VMError(
                                message=f"Cannot store property '{name}' on {type(obj).__name__}",
                                instruction_index=frame.ip - 1,
                                opcode=op,
                            )

                    case Opcode.LDA_KEYED_PROPERTY:
                        # operands = [obj_reg, key_reg, feedback_slot]
                        obj = frame.registers[operands[0]]
                        key = frame.registers[operands[1]]
                        if isinstance(obj, VMObject):
                            frame.accumulator = obj.properties.get(str(key), UNDEFINED)
                        elif isinstance(obj, list) and isinstance(key, int):
                            frame.accumulator = obj[key] if 0 <= key < len(obj) else UNDEFINED
                        else:
                            raise VMError(
                                message=f"Cannot keyed-load from {type(obj).__name__}",
                                instruction_index=frame.ip - 1,
                                opcode=op,
                            )

                    case Opcode.STA_KEYED_PROPERTY:
                        # operands = [obj_reg, key_reg, feedback_slot]
                        obj = frame.registers[operands[0]]
                        key = frame.registers[operands[1]]
                        if isinstance(obj, VMObject):
                            obj.properties[str(key)] = frame.accumulator
                        elif isinstance(obj, list) and isinstance(key, int):
                            while len(obj) <= key:
                                obj.append(UNDEFINED)
                            obj[key] = frame.accumulator
                        else:
                            raise VMError(
                                message=f"Cannot keyed-store on {type(obj).__name__}",
                                instruction_index=frame.ip - 1,
                                opcode=op,
                            )

                    case Opcode.LDA_NAMED_PROPERTY_NO_FEEDBACK:
                        obj = frame.registers[operands[0]]
                        name = frame.code.names[operands[1]]
                        if isinstance(obj, VMObject):
                            frame.accumulator = obj.properties.get(name, UNDEFINED)
                        elif isinstance(obj, list) and name == "length":
                            frame.accumulator = len(obj)
                        else:
                            raise VMError(
                                message=f"Cannot load property '{name}'",
                                instruction_index=frame.ip - 1,
                                opcode=op,
                            )

                    case Opcode.STA_NAMED_PROPERTY_NO_FEEDBACK:
                        obj = frame.registers[operands[0]]
                        name = frame.code.names[operands[1]]
                        if isinstance(obj, VMObject):
                            obj.properties[name] = frame.accumulator
                        else:
                            raise VMError(
                                message=f"Cannot store property '{name}'",
                                instruction_index=frame.ip - 1,
                                opcode=op,
                            )

                    case Opcode.DELETE_PROPERTY_STRICT | Opcode.DELETE_PROPERTY_SLOPPY:
                        obj = frame.registers[operands[0]]
                        key = frame.registers[operands[1]]
                        if isinstance(obj, VMObject):
                            obj.properties.pop(str(key), None)
                            frame.accumulator = True
                        else:
                            frame.accumulator = False

                    # --------------------------------------------------
                    # 0x8_ Object / array / closure creation
                    # --------------------------------------------------

                    case Opcode.CREATE_OBJECT_LITERAL:
                        frame.accumulator = _new_object()

                    case Opcode.CREATE_ARRAY_LITERAL:
                        frame.accumulator = []

                    case Opcode.CREATE_REGEXP_LITERAL:
                        # Return the pattern string as a placeholder.
                        frame.accumulator = frame.code.constants[operands[0]] if operands else ""

                    case Opcode.CREATE_CLOSURE:
                        inner_code = frame.code.constants[operands[0]]
                        if not isinstance(inner_code, CodeObject):
                            raise VMError(
                                message=f"CREATE_CLOSURE: constants[{operands[0]}] is not a CodeObject",
                                instruction_index=frame.ip - 1,
                                opcode=op,
                            )
                        frame.accumulator = VMFunction(
                            code=inner_code, context=frame.context
                        )

                    case Opcode.CREATE_CONTEXT | Opcode.PUSH_CONTEXT:
                        slot_count = operands[0] if operands else 0
                        frame.context = new_context(frame.context, slot_count)

                    case Opcode.CLONE_OBJECT:
                        if isinstance(frame.accumulator, VMObject):
                            cloned = VMObject(
                                hidden_class_id=new_hidden_class_id(),
                                properties=dict(frame.accumulator.properties),
                            )
                            frame.accumulator = cloned
                        else:
                            frame.accumulator = copy.copy(frame.accumulator)

                    # --------------------------------------------------
                    # 0x9_ Iteration
                    # --------------------------------------------------

                    case Opcode.GET_ITERATOR:
                        iterable = frame.accumulator
                        if isinstance(iterable, list):
                            iter_obj = _new_object()
                            iter_obj.properties["_iter"] = iter(iterable)  # type: ignore[assignment]
                            iter_obj.properties["_is_iterator"] = True
                            frame.accumulator = iter_obj
                        elif isinstance(iterable, VMObject):
                            iter_obj = _new_object()
                            iter_obj.properties["_iter"] = iter(iterable.properties.values())  # type: ignore[assignment]
                            iter_obj.properties["_is_iterator"] = True
                            frame.accumulator = iter_obj
                        else:
                            raise VMError(
                                message=f"Cannot iterate {type(iterable).__name__}",
                                instruction_index=frame.ip - 1,
                                opcode=op,
                            )

                    case Opcode.CALL_ITERATOR_STEP:
                        iter_obj = frame.accumulator
                        if not isinstance(iter_obj, VMObject):
                            raise VMError(
                                message="CALL_ITERATOR_STEP: not an iterator object",
                                instruction_index=frame.ip - 1,
                                opcode=op,
                            )
                        result_obj = _new_object()
                        raw_iter = iter_obj.properties.get("_iter")
                        try:
                            val = next(raw_iter)  # type: ignore[call-overload]
                            result_obj.properties["done"] = False
                            result_obj.properties["value"] = val
                        except StopIteration:
                            result_obj.properties["done"] = True
                            result_obj.properties["value"] = UNDEFINED
                        frame.accumulator = result_obj

                    case Opcode.GET_ITERATOR_DONE:
                        if isinstance(frame.accumulator, VMObject):
                            frame.accumulator = frame.accumulator.properties.get("done", UNDEFINED)
                        else:
                            frame.accumulator = UNDEFINED

                    case Opcode.GET_ITERATOR_VALUE:
                        if isinstance(frame.accumulator, VMObject):
                            frame.accumulator = frame.accumulator.properties.get("value", UNDEFINED)
                        else:
                            frame.accumulator = UNDEFINED

                    # --------------------------------------------------
                    # 0xA_ Exception handling
                    # --------------------------------------------------

                    case Opcode.THROW:
                        msg = str(frame.accumulator)
                        raise VMError(
                            message=msg,
                            instruction_index=frame.ip - 1,
                            opcode=op,
                        )

                    case Opcode.RETHROW:
                        raise VMError(
                            message="RETHROW: not implemented",
                            instruction_index=frame.ip - 1,
                            opcode=op,
                        )

                    # --------------------------------------------------
                    # 0xB_ Context / module
                    # --------------------------------------------------

                    case Opcode.POP_CONTEXT:
                        if frame.context is not None and frame.context.parent is not None:
                            frame.context = frame.context.parent

                    case Opcode.LDA_MODULE_VARIABLE:
                        name = frame.code.names[operands[0]]
                        frame.accumulator = self._globals.get(name, UNDEFINED)

                    case Opcode.STA_MODULE_VARIABLE:
                        name = frame.code.names[operands[0]]
                        self._globals[name] = frame.accumulator

                    # --------------------------------------------------
                    # 0xF_ VM control
                    # --------------------------------------------------

                    case Opcode.STACK_CHECK:
                        self._call_depth += 1
                        if self._call_depth > self._max_depth:
                            raise VMError(
                                message=(
                                    f"Maximum call stack depth ({self._max_depth}) exceeded"
                                ),
                                instruction_index=frame.ip - 1,
                                opcode=op,
                            )

                    case Opcode.DEBUGGER:
                        # No-op in this implementation.
                        pass

                    case Opcode.HALT:
                        if trace is not None:
                            acc_after = frame.accumulator
                            trace.append(TraceStep(
                                frame_depth=frame_depth,
                                ip=frame.ip - 1,
                                instruction=instr,
                                acc_before=acc_before,  # type: ignore[possibly-undefined]
                                acc_after=acc_after,
                                registers_before=regs_before,  # type: ignore[possibly-undefined]
                                registers_after=list(frame.registers),
                            ))
                        return frame.accumulator

                    case _:
                        raise VMError(
                            message=f"Unknown opcode 0x{op:02X}",
                            instruction_index=frame.ip - 1,
                            opcode=op,
                        )

            except VMError:
                raise
            except Exception as exc:
                raise VMError(
                    message=str(exc),
                    instruction_index=frame.ip - 1,
                    opcode=op,
                ) from exc

            # Append trace step after successful execution.
            if trace is not None:
                trace.append(TraceStep(
                    frame_depth=frame_depth,
                    ip=frame.ip - 1,
                    instruction=instr,
                    acc_before=acc_before,  # type: ignore[possibly-undefined]
                    acc_after=frame.accumulator,
                    registers_before=regs_before,  # type: ignore[possibly-undefined]
                    registers_after=list(frame.registers),
                ))

        # Implicit HALT at end of instructions.
        return frame.accumulator

    def _call_value(
        self,
        callee: VMValue,
        args: list[VMValue],
        frame: CallFrame,
        fb_slot: int,
        ip: int,
        op: int,
    ) -> VMValue:
        """Dispatch a function call to the appropriate handler.

        Handles three kinds of callees:
        1. ``VMFunction`` — bytecode function (recursive ``_run_frame`` call).
        2. Python callable (stored in globals) — direct Python call.
        3. Anything else — raises ``VMError``.

        Args:
            callee:  The value to call.
            args:    Argument list.
            frame:   The calling frame (for feedback recording).
            fb_slot: Feedback slot index for this call site.
            ip:      The IP of the CALL instruction (for error messages).
            op:      The opcode (for error messages).

        Returns:
            The return value.

        Raises:
            VMError: If the callee is not callable.
        """
        if isinstance(callee, VMFunction):
            record_call_site(frame.feedback_vector, fb_slot, "function")
            new_frame = self._new_frame(
                callee.code,
                caller_frame=frame,
                args=args,
                context=callee.context,
            )
            return self._run_frame(new_frame, frame_depth=1)
        if callable(callee):
            record_call_site(frame.feedback_vector, fb_slot, "builtin")
            result = callee(*args)
            return result if result is not None else UNDEFINED
        raise VMError(
            message=f"Not callable: {type(callee).__name__}",
            instruction_index=ip,
            opcode=op,
        )


# ---------------------------------------------------------------------------
# Module-level convenience functions
# ---------------------------------------------------------------------------

def execute(code: CodeObject) -> VMResult:
    """Create a fresh ``RegisterVM`` and execute ``code``.

    Convenience wrapper for one-off script execution where you don't
    need to share global state across multiple calls.

    Args:
        code: The ``CodeObject`` to execute.

    Returns:
        A ``VMResult``.

    Example::

        result = execute(code)
        assert result.return_value == 42
    """
    return RegisterVM().execute(code)


def execute_with_trace(code: CodeObject) -> tuple[VMResult, list[TraceStep]]:
    """Create a fresh ``RegisterVM`` and execute ``code`` with full tracing.

    Args:
        code: The ``CodeObject`` to execute.

    Returns:
        A ``(VMResult, list[TraceStep])`` tuple.

    Example::

        result, trace = execute_with_trace(code)
        for step in trace:
            print(step.ip, Opcode(step.instruction.opcode).name)
    """
    return RegisterVM().execute_with_trace(code)
