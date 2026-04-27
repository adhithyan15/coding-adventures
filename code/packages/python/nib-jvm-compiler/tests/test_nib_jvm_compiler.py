from __future__ import annotations

import os
import subprocess
from pathlib import Path

import pytest

from nib_jvm_compiler import (
    NibJvmCompiler,
    PackageError,
    compile_source,
    pack_source,
    write_class_file,
)


def test_compile_source_returns_pipeline_artifacts() -> None:
    source = """
fn answer() -> u4 {
    return 7;
}
"""
    result = compile_source(source)

    assert result.raw_ir is not None
    assert result.optimized_ir is not None
    assert result.class_bytes
    assert result.class_name == "NibProgram"
    assert result.parsed_class.this_class_name == "NibProgram"
    assert result.parsed_class.find_method("_start", "()I") is not None


def test_pack_source_is_alias_for_compile_source() -> None:
    source = "fn answer() -> u4 { return 7; }"
    compiled = compile_source(source)
    packed = pack_source(source)

    assert packed.class_bytes == compiled.class_bytes
    assert packed.class_name == compiled.class_name


def test_write_class_file_writes_output(tmp_path: Path) -> None:
    result = write_class_file("fn main() -> u4 { return 7; }", tmp_path)

    assert result.class_file_path == tmp_path / "NibProgram.class"
    assert result.class_file_path.read_bytes() == result.class_bytes


def test_compiler_instance_honors_class_name_and_main_wrapper_flag() -> None:
    result = NibJvmCompiler(
        class_name="demo.CustomNib",
        emit_main_wrapper=False,
    ).compile_source("fn main() -> u4 { return 7; }")

    assert result.class_name == "demo.CustomNib"
    assert result.parsed_class.this_class_name == "demo/CustomNib"
    assert result.parsed_class.find_method("main", "([Ljava/lang/String;)V") is None


def test_type_error_raises_package_error() -> None:
    source = "fn main() { let x: bool = 1 +% 2; }"

    with pytest.raises(PackageError) as exc_info:
        compile_source(source)

    assert exc_info.value.stage == "type-check"


def test_parse_error_raises_package_error() -> None:
    with pytest.raises(PackageError) as exc_info:
        NibJvmCompiler().compile_source("fn main(")

    assert exc_info.value.stage == "parse"


@pytest.mark.skipif(
    "GRAALVM_HOME" not in os.environ,
    reason="GRAALVM_HOME is not set for local runtime smoke tests",
)
def test_compiled_program_runs_on_graalvm_java_via_driver(tmp_path: Path) -> None:
    result = write_class_file(
        "fn main() -> u4 { return 7; }",
        tmp_path,
        class_name="NibRuntime",
    )
    graalvm_home = Path(os.environ["GRAALVM_HOME"])
    java_bin = graalvm_home / "bin" / "java"
    javac_bin = graalvm_home / "bin" / "javac"

    driver_path = _write_driver_source(tmp_path, "NibRuntime")
    subprocess.run(
        [str(javac_bin), "-cp", tmp_path, driver_path.name],
        cwd=tmp_path,
        check=True,
        capture_output=True,
    )
    completed = subprocess.run(
        [str(java_bin), "-cp", tmp_path, "InvokeNib"],
        cwd=tmp_path,
        check=True,
        capture_output=True,
    )

    assert result.class_file_path == tmp_path / "NibRuntime.class"
    assert completed.stdout == b"7"


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
