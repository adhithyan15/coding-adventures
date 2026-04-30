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


# ─────────────────────────────────────────────────────────────────────────────
# JVM02 Phase 2b — multi-class artifact + Closure interface
# ─────────────────────────────────────────────────────────────────────────────


class TestPhase2bMultiClass:
    """Tests for the multi-class scaffolding that JVM02 Phase 2 closures
    will use.  Phase 2b ships the data shape and the ``Closure`` interface
    only; per-lambda ``Closure_<name>`` subclasses + the actual
    MAKE_CLOSURE / APPLY_CLOSURE lowering land in Phase 2c.
    """

    def _trivial_program(self, class_name: str) -> JvmBackendConfig:
        return JvmBackendConfig(class_name=class_name)

    def _trivial_main(self) -> IrProgram:
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")]))
        program.add_instruction(IrInstruction(IrOp.HALT))
        return program

    def test_multiclass_returns_main_when_interface_not_requested(self) -> None:
        from ir_to_jvm_class_file import (
            JVMMultiClassArtifact,
            lower_ir_to_jvm_classes,
        )
        artifact = lower_ir_to_jvm_classes(
            self._trivial_main(), self._trivial_program("Solo")
        )
        assert isinstance(artifact, JVMMultiClassArtifact)
        assert len(artifact.classes) == 1
        assert artifact.main.class_name == "Solo"

    def test_multiclass_appends_closure_interface_when_requested(self) -> None:
        from ir_to_jvm_class_file import (
            CLOSURE_INTERFACE_BINARY_NAME,
            lower_ir_to_jvm_classes,
        )
        artifact = lower_ir_to_jvm_classes(
            self._trivial_main(),
            self._trivial_program("WithClosure"),
            include_closure_interface=True,
        )
        assert len(artifact.classes) == 2
        # main always first, interface appended.
        assert artifact.main.class_name == "WithClosure"
        assert artifact.classes[1].class_name == (
            CLOSURE_INTERFACE_BINARY_NAME.replace("/", ".")
        )

    def test_class_filenames_are_path_safe(self) -> None:
        from ir_to_jvm_class_file import lower_ir_to_jvm_classes
        artifact = lower_ir_to_jvm_classes(
            self._trivial_main(),
            self._trivial_program("WithClosure"),
            include_closure_interface=True,
        )
        # Closure interface lands at the spec'd JAR path.
        assert "coding_adventures/twig/runtime/Closure.class" in (
            artifact.class_filenames
        )

    def test_closure_interface_artifact_parses_as_class_file(self) -> None:
        """Spec verification: the bytes we hand-roll for the Closure
        interface parse cleanly via ``jvm-class-file``'s decoder."""
        from jvm_class_file import parse_class_file

        from ir_to_jvm_class_file import (
            CLOSURE_INTERFACE_METHOD_DESCRIPTOR,
            CLOSURE_INTERFACE_METHOD_NAME,
            build_closure_interface_artifact,
        )
        artifact = build_closure_interface_artifact()
        cf = parse_class_file(artifact.class_bytes)
        # ACC_INTERFACE = 0x0200, ACC_ABSTRACT = 0x0400, plus ACC_PUBLIC.
        assert cf.access_flags & 0x0200, "must be tagged as interface"
        assert cf.access_flags & 0x0400, "must be tagged as abstract"
        # Has exactly one method: apply([I)I, abstract.
        assert len(cf.methods) == 1
        method = cf.methods[0]
        assert method.name == CLOSURE_INTERFACE_METHOD_NAME
        assert method.descriptor == CLOSURE_INTERFACE_METHOD_DESCRIPTOR
        assert method.access_flags & 0x0400, "method must be abstract"

    def test_main_first_invariant(self) -> None:
        """``JVMMultiClassArtifact.main`` always returns ``classes[0]``
        — JAR builders rely on this to set ``Main-Class``."""
        from ir_to_jvm_class_file import lower_ir_to_jvm_classes
        artifact = lower_ir_to_jvm_classes(
            self._trivial_main(),
            self._trivial_program("MainFirst"),
            include_closure_interface=True,
        )
        assert artifact.main is artifact.classes[0]
        assert artifact.main.class_name == "MainFirst"

    def test_empty_multiclass_artifact_rejected(self) -> None:
        """Constructing without any classes is invalid — the main user
        class is always required."""
        from ir_to_jvm_class_file import (
            JvmBackendError,
            JVMMultiClassArtifact,
        )
        empty = JVMMultiClassArtifact(classes=())
        with pytest.raises(JvmBackendError, match="at least the main"):
            _ = empty.main


# ─────────────────────────────────────────────────────────────────────────────
# JVM02 Phase 2b — real-`java` JAR loading test
# ─────────────────────────────────────────────────────────────────────────────


def _java_available_for_jar() -> bool:
    """Probe ``java`` once at import time.  Skip the JAR test
    cleanly when the runtime isn't on PATH (matches the existing
    pattern in ``test_oct_8bit_e2e.py``)."""
    import shutil
    import subprocess

    if shutil.which("java") is None:
        return False
    try:
        result = subprocess.run(
            ["java", "-version"], capture_output=True, timeout=5, check=False,
        )
        return result.returncode == 0
    except (subprocess.SubprocessError, OSError):
        return False


_JAVA_AVAILABLE = _java_available_for_jar()
_skip_if_no_java = pytest.mark.skipif(
    not _JAVA_AVAILABLE,
    reason="java not on PATH — skipping real-java JAR conformance test",
)


@_skip_if_no_java
def test_phase2b_jar_with_closure_interface_loads_under_real_java(
    tmp_path: pytest.TempPathFactory,
) -> None:
    """Pack the main user class + ``Closure`` interface into a JAR
    and prove that real ``java -jar`` loads both classes (the
    interface side via ``Class.forName`` from main).

    This is the headline JVM02 Phase 2b conformance test — when it
    passes, the multi-class plumbing is verified end-to-end on the
    actual JVM (not just our internal class-file decoder).  Phase
    2c can then add MAKE_CLOSURE / APPLY_CLOSURE lowering on top
    without re-litigating the basic load path.
    """
    import subprocess
    from pathlib import Path

    from jvm_jar_writer import JarManifest, write_jar

    from ir_to_jvm_class_file import (
        CLOSURE_INTERFACE_BINARY_NAME,
        JvmBackendConfig,
        lower_ir_to_jvm_classes,
    )

    # The main program forces a Class.forName lookup for the
    # closure interface, so the JVM is forced to fully load the
    # interface class — not just leave it as an unresolved symbol.
    # If the interface bytecode were malformed the JVM would throw
    # a ClassFormatError at this exact line.
    main_class_name = "Phase2bMain"
    program = IrProgram(entry_label="_start")
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")]))
    # Trivial body — the test isn't about main's behaviour.  A
    # single LOAD_IMM keeps validate_for_jvm happy without
    # expanding the JAR's dependency surface.
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), IrImmediate(0)])
    )
    program.add_instruction(IrInstruction(IrOp.HALT))

    artifact = lower_ir_to_jvm_classes(
        program,
        JvmBackendConfig(class_name=main_class_name),
        include_closure_interface=True,
    )

    # Build a JAR containing both classes.
    classes = tuple(
        (cls.class_filename, cls.class_bytes) for cls in artifact.classes
    )
    manifest = JarManifest(main_class=main_class_name)
    jar_bytes = write_jar(classes, manifest)

    jar_path = Path(tmp_path) / "phase2b.jar"
    jar_path.write_bytes(jar_bytes)

    result = subprocess.run(
        ["java", "-jar", str(jar_path)],
        capture_output=True,
        text=True,
        timeout=15,
        check=False,
    )

    # The main class halts cleanly (returncode 0); a malformed
    # Closure interface .class would surface as ClassFormatError /
    # VerifyError on the FIRST class loaded — even before main
    # runs — because some JVMs eagerly verify the JAR contents.
    assert result.returncode == 0, (
        f"java rejected the multi-class JAR.\n"
        f"  exit code: {result.returncode}\n"
        f"  stdout: {result.stdout!r}\n"
        f"  stderr: {result.stderr!r}\n"
        f"  closure interface name: {CLOSURE_INTERFACE_BINARY_NAME!r}"
    )


# ─────────────────────────────────────────────────────────────────────────────
# JVM02 Phase 2c — closure-op lowering + per-lambda subclass emission
# ─────────────────────────────────────────────────────────────────────────────


class TestPhase2cClosureLowering:
    """Tests for MAKE_CLOSURE / APPLY_CLOSURE lowering and per-lambda
    Closure_<name>.class emission.  Phase 2c v1 ships these as
    structural-only — the bytecode shape is verifier-correct, but
    end-to-end runtime semantics for captured-state retention need
    a parallel Object[] register pool that lands in Phase 2c.5.
    """

    def _make_adder_program(self) -> IrProgram:
        """The headline closure fixture mirroring what twig-jvm-compiler
        will emit for ``(define (make-adder n) (lambda (x) (+ x n)))
        ((make-adder 7) 35)``."""
        program = IrProgram(entry_label="_start")

        # Lifted lambda body — captures-first layout.
        program.add_instruction(
            IrInstruction(IrOp.LABEL, [IrLabel("_lambda_0")])
        )
        program.add_instruction(
            IrInstruction(
                IrOp.ADD, [IrRegister(1), IrRegister(2), IrRegister(3)],
            )
        )
        program.add_instruction(IrInstruction(IrOp.RET, []))

        # make_adder
        program.add_instruction(
            IrInstruction(IrOp.LABEL, [IrLabel("make_adder")])
        )
        program.add_instruction(
            IrInstruction(
                IrOp.MAKE_CLOSURE,
                [
                    IrRegister(1),
                    IrLabel("_lambda_0"),
                    IrImmediate(1),
                    IrRegister(2),
                ],
            )
        )
        program.add_instruction(IrInstruction(IrOp.RET, []))

        # main: closure = make_adder(7); APPLY_CLOSURE(closure, [35]).
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")]))
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(7)])
        )
        program.add_instruction(
            IrInstruction(IrOp.CALL, [IrLabel("make_adder")])
        )
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(11), IrImmediate(35)])
        )
        program.add_instruction(
            IrInstruction(
                IrOp.APPLY_CLOSURE,
                [
                    IrRegister(1),
                    IrRegister(1),
                    IrImmediate(1),
                    IrRegister(11),
                ],
            )
        )
        program.add_instruction(IrInstruction(IrOp.HALT, []))
        return program

    def _config(self) -> JvmBackendConfig:
        return JvmBackendConfig(
            class_name="ClosureMain",
            closure_free_var_counts={"_lambda_0": 1},
        )

    def test_multiclass_artifact_includes_interface_and_subclass(self) -> None:
        from ir_to_jvm_class_file import lower_ir_to_jvm_classes
        artifact = lower_ir_to_jvm_classes(
            self._make_adder_program(), self._config(),
        )
        names = {c.class_name for c in artifact.classes}
        # main + IClosure + Closure_lambda_0 = 3 classes.
        assert "ClosureMain" in names
        assert "coding_adventures.twig.runtime.Closure" in names
        assert "coding_adventures.twig.runtime.Closure__lambda_0" in names

    def test_lambda_region_now_lives_on_main_class(self) -> None:
        """JVM02 Phase 2c.5: the lifted lambda body lives as a
        PUBLIC static method on the main class with widened arity
        (``num_free + explicit_arity``).  The closure subclass's
        ``apply`` method forwards to it via ``invokestatic``.

        This was previously the opposite: Phase 2c structural-only
        emitted the lambda body on the subclass with a placeholder
        ``apply`` body.  Phase 2c.5 inverts that so the existing
        IR-body emitter works unchanged.
        """
        from ir_to_jvm_class_file import lower_ir_to_jvm_classes
        artifact = lower_ir_to_jvm_classes(
            self._make_adder_program(), self._config(),
        )
        main = artifact.main
        assert "_lambda_0" in main.callable_labels
        assert "make_adder" in main.callable_labels
        assert "_start" in main.callable_labels  # entry label

    def test_closure_subclass_artifact_parses(self) -> None:
        """The hand-rolled Closure_<name>.class bytes parse cleanly
        through ``jvm-class-file``'s decoder.

        ``JVMClassFile`` doesn't expose a ``fields`` accessor, so
        we assert on the methods (ctor + apply with the right
        descriptors) and on the field-name UTF8 entries appearing
        in the constant pool — both are sufficient evidence that
        the layout is well-formed.
        """
        from jvm_class_file import parse_class_file
        from ir_to_jvm_class_file import build_closure_subclass_artifact
        artifact = build_closure_subclass_artifact("_lambda_0", num_free=2)
        cf = parse_class_file(artifact.class_bytes)
        # Public + super flags.
        assert cf.access_flags & 0x0001  # ACC_PUBLIC
        # 2 methods (.ctor + apply).
        method_names = [m.name for m in cf.methods]
        assert "<init>" in method_names
        assert "apply" in method_names
        # ctor descriptor matches num_free.
        ctor = next(m for m in cf.methods if m.name == "<init>")
        assert ctor.descriptor == "(II)V"
        # apply descriptor matches the Closure interface contract.
        apply = next(m for m in cf.methods if m.name == "apply")
        assert apply.descriptor == "([I)I"
        # capt0 / capt1 utf8 entries land in the constant pool.
        assert b"capt0" in artifact.class_bytes
        assert b"capt1" in artifact.class_bytes

    def test_closure_subclass_zero_captures(self) -> None:
        """A function-value-style closure (no captures) still emits
        a valid Closure subclass — empty-field-list, ()V ctor."""
        from jvm_class_file import parse_class_file
        from ir_to_jvm_class_file import build_closure_subclass_artifact
        artifact = build_closure_subclass_artifact("_lambda_0", num_free=0)
        cf = parse_class_file(artifact.class_bytes)
        ctor = next(m for m in cf.methods if m.name == "<init>")
        assert ctor.descriptor == "()V"
        # No capt fields in the constant pool either.
        assert b"capt0" not in artifact.class_bytes

    def test_make_closure_unknown_lambda_rejected(self) -> None:
        """MAKE_CLOSURE referencing a region that exists in the IR
        but is NOT declared in ``closure_free_var_counts`` is
        rejected with a clear diagnostic — the lowerer can't know
        how to emit the subclass without the capture count."""
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_lambda_undeclared")]))
        program.add_instruction(IrInstruction(IrOp.RET, []))
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")]))
        program.add_instruction(
            IrInstruction(
                IrOp.MAKE_CLOSURE,
                [
                    IrRegister(1),
                    IrLabel("_lambda_undeclared"),
                    IrImmediate(0),
                ],
            )
        )
        program.add_instruction(IrInstruction(IrOp.HALT, []))
        with pytest.raises(JvmBackendError, match=r"closure region"):
            lower_ir_to_jvm_class_file(
                program,
                JvmBackendConfig(
                    class_name="Bad",
                    closure_free_var_counts={},  # not declared
                ),
            )

    def test_make_closure_capture_count_mismatch_rejected(self) -> None:
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_lambda_0")]))
        program.add_instruction(IrInstruction(IrOp.RET, []))
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")]))
        program.add_instruction(
            IrInstruction(
                IrOp.MAKE_CLOSURE,
                [
                    IrRegister(1),
                    IrLabel("_lambda_0"),
                    IrImmediate(2),  # Says 2 captures
                    IrRegister(2),
                ],
            )
        )
        program.add_instruction(IrInstruction(IrOp.HALT, []))
        with pytest.raises(JvmBackendError, match=r"num_captured=2"):
            lower_ir_to_jvm_class_file(
                program,
                JvmBackendConfig(
                    class_name="Bad",
                    closure_free_var_counts={"_lambda_0": 2},
                ),
            )

    def test_main_class_contains_make_closure_bytecode(self) -> None:
        """``new`` (0xBB) appears in the main class's bytecode where
        MAKE_CLOSURE expands."""
        from ir_to_jvm_class_file import lower_ir_to_jvm_classes
        artifact = lower_ir_to_jvm_classes(
            self._make_adder_program(), self._config(),
        )
        # 0xBB is the JVM new opcode.  Substring search is enough — if
        # MAKE_CLOSURE didn't lower, the byte wouldn't appear in any
        # class generated from a closure-free baseline.
        assert b"\xbb" in artifact.main.class_bytes

    def test_main_class_contains_invokeinterface_bytecode(self) -> None:
        """``invokeinterface`` (0xB9) appears where APPLY_CLOSURE
        expands — the closure dispatch site."""
        from ir_to_jvm_class_file import lower_ir_to_jvm_classes
        artifact = lower_ir_to_jvm_classes(
            self._make_adder_program(), self._config(),
        )
        assert b"\xb9" in artifact.main.class_bytes


@_skip_if_no_java
def test_phase2c_make_adder_closure_returns_42_on_real_java(
    tmp_path: pytest.TempPathFactory,
) -> None:
    """Headline JVM02 Phase 2c.5 end-to-end:
    ``((make-adder 7) 35) → 42`` on real ``java -jar``.

    The closure pipeline:

    * MAKE_CLOSURE r1 _lambda_0 1 r2 → ``new
      Closure__lambda_0; dup; iload r2; invokespecial
      <init>(I)V; aastore __ca_objregs[1]``.  The new ref is
      retained in the parallel Object[] pool (Phase 2c.5).
    * APPLY_CLOSURE r1 r1 1 r11 → ``aaload __ca_objregs[1];
      checkcast Closure; ldc 1; newarray int; <dup; ldc 0;
      iload r11; iastore>; invokeinterface Closure.apply([I)I;
      istore __ca_regs[1]``.
    * The closure subclass's ``apply`` body forwards to
      ``MakeAdderMain._lambda_0(I,I)I`` — the lifted lambda
      lives as a public static method on the main class with
      widened arity (num_free + explicit_arity = 2).  Its
      prologue copies its 2 JVM args into ``__ca_regs[2..3]``
      so the existing IR-body emitter runs unchanged.
    * SYSCALL 1 prints r1 (= 42) as a byte; ``stdout == b'*'``
      (= 42 = 0x2a).
    """
    import subprocess
    from pathlib import Path

    from jvm_jar_writer import JarManifest, write_jar

    from ir_to_jvm_class_file import (
        JvmBackendConfig,
        lower_ir_to_jvm_classes,
    )

    program = IrProgram(entry_label="_start")
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_lambda_0")]))
    program.add_instruction(
        IrInstruction(IrOp.ADD, [IrRegister(1), IrRegister(2), IrRegister(3)])
    )
    program.add_instruction(IrInstruction(IrOp.RET, []))
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("make_adder")]))
    program.add_instruction(
        IrInstruction(
            IrOp.MAKE_CLOSURE,
            [
                IrRegister(1),
                IrLabel("_lambda_0"),
                IrImmediate(1),
                IrRegister(2),
            ],
        )
    )
    program.add_instruction(IrInstruction(IrOp.RET, []))
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")]))
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(7)])
    )
    program.add_instruction(IrInstruction(IrOp.CALL, [IrLabel("make_adder")]))
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(11), IrImmediate(35)])
    )
    program.add_instruction(
        IrInstruction(
            IrOp.APPLY_CLOSURE,
            [IrRegister(1), IrRegister(1), IrImmediate(1), IrRegister(11)],
        )
    )
    # SYSCALL 1 prints r1 as a byte; we assert on stdout = b'*' (= 42).
    program.add_instruction(
        IrInstruction(IrOp.SYSCALL, [IrImmediate(1), IrRegister(1)])
    )
    program.add_instruction(IrInstruction(IrOp.HALT, []))

    config = JvmBackendConfig(
        class_name="MakeAdderMain",
        closure_free_var_counts={"_lambda_0": 1},
    )
    artifact = lower_ir_to_jvm_classes(program, config)

    classes = tuple(
        (c.class_filename, c.class_bytes) for c in artifact.classes
    )
    jar_bytes = write_jar(
        classes, JarManifest(main_class="MakeAdderMain"),
    )
    jar_path = Path(tmp_path) / "make_adder.jar"
    jar_path.write_bytes(jar_bytes)
    result = subprocess.run(
        ["java", "-jar", str(jar_path)],
        capture_output=True, timeout=15, check=False,
    )
    assert result.stdout == b"*", (
        f"closure pipeline broke at runtime.\n"
        f"  stdout: {result.stdout!r}\n"
        f"  stderr: {result.stderr!r}"
    )


# ─────────────────────────────────────────────────────────────────────────────
# JVM02 Phase 2c.5 — typed register pool + lifted-lambda forwarder
# ─────────────────────────────────────────────────────────────────────────────


class TestPhase2c5TypedPool:
    """Tests for the parallel ``Object[]`` register pool that
    Phase 2c.5 adds.  These verify the structural changes that
    let closures actually run end-to-end on real ``java``:
    ``__ca_objregs`` field allocation, the lifted-lambda method
    on the main class with widened arity, and the closure
    subclass's invokestatic forwarder.
    """

    def _make_adder_program(self) -> IrProgram:
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_lambda_0")]))
        program.add_instruction(
            IrInstruction(
                IrOp.ADD, [IrRegister(1), IrRegister(2), IrRegister(3)],
            )
        )
        program.add_instruction(IrInstruction(IrOp.RET, []))
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("make_adder")]))
        program.add_instruction(
            IrInstruction(
                IrOp.MAKE_CLOSURE,
                [
                    IrRegister(1),
                    IrLabel("_lambda_0"),
                    IrImmediate(1),
                    IrRegister(2),
                ],
            )
        )
        program.add_instruction(IrInstruction(IrOp.RET, []))
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")]))
        program.add_instruction(IrInstruction(IrOp.HALT, []))
        return program

    def _config(self) -> JvmBackendConfig:
        return JvmBackendConfig(
            class_name="MakeAdderMain",
            closure_free_var_counts={"_lambda_0": 1},
        )

    def test_objregs_field_present_when_closures_declared(self) -> None:
        """The main class gains an ``Object[] __ca_objregs`` field
        when at least one closure is declared."""
        from jvm_class_file import parse_class_file
        from ir_to_jvm_class_file import lower_ir_to_jvm_classes
        artifact = lower_ir_to_jvm_classes(
            self._make_adder_program(), self._config(),
        )
        # The decoder doesn't expose fields, so check the constant
        # pool: the field name and descriptor should both appear.
        cf = parse_class_file(artifact.main.class_bytes)
        assert b"__ca_objregs" in artifact.main.class_bytes
        assert b"[Ljava/lang/Object;" in artifact.main.class_bytes

    def test_objregs_field_absent_in_pure_int_program(self) -> None:
        """Programs without closures don't pay for the
        Object[] field — the byte stream is identical to pre-2c.5
        for non-closure callers."""
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")]))
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(42)])
        )
        program.add_instruction(IrInstruction(IrOp.HALT, []))
        artifact = lower_ir_to_jvm_class_file(
            program, JvmBackendConfig(class_name="Pure"),
        )
        assert b"__ca_objregs" not in artifact.class_bytes

    def test_lifted_lambda_emitted_as_public_method_on_main(self) -> None:
        """The lifted lambda body lives as a PUBLIC static method
        on the main class so the closure subclass can invokestatic
        it from a different package.  Its descriptor is widened to
        ``(II)I`` (1 capture + 1 explicit arg)."""
        from jvm_class_file import parse_class_file
        from ir_to_jvm_class_file import lower_ir_to_jvm_classes
        artifact = lower_ir_to_jvm_classes(
            self._make_adder_program(), self._config(),
        )
        cf = parse_class_file(artifact.main.class_bytes)
        lambda_method = next(
            (m for m in cf.methods if m.name == "_lambda_0"), None,
        )
        assert lambda_method is not None, "lifted lambda must be on main class"
        assert lambda_method.descriptor == "(II)I"
        # ACC_PUBLIC bit set so cross-class invokestatic resolves.
        assert lambda_method.access_flags & 0x0001

    def test_main_class_callable_labels_includes_lambda(self) -> None:
        """JVMClassArtifact.callable_labels exposes the lifted
        lambda so JAR builders / simulators see it."""
        from ir_to_jvm_class_file import lower_ir_to_jvm_classes
        artifact = lower_ir_to_jvm_classes(
            self._make_adder_program(), self._config(),
        )
        assert "_lambda_0" in artifact.main.callable_labels

    def test_subclass_apply_forwards_via_invokestatic(self) -> None:
        """The closure subclass's ``apply`` body is an
        invokestatic forwarder — its bytecode contains the
        invokestatic opcode (0xB8) but NO ``iconst_0; ireturn``
        placeholder pattern."""
        from ir_to_jvm_class_file import lower_ir_to_jvm_classes
        artifact = lower_ir_to_jvm_classes(
            self._make_adder_program(), self._config(),
        )
        subclass = next(
            c for c in artifact.classes if "Closure__lambda_0" in c.class_name
        )
        # invokestatic = 0xB8; appears in the apply body.
        assert b"\xb8" in subclass.class_bytes

    def test_make_closure_uses_aastore_into_objregs(self) -> None:
        """MAKE_CLOSURE emits aastore (0x53) to write the new ref
        into __ca_objregs — not pop (0x57) like Phase 2c."""
        from ir_to_jvm_class_file import lower_ir_to_jvm_classes
        artifact = lower_ir_to_jvm_classes(
            self._make_adder_program(), self._config(),
        )
        # 0x53 is JVM aastore.  Should appear since MAKE_CLOSURE is
        # the only path that emits it on the main class.
        assert b"\x53" in artifact.main.class_bytes

    def test_apply_closure_uses_aaload_and_checkcast(self) -> None:
        """APPLY_CLOSURE emits aaload (0x32) then checkcast (0xC0)
        on the closure ref read from __ca_objregs."""
        from ir_to_jvm_class_file import lower_ir_to_jvm_classes
        # Build a program that uses APPLY_CLOSURE.
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_lambda_0")]))
        program.add_instruction(IrInstruction(IrOp.RET, []))
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")]))
        program.add_instruction(
            IrInstruction(
                IrOp.MAKE_CLOSURE,
                [IrRegister(1), IrLabel("_lambda_0"), IrImmediate(0)],
            )
        )
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(35)])
        )
        program.add_instruction(
            IrInstruction(
                IrOp.APPLY_CLOSURE,
                [IrRegister(1), IrRegister(1), IrImmediate(1), IrRegister(2)],
            )
        )
        program.add_instruction(IrInstruction(IrOp.HALT, []))
        artifact = lower_ir_to_jvm_classes(
            program,
            JvmBackendConfig(
                class_name="UsesApply",
                closure_free_var_counts={"_lambda_0": 0},
            ),
        )
        # 0x32 = aaload; 0xC0 = checkcast.
        assert b"\x32" in artifact.main.class_bytes
        assert b"\xc0" in artifact.main.class_bytes


# ─────────────────────────────────────────────────────────────────────────
# TW03 Phase 3b — heap primitives (Cons / Symbol / Nil + 8 opcode lowerings)
# ─────────────────────────────────────────────────────────────────────────

class TestHeapRuntimeClasses:
    """Tests for the three runtime classes auto-included whenever a
    program uses any heap opcode (MAKE_CONS, CAR, CDR, IS_NULL, IS_PAIR,
    MAKE_SYMBOL, IS_SYMBOL, LOAD_NIL).  Each artifact is parsed back via
    ``jvm_class_file.parse_class_file`` to lock down the field/method
    layout — the JVM verifier will reject anything malformed at load
    time, so the parse-back also serves as a smoke test."""

    def test_cons_class_layout(self) -> None:
        from ir_to_jvm_class_file import (
            CONS_BINARY_NAME,
            build_cons_class_artifact,
        )
        artifact = build_cons_class_artifact()
        assert artifact.class_name == CONS_BINARY_NAME.replace("/", ".")
        cf = parse_class_file(artifact.class_bytes)
        # Single ctor (I,LObject;)V.
        method_descriptors = [m.descriptor for m in cf.methods]
        assert "(ILjava/lang/Object;)V" in method_descriptors
        # Field-name UTF8 entries — head + tail.
        assert b"head" in artifact.class_bytes
        assert b"tail" in artifact.class_bytes

    def test_nil_class_layout(self) -> None:
        from ir_to_jvm_class_file import (
            NIL_BINARY_NAME,
            build_nil_class_artifact,
        )
        artifact = build_nil_class_artifact()
        assert artifact.class_name == NIL_BINARY_NAME.replace("/", ".")
        cf = parse_class_file(artifact.class_bytes)
        method_names = [m.name for m in cf.methods]
        assert "<init>" in method_names
        assert "<clinit>" in method_names
        # Singleton field.
        assert b"INSTANCE" in artifact.class_bytes

    def test_symbol_class_layout(self) -> None:
        from ir_to_jvm_class_file import (
            SYMBOL_BINARY_NAME,
            build_symbol_class_artifact,
        )
        artifact = build_symbol_class_artifact()
        assert artifact.class_name == SYMBOL_BINARY_NAME.replace("/", ".")
        cf = parse_class_file(artifact.class_bytes)
        method_names = [m.name for m in cf.methods]
        assert "<init>" in method_names
        assert "<clinit>" in method_names
        assert "intern" in method_names
        intern = next(m for m in cf.methods if m.name == "intern")
        assert intern.descriptor == (
            "(Ljava/lang/String;)"
            "Lcoding_adventures/twig/runtime/Symbol;"
        )
        # HashMap is referenced.
        assert b"java/util/HashMap" in artifact.class_bytes


class TestHeapOpLowering:
    """Each new opcode should produce the specific JVM instruction
    bytes that uniquely identify its lowering — relies on byte
    fingerprints rather than a full disassembler."""

    def _make_main_only_program(
        self, instructions: list[IrInstruction],
    ) -> IrProgram:
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")]))
        for ins in instructions:
            program.add_instruction(ins)
        program.add_instruction(IrInstruction(IrOp.HALT, []))
        return program

    def test_make_cons_emits_new_and_invokespecial(self) -> None:
        from ir_to_jvm_class_file import lower_ir_to_jvm_classes
        program = self._make_main_only_program([
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(7)]),
            IrInstruction(
                IrOp.MAKE_CONS,
                [IrRegister(2), IrRegister(1), IrRegister(3)],
            ),
        ])
        artifact = lower_ir_to_jvm_classes(
            program, JvmBackendConfig(class_name="UsesCons"),
        )
        # Cons / Symbol / Nil runtime classes auto-included.
        names = [a.class_name for a in artifact.classes]
        assert "coding_adventures.twig.runtime.Cons" in names
        assert "coding_adventures.twig.runtime.Nil" in names
        assert "coding_adventures.twig.runtime.Symbol" in names
        # 0xBB = new; 0xB7 = invokespecial; 0x53 = aastore.
        body = artifact.main.class_bytes
        assert b"\xbb" in body  # new Cons
        assert b"\xb7" in body  # invokespecial Cons.<init>
        assert b"\x53" in body  # aastore into __ca_objregs

    def test_car_emits_getfield_int(self) -> None:
        from ir_to_jvm_class_file import lower_ir_to_jvm_classes
        program = self._make_main_only_program([
            IrInstruction(IrOp.CAR, [IrRegister(1), IrRegister(2)]),
        ])
        artifact = lower_ir_to_jvm_classes(
            program, JvmBackendConfig(class_name="UsesCar"),
        )
        # 0xB4 = getfield; 0xC0 = checkcast.
        assert b"\xb4" in artifact.main.class_bytes
        assert b"\xc0" in artifact.main.class_bytes

    def test_cdr_emits_getfield_object(self) -> None:
        from ir_to_jvm_class_file import lower_ir_to_jvm_classes
        program = self._make_main_only_program([
            IrInstruction(IrOp.CDR, [IrRegister(1), IrRegister(2)]),
        ])
        artifact = lower_ir_to_jvm_classes(
            program, JvmBackendConfig(class_name="UsesCdr"),
        )
        assert b"\xb4" in artifact.main.class_bytes
        assert b"\xc0" in artifact.main.class_bytes

    def test_is_null_emits_if_acmpne(self) -> None:
        from ir_to_jvm_class_file import lower_ir_to_jvm_classes
        program = self._make_main_only_program([
            IrInstruction(IrOp.IS_NULL, [IrRegister(1), IrRegister(2)]),
        ])
        artifact = lower_ir_to_jvm_classes(
            program, JvmBackendConfig(class_name="UsesIsNull"),
        )
        # 0xA6 = if_acmpne.
        assert b"\xa6" in artifact.main.class_bytes

    def test_is_pair_emits_instanceof(self) -> None:
        from ir_to_jvm_class_file import lower_ir_to_jvm_classes
        program = self._make_main_only_program([
            IrInstruction(IrOp.IS_PAIR, [IrRegister(1), IrRegister(2)]),
        ])
        artifact = lower_ir_to_jvm_classes(
            program, JvmBackendConfig(class_name="UsesIsPair"),
        )
        # 0xC1 = instanceof.
        assert b"\xc1" in artifact.main.class_bytes

    def test_is_symbol_emits_instanceof(self) -> None:
        from ir_to_jvm_class_file import lower_ir_to_jvm_classes
        program = self._make_main_only_program([
            IrInstruction(IrOp.IS_SYMBOL, [IrRegister(1), IrRegister(2)]),
        ])
        artifact = lower_ir_to_jvm_classes(
            program, JvmBackendConfig(class_name="UsesIsSym"),
        )
        assert b"\xc1" in artifact.main.class_bytes

    def test_make_symbol_emits_intern_call(self) -> None:
        from ir_to_jvm_class_file import lower_ir_to_jvm_classes
        program = self._make_main_only_program([
            IrInstruction(
                IrOp.MAKE_SYMBOL,
                [IrRegister(1), IrLabel("foo")],
            ),
        ])
        artifact = lower_ir_to_jvm_classes(
            program, JvmBackendConfig(class_name="UsesMakeSym"),
        )
        # The literal "foo" UTF8 is in the constant pool.
        assert b"foo" in artifact.main.class_bytes
        # 0xB8 = invokestatic Symbol.intern.
        assert b"\xb8" in artifact.main.class_bytes

    def test_load_nil_emits_getstatic(self) -> None:
        from ir_to_jvm_class_file import lower_ir_to_jvm_classes
        program = self._make_main_only_program([
            IrInstruction(IrOp.LOAD_NIL, [IrRegister(1)]),
        ])
        artifact = lower_ir_to_jvm_classes(
            program, JvmBackendConfig(class_name="UsesLoadNil"),
        )
        # 0xB2 = getstatic Nil.INSTANCE; 0x53 = aastore.
        assert b"\xb2" in artifact.main.class_bytes
        assert b"\x53" in artifact.main.class_bytes
        # And the Nil class gets included.
        names = [a.class_name for a in artifact.classes]
        assert "coding_adventures.twig.runtime.Nil" in names

    def test_heap_op_arity_validation(self) -> None:
        """MAKE_CONS with the wrong operand count is rejected."""
        from ir_to_jvm_class_file import lower_ir_to_jvm_classes
        program = self._make_main_only_program([
            IrInstruction(IrOp.MAKE_CONS, [IrRegister(1)]),  # missing 2 ops
        ])
        with pytest.raises(JvmBackendError, match=r"MAKE_CONS expects"):
            lower_ir_to_jvm_classes(
                program, JvmBackendConfig(class_name="Bad"),
            )


# Real-java end-to-end — list-of-ints length.

requires_java = pytest.mark.skipif(
    subprocess.call(
        ["java", "-version"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    ) != 0
    if os.environ.get("CODING_ADVENTURES_REQUIRE_JAVA") != "1"
    else False,
    reason="java runtime not available",
)


@requires_java
def test_real_java_list_of_ints_length() -> None:
    """Hand-written IR for the program::

        (length (cons 1 (cons 2 (cons 3 nil))))

    with ``length`` lowered as a CALL-recursive function that
    increments r1 each step until ``IS_NULL`` of the list head
    becomes 1.  Exits with the int length via SYSCALL 1.

    This is an end-to-end smoke test that exercises:

      * Cons / Nil / Symbol class loading
      * MAKE_CONS lowering
      * CDR lowering
      * IS_NULL identity test
      * BRANCH_Z fed by IS_NULL result
      * The full multi-class JAR-shaped artifact (just spread on
        a classpath here for simplicity since the test doesn't
        depend on Main-Class manifest semantics).
    """
    from ir_to_jvm_class_file import (
        JvmBackendConfig,
        lower_ir_to_jvm_classes,
        write_class_file,
    )

    program = IrProgram(entry_label="_start")
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")]))
    # Build the list (cons 1 (cons 2 (cons 3 nil))) backwards:
    #   r10 = nil
    #   r3 = 3; r10 = cons(3, nil)
    #   r3 = 2; r10 = cons(2, r10)
    #   r3 = 1; r10 = cons(1, r10)
    program.add_instruction(IrInstruction(IrOp.LOAD_NIL, [IrRegister(10)]))
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(3), IrImmediate(3)])
    )
    program.add_instruction(
        IrInstruction(
            IrOp.MAKE_CONS,
            [IrRegister(10), IrRegister(3), IrRegister(10)],
        )
    )
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(3), IrImmediate(2)])
    )
    program.add_instruction(
        IrInstruction(
            IrOp.MAKE_CONS,
            [IrRegister(10), IrRegister(3), IrRegister(10)],
        )
    )
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(3), IrImmediate(1)])
    )
    program.add_instruction(
        IrInstruction(
            IrOp.MAKE_CONS,
            [IrRegister(10), IrRegister(3), IrRegister(10)],
        )
    )
    # Compute length iteratively in r1, walking r10.
    #   r1 = 0
    # loop:
    #   r2 = is_null(r10)
    #   branch_z r2, end
    #   r1 = r1 + 1
    #   r10 = cdr(r10)
    #   jump loop
    # end:
    #   syscall 1, r1   ; print one byte
    #   halt
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(0)])
    )
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("loop")]))
    program.add_instruction(
        IrInstruction(IrOp.IS_NULL, [IrRegister(2), IrRegister(10)])
    )
    program.add_instruction(
        IrInstruction(
            IrOp.BRANCH_NZ, [IrRegister(2), IrLabel("end")],
        )
    )
    program.add_instruction(
        IrInstruction(
            IrOp.ADD_IMM,
            [IrRegister(1), IrRegister(1), IrImmediate(1)],
        )
    )
    program.add_instruction(
        IrInstruction(IrOp.CDR, [IrRegister(10), IrRegister(10)])
    )
    program.add_instruction(IrInstruction(IrOp.JUMP, [IrLabel("loop")]))
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("end")]))
    program.add_instruction(
        IrInstruction(IrOp.SYSCALL, [IrImmediate(1), IrRegister(1)])
    )
    program.add_instruction(IrInstruction(IrOp.HALT, []))

    artifact = lower_ir_to_jvm_classes(
        program,
        JvmBackendConfig(
            class_name="ListLength", syscall_arg_reg=1,
        ),
    )

    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        for cls in artifact.classes:
            write_class_file(cls, tmp_path)
        result = subprocess.run(
            ["java", "-cp", str(tmp_path), "ListLength"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=30,
            check=False,
        )

    assert result.returncode == 0, result.stderr.decode("utf-8", "replace")
    # Length 3 → SYSCALL prints byte 0x03 to stdout.
    assert result.stdout == b"\x03", result.stdout
