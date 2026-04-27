"""test_wasi.py --- Tests for the WASI host implementation.

Covers: fd_write/fd_read, proc_exit, ENOSYS fallbacks,
resolve_function for known/unknown functions, resolve_global/memory/table.
"""

from __future__ import annotations

import pytest

from wasm_execution import LinearMemory, i32
from wasm_runtime.wasi_host import EBADF, ENOSYS, ESUCCESS, ProcExitError, WasiConfig, WasiHost, _HostFunc


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
# WasiHost resolution
# ===========================================================================


class TestWasiHostResolve:
    def test_resolve_fd_write(self) -> None:
        host = WasiHost()
        func = host.resolve_function("wasi_snapshot_preview1", "fd_write")
        assert func is not None

    def test_resolve_fd_read(self) -> None:
        host = WasiHost()
        func = host.resolve_function("wasi_snapshot_preview1", "fd_read")
        assert func is not None

    def test_resolve_proc_exit(self) -> None:
        host = WasiHost()
        func = host.resolve_function("wasi_snapshot_preview1", "proc_exit")
        assert func is not None

    def test_resolve_unknown_returns_stub(self) -> None:
        host = WasiHost()
        func = host.resolve_function("wasi_snapshot_preview1", "args_get")
        assert func is not None
        # The fallback should return ENOSYS
        result = func.call([])
        assert result[0].value == ENOSYS

    def test_resolve_wrong_module_returns_none(self) -> None:
        host = WasiHost()
        func = host.resolve_function("some_other_module", "fd_write")
        assert func is None

    def test_resolve_global_returns_none(self) -> None:
        host = WasiHost()
        assert host.resolve_global("wasi_snapshot_preview1", "x") is None

    def test_resolve_memory_returns_none(self) -> None:
        host = WasiHost()
        assert host.resolve_memory("wasi_snapshot_preview1", "mem") is None

    def test_resolve_table_returns_none(self) -> None:
        host = WasiHost()
        assert host.resolve_table("wasi_snapshot_preview1", "tbl") is None


# ===========================================================================
# fd_write
# ===========================================================================


class TestFdWrite:
    def _setup_memory_with_iovec(
        self, text: str, fd: int = 1
    ) -> tuple[WasiHost, LinearMemory, list]:
        """Set up memory with a single iovec pointing to `text`."""
        captured: list[str] = []
        host = WasiHost(stdout=captured.append, stderr=captured.append)
        mem = LinearMemory(1)
        host.set_memory(mem)

        # Write the text bytes starting at offset 100
        for j, ch in enumerate(text):
            mem._data[100 + j] = ord(ch)

        # iovec at offset 0: buf_ptr=100, buf_len=len(text)
        mem.store_i32(0, 100)
        mem.store_i32(4, len(text))

        return host, mem, captured

    def test_fd_write_stdout(self) -> None:
        host, mem, captured = self._setup_memory_with_iovec("Hello")
        func = host.resolve_function("wasi_snapshot_preview1", "fd_write")
        # fd=1 (stdout), iovs_ptr=0, iovs_len=1, nwritten_ptr=200
        result = func.call([i32(1), i32(0), i32(1), i32(200)])
        assert result[0].value == ESUCCESS
        assert captured == ["Hello"]
        assert mem.load_i32(200) == 5

    def test_fd_write_stderr(self) -> None:
        host, mem, captured = self._setup_memory_with_iovec("Error")
        func = host.resolve_function("wasi_snapshot_preview1", "fd_write")
        result = func.call([i32(2), i32(0), i32(1), i32(200)])
        assert result[0].value == ESUCCESS
        assert captured == ["Error"]

    def test_fd_write_no_memory(self) -> None:
        """fd_write without set_memory should return ENOSYS."""
        host = WasiHost()
        func = host.resolve_function("wasi_snapshot_preview1", "fd_write")
        result = func.call([i32(1), i32(0), i32(0), i32(0)])
        assert result[0].value == ENOSYS


class TestFdRead:
    def _setup_memory_with_read_iovec(
        self,
        reader,
    ) -> tuple[WasiHost, LinearMemory]:
        host = WasiHost(config=WasiConfig(stdin=reader))
        mem = LinearMemory(1)
        host.set_memory(mem)
        mem.store_i32(0, 100)
        mem.store_i32(4, 4)
        return host, mem

    def test_fd_read_stdin(self) -> None:
        host, mem = self._setup_memory_with_read_iovec(lambda _n: b"Hi")
        func = host.resolve_function("wasi_snapshot_preview1", "fd_read")
        result = func.call([i32(0), i32(0), i32(1), i32(200)])
        assert result[0].value == ESUCCESS
        assert mem.load_i32_8u(100) == ord("H")
        assert mem.load_i32_8u(101) == ord("i")
        assert mem.load_i32(200) == 2

    def test_fd_read_eof_writes_zero_bytes(self) -> None:
        host, mem = self._setup_memory_with_read_iovec(lambda _n: b"")
        mem.store_i32_8(100, 255)
        func = host.resolve_function("wasi_snapshot_preview1", "fd_read")
        result = func.call([i32(0), i32(0), i32(1), i32(200)])
        assert result[0].value == ESUCCESS
        assert mem.load_i32_8u(100) == 255
        assert mem.load_i32(200) == 0

    def test_fd_read_rejects_non_stdin_fd(self) -> None:
        host, _mem = self._setup_memory_with_read_iovec(lambda _n: b"abc")
        func = host.resolve_function("wasi_snapshot_preview1", "fd_read")
        result = func.call([i32(1), i32(0), i32(1), i32(200)])
        assert result[0].value == EBADF

    def test_fd_read_no_memory(self) -> None:
        host = WasiHost(config=WasiConfig(stdin=lambda _n: b"x"))
        func = host.resolve_function("wasi_snapshot_preview1", "fd_read")
        result = func.call([i32(0), i32(0), i32(1), i32(0)])
        assert result[0].value == ENOSYS


# ===========================================================================
# proc_exit
# ===========================================================================


class TestProcExit:
    def test_proc_exit_raises(self) -> None:
        host = WasiHost()
        func = host.resolve_function("wasi_snapshot_preview1", "proc_exit")
        with pytest.raises(ProcExitError) as exc_info:
            func.call([i32(42)])
        assert exc_info.value.exit_code == 42

    def test_proc_exit_zero(self) -> None:
        host = WasiHost()
        func = host.resolve_function("wasi_snapshot_preview1", "proc_exit")
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
