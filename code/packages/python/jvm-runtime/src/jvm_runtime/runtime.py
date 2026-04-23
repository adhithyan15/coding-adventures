"""Top-level orchestration for the modular JVM prototype.

Architecture
------------

Execution passes through four layers::

    JVMRuntime.run_method()
        ├─ load_class()          parse the .class binary
        ├─ run <clinit>          initialise static arrays / fields
        └─ run target method
               │
               ▼  JVMSimulator.run()
               │   one instruction at a time; calls host for I/O and dispatch
               │
               ▼  JVMStdlibHost
                   ├─ get_static        System.out / System.in sentinels
                   ├─ invoke_virtual    PrintStream.write/flush/println,
                   │                   InputStream.read
                   └─ invoke_static     java.util.Arrays.fill,
                                       + recursive dispatch to any static
                                         method in the *same* class file

Shared static fields
--------------------

The JVM maps ``putstatic`` / ``getstatic`` to a single ``dict[reference,
value]`` instance (``shared`` in ``run_method``).  Every simulator instance
that participates in one top-level call — including nested inner simulators
spun up by ``_run_method_with_shared_state`` — receives a reference to the
*same* dict, so register arrays created in ``<clinit>`` are immediately
visible to the helper methods called from the main ``_start`` method.
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass

from jvm_bytecode_disassembler import JVMMethodBody, JVMVersion, disassemble_method_body
from jvm_class_file import (
    JVMClassFile,
    JVMFieldReference,
    JVMMethodReference,
    parse_class_file,
)
from jvm_simulator import JVMSimulator, JVMTrace


# ---------------------------------------------------------------------------
# Sentinel objects pushed onto the operand stack for well-known Java types.
# The simulator has no concept of Java objects; we use these tiny frozen
# dataclasses to let invoke_virtual dispatch correctly without a real heap.
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class JVMPrintStream:
    """Stands in for a java.io.PrintStream instance on the operand stack."""

    class_name: str = "java/io/PrintStream"


@dataclass(frozen=True)
class JVMInputStream:
    """Stands in for a java.io.InputStream instance on the operand stack.

    read() always returns -1 (EOF) because the BASIC pipeline never uses
    the INPUT statement in V1.
    """

    class_name: str = "java/io/InputStream"


# ---------------------------------------------------------------------------
# Host bridge
# ---------------------------------------------------------------------------


class JVMStdlibHost:
    """Implement the tiny JVM stdlib surface area that our programs need.

    The simulator calls back into this object whenever it encounters
    ``getstatic``, ``invokevirtual``, or ``invokestatic`` instructions.
    Each method either handles the call directly (for well-known stdlib
    references) or, for ``invokestatic``, delegates back to the JVMRuntime
    to execute another method from the *same* class file.

    The ``_runtime`` attribute is set by JVMRuntime immediately after
    construction to break the circular initialisation dependency.
    """

    def __init__(self, stdout: Callable[[str], None] | None = None) -> None:
        self._stdout = stdout
        self.output: list[str] = []
        # Set by JVMRuntime.__init__ after construction so invoke_static can
        # look up and run methods in the currently loaded class file.
        self._runtime: JVMRuntime | None = None

    # ------------------------------------------------------------------
    # getstatic — return sentinel objects for well-known static fields
    # ------------------------------------------------------------------

    def get_static(self, reference: object) -> object:
        """Return a sentinel for a well-known JVM static field.

        Handles only ``java.lang.System.out`` and ``java.lang.System.in``.
        All other field references raise RuntimeError so the caller can
        distinguish "this is a stdlib field" from "this is a user-defined
        static field" (the latter live in the simulator's ``static_fields``
        dict and are looked up *before* calling this method).
        """
        if isinstance(reference, JVMFieldReference) and reference.class_name == "java/lang/System":
            if reference.name == "out" and reference.descriptor == "Ljava/io/PrintStream;":
                return JVMPrintStream()
            if reference.name == "in" and reference.descriptor == "Ljava/io/InputStream;":
                return JVMInputStream()
        msg = f"Unsupported static field reference: {reference!r}"
        raise RuntimeError(msg)

    # ------------------------------------------------------------------
    # invoke_virtual — virtual calls on sentinel objects
    # ------------------------------------------------------------------

    def invoke_virtual(
        self,
        reference: object,
        receiver: object,
        args: list[object],
    ) -> object | None:
        """Dispatch a virtual method call on one of our sentinel objects.

        Supported calls:

        ``PrintStream.println(String)``
            The classic Hello World path.  Appends ``str + "\\n"`` to output.

        ``PrintStream.write(int)``
            Write a single raw byte (used by the BASIC numeric-print path and
            string-literal printing).  The byte value is converted to a
            Unicode character via ``chr(byte & 0xFF)``.

        ``PrintStream.flush()``
            No-op: the simulator captures output in memory rather than
            buffering it, so flushing is always a no-op.

        ``InputStream.read()``
            Returns ``-1`` (EOF).  BASIC V1 does not support INPUT, so stdin
            is treated as immediately empty.
        """
        if isinstance(reference, JVMMethodReference):
            if isinstance(receiver, JVMPrintStream) and reference.class_name == "java/io/PrintStream":
                # println(String) — used by the Hello World class-file smoke test
                if (reference.name == "println"
                        and reference.descriptor == "(Ljava/lang/String;)V"
                        and len(args) == 1
                        and isinstance(args[0], str)):
                    message = f"{args[0]}\n"
                    self.output.append(message)
                    if self._stdout is not None:
                        self._stdout(message)
                    return None

                # write(int) — emit one raw ASCII byte.
                # The BASIC IR compiler routes every PRINT character through
                # SYSCALL 1, which calls this via the generated _syscall helper.
                if (reference.name == "write"
                        and reference.descriptor == "(I)V"
                        and len(args) == 1):
                    byte_val = int(args[0]) & 0xFF
                    char = chr(byte_val)
                    self.output.append(char)
                    if self._stdout is not None:
                        self._stdout(char)
                    return None

                # flush() — flush stdout; no-op in our in-memory simulator.
                if reference.name == "flush" and reference.descriptor == "()V":
                    return None

            if isinstance(receiver, JVMInputStream) and reference.class_name == "java/io/InputStream":
                # read() — BASIC V1 has no INPUT; return EOF.
                if reference.name == "read" and reference.descriptor == "()I":
                    return -1

        msg = f"Unsupported virtual method reference: {reference!r}"
        raise RuntimeError(msg)

    # ------------------------------------------------------------------
    # invoke_static — stdlib helpers + program-method dispatch
    # ------------------------------------------------------------------

    def invoke_static(
        self,
        reference: object,
        static_fields: dict[object, object],
        args: list[object],
    ) -> object | None:
        """Dispatch an invokestatic call.

        Two cases are handled:

        1. ``java.util.Arrays.fill([BIIB)V``
           Fill a slice of a byte array with a constant byte value.
           The generated ``<clinit>`` uses this to zero-initialise the
           program's memory region.

        2. A static method whose class matches the currently-loaded class
           file (i.e. one of the generated helper methods like
           ``__ca_regGet``, ``__ca_regSet``, ``__ca_syscall``).
           The call is forwarded to ``JVMRuntime._run_method_with_shared_state``
           which runs that method in a fresh inner simulator that shares the
           *same* ``static_fields`` dict, maintaining coherent program state.
        """
        if isinstance(reference, JVMMethodReference):
            # ── java.util.Arrays.fill([BIIB)V ────────────────────────────────
            # Signature: fill(byte[] a, int fromIndex, int toIndex, byte val)
            # Used by <clinit> to initialise memory regions to a non-zero byte.
            if (reference.class_name == "java/util/Arrays"
                    and reference.name == "fill"
                    and reference.descriptor == "([BIIB)V"):
                arrayref, from_idx, to_idx, value = args
                byte_val = int(value) & 0xFF
                for i in range(int(from_idx), int(to_idx)):
                    arrayref[i] = byte_val  # type: ignore[index]
                return None

            # ── Generated class methods ──────────────────────────────────────
            # Anything whose class_name matches our loaded class is a helper
            # method (__ca_regGet, __ca_regSet, __ca_syscall, …) or an IR
            # callable region (_start, etc.).
            if (self._runtime is not None
                    and self._runtime._class_file is not None
                    and reference.class_name == self._runtime._class_file.this_class_name):
                return self._runtime._run_method_with_shared_state(
                    name=reference.name,
                    descriptor=reference.descriptor,
                    static_fields=static_fields,
                    args=args,
                )

        msg = f"Unsupported static method reference: {reference!r}"
        raise RuntimeError(msg)


# ---------------------------------------------------------------------------
# Result type
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class JVMRunResult:
    """Result returned by JVMRuntime.run_method.

    Attributes:
        class_file:   Parsed class file loaded for this run.
        method:       Disassembled method body for the entry-point method.
        traces:       Instruction traces from the top-level method execution.
                      Traces from helper method invocations (via invokestatic)
                      are **not** included — they execute inside inner
                      simulators.
        output:       Accumulated stdout from all PrintStream.write/println
                      calls during the entire execution (all nested calls
                      included, because the host's ``output`` list is shared).
        return_value: The top-level method's return value, or ``None`` for
                      void methods.
    """

    class_file: JVMClassFile
    method: JVMMethodBody
    traces: tuple[JVMTrace, ...]
    output: str
    return_value: object | None


# ---------------------------------------------------------------------------
# Runtime orchestrator
# ---------------------------------------------------------------------------


class JVMRuntime:
    """Compose class-file decode, disassembly, and simulator execution.

    Usage::

        runtime = JVMRuntime()
        result = runtime.run_method(class_bytes, method_name="_start", descriptor="()I")
        print(result.output)

    The runtime automatically runs ``<clinit>`` (if present) before the
    requested method so that static arrays are initialised before any
    ``invokestatic`` helper calls reference them.
    """

    def __init__(self, *, stdout: Callable[[str], None] | None = None) -> None:
        self.host = JVMStdlibHost(stdout=stdout)
        # Back-reference so the host can dispatch invokestatic to us
        self.host._runtime = self
        # Top-level simulator; reused across run_method calls
        self.simulator = JVMSimulator(host=self.host)
        # Set before any _run_method_with_shared_state call so invoke_static
        # can look up methods in the right class file
        self._class_file: JVMClassFile | None = None

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def load_class(self, class_bytes: bytes) -> JVMClassFile:
        """Parse a raw .class byte string into a JVMClassFile."""
        return parse_class_file(class_bytes)

    def disassemble_method(
        self,
        class_file: JVMClassFile,
        *,
        method_name: str,
        descriptor: str,
    ) -> JVMMethodBody:
        """Disassemble and return the named method, raising if not found."""
        method = class_file.find_method(method_name, descriptor)
        if method is None or method.code_attribute is None:
            msg = (
                f"Method {method_name}{descriptor} was not found "
                "or has no Code attribute"
            )
            raise RuntimeError(msg)
        return self._build_method_body(class_file, method)

    def run_method(
        self,
        class_file_or_bytes: JVMClassFile | bytes,
        *,
        method_name: str,
        descriptor: str,
    ) -> JVMRunResult:
        """Load a class file and execute one of its static methods.

        Steps:

        1. Parse (or accept) the class file.
        2. Create a fresh ``shared`` dict for static fields.
        3. If the class has ``<clinit>``, run it in a temporary inner
           simulator so that static arrays are allocated and populated
           before the target method starts.
        4. Run the requested method on the top-level ``self.simulator``
           (which shares the same ``static_fields`` dict) and collect
           instruction traces.

        All nested ``invokestatic`` calls during step 4 spin up additional
        inner simulators that also share the same dict, so register reads
        and writes are coherent across the entire call tree.
        """
        class_file = (
            class_file_or_bytes
            if isinstance(class_file_or_bytes, JVMClassFile)
            else self.load_class(class_file_or_bytes)
        )
        self._class_file = class_file

        # A single dict shared by every simulator that participates in this run.
        # Keyed by JVMFieldReference (frozen dataclass → hashable).
        shared: dict[object, object] = {}

        # ── <clinit> ─────────────────────────────────────────────────────────
        # The class initialiser allocates the register and memory arrays and
        # stores them into static fields.  We must run it before _start so
        # that __ca_regs and __ca_memory exist when helper methods access them.
        clinit = self._disassemble_method_optional(
            class_file, method_name="<clinit>", descriptor="()V"
        )
        if clinit is not None:
            clinit_sim = JVMSimulator(host=self.host, static_fields=shared)
            clinit_sim.load_method(clinit)
            clinit_sim.run(max_steps=1_000_000)

        # ── Target method ─────────────────────────────────────────────────────
        method = self.disassemble_method(
            class_file, method_name=method_name, descriptor=descriptor
        )
        self.host.output.clear()
        # Point the top-level simulator at the shared field store so that
        # putstatic/getstatic in the target method also see the same arrays.
        self.simulator.static_fields = shared
        self.simulator.load_method(method)
        traces = tuple(self.simulator.run())

        return JVMRunResult(
            class_file=class_file,
            method=method,
            traces=traces,
            output="".join(self.host.output),
            return_value=self.simulator.return_value,
        )

    def run_main(
        self,
        class_file_or_bytes: JVMClassFile | bytes,
        args: list[str] | None = None,
    ) -> JVMRunResult:
        """Convenience wrapper: run the standard ``main([Ljava/lang/String;)V`` method."""
        _ = args
        return self.run_method(
            class_file_or_bytes,
            method_name="main",
            descriptor="([Ljava/lang/String;)V",
        )

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    def _disassemble_method_optional(
        self,
        class_file: JVMClassFile,
        *,
        method_name: str,
        descriptor: str,
    ) -> JVMMethodBody | None:
        """Like disassemble_method but returns None if the method is absent."""
        method = class_file.find_method(method_name, descriptor)
        if method is None or method.code_attribute is None:
            return None
        return self._build_method_body(class_file, method)

    def _build_method_body(self, class_file: JVMClassFile, method: object) -> JVMMethodBody:
        """Build a constant-pool-resolved JVMMethodBody from a parsed method entry."""
        constant_pool_lookup: dict[int, object] = {}
        for index in range(1, len(class_file.constant_pool)):
            entry = class_file.constant_pool[index]
            if entry is None:
                continue
            try:
                constant_pool_lookup[index] = class_file.resolve_constant(index)
                continue
            except ValueError:
                pass
            if type(entry).__name__ == "JVMFieldrefInfo":
                constant_pool_lookup[index] = class_file.resolve_fieldref(index)
            elif type(entry).__name__ == "JVMMethodrefInfo":
                constant_pool_lookup[index] = class_file.resolve_methodref(index)
        return disassemble_method_body(
            method.code_attribute.code,  # type: ignore[attr-defined]
            version=JVMVersion(class_file.version.major, class_file.version.minor),
            max_stack=method.code_attribute.max_stack,  # type: ignore[attr-defined]
            max_locals=method.code_attribute.max_locals,  # type: ignore[attr-defined]
            constant_pool=constant_pool_lookup,
        )

    def _run_method_with_shared_state(
        self,
        *,
        name: str,
        descriptor: str,
        static_fields: dict[object, object],
        args: list[object],
    ) -> object | None:
        """Execute a class method sharing the caller's static_fields dict.

        Called by JVMStdlibHost.invoke_static to recursively execute helper
        methods (__ca_regGet, __ca_regSet, __ca_syscall, …) while keeping the
        register and memory arrays coherent with the outer execution.

        A fresh inner JVMSimulator is created for each call frame (the JVM
        uses a per-frame evaluation stack, not a global one).  Argument
        values are injected into the inner simulator's local variable slots
        0, 1, 2, … before execution begins.
        """
        assert self._class_file is not None, (
            "_run_method_with_shared_state called without a loaded class file"
        )
        method = self.disassemble_method(
            self._class_file,
            method_name=name,
            descriptor=descriptor,
        )
        inner = JVMSimulator(host=self.host, static_fields=static_fields)
        inner.load_method(method)
        for i, arg in enumerate(args):
            if i < len(inner.locals):
                inner.locals[i] = arg
        inner.run(max_steps=1_000_000)
        return inner.return_value
