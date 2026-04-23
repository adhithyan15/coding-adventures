"""End-to-end compiler from Oct source to JVM class-file bytes.

Pipeline
--------

::

    Oct source text
        → oct-lexer / oct-parser       (characters → AST)
        → oct-type-checker             (untyped AST → typed AST)
        → oct-ir-compiler (JVM_IO)     (typed AST → IrProgram with JVM SYSCALLs)
        → ir-to-jvm-class-file         (IrProgram → JVM ``.class`` file bytes)
        → jvm-class-file               (parse + structural validation)

The ``JVM_IO`` config (``write_byte_syscall=1``, ``read_byte_syscall=4``)
tells the Oct IR compiler to emit ``SYSCALL 1`` for ``out()`` and
``SYSCALL 4`` for ``in()``, matching the JVM host's ABI:

    SYSCALL 1 → ``System.out.write(int)``  (write byte to stdout)
    SYSCALL 4 → ``System.in.read()``       (read byte from stdin)

These match the Dartmouth BASIC convention used in ``ir-to-jvm-class-file``.
"""

from __future__ import annotations

from dataclasses import dataclass, replace
from pathlib import Path

from ir_to_jvm_class_file import (
    JvmBackendConfig,
    JVMClassArtifact,
    lower_ir_to_jvm_class_file,
)
from ir_to_jvm_class_file import (
    write_class_file as backend_write_class_file,
)
from jvm_class_file import parse_class_file
from oct_ir_compiler import JVM_IO, OctCompileResult, compile_oct
from oct_parser import parse_oct
from oct_type_checker import check_oct


@dataclass(frozen=True)
class PackageResult:
    """All artifacts produced by a successful Oct → JVM compilation.

    Attributes:
        source:          The original Oct source string.
        class_name:      The JVM class name (e.g. ``"OctProgram"``).
        ast:             The untyped AST (from ``oct-parser``).
        ir:              The compiled ``IrProgram`` (from ``oct-ir-compiler``).
        artifact:        The raw ``JVMClassArtifact`` (from ``ir-to-jvm-class-file``).
        parsed_class:    Decoded class-file object (structural validation).
        class_bytes:     Raw ``.class`` file bytes.
        class_file_path: Where the class file was written (``write_class_file`` only).
    """

    source: str
    class_name: str
    ast: object
    ir: object
    artifact: JVMClassArtifact
    parsed_class: object
    class_bytes: bytes
    class_file_path: Path | None = None


class PackageError(Exception):
    """Raised when any pipeline stage fails.

    Attributes:
        stage:   Which pipeline stage failed (e.g. ``"parse"``, ``"type-check"``).
        message: Human-readable description of the failure.
        cause:   The underlying exception, if any.
    """

    def __init__(
        self,
        stage: str,
        message: str,
        cause: Exception | None = None,
    ) -> None:
        super().__init__(message)
        self.stage = stage
        self.message = message
        self.cause = cause

    def __str__(self) -> str:
        return f"[{self.stage}] {self.message}"


class OctJvmCompiler:
    """Compile Oct source into JVM class-file bytes.

    The compiler runs the full Oct pipeline — lexing, parsing, type-checking,
    IR compilation with ``JVM_IO`` config, and JVM class-file lowering — and
    returns the parsed and validated ``.class`` file.

    Example::

        from oct_jvm_compiler import OctJvmCompiler

        result = OctJvmCompiler().compile_source("fn main() { }")
        assert result.class_bytes[:4] == b"\\xca\\xfe\\xba\\xbe"  # JVM magic
    """

    def __init__(
        self,
        *,
        class_name: str = "OctProgram",
        emit_main_wrapper: bool = True,
    ) -> None:
        self.class_name = class_name
        # emit_main_wrapper wraps the _start method in a public static main(String[])
        # method so the class can be executed with `java OctProgram`.
        self.emit_main_wrapper = emit_main_wrapper

    def compile_source(
        self,
        source: str,
        *,
        class_name: str | None = None,
        emit_main_wrapper: bool | None = None,
    ) -> PackageResult:
        """Compile Oct *source* to a JVM ``.class`` file.

        Args:
            source:            Oct source text.
            class_name:        Override the JVM class name for this call.
            emit_main_wrapper: Override whether to emit ``public static main``.

        Returns:
            A ``PackageResult`` with all compilation artifacts.

        Raises:
            PackageError: If any pipeline stage fails.
        """
        resolved_class_name = class_name if class_name is not None else self.class_name
        resolved_emit_main = (
            self.emit_main_wrapper if emit_main_wrapper is None else emit_main_wrapper
        )

        # ── Stage 1: Parse ───────────────────────────────────────────────────
        try:
            ast = parse_oct(source)
        except Exception as exc:
            raise PackageError("parse", str(exc), exc) from exc

        # ── Stage 2: Type-check ──────────────────────────────────────────────
        tc_result = check_oct(ast)
        if not tc_result.ok:
            msgs = "; ".join(str(e.message) for e in tc_result.errors)
            raise PackageError("type-check", msgs)

        # ── Stage 3: IR compile (JVM_IO config) ──────────────────────────────
        # JVM_IO sets write_byte_syscall=1 and read_byte_syscall=4, so Oct's
        # out(PORT, val) emits SYSCALL 1 (System.out.write) and in(PORT) emits
        # SYSCALL 4 (System.in.read), matching the JVM host's ABI.
        try:
            ir_result: OctCompileResult = compile_oct(
                tc_result.typed_ast, config=JVM_IO
            )
        except Exception as exc:  # pragma: no cover
            raise PackageError("ir-compile", str(exc), exc) from exc

        # ── Stage 4: Lower to JVM class file ─────────────────────────────────
        try:
            artifact = lower_ir_to_jvm_class_file(
                ir_result.program,
                JvmBackendConfig(
                    class_name=resolved_class_name,
                    emit_main_wrapper=resolved_emit_main,
                ),
            )
        except Exception as exc:  # pragma: no cover
            raise PackageError("lower-jvm", str(exc), exc) from exc

        # ── Stage 5: Parse / structurally validate the class file ────────────
        try:
            parsed_class = parse_class_file(artifact.class_bytes)
        except Exception as exc:  # pragma: no cover
            raise PackageError("validate-class", str(exc), exc) from exc

        return PackageResult(
            source=source,
            class_name=resolved_class_name,
            ast=ast,
            ir=ir_result.program,
            artifact=artifact,
            parsed_class=parsed_class,
            class_bytes=artifact.class_bytes,
        )

    def write_class_file(
        self,
        source: str,
        output_dir: str | Path,
        *,
        class_name: str | None = None,
        emit_main_wrapper: bool | None = None,
    ) -> PackageResult:
        """Compile *source* and write the JVM ``.class`` file to *output_dir*.

        The file is written as ``<output_dir>/<class_name>.class``.

        Args:
            source:     Oct source text.
            output_dir: Directory in which to write the ``.class`` file.

        Returns:
            A ``PackageResult`` with ``class_file_path`` set.

        Raises:
            PackageError: If compilation or the file write fails.
        """
        result = self.compile_source(
            source,
            class_name=class_name,
            emit_main_wrapper=emit_main_wrapper,
        )
        try:
            path = backend_write_class_file(result.artifact, output_dir)
        except Exception as exc:  # pragma: no cover
            raise PackageError("write", str(exc), exc) from exc
        return replace(result, class_file_path=path)


# ---------------------------------------------------------------------------
# Module-level convenience functions
# ---------------------------------------------------------------------------


def compile_source(
    source: str,
    *,
    class_name: str = "OctProgram",
    emit_main_wrapper: bool = True,
) -> PackageResult:
    """Compile Oct *source* to a JVM ``.class`` file (stateless wrapper).

    Equivalent to ``OctJvmCompiler(...).compile_source(source)``.
    """
    return OctJvmCompiler(
        class_name=class_name,
        emit_main_wrapper=emit_main_wrapper,
    ).compile_source(source)


def pack_source(
    source: str,
    *,
    class_name: str = "OctProgram",
    emit_main_wrapper: bool = True,
) -> PackageResult:
    """Alias for :func:`compile_source` — compile Oct source to a JVM class file."""
    return compile_source(
        source,
        class_name=class_name,
        emit_main_wrapper=emit_main_wrapper,
    )


def write_class_file(
    source: str,
    output_dir: str | Path,
    *,
    class_name: str = "OctProgram",
    emit_main_wrapper: bool = True,
) -> PackageResult:
    """Compile Oct *source* and write the JVM ``.class`` file to *output_dir*."""
    return OctJvmCompiler(
        class_name=class_name,
        emit_main_wrapper=emit_main_wrapper,
    ).write_class_file(source, output_dir)
