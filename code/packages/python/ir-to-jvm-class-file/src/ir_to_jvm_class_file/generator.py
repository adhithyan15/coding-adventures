"""JVMCodeGenerator — CodeGenerator[IrProgram, JVMClassArtifact] adapter (LANG20).

This module adapts the existing ``lower_ir_to_jvm_class_file`` /
``validate_for_jvm`` functions to the ``CodeGenerator[IR, Assembly]``
protocol defined in ``codegen-core``.

Pipeline context
----------------

The ``JVMCodeGenerator`` sits at the *codegen boundary*::

    IrProgram
        ↓
    [JVMCodeGenerator.validate()]   — check opcode support, value range, syscall set
    [JVMCodeGenerator.generate()]   — emit JVM class file bytes + metadata
        ↓ JVMClassArtifact(class_bytes: bytes, callable_labels, data_offsets)
        ├─→ jvm-simulator.load(class_bytes)     (simulator pipeline)
        └─→ write_class_file(artifact, dir)      (AOT pipeline, future)

The generator does **not** run the JVM simulator.  Execution is the concern
of the downstream pipeline.

``name = "jvm"``
    Short identifier used by ``CodeGeneratorRegistry`` lookups.

Why ``JVMClassArtifact`` and not plain ``bytes``?
    The artifact carries ``class_bytes`` (the standard .class binary) plus
    ``callable_labels`` and ``data_offsets`` metadata.  Downstream consumers
    (simulator, test harnesses, debuggers) need the metadata; plain bytes would
    discard it.
"""

from __future__ import annotations

from compiler_ir import IrProgram

from ir_to_jvm_class_file.backend import (
    JvmBackendConfig,
    JVMClassArtifact,
    lower_ir_to_jvm_class_file,
    validate_for_jvm,
)


class JVMCodeGenerator:
    """Validate-and-generate adapter for the JVM class-file backend.

    Satisfies ``CodeGenerator[IrProgram, JVMClassArtifact]`` structurally.

    The JVM backend emits standard Java Virtual Machine class files (magic
    bytes ``0xCAFEBABE``).  The class name and Java version are controlled via
    ``JvmBackendConfig`` — the default configuration uses a generic class name
    and Java 5 (major version 49).

    Attributes
    ----------
    name:
        ``"jvm"`` — used by ``CodeGeneratorRegistry`` for lookup.

    Parameters
    ----------
    config:
        Optional ``JvmBackendConfig`` instance.  Defaults to
        ``JvmBackendConfig()`` (class name ``"Program"``, Java 5).

    Examples
    --------
    >>> from compiler_ir import IrImmediate, IrInstruction, IrOp, IrProgram, IrRegister
    >>> prog = IrProgram(entry_label="_start")
    >>> load = IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), IrImmediate(1)], id=0)
    >>> prog.add_instruction(load)
    >>> prog.add_instruction(IrInstruction(IrOp.HALT, [], id=1))
    >>> gen = JVMCodeGenerator()
    >>> gen.validate(prog)
    []
    >>> result = gen.generate(prog)
    >>> result.class_bytes[:4]
    b'\\xca\\xfe\\xba\\xbe'
    """

    name = "jvm"

    def __init__(self, config: JvmBackendConfig | None = None) -> None:
        self._config = config or JvmBackendConfig(class_name="Program")

    def validate(self, ir: IrProgram) -> list[str]:
        """Validate ``ir`` for JVM compatibility.

        Checks performed (see ``validate_for_jvm`` for full details):

        - Every opcode is in the supported JVM backend set.
        - Every ``IrImmediate`` fits in a 32-bit signed integer
          (−2 147 483 648 to 2 147 483 647).
        - Only ``SYSCALL 1`` and ``SYSCALL 4`` are used.

        Parameters
        ----------
        ir:
            The ``IrProgram`` to inspect.

        Returns
        -------
        list[str]
            Human-readable error messages.  Empty list = compatible.
        """
        return validate_for_jvm(ir)

    def generate(self, ir: IrProgram) -> JVMClassArtifact:
        """Compile ``ir`` to a JVM class file.

        Runs ``validate()`` internally.  Raises ``JvmBackendError`` if the
        program fails validation.

        Parameters
        ----------
        ir:
            A validated (or to-be-validated) ``IrProgram``.

        Returns
        -------
        JVMClassArtifact
            ``class_bytes`` — valid JVM class file starting with ``0xCAFEBABE``.
            ``class_name`` — fully-qualified class name.
            ``callable_labels`` — entry and called label names.
            ``data_offsets`` — label → data-segment offset map.

        Raises
        ------
        JvmBackendError
            If the IR fails validation.
        """
        return lower_ir_to_jvm_class_file(ir, self._config)
