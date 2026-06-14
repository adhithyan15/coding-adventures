"""CILCodeGenerator — CodeGenerator[IrProgram, CILProgramArtifact] adapter (LANG20).

This module adapts the existing ``lower_ir_to_cil_bytecode`` /
``validate_for_clr`` functions to the ``CodeGenerator[IR, Assembly]``
protocol defined in ``codegen-core``.

Pipeline context
----------------

The ``CILCodeGenerator`` sits at the *codegen boundary*::

    IrProgram
        ↓
    [CILCodeGenerator.validate()]   — check opcode support, value range, syscall set
    [CILCodeGenerator.generate()]   — emit structured CIL program artifact
        ↓ CILProgramArtifact(entry_label, methods, data_offsets, …)
        ├─→ clr-simulator.load(artifact)        (simulator pipeline — accepts directly)
        └─→ PE packager → executable .exe        (AOT pipeline, future)

The CLR simulator accepts ``CILProgramArtifact`` directly, so no binary
encoding step is needed for simulation.

``name = "cil"``
    Short identifier used by ``CodeGeneratorRegistry`` lookups.

Why ``CILProgramArtifact`` and not plain ``bytes``?
    The artifact is a structured multi-method object.  Each
    ``CILMethodArtifact`` carries raw CIL bytecode (``body: bytes``), stack
    depth, and local variable types.  The CLR simulator needs this structure;
    a PE packager would additionally need the token table and helper specs.
    Exposing the full artifact preserves all metadata for downstream consumers.
"""

from __future__ import annotations

from compiler_ir import IrProgram

from ir_to_cil_bytecode.backend import (
    CILBackendConfig,
    CILProgramArtifact,
    lower_ir_to_cil_bytecode,
    validate_for_clr,
)


class CILCodeGenerator:
    """Validate-and-generate adapter for the CIL (Common Intermediate Language) backend.

    Satisfies ``CodeGenerator[IrProgram, CILProgramArtifact]`` structurally.

    CIL is the bytecode of the Common Language Runtime (.NET / Mono).  The
    backend emits multi-method artifacts whose method bodies contain raw CIL
    bytecode sequences.

    Attributes
    ----------
    name:
        ``"cil"`` — used by ``CodeGeneratorRegistry`` for lookup.

    Parameters
    ----------
    config:
        Optional ``CILBackendConfig`` instance.  Defaults to
        ``CILBackendConfig()`` (standard stack depth, method settings).

    Examples
    --------
    >>> from compiler_ir import IrImmediate, IrInstruction, IrOp, IrProgram, IrRegister
    >>> prog = IrProgram(entry_label="_start")
    >>> load = IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), IrImmediate(1)], id=0)
    >>> prog.add_instruction(load)
    >>> prog.add_instruction(IrInstruction(IrOp.HALT, [], id=1))
    >>> gen = CILCodeGenerator()
    >>> gen.validate(prog)
    []
    >>> artifact = gen.generate(prog)
    >>> artifact.entry_label
    '_start'
    """

    name = "cil"

    def __init__(self, config: CILBackendConfig | None = None) -> None:
        self._config = config or CILBackendConfig()

    def validate(self, ir: IrProgram) -> list[str]:
        """Validate ``ir`` for CLR/CIL compatibility.

        Checks performed (see ``validate_for_clr`` for full details):

        - Every opcode is in the supported CIL backend set.
        - Every ``IrImmediate`` fits in a 32-bit signed integer.
        - Only ``SYSCALL 1``, ``SYSCALL 2``, and ``SYSCALL 10`` are used.

        Parameters
        ----------
        ir:
            The ``IrProgram`` to inspect.

        Returns
        -------
        list[str]
            Human-readable error messages.  Empty list = compatible.
        """
        return validate_for_clr(ir)

    def generate(self, ir: IrProgram) -> CILProgramArtifact:
        """Compile ``ir`` to a structured CIL program artifact.

        Runs ``validate()`` internally.  Raises ``CILBackendError`` if the
        program fails validation.

        Parameters
        ----------
        ir:
            A validated (or to-be-validated) ``IrProgram``.

        Returns
        -------
        CILProgramArtifact
            ``entry_label`` — name of the entry-point method.
            ``methods`` — tuple of ``CILMethodArtifact`` (name, body bytes,
            max_stack, local_types).
            ``data_offsets`` — label → static data offset map.

        Raises
        ------
        CILBackendError
            If the IR fails validation.
        """
        return lower_ir_to_cil_bytecode(ir, self._config)
