"""IIRModule — the top-level container for an InterpreterIR program.

A module holds all functions compiled from a single source file (or REPL
session).  It is the unit handed to ``vm-core.execute()`` and
``jit-core.execute_with_jit()``.

REPL sessions mutate the module incrementally: each new input appends new
``IIRFunction`` objects or replaces existing ones by name.

Example::

    module = IIRModule(
        name="hello.bas",
        language="basic",
        functions=[...],
        entry_point="main",
    )
    result = vm.execute(module)
"""

from __future__ import annotations

from dataclasses import dataclass, field

from interpreter_ir.function import IIRFunction


@dataclass
class IIRModule:
    """Top-level container for an InterpreterIR program.

    Parameters
    ----------
    name:
        A human-readable name, typically the source file path.
    functions:
        All functions in the program, in definition order.
    entry_point:
        Name of the function to call when the module is executed.
        ``None`` means no automatic entry point (useful for libraries).
    language:
        Source language identifier.  Used by tooling for display only;
        not interpreted by ``vm-core``.
    """

    name: str
    functions: list[IIRFunction] = field(default_factory=list)
    entry_point: str | None = "main"
    language: str = "unknown"

    # -----------------------------------------------------------------------
    # Lookup
    # -----------------------------------------------------------------------

    def get_function(self, fn_name: str) -> IIRFunction | None:
        """Return the ``IIRFunction`` with the given name, or ``None``."""
        for fn in self.functions:
            if fn.name == fn_name:
                return fn
        return None

    def function_names(self) -> list[str]:
        """Return all function names in definition order."""
        return [fn.name for fn in self.functions]

    # -----------------------------------------------------------------------
    # Mutation (used by REPL incremental compilation)
    # -----------------------------------------------------------------------

    def add_or_replace(self, fn: IIRFunction) -> None:
        """Append ``fn`` or replace an existing function with the same name.

        Called by the REPL integration (LANG08) when the user redefines a
        function in a later input.
        """
        for i, existing in enumerate(self.functions):
            if existing.name == fn.name:
                self.functions[i] = fn
                return
        self.functions.append(fn)

    # -----------------------------------------------------------------------
    # Validation
    # -----------------------------------------------------------------------

    def validate(self) -> list[str]:
        """Return a list of validation error strings (empty = valid).

        Checks:
        - No duplicate function names
        - Entry point function exists (if entry_point is set)
        - No instruction references an undefined label within its function
        """
        errors: list[str] = []
        seen: set[str] = set()
        for fn in self.functions:
            if fn.name in seen:
                errors.append(f"duplicate function name: {fn.name!r}")
            seen.add(fn.name)

        if self.entry_point is not None and self.entry_point not in seen:
            errors.append(
                f"entry_point {self.entry_point!r} not found in module functions"
            )

        for fn in self.functions:
            defined_labels = {
                instr.srcs[0]
                for instr in fn.instructions
                if instr.op == "label" and instr.srcs
            }
            for instr in fn.instructions:
                if instr.op in {"jmp", "jmp_if_true", "jmp_if_false"}:
                    label = instr.srcs[-1] if instr.srcs else None
                    if isinstance(label, str) and label not in defined_labels:
                        errors.append(
                            f"function {fn.name!r}: branch to undefined "
                            f"label {label!r}"
                        )
        return errors

    def __repr__(self) -> str:
        return (
            f"IIRModule({self.name!r}, "
            f"language={self.language!r}, "
            f"functions={self.function_names()}, "
            f"entry={self.entry_point!r})"
        )
