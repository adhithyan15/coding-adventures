"""assembler.py -- Two-pass Intel 8008 assembler.

The Classic Two-Pass Assembly Algorithm
-----------------------------------------

Assembling machine code from symbolic text is one of the oldest problems
in computing.  The challenge is *forward references*: an instruction like
``JTZ loop_end`` appears *before* the label ``loop_end:`` is defined.  We
don't know ``loop_end``'s address when we first encounter the jump.

The solution -- invented in the 1950s -- is to make **two passes** over the
source:

Pass 1 -- Symbol Collection
~~~~~~~~~~~~~~~~~~~~~~~~~~~

Walk every line.  Keep a *program counter* (PC) that tracks the byte
address of the current instruction.  When we see a label definition
``my_label:``, record ``symbols["my_label"] = pc``.  When we see an
instruction, advance PC by the instruction's byte size (1, 2, or 3 bytes).

After Pass 1, ``symbols`` maps every label to its final address.

Pass 2 -- Code Emission
~~~~~~~~~~~~~~~~~~~~~~~

Walk every line again.  For each instruction, call ``encode_instruction``
with the (now-complete) symbol table.  Any ``JTZ loop_end`` can now be
resolved because ``symbols["loop_end"]`` is known.  Append the encoded
bytes to the output buffer.

Pass 1 and Pass 2 handle *exactly the same line sequence* -- same loop,
same line objects.  The only difference is what we *do* on each line:
collect vs. emit.

Addressing Notes
----------------

The Intel 8008 has a 14-bit address space (16 KB = 0x0000–0x3FFF).
The ROM region for code is 0x0000–0x1FFF (8 KB).  The RAM region for
static variables starts at 0x2000.  Jump/call addresses are encoded as
full 14-bit values -- there is no page-relative addressing constraint
(unlike the 4004's JCN which is page-relative).

The ``ORG addr`` directive sets the program counter to ``addr``.  Programs
typically begin with ``ORG 0x0000``.

hi()/lo() Directives
~~~~~~~~~~~~~~~~~~~~

The code generator emits ``MVI H, hi(symbol)`` and ``MVI L, lo(symbol)``
to load the address of a static variable into H:L.  The assembler resolves
these during Pass 2:

  hi(sym) = (sym_address >> 8) & 0x3F  (high 6 bits of 14-bit address)
  lo(sym) = sym_address & 0xFF          (low 8 bits)

This lets the code generator emit symbolic names for static variable
addresses, which are resolved to concrete numbers by the assembler.

Example
-------

::

    from intel_8008_assembler.assembler import Intel8008Assembler

    asm = Intel8008Assembler()
    program = '''
        ORG 0x0000
    _start:
        MVI  B, 0
        CAL  _fn_main
        HLT
    _fn_main:
        MVI  D, 42
        MOV  A, D
        RFC
    '''
    binary = asm.assemble(program)
    print(binary.hex())

Public API
----------

- ``Intel8008Assembler``         -- main class; call ``.assemble(text) -> bytes``
- ``assemble(text) -> bytes``     -- module-level convenience function
- ``AssemblerError``             -- re-exported from ``encoder``
"""

from __future__ import annotations

from intel_8008_assembler.encoder import (
    AssemblerError,
    encode_instruction,
    instruction_size,
)
from intel_8008_assembler.lexer import ParsedLine, lex_program

# The Intel 8008 has a 14-bit address space covering 16 KB (0x0000–0x3FFF).
_MAX_ADDRESS = 0x3FFF


class Intel8008Assembler:
    """Two-pass Intel 8008 assembler.

    Usage::

        assembler = Intel8008Assembler()
        binary = assembler.assemble(source_text)

    The assembler is stateless between calls -- each call to
    ``assemble()`` gets a fresh symbol table and program counter.
    It is safe to reuse the same instance for multiple programs.
    """

    def assemble(self, text: str) -> bytes:
        """Assemble Intel 8008 assembly source text into binary bytes.

        The method runs two passes:

        **Pass 1** -- builds the symbol table.
        **Pass 2** -- encodes instructions using the completed symbol table.

        Args:
            text: Multi-line Intel 8008 assembly source, as produced by
                  the ``ir-to-intel-8008-compiler`` code generator.

        Returns:
            A ``bytes`` object containing the raw machine code.

        Raises:
            AssemblerError: On unknown mnemonics, undefined labels, or
                            out-of-range values.

        Example::

            binary = Intel8008Assembler().assemble('''
                ORG 0x0000
            _start:
                MVI  B, 0
                CAL  _fn_main
                HLT
            _fn_main:
                MVI  D, 42
                MOV  A, D
                RFC
            ''')
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
            AssemblerError: If a mnemonic is unknown or ORG is out of range.
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
                if org_addr > _MAX_ADDRESS:
                    raise AssemblerError(
                        f"ORG address 0x{org_addr:X} exceeds Intel 8008 "
                        f"address space (max 0x{_MAX_ADDRESS:X})"
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
                if not parsed.operands:
                    raise AssemblerError("ORG requires an address operand")
                org_addr = _parse_number(parsed.operands[0])
                if org_addr > _MAX_ADDRESS:
                    raise AssemblerError(
                        f"ORG address 0x{org_addr:X} exceeds Intel 8008 "
                        f"address space (max 0x{_MAX_ADDRESS:X})"
                    )
                # If ORG is advancing past where we are, pad with 0xFF
                # (erased flash / ROM state).
                if org_addr > pc:
                    output.extend(b"\xff" * (org_addr - pc))
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
    """Assemble Intel 8008 assembly text into binary bytes.

    This is a convenience wrapper around ``Intel8008Assembler.assemble()``.
    Useful when you don't need to keep the assembler instance around.

    Args:
        text: Multi-line assembly source text.

    Returns:
        Encoded binary bytes.

    Raises:
        AssemblerError: On any assembly error.

    Example::

        from intel_8008_assembler.assembler import assemble

        binary = assemble(\"\"\"
            ORG 0x0000
        _start:
            HLT
        \"\"\")
        # -> bytes([0xFF])
    """
    return Intel8008Assembler().assemble(text)


# ---------------------------------------------------------------------------
# Internal helper
# ---------------------------------------------------------------------------

def _parse_number(value: str) -> int:
    """Parse a numeric literal (decimal or ``0x``-hex) from a directive operand.

    Args:
        value: String such as ``"0x0000"`` or ``"256"``.

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
