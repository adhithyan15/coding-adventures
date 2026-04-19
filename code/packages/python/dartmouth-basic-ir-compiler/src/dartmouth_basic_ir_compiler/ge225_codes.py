"""GE-225 typewriter character codes for PRINT statement compilation.

The GE-225 typewriter uses a 6-bit character code stored in the N register.
These codes are not ASCII — they are a character set specific to the GE-225's
typewriter peripheral, documented in the GE-225 programming manual.

When a BASIC PRINT statement contains a string literal, each character is
converted at compile time to its GE-225 typewriter code. The compiled IR then
loads this code into v0 (the syscall argument register) and calls SYSCALL 1,
which the GE-225 backend translates into the SAN 6 + TYP sequence.

Historical note: the 1964 BASIC teletype could only print uppercase letters,
digits, and a handful of punctuation characters. String literals like "HELLO"
worked perfectly; lowercase letters were uppercased automatically by this
compiler as a convenience.

The codes are given in octal to match the GE-225 manual notation:

  0o00 = '0'   0o01 = '1'   0o02 = '2'   0o03 = '3'   0o04 = '4'
  0o05 = '5'   0o06 = '6'   0o07 = '7'   0o10 = '8'   0o11 = '9'
  0o13 = '/'   0o21 = 'A'   0o22 = 'B'   ...          0o71 = 'Z'

Character 0o37 (decimal 31) is a carriage return, appended automatically at
the end of every PRINT statement (as the original DTSS system did).
"""

from __future__ import annotations

GE225_CODES: dict[str, int] = {
    "0": 0o00,
    "1": 0o01,
    "2": 0o02,
    "3": 0o03,
    "4": 0o04,
    "5": 0o05,
    "6": 0o06,
    "7": 0o07,
    "8": 0o10,
    "9": 0o11,
    "/": 0o13,
    "A": 0o21,
    "B": 0o22,
    "C": 0o23,
    "D": 0o24,
    "E": 0o25,
    "F": 0o26,
    "G": 0o27,
    "H": 0o30,
    "I": 0o31,
    "-": 0o33,
    ".": 0o40,
    "J": 0o41,
    "K": 0o42,
    "L": 0o43,
    "M": 0o44,
    "N": 0o45,
    "O": 0o46,
    "P": 0o47,
    "Q": 0o50,
    "R": 0o51,
    "$": 0o53,
    " ": 0o60,
    "S": 0o62,
    "T": 0o63,
    "U": 0o64,
    "V": 0o65,
    "W": 0o66,
    "X": 0o67,
    "Y": 0o70,
    "Z": 0o71,
}

CARRIAGE_RETURN_CODE: int = 0o37


def ascii_to_ge225(ch: str) -> int | None:
    """Convert a single character to its GE-225 typewriter code.

    Lowercase letters are uppercased before lookup. Characters that have no
    GE-225 equivalent return ``None``.

    Args:
        ch: A single character string.

    Returns:
        The 6-bit GE-225 typewriter code, or ``None`` if unsupported.

    Example::

        ascii_to_ge225("H")   # 0o30 = 24
        ascii_to_ge225("h")   # 0o30 = 24 (lowercase uppercased)
        ascii_to_ge225("@")   # None (no GE-225 equivalent)
    """
    return GE225_CODES.get(ch.upper())
