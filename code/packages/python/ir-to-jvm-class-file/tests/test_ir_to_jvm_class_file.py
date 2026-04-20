from __future__ import annotations

import os
import subprocess
import tempfile
from pathlib import Path

import pytest
from compiler_ir import IrDataDecl
from jvm_class_file import parse_class_file

from ir_to_jvm_class_file import (
    JvmBackendConfig,
    JVMClassArtifact,
    lower_ir_to_jvm_class_file,
    write_class_file,
)
from ir_to_jvm_class_file.backend import JvmBackendError


def compile_brainfuck_program(source: str) -> bytes:
    from brainfuck import parse_brainfuck
    from brainfuck_ir_compiler import compile_brainfuck, release_config

    ast = parse_brainfuck(source)
    result = compile_brainfuck(ast, "test.bf", release_config())
    artifact = lower_ir_to_jvm_class_file(
        result.program,
        JvmBackendConfig(class_name="BrainfuckProgram"),
    )
    return artifact.class_bytes


def compile_nib_program(source: str) -> bytes:
    return lower_nib_program(source, "NibProgram").class_bytes


def lower_nib_program(source: str, class_name: str):
    from nib_ir_compiler import compile_nib, release_config
    from nib_parser import parse_nib
    from nib_type_checker import check

    ast = parse_nib(source)
    typed = check(ast)
    assert typed.ok, [error.message for error in typed.errors]
    result = compile_nib(typed.typed_ast, release_config())
    return lower_ir_to_jvm_class_file(
        result.program,
        JvmBackendConfig(class_name=class_name),
    )


def test_brainfuck_source_compiles_to_parseable_class_file() -> None:
    parsed = parse_class_file(compile_brainfuck_program("+."))

    assert parsed.this_class_name == "BrainfuckProgram"
    assert parsed.find_method("_start", "()I") is not None
    assert parsed.find_method("main", "([Ljava/lang/String;)V") is not None
    assert parsed.find_method("__ca_syscall", "(II)V") is not None


def test_nib_source_compiles_to_parseable_class_file() -> None:
    parsed = parse_class_file(
        compile_nib_program("static x: u4 = 7; fn main() { let y: u4 = x; }")
    )

    assert parsed.this_class_name == "NibProgram"
    assert parsed.find_method("_start", "()I") is not None
    assert parsed.find_method("_fn_main", "()I") is not None
    assert parsed.find_method("__ca_memLoadByte", "(I)I") is not None


def test_write_class_file_uses_classpath_layout() -> None:
    artifact = lower_ir_to_jvm_class_file(
        program=_simple_program(),
        config=JvmBackendConfig(class_name="demo.Example"),
    )

    with tempfile.TemporaryDirectory() as tempdir:
        target = write_class_file(artifact, tempdir)
        assert target == Path(tempdir) / "demo" / "Example.class"
        assert target.read_bytes() == artifact.class_bytes


def test_invalid_class_name_is_rejected() -> None:
    with pytest.raises(JvmBackendError, match="legal Java binary name"):
        lower_ir_to_jvm_class_file(
            program=_simple_program(),
            config=JvmBackendConfig(class_name=".Example"),
        )


def test_write_class_file_rejects_path_escaping_artifact() -> None:
    artifact = JVMClassArtifact(
        class_name=".Escape",
        class_bytes=b"class-bytes",
        callable_labels=(),
        data_offsets={},
    )

    with (
        tempfile.TemporaryDirectory() as tempdir,
        pytest.raises(JvmBackendError, match="escapes the requested"),
    ):
        write_class_file(artifact, tempdir)


def test_write_class_file_rejects_symlinked_parent_directory() -> None:
    artifact = lower_ir_to_jvm_class_file(
        program=_simple_program(),
        config=JvmBackendConfig(class_name="demo.Example"),
    )

    with (
        tempfile.TemporaryDirectory() as tempdir,
        tempfile.TemporaryDirectory() as sink,
    ):
        os.symlink(sink, Path(tempdir) / "demo")
        with pytest.raises(JvmBackendError, match="symlinked or invalid directory"):
            write_class_file(artifact, tempdir)


def test_large_static_data_is_rejected() -> None:
    program = _simple_program()
    program.add_data(IrDataDecl(label="huge", size=(16 * 1024 * 1024) + 1, init=1))

    with pytest.raises(JvmBackendError, match="Total static data exceeds"):
        lower_ir_to_jvm_class_file(
            program=program,
            config=JvmBackendConfig(class_name="TooMuchData"),
        )


@pytest.mark.skipif(
    "GRAALVM_HOME" not in os.environ,
    reason="GRAALVM_HOME is not set for local runtime smoke tests",
)
def test_generated_brainfuck_class_runs_on_graalvm_java() -> None:
    artifact = lower_ir_to_jvm_class_file(
        program=_brainfuck_ir_for_output_a(),
        config=JvmBackendConfig(class_name="BrainfuckA"),
    )

    graalvm_home = Path(os.environ["GRAALVM_HOME"])
    java_bin = graalvm_home / "bin" / "java"

    with tempfile.TemporaryDirectory() as tempdir:
        write_class_file(artifact, tempdir)
        result = subprocess.run(
            [str(java_bin), "-cp", tempdir, "BrainfuckA"],
            check=True,
            capture_output=True,
        )

    assert result.stdout == b"A"


@pytest.mark.skipif(
    "GRAALVM_HOME" not in os.environ,
    reason="GRAALVM_HOME is not set for local runtime smoke tests",
)
def test_generated_brainfuck_class_can_build_native_image() -> None:
    artifact = lower_ir_to_jvm_class_file(
        program=_brainfuck_ir_for_output_a(),
        config=JvmBackendConfig(class_name="BrainfuckNativeA"),
    )

    graalvm_home = Path(os.environ["GRAALVM_HOME"])
    native_image_bin = graalvm_home / "bin" / "native-image"

    with tempfile.TemporaryDirectory() as tempdir:
        write_class_file(artifact, tempdir)
        subprocess.run(
            [
                str(native_image_bin),
                "-cp",
                tempdir,
                "BrainfuckNativeA",
                "brainfuck-native-a",
            ],
            cwd=tempdir,
            check=True,
            capture_output=True,
        )
        result = subprocess.run(
            [str(Path(tempdir) / "brainfuck-native-a")],
            cwd=tempdir,
            check=True,
            capture_output=True,
        )

    assert result.stdout == b"A"


@pytest.mark.skipif(
    "GRAALVM_HOME" not in os.environ,
    reason="GRAALVM_HOME is not set for local runtime smoke tests",
)
def test_generated_nib_class_runs_on_graalvm_java_via_driver() -> None:
    artifact = lower_nib_program(
        "fn main() -> u4 { return 7; }",
        "NibReturn",
    )

    graalvm_home = Path(os.environ["GRAALVM_HOME"])
    java_bin = graalvm_home / "bin" / "java"
    javac_bin = graalvm_home / "bin" / "javac"

    with tempfile.TemporaryDirectory() as tempdir:
        write_class_file(artifact, tempdir)
        _write_driver_source(Path(tempdir), "NibReturn")
        subprocess.run(
            [str(javac_bin), "-cp", tempdir, "InvokeNib.java"],
            cwd=tempdir,
            check=True,
            capture_output=True,
        )
        result = subprocess.run(
            [str(java_bin), "-cp", tempdir, "InvokeNib"],
            cwd=tempdir,
            check=True,
            capture_output=True,
        )

    assert result.stdout == b"7"


@pytest.mark.skipif(
    "GRAALVM_HOME" not in os.environ,
    reason="GRAALVM_HOME is not set for local runtime smoke tests",
)
def test_generated_nib_class_can_build_native_image_via_driver() -> None:
    artifact = lower_nib_program(
        "fn main() -> u4 { return 7; }",
        "NibNativeReturn",
    )

    graalvm_home = Path(os.environ["GRAALVM_HOME"])
    javac_bin = graalvm_home / "bin" / "javac"
    native_image_bin = graalvm_home / "bin" / "native-image"

    with tempfile.TemporaryDirectory() as tempdir:
        write_class_file(artifact, tempdir)
        _write_driver_source(Path(tempdir), "NibNativeReturn")
        subprocess.run(
            [str(javac_bin), "-cp", tempdir, "InvokeNib.java"],
            cwd=tempdir,
            check=True,
            capture_output=True,
        )
        subprocess.run(
            [
                str(native_image_bin),
                "-cp",
                tempdir,
                "InvokeNib",
                "invoke-nib",
            ],
            cwd=tempdir,
            check=True,
            capture_output=True,
        )
        result = subprocess.run(
            [str(Path(tempdir) / "invoke-nib")],
            cwd=tempdir,
            check=True,
            capture_output=True,
        )

    assert result.stdout == b"7"


def _simple_program():
    from compiler_ir import (
        IrImmediate,
        IrInstruction,
        IrLabel,
        IrOp,
        IrProgram,
        IrRegister,
    )

    program = IrProgram(entry_label="_start")
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(0)], id=0)
    )
    program.add_instruction(IrInstruction(IrOp.HALT, [], id=1))
    return program


def _brainfuck_ir_for_output_a():
    from brainfuck import parse_brainfuck
    from brainfuck_ir_compiler import compile_brainfuck, release_config

    source = "+" * 65 + "."
    ast = parse_brainfuck(source)
    return compile_brainfuck(ast, "output_a.bf", release_config()).program


def _write_driver_source(tempdir: Path, class_name: str) -> Path:
    driver_source = "\n".join(
        [
            "public final class InvokeNib {",
            "    public static void main(String[] args) {",
            f"        System.out.print({class_name}._start());",
            "    }",
            "}",
        ]
    )
    driver_path = tempdir / "InvokeNib.java"
    driver_path.write_text(driver_source, encoding="utf-8")
    return driver_path
