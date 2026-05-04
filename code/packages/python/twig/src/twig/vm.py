"""``TwigVM`` — wrap ``vm-core`` with Twig's heap + builtins.

What this wrapper does
======================
``vm-core`` is general-purpose: it knows nothing about cons cells,
closures, or symbols.  Twig's host registers all of that machinery
through the ``BuiltinRegistry`` interface — every builtin name the
compiler emits in ``call_builtin`` resolves here to a Python
callable that operates on the host's :class:`Heap`.

Why a single wrapper class
==========================
Other languages in the repo (BrainfuckVM, …) follow the same shape:
one wrapper that owns the per-run state, sets up vm-core, runs the
program, and returns the program's output.  Twig follows suit.  The
``run()`` method is the high-level entry point; ``execute_module``
is the loop that handles re-entry on ``apply_closure``.
"""

from __future__ import annotations

import io
import sys
from typing import Any

from interpreter_ir import IIRModule
from vm_core import VMCore, VMMetrics

from twig.ast_extract import extract_program
from twig.compiler import compile_program
from twig.errors import TwigRuntimeError
from twig.heap import NIL, Heap, HeapHandle
from twig.parser import parse_twig


class TwigVM:
    """Twig-configured ``vm-core`` runtime.

    Construct once, call :meth:`run` for each program.  Each ``run``
    builds a fresh execution environment (heap, globals, output
    buffer) so programs don't leak state into one another.

    Parameters
    ----------
    max_frames:
        Forwarded to ``VMCore`` — the call-stack-depth ceiling.
        Defaults to 256, which comfortably handles factorial of 100
        but bounds runaway recursion.
    """

    def __init__(self, *, max_frames: int = 256) -> None:
        self._max_frames = max_frames
        self._last_metrics: VMMetrics | None = None
        self._last_heap: Heap | None = None
        self._last_globals: dict[str, Any] | None = None

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def compile(self, source: str) -> IIRModule:
        """Lex, parse, and compile ``source`` into an :class:`IIRModule`."""
        return compile_program(extract_program(parse_twig(source)))

    def run(self, source: str) -> tuple[str, Any]:
        """Compile and run ``source``.

        Returns a tuple ``(stdout, value)`` where ``stdout`` is the
        concatenation of every ``print`` call's output (newline-
        separated, like Scheme's ``display``) and ``value`` is the
        program's final expression value (or ``nil`` if the program
        had no top-level expression).
        """
        return self.execute_module(self.compile(source))

    def execute_module(self, module: IIRModule) -> tuple[str, Any]:
        """Execute a compiled module — useful for reusing one IIR across
        multiple runs (e.g. driver loops, benchmarks).

        Host calls are lowered at compile time to
        ``call_builtin "syscall" <num> arg…``; the ``"syscall"``
        builtin registered below dispatches by number to the correct
        host operation.  No module patching is needed here.
        """
        heap = Heap()
        globals_table: dict[str, Any] = {}
        output = io.StringIO()

        # The VM is *re-entrant* during this run because
        # ``apply_closure`` calls vm.execute again.  We need to share
        # the heap, globals, and output across these re-entries — we
        # do so by closing over them in the builtin callables.
        #
        # We also override ``jmp_if_true`` / ``jmp_if_false`` to use
        # *Scheme* truthiness instead of vm-core's default Python
        # truthiness.  Scheme says only ``#f`` and ``nil`` are
        # falsy; everything else (including ``0``) is truthy.  This
        # matters surprisingly often — a programmer writing ``(if x
        # ...)`` to mean "is x present?" expects ``0`` to count as
        # present.
        vm = VMCore(
            max_frames=self._max_frames,
            profiler_enabled=False,
            opcodes={
                "jmp_if_true": _make_jmp_if(truthy=True),
                "jmp_if_false": _make_jmp_if(truthy=False),
            },
        )
        self._register_builtins(vm, heap, globals_table, output, module)

        result = vm.execute(module, fn="main")

        self._last_metrics = vm.metrics()
        self._last_heap = heap
        self._last_globals = globals_table
        return output.getvalue(), result

    # ------------------------------------------------------------------
    # Read-only properties
    # ------------------------------------------------------------------

    @property
    def metrics(self) -> VMMetrics | None:
        return self._last_metrics

    @property
    def heap(self) -> Heap | None:
        """The :class:`Heap` from the most recent run.

        Useful for tests asserting GC correctness — e.g. that
        ``heap.stats().live_objects`` returns to zero after a clean
        program.
        """
        return self._last_heap

    @property
    def globals_(self) -> dict[str, Any] | None:
        """Snapshot of the global table from the most recent run."""
        return self._last_globals

    # ------------------------------------------------------------------
    # Builtin wiring
    # ------------------------------------------------------------------

    def _register_builtins(
        self,
        vm: VMCore,
        heap: Heap,
        globals_table: dict[str, Any],
        output: io.StringIO,
        module: IIRModule,
    ) -> None:
        """Wire every builtin name the compiler emits to a host callable."""

        # ── Arithmetic + comparison ───────────────────────────────────
        # Variadic for ``+`` / ``*`` / ``=`` to match Scheme; binary
        # for the rest, which matches our compile-site shape.

        def add(args: list[Any]) -> int:
            return sum(_int(a) for a in args)

        def sub(args: list[Any]) -> int:
            if not args:
                raise TwigRuntimeError("(- ) needs at least one argument")
            if len(args) == 1:
                return -_int(args[0])
            head, *rest = args
            return _int(head) - sum(_int(a) for a in rest)

        def mul(args: list[Any]) -> int:
            result = 1
            for a in args:
                result *= _int(a)
            return result

        def div(args: list[Any]) -> int:
            if len(args) != 2:
                raise TwigRuntimeError("(/ ) requires exactly 2 arguments")
            num, den = _int(args[0]), _int(args[1])
            if den == 0:
                raise TwigRuntimeError("division by zero")
            # Python ``//`` floors; Scheme integer division truncates.
            # Use truncation for predictable behaviour with negatives.
            q = abs(num) // abs(den)
            return q if (num >= 0) == (den >= 0) else -q

        def eq(args: list[Any]) -> bool:
            if len(args) < 2:
                return True
            first = args[0]
            return all(_value_eq(first, a) for a in args[1:])

        def lt(args: list[Any]) -> bool:
            return _int(args[0]) < _int(args[1])

        def gt(args: list[Any]) -> bool:
            return _int(args[0]) > _int(args[1])

        # ── Cons / car / cdr ──────────────────────────────────────────
        def cons(args: list[Any]) -> HeapHandle:
            if len(args) != 2:
                raise TwigRuntimeError("(cons a b) takes exactly 2 arguments")
            return heap.alloc_cons(args[0], args[1])

        def car(args: list[Any]) -> Any:
            return heap.car(_handle(args[0]))

        def cdr(args: list[Any]) -> Any:
            return heap.cdr(_handle(args[0]))

        # ── Predicates ────────────────────────────────────────────────
        def is_null(args: list[Any]) -> bool:
            return args[0] is NIL

        def is_pair(args: list[Any]) -> bool:
            return heap.is_cons(args[0])

        def is_number(args: list[Any]) -> bool:
            v = args[0]
            return isinstance(v, int) and not isinstance(v, bool)

        def is_symbol(args: list[Any]) -> bool:
            return heap.is_symbol(args[0])

        # ── I/O ───────────────────────────────────────────────────────
        def do_print(args: list[Any]) -> Any:
            output.write(_format(args[0], heap))
            output.write("\n")
            return NIL

        # ── Host syscall dispatcher (TW04 Phase 4c) ──────────────────
        # The compiler lowers every ``(host/write-byte x)`` etc. to
        # ``call_builtin "syscall" <num> arg…``.  Numbers follow the
        # platform-independent convention established by the Brainfuck
        # backends (see ``_HOST_SYSCALLS`` in ``twig/compiler.py``):
        #
        #   1 — write-byte  (write args[1] & 0xFF to stdout)
        #   2 — read-byte   (read one byte; return -1 on EOF)
        #  10 — exit        (sys.exit with args[1] as the code)
        #
        # Using a single ``"syscall"`` builtin (instead of one per
        # operation) keeps the builtin namespace clean and mirrors
        # how the JVM and CLR backends lower the same numbers.
        def syscall(args: list[Any]) -> Any:
            """Interpreter-side syscall dispatcher."""
            num = _int(args[0])
            if num == 1:   # write-byte
                b = _int(args[1]) & 0xFF
                sys.stdout.buffer.write(bytes([b]))
                sys.stdout.buffer.flush()
                return NIL
            if num == 2:   # read-byte
                raw = sys.stdin.buffer.read(1)
                return -1 if not raw else raw[0]
            if num == 10:  # exit
                sys.exit(_int(args[1]))
            raise TwigRuntimeError(f"unknown syscall number: {num}")

        # ── Heap construction (compiler-emitted plumbing) ─────────────
        def make_nil(_args: list[Any]) -> Any:
            return NIL

        def make_symbol(args: list[Any]) -> HeapHandle:
            return heap.make_symbol(str(args[0]))

        def make_closure(args: list[Any]) -> HeapHandle:
            # args = [fn_name, capt0, capt1, ...]
            fn_name = str(args[0])
            captured = list(args[1:])
            return heap.alloc_closure(fn_name, captured)

        def make_builtin_closure(args: list[Any]) -> HeapHandle:
            # First-class builtin: store the builtin name as the
            # ``fn_name`` plus a sentinel marker so apply_closure
            # knows to route through call_builtin instead of vm.execute.
            name = str(args[0])
            return heap.alloc_closure(f"__builtin__:{name}", [])

        def apply_closure(args: list[Any]) -> Any:
            # args = [closure_handle, user_arg0, user_arg1, ...]
            handle = _handle(args[0])
            user_args = list(args[1:])
            fn_name = heap.closure_fn(handle)
            captured = heap.closure_captured(handle)

            if fn_name.startswith("__builtin__:"):
                # First-class builtin: forward to the same builtin
                # registry.
                name = fn_name[len("__builtin__:"):]
                return vm.builtins.call(name, user_args)

            full_args = list(captured) + user_args
            # Re-enter the VM by calling ``execute`` — but because
            # ``execute`` resets ``_frames`` / ``_module`` /
            # ``_interrupted`` on entry (it expects to be the
            # outermost call), we must save those fields, let the
            # call run, then restore them so the dispatch loop the
            # outer caller is mid-way through resumes correctly.
            #
            # This is a deliberate V1 trade-off — the cleaner fix
            # is to add a public "call into a function from a
            # builtin" entry point on ``vm-core``.  TW01 / future
            # GC work is the natural place to design that API.
            saved_frames = vm._frames
            saved_module = vm._module
            saved_interrupted = vm._interrupted
            try:
                return vm.execute(module, fn=fn_name, args=full_args)
            finally:
                vm._frames = saved_frames
                vm._module = saved_module
                vm._interrupted = saved_interrupted

        def global_get(args: list[Any]) -> Any:
            name = str(args[0])
            if name not in globals_table:
                raise TwigRuntimeError(f"unbound global {name!r}")
            return globals_table[name]

        def global_set(args: list[Any]) -> Any:
            name = str(args[0])
            globals_table[name] = args[1]
            return NIL

        def move(args: list[Any]) -> Any:
            """Type-faithful identity.  The compiler uses this to merge
            two control-flow branches into one register without going
            through ``add x 0``, which would coerce ``True`` to ``1``
            because Python's ``bool`` is a subclass of ``int``.
            """
            return args[0]

        # Register everything.
        for name, fn in {
            "+": add,
            "-": sub,
            "*": mul,
            "/": div,
            "=": eq,
            "<": lt,
            ">": gt,
            "cons": cons,
            "car": car,
            "cdr": cdr,
            "null?": is_null,
            "pair?": is_pair,
            "number?": is_number,
            "symbol?": is_symbol,
            "print": do_print,
            # Host syscall dispatcher (TW04 Phase 4c).
            "syscall": syscall,
            "make_nil": make_nil,
            "make_symbol": make_symbol,
            "make_closure": make_closure,
            "make_builtin_closure": make_builtin_closure,
            "apply_closure": apply_closure,
            "global_get": global_get,
            "global_set": global_set,
            "_move": move,
        }.items():
            vm.register_builtin(name, fn)


# ---------------------------------------------------------------------------
# Helpers — runtime-value coercions
# ---------------------------------------------------------------------------


def _int(value: Any) -> int:
    """Coerce a Twig value to a Python int, or raise a useful error."""
    if isinstance(value, bool):
        # bool is a subclass of int; reject to match Scheme's
        # numeric-tower predicates ("#t is not a number").
        raise TwigRuntimeError(f"expected number, got boolean: {value!r}")
    if isinstance(value, int):
        return value
    raise TwigRuntimeError(f"expected number, got {type(value).__name__}")


def _handle(value: Any) -> HeapHandle:
    """Coerce a Twig value to a HeapHandle, or raise a useful error."""
    if isinstance(value, HeapHandle):
        return value
    raise TwigRuntimeError(f"expected heap handle, got {type(value).__name__}")


def _value_eq(a: Any, b: Any) -> bool:
    """Twig's ``=`` — value equality on numbers and atomic comparison
    elsewhere.  Two heap handles compare equal iff they are the same
    handle (matching Scheme's ``eq?`` for symbols / cons / closures).
    """
    if isinstance(a, bool) or isinstance(b, bool):
        return a is b
    if isinstance(a, int) and isinstance(b, int):
        return a == b
    return a is b


def _scheme_truthy(value: Any) -> bool:
    """Twig follows Scheme truthiness: only ``#f`` and ``nil`` are
    false.  Everything else — including ``0`` and the empty cons
    cell — is true.  Without this override, vm-core would treat
    ``0`` as falsy (Python semantics), which surprises programmers
    using ``if`` to test for "is this number present?".
    """
    if value is False:
        return False
    return value is not NIL


def _make_jmp_if(*, truthy: bool):
    """Build a vm-core opcode handler implementing Scheme truthiness.

    ``truthy=True`` produces a handler equivalent to ``jmp_if_true``;
    ``truthy=False`` produces ``jmp_if_false``.  Both consult
    :func:`_scheme_truthy` for the test, then update ``frame.ip``
    via the same ``frame.fn.label_index`` call vm-core's standard
    handlers use.
    """
    def handle(_vm: Any, frame: Any, instr: Any) -> None:
        cond = frame.resolve(instr.srcs[0])
        if _scheme_truthy(cond) is truthy:
            target = str(instr.srcs[1])
            frame.ip = frame.fn.label_index(target)
        return None

    return handle


def _format(value: Any, heap: Heap) -> str:
    """Pretty-print a Twig value for the ``print`` builtin."""
    if value is NIL:
        return "nil"
    if isinstance(value, bool):
        return "#t" if value else "#f"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, HeapHandle):
        if heap.is_symbol(value):
            return heap.symbol_name(value)
        if heap.is_cons(value):
            # Render as ``(a b c)`` for proper lists, ``(a . b)``
            # for improper.  Standard Scheme display.
            parts: list[str] = []
            current: Any = value
            while heap.is_cons(current):
                parts.append(_format(heap.car(current), heap))
                current = heap.cdr(current)
            if current is NIL:
                return "(" + " ".join(parts) + ")"
            return "(" + " ".join(parts) + " . " + _format(current, heap) + ")"
        if heap.is_closure(value):
            return f"#<closure:{heap.closure_fn(value)}>"
    return repr(value)
