"""CIRInstr — the CompilerIR subset emitted by jit-core's specialization pass.

jit-core emits a flat list of typed instructions (no SSA phi-nodes, no loop
induction variables — those are for the compiled-language path).  Every
``CIRInstr`` has a concrete ``type``; the string ``"any"`` appears only for
instructions whose type could not be determined (generic runtime calls).

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

Literal representation
----------------------
>>> instr = CIRInstr(op="add_u8", dest="v0", srcs=["a", "b"], type="u8")
>>> str(instr)
'v0 = add_u8 a, b  [u8]'
"""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass
class CIRInstr:
    """A single instruction in the jit-core CompilerIR subset.

    Produced by ``specialise()`` and consumed by backend ``compile()``.
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
        """Return True if this instruction is a type guard (``type_assert``)."""
        return self.op == "type_assert" and self.deopt_to is not None

    def is_generic(self) -> bool:
        """Return True if this instruction is a generic runtime call."""
        return self.op == "call_runtime"
