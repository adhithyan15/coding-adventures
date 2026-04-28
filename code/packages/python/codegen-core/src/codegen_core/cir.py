"""CIRInstr — the CompilerIR subset shared by all JIT/AOT specialisation passes.

``CIRInstr`` is the common typed instruction that flows through every
compilation pipeline that starts from an interpreted-language
``IIRModule``:

- **JIT path**: ``specialise(IIRFunction)`` → ``list[CIRInstr]``
  → ``jit_core.optimizer.run()`` → ``backend.compile()``
- **AOT path**: ``aot_specialise(IIRFunction, inferred)`` → ``list[CIRInstr]``
  → ``aot_core.optimizer.run()`` → ``backend.compile()``

Placing ``CIRInstr`` here — in ``codegen-core`` — rather than in
``jit-core`` removes the backwards dependency where ``aot-core``
imported a JIT-specific module just for a data type.  Both ``jit-core``
and ``aot-core`` now import from ``codegen_core``; ``jit_core.cir``
re-exports this class for backwards compatibility.

Instruction format
------------------
``op``
    Typed mnemonic, e.g. ``"add_u8"``, ``"cmp_lt_u8"``, ``"const_u8"``.
    Control-flow ops retain their original names (``"jmp"``, ``"label"``).
    Generic fallbacks use ``"call_runtime"`` with the runtime name as ``srcs[0]``.

``dest``
    Name of the destination virtual variable, or ``None`` for void ops.

``srcs``
    Operands: variable names (``str``) or literals (``int``, ``float``, ``bool``).
    For ``call_runtime`` the first element is the runtime function name.

``type``
    Concrete IIR type string.  Backends can assume this is never ``"any"``
    for arithmetic / comparison ops; it may be ``"any"`` for ``call_runtime``.

``deopt_to``
    Instruction index in the *source* ``IIRFunction`` where execution should
    resume if a type guard fails.  ``None`` for non-guard instructions.

Examples
--------
>>> instr = CIRInstr(op="add_u8", dest="v0", srcs=["a", "b"], type="u8")
>>> str(instr)
'v0 = add_u8 a, b  [u8]'

>>> guard = CIRInstr(  # doctest: +SKIP
...     op="type_assert", dest=None, srcs=["x", "u8"], type="void", deopt_to=3
... )
>>> guard.is_type_guard()
True
"""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass
class CIRInstr:
    """A single instruction in the codegen-core CompilerIR subset.

    Produced by ``jit_core.specialise()`` or ``aot_core.aot_specialise()``
    and consumed by any ``Backend[list[CIRInstr]]``.
    """

    op: str
    dest: str | None
    srcs: list[str | int | float | bool] = field(default_factory=list)
    type: str = "any"
    deopt_to: int | None = None

    def __str__(self) -> str:
        srcs_str = ", ".join(str(s) for s in self.srcs)
        lhs = f"{self.dest} = " if self.dest else ""
        deopt = f"  [deopt→{self.deopt_to}]" if self.deopt_to is not None else ""
        return f"{lhs}{self.op} {srcs_str}  [{self.type}]{deopt}"

    def is_type_guard(self) -> bool:
        """Return True if this instruction is a type guard (``type_assert``).

        Type guards are emitted by the specialisation pass when the source
        instruction was dynamically typed (``type_hint == "any"``).  The
        backend is responsible for implementing the guard check; if the
        runtime type does not match, execution falls back to the interpreter
        at the IIR index stored in ``deopt_to``.
        """
        return self.op == "type_assert" and self.deopt_to is not None

    def is_generic(self) -> bool:
        """Return True if this instruction is a generic runtime call.

        Generic runtime calls (``call_runtime``) are used for operations
        that the specialisation pass could not type — typically because the
        profiler saw too few observations or all observations were ``"any"``.
        A backend may choose to handle these via a slow-path interpreter
        call rather than emitting native code.
        """
        return self.op == "call_runtime"
