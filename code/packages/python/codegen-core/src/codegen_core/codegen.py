"""CodeGenerator protocol — generic IR-to-assembly interface (LANG20).

Why a separate CodeGenerator protocol?
---------------------------------------

``Backend[IR]`` (LANG19) bundles validate + codegen + assemble + run into one
interface.  That works when a single object owns the full pipeline.  But when
you want to share the codegen layer across multiple downstream pipelines — AOT,
JIT, and simulator — you need a finer split:

.. code-block:: text

    Frontend (Nib, BF, Algol, BASIC, Tetrad)
        ↓ IrProgram  (or list[CIRInstr] via LANG21 bridge)
    [Optimizer] — optional IR → IR transformation
        ↓
    [CodeGenerator] — validate + generate assembly   ← THIS MODULE
        ↓ Assembly (str | bytes | WasmModule | CILProgramArtifact | …)
        ├─→ [Assembler → Packager] → executable binary  (AOT, future)
        ├─→ [JIT runner]           → execute immediately (JIT, future)
        └─→ [Simulator]            → run directly        (orthogonal, future)

``CodeGenerator[IR, Assembly]`` is the boundary at which IR becomes assembly.
Everything downstream of that boundary is outside the concern of the code
generator.

The simulator pipeline is *orthogonal*: it takes the assembly output — packaged
or not — and runs it directly on simulated hardware.  No binary encoding step
is needed for a software simulator.

Assembly types
--------------

Different backends naturally produce different "assembly" forms:

``str``
    Text assembly for Intel 4004 and Intel 8008 — lines of mnemonics.
    Passed to a text assembler to produce binary.

``bytes``
    Ready-to-execute binary for GE-225 and JVM — these backends combine
    codegen and assembly into one step; the output is already binary.

``WasmModule``
    Structured WASM 1.0 module object.  ``wasm-module-encoder.encode_module()``
    converts it to standard binary bytes.

``CILProgramArtifact``
    Structured multi-method CIL artifact.  The CLR simulator accepts it
    directly; a PE packager would encode it to disk.

The ``Assembly`` type variable captures this heterogeneity so static type
checkers track the assembly form end-to-end.
"""

from __future__ import annotations

from typing import Any, Protocol, TypeVar, runtime_checkable

# ---------------------------------------------------------------------------
# Type variables
# ---------------------------------------------------------------------------

# IR is the intermediate representation type (input to the code generator).
# It is invariant here — the same TypeVar usage as Backend[IR] in backend.py.
IR = TypeVar("IR")

# Assembly is the output type (generated assembly / machine code).
# Invariant — callers and backends agree on the exact return type.
Assembly = TypeVar("Assembly")


# ---------------------------------------------------------------------------
# CodeGenerator protocol
# ---------------------------------------------------------------------------


@runtime_checkable
class CodeGenerator(Protocol[IR, Assembly]):
    """Validates IR for a target architecture and generates target assembly.

    A *code generator* translates a typed IR value into target-specific
    assembly code.  It does **not** assemble (text → binary), package, link,
    or execute.

    Type parameters
    ---------------
    IR
        The IR type this generator accepts.  Typically ``IrProgram`` for the
        compiled-language path, or ``list[CIRInstr]`` for the JIT/AOT path
        (the latter requires the LANG21 bridge).

    Assembly
        The generated assembly type.  Varies by backend:

        - ``str``                  — Intel 4004, Intel 8008 (text mnemonics)
        - ``bytes``                — GE-225, JVM  (binary already assembled)
        - ``WasmModule``           — WASM 1.0 structured module
        - ``CILProgramArtifact``   — CIL multi-method artifact

    Attributes
    ----------
    name:
        Short human-readable identifier for this target, e.g. ``"ge225"``,
        ``"jvm"``, ``"wasm"``, ``"cil"``, ``"intel4004"``, ``"intel8008"``.

    Examples
    --------
    Typical validate-then-generate call pattern::

        gen = GE225CodeGenerator()
        errors = gen.validate(ir)
        if errors:
            for msg in errors:
                print(f"  ERROR: {msg}")
        else:
            result = gen.generate(ir)  # CompileResult

    Using a registry to select a backend at runtime::

        registry = CodeGeneratorRegistry()
        registry.register(GE225CodeGenerator())
        registry.register(JVMCodeGenerator())
        gen = registry.get_or_raise("jvm")
        assembly = gen.generate(ir)
    """

    name: str

    def validate(self, ir: IR) -> list[str]:
        """Validate IR for this target architecture.

        Checks all target-specific constraints — opcode support, value ranges,
        call-depth limits, etc. — without generating any code.  Returns all
        errors at once so the caller sees the full picture.

        Parameters
        ----------
        ir:
            The typed IR to inspect.

        Returns
        -------
        list[str]
            Human-readable error messages.  An *empty list* means the IR is
            compatible with this target and ``generate()`` will succeed.
        """
        ...

    def generate(self, ir: IR) -> Assembly:
        """Generate assembly from IR.

        Validates first (internally); raises a backend-specific exception if
        the IR is invalid.  If you need to inspect errors before generating,
        call ``validate()`` separately.

        Parameters
        ----------
        ir:
            The typed IR to translate.

        Returns
        -------
        Assembly
            Target-specific assembly output.  The exact type depends on the
            backend (see class docstring).

        Raises
        ------
        Exception
            Backend-specific exception on validation failure or unsupported IR.
            For example, ``CodeGenError`` for GE-225, ``JvmBackendError`` for
            JVM, ``WasmLoweringError`` for WASM, ``CILBackendError`` for CIL.
        """
        ...


# ---------------------------------------------------------------------------
# CodeGeneratorRegistry
# ---------------------------------------------------------------------------


class CodeGeneratorRegistry:
    """Name → CodeGenerator lookup, independent of IR/Assembly types.

    A ``CodeGeneratorRegistry`` decouples generator *selection* from generator
    *construction*.  Register all available generators at startup; look them
    up by name at generation time.

    Generators are stored as ``Any`` because the two type parameters (``IR``
    and ``Assembly``) differ between backends — there is no single type that
    covers all of them.  Callers that need type safety should narrow the result
    to the appropriate ``CodeGenerator[IR, Assembly]`` after retrieval.

    Examples
    --------
    >>> class _MockGen:
    ...     name = "mock"
    ...     def validate(self, ir): return []
    ...     def generate(self, ir): return "NOP\\n"
    >>> registry = CodeGeneratorRegistry()
    >>> registry.register(_MockGen())
    >>> registry.get("mock").name
    'mock'
    >>> registry.names()
    ['mock']
    >>> len(registry)
    1
    """

    def __init__(self) -> None:
        self._generators: dict[str, Any] = {}

    def register(self, generator: Any) -> None:
        """Add ``generator`` to the registry under its ``name``.

        If a generator with the same name already exists it is replaced.

        Parameters
        ----------
        generator:
            Any object satisfying ``CodeGenerator[IR, Assembly]`` for some
            ``IR`` and ``Assembly``.  Must have a ``name`` attribute.
        """
        self._generators[generator.name] = generator

    def get(self, name: str) -> Any | None:
        """Return the generator registered under ``name``, or ``None``.

        Parameters
        ----------
        name:
            The ``CodeGenerator.name`` value used when registering.

        Returns
        -------
        Any | None
            The registered generator, or ``None`` if not found.
        """
        return self._generators.get(name)

    def get_or_raise(self, name: str) -> Any:
        """Return the generator registered under ``name``, or raise ``KeyError``.

        Parameters
        ----------
        name:
            The ``CodeGenerator.name`` value used when registering.

        Returns
        -------
        Any
            The registered generator.

        Raises
        ------
        KeyError
            If no generator with ``name`` has been registered.
        """
        try:
            return self._generators[name]
        except KeyError:
            available = ", ".join(sorted(self._generators)) or "<none>"
            raise KeyError(
                f"No code generator named {name!r} in registry. "
                f"Available: {available}"
            ) from None

    def names(self) -> list[str]:
        """Return a sorted list of all registered generator names."""
        return sorted(self._generators)

    def all(self) -> list[Any]:
        """Return all registered generators in name-sorted order."""
        return [self._generators[k] for k in sorted(self._generators)]

    def __len__(self) -> int:
        return len(self._generators)

    def __contains__(self, name: object) -> bool:
        return name in self._generators

    def __repr__(self) -> str:
        names = ", ".join(sorted(self._generators)) or "<empty>"
        return f"CodeGeneratorRegistry({names})"
