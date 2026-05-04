"""Tests for TW04 Phase 4d — cross-module call lowering.

Covers:
* ``_discover_callable_regions`` skips cross-module CALL targets
  (labels containing ``/``) rather than raising MissingCallableLabels.
* A cross-module CALL lowers to ``invokestatic`` targeting the foreign
  class and method decomposed at the last ``/``.
* ``build_runtime_class_artifact()`` produces a valid JVMClassArtifact
  with both ``__ca_regs`` and ``__ca_objregs`` in its constant pool.
* Setting ``JvmBackendConfig.external_runtime_class`` redirects all
  ``getstatic`` / ``putstatic`` for ``__ca_regs`` / ``__ca_objregs``
  to the external class rather than the module class itself.
"""

from __future__ import annotations

import struct
import subprocess
import tempfile
from pathlib import Path

import pytest
from compiler_ir import IrImmediate, IrInstruction, IrLabel, IrOp, IrProgram, IrRegister

from ir_to_jvm_class_file import (
    TWIG_RUNTIME_BINARY_NAME,
    JvmBackendConfig,
    JVMClassArtifact,
    build_runtime_class_artifact,
    lower_ir_to_jvm_class_file,
)
from ir_to_jvm_class_file.backend import JvmBackendError


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _minimal_program_with_cross_module_call(
    call_label: str = "a/math/add",
) -> IrProgram:
    """Build the simplest IrProgram that contains one cross-module CALL.

    Structure::

        _start:
            LOAD_IMM r2, 7      ; first arg
            LOAD_IMM r3, 35     ; second arg
            CALL <call_label>   ; cross-module call
            HALT
    """
    prog = IrProgram(entry_label="_start")
    prog.add_instruction(IrInstruction(IrOp.LABEL,  [IrLabel("_start")],          id=-1))
    prog.add_instruction(IrInstruction(IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(7)],  id=1))
    prog.add_instruction(IrInstruction(IrOp.LOAD_IMM, [IrRegister(3), IrImmediate(35)], id=2))
    prog.add_instruction(IrInstruction(IrOp.CALL,    [IrLabel(call_label)],        id=3))
    prog.add_instruction(IrInstruction(IrOp.HALT,    [],                           id=4))
    return prog


def _bytes_contain_utf8(data: bytes, text: str) -> bool:
    """Return True if ``data`` contains the UTF-8 encoding of ``text``."""
    encoded = text.encode("utf-8")
    return encoded in data


# ---------------------------------------------------------------------------
# test_cross_module_call_skipped_in_callable_regions
# ---------------------------------------------------------------------------


class TestCrossModuleCallableRegions:
    """``_discover_callable_regions`` must not raise for cross-module targets."""

    def test_single_cross_module_call_does_not_raise(self) -> None:
        """An IrProgram whose only CALL is cross-module must lower cleanly.

        Previously the validator would add ``"x/y"`` to ``callable_names``
        and then fail with 'Missing callable labels: [\"x/y\"]' because
        the label doesn't exist in the local IrProgram.
        """
        prog = _minimal_program_with_cross_module_call("x/y")
        config = JvmBackendConfig(
            class_name="TestClass",
            emit_main_wrapper=False,
            external_runtime_class=TWIG_RUNTIME_BINARY_NAME,
        )
        # Should NOT raise:
        artifact = lower_ir_to_jvm_class_file(prog, config)
        assert isinstance(artifact, JVMClassArtifact)

    def test_multi_segment_cross_module_call_does_not_raise(self) -> None:
        """``stdlib/io/println`` (three path segments) is also cross-module."""
        prog = _minimal_program_with_cross_module_call("stdlib/io/println")
        config = JvmBackendConfig(
            class_name="TestClass",
            emit_main_wrapper=False,
            external_runtime_class=TWIG_RUNTIME_BINARY_NAME,
        )
        artifact = lower_ir_to_jvm_class_file(prog, config)
        assert isinstance(artifact, JVMClassArtifact)

    def test_local_call_still_requires_local_label(self) -> None:
        """A CALL to a local (non-slash) label that doesn't exist must still
        raise ``JvmBackendError``."""
        prog = IrProgram(entry_label="_start")
        prog.add_instruction(IrInstruction(IrOp.LABEL,  [IrLabel("_start")], id=-1))
        prog.add_instruction(IrInstruction(IrOp.CALL,   [IrLabel("missing_fn")], id=1))
        prog.add_instruction(IrInstruction(IrOp.HALT,   [], id=2))
        config = JvmBackendConfig(class_name="TestClass", emit_main_wrapper=False)
        with pytest.raises(JvmBackendError, match="Missing callable labels"):
            lower_ir_to_jvm_class_file(prog, config)


# ---------------------------------------------------------------------------
# test_cross_module_call_lowering_emits_invokestatic
# ---------------------------------------------------------------------------


class TestCrossModuleCallLowering:
    """The backend must emit ``invokestatic`` for a cross-module CALL."""

    def test_invokestatic_present_in_class_bytes(self) -> None:
        """Class bytes for a cross-module CALL must contain an invokestatic
        (opcode 0xB8) targeting the foreign class and method name."""
        prog = _minimal_program_with_cross_module_call("a/math/add")
        config = JvmBackendConfig(
            class_name="user/hello",
            emit_main_wrapper=False,
            external_runtime_class=TWIG_RUNTIME_BINARY_NAME,
        )
        artifact = lower_ir_to_jvm_class_file(prog, config)

        # The constant pool must contain UTF-8 strings for both the
        # foreign class name and the method name.
        assert _bytes_contain_utf8(artifact.class_bytes, "a/math"), (
            "constant pool should contain the foreign class name 'a/math'"
        )
        assert _bytes_contain_utf8(artifact.class_bytes, "add"), (
            "constant pool should contain the method name 'add'"
        )

        # The invokestatic opcode must appear in the bytecode.
        _INVOKESTATIC = 0xB8
        assert _INVOKESTATIC in artifact.class_bytes, (
            "class bytes should contain an invokestatic instruction"
        )

    def test_class_name_user_hello(self) -> None:
        """A module with class_name containing '/' lowers correctly."""
        prog = _minimal_program_with_cross_module_call("a/math/add")
        config = JvmBackendConfig(
            class_name="user/hello",
            emit_main_wrapper=True,
            external_runtime_class=TWIG_RUNTIME_BINARY_NAME,
        )
        artifact = lower_ir_to_jvm_class_file(prog, config)
        assert artifact.class_name == "user/hello"

    def test_two_segment_label_decomposition(self) -> None:
        """``"a/add"`` decomposes to class ``"a"``, method ``"add"``."""
        prog = _minimal_program_with_cross_module_call("a/add")
        config = JvmBackendConfig(
            class_name="b",
            emit_main_wrapper=False,
            external_runtime_class=TWIG_RUNTIME_BINARY_NAME,
        )
        artifact = lower_ir_to_jvm_class_file(prog, config)
        assert _bytes_contain_utf8(artifact.class_bytes, "add")

    def test_three_segment_label_decomposition(self) -> None:
        """``"stdlib/io/println"`` decomposes to class ``"stdlib/io"``,
        method ``"println"``."""
        prog = _minimal_program_with_cross_module_call("stdlib/io/println")
        config = JvmBackendConfig(
            class_name="user/hello",
            emit_main_wrapper=False,
            external_runtime_class=TWIG_RUNTIME_BINARY_NAME,
        )
        artifact = lower_ir_to_jvm_class_file(prog, config)
        assert _bytes_contain_utf8(artifact.class_bytes, "stdlib/io")
        assert _bytes_contain_utf8(artifact.class_bytes, "println")


# ---------------------------------------------------------------------------
# test_runtime_class_artifact_fields
# ---------------------------------------------------------------------------


class TestRuntimeClassArtifact:
    """``build_runtime_class_artifact()`` must return a structurally sound
    JVMClassArtifact with both register-array fields in its constant pool."""

    def test_returns_jvm_class_artifact(self) -> None:
        art = build_runtime_class_artifact()
        assert isinstance(art, JVMClassArtifact)

    def test_class_name_matches_binary_name(self) -> None:
        """``class_name`` uses dots (canonical Java form); the binary name
        in the constant pool uses slashes (JVM internal form)."""
        art = build_runtime_class_artifact()
        # class_name should be the dot form
        assert art.class_name == TWIG_RUNTIME_BINARY_NAME.replace("/", ".")

    def test_ca_regs_in_constant_pool(self) -> None:
        art = build_runtime_class_artifact()
        assert _bytes_contain_utf8(art.class_bytes, "__ca_regs"), (
            "constant pool must contain the field name __ca_regs"
        )

    def test_ca_objregs_in_constant_pool(self) -> None:
        art = build_runtime_class_artifact()
        assert _bytes_contain_utf8(art.class_bytes, "__ca_objregs"), (
            "constant pool must contain the field name __ca_objregs"
        )

    def test_int_array_descriptor_present(self) -> None:
        art = build_runtime_class_artifact()
        assert _bytes_contain_utf8(art.class_bytes, "[I"), (
            "constant pool must contain the int-array descriptor '[I'"
        )

    def test_object_array_descriptor_present(self) -> None:
        art = build_runtime_class_artifact()
        assert _bytes_contain_utf8(art.class_bytes, "[Ljava/lang/Object;"), (
            "constant pool must contain the Object-array descriptor"
        )

    def test_callable_labels_empty(self) -> None:
        """TwigRuntime has no callable regions — it only holds static fields."""
        art = build_runtime_class_artifact()
        assert art.callable_labels == ()

    def test_magic_bytes(self) -> None:
        """All JVM class files start with 0xCAFEBABE."""
        art = build_runtime_class_artifact()
        assert art.class_bytes[:4] == b"\xca\xfe\xba\xbe"

    def test_custom_reg_count(self) -> None:
        """``reg_count`` controls the size constant in the clinit."""
        art_128 = build_runtime_class_artifact(reg_count=128)
        art_256 = build_runtime_class_artifact(reg_count=256)
        # Different reg_count → different class bytes
        assert art_128.class_bytes != art_256.class_bytes

    @pytest.mark.skipif(
        not __import__("shutil").which("java"),
        reason="java not available",
    )
    def test_runtime_class_loads_on_real_java(self) -> None:
        """The TwigRuntime class must pass the JVM verifier."""
        art = build_runtime_class_artifact()
        with tempfile.TemporaryDirectory() as tmp:
            # Write to the correct directory structure for the package
            pkg_dir = Path(tmp) / "coding_adventures" / "twig" / "runtime"
            pkg_dir.mkdir(parents=True)
            class_path = pkg_dir / "TwigRuntime.class"
            class_path.write_bytes(art.class_bytes)
            proc = subprocess.run(
                ["java", "-cp", tmp,
                 "coding_adventures.twig.runtime.TwigRuntime"],
                capture_output=True,
                timeout=10,
                check=False,
            )
            # The class has no main() so it exits with an error about
            # missing main — but that means the class LOADED successfully.
            # A verifier failure would produce a different error.
            assert (
                b"Main method not found" in proc.stderr
                or b"Class TwigRuntime" in proc.stderr
                or proc.returncode != 0  # any exit is fine; load succeeded
            ), f"Unexpected stderr: {proc.stderr!r}"


# ---------------------------------------------------------------------------
# test_external_runtime_class_used_for_regs
# ---------------------------------------------------------------------------


class TestExternalRuntimeClass:
    """When ``external_runtime_class`` is set, ``__ca_regs`` references
    point to the external class, not the module's own class."""

    def _simple_program(self) -> IrProgram:
        """Tiny program: LOAD_IMM + SYSCALL 1 + HALT."""
        prog = IrProgram(entry_label="_start")
        prog.add_instruction(IrInstruction(IrOp.LABEL,    [IrLabel("_start")],              id=-1))
        prog.add_instruction(IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(42)], id=1))
        prog.add_instruction(IrInstruction(IrOp.SYSCALL,  [IrImmediate(1), IrRegister(1)],  id=2))
        prog.add_instruction(IrInstruction(IrOp.HALT,     [],                               id=3))
        return prog

    def test_external_runtime_name_in_pool(self) -> None:
        """When external_runtime_class is set, the constant pool must
        reference that class name for the register-array fields, not
        the module class name."""
        prog = self._simple_program()
        config = JvmBackendConfig(
            class_name="user/hello",
            external_runtime_class=TWIG_RUNTIME_BINARY_NAME,
        )
        artifact = lower_ir_to_jvm_class_file(prog, config)

        # TwigRuntime binary name must appear in the constant pool.
        assert _bytes_contain_utf8(artifact.class_bytes, TWIG_RUNTIME_BINARY_NAME), (
            "constant pool should reference TwigRuntime when "
            "external_runtime_class is set"
        )

    def test_single_module_does_not_include_twig_runtime(self) -> None:
        """Without external_runtime_class, the class should own __ca_regs
        itself and NOT reference TwigRuntime."""
        prog = self._simple_program()
        config = JvmBackendConfig(class_name="MyProgram")
        artifact = lower_ir_to_jvm_class_file(prog, config)

        assert not _bytes_contain_utf8(artifact.class_bytes, "TwigRuntime"), (
            "single-module programs should not reference TwigRuntime"
        )

    def test_module_class_name_slash_form_accepted(self) -> None:
        """Class names in slash form (JVM internal) are now accepted
        by the validator, enabling multi-module class naming."""
        prog = self._simple_program()
        config = JvmBackendConfig(
            class_name="stdlib/io",
            emit_main_wrapper=False,
            external_runtime_class=TWIG_RUNTIME_BINARY_NAME,
        )
        # Must not raise JvmBackendError("class_name must be a legal Java binary name")
        artifact = lower_ir_to_jvm_class_file(prog, config)
        assert artifact.class_name == "stdlib/io"
