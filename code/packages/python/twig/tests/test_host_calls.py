"""Tests for TW04 Phase 4c — host module syscall convention.

Covers:
* ``_is_module_qualified`` helper (correctly identifies module/name patterns)
* ``_HOST_SYSCALLS`` mapping (write-byte=1, read-byte=2, exit=10)
* Compiler emits ``call_builtin "syscall" <num> <arg_reg>`` for host calls
* VM executes syscall 1 (write-byte) and writes the correct byte to stdout
* VM executes syscall 2 (read-byte) and returns the byte read from stdin
* VM raises on unknown syscall number
* ``free_vars`` does NOT include ``host/write-byte`` etc. in free-var sets
* Unknown host-module calls raise :class:`TwigCompileError`
"""

from __future__ import annotations

import io
import sys
from unittest.mock import patch

import pytest

from twig import TwigVM
from twig.ast_extract import extract_program
from twig.ast_nodes import Lambda
from twig.compiler import _HOST_SYSCALLS, _is_module_qualified, compile_program
from twig.errors import TwigCompileError, TwigExitRequest, TwigRuntimeError
from twig.free_vars import free_vars
from twig.parser import parse_twig


# ---------------------------------------------------------------------------
# _is_module_qualified
# ---------------------------------------------------------------------------


class TestIsModuleQualified:
    """The helper must distinguish ``module/name`` from bare operators."""

    def test_host_write_byte_is_qualified(self) -> None:
        assert _is_module_qualified("host/write-byte") is True

    def test_host_read_byte_is_qualified(self) -> None:
        assert _is_module_qualified("host/read-byte") is True

    def test_host_exit_is_qualified(self) -> None:
        assert _is_module_qualified("host/exit") is True

    def test_stdlib_nested_is_qualified(self) -> None:
        assert _is_module_qualified("stdlib/io/println") is True

    def test_bare_slash_is_not_qualified(self) -> None:
        """The division operator ``/`` is a bare slash — not module-qualified."""
        assert _is_module_qualified("/") is False

    def test_leading_slash_is_not_qualified(self) -> None:
        assert _is_module_qualified("/foo") is False

    def test_trailing_slash_is_not_qualified(self) -> None:
        assert _is_module_qualified("foo/") is False

    def test_plain_name_is_not_qualified(self) -> None:
        assert _is_module_qualified("print") is False
        assert _is_module_qualified("write-byte") is False

    def test_empty_string_is_not_qualified(self) -> None:
        assert _is_module_qualified("") is False


# ---------------------------------------------------------------------------
# _HOST_SYSCALLS mapping
# ---------------------------------------------------------------------------


class TestHostSyscallNumbers:
    """Verify the platform-independent syscall numbering."""

    def test_write_byte_is_syscall_1(self) -> None:
        assert _HOST_SYSCALLS["host/write-byte"] == 1

    def test_read_byte_is_syscall_2(self) -> None:
        assert _HOST_SYSCALLS["host/read-byte"] == 2

    def test_exit_is_syscall_10(self) -> None:
        assert _HOST_SYSCALLS["host/exit"] == 10


# ---------------------------------------------------------------------------
# Compiler IR output
# ---------------------------------------------------------------------------


class TestCompilerSyscallEmission:
    """The interpreter compiler must emit ``call_builtin 'syscall' num arg``."""

    def _compile(self, source: str):  # type: ignore[return]
        prog = extract_program(parse_twig(source))
        return compile_program(prog)

    def test_write_byte_emits_syscall_1(self) -> None:
        module = self._compile("(host/write-byte 65)")
        main = module.get_function("main")
        assert main is not None
        syscall_instrs = [
            i for i in main.instructions
            if i.op == "call_builtin"
            and i.srcs
            and i.srcs[0] == "syscall"
        ]
        assert len(syscall_instrs) == 1
        instr = syscall_instrs[0]
        # srcs = ["syscall", 1, arg_reg]
        assert instr.srcs[1] == 1, f"expected syscall 1, got {instr.srcs[1]}"

    def test_read_byte_emits_syscall_2(self) -> None:
        module = self._compile("(host/read-byte)")
        main = module.get_function("main")
        assert main is not None
        syscall_instrs = [
            i for i in main.instructions
            if i.op == "call_builtin"
            and i.srcs
            and i.srcs[0] == "syscall"
        ]
        assert len(syscall_instrs) == 1
        assert syscall_instrs[0].srcs[1] == 2

    def test_exit_emits_syscall_10(self) -> None:
        module = self._compile("(host/exit 0)")
        main = module.get_function("main")
        assert main is not None
        syscall_instrs = [
            i for i in main.instructions
            if i.op == "call_builtin"
            and i.srcs
            and i.srcs[0] == "syscall"
        ]
        assert len(syscall_instrs) == 1
        assert syscall_instrs[0].srcs[1] == 10

    def test_unknown_host_call_raises(self) -> None:
        with pytest.raises(TwigCompileError, match="unknown host call"):
            self._compile("(host/unknown-thing 99)")


# ---------------------------------------------------------------------------
# VM execution — syscall 1 (write-byte)
# ---------------------------------------------------------------------------


class TestVMSyscall:
    """The TwigVM must route ``call_builtin 'syscall'`` to the real host ops."""

    @staticmethod
    def _fake_stdout() -> tuple[io.BytesIO, io.TextIOWrapper]:
        """Build a (buf, stdout) pair where ``stdout.buffer`` is ``buf``."""
        buf = io.BytesIO()
        return buf, io.TextIOWrapper(buf, encoding="utf-8", write_through=True)

    @staticmethod
    def _fake_stdin(data: bytes) -> io.TextIOWrapper:
        """Build a fake stdin whose ``buffer`` contains ``data``."""
        return io.TextIOWrapper(io.BytesIO(data), encoding="utf-8")

    def test_write_byte_65_outputs_A(self) -> None:
        """SYSCALL 1 with argument 65 must write byte 0x41 (ASCII 'A')."""
        vm = TwigVM()
        buf, fake = self._fake_stdout()
        with patch("sys.stdout", fake):
            vm.run("(host/write-byte 65)")
        assert buf.getvalue() == b"A"

    def test_write_byte_masks_to_unsigned_byte(self) -> None:
        """Values larger than 255 should be masked to the low byte."""
        vm = TwigVM()
        buf, fake = self._fake_stdout()
        with patch("sys.stdout", fake):
            vm.run("(host/write-byte 321)")  # 321 & 0xFF = 65 = 'A'
        assert buf.getvalue() == b"A"

    def test_read_byte_returns_byte_from_stdin(self) -> None:
        """SYSCALL 2 reads one byte and returns it as an integer."""
        vm = TwigVM()
        with patch("sys.stdin", self._fake_stdin(b"Z")):
            _, result = vm.run("(host/read-byte)")
        assert result == ord("Z")

    def test_read_byte_returns_minus_one_on_eof(self) -> None:
        """SYSCALL 2 returns −1 when stdin is at EOF."""
        vm = TwigVM()
        with patch("sys.stdin", self._fake_stdin(b"")):
            _, result = vm.run("(host/read-byte)")
        assert result == -1

    def test_exit_raises_twig_exit_request(self) -> None:
        """SYSCALL 10 must raise TwigExitRequest (not sys.exit) so embedded
        hosts can catch it without the entire Python process dying."""
        vm = TwigVM()
        with pytest.raises(TwigExitRequest) as exc_info:
            vm.run("(host/exit 42)")
        assert exc_info.value.code == 42

    def test_exit_code_zero(self) -> None:
        """Exit code 0 also raises TwigExitRequest."""
        vm = TwigVM()
        with pytest.raises(TwigExitRequest) as exc_info:
            vm.run("(host/exit 0)")
        assert exc_info.value.code == 0

    def test_unknown_syscall_raises_runtime_error(self) -> None:
        """A syscall number not in {1, 2, 10} must raise TwigRuntimeError.

        We test this by bypassing the compiler (which only emits known
        syscall numbers) and directly registering a tiny module that
        calls the 'syscall' builtin with number 99.
        """
        from interpreter_ir import FunctionTypeStatus, IIRFunction, IIRInstr, IIRModule

        # Build a minimal IIR module that calls syscall 99.
        instr_num = IIRInstr("const", "_n1", [99], type_hint="any")
        instr_call = IIRInstr(
            "call_builtin", "_r1", ["syscall", 99], type_hint="any"
        )
        instr_ret = IIRInstr("ret", None, ["_n1"], type_hint="any")
        main_fn = IIRFunction(
            name="main",
            params=[],
            return_type="any",
            instructions=[instr_num, instr_call, instr_ret],
            register_count=4,
            type_status=FunctionTypeStatus.UNTYPED,
        )
        module = IIRModule(
            name="test_unknown_syscall",
            functions=[main_fn],
            entry_point="main",
            language="twig",
        )
        vm = TwigVM()
        with pytest.raises(TwigRuntimeError, match="unknown syscall"):
            vm.execute_module(module)


# ---------------------------------------------------------------------------
# free_vars — host names are not closure captures
# ---------------------------------------------------------------------------


def _lambda_from(source: str) -> Lambda:
    """Parse and extract the innermost lambda from ``source``.

    For ``(define (f x) (lambda (y) ...))``, returns the *inner*
    lambda ``(lambda (y) ...)``, not the outer function body.
    """
    from twig.ast_nodes import Define

    prog = extract_program(parse_twig(source))
    for form in prog.forms:
        if isinstance(form, Lambda):
            return form
        if isinstance(form, Define) and isinstance(form.expr, Lambda):
            outer = form.expr
            # Walk one level deeper — find the first nested lambda.
            for expr in outer.body:
                if isinstance(expr, Lambda):
                    return expr
            return outer
    raise AssertionError("no lambda found")


class TestFreeVarsHostCalls:
    """Module-qualified names must not appear in free-variable sets."""

    def test_host_write_byte_not_free(self) -> None:
        """``x`` is free; ``host/write-byte`` is a qualified name — not free."""
        lam = _lambda_from(
            "(lambda (y) (host/write-byte y))"
        )
        fvs = free_vars(lam, globals_={"+"})
        assert "host/write-byte" not in fvs
        # ``y`` is a param, so it's also not free.
        assert "y" not in fvs

    def test_captured_var_still_free_with_host_call(self) -> None:
        """Local variable x is free; host call is not."""
        lam = _lambda_from(
            "(define (f x) (lambda (y) (host/write-byte x)))"
        )
        fvs = free_vars(lam, globals_={"+"})
        assert "x" in fvs
        assert "host/write-byte" not in fvs

    def test_division_operator_is_not_module_qualified(self) -> None:
        """``/`` must NOT be treated as a module-qualified name."""
        lam = _lambda_from(
            "(define (f n) (lambda (x) (/ x n)))"
        )
        # ``n`` is captured; ``/`` is a builtin (in globals_)
        fvs = free_vars(lam, globals_={"/", "+"})
        assert "n" in fvs
        assert "/" not in fvs  # it's in globals_, so not free
