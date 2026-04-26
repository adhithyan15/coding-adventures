"""The MACSYMA `%i` / `%o` history table.

Maxima labels every input as ``%iN`` and every output as ``%oN``. The
``%`` shorthand resolves to the most recent ``%o``. The REPL writes
to this table on each turn; the VM reads from it via a binding-lookup
hook so user expressions like ``%i3 + 1`` work transparently.

This is a session-local in-memory data structure. There is no
persistence.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from symbolic_ir import IRNode


@dataclass
class History:
    """Input/output history for one MACSYMA REPL session.

    Indices start at 1 (Maxima convention). After ``record_input(ir)``
    returns ``n``, ``get_input(n)`` returns the same IR. Outputs follow
    the same pattern.

    The REPL is responsible for keeping inputs and outputs aligned;
    typically every ``record_input`` is followed by exactly one
    ``record_output`` before the next ``record_input`` happens.
    """

    inputs: list[IRNode] = field(default_factory=list)
    outputs: list[IRNode] = field(default_factory=list)

    # ---- writers ---------------------------------------------------------

    def record_input(self, ir: IRNode) -> int:
        """Append ``ir`` to the input history. Returns the new index."""
        self.inputs.append(ir)
        return len(self.inputs)

    def record_output(self, ir: IRNode) -> int:
        """Append ``ir`` to the output history. Returns the new index."""
        self.outputs.append(ir)
        return len(self.outputs)

    # ---- readers ---------------------------------------------------------

    def get_input(self, n: int) -> IRNode | None:
        """Return ``%iN`` or None if out of range."""
        if 1 <= n <= len(self.inputs):
            return self.inputs[n - 1]
        return None

    def get_output(self, n: int) -> IRNode | None:
        """Return ``%oN`` or None if out of range."""
        if 1 <= n <= len(self.outputs):
            return self.outputs[n - 1]
        return None

    def last_output(self) -> IRNode | None:
        """Return the most recent ``%o`` or None if no outputs yet.

        Used to resolve the bare ``%`` shorthand.
        """
        return self.outputs[-1] if self.outputs else None

    def next_input_index(self) -> int:
        """Return the index that the next ``record_input`` will produce.

        Useful for prompts: ``(%i{history.next_input_index()}) ``.
        """
        return len(self.inputs) + 1

    # ---- mutation --------------------------------------------------------

    def reset(self) -> None:
        """Clear the entire history."""
        self.inputs.clear()
        self.outputs.clear()

    # ---- name-resolution hook --------------------------------------------

    def resolve_history_symbol(self, name: str) -> IRNode | None:
        """Resolve ``%``, ``%iN``, ``%oN`` symbol names to IR.

        Returns the corresponding IR node or None if the name is not a
        history reference (or is out of range). The MacsymaBackend
        installs this as a fallback inside its ``lookup`` chain.
        """
        if name == "%":
            return self.last_output()
        if name.startswith("%i") and name[2:].isdigit():
            return self.get_input(int(name[2:]))
        if name.startswith("%o") and name[2:].isdigit():
            return self.get_output(int(name[2:]))
        return None
