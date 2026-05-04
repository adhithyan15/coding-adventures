"""Encode a structured ``BEAMModule`` into a ``.beam`` container.

This module is the inverse of ``beam_bytes_decoder.parse_beam_container``
(plus the chunk-specific decoders).  It is *purely* a file-format
writer — it knows nothing about Twig, ``compiler-ir``, or what an
executable Erlang program needs.  The caller (``ir-to-beam`` in
BEAM01 Phase 3) is responsible for handing us a coherent
``BEAMModule``.

File-format reference
=====================

A ``.beam`` file is an IFF container::

    "FOR1"        — 4-byte magic
    <u32 BE>      — total file length minus 8
    "BEAM"        — 4-byte form type
    <chunks...>   — each chunk:
                      4-byte ASCII tag (e.g. "AtU8", "Code")
                      <u32 BE> chunk byte length (excluding header)
                      <chunk payload>
                      0..3 bytes of zero padding to a 4-byte boundary

Required chunks for a loadable module: ``AtU8``, ``Code``,
``ImpT``, ``ExpT``.  ``LocT`` is optional; we always emit it
(possibly empty) for symmetry with the decoder which will read
it when present.

Compact-term operand encoding (ECMA-style BEAM reference)
=========================================================

Every ``Code`` chunk operand is encoded with a 3-bit type tag in
the low bits of the first byte plus length-prefixed value bits
in the high bits:

    Tag (3 bits) | Type
    -------------|------------------------------------------------
    0b000        | u (small unsigned int / atom-table index / ...)
    0b001        | i (signed integer literal)
    0b010        | a (atom-table index — index 0 is the 'nil' atom)
    0b011        | x (x register — function argument / scratch)
    0b100        | y (y register — stack-allocated local)
    0b101        | f (label / function reference)
    0b110        | h (character — only used by old VM)
    0b111        | z (extended — list, fpreg, alloc-list, lit-table)

Length encoding (after the 3-bit tag):

- Bit 3 = 0: value is the high 4 bits of the byte (range 0..15).
- Bit 3 = 1, bit 4 = 0: value is high 3 bits + the next byte
  (range 0..2047).
- Bit 3 = 1, bit 4 = 1: value is encoded as a multi-byte
  big-endian integer; the next 3 bits give the length minus 2.

This module exposes ``encode_compact_term(tag, value)`` directly
so unit tests can exercise the bit-wrangling without going through
a full module encode.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass, field
from enum import IntEnum
from typing import Final


class BEAMEncodeError(ValueError):
    """Raised when a ``BEAMModule`` cannot be encoded as ``.beam``."""


class BEAMTag(IntEnum):
    """The 3-bit operand type tags used by BEAM's compact-term encoding.

    Values match the bit pattern in the low 3 bits of the first
    byte of a compact-term-encoded operand.
    """

    U = 0  # unsigned literal (small integer, index into something)
    I = 1  # signed integer literal  # noqa: E741
    A = 2  # atom-table index (1-based; index 0 is the nil atom)
    X = 3  # x register (argument / scratch register)
    Y = 4  # y register (stack-allocated local variable)
    F = 5  # label / function reference (1-based)
    H = 6  # character (legacy; we never emit this)
    Z = 7  # extended encoding (list, fpreg, alloc-list, lit-table)


@dataclass(frozen=True)
class BEAMOperand:
    """One operand of a BEAM instruction.

    ``tag`` selects how ``value`` is encoded; ``value`` is always a
    non-negative integer at this layer (signed integers under
    ``BEAMTag.I`` are still represented positively here — the
    encoder does the sign-bit dance for the caller).
    """

    tag: BEAMTag
    value: int


@dataclass(frozen=True)
class BEAMInstruction:
    """One BEAM instruction: opcode byte + zero or more operands."""

    opcode: int
    operands: tuple[BEAMOperand, ...] = ()


@dataclass(frozen=True)
class BEAMImport:
    """One row of the ``ImpT`` (import) table.

    All three indices are 1-based atom-table indices (or arity, in
    the case of ``arity``).
    """

    module_atom_index: int
    function_atom_index: int
    arity: int


@dataclass(frozen=True)
class BEAMExport:
    """One row of the ``ExpT`` (export) or ``LocT`` (local) table.

    ``function_atom_index`` is 1-based; ``label`` is the BEAM label
    number (also 1-based).
    """

    function_atom_index: int
    arity: int
    label: int


@dataclass(frozen=True)
class BEAMFun:
    """One row of the ``FunT`` (fun) table — describes a closure /
    lambda lifted from the source program.

    BEAM has first-class fun objects; the ``FunT`` chunk plus the
    ``make_fun2`` opcode together let us construct closure values
    that capture free variables.  See
    ``code/specs/BEAM02-closure-lowering.md`` for the lowering plan.

    Fields:

    * ``function_atom_index`` — 1-based atom-table index of the
      lifted lambda's name (e.g. ``_lambda_0`` interned as an atom).
    * ``arity`` — the lambda's parameter count, NOT counting
      captured free variables.
    * ``code_label`` — BEAM label number where the lambda body
      starts.  Must match the label used by the ``make_fun2``
      opcode at the construction site.
    * ``index`` — fun-table entry index, sequential from 0.  Real
      ``erlc`` mirrors this in the chunk's ``index`` column.
    * ``num_free`` — count of captured free variables.  Determines
      how many x-registers ``make_fun2`` reads at the construction
      site.
    * ``old_uniq`` — a 32-bit hash used by Erlang's loader to
      version funs across module reloads.  Real ``erlc`` computes
      a CRC32 of the function body; for first-emit modules a
      stable derived value (e.g. CRC32 of the lambda's IR label
      name) is acceptable and what we use.
    """

    function_atom_index: int
    arity: int
    code_label: int
    index: int
    num_free: int
    old_uniq: int


@dataclass(frozen=True)
class BEAMModule:
    """Structural description of a BEAM module ready for encoding.

    Field semantics:

    * ``name`` — module name as a Python ``str``.  This MUST equal
      ``atoms[0]`` (atoms are 1-based; the spec mandates the module
      name at atom index 1).  ``encode_beam`` validates this.
    * ``atoms`` — every atom referenced by the module, in
      declaration order.  ``atoms[0]`` is the module name; further
      entries are exported function names, imported atoms, and
      literal atoms used by instructions.
    * ``instructions`` — the linear instruction stream that will
      become the ``Code`` chunk's body.  Caller is responsible for
      emitting ``label`` instructions in the right places —
      ``encode_beam`` does no flow analysis.
    * ``imports`` / ``exports`` / ``locals_`` — the ``ImpT`` /
      ``ExpT`` / ``LocT`` table rows.  Atom indices in these rows
      are also 1-based references into ``atoms``.
    * ``label_count`` — the maximum label number used in the
      instruction stream, plus one.  Erlang's loader uses this to
      pre-allocate the label table.
    * ``max_opcode`` — the maximum opcode byte used in the
      instruction stream.  If 0, ``encode_beam`` derives it from
      the instruction list.
    * ``instruction_set_version`` — the BEAM ``format_number`` to
      put in the ``Code`` chunk header.  Default 0 (the
      conventional value real-world ``erlc`` writes for OTP 26+).
    """

    name: str
    atoms: tuple[str, ...]
    instructions: tuple[BEAMInstruction, ...]
    imports: tuple[BEAMImport, ...] = ()
    exports: tuple[BEAMExport, ...] = ()
    locals_: tuple[BEAMExport, ...] = ()
    funs: tuple[BEAMFun, ...] = ()
    label_count: int = 0
    max_opcode: int = 0
    instruction_set_version: int = 0
    extra_chunks: tuple[tuple[str, bytes], ...] = field(default_factory=tuple)


# ---------------------------------------------------------------------------
# Compact-term encoding
# ---------------------------------------------------------------------------

# Mask for the 3-bit type tag in the first byte of a compact-term operand.
_TAG_MASK: Final[int] = 0b111

# Threshold below which a value fits in the high-4-bit "small" form.
_SMALL_THRESHOLD: Final[int] = 0x10  # 16

# Threshold below which a value fits in the 11-bit "medium" form
# (3 high bits of the first byte + one trailing byte).
_MEDIUM_THRESHOLD: Final[int] = 0x800  # 2048


def encode_compact_term(tag: BEAMTag, value: int) -> bytes:
    """Encode one operand using BEAM's compact-term format.

    See module docstring for the bit layout.  Exposed at the
    package level so tests can exercise the encoder without
    constructing a full ``BEAMModule``.

    >>> encode_compact_term(BEAMTag.U, 0).hex()
    '00'
    >>> encode_compact_term(BEAMTag.U, 15).hex()
    'f0'
    >>> encode_compact_term(BEAMTag.U, 16).hex()  # crosses to medium form
    '0810'
    """
    if value < 0:
        msg = (
            f"compact-term encoding requires a non-negative value, "
            f"got tag={tag.name} value={value}"
        )
        raise BEAMEncodeError(msg)

    tag_bits = int(tag) & _TAG_MASK

    if value < _SMALL_THRESHOLD:
        # Small form: high 4 bits hold the value, bit 3 = 0.
        first = (value << 4) | tag_bits
        return bytes([first])

    if value < _MEDIUM_THRESHOLD:
        # Medium form: bit 3 = 1, bit 4 = 0, top 3 bits of first
        # byte hold high 3 bits of value, second byte holds low 8.
        first = ((value >> 3) & 0xE0) | 0b1000 | tag_bits
        second = value & 0xFF
        return bytes([first, second])

    # Large form: bit 3 = 1, bit 4 = 1.  Encode value as a
    # variable-width big-endian integer and put the byte count
    # minus 2 in the top 3 bits of the first byte (special-cased
    # to "7" + a nested compact-u for byte counts >= 9).
    big = value.to_bytes((value.bit_length() + 7) // 8 or 1, "big")
    # Strip leading zero bytes so the value-bytes are minimal.
    while len(big) > 1 and big[0] == 0:
        big = big[1:]

    length = len(big)
    if length <= 8:
        first = (((length - 2) & 0b111) << 5) | 0b11000 | tag_bits
        return bytes([first]) + big

    # Very large: top 3 bits = 7, then a nested compact-u for
    # length minus 9, then the value bytes.
    first = (0b111 << 5) | 0b11000 | tag_bits
    nested = encode_compact_term(BEAMTag.U, length - 9)
    return bytes([first]) + nested + big


# ---------------------------------------------------------------------------
# Chunk encoders
# ---------------------------------------------------------------------------


def _encode_atu8(atoms: tuple[str, ...]) -> bytes:
    """Encode the ``AtU8`` chunk.

    Layout::

        <u32 BE>  count of atoms
        for each atom:
          <u8>    UTF-8 length (must be < 256)
          <utf8 bytes>

    Real ``erlc`` uses the negative-count form when any single atom
    is longer than 255 bytes (compact-u length encoding).  All Twig-
    generated atoms are short module / function names, so we always
    emit the short-length form.
    """
    out = bytearray()
    out.extend(struct.pack(">I", len(atoms)))
    for atom in atoms:
        encoded = atom.encode("utf-8")
        if len(encoded) > 255:
            msg = (
                f"atom {atom!r} encodes to {len(encoded)} bytes; "
                "the encoder only supports atoms < 256 bytes"
            )
            raise BEAMEncodeError(msg)
        out.append(len(encoded))
        out.extend(encoded)
    return bytes(out)


def _derive_max_opcode(instructions: tuple[BEAMInstruction, ...]) -> int:
    return max((i.opcode for i in instructions), default=0)


def _encode_code(module: BEAMModule) -> bytes:
    """Encode the ``Code`` chunk.

    Layout::

        <u32 BE>  sub_size              (= 16; size of the rest of the header)
        <u32 BE>  format_number         (instruction-set version)
        <u32 BE>  max_opcode
        <u32 BE>  label_count
        <u32 BE>  function_count
        <opcode + operands>...

    ``function_count`` is read from the ``ExpT`` + ``LocT`` row
    counts so we don't need a separate field on ``BEAMModule``.
    """
    sub_size = 16
    max_opcode = module.max_opcode or _derive_max_opcode(module.instructions)
    function_count = len(module.exports) + len(module.locals_)

    header = struct.pack(
        ">IIIII",
        sub_size,
        module.instruction_set_version,
        max_opcode,
        module.label_count,
        function_count,
    )

    code_body = bytearray()
    for instr in module.instructions:
        code_body.append(instr.opcode & 0xFF)
        for operand in instr.operands:
            code_body.extend(encode_compact_term(operand.tag, operand.value))

    return header + bytes(code_body)


def _encode_imports(imports: tuple[BEAMImport, ...]) -> bytes:
    """Encode the ``ImpT`` chunk.

    Layout::

        <u32 BE>  count
        for each row:
          <u32 BE>  module_atom_index
          <u32 BE>  function_atom_index
          <u32 BE>  arity
    """
    out = bytearray(struct.pack(">I", len(imports)))
    for row in imports:
        out.extend(
            struct.pack(
                ">III",
                row.module_atom_index,
                row.function_atom_index,
                row.arity,
            )
        )
    return bytes(out)


def _encode_funt(funs: tuple[BEAMFun, ...]) -> bytes:
    """Encode the ``FunT`` chunk.

    Layout::

        <u32 BE>  count
        for each row:
          <u32 BE>  function_atom_index
          <u32 BE>  arity
          <u32 BE>  code_label
          <u32 BE>  index
          <u32 BE>  num_free
          <u32 BE>  old_uniq

    Each row is 24 bytes.  See ``BEAMFun`` for field semantics.
    """
    out = bytearray(struct.pack(">I", len(funs)))
    for fun in funs:
        out.extend(
            struct.pack(
                ">IIIIII",
                fun.function_atom_index,
                fun.arity,
                fun.code_label,
                fun.index,
                fun.num_free,
                fun.old_uniq & 0xFFFFFFFF,  # mask to 32 bits
            )
        )
    return bytes(out)


def _encode_exports(exports: tuple[BEAMExport, ...]) -> bytes:
    """Encode the ``ExpT`` or ``LocT`` chunk.

    Both chunks share the same layout::

        <u32 BE>  count
        for each row:
          <u32 BE>  function_atom_index
          <u32 BE>  arity
          <u32 BE>  label
    """
    out = bytearray(struct.pack(">I", len(exports)))
    for row in exports:
        out.extend(
            struct.pack(
                ">III",
                row.function_atom_index,
                row.arity,
                row.label,
            )
        )
    return bytes(out)


# ---------------------------------------------------------------------------
# IFF container assembly
# ---------------------------------------------------------------------------


def _wrap_chunk(chunk_id: str, payload: bytes) -> bytes:
    """Wrap one payload in the standard IFF chunk header.

    Each chunk: ``<4-byte ASCII id><u32 BE size><payload><pad>``.
    Padding zeros bring the total chunk size to a 4-byte boundary.
    """
    if len(chunk_id) != 4:
        msg = f"chunk id must be exactly 4 ASCII bytes, got {chunk_id!r}"
        raise BEAMEncodeError(msg)
    pad = (4 - (len(payload) % 4)) % 4
    return (
        chunk_id.encode("ascii")
        + struct.pack(">I", len(payload))
        + payload
        + (b"\x00" * pad)
    )


def encode_beam(module: BEAMModule) -> bytes:
    """Encode a ``BEAMModule`` as a complete ``.beam`` container.

    Returns a ``bytes`` object suitable for writing directly to
    ``<module-name>.beam`` and loading via ``erl`` /
    ``code:load_file/1``.

    Raises ``BEAMEncodeError`` if the module is structurally
    invalid (empty atom table, name mismatch, etc.).
    """
    _validate(module)

    chunks: list[tuple[str, bytes]] = [
        ("AtU8", _encode_atu8(module.atoms)),
        ("Code", _encode_code(module)),
        # ``StrT`` is the literal-strings heap.  Real Erlang loaders
        # expect this chunk to be present even when empty (modern
        # OTP gracefully accepts its absence, but older OTPs and
        # some BEAM tools complain).  An empty ``StrT`` is just zero
        # bytes of payload.
        ("StrT", b""),
        ("ImpT", _encode_imports(module.imports)),
        ("ExpT", _encode_exports(module.exports)),
    ]
    if module.locals_:
        chunks.append(("LocT", _encode_exports(module.locals_)))
    if module.funs:
        # ``FunT`` goes after ``LocT`` per the conventional chunk
        # ordering ``erlc`` uses.  Order isn't strictly load-bearing
        # (the BEAM loader looks chunks up by ID, not position) but
        # matching ``erlc`` keeps disassembler output predictable.
        chunks.append(("FunT", _encode_funt(module.funs)))
    chunks.extend(module.extra_chunks)

    body = bytearray(b"BEAM")
    for chunk_id, payload in chunks:
        body.extend(_wrap_chunk(chunk_id, payload))

    # FOR1 size = total bytes after the 8-byte ``FOR1<u32>`` header;
    # i.e. the size of ``"BEAM" + chunks``.
    return b"FOR1" + struct.pack(">I", len(body)) + bytes(body)


# ---------------------------------------------------------------------------
# Validation
# ---------------------------------------------------------------------------


def _validate(module: BEAMModule) -> None:
    if not module.atoms:
        raise BEAMEncodeError("BEAMModule.atoms must not be empty")
    if module.atoms[0] != module.name:
        msg = (
            f"BEAMModule.name {module.name!r} must equal atoms[0] "
            f"({module.atoms[0]!r}); ECMA-style BEAM file format mandates "
            "the module name at atom index 1 (Python index 0)."
        )
        raise BEAMEncodeError(msg)

    n_atoms = len(module.atoms)
    for row in module.imports:
        for idx, label in (
            (row.module_atom_index, "module_atom_index"),
            (row.function_atom_index, "function_atom_index"),
        ):
            if idx < 1 or idx > n_atoms:
                msg = (
                    f"ImpT row references {label}={idx} but only "
                    f"{n_atoms} atoms are declared"
                )
                raise BEAMEncodeError(msg)
    for kind, rows in (("ExpT", module.exports), ("LocT", module.locals_)):
        for row in rows:
            if row.function_atom_index < 1 or row.function_atom_index > n_atoms:
                msg = (
                    f"{kind} row references function_atom_index="
                    f"{row.function_atom_index} but only {n_atoms} atoms "
                    "are declared"
                )
                raise BEAMEncodeError(msg)
            if row.label < 1:
                msg = f"{kind} row label must be >= 1, got {row.label}"
                raise BEAMEncodeError(msg)
            if module.label_count and row.label > module.label_count:
                msg = (
                    f"{kind} row references label {row.label} but "
                    f"label_count is {module.label_count}"
                )
                raise BEAMEncodeError(msg)

    # FunT — same atom-index + label sanity, plus monotonic
    # ``index`` field check (real ``erlc`` always emits index
    # rows in 0..N-1 order; the loader doesn't enforce it but
    # disassembler tooling tends to assume it).
    for i, fun in enumerate(module.funs):
        if fun.function_atom_index < 1 or fun.function_atom_index > n_atoms:
            msg = (
                f"FunT row references function_atom_index="
                f"{fun.function_atom_index} but only {n_atoms} atoms "
                "are declared"
            )
            raise BEAMEncodeError(msg)
        if fun.code_label < 1:
            msg = f"FunT row code_label must be >= 1, got {fun.code_label}"
            raise BEAMEncodeError(msg)
        if module.label_count and fun.code_label > module.label_count:
            msg = (
                f"FunT row references code_label {fun.code_label} but "
                f"label_count is {module.label_count}"
            )
            raise BEAMEncodeError(msg)
        if fun.index != i:
            msg = (
                f"FunT row {i} has index={fun.index}; convention is to "
                "number rows sequentially from 0 in declaration order"
            )
            raise BEAMEncodeError(msg)
        if fun.num_free < 0:
            msg = f"FunT row num_free must be >= 0, got {fun.num_free}"
            raise BEAMEncodeError(msg)
        if fun.arity < 0:
            msg = f"FunT row arity must be >= 0, got {fun.arity}"
            raise BEAMEncodeError(msg)
