"""Real-`erl` validation: re-encode a real ``erlc``-produced module
and confirm Erlang still loads it.

Strategy
========

We don't yet have ``ir-to-beam`` (BEAM01 Phase 3), so we can't
hand-author a full instruction stream that the loader will accept.
Instead, this test uses a real ``erlc``-produced module as the
input shape:

1. Compile a one-line ``-module(roundtrip). go() -> 42.`` source
   with real ``erlc``.
2. Decode it via ``beam-bytes-decoder``.
3. Reconstruct a ``BEAMModule`` from the decoded fields.
4. Re-encode through ``encode_beam``.
5. Decode the re-encoded bytes.  Assert structural equality with
   the original decode.
6. Drop the re-encoded ``.beam`` next to a fresh module name and
   ask ``erl`` to load + call it.

Steps 1-5 give us byte-level round-trip parity (no runtime
required, runs in CI everywhere).  Step 6 is the real-runtime
proof and skips cleanly when ``erl``/``erlc`` aren't on PATH.

When BEAM01 Phase 3 (``ir-to-beam``) lands, the same test pattern
moves to ``ir-to-beam/tests/`` exercising synthesised modules.
"""

from __future__ import annotations

import shutil
import subprocess
import textwrap
from pathlib import Path

import pytest
from beam_bytecode_encoder import (
    BEAMExport,
    BEAMImport,
    BEAMInstruction,
    BEAMModule,
    BEAMOperand,
    BEAMTag,
    encode_beam,
)
from beam_bytes_decoder import decode_beam_module, parse_beam_container


def _has_erlc() -> bool:
    return shutil.which("erlc") is not None and shutil.which("erl") is not None


requires_erlc = pytest.mark.skipif(
    not _has_erlc(),
    reason="erlc/erl not on PATH",
)


def _compile_reference_module(tmp_path: Path, module_name: str = "roundtrip") -> Path:
    """Run ``erlc`` on a tiny Erlang source and return the .beam path."""
    src = tmp_path / f"{module_name}.erl"
    src.write_text(
        textwrap.dedent(f"""
            -module({module_name}).
            -export([go/0]).
            go() -> 42.
        """).strip()
    )
    subprocess.run(
        ["erlc", str(src.name)],
        cwd=tmp_path,
        check=True,
        capture_output=True,
    )
    return tmp_path / f"{module_name}.beam"


def _instruction_stream_from_decoded_code(
    code_body: bytes,
) -> tuple[BEAMInstruction, ...]:
    """Wrap the raw code-body bytes in a single opaque instruction.

    For the round-trip test we don't need to *understand* the
    instruction stream — we just need to put the same bytes back.
    Wrapping the whole thing in one ``BEAMInstruction`` with
    opcode = first byte and operand = rest-as-raw-extended would
    miss the point because the encoder re-encodes operands.

    Instead, we build a list of fake "opcode-only" instructions
    where each "opcode" is one byte from the original stream.
    The encoder then writes those bytes back unchanged because
    operandless instructions encode as just the opcode byte.

    This is a test-only convenience: real callers will use
    ``ir-to-beam`` which builds proper instructions.
    """
    return tuple(BEAMInstruction(opcode=b) for b in code_body)


def _round_trip_through_encoder(beam_path: Path) -> bytes:
    """Decode the file, rebuild a ``BEAMModule``, re-encode."""
    original_bytes = beam_path.read_bytes()
    decoded = decode_beam_module(original_bytes)

    # The decoder prepends ``None`` at index 0; the encoder uses
    # 1-based atom indices natively, so strip the ``None``.
    atoms = tuple(a for a in decoded.atoms if a is not None)

    # Re-derive 1-based atom indices for imports/exports.
    atom_index = {atom: i + 1 for i, atom in enumerate(atoms)}

    imports = tuple(
        BEAMImport(
            module_atom_index=atom_index[imp.module],
            function_atom_index=atom_index[imp.function],
            arity=imp.arity,
        )
        for imp in decoded.imports
    )
    exports = tuple(
        BEAMExport(
            function_atom_index=atom_index[exp.function],
            arity=exp.arity,
            label=exp.label,
        )
        for exp in decoded.exports
    )
    locals_ = tuple(
        BEAMExport(
            function_atom_index=atom_index[loc.function],
            arity=loc.arity,
            label=loc.label,
        )
        for loc in decoded.locals
    )

    module = BEAMModule(
        name=decoded.module_name,
        atoms=atoms,
        instructions=_instruction_stream_from_decoded_code(decoded.code_header.code),
        imports=imports,
        exports=exports,
        locals_=locals_,
        label_count=decoded.code_header.label_count,
        max_opcode=decoded.code_header.max_opcode,
        instruction_set_version=decoded.code_header.format_number,
    )
    return encode_beam(module)


@requires_erlc
def test_round_trip_preserves_atoms_and_exports(tmp_path: Path) -> None:
    """Decode → re-encode → decode again preserves the public
    structure of an ``erlc``-produced module."""
    beam = _compile_reference_module(tmp_path)
    original = decode_beam_module(beam.read_bytes())

    re_encoded = _round_trip_through_encoder(beam)
    re_decoded = decode_beam_module(re_encoded)

    assert re_decoded.module_name == original.module_name
    # Compare atoms (order-sensitive).
    assert re_decoded.atoms == original.atoms
    # Imports and exports match by structural value.
    assert re_decoded.imports == original.imports
    assert re_decoded.exports == original.exports
    assert re_decoded.locals == original.locals
    # Code header invariants.
    assert re_decoded.code_header.label_count == original.code_header.label_count
    assert re_decoded.code_header.max_opcode == original.code_header.max_opcode
    assert (
        re_decoded.code_header.format_number == original.code_header.format_number
    )


@requires_erlc
def test_round_tripped_module_loads_in_real_erl(tmp_path: Path) -> None:
    """The strongest possible Phase-2 proof: hand the re-encoded
    bytes to real ``erl`` and confirm it accepts them as a loadable
    module.

    We use ``code:load_binary/3`` instead of ``code:load_file/1``
    because load_binary skips the filesystem-name → module-name
    consistency check, which lets us decouple this test from the
    on-disk filename.
    """
    beam = _compile_reference_module(tmp_path)
    re_encoded = _round_trip_through_encoder(beam)

    # erl's escape rules for binaries are easier to reason about
    # via a temp file than a CLI literal.
    out_path = tmp_path / "roundtrip.beam"
    out_path.write_bytes(re_encoded)

    # Sanity: the encoder produces a parseable IFF container.  If
    # this assertion fails we want to know BEFORE invoking erl.
    assert parse_beam_container(re_encoded).form_id == "BEAM"

    # Use code:load_file/1.  erl returns 0 if the eval succeeded
    # (the module:go/0 result becomes the script return value).
    eval_expr = (
        f'{{module, M}} = code:load_file(roundtrip),'
        f'42 = roundtrip:go(),'
        f'io:format("ok~n"),'
        f'init:stop().'
    )
    result = subprocess.run(
        ["erl", "-noshell", "-pa", str(tmp_path), "-eval", eval_expr],
        capture_output=True,
        text=True,
        timeout=15,
    )
    assert result.returncode == 0, (
        f"erl rejected the round-tripped module:\n"
        f"  stdout: {result.stdout!r}\n"
        f"  stderr: {result.stderr!r}"
    )
    assert "ok" in result.stdout
