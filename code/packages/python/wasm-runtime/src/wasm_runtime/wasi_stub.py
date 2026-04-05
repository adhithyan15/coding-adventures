"""wasi_stub.py --- Minimal WASI implementation.

Provides fd_write (stdout/stderr capture) and proc_exit. All other WASI
functions return ENOSYS (52 = not implemented).
"""

from __future__ import annotations

from typing import Any

from wasm_types import FuncType, ValueType

from wasm_execution import LinearMemory, Table, TrapError, WasmValue, i32

ENOSYS = 52
ESUCCESS = 0


class ProcExitError(Exception):
    """Thrown when a WASM program calls proc_exit."""

    def __init__(self, exit_code: int) -> None:
        super().__init__(f"proc_exit({exit_code})")
        self.exit_code = exit_code


class _HostFunc:
    """A simple HostFunction implementation."""

    def __init__(self, func_type: FuncType, impl: Any) -> None:
        self._type = func_type
        self._impl = impl

    @property
    def type(self) -> FuncType:
        return self._type

    def call(self, args: list[WasmValue]) -> list[WasmValue]:
        return self._impl(args)


class WasiStub:
    """Minimal WASI host implementation.

    Provides fd_write (capture stdout/stderr) and proc_exit.
    """

    def __init__(
        self,
        stdout: Any | None = None,
        stderr: Any | None = None,
    ) -> None:
        self._stdout = stdout or (lambda _t: None)
        self._stderr = stderr or (lambda _t: None)
        self._instance_memory: LinearMemory | None = None

    def set_memory(self, memory: LinearMemory) -> None:
        """Set the instance's memory (needed for fd_write)."""
        self._instance_memory = memory

    def resolve_function(self, module_name: str, name: str) -> Any | None:
        if module_name != "wasi_snapshot_preview1":
            return None
        if name == "fd_write":
            return self._make_fd_write()
        if name == "proc_exit":
            return self._make_proc_exit()
        return self._make_stub(name)

    def resolve_global(self, _module_name: str, _name: str) -> Any | None:
        return None

    def resolve_memory(self, _module_name: str, _name: str) -> Any | None:
        return None

    def resolve_table(self, _module_name: str, _name: str) -> Any | None:
        return None

    def _make_fd_write(self) -> _HostFunc:
        stub = self

        def fd_write_impl(args: list[WasmValue]) -> list[WasmValue]:
            fd = args[0].value
            iovs_ptr = args[1].value
            iovs_len = args[2].value
            nwritten_ptr = args[3].value

            if stub._instance_memory is None:
                return [i32(ENOSYS)]

            mem = stub._instance_memory
            total_written = 0

            for idx in range(iovs_len):
                buf_ptr = mem.load_i32(iovs_ptr + idx * 8) & 0xFFFFFFFF
                buf_len = mem.load_i32(iovs_ptr + idx * 8 + 4) & 0xFFFFFFFF

                chars = []
                for j in range(buf_len):
                    chars.append(chr(mem.load_i32_8u(buf_ptr + j)))

                text = "".join(chars)
                total_written += buf_len

                if fd == 1:
                    stub._stdout(text)
                elif fd == 2:
                    stub._stderr(text)

            mem.store_i32(nwritten_ptr, total_written)
            return [i32(ESUCCESS)]

        return _HostFunc(
            FuncType(
                params=(ValueType.I32, ValueType.I32, ValueType.I32, ValueType.I32),
                results=(ValueType.I32,),
            ),
            fd_write_impl,
        )

    def _make_proc_exit(self) -> _HostFunc:
        def proc_exit_impl(args: list[WasmValue]) -> list[WasmValue]:
            exit_code = args[0].value
            raise ProcExitError(exit_code)

        return _HostFunc(
            FuncType(params=(ValueType.I32,), results=()),
            proc_exit_impl,
        )

    def _make_stub(self, _name: str) -> _HostFunc:
        def stub_impl(_args: list[WasmValue]) -> list[WasmValue]:
            return [i32(ENOSYS)]

        return _HostFunc(
            FuncType(params=(), results=(ValueType.I32,)),
            stub_impl,
        )
