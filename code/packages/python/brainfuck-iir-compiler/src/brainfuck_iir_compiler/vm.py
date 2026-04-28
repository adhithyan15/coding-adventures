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

Why not just use ``vm-core`` directly?
======================================
You can.  ``BrainfuckVM`` is a convenience: it gives you a one-call
``run(source) -> bytes`` that is the natural shape for Brainfuck.  Once
the JIT and AOT integrations land (BF05+), this same wrapper grows the
``jit=True`` and ``aot=True`` knobs without changing its surface — the
host code that already calls ``run()`` does not need to change.

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
        if jit:
            raise NotImplementedError(
                "BrainfuckVM(jit=True) is not yet wired — see "
                "code/specs/BF04-brainfuck-iir-compiler.md §'Out of scope' "
                "and the BF05 follow-up spec.  Call with jit=False for now."
            )
        if tape_size <= 0:
            raise ValueError("tape_size must be positive")
        if max_steps is not None and max_steps <= 0:
            raise ValueError("max_steps must be positive when provided")

        self._tape_size: int = tape_size
        self._max_steps: int | None = max_steps
        self._last_metrics: VMMetrics | None = None
        self._vm: VMCore | None = None

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

        try:
            vm.execute(module, fn="main")
        finally:
            self._last_metrics = vm.metrics()

        return bytes(output)

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
