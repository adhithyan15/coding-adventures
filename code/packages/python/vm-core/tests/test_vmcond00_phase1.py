"""Tests for VMCOND00 Phase 1 — syscall_checked and branch_err opcodes.

VMCOND00 Phase 1 adds two new IIR opcodes to vm-core:

    syscall_checked  n, arg_reg, val_dst, err_dst
        Invoke host syscall n.  Store success value in val_dst and
        error code (0 ok, -1 EOF, <-1 negated errno) in err_dst.
        Never traps; all errors surface through err_dst.

    branch_err  err_reg, label
        Jump to label when err_reg != 0 (syscall failed).
        Fall through when err_reg == 0 (success).

Public API additions:

    VMCore.register_syscall(n, impl)
        Register (arg: int) -> (value: int, error_code: int) for syscall n.

    VMCore.unregister_syscall(n)
        Remove a previously registered syscall implementation.

Coverage targets:
  - All four branches of handle_syscall_checked:
        (a) unknown syscall number → EINVAL
        (b) impl raises Python exception → EINVAL
        (c) success path
        (d) error path (e.g. EOF)
  - Both branches of handle_branch_err:
        (a) err != 0 → branch taken
        (b) err == 0 → fall-through
  - register_syscall and unregister_syscall in VMCore.
"""

from __future__ import annotations

import pytest
from interpreter_ir import IIRFunction, IIRInstr, IIRModule

from vm_core import VMCore

# ---------------------------------------------------------------------------
# Test helpers — same helpers used in test_vm_core.py
# ---------------------------------------------------------------------------

def _fn(
    name: str,
    params: list[tuple[str, str]],
    *instrs: IIRInstr,
    return_type: str = "any",
) -> IIRFunction:
    """Build an IIRFunction with auto-computed register_count."""
    return IIRFunction(
        name=name,
        params=params,
        return_type=return_type,
        instructions=list(instrs),
        register_count=max(8, len(params) + len(instrs)),
    )


def _mod(*fns: IIRFunction) -> IIRModule:
    return IIRModule(name="test", functions=list(fns))


def _i(op: str, dest: str | None = None, srcs: list | None = None,
       type_hint: str = "any") -> IIRInstr:
    return IIRInstr(op=op, dest=dest, srcs=srcs or [], type_hint=type_hint)


# EINVAL negated — matches POSIX errno 22.
_EINVAL = -22


# ---------------------------------------------------------------------------
# TestRegisterSyscall — VMCore.register_syscall / unregister_syscall
# ---------------------------------------------------------------------------

class TestRegisterSyscall:
    """Test the public API for registering host syscall implementations."""

    def test_register_syscall_populates_table(self) -> None:
        """register_syscall stores the impl in _syscall_table under the number."""
        vm = VMCore()
        impl = lambda arg: (0, 0)  # noqa: E731
        vm.register_syscall(1, impl)
        assert vm._syscall_table[1] is impl

    def test_register_multiple_syscalls(self) -> None:
        """Multiple syscalls can be registered independently."""
        vm = VMCore()
        impl1 = lambda arg: (0, 0)  # noqa: E731
        impl2 = lambda arg: (arg, 0)  # noqa: E731
        vm.register_syscall(1, impl1)
        vm.register_syscall(2, impl2)
        assert vm._syscall_table[1] is impl1
        assert vm._syscall_table[2] is impl2

    def test_register_syscall_overwrite(self) -> None:
        """Registering the same number twice replaces the previous impl."""
        vm = VMCore()
        impl_old = lambda arg: (0, 0)  # noqa: E731
        impl_new = lambda arg: (1, 0)  # noqa: E731
        vm.register_syscall(1, impl_old)
        vm.register_syscall(1, impl_new)
        assert vm._syscall_table[1] is impl_new

    def test_unregister_syscall_removes_entry(self) -> None:
        """unregister_syscall removes a registered impl."""
        vm = VMCore()
        vm.register_syscall(1, lambda arg: (0, 0))  # noqa: E731
        vm.unregister_syscall(1)
        assert 1 not in vm._syscall_table

    def test_unregister_unregistered_is_noop(self) -> None:
        """unregister_syscall on an unknown number is a safe no-op."""
        vm = VMCore()
        vm.unregister_syscall(999)  # must not raise

    def test_syscall_table_empty_by_default(self) -> None:
        """A fresh VMCore has no registered syscalls."""
        vm = VMCore()
        assert vm._syscall_table == {}

    def test_register_syscall_rejects_zero(self) -> None:
        """Syscall 0 is reserved by the ABI — register_syscall must reject it."""
        vm = VMCore()
        with pytest.raises(ValueError, match="outside the valid range"):
            vm.register_syscall(0, lambda arg: (0, 0))  # noqa: E731

    def test_register_syscall_rejects_above_255(self) -> None:
        """Syscall numbers > 255 are beyond the canonical table and must be rejected."""
        vm = VMCore()
        with pytest.raises(ValueError, match="outside the valid range"):
            vm.register_syscall(256, lambda arg: (0, 0))  # noqa: E731

    def test_register_syscall_rejects_negative(self) -> None:
        """Negative syscall numbers are not valid SYSCALL00 numbers."""
        vm = VMCore()
        with pytest.raises(ValueError, match="outside the valid range"):
            vm.register_syscall(-1, lambda arg: (0, 0))  # noqa: E731

    def test_register_syscall_accepts_boundary_255(self) -> None:
        """Syscall 255 is at the upper boundary of the valid range."""
        vm = VMCore()
        impl = lambda arg: (0, 0)  # noqa: E731
        vm.register_syscall(255, impl)
        assert vm._syscall_table[255] is impl

    def test_register_syscall_accepts_boundary_1(self) -> None:
        """Syscall 1 is at the lower boundary of the valid range."""
        vm = VMCore()
        impl = lambda arg: (0, 0)  # noqa: E731
        vm.register_syscall(1, impl)
        assert vm._syscall_table[1] is impl


# ---------------------------------------------------------------------------
# TestSyscallChecked — handle_syscall_checked dispatch logic
# ---------------------------------------------------------------------------

class TestSyscallChecked:
    """Test the syscall_checked opcode handler."""

    def _run_syscall_checked(
        self,
        vm: VMCore,
        n: int,
        arg_value: int,
    ) -> tuple[int, int]:
        """Run a minimal program with syscall_checked; return (val, err)."""
        #
        # IIR program:
        #   main:
        #     const arg, <arg_value>
        #     syscall_checked <n>, arg, val, err
        #     ret val          ; return the success value for inspection
        #
        # We inspect err by running a second program or by reading the register
        # directly after execution through a custom ret.  Simpler: return val
        # and check err via a second helper.
        #
        # Actually, let's return a tuple via a builtin.  Even simpler:
        # we build two separate programs and read both registers.
        # Cleanest: encode both values into a single integer via val*1000+err
        # — but that conflates negative numbers.
        #
        # Best: use a custom "pair" builtin that returns a Python tuple.
        # The test inspects the tuple from the return value.
        #
        results: list[tuple[int, int]] = []

        def capture_pair(args: list) -> int:
            """Builtin that captures (val, err) and returns 0."""
            results.append((int(args[0]), int(args[1])))
            return 0

        vm.register_builtin("_capture", capture_pair)

        prog = _fn(
            "main",
            [],
            _i("const", "arg", [arg_value]),
            _i("syscall_checked", None, [n, "arg", "val", "err"]),
            _i("call_builtin", "dummy", ["_capture", "val", "err"]),
            _i("ret", None, ["dummy"]),
        )
        vm.execute(_mod(prog))
        return results[0]

    def test_success_path(self) -> None:
        """A registered syscall that succeeds stores value and 0 in err_dst."""
        vm = VMCore()
        # Syscall 1 = write-byte; arg is the byte; returns (0, 0) on success.
        vm.register_syscall(1, lambda arg: (0, 0))
        val, err = self._run_syscall_checked(vm, 1, 65)
        assert val == 0
        assert err == 0

    def test_success_with_return_value(self) -> None:
        """Syscall 2 = read-byte; implementation returns (byte, 0) on success."""
        vm = VMCore()
        # Mock read-byte that always "reads" 42.
        vm.register_syscall(2, lambda arg: (42, 0))
        val, err = self._run_syscall_checked(vm, 2, 0)
        assert val == 42
        assert err == 0

    def test_eof_path(self) -> None:
        """Syscall returning (0, -1) encodes EOF; val=0, err=-1."""
        vm = VMCore()
        vm.register_syscall(2, lambda arg: (0, -1))
        val, err = self._run_syscall_checked(vm, 2, 0)
        assert val == 0
        assert err == -1

    def test_errno_path(self) -> None:
        """Syscall returning (0, -5) encodes EIO; val=0, err=-5."""
        vm = VMCore()
        vm.register_syscall(2, lambda arg: (0, -5))
        val, err = self._run_syscall_checked(vm, 2, 0)
        assert val == 0
        assert err == -5

    def test_unknown_syscall_returns_einval(self) -> None:
        """An unregistered syscall number stores 0, EINVAL in the output registers."""
        vm = VMCore()  # no syscalls registered
        val, err = self._run_syscall_checked(vm, 99, 0)
        assert val == 0
        assert err == _EINVAL

    def test_impl_raises_exception_returns_einval(self) -> None:
        """If the syscall impl raises, the handler catches it and stores EINVAL."""
        vm = VMCore()

        def exploding_syscall(arg: int) -> tuple[int, int]:
            raise RuntimeError("I/O device on fire")  # noqa: EM101

        vm.register_syscall(1, exploding_syscall)
        val, err = self._run_syscall_checked(vm, 1, 0)
        assert val == 0
        assert err == _EINVAL

    def test_arg_register_is_passed_to_impl(self) -> None:
        """The resolved arg register value is forwarded to the syscall impl."""
        received: list[int] = []

        def recording_syscall(arg: int) -> tuple[int, int]:
            received.append(arg)
            return (0, 0)

        vm = VMCore()
        vm.register_syscall(1, recording_syscall)
        self._run_syscall_checked(vm, 1, 65)
        assert received == [65]


# ---------------------------------------------------------------------------
# TestBranchErr — handle_branch_err dispatch logic
# ---------------------------------------------------------------------------

class TestBranchErr:
    """Test the branch_err opcode handler."""

    def _run_branch_err_program(self, err_value: int) -> int:
        """Run a branch_err program; return 1 if branch taken, 0 if fall-through."""
        #
        # IIR program:
        #   main:
        #     const err, <err_value>
        #     branch_err err, error_path
        #     const result, 0        ; fall-through → success path → return 0
        #     ret result
        #   label error_path
        #     const result, 1        ; branch taken → error path → return 1
        #     ret result
        #
        prog = _fn(
            "main",
            [],
            _i("const", "err", [err_value]),
            _i("branch_err", None, ["err", "error_path"]),
            _i("const", "result", [0]),
            _i("ret", None, ["result"]),
            _i("label", None, ["error_path"]),
            _i("const", "result", [1]),
            _i("ret", None, ["result"]),
        )
        vm = VMCore()
        return vm.execute(_mod(prog))

    def test_branch_taken_on_nonzero_err(self) -> None:
        """branch_err jumps when err_reg is -1 (EOF)."""
        result = self._run_branch_err_program(-1)
        assert result == 1  # branch was taken

    def test_branch_taken_on_negative_errno(self) -> None:
        """branch_err jumps when err_reg is a negated errno (e.g. -22)."""
        result = self._run_branch_err_program(-22)
        assert result == 1

    def test_branch_taken_on_positive_nonzero(self) -> None:
        """branch_err jumps for any non-zero value, not just negatives."""
        result = self._run_branch_err_program(1)
        assert result == 1

    def test_fallthrough_on_zero_err(self) -> None:
        """branch_err falls through (does NOT jump) when err_reg is 0."""
        result = self._run_branch_err_program(0)
        assert result == 0  # fell through


# ---------------------------------------------------------------------------
# TestSyscallCheckedWithBranchErr — integrated round-trip tests
# ---------------------------------------------------------------------------

class TestSyscallCheckedWithBranchErr:
    """Integration tests: syscall_checked feeds branch_err for real control flow."""

    def test_read_byte_success_reads_value(self) -> None:
        """A mock read-byte syscall that succeeds routes through the happy path."""
        vm = VMCore()
        # Syscall 2 = read-byte; returns (byte=42, err=0) on success.
        vm.register_syscall(2, lambda arg: (42, 0))

        #
        # main:
        #   const arg, 0
        #   syscall_checked 2, arg, val, err
        #   branch_err err, eof_path
        #   ret val           ; happy path — return the byte
        # label eof_path
        #   const minus_one, -1
        #   ret minus_one     ; error path — return sentinel -1
        #
        prog = _fn(
            "main",
            [],
            _i("const", "arg", [0]),
            _i("syscall_checked", None, [2, "arg", "val", "err"]),
            _i("branch_err", None, ["err", "eof_path"]),
            _i("ret", None, ["val"]),
            _i("label", None, ["eof_path"]),
            _i("const", "minus_one", [-1]),
            _i("ret", None, ["minus_one"]),
        )
        result = vm.execute(_mod(prog))
        assert result == 42

    def test_read_byte_eof_routes_to_error_path(self) -> None:
        """A mock read-byte syscall that returns EOF routes through the error path."""
        vm = VMCore()
        # Syscall 2 = read-byte; EOF → (0, -1).
        vm.register_syscall(2, lambda arg: (0, -1))

        prog = _fn(
            "main",
            [],
            _i("const", "arg", [0]),
            _i("syscall_checked", None, [2, "arg", "val", "err"]),
            _i("branch_err", None, ["err", "eof_path"]),
            _i("ret", None, ["val"]),
            _i("label", None, ["eof_path"]),
            _i("const", "minus_one", [-1]),
            _i("ret", None, ["minus_one"]),
        )
        result = vm.execute(_mod(prog))
        assert result == -1  # reached eof_path

    def test_unknown_syscall_routes_to_error_path(self) -> None:
        """An unregistered syscall yields EINVAL which branches to the error path."""
        vm = VMCore()  # no syscalls registered

        prog = _fn(
            "main",
            [],
            _i("const", "arg", [0]),
            _i("syscall_checked", None, [99, "arg", "val", "err"]),
            _i("branch_err", None, ["err", "err_path"]),
            _i("ret", None, ["val"]),       # success (won't run)
            _i("label", None, ["err_path"]),
            _i("ret", None, ["err"]),       # return the error code
        )
        result = vm.execute(_mod(prog))
        assert result == _EINVAL

    def test_multiple_syscalls_with_branching(self) -> None:
        """A program that writes a byte (success) then reads a byte (success)."""
        written: list[int] = []
        read_counter = [0]

        def write_byte(arg: int) -> tuple[int, int]:
            written.append(arg & 0xFF)
            return (0, 0)

        def read_byte(arg: int) -> tuple[int, int]:
            read_counter[0] += 1
            return (ord("X"), 0)

        vm = VMCore()
        vm.register_syscall(1, write_byte)
        vm.register_syscall(2, read_byte)

        #
        # Write byte 65 ('A'), then read one byte.
        # Both succeed; return the read byte.
        #
        prog = _fn(
            "main",
            [],
            _i("const", "byte_to_write", [65]),
            _i("syscall_checked", None, [1, "byte_to_write", "wval", "werr"]),
            _i("branch_err", None, ["werr", "write_failed"]),
            _i("const", "zero", [0]),
            _i("syscall_checked", None, [2, "zero", "rval", "rerr"]),
            _i("branch_err", None, ["rerr", "read_failed"]),
            _i("ret", None, ["rval"]),
            _i("label", None, ["write_failed"]),
            _i("const", "neg1", [-1]),
            _i("ret", None, ["neg1"]),
            _i("label", None, ["read_failed"]),
            _i("const", "neg2", [-2]),
            _i("ret", None, ["neg2"]),
        )
        result = vm.execute(_mod(prog))
        assert written == [65]
        assert read_counter[0] == 1
        assert result == ord("X")
