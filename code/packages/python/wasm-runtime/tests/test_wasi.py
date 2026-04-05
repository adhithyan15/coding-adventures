"""test_wasi.py --- Tests for the WASI stub implementation.

Covers: fd_write stdout/stderr capture, proc_exit, ENOSYS stubs,
resolve_function for known/unknown functions, resolve_global/memory/table.
"""

from __future__ import annotations

import pytest

from wasm_execution import LinearMemory, i32
from wasm_runtime.wasi_stub import ENOSYS, ESUCCESS, ProcExitError, WasiStub, _HostFunc


# ===========================================================================
# ProcExitError
# ===========================================================================


class TestProcExitError:
    def test_exit_code(self) -> None:
        err = ProcExitError(42)
        assert err.exit_code == 42
        assert "42" in str(err)

    def test_exit_code_zero(self) -> None:
        err = ProcExitError(0)
        assert err.exit_code == 0


# ===========================================================================
# WasiStub resolution
# ===========================================================================


class TestWasiStubResolve:
    def test_resolve_fd_write(self) -> None:
        stub = WasiStub()
        func = stub.resolve_function("wasi_snapshot_preview1", "fd_write")
        assert func is not None

    def test_resolve_proc_exit(self) -> None:
        stub = WasiStub()
        func = stub.resolve_function("wasi_snapshot_preview1", "proc_exit")
        assert func is not None

    def test_resolve_unknown_returns_stub(self) -> None:
        stub = WasiStub()
        func = stub.resolve_function("wasi_snapshot_preview1", "args_get")
        assert func is not None
        # The stub should return ENOSYS
        result = func.call([])
        assert result[0].value == ENOSYS

    def test_resolve_wrong_module_returns_none(self) -> None:
        stub = WasiStub()
        func = stub.resolve_function("some_other_module", "fd_write")
        assert func is None

    def test_resolve_global_returns_none(self) -> None:
        stub = WasiStub()
        assert stub.resolve_global("wasi_snapshot_preview1", "x") is None

    def test_resolve_memory_returns_none(self) -> None:
        stub = WasiStub()
        assert stub.resolve_memory("wasi_snapshot_preview1", "mem") is None

    def test_resolve_table_returns_none(self) -> None:
        stub = WasiStub()
        assert stub.resolve_table("wasi_snapshot_preview1", "tbl") is None


# ===========================================================================
# fd_write
# ===========================================================================


class TestFdWrite:
    def _setup_memory_with_iovec(
        self, text: str, fd: int = 1
    ) -> tuple[WasiStub, LinearMemory, list]:
        """Set up memory with a single iovec pointing to `text`."""
        captured: list[str] = []
        stub = WasiStub(stdout=captured.append, stderr=captured.append)
        mem = LinearMemory(1)
        stub.set_memory(mem)

        # Write the text bytes starting at offset 100
        for j, ch in enumerate(text):
            mem._data[100 + j] = ord(ch)

        # iovec at offset 0: buf_ptr=100, buf_len=len(text)
        mem.store_i32(0, 100)
        mem.store_i32(4, len(text))

        return stub, mem, captured

    def test_fd_write_stdout(self) -> None:
        stub, mem, captured = self._setup_memory_with_iovec("Hello")
        func = stub.resolve_function("wasi_snapshot_preview1", "fd_write")
        # fd=1 (stdout), iovs_ptr=0, iovs_len=1, nwritten_ptr=200
        result = func.call([i32(1), i32(0), i32(1), i32(200)])
        assert result[0].value == ESUCCESS
        assert captured == ["Hello"]
        assert mem.load_i32(200) == 5

    def test_fd_write_stderr(self) -> None:
        stub, mem, captured = self._setup_memory_with_iovec("Error")
        func = stub.resolve_function("wasi_snapshot_preview1", "fd_write")
        result = func.call([i32(2), i32(0), i32(1), i32(200)])
        assert result[0].value == ESUCCESS
        assert captured == ["Error"]

    def test_fd_write_no_memory(self) -> None:
        """fd_write without set_memory should return ENOSYS."""
        stub = WasiStub()
        func = stub.resolve_function("wasi_snapshot_preview1", "fd_write")
        result = func.call([i32(1), i32(0), i32(0), i32(0)])
        assert result[0].value == ENOSYS


# ===========================================================================
# proc_exit
# ===========================================================================


class TestProcExit:
    def test_proc_exit_raises(self) -> None:
        stub = WasiStub()
        func = stub.resolve_function("wasi_snapshot_preview1", "proc_exit")
        with pytest.raises(ProcExitError) as exc_info:
            func.call([i32(42)])
        assert exc_info.value.exit_code == 42

    def test_proc_exit_zero(self) -> None:
        stub = WasiStub()
        func = stub.resolve_function("wasi_snapshot_preview1", "proc_exit")
        with pytest.raises(ProcExitError) as exc_info:
            func.call([i32(0)])
        assert exc_info.value.exit_code == 0


# ===========================================================================
# _HostFunc
# ===========================================================================


class TestHostFunc:
    def test_type_property(self) -> None:
        from wasm_types import FuncType, ValueType
        ft = FuncType(params=(ValueType.I32,), results=(ValueType.I32,))
        hf = _HostFunc(ft, lambda args: [i32(args[0].value * 2)])
        assert hf.type == ft

    def test_call(self) -> None:
        from wasm_types import FuncType, ValueType
        ft = FuncType(params=(ValueType.I32,), results=(ValueType.I32,))
        hf = _HostFunc(ft, lambda args: [i32(args[0].value + 1)])
        result = hf.call([i32(10)])
        assert result[0].value == 11
