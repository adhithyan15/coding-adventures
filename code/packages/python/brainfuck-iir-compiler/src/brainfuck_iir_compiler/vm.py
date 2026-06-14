"""``BrainfuckVM`` — a thin wrapper around ``vm-core`` configured for Brainfuck.

What this wrapper actually does
===============================
``vm-core`` is general-purpose: it runs whatever ``IIRModule`` you give it
and asks the host to wire up any builtins the program needs.  Every host
language using the LANG pipeline therefore needs a small adapter that
configures the VM the way that language expects.  For Brainfuck that
means three things:

1. **u8 wraparound** — Brainfuck cells are 8 bits; ``+`` on 255 yields 0.
   ``VMCore(u8_wrap=True)`` does this automatically.
2. **stdio builtins** — ``.`` and ``,`` compile to ``call_builtin
   "putchar"`` / ``"getchar"``.  The wrapper wires those names to per-run
   input and output buffers.
3. **Tape bounds** — the data pointer must stay within
   ``[0, tape_size)``.  Out-of-bounds reads return 0 (matching the
   common "lazy infinite tape" Brainfuck convention) and out-of-bounds
   writes raise :class:`BrainfuckError`.

JIT mode (``jit=True``)
=======================
Constructed with ``jit=True``, the wrapper attaches ``jit-core`` (LANG03)
with the in-house ``WASMBackend`` and tries to specialise ``main`` to
WebAssembly bytes that run on the in-house ``wasm-runtime``.  Brainfuck
functions are FULLY_TYPED, so tier-up happens before the first
interpreted call (threshold 0).

For programs whose IIR uses only ``const`` / arithmetic / ``load_mem`` /
``store_mem`` / control flow, JIT compilation succeeds and the binary
runs natively.  For programs that use ``call_builtin "putchar"`` /
``"getchar"`` (i.e. anything with ``.`` or ``,``), the lowering pipeline
returns ``None``; ``WASMBackend.compile`` reports failure; ``jit-core``
falls through to the interpreter.  This deopt is silent and observable
only via ``is_jit_compiled``.

Wiring host I/O so I/O programs also JIT (via WASI ``fd_write`` /
``fd_read``) is BF06.

Why not just use ``vm-core`` directly?
======================================
You can.  ``BrainfuckVM`` is a convenience: it gives you a one-call
``run(source) -> bytes`` that is the natural shape for Brainfuck.

Why ``call_builtin`` rather than ``io_in``/``io_out``?
======================================================
Both routes work.  ``io_in`` / ``io_out`` would require a port-handler
contract on ``vm-core`` that does not exist yet (the ``io_ports`` dict
is just a value store, not a handler dispatch).  Builtins, by contrast,
are the documented host-callable seam.  Using them now means the VM
plumbing stays unmodified and the JIT can later specialise the call
site directly without us having to invent an I/O port protocol.
"""

from __future__ import annotations

from typing import Any

from interpreter_ir import IIRInstr, IIRModule
from vm_core import VMCore, VMMetrics

from brainfuck_iir_compiler.compiler import compile_source
from brainfuck_iir_compiler.errors import BrainfuckError

# Default tape size — matches the canonical Brainfuck spec (Urban Müller, 1993).
_DEFAULT_TAPE_SIZE: int = 30_000


class BrainfuckVM:
    """Brainfuck-configured ``vm-core`` wrapper.

    Constructing the wrapper does *not* create a long-lived ``VMCore``
    instance — instead, each call to :meth:`run` builds a fresh VM.
    This keeps state hygiene trivial: there is no way for one program's
    pointer or memory to leak into the next.  If you need a sticky
    long-running session (REPL, notebook), reach for the underlying
    ``VMCore`` directly.

    Parameters
    ----------
    jit:
        If True, the wrapper would attach ``jit-core`` for tier-up.  Not
        yet wired in BF04 — raises :class:`NotImplementedError` pointing
        to BF05.  The flag exists in this spec so callers can opt in once
        BF05 lands without changing their call sites.
    tape_size:
        Maximum pointer value (exclusive).  Out-of-bounds writes raise
        :class:`BrainfuckError`; out-of-bounds reads return 0.
    max_steps:
        Optional fuel cap.  When set, the VM raises :class:`BrainfuckError`
        after that many IIR instructions execute.  ``None`` (the default)
        means run forever, which matches a hardware Brainfuck machine.
    """

    def __init__(
        self,
        *,
        jit: bool = False,
        tape_size: int = _DEFAULT_TAPE_SIZE,
        max_steps: int | None = None,
    ) -> None:
        if tape_size <= 0:
            raise ValueError("tape_size must be positive")
        if max_steps is not None and max_steps <= 0:
            raise ValueError("max_steps must be positive when provided")

        self._jit_enabled: bool = jit
        self._tape_size: int = tape_size
        self._max_steps: int | None = max_steps
        self._last_metrics: VMMetrics | None = None
        self._vm: VMCore | None = None
        # Updated each run when jit=True: True iff the JIT successfully
        # produced a native binary for ``main``.  Tests assert against
        # this to confirm the JIT path actually fired.
        self._last_jit_compiled: bool = False

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def compile(self, source: str) -> IIRModule:
        """Compile ``source`` to ``IIRModule`` without executing it."""
        return compile_source(source)

    def run(self, source: str, *, input_bytes: bytes = b"") -> bytes:
        """Compile and execute ``source``; return collected stdout bytes.

        ``input_bytes`` is the seed for ``,`` reads.  Reading past the end
        yields 0 — the most common Brainfuck convention.
        """
        module = self.compile(source)
        return self.execute_module(module, input_bytes=input_bytes)

    def execute_module(
        self,
        module: IIRModule,
        *,
        input_bytes: bytes = b"",
    ) -> bytes:
        """Execute an already-compiled ``module`` and return stdout bytes.

        Useful for callers that want to compile once and run many times
        with different inputs (REPLs, notebooks, fuzz harnesses).
        """
        output = bytearray()
        input_buffer = list(input_bytes)
        step_counter = [0]  # mutable cell so the closure can update it

        # ---- builtin: putchar(value) ---------------------------------
        # Brainfuck's `.` compiles to `call_builtin "putchar" v`.  The
        # VM hands us [v] as a single-element list; we mask & cast to a
        # byte and append it to the run-local output buffer.
        def putchar(args: list[Any]) -> None:
            (value,) = args
            output.append(int(value) & 0xFF)
            return None

        # ---- builtin: getchar() -> int -------------------------------
        # Brainfuck's `,` compiles to `v = call_builtin "getchar"`.  We
        # return 0 on EOF — this matches the convention used by
        # bf2c, bff, and most reference interpreters.
        def getchar(_args: list[Any]) -> int:
            if not input_buffer:
                return 0
            return input_buffer.pop(0)

        # ---- bounds guard for memory access --------------------------
        # We hook this in by overriding the standard ``load_mem`` and
        # ``store_mem`` opcode handlers.  Out-of-bounds reads return 0
        # (lazy-infinite-tape semantics) and out-of-bounds writes raise.
        tape_size = self._tape_size
        max_steps = self._max_steps

        def handle_load_mem(vm: VMCore, frame: Any, instr: IIRInstr) -> int:
            addr = int(frame.resolve(instr.srcs[0]))
            if addr < 0 or addr >= tape_size:
                value = 0
            else:
                value = int(vm.memory.get(addr, 0)) & 0xFF
            if instr.dest:
                frame.assign(instr.dest, value)
            return value

        def handle_store_mem(vm: VMCore, frame: Any, instr: IIRInstr) -> None:
            addr = int(frame.resolve(instr.srcs[0]))
            if addr < 0 or addr >= tape_size:
                raise BrainfuckError(
                    f"data pointer {addr} out of bounds [0, {tape_size})"
                )
            value = int(frame.resolve(instr.srcs[1])) & 0xFF
            vm.memory[addr] = value
            return None

        # Fuel-cap wrapper: we wrap the standard "advance one instruction"
        # via a label handler that bumps the counter.  ``label`` is a
        # convenient host because it executes as a no-op in the standard
        # table and runs at every loop top — it's not a *true* fuel
        # counter (which would tick per dispatch), but it's accurate
        # enough for "this loop is runaway".  For an exact per-instruction
        # cap, callers should reach for VMCore.interrupt() from a watchdog
        # thread.
        def handle_label(_vm: VMCore, _frame: Any, _instr: IIRInstr) -> None:
            if max_steps is not None:
                step_counter[0] += 1
                if step_counter[0] > max_steps:
                    raise BrainfuckError(
                        f"max_steps exceeded ({max_steps} label crossings)"
                    )
            return None

        vm = VMCore(
            u8_wrap=True,
            opcodes={
                "load_mem": handle_load_mem,
                "store_mem": handle_store_mem,
                "label": handle_label,
            },
        )
        vm.register_builtin("putchar", putchar)
        vm.register_builtin("getchar", getchar)
        self._vm = vm

        # ──────────────────────────────────────────────────────────────
        # JIT path (BF05+BF06).  When jit=True we attempt to compile
        # ``main`` to WebAssembly via WASMBackend before falling back
        # to the interpreter.  BF06 wired ``call_builtin "putchar"`` /
        # ``"getchar"`` through WASI ``fd_write`` / ``fd_read``, so
        # I/O-using programs JIT too — provided the host can supply
        # stdout / stdin callbacks.  We construct a per-run WasiHost
        # bound to the same ``output`` bytearray and ``input_buffer``
        # the interpreter path already manages.
        # ──────────────────────────────────────────────────────────────
        self._last_jit_compiled = False
        if self._jit_enabled and self._try_jit_run(
            vm, module, output, input_buffer
        ):
            # JIT ran to completion.  ``output`` may now contain bytes
            # the WASM binary emitted via fd_write — those were
            # appended by the WasiHost callback we wired in below.
            self._last_metrics = vm.metrics()
            return bytes(output)

        try:
            vm.execute(module, fn="main")
        finally:
            self._last_metrics = vm.metrics()

        return bytes(output)

    # ------------------------------------------------------------------
    # JIT helpers
    # ------------------------------------------------------------------

    def _try_jit_run(
        self,
        vm: VMCore,
        module: IIRModule,
        output: bytearray,
        input_buffer: list[int],
    ) -> bool:
        """Try compiling ``main`` to WASM and running the binary.

        Returns ``True`` if the JIT produced a binary that ran to
        completion, ``False`` if anything in the pipeline failed.  On
        ``False`` the caller should fall back to the interpreter.

        The ``jit-core`` / ``wasm-backend`` / ``wasm-runtime`` packages
        are imported lazily so that test environments without the WASM
        stack still load this module — a missing import simply forces
        a deopt, which is the correct behaviour anyway.

        ``output`` and ``input_buffer`` are the same buffers the
        interpreter path uses.  Wiring them through a ``WasiHost``
        means a JIT'd Hello World writes bytes to ``output`` exactly
        as the interpreter would, and ``,`` reads pop from
        ``input_buffer``.
        """
        try:
            from jit_core import JITCore
            from wasm_backend import WASMBackend
            from wasm_runtime import WasiHost
            from wasm_runtime.wasi_host import WasiConfig
        except ImportError:
            return False

        # ── BF06: route stdout/stdin through the same buffers used by
        # the interpreter path.  WASI fd_write decodes bytes as Latin-1
        # before calling stdout (1 byte → 1 char), so the round-trip
        # back to bytes via ``encode("latin-1")`` is exact.
        def stdout_cb(text: str) -> None:
            output.extend(text.encode("latin-1"))

        def stdin_cb(n: int) -> bytes:
            chunk = bytes(input_buffer[:n])
            del input_buffer[:n]
            return chunk

        try:
            # ``WasiHost`` only takes stdout/stderr as keyword args; stdin
            # must come through a ``WasiConfig``.  Build a minimal config
            # with both callbacks wired and let the host wrap it.
            config = WasiConfig(stdin=stdin_cb, stdout=stdout_cb)
            host = WasiHost(config)
            jit = JITCore(
                vm,
                WASMBackend(host=host),
                threshold_fully_typed=0,
                threshold_partial=10,
                threshold_untyped=100,
            )
            # Setting the module is required before ``compile()``;
            # ``execute_with_jit()`` would do it but also runs the
            # interpreter in Phase 2, which is not what we want for an
            # entry-point-JIT scenario.  Touching the private attribute
            # here is a deliberate V1 trade-off; LANG22 (multi-function
            # calling convention) is the natural place to add a public
            # "compile then run from cache" entry point.
            jit._module = module
            if not jit.compile("main"):
                return False
            entry = jit._cache.get("main")
            if entry is None:
                return False
            jit._backend.run(entry.binary, [])
        except Exception:
            # Any backend / runtime failure → deopt to interpreter.
            return False

        self._last_jit_compiled = True
        return True

    # ------------------------------------------------------------------
    # Read-only properties
    # ------------------------------------------------------------------

    @property
    def metrics(self) -> VMMetrics | None:
        """Last-run VM metrics, or None before any run."""
        return self._last_metrics

    @property
    def vm(self) -> VMCore | None:
        """The most recently constructed underlying ``VMCore``, or None."""
        return self._vm

    @property
    def tape_size(self) -> int:
        return self._tape_size

    @property
    def jit_enabled(self) -> bool:
        """Whether the wrapper was constructed with ``jit=True``."""
        return self._jit_enabled

    @property
    def is_jit_compiled(self) -> bool:
        """Whether the most recent run JIT-compiled ``main`` successfully.

        ``False`` when the wrapper was constructed with ``jit=False``,
        when no run has been performed yet, or when the JIT path was
        attempted but deopted (e.g. because the program contains I/O,
        which is not yet wired through WASI — see BF05 / BF06).
        """
        return self._last_jit_compiled
