"""Brainfuck Translator — Source Code to Bytecode in One Pass.

==========================================================================
Why "Translator" and not "Compiler"?
==========================================================================

A compiler transforms a high-level *structured* representation (an AST)
into lower-level instructions. It handles scoping, type checking, operator
precedence, and all the complexity that comes with real languages.

Brainfuck doesn't have any of that. There's no AST, no scoping, no types.
Each source character maps directly to one instruction. The only non-trivial
step is **bracket matching** — connecting ``[`` to its matching ``]`` so the
VM knows where to jump.

So we call this a "translator" rather than a "compiler": it translates
characters to opcodes, with bracket matching as the sole transformation.

==========================================================================
How Bracket Matching Works
==========================================================================

Bracket matching is a classic stack problem:

1. Scan the source left to right.
2. When we see ``[``, emit a ``LOOP_START`` with a placeholder target (0),
   and push its instruction index onto a stack.
3. When we see ``]``, pop the matching ``[`` index from the stack.
   - Patch the ``[`` instruction to jump to one past the current ``]``.
   - Emit a ``LOOP_END`` that jumps back to the ``[``.
4. After scanning, if the stack isn't empty, we have unmatched ``[``.

This is identical to how the GenericCompiler's ``emit_jump()`` /
``patch_jump()`` work — but we do it by hand since Brainfuck is too
simple to need the full compiler framework.

==========================================================================
Example
==========================================================================

Source: ``++[>+<-]``

Translation::

    Index  Opcode       Operand   Source
    ─────────────────────────────────────
    0      INC          —         +
    1      INC          —         +
    2      LOOP_START   7         [  (jump to 7 if cell==0)
    3      RIGHT        —         >
    4      INC          —         +
    5      LEFT         —         <
    6      DEC          —         -
    7      LOOP_END     2         ]  (jump to 2 if cell!=0)
    8      HALT         —         (end)

When cell 0 reaches zero, LOOP_START at index 2 jumps to index 7+1=8
(one past the LOOP_END, which is HALT). When the cell is still nonzero,
LOOP_END at index 7 jumps back to index 2.

Wait — the table says LOOP_START's operand is 7, but shouldn't it jump
to 8 (past the ``]``)? Actually, let's look carefully: LOOP_START jumps
to the instruction *after* LOOP_END, which is index 8. So the operand
should be 8, not 7. Let me correct that in the implementation.
"""

from __future__ import annotations

from virtual_machine import CodeObject, Instruction

from brainfuck.opcodes import CHAR_TO_OP, Op


class TranslationError(Exception):
    """Raised when the Brainfuck source has mismatched brackets."""


def translate(source: str) -> CodeObject:
    """Translate Brainfuck source code into a CodeObject.

    Parameters
    ----------
    source : str
        The Brainfuck program. Non-command characters are ignored.

    Returns
    -------
    CodeObject
        A CodeObject with instructions ready for the GenericVM.
        The constant and name pools are empty (Brainfuck has no
        variables or literals).

    Raises
    ------
    TranslationError
        If brackets are mismatched.

    Examples
    --------
    >>> code = translate("+++.")
    >>> len(code.instructions)  # 3 INCs + 1 OUTPUT + 1 HALT
    5
    >>> code.constants
    []
    >>> code.names
    []
    """
    instructions: list[Instruction] = []
    bracket_stack: list[int] = []

    for char in source:
        op = CHAR_TO_OP.get(char)
        if op is None:
            # Not a Brainfuck command — skip (it's a comment)
            continue

        if op == Op.LOOP_START:
            # Emit LOOP_START with placeholder operand (will be patched)
            index = len(instructions)
            instructions.append(Instruction(Op.LOOP_START, 0))
            bracket_stack.append(index)

        elif op == Op.LOOP_END:
            if not bracket_stack:
                raise TranslationError(
                    "Unmatched ']' — no matching '[' found"
                )
            # Pop the matching [ index
            start_index = bracket_stack.pop()

            # The LOOP_END instruction index
            end_index = len(instructions)

            # Patch LOOP_START to jump past LOOP_END (end_index + 1)
            instructions[start_index] = Instruction(
                Op.LOOP_START, end_index + 1
            )

            # Emit LOOP_END that jumps back to LOOP_START
            instructions.append(Instruction(Op.LOOP_END, start_index))

        else:
            # Simple command — no operand needed
            instructions.append(Instruction(op, None))

    if bracket_stack:
        raise TranslationError(
            f"Unmatched '[' — {len(bracket_stack)} unclosed bracket(s)"
        )

    # Append HALT
    instructions.append(Instruction(Op.HALT, None))

    return CodeObject(
        instructions=instructions,
        constants=[],
        names=[],
    )
