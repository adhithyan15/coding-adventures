"""Generic Virtual Machine — A Pluggable Stack-Based Bytecode Interpreter.

==========================================================================
Chapter 1: Why a *Generic* VM?
==========================================================================

The original ``VirtualMachine`` class (in ``vm.py``) is a complete, working
bytecode interpreter. But it has one limitation: its instruction set is
hardcoded. Every opcode — LOAD_CONST, ADD, JUMP — is baked into a big
``match`` statement inside ``_dispatch()``. Want to add a new opcode? You have
to edit the VM source code.

This is fine for a single language, but we want something more ambitious:
**one VM chassis that can run bytecode from any language** — Starlark today,
Python tomorrow, Ruby next week. Each language defines its own opcodes with
its own semantics, but the *execution engine* (stack, memory, eval loop,
call stack) stays the same.

This is exactly how the JVM works:
    - The JVM provides the chassis: stack frames, GC, class loading.
    - Each JVM language (Java, Kotlin, Scala, Clojure) compiles to JVM
      bytecode using the *same* opcodes, but they could in theory define
      language-specific semantics via different compilation strategies.

Our approach goes one step further: the opcodes themselves are pluggable.
Starlark registers its opcodes (ADD, SUB, BUILD_LIST, etc.), and a future
Python plugin registers additional ones (SETUP_EXCEPT, YIELD_VALUE, etc.).

==========================================================================
Chapter 2: The Plugin Architecture
==========================================================================

The key insight is that every VM, regardless of language, needs the same
fundamental primitives:

**Universal primitives (provided by GenericVM):**

1. An **operand stack** — push/pop values during computation
2. A **call stack** — save/restore execution contexts for function calls
3. **Global variable storage** — named variables in a dictionary
4. **Local variable slots** — fast indexed slots for function locals
5. A **program counter (PC)** — tracks the current instruction
6. A **fetch-decode-execute loop** — the eval loop that drives everything
7. **Execution tracing** — step-by-step snapshots for debugging

**Language-specific parts (provided by plugins):**

1. **Opcode definitions** — what opcodes exist (ADD, BUILD_LIST, etc.)
2. **Opcode handlers** — what each opcode *does* (how ADD works for strings
   vs ints, how BUILD_LIST creates a list, etc.)
3. **Built-in functions** — language-specific standard library (len, range, etc.)
4. **Value type semantics** — truthiness, equality, ordering rules
5. **Runtime restrictions** — recursion limits, freezing, etc.

The plugin interface is simple: languages call ``register_opcode(number, handler)``
to add their opcodes. The eval loop dispatches to the registered handler.
If no handler is registered for an opcode, the VM raises an error.

Think of it like a car:
    - The **chassis** (frame, wheels, steering, brakes) = GenericVM
    - The **engine** (gas, electric, hybrid) = language-specific plugin
    - You can put different engines in the same chassis.

==========================================================================
Chapter 3: OpcodeHandler Protocol
==========================================================================

Each opcode handler is a callable that receives the VM instance, the current
instruction, and the CodeObject. It mutates the VM state (stack, pc, variables)
and optionally returns a string (for PRINT-like opcodes that produce output).

    def handle_add(vm: GenericVM, instr: Instruction, code: CodeObject) -> str | None:
        b = vm.pop()
        a = vm.pop()
        vm.push(a + b)
        vm.advance_pc()
        return None

This signature gives handlers full control over the VM. They can:
- Push/pop values on the stack
- Read/write variables
- Modify the program counter (for jumps)
- Push/pop call frames (for function calls)
- Produce output (for PRINT)
- Raise errors (for invalid operations)

The handler *must* advance the PC when appropriate. Most handlers call
``vm.advance_pc()`` to move to the next instruction. Jump handlers set
``vm.pc`` directly instead.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Protocol

from virtual_machine.vm import (
    CallFrame,
    CodeObject,
    Instruction,
    InvalidOpcodeError,
    StackUnderflowError,
    VMError,
    VMTrace,
)


# =========================================================================
# TypedVMValue — A value tagged with its type
# =========================================================================
#
# In a type-safe execution environment like WebAssembly, every value on the
# stack carries a *type tag* alongside its payload. This is different from
# the untyped GenericVM stack where values can be anything.
#
# The ``type`` field is an integer that corresponds to WASM's ValueType:
#   0x7F = i32, 0x7E = i64, 0x7D = f32, 0x7C = f64
#
# The ``value`` field holds the actual data (Python int, float, etc.).
#
# This dataclass is NOT frozen because the execution engine sometimes
# needs to update values in-place (e.g., in the locals array). It uses
# structural equality by default.
#
#   ┌──────────────────────────────────────────────┐
#   │  TypedVMValue                                 │
#   │  ┌──────────┐  ┌──────────────────────────┐  │
#   │  │ type: int │  │ value: Any               │  │
#   │  │ (0x7F)   │  │ (42)                     │  │
#   │  └──────────┘  └──────────────────────────┘  │
#   └──────────────────────────────────────────────┘
# =========================================================================


@dataclass
class TypedVMValue:
    """A value tagged with its type — the fundamental unit on the typed stack.

    In WebAssembly, every value has an explicit type tag. This contrasts with
    the untyped ``stack`` on GenericVM where any Python object can be pushed.
    The typed stack ensures type safety at runtime.

    Attributes:
        type: An integer type tag (e.g., 0x7F for i32 in WASM).
        value: The raw payload (int, float, or any compatible type).

    Example::

        val = TypedVMValue(type=0x7F, value=42)
        assert val.type == 0x7F
        assert val.value == 42
    """

    type: int
    """Integer type tag (matches WASM ValueType encoding)."""

    value: Any
    """The raw value payload."""


# =========================================================================
# OpcodeHandler Protocol
# =========================================================================


class OpcodeHandler(Protocol):
    """Protocol for opcode handler functions.

    Every opcode handler must conform to this signature. It receives:

    - **vm** — The GenericVM instance. Use ``vm.push()``, ``vm.pop()``, etc.
      to manipulate state.
    - **instr** — The current instruction (opcode + operand).
    - **code** — The CodeObject being executed (for accessing constant/name pools).

    Returns:
        A string if the handler produces output (like PRINT), else None.

    The handler MUST advance ``vm.pc`` when appropriate:
    - Most handlers: call ``vm.advance_pc()`` to move to the next instruction.
    - Jump handlers: set ``vm.pc = target`` directly.
    - HALT handler: set ``vm.halted = True`` (PC doesn't matter).

    Example::

        def handle_load_const(vm, instr, code):
            index = instr.operand
            vm.push(code.constants[index])
            vm.advance_pc()
            return None
    """

    def __call__(
        self,
        vm: GenericVM,
        instr: Instruction,
        code: CodeObject,
    ) -> str | None: ...


# =========================================================================
# TypeError for VM operations
# =========================================================================


class VMTypeError(VMError):
    """Raised when an operation receives values of incompatible types.

    This is our equivalent of Python's ``TypeError`` or Java's
    ``ClassCastException``. It occurs when, for example, you try to
    subtract a string from an integer.
    """


class MaxRecursionError(VMError):
    """Raised when the call stack exceeds the maximum depth.

    This catches infinite recursion (or in Starlark's case, any recursion
    at all, since Starlark forbids it).
    """


# =========================================================================
# BuiltinFunction — callable from bytecode
# =========================================================================


@dataclass
class BuiltinFunction:
    """A built-in function that can be called from bytecode.

    Built-in functions are registered with the VM by name. When the compiler
    encounters a call to ``len(x)`` or ``print(x)``, it emits instructions
    that push the arguments and then CALL the function by name. The VM
    looks up the name, finds a ``BuiltinFunction``, and invokes it.

    The ``implementation`` callable receives the argument list and returns
    a value. It can also raise ``VMError`` subclasses for invalid arguments.

    Example::

        def builtin_len(args):
            if len(args) != 1:
                raise VMTypeError("len() takes exactly 1 argument")
            obj = args[0]
            if isinstance(obj, (list, dict, str, tuple)):
                return len(obj)
            raise VMTypeError(f"object of type '{type(obj).__name__}' has no len()")

        vm.register_builtin("len", builtin_len)
    """

    name: str
    """The function's name as it appears in source code."""

    implementation: Any  # Callable[[list[Any]], Any]
    """The Python function that implements this built-in."""


# =========================================================================
# The Generic VM
# =========================================================================


class GenericVM:
    """A pluggable stack-based bytecode interpreter.

    This is the **chassis** — it provides universal execution primitives that
    every virtual machine needs, regardless of what language it's running.
    Languages register their opcodes via ``register_opcode()`` and their
    built-in functions via ``register_builtin()``.

    **What GenericVM provides (universal):**

    - Operand stack with push/pop/peek
    - Global variable storage (named dictionary)
    - Local variable slots (indexed array)
    - Call stack for function calls
    - Program counter and fetch-decode-execute loop
    - Execution tracing
    - Error infrastructure

    **What plugins provide (language-specific):**

    - Opcode handlers (registered via ``register_opcode()``)
    - Built-in functions (registered via ``register_builtin()``)
    - Runtime restrictions (max recursion depth, freezing, etc.)

    Usage::

        vm = GenericVM()

        # Register opcodes
        vm.register_opcode(0x01, handle_load_const)
        vm.register_opcode(0x20, handle_add)
        vm.register_opcode(0xFF, handle_halt)

        # Register built-ins
        vm.register_builtin("print", builtin_print)
        vm.register_builtin("len", builtin_len)

        # Execute
        traces = vm.execute(code_object)
    """

    def __init__(self) -> None:
        """Initialize a fresh GenericVM with empty state and no opcodes.

        After construction, you must register at least some opcode handlers
        before calling ``execute()``, otherwise every instruction will fail
        with ``InvalidOpcodeError``.
        """
        # -- Execution state --------------------------------------------------
        self.stack: list[Any] = []
        """The operand stack — where all computation happens."""

        self.variables: dict[str, Any] = {}
        """Named variable storage — like a global/module scope dictionary."""

        self.locals: list[Any] = []
        """Local variable slots — a flat array indexed by slot number.

        Inside functions, local variables are stored here for O(1) access.
        The compiler assigns each local variable a slot number.
        """

        self.pc: int = 0
        """Program counter — index of the next instruction to execute."""

        self.halted: bool = False
        """Whether the VM has stopped execution."""

        self.output: list[str] = []
        """Captured output from PRINT-like opcodes."""

        self.call_stack: list[CallFrame] = []
        """Stack of saved execution contexts for function calls."""

        # -- Plugin registries ------------------------------------------------
        self._handlers: dict[int, OpcodeHandler] = {}
        """Opcode → handler mapping. Languages register their handlers here.

        The key is the opcode number (e.g., 0x01 for LOAD_CONST). The value
        is a callable that handles that opcode. The eval loop dispatches to
        the registered handler for each instruction.
        """

        self._builtins: dict[str, BuiltinFunction] = {}
        """Built-in function registry. Name → BuiltinFunction mapping.

        When the VM encounters a function call and the callee is not a
        user-defined function, it looks here for a built-in implementation.
        """

        # -- Configuration ----------------------------------------------------
        self._max_recursion_depth: int | None = None
        """Maximum call stack depth. None means unlimited.

        Starlark sets this to 0 (no recursion at all). A future Python plugin
        might set it to 1000 (Python's default).
        """

        self._frozen: bool = False
        """Whether mutation is currently frozen.

        When frozen, STORE_NAME/STORE_LOCAL/STORE_SUBSCRIPT/etc. raise errors.
        Used by Starlark to prevent mutation after module evaluation.
        """

        # -- Typed stack (for WASM-style typed execution) ---------------------
        self.typed_stack: list[TypedVMValue] = []
        """A secondary stack that carries type-tagged values.

        WebAssembly requires every value to have an explicit type (i32, i64,
        f32, f64). Instead of modifying the existing untyped ``stack`` (which
        other plugins use), we add a parallel typed stack. WASM instruction
        handlers use ``push_typed`` / ``pop_typed`` / ``peek_typed`` to work
        with typed values while leaving the untyped stack alone.
        """

        # -- Execution context (for WASM-specific state) ----------------------
        self.execution_context: Any | None = None
        """An opaque context object passed to context-aware opcode handlers.

        When executing WASM code, this holds the WasmExecutionContext (memory,
        tables, globals, locals, label stack, etc.). Regular opcode handlers
        don't use this — only handlers registered via ``register_context_opcode``
        receive it as a fourth argument.
        """

        self._context_handlers: dict[int, Any] = {}
        """Opcode → context-aware handler mapping.

        Like ``_handlers`` but these receive a fourth argument (the execution
        context). During context execution, these take priority over regular
        handlers.
        """

        self._pre_instruction_hook: Any | None = None
        """Optional callback invoked before each instruction during step().

        Signature: (vm, instruction, code) -> Instruction | None
        If it returns an Instruction, that instruction replaces the current one.
        Used by the WASM decoder to transform variable-length bytecodes into
        fixed-format instructions.
        """

        self._post_instruction_hook: Any | None = None
        """Optional callback invoked after each instruction during step().

        Signature: (vm, instruction, code, context) -> None
        Used for instrumentation, logging, or detecting frame changes.
        """

    # =====================================================================
    # Plugin Registration
    # =====================================================================

    def register_opcode(self, opcode: int, handler: OpcodeHandler) -> None:
        """Register a handler for an opcode number.

        This is the primary plugin interface. Languages call this method
        to teach the VM how to handle their opcodes.

        Parameters
        ----------
        opcode : int
            The opcode number (e.g., 0x01 for LOAD_CONST, 0x20 for ADD).
        handler : OpcodeHandler
            A callable ``(vm, instr, code) -> str | None`` that executes
            this opcode.

        Example::

            def handle_add(vm, instr, code):
                b = vm.pop()
                a = vm.pop()
                vm.push(a + b)
                vm.advance_pc()

            vm.register_opcode(0x20, handle_add)
        """
        self._handlers[opcode] = handler

    def register_builtin(self, name: str, implementation: Any) -> None:
        """Register a built-in function by name.

        Built-in functions are called from bytecode just like user-defined
        functions, but they're implemented in Python (the host language)
        rather than in bytecode.

        Parameters
        ----------
        name : str
            The function name as it appears in source code (e.g., "len", "print").
        implementation : callable
            A function that takes a list of arguments and returns a value.
            Can raise VMError subclasses for invalid arguments.

        Example::

            def builtin_len(args):
                if len(args) != 1:
                    raise VMTypeError("len() takes exactly 1 argument")
                return len(args[0])

            vm.register_builtin("len", builtin_len)
        """
        self._builtins[name] = BuiltinFunction(name=name, implementation=implementation)

    def get_builtin(self, name: str) -> BuiltinFunction | None:
        """Look up a built-in function by name. Returns None if not found."""
        return self._builtins.get(name)

    def register_context_opcode(self, opcode: int, handler: Any) -> None:
        """Register a context-aware handler for an opcode.

        Context-aware handlers receive four arguments instead of three:
        ``(vm, instruction, code, context)``. The fourth argument is whatever
        ``execution_context`` was set when ``execute_with_context`` was called.

        During context execution, these handlers take priority over regular
        handlers registered via ``register_opcode()``.

        Parameters
        ----------
        opcode : int
            The opcode number.
        handler : callable
            A callable ``(vm, instr, code, ctx) -> str | None``.

        Example::

            def handle_local_get(vm, instr, code, ctx):
                index = instr.operand
                vm.push_typed(ctx.typed_locals[index])
                vm.advance_pc()

            vm.register_context_opcode(0x20, handle_local_get)
        """
        self._context_handlers[opcode] = handler

    def set_pre_instruction_hook(self, hook: Any) -> None:
        """Set a callback invoked before each instruction dispatch.

        The hook receives ``(vm, instruction, code)`` and may return a
        modified Instruction to replace the current one (or None to keep it).
        This is used by the WASM decoder to transform variable-length
        bytecodes on the fly.
        """
        self._pre_instruction_hook = hook

    def set_post_instruction_hook(self, hook: Any) -> None:
        """Set a callback invoked after each instruction dispatch.

        The hook receives ``(vm, instruction, code)`` and returns nothing.
        Used for instrumentation or detecting frame changes.
        """
        self._post_instruction_hook = hook

    # =====================================================================
    # Configuration
    # =====================================================================

    def inject_globals(self, globals_dict: dict[str, Any]) -> None:
        """Pre-seed named variables into the VM's global scope.

        These variables are available to the program as regular variables
        but are set before execution begins. Useful for build context,
        environment info, etc.

        Injected globals are merged into ``variables`` — they don't
        replace the map. If a key already exists, the injected value
        overwrites it.

        Parameters
        ----------
        globals_dict : dict[str, Any]
            A mapping of variable names to values.

        Example::

            vm.inject_globals({"_ctx": {"os": "darwin", "arch": "arm64"}})
        """
        for key, value in globals_dict.items():
            self.variables[key] = value

    def set_max_recursion_depth(self, depth: int | None) -> None:
        """Set the maximum call stack depth.

        Parameters
        ----------
        depth : int or None
            Maximum depth. ``0`` means no function calls allowed (Starlark's
            restriction on recursion). ``None`` means unlimited.
        """
        self._max_recursion_depth = depth

    def set_frozen(self, frozen: bool) -> None:
        """Set whether the VM is in frozen mode.

        When frozen, any attempt to mutate state (store variables, modify
        collections) raises a VMError. This implements Starlark's freeze
        semantics — after module evaluation, all values become immutable.
        """
        self._frozen = frozen

    @property
    def is_frozen(self) -> bool:
        """Whether the VM is currently in frozen mode."""
        return self._frozen

    @property
    def max_recursion_depth(self) -> int | None:
        """The configured maximum recursion depth."""
        return self._max_recursion_depth

    # =====================================================================
    # Stack Operations — universal helpers for opcode handlers
    # =====================================================================

    def push(self, value: Any) -> None:
        """Push a value onto the operand stack.

        This is the most fundamental VM operation. Every computation starts
        by pushing values and ends by popping results.

        Parameters
        ----------
        value : Any
            The value to push. Can be any runtime value (int, str, list, etc.).
        """
        self.stack.append(value)

    def pop(self) -> Any:
        """Pop and return the top value from the operand stack.

        Raises
        ------
        StackUnderflowError
            If the stack is empty. This usually indicates a compiler bug
            (it emitted instructions that expect more values than are present).
        """
        if len(self.stack) == 0:
            raise StackUnderflowError(
                "Cannot pop from an empty stack. "
                "This usually means the compiler emitted incorrect bytecode."
            )
        return self.stack.pop()

    def peek(self) -> Any:
        """Return the top value without removing it.

        Raises
        ------
        StackUnderflowError
            If the stack is empty.
        """
        if len(self.stack) == 0:
            raise StackUnderflowError(
                "Cannot peek at an empty stack."
            )
        return self.stack[-1]

    # =====================================================================
    # Typed Stack Operations — for type-safe execution (WASM)
    # =====================================================================

    def push_typed(self, value: TypedVMValue) -> None:
        """Push a typed value onto the typed stack.

        Used by WASM instruction handlers for type-safe stack operations.
        The typed stack is separate from the untyped ``stack`` so that
        existing plugins (Starlark, etc.) are unaffected.

        Parameters
        ----------
        value : TypedVMValue
            A value with a type tag and payload.
        """
        self.typed_stack.append(value)

    def pop_typed(self) -> TypedVMValue:
        """Pop and return the top typed value.

        Raises
        ------
        StackUnderflowError
            If the typed stack is empty.
        """
        if len(self.typed_stack) == 0:
            raise StackUnderflowError(
                "Cannot pop from an empty typed stack."
            )
        return self.typed_stack.pop()

    def peek_typed(self) -> TypedVMValue:
        """Return the top typed value without removing it.

        Raises
        ------
        StackUnderflowError
            If the typed stack is empty.
        """
        if len(self.typed_stack) == 0:
            raise StackUnderflowError(
                "Cannot peek at an empty typed stack."
            )
        return self.typed_stack[-1]

    # =====================================================================
    # Call Stack Operations
    # =====================================================================

    def push_frame(self, frame: CallFrame) -> None:
        """Push a call frame onto the call stack.

        This is called when entering a function. The frame saves the caller's
        execution context so it can be restored when the function returns.

        Raises
        ------
        MaxRecursionError
            If the call stack depth exceeds the configured maximum.
        """
        if self._max_recursion_depth is not None:
            if len(self.call_stack) >= self._max_recursion_depth:
                raise MaxRecursionError(
                    f"Maximum recursion depth exceeded "
                    f"(limit: {self._max_recursion_depth})"
                )
        self.call_stack.append(frame)

    def pop_frame(self) -> CallFrame:
        """Pop and return the top call frame.

        This is called when returning from a function. The frame contains
        the saved execution context to restore.

        Raises
        ------
        VMError
            If the call stack is empty (return without matching call).
        """
        if len(self.call_stack) == 0:
            raise VMError("Cannot return — call stack is empty")
        return self.call_stack.pop()

    # =====================================================================
    # Program Counter Operations
    # =====================================================================

    def advance_pc(self) -> None:
        """Advance the program counter by one instruction.

        Most opcode handlers call this at the end of their execution.
        Jump handlers set ``self.pc`` directly instead.
        """
        self.pc += 1

    def jump_to(self, target: int) -> None:
        """Set the program counter to a specific instruction index.

        Used by jump/branch handlers to alter control flow.

        Parameters
        ----------
        target : int
            The instruction index to jump to.
        """
        self.pc = target

    # =====================================================================
    # Execution Engine
    # =====================================================================

    def execute(self, code: CodeObject) -> list[VMTrace]:
        """Execute a CodeObject using the registered opcode handlers.

        This is the main entry point. It runs the universal fetch-decode-execute
        loop, dispatching each instruction to the registered handler.

        Parameters
        ----------
        code : CodeObject
            The compiled bytecode to execute.

        Returns
        -------
        list[VMTrace]
            A trace entry for every instruction executed.

        Raises
        ------
        InvalidOpcodeError
            If an instruction's opcode has no registered handler.
        VMError
            If a runtime error occurs during execution.
        """
        traces: list[VMTrace] = []

        while not self.halted and self.pc < len(code.instructions):
            trace = self.step(code)
            traces.append(trace)

        return traces

    def step(self, code: CodeObject) -> VMTrace:
        """Execute one instruction and return a trace.

        This is the single-step entry point for debuggers and visualizers.
        If a pre-instruction hook is set, it is called before dispatch.
        If a post-instruction hook is set, it is called after dispatch.
        During context execution, context handlers take priority.

        Parameters
        ----------
        code : CodeObject
            The CodeObject being executed.

        Returns
        -------
        VMTrace
            A snapshot of the VM state before and after this instruction.
        """
        # -- Fetch --
        instruction = code.instructions[self.pc]
        pc_before = self.pc
        stack_before = list(self.stack)

        # -- Pre-instruction hook (e.g., WASM bytecode decoder) --
        if self._pre_instruction_hook is not None:
            replacement = self._pre_instruction_hook(self, instruction, code)
            if replacement is not None:
                instruction = replacement

        # -- Decode & Execute --
        # During context execution, prefer context handlers.
        handler = None
        if self.execution_context is not None:
            ctx_handler = self._context_handlers.get(instruction.opcode)
            if ctx_handler is not None:
                output_value = ctx_handler(
                    self, instruction, code, self.execution_context
                )
                handler = ctx_handler  # mark as found
            else:
                # Fall back to regular handler.
                handler = self._handlers.get(instruction.opcode)
                if handler is None:
                    raise InvalidOpcodeError(
                        f"Unknown opcode: {instruction.opcode:#04x}. "
                        f"No handler registered."
                    )
                output_value = handler(self, instruction, code)
        else:
            handler = self._handlers.get(instruction.opcode)
            if handler is None:
                raise InvalidOpcodeError(
                    f"Unknown opcode: {instruction.opcode:#04x}. "
                    f"No handler registered. Did you forget to register "
                    f"this opcode with vm.register_opcode()?"
                )
            output_value = handler(self, instruction, code)

        # -- Post-instruction hook --
        if self._post_instruction_hook is not None:
            self._post_instruction_hook(self, instruction, code)

        # -- Build trace --
        trace = VMTrace(
            pc=pc_before,
            instruction=instruction,
            stack_before=stack_before,
            stack_after=list(self.stack),
            variables=dict(self.variables),
            output=output_value,
            description=self._describe_step(instruction, code, stack_before),
        )

        return trace

    def execute_with_context(self, code: CodeObject, context: Any) -> list[VMTrace]:
        """Execute a CodeObject with an execution context.

        This is the WASM-specific entry point. It sets the execution context
        before running the eval loop, then clears it afterward. Context-aware
        handlers (registered via ``register_context_opcode``) will receive
        the context as their fourth argument.

        Parameters
        ----------
        code : CodeObject
            The compiled bytecode to execute.
        context : Any
            The execution context (e.g., WasmExecutionContext).

        Returns
        -------
        list[VMTrace]
            A trace entry for every instruction executed.
        """
        self.execution_context = context
        try:
            return self.execute(code)
        finally:
            self.execution_context = None

    def reset(self) -> None:
        """Reset the VM to its initial state, preserving registered handlers.

        This clears all execution state (stack, variables, PC, etc.) but
        keeps the registered opcode handlers, built-in functions, context
        handlers, and hooks. Use this between executions when you want to
        reuse the same VM configuration.
        """
        self.stack = []
        self.variables = {}
        self.locals = []
        self.pc = 0
        self.halted = False
        self.output = []
        self.call_stack = []
        self._frozen = False
        self.typed_stack = []
        self.execution_context = None

    # =====================================================================
    # Internals
    # =====================================================================

    def _describe_step(
        self,
        instruction: Instruction,
        code: CodeObject,
        stack_before: list[Any],
    ) -> str:
        """Generate a human-readable description of what an instruction did.

        This is used for tracing and debugging output. The description
        is a plain English explanation of the operation.
        """
        op = instruction.opcode
        operand = instruction.operand

        # Try to provide a meaningful description based on common opcodes
        # For unknown opcodes, fall back to a generic description
        op_name = f"0x{op:02x}"

        if operand is not None:
            return f"Execute {op_name} with operand {operand}"
        return f"Execute {op_name}"
