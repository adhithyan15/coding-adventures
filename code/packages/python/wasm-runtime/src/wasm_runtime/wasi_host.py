"""wasi_host.py --- WASI host implementation (Tiers 1-3).

WebAssembly System Interface (WASI) provides a portable POSIX-like API so
that WASM modules can talk to the outside world (files, clocks, random
numbers, command-line args, environment variables) without depending on any
specific operating system.

This module implements the runtime's WASI host surface so real programs
can talk to the outside world through imported WASI functions.

  Tier 1 (already existed)
  ─────────────────────────
  - fd_write        — write iovec buffers to stdout or stderr
  - fd_read         — read iovec buffers from stdin
  - proc_exit       — terminate the WASM program with an exit code

  Tier 3 (new in this revision)
  ─────────────────────────────
  - args_sizes_get  — how many args and how much buffer space they need
  - args_get        — copy null-terminated arg strings into WASM memory
  - environ_sizes_get — same, but for KEY=VALUE environment strings
  - environ_get     — copy null-terminated env strings into WASM memory
  - clock_res_get   — resolution of a WASI clock (always 1 ms here)
  - clock_time_get  — current time on a WASI clock (real or monotonic)
  - random_get      — fill a buffer with cryptographically random bytes
  - sched_yield     — cooperative scheduler hint (no-op in single-thread)

Design: pluggable clock and random
───────────────────────────────────
Clock and random number generation are injected through abstract base
classes (``WasiClock`` and ``WasiRandom``).  The defaults call the real
OS (``time.time_ns()``, ``secrets.token_bytes()``), but tests can swap
them for deterministic fakes — no monkey-patching required.

This mirrors the principle behind Dependency Injection: "don't call us,
we'll call you".  A unit test provides a ``FakeClock`` that always returns
the same timestamp, so assertions are stable across machines and time zones.

Error codes used
─────────────────
WASI uses numeric errno codes (same idea as POSIX):
  0  = ESUCCESS  — call succeeded
 28  = EINVAL    — invalid argument (unknown clock id)
 52  = ENOSYS    — function not implemented
"""

from __future__ import annotations

import secrets
import time
from abc import ABC, abstractmethod
from collections.abc import Callable
from dataclasses import dataclass, field
from typing import Any

from wasm_execution import LinearMemory, WasmValue, i32
from wasm_types import FuncType, ValueType

# ---------------------------------------------------------------------------
# WASI errno constants
# ---------------------------------------------------------------------------
ESUCCESS = 0    # no error
EBADF = 8       # bad file descriptor
EINVAL = 28     # invalid argument  (e.g. unknown clock id)
ENOSYS = 52     # function not (yet) implemented


# ---------------------------------------------------------------------------
# Clock and random abstractions
# ---------------------------------------------------------------------------


class WasiClock(ABC):
    """Abstract interface for WASI clock queries.

    Implementors must provide nanosecond timestamps.  Two flavours exist:

    * **realtime** — wall-clock time since the Unix epoch (1970-01-01 00:00 UTC).
      Useful for "what time is it now?" but can jump backwards if NTP adjusts
      the system clock.

    * **monotonic** — time since some arbitrary start point.  Guaranteed to
      never go backwards.  Useful for measuring elapsed time (e.g. "how long
      did this function take?").
    """

    @abstractmethod
    def realtime_ns(self) -> int:
        """Return nanoseconds since the Unix epoch as an integer."""
        ...

    @abstractmethod
    def monotonic_ns(self) -> int:
        """Return nanoseconds since an arbitrary start (monotonic) as an integer."""
        ...

    @abstractmethod
    def resolution_ns(self, clock_id: int) -> int:
        """Return the clock's resolution in nanoseconds.

        Resolution is the smallest time difference the clock can measure.
        A 1 ms resolution clock (1_000_000 ns) cannot distinguish events
        that are 500 µs apart.
        """
        ...


class WasiRandom(ABC):
    """Abstract interface for cryptographically secure random bytes.

    "Cryptographically secure" means an attacker who sees past output cannot
    predict future output, even with unlimited compute.  This is stronger
    than ``random.random()`` which is fine for simulations but not for
    generating secret keys or session tokens.
    """

    @abstractmethod
    def fill_bytes(self, n: int) -> bytes:
        """Return ``n`` cryptographically random bytes."""
        ...


class SystemClock(WasiClock):
    """Production clock: delegates to the OS via the ``time`` module.

    ``time.time_ns()`` returns wall-clock nanoseconds.
    ``time.monotonic_ns()`` returns a monotonically increasing counter.
    Resolution is reported as 1 ms (1_000_000 ns) — a conservative
    estimate that works on all platforms Python supports.
    """

    def realtime_ns(self) -> int:
        return time.time_ns()

    def monotonic_ns(self) -> int:
        return time.monotonic_ns()

    def resolution_ns(self, clock_id: int) -> int:  # noqa: ARG002
        # 1 millisecond = 1_000_000 nanoseconds.
        # Real OS clocks are often finer, but 1 ms is a safe lower bound.
        return 1_000_000


class SystemRandom(WasiRandom):
    """Production randomness: delegates to ``secrets.token_bytes()``.

    ``secrets`` is backed by the OS CSPRNG (``/dev/urandom`` on Linux/macOS,
    ``BCryptGenRandom`` on Windows).  It is the Python-recommended source for
    any security-sensitive random data.
    """

    def fill_bytes(self, n: int) -> bytes:
        return secrets.token_bytes(n)


# ---------------------------------------------------------------------------
# WasiConfig — bundles all runtime options in one place
# ---------------------------------------------------------------------------


@dataclass
class WasiConfig:
    """All knobs you can pass to a WasiHost.

    Fields
    ──────
    args    — command-line arguments as a list of strings.  args[0] is
              conventionally the program name.
    env     — environment variables as a {key: value} dict.
    stdin   — callable that receives a byte count and returns up to that
              many bytes of input. If None, reads always return EOF.
    stdout  — callable that receives each piece of text written to fd 1.
              If None, output is silently discarded.
    stderr  — same but for fd 2.
    clock   — WasiClock implementation.  Defaults to SystemClock (real time).
    random  — WasiRandom implementation.  Defaults to SystemRandom (OS CSPRNG).
    """

    args: list[str] = field(default_factory=list)
    env: dict[str, str] = field(default_factory=dict)
    stdin: Callable[[int], bytes | bytearray | str | None] | None = None
    stdout: Callable[[str], None] | None = None
    stderr: Callable[[str], None] | None = None
    clock: WasiClock = field(default_factory=SystemClock)
    random: WasiRandom = field(default_factory=SystemRandom)


# ---------------------------------------------------------------------------
# ProcExitError
# ---------------------------------------------------------------------------


class ProcExitError(Exception):
    """Thrown when a WASM program calls proc_exit.

    Unlike C's ``exit()``, we cannot actually terminate just the WASM module
    without also stopping the Python host.  Instead we raise this exception and
    let the caller decide what to do with the exit code.
    """

    def __init__(self, exit_code: int) -> None:
        super().__init__(f"proc_exit({exit_code})")
        self.exit_code = exit_code


# ---------------------------------------------------------------------------
# _HostFunc — tiny wrapper that glues a Python callable to a WASM FuncType
# ---------------------------------------------------------------------------


class _HostFunc:
    """A lightweight host function: a FuncType paired with a Python callable.

    WASM host functions must advertise their type (parameter and result
    types) so the module linker can type-check imports at instantiation time.
    ``_HostFunc`` wraps any Python callable with the necessary metadata.
    """

    def __init__(self, func_type: FuncType, impl: Any) -> None:  # noqa: ANN401
        self._type = func_type
        self._impl = impl

    @property
    def type(self) -> FuncType:
        return self._type

    def call(self, args: list[WasmValue]) -> list[WasmValue]:
        return self._impl(args)


# ---------------------------------------------------------------------------
# WasiHost — the main host object
# ---------------------------------------------------------------------------


class WasiHost:
    """WASI host implementation covering Tiers 1 and 3.

    Usage (simple — backwards-compatible with old keyword-arg style)
    ────────────────────────────────────────────────────────────────
    host = WasiHost(stdout=print, stderr=print)

    Usage (full config)
    ────────────────────────────────────────────────────────────────
    from wasm_runtime.wasi_host import WasiConfig, FakeClock
    cfg = WasiConfig(args=["myapp", "--flag"], env={"HOME": "/tmp"},
                     stdout=print, clock=FakeClock())
    host = WasiHost(cfg)

    Backwards compatibility
    ───────────────────────
    The old constructor accepted ``stdout`` and ``stderr`` as keyword
    arguments directly.  That form still works: passing keyword-only
    ``stdout``/``stderr`` to ``WasiHost()`` creates a ``WasiConfig``
    internally.  If you pass a ``WasiConfig`` as the first positional
    argument, keyword args are ignored.
    """

    def __init__(
        self,
        config: WasiConfig | None = None,
        *,
        stdout: Callable[[str], None] | None = None,
        stderr: Callable[[str], None] | None = None,
    ) -> None:
        # If no config was given, build one from the legacy keyword arguments.
        # This keeps straightforward call-sites like WasiHost(stdout=print)
        # working without any changes.
        if config is None:
            config = WasiConfig(stdout=stdout, stderr=stderr)

        self._stdin = config.stdin or (lambda _n: b"")
        self._stdout = config.stdout or (lambda _t: None)
        self._stderr = config.stderr or (lambda _t: None)
        self._args = config.args
        self._env = config.env
        self._clock = config.clock
        self._random = config.random
        self._instance_memory: LinearMemory | None = None

    # ------------------------------------------------------------------
    # Memory binding — called by the runtime after the module is linked
    # ------------------------------------------------------------------

    def set_memory(self, memory: LinearMemory) -> None:
        """Bind the WASM linear memory so WASI functions can read/write it."""
        self._instance_memory = memory

    # ------------------------------------------------------------------
    # Import resolution — the runtime calls these to link imports
    # ------------------------------------------------------------------

    def resolve_function(self, module_name: str, name: str) -> Any | None:  # noqa: ANN401
        """Return a _HostFunc for the named WASI function, or None."""
        if module_name != "wasi_snapshot_preview1":
            return None

        # Map each WASI function name to its maker method.
        dispatch: dict[str, Any] = {
            "fd_write": self._make_fd_write,
            "fd_read": self._make_fd_read,
            "proc_exit": self._make_proc_exit,
            "args_sizes_get": self._make_args_sizes_get,
            "args_get": self._make_args_get,
            "environ_sizes_get": self._make_environ_sizes_get,
            "environ_get": self._make_environ_get,
            "clock_res_get": self._make_clock_res_get,
            "clock_time_get": self._make_clock_time_get,
            "random_get": self._make_random_get,
            "sched_yield": self._make_sched_yield,
        }
        maker = dispatch.get(name)
        if maker is not None:
            return maker()
        # Everything else returns ENOSYS so the module can link but will
        # get an explicit "not implemented" error if called at runtime.
        return self._make_stub(name)

    def resolve_global(self, _module_name: str, _name: str) -> Any | None:  # noqa: ANN401
        return None

    def resolve_memory(self, _module_name: str, _name: str) -> Any | None:  # noqa: ANN401
        return None

    def resolve_table(self, _module_name: str, _name: str) -> Any | None:  # noqa: ANN401
        return None

    # ------------------------------------------------------------------
    # Tier 1: fd_write, fd_read, and proc_exit
    # ------------------------------------------------------------------

    def _make_fd_write(self) -> _HostFunc:
        """Build the fd_write host function.

        fd_write(fd, iovs_ptr, iovs_len, nwritten_ptr) -> errno

        An *iovec* (I/O vector) is a {pointer, length} pair that describes
        one contiguous slice of memory to write.  fd_write accepts an array
        of iovecs and writes them all in order — a single syscall that avoids
        multiple round-trips for scatter/gather I/O.

        Memory layout of one iovec at offset ``base``:
          [base+0 .. base+3]  buf_ptr   (i32, little-endian)
          [base+4 .. base+7]  buf_len   (i32, little-endian)
        """
        host = self

        def fd_write_impl(args: list[WasmValue]) -> list[WasmValue]:
            fd = args[0].value
            iovs_ptr = args[1].value
            iovs_len = args[2].value
            nwritten_ptr = args[3].value

            if host._instance_memory is None:
                return [i32(ENOSYS)]

            mem = host._instance_memory
            total_written = 0

            for idx in range(iovs_len):
                # Each iovec is 8 bytes: 4-byte ptr + 4-byte length.
                buf_ptr = mem.load_i32(iovs_ptr + idx * 8) & 0xFFFFFFFF
                buf_len = mem.load_i32(iovs_ptr + idx * 8 + 4) & 0xFFFFFFFF

                # Read individual bytes and decode as Latin-1 (1 byte → 1 char).
                chars = []
                for j in range(buf_len):
                    chars.append(chr(mem.load_i32_8u(buf_ptr + j)))

                text = "".join(chars)
                total_written += buf_len

                if fd == 1:
                    host._stdout(text)
                elif fd == 2:
                    host._stderr(text)

            mem.store_i32(nwritten_ptr, total_written)
            return [i32(ESUCCESS)]

        return _HostFunc(
            FuncType(
                params=(ValueType.I32, ValueType.I32, ValueType.I32, ValueType.I32),
                results=(ValueType.I32,),
            ),
            fd_write_impl,
        )

    def _make_fd_read(self) -> _HostFunc:
        """Build the fd_read host function.

        fd_read(fd, iovs_ptr, iovs_len, nread_ptr) -> errno

        WASI uses the same iovec layout for reads and writes. Each entry
        describes a writable buffer in WASM memory. We copy up to ``buf_len``
        bytes into each buffer in order and stop early on EOF.
        """
        host = self

        def fd_read_impl(args: list[WasmValue]) -> list[WasmValue]:
            fd = args[0].value
            iovs_ptr = args[1].value
            iovs_len = args[2].value
            nread_ptr = args[3].value

            if host._instance_memory is None:
                return [i32(ENOSYS)]
            if fd != 0:
                return [i32(EBADF)]

            mem = host._instance_memory
            total_read = 0

            for idx in range(iovs_len):
                buf_ptr = mem.load_i32(iovs_ptr + idx * 8) & 0xFFFFFFFF
                buf_len = mem.load_i32(iovs_ptr + idx * 8 + 4) & 0xFFFFFFFF

                chunk = host._stdin(buf_len)
                if chunk is None:
                    chunk_bytes = b""
                elif isinstance(chunk, str):
                    chunk_bytes = chunk.encode("latin-1")
                else:
                    chunk_bytes = bytes(chunk)

                chunk_bytes = chunk_bytes[:buf_len]
                for offset, byte in enumerate(chunk_bytes):
                    mem.store_i32_8(buf_ptr + offset, byte)

                total_read += len(chunk_bytes)
                if len(chunk_bytes) < buf_len:
                    break

            mem.store_i32(nread_ptr, total_read)
            return [i32(ESUCCESS)]

        return _HostFunc(
            FuncType(
                params=(ValueType.I32, ValueType.I32, ValueType.I32, ValueType.I32),
                results=(ValueType.I32,),
            ),
            fd_read_impl,
        )

    def _make_proc_exit(self) -> _HostFunc:
        """Build the proc_exit host function.

        proc_exit(code) — terminate the WASM program.

        WASI programs call this instead of returning from main so that the
        exit code is communicated even if the call stack is deeply nested.
        We translate it into a Python exception so the host can catch it.
        """
        def proc_exit_impl(args: list[WasmValue]) -> list[WasmValue]:
            exit_code = args[0].value
            raise ProcExitError(exit_code)

        return _HostFunc(
            FuncType(params=(ValueType.I32,), results=()),
            proc_exit_impl,
        )

    # ------------------------------------------------------------------
    # Tier 3: args
    # ------------------------------------------------------------------

    def _make_args_sizes_get(self) -> _HostFunc:
        """Build args_sizes_get.

        args_sizes_get(argc_ptr, argv_buf_size_ptr) -> errno

        The WASM program calls this first to find out:
          * How many arguments are there?  (argc)
          * How many bytes of buffer does it need for all arg strings?

        It then allocates that buffer, calls args_get, and reads the strings.

        Buffer size = sum of (len(arg_as_utf8) + 1) for each arg.
        The ``+1`` is for the null terminator (C string convention).

        Example: args = ["myapp", "hello"]
          "myapp\0" = 6 bytes
          "hello\0" = 6 bytes
          total = 12 bytes, argc = 2
        """
        host = self

        def args_sizes_get_impl(args: list[WasmValue]) -> list[WasmValue]:
            if host._instance_memory is None:
                return [i32(ENOSYS)]
            mem = host._instance_memory
            argc_ptr = args[0].value & 0xFFFFFFFF
            argv_buf_size_ptr = args[1].value & 0xFFFFFFFF

            argc = len(host._args)
            buf_size = sum(len(arg.encode("utf-8")) + 1 for arg in host._args)

            mem.store_i32(argc_ptr, argc)
            mem.store_i32(argv_buf_size_ptr, buf_size)
            return [i32(ESUCCESS)]

        return _HostFunc(
            FuncType(
                params=(ValueType.I32, ValueType.I32),
                results=(ValueType.I32,),
            ),
            args_sizes_get_impl,
        )

    def _make_args_get(self) -> _HostFunc:
        """Build args_get.

        args_get(argv_ptr, argv_buf_ptr) -> errno

        Writes two things into WASM memory:
          1. An array of i32 pointers at ``argv_ptr``: argv[0], argv[1], ...
             Each points to the corresponding null-terminated string.
          2. The actual UTF-8 string bytes (with null terminators) packed
             contiguously starting at ``argv_buf_ptr``.

        Memory layout after the call (args = ["a", "bb"]):
          argv_ptr:       [ptr_to_"a\\0"] [ptr_to_"bb\\0"]
          argv_buf_ptr:   'a' '\\0' 'b' 'b' '\\0'
        """
        host = self

        def args_get_impl(args: list[WasmValue]) -> list[WasmValue]:
            if host._instance_memory is None:
                return [i32(ENOSYS)]
            mem = host._instance_memory
            argv_ptr = args[0].value & 0xFFFFFFFF
            argv_buf_ptr = args[1].value & 0xFFFFFFFF

            offset = argv_buf_ptr
            for i, arg in enumerate(host._args):
                # Write the pointer for this arg into the argv array.
                # Each pointer is 4 bytes (i32) in WASM's 32-bit address space.
                mem.store_i32(argv_ptr + i * 4, offset)

                # Write the null-terminated UTF-8 bytes into the buffer.
                encoded = arg.encode("utf-8") + b"\x00"
                for j, byte in enumerate(encoded):
                    mem.store_i32_8(offset + j, byte)
                offset += len(encoded)

            return [i32(ESUCCESS)]

        return _HostFunc(
            FuncType(
                params=(ValueType.I32, ValueType.I32),
                results=(ValueType.I32,),
            ),
            args_get_impl,
        )

    # ------------------------------------------------------------------
    # Tier 3: environ
    # ------------------------------------------------------------------

    def _make_environ_sizes_get(self) -> _HostFunc:
        """Build environ_sizes_get.

        environ_sizes_get(count_ptr, buf_size_ptr) -> errno

        Exactly like args_sizes_get but for environment variables.
        Each env entry is formatted as ``"KEY=VALUE\\0"``.

        Example: env = {"HOME": "/home/user"}
          "HOME=/home/user\\0" = 15 bytes, count = 1
        """
        host = self

        def environ_sizes_get_impl(args: list[WasmValue]) -> list[WasmValue]:
            if host._instance_memory is None:
                return [i32(ENOSYS)]
            mem = host._instance_memory
            count_ptr = args[0].value & 0xFFFFFFFF
            buf_size_ptr = args[1].value & 0xFFFFFFFF

            env_strings = [f"{k}={v}" for k, v in host._env.items()]
            count = len(env_strings)
            buf_size = sum(len(s.encode("utf-8")) + 1 for s in env_strings)

            mem.store_i32(count_ptr, count)
            mem.store_i32(buf_size_ptr, buf_size)
            return [i32(ESUCCESS)]

        return _HostFunc(
            FuncType(
                params=(ValueType.I32, ValueType.I32),
                results=(ValueType.I32,),
            ),
            environ_sizes_get_impl,
        )

    def _make_environ_get(self) -> _HostFunc:
        """Build environ_get.

        environ_get(environ_ptr, environ_buf_ptr) -> errno

        Mirrors args_get but for environment variable strings ("KEY=VALUE\\0").
        The layout in memory is identical to args_get: an array of i32
        pointers followed by the packed null-terminated strings.
        """
        host = self

        def environ_get_impl(args: list[WasmValue]) -> list[WasmValue]:
            if host._instance_memory is None:
                return [i32(ENOSYS)]
            mem = host._instance_memory
            environ_ptr = args[0].value & 0xFFFFFFFF
            environ_buf_ptr = args[1].value & 0xFFFFFFFF

            env_strings = [f"{k}={v}" for k, v in host._env.items()]
            offset = environ_buf_ptr
            for i, s in enumerate(env_strings):
                mem.store_i32(environ_ptr + i * 4, offset)
                encoded = s.encode("utf-8") + b"\x00"
                for j, byte in enumerate(encoded):
                    mem.store_i32_8(offset + j, byte)
                offset += len(encoded)

            return [i32(ESUCCESS)]

        return _HostFunc(
            FuncType(
                params=(ValueType.I32, ValueType.I32),
                results=(ValueType.I32,),
            ),
            environ_get_impl,
        )

    # ------------------------------------------------------------------
    # Tier 3: clocks
    # ------------------------------------------------------------------

    def _make_clock_res_get(self) -> _HostFunc:
        """Build clock_res_get.

        clock_res_get(id, resolution_ptr) -> errno

        Writes the resolution of clock ``id`` (in nanoseconds) as a 64-bit
        little-endian integer at ``resolution_ptr``.

        Resolution tells callers how fine-grained the clock is.  If resolution
        is 1_000_000 ns (1 ms), there's no point asking for the time more
        than ~1000 times per second — you'll just get repeated values.

        WASI clock IDs:
          0 = REALTIME    (wall clock)
          1 = MONOTONIC   (never goes backwards)
          2 = PROCESS_CPUTIME  (CPU time consumed by this process)
          3 = THREAD_CPUTIME   (CPU time consumed by this thread)
        """
        host = self

        def clock_res_get_impl(args: list[WasmValue]) -> list[WasmValue]:
            if host._instance_memory is None:
                return [i32(ENOSYS)]
            mem = host._instance_memory
            clock_id = args[0].value
            resolution_ptr = args[1].value & 0xFFFFFFFF

            resolution = host._clock.resolution_ns(clock_id)
            # WASI clock timestamps are i64 (nanoseconds fit comfortably).
            mem.store_i64(resolution_ptr, resolution)
            return [i32(ESUCCESS)]

        return _HostFunc(
            FuncType(
                params=(ValueType.I32, ValueType.I32),
                results=(ValueType.I32,),
            ),
            clock_res_get_impl,
        )

    def _make_clock_time_get(self) -> _HostFunc:
        """Build clock_time_get.

        clock_time_get(id, precision, time_ptr) -> errno

        Reads the current value of clock ``id`` and writes it as a 64-bit
        nanosecond count at ``time_ptr``.

        ``precision`` is a hint (i64, in nanoseconds) that tells the
        implementation how fine-grained the caller needs the result.  We
        ignore it here — we always return the best precision we have.

        Clock IDs handled:
          0 (REALTIME)         → wall-clock nanoseconds since Unix epoch
          1 (MONOTONIC)        → monotonic nanoseconds
          2 (PROCESS_CPUTIME)  → treated as realtime (simplification)
          3 (THREAD_CPUTIME)   → treated as realtime (simplification)
          other                → EINVAL (28)

        The precision parameter is i64 in the WASI spec, so the FuncType
        must declare ValueType.I64 for that slot.
        """
        host = self

        def clock_time_get_impl(args: list[WasmValue]) -> list[WasmValue]:
            if host._instance_memory is None:
                return [i32(ENOSYS)]
            mem = host._instance_memory
            clock_id = args[0].value
            # args[1] is precision (i64) — we accept it but don't use it.
            time_ptr = args[2].value & 0xFFFFFFFF

            if clock_id == 1:
                # Monotonic: guaranteed not to go backwards — ideal for
                # measuring elapsed durations (timeouts, benchmarks).
                ns = host._clock.monotonic_ns()
            elif clock_id in (0, 2, 3):
                # Realtime, process CPU time, thread CPU time.
                # We map the latter two to realtime for simplicity.
                ns = host._clock.realtime_ns()
            else:
                # Unknown clock — return EINVAL without touching memory.
                return [i32(EINVAL)]

            mem.store_i64(time_ptr, ns)
            return [i32(ESUCCESS)]

        return _HostFunc(
            FuncType(
                params=(ValueType.I32, ValueType.I64, ValueType.I32),
                results=(ValueType.I32,),
            ),
            clock_time_get_impl,
        )

    # ------------------------------------------------------------------
    # Tier 3: random
    # ------------------------------------------------------------------

    def _make_random_get(self) -> _HostFunc:
        """Build random_get.

        random_get(buf_ptr, buf_len) -> errno

        Fills ``buf_len`` bytes of WASM memory starting at ``buf_ptr`` with
        cryptographically random data.

        WASM programs use this to seed their own PRNGs or to generate nonces,
        session tokens, or cryptographic keys — anything where predictability
        would be a security risk.

        We delegate to ``WasiRandom.fill_bytes()`` which defaults to
        ``secrets.token_bytes()`` — the OS CSPRNG.
        """
        host = self

        def random_get_impl(args: list[WasmValue]) -> list[WasmValue]:
            if host._instance_memory is None:
                return [i32(ENOSYS)]
            mem = host._instance_memory
            buf_ptr = args[0].value & 0xFFFFFFFF
            buf_len = args[1].value & 0xFFFFFFFF

            rand_bytes = host._random.fill_bytes(buf_len)
            for i, byte in enumerate(rand_bytes):
                mem.store_i32_8(buf_ptr + i, byte)
            return [i32(ESUCCESS)]

        return _HostFunc(
            FuncType(
                params=(ValueType.I32, ValueType.I32),
                results=(ValueType.I32,),
            ),
            random_get_impl,
        )

    # ------------------------------------------------------------------
    # Tier 3: scheduler
    # ------------------------------------------------------------------

    def _make_sched_yield(self) -> _HostFunc:
        """Build sched_yield.

        sched_yield() -> errno

        In a multi-threaded environment, yield invites the OS scheduler to
        run another thread.  In our single-threaded Python host there's only
        one WASM module running, so this is a deliberate no-op.  We return
        success (0) immediately so programs that call it don't error out.

        Think of it like a polite "go ahead, I'm not in a hurry" that
        other threads can (but don't have to) act on.
        """
        def sched_yield_impl(_args: list[WasmValue]) -> list[WasmValue]:
            return [i32(ESUCCESS)]

        return _HostFunc(
            FuncType(params=(), results=(ValueType.I32,)),
            sched_yield_impl,
        )

    # ------------------------------------------------------------------
    # Catch-all stub for unimplemented functions
    # ------------------------------------------------------------------

    def _make_stub(self, _name: str) -> _HostFunc:
        """Return a stub that always reports ENOSYS (not implemented).

        This lets any WASM module import arbitrary WASI functions and link
        successfully.  If the function is actually called at runtime, ENOSYS
        is returned — the program can then decide whether to abort or fall
        back to another code path.
        """
        def stub_impl(_args: list[WasmValue]) -> list[WasmValue]:
            return [i32(ENOSYS)]

        return _HostFunc(
            FuncType(params=(), results=(ValueType.I32,)),
            stub_impl,
        )
