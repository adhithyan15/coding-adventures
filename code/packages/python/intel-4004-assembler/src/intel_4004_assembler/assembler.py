"""assembler.py -- Two-pass Intel 4004 assembler.

The Classic Two-Pass Assembly Algorithm
-----------------------------------------

Assembling machine code from symbolic text is one of the oldest problems
in computing.  The challenge is *forward references*: an instruction like
``JUN loop_end`` appears *before* the label ``loop_end:`` is defined.  We
don't know ``loop_end``'s address when we first encounter the jump.

The solution -- invented in the 1950s -- is to make **two passes** over the
source:

Pass 1 -- Symbol Collection
~~~~~~~~~~~~~~~~~~~~~~~~~~~

Walk every line.  Keep a *program counter* (PC) that tracks the byte
address of the current instruction.  When we see a label definition
``my_label:``, record ``symbols["my_label"] = pc``.  When we see an
instruction, advance PC by the instruction's byte size (1 or 2).

After Pass 1, ``symbols`` maps every label to its final address.

Pass 2 -- Code Emission
~~~~~~~~~~~~~~~~~~~~~~~

Walk every line again.  For each instruction, call ``encode_instruction``
with the (now-complete) symbol table.  Any ``JUN loop_end`` can now be
resolved because ``symbols["loop_end"]`` is known.  Append the encoded
bytes to the output buffer.

Pass 1 and Pass 2 handle *exactly the same line sequence* -- same loop,
same line objects.  The only difference is what we *do* on each line:
collect vs. emit.

Addressing Notes
----------------

The Intel 4004 has a 12-bit address space (4 KB of ROM).  Addresses are
written as 3 hex digits: ``0x000`` to ``0xFFF``.

The ``ORG addr`` directive sets the program counter to ``addr``.  This
is used at the top of programs to tell the assembler where the code will
be loaded in memory.  In a real ROM, the code starts at address 0.

Self-loop / HLT
~~~~~~~~~~~~~~~

The 4004 has no HALT instruction (the simulator adds one as 0x01, but
on real hardware you'd halt by looping forever).  The assembly text
``JUN $`` means "jump to the current instruction" -- a one-instruction
infinite loop.  ``$`` resolves to the PC of the JUN itself.

Example
-------

::

    from intel_4004_assembler.assembler import Intel4004Assembler

    asm = Intel4004Assembler()
    program = '''
        ORG 0x000
    _start:
        LDM 5
        XCH R2
        HLT
    '''
    binary = asm.assemble(program)
    # -> bytes([0xD5, 0xB2, 0x01])
    print(binary.hex())  # "d5b201"

Public API
----------

- ``Intel4004Assembler``        -- main class; call ``.assemble(text) -> bytes``
- ``assemble(text) -> bytes``    -- module-level convenience function
- ``AssemblerError``            -- re-exported from ``encoder``
"""

from __future__ import annotations

from intel_4004_assembler.encoder import (
    AssemblerError,
    encode_instruction,
    instruction_size,
)
from intel_4004_assembler.lexer import ParsedLine, lex_program


class Intel4004Assembler:
    """Two-pass Intel 4004 assembler.

    Usage::

        assembler = Intel4004Assembler()
        binary = assembler.assemble(source_text)

    The assembler is stateless between calls -- each call to
    ``assemble()`` gets a fresh symbol table and program counter.
    It is safe to reuse the same instance for multiple programs.
    """

    def assemble(self, text: str) -> bytes:
        """Assemble Intel 4004 assembly source text into binary bytes.

        The method runs two passes:

        **Pass 1** -- builds the symbol table.
        **Pass 2** -- encodes instructions using the completed symbol table.

        Args:
            text: Multi-line Intel 4004 assembly source, as produced by
                  the ``ir-to-intel-4004-compiler`` code generator.

        Returns:
            A ``bytes`` object containing the raw machine code.

        Raises:
            AssemblerError: On unknown mnemonics, undefined labels, or
                            out-of-range values.

        Example::

            binary = Intel4004Assembler().assemble('''
                ORG 0x000
            _start:
                LDM 5
                XCH R2
                HLT
            ''')
            assert binary == bytes([0xD5, 0xB2, 0x01])
        """
        lines = lex_program(text)
        symbols = self._pass1(lines)
        return self._pass2(lines, symbols)

    # ------------------------------------------------------------------
    # Pass 1 -- Symbol table collection
    # ------------------------------------------------------------------

    def _pass1(self, lines: list[ParsedLine]) -> dict[str, int]:
        """Build the symbol table by scanning labels and tracking PC.

        Rules:
        - Start with ``pc = 0``.
        - ``ORG addr`` sets ``pc = addr``.
        - A label on a line records ``{label: pc}`` *before* any instruction
          on the same line advances the PC (labels point at the instruction
          that follows them).
        - Instructions advance ``pc`` by ``instruction_size(mnemonic)``.
        - Blank / comment lines leave PC unchanged.

        Args:
            lines: Parsed lines from ``lex_program``.

        Returns:
            Dict mapping label name -> byte address.

        Raises:
            AssemblerError: If a mnemonic is unknown (size cannot be computed).
        """
        symbols: dict[str, int] = {}
        pc = 0

        for parsed in lines:
            # Record label at the current PC (before the instruction on this line).
            if parsed.label is not None:
                symbols[parsed.label] = pc

            if parsed.mnemonic is None:
                # Blank line or comment -- nothing to do.
                continue

            mnemonic = parsed.mnemonic.upper()

            if mnemonic == "ORG":
                # ORG sets the PC; it does not emit bytes.
                if not parsed.operands:
                    raise AssemblerError("ORG requires an address operand")
                org_addr = _parse_number(parsed.operands[0])
                # The Intel 4004 has a 12-bit address space (0x000..0xFFF = 4 KB).
                # Guard against absurdly large ORG values to prevent padding DoS.
                if org_addr > 0xFFF:
                    raise AssemblerError(
                        f"ORG address 0x{org_addr:X} exceeds Intel 4004 "
                        f"address space (max 0xFFF)"
                    )
                pc = org_addr
                continue

            # Advance PC by the byte size of this instruction.
            pc += instruction_size(mnemonic, parsed.operands)

        return symbols

    # ------------------------------------------------------------------
    # Pass 2 -- Code emission
    # ------------------------------------------------------------------

    def _pass2(self, lines: list[ParsedLine], symbols: dict[str, int]) -> bytes:
        """Emit bytes for each instruction using the completed symbol table.

        Args:
            lines:   Parsed lines (same list as used in Pass 1).
            symbols: Completed symbol table from Pass 1.

        Returns:
            Encoded binary as a ``bytes`` object.

        Raises:
            AssemblerError: On encoding errors (undefined label, out-of-range).
        """
        output = bytearray()
        pc = 0

        for parsed in lines:
            if parsed.mnemonic is None:
                continue

            mnemonic = parsed.mnemonic.upper()

            if mnemonic == "ORG":
                # ORG adjusts the logical PC (and may pad output with zeros
                # if ORG > current offset, but for simplicity we just track PC).
                if not parsed.operands:
                    raise AssemblerError("ORG requires an address operand")
                org_addr = _parse_number(parsed.operands[0])
                # Guard: the Intel 4004 has a 12-bit address space.
                if org_addr > 0xFFF:
                    raise AssemblerError(
                        f"ORG address 0x{org_addr:X} exceeds Intel 4004 "
                        f"address space (max 0xFFF)"
                    )
                # If ORG is advancing past where we are, pad with NOP (0x00).
                if org_addr > pc:
                    output.extend(b"\x00" * (org_addr - pc))
                pc = org_addr
                continue

            # Encode the instruction.
            encoded = encode_instruction(mnemonic, parsed.operands, symbols, pc)
            output.extend(encoded)
            pc += len(encoded)

        return bytes(output)


# ---------------------------------------------------------------------------
# Module-level convenience function
# ---------------------------------------------------------------------------

def assemble(text: str) -> bytes:
    """Assemble Intel 4004 assembly text into binary bytes.

    This is a convenience wrapper around ``Intel4004Assembler.assemble()``.
    Useful when you don't need to keep the assembler instance around.

    Args:
        text: Multi-line assembly source text.

    Returns:
        Encoded binary bytes.

    Raises:
        AssemblerError: On any assembly error.

    Example::

        from intel_4004_assembler.assembler import assemble

        binary = assemble(\"\"\"
            ORG 0x000
        _start:
            LDM 5
            HLT
        \"\"\")
        # -> bytes([0xD5, 0x01])
    """
    return Intel4004Assembler().assemble(text)


# ---------------------------------------------------------------------------
# Internal helper
# ---------------------------------------------------------------------------

def _parse_number(value: str) -> int:
    """Parse a numeric literal (decimal or ``0x``-hex) from a directive operand.

    Args:
        value: String such as ``"0x000"`` or ``"256"``.

    Returns:
        Integer value.

    Raises:
        AssemblerError: If the string is not a valid integer literal.
    """
    try:
        if value.lower().startswith("0x"):
            return int(value, 16)
        return int(value, 10)
    except ValueError:
        raise AssemblerError(f"Invalid address literal: {value!r}") from None
