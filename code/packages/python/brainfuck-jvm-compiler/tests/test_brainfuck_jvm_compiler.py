from __future__ import annotations

import os
import subprocess
from pathlib import Path

import pytest

from brainfuck_jvm_compiler import (
    BrainfuckJvmCompiler,
    PackageError,
    compile_source,
    pack_source,
    write_class_file,
)


def test_compile_source_returns_pipeline_artifacts() -> None:
    result = compile_source("+.")

    assert result.raw_ir is not None
    assert result.optimized_ir is not None
    assert result.class_bytes
    assert result.class_name == "BrainfuckProgram"
    assert result.parsed_class.this_class_name == "BrainfuckProgram"
    assert result.parsed_class.find_method("_start", "()I") is not None


def test_pack_source_is_alias_for_compile_source() -> None:
    compiled = compile_source("+.")
    packed = pack_source("+.")

    assert packed.class_bytes == compiled.class_bytes
    assert packed.class_name == compiled.class_name


def test_write_class_file_writes_output(tmp_path: Path) -> None:
    result = write_class_file("+.", tmp_path)
    assert result.class_file_path == tmp_path / "BrainfuckProgram.class"
    assert result.class_file_path.read_bytes() == result.class_bytes


def test_compiler_instance_honors_custom_filename_and_class_name() -> None:
    result = BrainfuckJvmCompiler(
        filename="hello.bf",
        class_name="demo.HelloBrainfuck",
    ).compile_source("+")

    assert result.filename == "hello.bf"
    assert result.class_name == "demo.HelloBrainfuck"
    assert result.parsed_class.this_class_name == "demo/HelloBrainfuck"


def test_invalid_class_name_raises_stage_labeled_error() -> None:
    with pytest.raises(PackageError) as exc_info:
        compile_source("+", class_name=".BadName")

    assert exc_info.value.stage == "lower-jvm"


@pytest.mark.skipif(
    "GRAALVM_HOME" not in os.environ,
    reason="GRAALVM_HOME is not set for local runtime smoke tests",
)
def test_compiled_program_runs_on_graalvm_java(tmp_path: Path) -> None:
    result = write_class_file("+" * 65 + ".", tmp_path, class_name="BrainfuckRuntime")
    graalvm_home = Path(os.environ["GRAALVM_HOME"])
    java_bin = graalvm_home / "bin" / "java"

    completed = subprocess.run(
        [str(java_bin), "-cp", tmp_path, "BrainfuckRuntime"],
        check=True,
        capture_output=True,
    )

    assert result.class_file_path == tmp_path / "BrainfuckRuntime.class"
    assert completed.stdout == b"A"
