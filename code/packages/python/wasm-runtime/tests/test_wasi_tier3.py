"""test_wasi_tier3.py --- Tests for WASI Tier 3 functions.

Covers the eight new WASI functions added in the Tier 3 revision:
  - args_sizes_get      : report argument count and buffer size
  - args_get            : copy arguments into WASM memory
  - environ_sizes_get   : report env-var count and buffer size
  - environ_get         : copy environment variables into WASM memory
  - clock_res_get       : report clock resolution
  - clock_time_get      : read a clock value (realtime or monotonic)
  - random_get          : fill memory with random bytes
  - sched_yield         : cooperative yield (no-op)

All memory-writing tests use real LinearMemory objects so we exercise the
actual store/load path, not just the host logic in isolation.

Determinism
───────────
Clock and random functions are tested using injected fakes (FakeClock,
FakeRandom) that return hardcoded values.  This makes the tests
completely deterministic regardless of when or where they run.
"""

from __future__ import annotations

from wasm_execution import LinearMemory, i32, i64

from wasm_runtime.wasi_host import (
    EINVAL,
    ENOSYS,
    ESUCCESS,
    WasiClock,
    WasiConfig,
    WasiHost,
    WasiRandom,
)

# ===========================================================================
# Test doubles
# ===========================================================================


class FakeClock(WasiClock):
    """Deterministic clock for testing.

    Returns fixed nanosecond timestamps so assertions never depend on the
    current wall-clock time.

    Values chosen to be larger than 2^32 (4_294_967_296) to verify that
    i64 storage works correctly — a bug that only stores 32 bits would
    silently truncate the value.
    """

    def realtime_ns(self) -> int:
        # 2023-11-15 00:00:00 UTC in nanoseconds — fits in i64, not in i32.
        return 1_700_000_000_000_000_001

    def monotonic_ns(self) -> int:
        # 42 seconds since some arbitrary start — still > 2^32.
        return 42_000_000_000

    def resolution_ns(self, clock_id: int) -> int:  # noqa: ARG002
        # 1 ms = 1_000_000 ns
        return 1_000_000


class FakeRandom(WasiRandom):
    """Deterministic random source for testing.

    Always returns 0xAB bytes.  Real random data would make it impossible
    to assert specific byte values in memory.
    """

    def fill_bytes(self, n: int) -> bytes:
        return bytes([0xAB] * n)


def _make_host(**kwargs: object) -> tuple[WasiHost, LinearMemory]:
    """Create a WasiHost with FakeClock/FakeRandom and one page of memory."""
    config = WasiConfig(clock=FakeClock(), random=FakeRandom(), **kwargs)
    host = WasiHost(config)
    mem = LinearMemory(1)  # 1 page = 65536 bytes — plenty for our tests
    host.set_memory(mem)
    return host, mem


# ---------------------------------------------------------------------------
# Helper: read an i64 from LinearMemory at a given offset
# ---------------------------------------------------------------------------

def _read_i64(mem: LinearMemory, offset: int) -> int:
    """Read a little-endian i64 from memory (as unsigned 64-bit)."""
    return mem.load_i64(offset) & 0xFFFFFFFFFFFFFFFF


# ===========================================================================
# 1. args_sizes_get
# ===========================================================================


class TestArgsSizesGet:
    def test_basic(self) -> None:
        """argc=2, buf_size=12 for ["myapp", "hello"].

        "myapp\\0" = 6 bytes, "hello\\0" = 6 bytes, total = 12.
        """
        host, mem = _make_host(args=["myapp", "hello"])
        func = host.resolve_function("wasi_snapshot_preview1", "args_sizes_get")

        argc_ptr = 8
        buf_ptr = 12
        result = func.call([i32(argc_ptr), i32(buf_ptr)])

        assert result[0].value == ESUCCESS
        assert mem.load_i32(argc_ptr) == 2
        assert mem.load_i32(buf_ptr) == 12  # 6 + 6

    def test_empty_args(self) -> None:
        """No args → argc=0, buf_size=0."""
        host, mem = _make_host(args=[])
        func = host.resolve_function("wasi_snapshot_preview1", "args_sizes_get")

        result = func.call([i32(0), i32(4)])
        assert result[0].value == ESUCCESS
        assert mem.load_i32(0) == 0
        assert mem.load_i32(4) == 0

    def test_single_arg(self) -> None:
        """Single arg "x\\0" = 2 bytes."""
        host, mem = _make_host(args=["x"])
        func = host.resolve_function("wasi_snapshot_preview1", "args_sizes_get")

        result = func.call([i32(0), i32(4)])
        assert result[0].value == ESUCCESS
        assert mem.load_i32(0) == 1
        assert mem.load_i32(4) == 2


# ===========================================================================
# 2. args_get
# ===========================================================================


class TestArgsGet:
    def test_pointers_and_strings(self) -> None:
        """argv pointers and null-terminated strings land in the right places.

        With args=["myapp", "hello"]:
          argv_ptr[0] = argv_buf_ptr       (points to "myapp\\0")
          argv_ptr[1] = argv_buf_ptr + 6   (points to "hello\\0")
          Memory at argv_buf_ptr: m y a p p \\0 h e l l o \\0
        """
        host, mem = _make_host(args=["myapp", "hello"])
        func = host.resolve_function("wasi_snapshot_preview1", "args_get")

        argv_ptr = 100       # where the pointer array goes
        argv_buf_ptr = 200   # where the string data goes

        result = func.call([i32(argv_ptr), i32(argv_buf_ptr)])
        assert result[0].value == ESUCCESS

        # Check pointer array: argv[0] → 200, argv[1] → 206
        assert mem.load_i32(argv_ptr + 0) == argv_buf_ptr
        assert mem.load_i32(argv_ptr + 4) == argv_buf_ptr + 6  # "myapp\0" is 6 bytes

        # Check "myapp\0" at offset 200
        expected = b"myapp\x00hello\x00"
        for i, byte in enumerate(expected):
            actual = mem.load_i32_8u(argv_buf_ptr + i)
            assert actual == byte, f"byte {i}: expected {byte:#x}, got {actual:#x}"

    def test_null_terminator_present(self) -> None:
        """Each argument string is followed by a null byte."""
        host, mem = _make_host(args=["ab"])
        func = host.resolve_function("wasi_snapshot_preview1", "args_get")

        result = func.call([i32(0), i32(100)])
        assert result[0].value == ESUCCESS

        # "ab\0" at offset 100
        assert mem.load_i32_8u(100) == ord("a")
        assert mem.load_i32_8u(101) == ord("b")
        assert mem.load_i32_8u(102) == 0  # null terminator


# ===========================================================================
# 3. environ_sizes_get
# ===========================================================================


class TestEnvironSizesGet:
    def test_single_var(self) -> None:
        """env={"HOME": "/home/user"} → count=1, buf_size=16.

        "HOME=/home/user\\0":
          H O M E = / h o m e / u s e r \\0
          4 + 1 + 10 + 1 = 16 bytes total.
        """
        host, mem = _make_host(env={"HOME": "/home/user"})
        func = host.resolve_function("wasi_snapshot_preview1", "environ_sizes_get")

        count_ptr = 0
        buf_size_ptr = 4

        result = func.call([i32(count_ptr), i32(buf_size_ptr)])
        assert result[0].value == ESUCCESS
        assert mem.load_i32(count_ptr) == 1
        assert mem.load_i32(buf_size_ptr) == 16  # len("HOME=/home/user\0")

    def test_empty_env(self) -> None:
        """No env vars → count=0, buf_size=0."""
        host, mem = _make_host(env={})
        func = host.resolve_function("wasi_snapshot_preview1", "environ_sizes_get")

        result = func.call([i32(0), i32(4)])
        assert result[0].value == ESUCCESS
        assert mem.load_i32(0) == 0
        assert mem.load_i32(4) == 0


# ===========================================================================
# 4. environ_get
# ===========================================================================


class TestEnvironGet:
    def test_single_var_in_memory(self) -> None:
        """HOME=/home/user\\0 is written correctly into memory."""
        host, mem = _make_host(env={"HOME": "/home/user"})
        func = host.resolve_function("wasi_snapshot_preview1", "environ_get")

        environ_ptr = 100
        environ_buf_ptr = 200

        result = func.call([i32(environ_ptr), i32(environ_buf_ptr)])
        assert result[0].value == ESUCCESS

        # The single pointer in the environ array points to environ_buf_ptr
        assert mem.load_i32(environ_ptr) == environ_buf_ptr

        # Check the string bytes: "HOME=/home/user\0"
        expected = b"HOME=/home/user\x00"
        for i, byte in enumerate(expected):
            actual = mem.load_i32_8u(environ_buf_ptr + i)
            assert actual == byte, f"byte {i}: expected {byte:#x}, got {actual:#x}"

    def test_null_terminator_present(self) -> None:
        """Environment string ends with a null terminator."""
        host, mem = _make_host(env={"X": "1"})
        func = host.resolve_function("wasi_snapshot_preview1", "environ_get")

        result = func.call([i32(0), i32(100)])
        assert result[0].value == ESUCCESS

        # "X=1\0"
        assert mem.load_i32_8u(100) == ord("X")
        assert mem.load_i32_8u(101) == ord("=")
        assert mem.load_i32_8u(102) == ord("1")
        assert mem.load_i32_8u(103) == 0


# ===========================================================================
# 5. clock_time_get — realtime (id=0)
# ===========================================================================


class TestClockTimeGetRealtime:
    def test_realtime_clock(self) -> None:
        """clock_time_get(0) writes FakeClock.realtime_ns() as i64."""
        host, mem = _make_host()
        func = host.resolve_function("wasi_snapshot_preview1", "clock_time_get")

        time_ptr = 100
        # id=0 (realtime), precision=0 (i64), time_ptr
        result = func.call([i32(0), i64(0), i32(time_ptr)])
        assert result[0].value == ESUCCESS

        ns = _read_i64(mem, time_ptr)
        assert ns == 1_700_000_000_000_000_001

    def test_process_cputime_treated_as_realtime(self) -> None:
        """clock_time_get(2) also returns realtime (PROCESS_CPUTIME mapped to real)."""
        host, mem = _make_host()
        func = host.resolve_function("wasi_snapshot_preview1", "clock_time_get")

        result = func.call([i32(2), i64(0), i32(100)])
        assert result[0].value == ESUCCESS
        assert _read_i64(mem, 100) == 1_700_000_000_000_000_001

    def test_thread_cputime_treated_as_realtime(self) -> None:
        """clock_time_get(3) also returns realtime (THREAD_CPUTIME mapped to real)."""
        host, mem = _make_host()
        func = host.resolve_function("wasi_snapshot_preview1", "clock_time_get")

        result = func.call([i32(3), i64(0), i32(100)])
        assert result[0].value == ESUCCESS
        assert _read_i64(mem, 100) == 1_700_000_000_000_000_001


# ===========================================================================
# 6. clock_time_get — monotonic (id=1)
# ===========================================================================


class TestClockTimeGetMonotonic:
    def test_monotonic_clock(self) -> None:
        """clock_time_get(1) writes FakeClock.monotonic_ns() as i64."""
        host, mem = _make_host()
        func = host.resolve_function("wasi_snapshot_preview1", "clock_time_get")

        time_ptr = 200
        result = func.call([i32(1), i64(0), i32(time_ptr)])
        assert result[0].value == ESUCCESS
        assert _read_i64(mem, time_ptr) == 42_000_000_000

    def test_unknown_clock_returns_einval(self) -> None:
        """Unknown clock id returns EINVAL without writing memory."""
        host, mem = _make_host()
        func = host.resolve_function("wasi_snapshot_preview1", "clock_time_get")

        time_ptr = 0
        # Pre-fill memory with a sentinel value so we can tell it wasn't touched.
        # Use 0x7EADBEEF (signed i32-safe sentinel) instead of 0xDEADBEEF.
        mem.store_i32(time_ptr, 0x7EADBEEF)

        result = func.call([i32(99), i64(0), i32(time_ptr)])
        assert result[0].value == EINVAL
        # Memory should be unchanged — the bad clock id was rejected.
        assert mem.load_i32(time_ptr) == 0x7EADBEEF


# ===========================================================================
# 7. clock_res_get
# ===========================================================================


class TestClockResGet:
    def test_resolution_clock0(self) -> None:
        """clock_res_get(0) writes 1_000_000 ns (1 ms) as i64."""
        host, mem = _make_host()
        func = host.resolve_function("wasi_snapshot_preview1", "clock_res_get")

        resolution_ptr = 50
        result = func.call([i32(0), i32(resolution_ptr)])
        assert result[0].value == ESUCCESS
        assert _read_i64(mem, resolution_ptr) == 1_000_000

    def test_resolution_clock1(self) -> None:
        """clock_res_get(1) also returns 1_000_000 (FakeClock is id-agnostic)."""
        host, mem = _make_host()
        func = host.resolve_function("wasi_snapshot_preview1", "clock_res_get")

        result = func.call([i32(1), i32(50)])
        assert result[0].value == ESUCCESS
        assert _read_i64(mem, 50) == 1_000_000


# ===========================================================================
# 8. random_get
# ===========================================================================


class TestRandomGet:
    def test_four_bytes(self) -> None:
        """random_get fills 4 bytes with 0xAB at the given pointer."""
        host, mem = _make_host()
        func = host.resolve_function("wasi_snapshot_preview1", "random_get")

        buf_ptr = 300
        result = func.call([i32(buf_ptr), i32(4)])
        assert result[0].value == ESUCCESS

        for i in range(4):
            assert mem.load_i32_8u(buf_ptr + i) == 0xAB

    def test_zero_bytes(self) -> None:
        """random_get with buf_len=0 succeeds and writes nothing."""
        host, mem = _make_host()
        func = host.resolve_function("wasi_snapshot_preview1", "random_get")

        mem.store_i32(0, 0x12345678)  # sentinel
        result = func.call([i32(0), i32(0)])
        assert result[0].value == ESUCCESS
        assert mem.load_i32(0) == 0x12345678  # unchanged

    def test_large_fill(self) -> None:
        """random_get fills 32 bytes, all 0xAB."""
        host, mem = _make_host()
        func = host.resolve_function("wasi_snapshot_preview1", "random_get")

        buf_ptr = 1000
        result = func.call([i32(buf_ptr), i32(32)])
        assert result[0].value == ESUCCESS
        for i in range(32):
            assert mem.load_i32_8u(buf_ptr + i) == 0xAB


# ===========================================================================
# 9. sched_yield
# ===========================================================================


class TestSchedYield:
    def test_returns_success(self) -> None:
        """sched_yield() always returns i32(0) (ESUCCESS)."""
        host, _ = _make_host()
        func = host.resolve_function("wasi_snapshot_preview1", "sched_yield")
        result = func.call([])
        assert result[0].value == ESUCCESS

    def test_idempotent(self) -> None:
        """Calling sched_yield multiple times always succeeds."""
        host, _ = _make_host()
        func = host.resolve_function("wasi_snapshot_preview1", "sched_yield")
        for _ in range(10):
            result = func.call([])
            assert result[0].value == ESUCCESS


# ===========================================================================
# 10. WasiConfig and constructor backwards-compatibility
# ===========================================================================


class TestWasiConfig:
    def test_default_config(self) -> None:
        """WasiHost() with no args uses SystemClock and SystemRandom."""
        host = WasiHost()
        # Just check it doesn't crash and resolves functions
        func = host.resolve_function("wasi_snapshot_preview1", "sched_yield")
        assert func is not None

    def test_legacy_keyword_args(self) -> None:
        """WasiHost(stdout=fn, stderr=fn) still works."""
        captured: list[str] = []
        host = WasiHost(stdout=captured.append, stderr=captured.append)
        mem = LinearMemory(1)
        host.set_memory(mem)

        # Write "hi\0" at offset 100, iovec at 0
        for j, ch in enumerate(b"hi"):
            mem._data[100 + j] = ch
        mem.store_i32(0, 100)
        mem.store_i32(4, 2)

        func = host.resolve_function("wasi_snapshot_preview1", "fd_write")
        result = func.call([i32(1), i32(0), i32(1), i32(200)])
        assert result[0].value == ESUCCESS
        assert captured == ["hi"]

    def test_full_config(self) -> None:
        """WasiConfig with all fields set is accepted."""
        config = WasiConfig(
            args=["prog"],
            env={"KEY": "VALUE"},
            clock=FakeClock(),
            random=FakeRandom(),
        )
        host = WasiHost(config)
        mem = LinearMemory(1)
        host.set_memory(mem)

        func = host.resolve_function("wasi_snapshot_preview1", "args_sizes_get")
        result = func.call([i32(0), i32(4)])
        assert result[0].value == ESUCCESS
        assert mem.load_i32(0) == 1  # one arg: "prog"


# ===========================================================================
# 11. No-memory guard
# ===========================================================================


class TestNoMemoryGuard:
    """All memory-touching functions must return ENOSYS if set_memory not called."""

    def _host_no_mem(self) -> WasiHost:
        config = WasiConfig(clock=FakeClock(), random=FakeRandom())
        return WasiHost(config)  # no set_memory call

    def test_args_sizes_get_no_memory(self) -> None:
        host = self._host_no_mem()
        func = host.resolve_function("wasi_snapshot_preview1", "args_sizes_get")
        assert func.call([i32(0), i32(4)])[0].value == ENOSYS

    def test_args_get_no_memory(self) -> None:
        host = self._host_no_mem()
        func = host.resolve_function("wasi_snapshot_preview1", "args_get")
        assert func.call([i32(0), i32(0)])[0].value == ENOSYS

    def test_environ_sizes_get_no_memory(self) -> None:
        host = self._host_no_mem()
        func = host.resolve_function("wasi_snapshot_preview1", "environ_sizes_get")
        assert func.call([i32(0), i32(4)])[0].value == ENOSYS

    def test_environ_get_no_memory(self) -> None:
        host = self._host_no_mem()
        func = host.resolve_function("wasi_snapshot_preview1", "environ_get")
        assert func.call([i32(0), i32(0)])[0].value == ENOSYS

    def test_clock_res_get_no_memory(self) -> None:
        host = self._host_no_mem()
        func = host.resolve_function("wasi_snapshot_preview1", "clock_res_get")
        assert func.call([i32(0), i32(0)])[0].value == ENOSYS

    def test_clock_time_get_no_memory(self) -> None:
        host = self._host_no_mem()
        func = host.resolve_function("wasi_snapshot_preview1", "clock_time_get")
        assert func.call([i32(0), i64(0), i32(0)])[0].value == ENOSYS

    def test_random_get_no_memory(self) -> None:
        host = self._host_no_mem()
        func = host.resolve_function("wasi_snapshot_preview1", "random_get")
        assert func.call([i32(0), i32(4)])[0].value == ENOSYS


# ===========================================================================
# 12. Square end-to-end still passes (regression)
# ===========================================================================


class TestSquareRegression:
    """Verify that the square wasm end-to-end test still works after our changes."""

    def test_square_5(self) -> None:
        from wasm_runtime import WasmRuntime

        def _leb128(n: int) -> bytes:
            result = bytearray()
            while True:
                byte = n & 0x7F
                n >>= 7
                if n > 0:
                    byte |= 0x80
                result.append(byte)
                if n == 0:
                    break
            return bytes(result)

        def _section(sid: int, payload: bytes) -> bytes:
            return bytes([sid]) + _leb128(len(payload)) + payload

        header = bytes([0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00])
        type_payload = _leb128(1) + bytes([0x60, 0x01, 0x7F, 0x01, 0x7F])
        func_payload = _leb128(1) + _leb128(0)
        export_name = b"square"
        export_payload = (
            _leb128(1) + _leb128(len(export_name)) + export_name
            + bytes([0x00]) + _leb128(0)
        )
        body_code = bytes([0x20, 0x00, 0x20, 0x00, 0x6C, 0x0B])
        body = _leb128(0) + body_code
        code_payload = _leb128(1) + _leb128(len(body)) + body

        wasm = (header + _section(1, type_payload) + _section(3, func_payload)
                + _section(7, export_payload) + _section(10, code_payload))

        runtime = WasmRuntime()
        result = runtime.load_and_run(wasm, "square", [5])
        assert result == [25]
