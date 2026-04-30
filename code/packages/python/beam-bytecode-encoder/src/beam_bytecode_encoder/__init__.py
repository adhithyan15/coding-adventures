"""Pure encoder: structured ``BEAMModule`` → ``.beam`` container bytes.

This is BEAM01 Phase 2.  See
``code/specs/BEAM01-twig-on-real-erl.md`` for the broader plan.
"""

from beam_bytecode_encoder.encoder import (
    BEAMEncodeError,
    BEAMExport,
    BEAMFun,
    BEAMImport,
    BEAMInstruction,
    BEAMModule,
    BEAMOperand,
    BEAMTag,
    encode_beam,
    encode_compact_term,
)

__all__ = [
    "BEAMEncodeError",
    "BEAMExport",
    "BEAMFun",
    "BEAMImport",
    "BEAMInstruction",
    "BEAMModule",
    "BEAMOperand",
    "BEAMTag",
    "encode_beam",
    "encode_compact_term",
]
