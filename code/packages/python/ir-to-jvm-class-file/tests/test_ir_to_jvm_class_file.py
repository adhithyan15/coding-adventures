from __future__ import annotations

import os
import subprocess
import tempfile
from pathlib import Path

import pytest
from compiler_ir import IrDataDecl, IrImmediate, IrInstruction, IrLabel, IrOp, IrProgram, IrRegister
from jvm_class_file import parse_class_file

from ir_to_jvm_class_file import (
    JvmBackendConfig,
    JVMClassArtifact,
    lower_ir_to_jvm_class_file,
    validate_for_jvm,
    write_class_file,
)
from ir_to_jvm_class_file.backend import JvmBackendError, _JVM_SUPPORTED_OPCODES


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


# ---------------------------------------------------------------------------
# Shared helpers for validator tests
# ---------------------------------------------------------------------------

def _instr(op: IrOp, *operands) -> IrInstruction:
    return IrInstruction(opcode=op, operands=list(operands), id=-1)


def _reg(n: int) -> IrRegister:
    return IrRegister(index=n)


def _imm(v: int) -> IrImmediate:
    return IrImmediate(value=v)


def _lbl(name: str) -> IrLabel:
    return IrLabel(name=name)


def _prog(*instrs: IrInstruction) -> IrProgram:
    prog = IrProgram(entry_label="_start")
    for instr in instrs:
        prog.add_instruction(instr)
    return prog


# ---------------------------------------------------------------------------
# Validator tests
# ---------------------------------------------------------------------------


class TestValidateForJvm:
    """Tests for validate_for_jvm() — the pre-flight IR inspector.

    The JVM V1 backend checks three rules before producing any bytecode:

    1. **Opcode support** — every opcode must appear in the supported set.
       All IrOp values are currently supported, including the five bitwise
       opcodes (OR, OR_IMM, XOR, XOR_IMM, NOT) added in compiler-ir v0.3.0.
       These are lowered using native JVM ``ior`` (0x80) / ``ixor`` (0x82)
       bytecodes; NOT emits ``iconst_m1`` + ``ixor`` to flip all 32 bits.

    2. **Constant range** — every IrImmediate in LOAD_IMM or ADD_IMM must
       fit in a JVM 32-bit signed integer (−2 147 483 648 to 2 147 483 647).

    3. **SYSCALL number** — only SYSCALL 1 (write byte) and SYSCALL 4
       (read byte) are wired up in the V1 JVM backend.
    """

    def test_valid_program_returns_no_errors(self) -> None:
        """A well-formed program with supported opcodes and in-range
        constants produces zero validation errors."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(65)),
            _instr(IrOp.ADD_IMM,  _reg(2), _reg(1), _imm(1)),
            _instr(IrOp.SYSCALL,  _imm(1), _reg(0)),
            _instr(IrOp.HALT),
        )
        assert validate_for_jvm(program) == []

    def test_empty_program_returns_no_errors(self) -> None:
        """An empty IR program (no instructions) is trivially valid."""
        prog = IrProgram(entry_label="_start")
        assert validate_for_jvm(prog) == []

    # ── Rule 1: opcode support ───────────────────────────────────────────────
    # The V1 JVM backend handles the subset listed in _JVM_SUPPORTED_OPCODES.
    # Shared IR may grow beyond that; those additions should be explicitly
    # rejected until the JVM backend learns them.

    def test_all_supported_opcodes_pass_opcode_check(self) -> None:
        """Every opcode claimed by the JVM backend passes rule 1 validation."""
        for op in _JVM_SUPPORTED_OPCODES:
            program = _prog(_instr(op))
            errors = validate_for_jvm(program)
            # Filter out any errors that are not about opcode support —
            # some opcodes may trigger rule 2 or 3 errors with dummy
            # operands, which is fine.  We only care that rule 1 never fires.
            rule1_errors = [e for e in errors if "unsupported opcode" in e]
            assert rule1_errors == [], (
                f"IrOp.{op.name} was unexpectedly rejected by the JVM "
                f"opcode-support check: {rule1_errors}"
            )

    def test_new_f64_opcodes_are_explicitly_rejected(self) -> None:
        """The V1 JVM backend still rejects the new floating-point IR ops."""
        unsupported = {
            IrOp.LOAD_F64_IMM,
            IrOp.LOAD_F64,
            IrOp.STORE_F64,
            IrOp.F64_ADD,
            IrOp.F64_SUB,
            IrOp.F64_MUL,
            IrOp.F64_DIV,
            IrOp.F64_CMP_EQ,
            IrOp.F64_CMP_NE,
            IrOp.F64_CMP_LT,
            IrOp.F64_CMP_GT,
            IrOp.F64_CMP_LE,
            IrOp.F64_CMP_GE,
            IrOp.F64_FROM_I32,
        }
        for op in unsupported:
            program = _prog(_instr(op))
            errors = validate_for_jvm(program)
            assert any("unsupported opcode" in error for error in errors), (
                f"IrOp.{op.name} should remain unsupported in the V1 JVM backend"
            )

    # ── Bitwise opcode integration tests ─────────────────────────────────────
    # These tests verify that OR, OR_IMM, XOR, XOR_IMM, and NOT are accepted
    # by the validator (rule 1 passes) and that the lowered bytecode can be
    # parsed as a well-formed class file.

    def test_or_opcode_accepted_by_validator(self) -> None:
        """IrOp.OR passes the opcode-support check."""
        program = _prog(_instr(IrOp.OR, _reg(2), _reg(0), _reg(1)))
        errors = [e for e in validate_for_jvm(program) if "unsupported opcode" in e]
        assert errors == []

    def test_xor_opcode_accepted_by_validator(self) -> None:
        """IrOp.XOR passes the opcode-support check."""
        program = _prog(_instr(IrOp.XOR, _reg(2), _reg(0), _reg(1)))
        errors = [e for e in validate_for_jvm(program) if "unsupported opcode" in e]
        assert errors == []

    def test_not_opcode_accepted_by_validator(self) -> None:
        """IrOp.NOT passes the opcode-support check."""
        program = _prog(_instr(IrOp.NOT, _reg(1), _reg(0)))
        errors = [e for e in validate_for_jvm(program) if "unsupported opcode" in e]
        assert errors == []

    def test_or_imm_opcode_accepted_by_validator(self) -> None:
        """IrOp.OR_IMM passes the opcode-support check."""
        program = _prog(_instr(IrOp.OR_IMM, _reg(1), _reg(0), _imm(0xFF)))
        errors = [e for e in validate_for_jvm(program) if "unsupported opcode" in e]
        assert errors == []

    def test_xor_imm_opcode_accepted_by_validator(self) -> None:
        """IrOp.XOR_IMM passes the opcode-support check."""
        program = _prog(_instr(IrOp.XOR_IMM, _reg(1), _reg(0), _imm(0xAA)))
        errors = [e for e in validate_for_jvm(program) if "unsupported opcode" in e]
        assert errors == []

    def _or_program(self, a: int, b: int) -> IrProgram:
        """Build a minimal IR program that ORs two immediates and returns the
        result in r1 (the ABI return register)."""
        return _prog(
            _instr(IrOp.LABEL,    _lbl("_start")),
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(a)),
            _instr(IrOp.LOAD_IMM, _reg(3), _imm(b)),
            _instr(IrOp.OR,       _reg(1), _reg(2), _reg(3)),
            _instr(IrOp.HALT),
        )

    def _xor_program(self, a: int, b: int) -> IrProgram:
        """Build a minimal IR program that XORs two immediates and returns the
        result in r1."""
        return _prog(
            _instr(IrOp.LABEL,    _lbl("_start")),
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(a)),
            _instr(IrOp.LOAD_IMM, _reg(3), _imm(b)),
            _instr(IrOp.XOR,      _reg(1), _reg(2), _reg(3)),
            _instr(IrOp.HALT),
        )

    def _not_program(self, a: int) -> IrProgram:
        """Build a minimal IR program that NOTs an immediate and returns the
        result in r1."""
        return _prog(
            _instr(IrOp.LABEL,    _lbl("_start")),
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(a)),
            _instr(IrOp.NOT,      _reg(1), _reg(2)),
            _instr(IrOp.HALT),
        )

    def _or_imm_program(self, a: int, imm: int) -> IrProgram:
        """Build a minimal IR program that ORs a register with an immediate."""
        return _prog(
            _instr(IrOp.LABEL,    _lbl("_start")),
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(a)),
            _instr(IrOp.OR_IMM,   _reg(1), _reg(2), _imm(imm)),
            _instr(IrOp.HALT),
        )

    def _xor_imm_program(self, a: int, imm: int) -> IrProgram:
        """Build a minimal IR program that XORs a register with an immediate."""
        return _prog(
            _instr(IrOp.LABEL,    _lbl("_start")),
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(a)),
            _instr(IrOp.XOR_IMM,  _reg(1), _reg(2), _imm(imm)),
            _instr(IrOp.HALT),
        )

    def test_or_lowers_to_parseable_class_file(self) -> None:
        """IrOp.OR lowers to a structurally valid JVM class file."""
        program = self._or_program(0b1010, 0b0101)
        artifact = lower_ir_to_jvm_class_file(
            program, JvmBackendConfig(class_name="OrTest")
        )
        from jvm_class_file import parse_class_file
        parsed = parse_class_file(artifact.class_bytes)
        assert parsed.find_method("_start", "()I") is not None

    def test_xor_lowers_to_parseable_class_file(self) -> None:
        """IrOp.XOR lowers to a structurally valid JVM class file."""
        program = self._xor_program(0b1111, 0b1010)
        artifact = lower_ir_to_jvm_class_file(
            program, JvmBackendConfig(class_name="XorTest")
        )
        from jvm_class_file import parse_class_file
        parsed = parse_class_file(artifact.class_bytes)
        assert parsed.find_method("_start", "()I") is not None

    def test_not_lowers_to_parseable_class_file(self) -> None:
        """IrOp.NOT lowers to a structurally valid JVM class file."""
        program = self._not_program(0)
        artifact = lower_ir_to_jvm_class_file(
            program, JvmBackendConfig(class_name="NotTest")
        )
        from jvm_class_file import parse_class_file
        parsed = parse_class_file(artifact.class_bytes)
        assert parsed.find_method("_start", "()I") is not None

    def test_or_imm_lowers_to_parseable_class_file(self) -> None:
        """IrOp.OR_IMM lowers to a structurally valid JVM class file."""
        program = self._or_imm_program(0b1010, 0b0101)
        artifact = lower_ir_to_jvm_class_file(
            program, JvmBackendConfig(class_name="OrImmTest")
        )
        from jvm_class_file import parse_class_file
        parsed = parse_class_file(artifact.class_bytes)
        assert parsed.find_method("_start", "()I") is not None

    def test_xor_imm_lowers_to_parseable_class_file(self) -> None:
        """IrOp.XOR_IMM lowers to a structurally valid JVM class file."""
        program = self._xor_imm_program(0b1111, 0b1010)
        artifact = lower_ir_to_jvm_class_file(
            program, JvmBackendConfig(class_name="XorImmTest")
        )
        from jvm_class_file import parse_class_file
        parsed = parse_class_file(artifact.class_bytes)
        assert parsed.find_method("_start", "()I") is not None

    def test_or_bytecode_is_present(self) -> None:
        """The OR instruction emits the JVM ior opcode (0x80) in the method body."""
        program = self._or_program(0b1010, 0b0101)
        artifact = lower_ir_to_jvm_class_file(
            program, JvmBackendConfig(class_name="OrBytecodeCheck")
        )
        # 0x80 is the ior opcode — it must appear somewhere in the class bytes.
        assert b"\x80" in artifact.class_bytes

    def test_xor_bytecode_is_present(self) -> None:
        """The XOR instruction emits the JVM ixor opcode (0x82) in the method body."""
        program = self._xor_program(0b1111, 0b1010)
        artifact = lower_ir_to_jvm_class_file(
            program, JvmBackendConfig(class_name="XorBytecodeCheck")
        )
        # 0x82 is the ixor opcode — it must appear somewhere in the class bytes.
        assert b"\x82" in artifact.class_bytes

    def test_not_uses_iconst_m1_and_ixor(self) -> None:
        """The NOT instruction emits iconst_m1 (0x02) followed by ixor (0x82).

        NOT(x) = x XOR -1.  In two's-complement, XOR-ing with all-ones flips
        every bit, which is the correct bitwise NOT for a 32-bit integer.
        iconst_m1 pushes -1 (= 0xFFFFFFFF as unsigned bits) onto the operand
        stack, and ixor applies the flip."""
        program = self._not_program(0)
        artifact = lower_ir_to_jvm_class_file(
            program, JvmBackendConfig(class_name="NotBytecodeCheck")
        )
        # Both iconst_m1 and ixor must appear in the generated bytecode.
        assert b"\x02" in artifact.class_bytes   # iconst_m1
        assert b"\x82" in artifact.class_bytes   # ixor

    def test_or_imm_bytecode_is_present(self) -> None:
        """OR_IMM emits ior (0x80) in the generated class bytes."""
        program = self._or_imm_program(0b1010, 0b0101)
        artifact = lower_ir_to_jvm_class_file(
            program, JvmBackendConfig(class_name="OrImmBytecodeCheck")
        )
        assert b"\x80" in artifact.class_bytes

    def test_xor_imm_bytecode_is_present(self) -> None:
        """XOR_IMM emits ixor (0x82) in the generated class bytes."""
        program = self._xor_imm_program(0b1111, 0b1010)
        artifact = lower_ir_to_jvm_class_file(
            program, JvmBackendConfig(class_name="XorImmBytecodeCheck")
        )
        assert b"\x82" in artifact.class_bytes

    # ── Rule 2: constant range ────────────────────────────────────────────────

    def test_load_imm_max_valid_constant_accepted(self) -> None:
        """2 147 483 647 (= 2^31 − 1) is the largest valid JVM int constant."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(2_147_483_647)),
            _instr(IrOp.HALT),
        )
        assert validate_for_jvm(program) == []

    def test_load_imm_min_valid_constant_accepted(self) -> None:
        """−2 147 483 648 (= −2^31) is the smallest valid JVM int constant."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(-2_147_483_648)),
            _instr(IrOp.HALT),
        )
        assert validate_for_jvm(program) == []

    def test_load_imm_one_above_max_rejected(self) -> None:
        """2 147 483 648 is exactly one above the 32-bit maximum."""
        program = _prog(_instr(IrOp.LOAD_IMM, _reg(1), _imm(2_147_483_648)))
        errors = validate_for_jvm(program)
        assert len(errors) == 1
        assert "2,147,483,648" in errors[0]
        assert "LOAD_IMM" in errors[0]

    def test_load_imm_one_below_min_rejected(self) -> None:
        """−2 147 483 649 is exactly one below the 32-bit minimum."""
        program = _prog(_instr(IrOp.LOAD_IMM, _reg(1), _imm(-2_147_483_649)))
        errors = validate_for_jvm(program)
        assert len(errors) == 1
        assert "-2,147,483,649" in errors[0]
        assert "LOAD_IMM" in errors[0]

    def test_add_imm_overflow_rejected(self) -> None:
        """ADD_IMM with an immediate that overflows 32 bits is also caught."""
        program = _prog(
            _instr(IrOp.ADD_IMM, _reg(2), _reg(1), _imm(3_000_000_000)),
        )
        errors = validate_for_jvm(program)
        assert len(errors) == 1
        assert "3,000,000,000" in errors[0]
        assert "ADD_IMM" in errors[0]

    def test_large_negative_add_imm_rejected(self) -> None:
        """ADD_IMM with a very large negative immediate that underflows 32 bits."""
        program = _prog(
            _instr(IrOp.ADD_IMM, _reg(2), _reg(1), _imm(-2_147_483_649)),
        )
        errors = validate_for_jvm(program)
        assert len(errors) == 1
        assert "ADD_IMM" in errors[0]

    # ── Rule 3: SYSCALL number ────────────────────────────────────────────────

    def test_syscall_1_accepted(self) -> None:
        """SYSCALL 1 (write byte / print char) is supported in V1 JVM backend."""
        program = _prog(
            _instr(IrOp.SYSCALL, _imm(1), _reg(0)),
            _instr(IrOp.HALT),
        )
        assert validate_for_jvm(program) == []

    def test_syscall_4_accepted(self) -> None:
        """SYSCALL 4 (read byte from stdin) is supported in V1 JVM backend."""
        program = _prog(
            _instr(IrOp.SYSCALL, _imm(4), _reg(0)),
            _instr(IrOp.HALT),
        )
        assert validate_for_jvm(program) == []

    def test_syscall_unknown_rejected(self) -> None:
        """A SYSCALL number other than 1 or 4 is not wired up in the JVM backend."""
        program = _prog(_instr(IrOp.SYSCALL, _imm(99), _reg(0)))
        errors = validate_for_jvm(program)
        assert len(errors) == 1
        assert "unsupported SYSCALL" in errors[0]
        assert "99" in errors[0]

    def test_syscall_10_rejected(self) -> None:
        """SYSCALL 10 (WASM exit convention) is not wired in the JVM backend;
        HALT is the correct way to terminate a JVM-targeted program."""
        program = _prog(_instr(IrOp.SYSCALL, _imm(10), _reg(0)))
        errors = validate_for_jvm(program)
        assert len(errors) == 1
        assert "unsupported SYSCALL" in errors[0]

    # ── Multiple errors ───────────────────────────────────────────────────────

    def test_multiple_violations_all_reported(self) -> None:
        """The validator accumulates every violation rather than stopping at
        the first, so callers get a complete picture of what must change."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(5_000_000_000)),   # overflows
            _instr(IrOp.SYSCALL,  _imm(99), _reg(0)),               # bad syscall
            _instr(IrOp.HALT),
        )
        errors = validate_for_jvm(program)
        assert len(errors) == 2

    # ── Integration: lower_ir_to_jvm_class_file calls validate first ─────────

    def test_compile_raises_on_oversized_constant(self) -> None:
        """lower_ir_to_jvm_class_file must raise JvmBackendError (not silently
        corrupt data) when the IR contains a constant that overflows 32 bits."""
        program = _prog(_instr(IrOp.LOAD_IMM, _reg(1), _imm(5_000_000_000)))
        with pytest.raises(JvmBackendError, match="pre-flight"):
            lower_ir_to_jvm_class_file(program, JvmBackendConfig(class_name="Bad"))

    def test_compile_raises_on_unsupported_syscall(self) -> None:
        """lower_ir_to_jvm_class_file raises JvmBackendError for an unknown
        SYSCALL number before generating any bytecode."""
        program = _prog(
            _instr(IrOp.SYSCALL, _imm(42), _reg(0)),
            _instr(IrOp.HALT),
        )
        with pytest.raises(JvmBackendError, match="pre-flight"):
            lower_ir_to_jvm_class_file(program, JvmBackendConfig(class_name="Bad"))
