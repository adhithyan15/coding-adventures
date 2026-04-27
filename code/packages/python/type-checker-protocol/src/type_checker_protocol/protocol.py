"""
Generic type-checker protocol — the shared contract all language type checkers
must implement.

──────────────────────────────────────────────────────────────────────────────
WHAT IS A TYPE CHECKER?
──────────────────────────────────────────────────────────────────────────────

A compiler typically passes source code through several stages:

    Source text
    → Lexer        (characters → tokens)
    → Parser       (tokens → AST)
    → Type Checker (untyped AST → typed AST)   ← this layer
    → IR Compiler  (typed AST → IR)
    → Backend Validator (IR → validated IR)     ← ISA/hardware checks here
    → Code Generator (validated IR → machine code / bytecode)

The type checker's job is to *verify* that a program is type-safe and to
*annotate* the AST with type information that later stages need.

For example, in a statically typed language the expression `1 + "hello"` is
illegal.  The type checker detects this, produces a ``TypeErrorDiagnostic``
(with the line/column where the mistake is), and reports it back to the user.

If the program *is* type-safe, the type checker produces a *typed AST* —
the same tree, but with every node annotated with its resolved type.  Later
stages (IR generation, optimization) can then trust those annotations without
doing type inference again.

──────────────────────────────────────────────────────────────────────────────
SCOPE: LANGUAGE SEMANTICS ONLY — NOT HARDWARE CONSTRAINTS
──────────────────────────────────────────────────────────────────────────────

``TypeChecker`` enforces *language-level* invariants only.  Examples of what
belongs here:

  ✓  Type mismatches (``Int + String``)
  ✓  Use of undeclared variables
  ✓  A language rule that forbids recursion
  ✓  Static bounds checks on for-loop ranges (when the language specifies them)

Examples of what does *not* belong here:

  ✗  "Call depth must be ≤ 2"          ← ISA/hardware limit, not a language rule
  ✗  "Total RAM usage must be ≤ 160 B" ← target-specific resource constraint
  ✗  "Register count must be ≤ 16"     ← CPU architecture detail

Hardware and ISA constraints belong in each backend's own validator (e.g.,
``Intel4004Validator``, ``ArmValidator``), which runs *after* IR generation and
*before* code emission.  Keeping them out of the type checker makes the design
composable: the same Nib frontend can target Intel 4004, ARM, or a future ISA
without ever touching this layer.

──────────────────────────────────────────────────────────────────────────────
WHY A PROTOCOL?
──────────────────────────────────────────────────────────────────────────────

This repo will have *many* type checkers:

    NibTypeChecker         — for the Nib language
    LatticeTypeChecker     — for the Lattice stylesheet language
    MosaicTypeChecker      — for the Mosaic component language
    … and so on

All of them must implement the *same* interface so that:

  1. The compiler pipeline can compose them uniformly.
  2. We can test each checker in isolation against the same contract.
  3. Tools like static analysis dashboards can treat all checkers the same way.

Python's ``typing.Protocol`` gives us *structural subtyping* — a class
satisfies the protocol if it has the right methods with the right signatures,
without needing to inherit from anything.  This is sometimes called "duck
typing with types".

──────────────────────────────────────────────────────────────────────────────
GENERICS: WHY TypeChecker[ASTIn, ASTOut]?
──────────────────────────────────────────────────────────────────────────────

Different languages have different AST node types.  A Nib type checker works
on ``NibASTNode`` objects; a Lattice type checker works on ``LatticeNode``
objects.  We want mypy to catch mismatches:

    # This should be a type error — wrong checker for the wrong AST:
    nib_checker: TypeChecker[NibNode, TypedNibNode] = LatticeTypeChecker()

The two type parameters let mypy enforce this at compile time:

    ``ASTIn``  — the untyped AST that goes *in*  (contravariant: a checker
                 that accepts a supertype of NibNode is also valid here)
    ``ASTOut`` — the typed AST that comes *out* (covariant: returning a
                 subtype of TypedNibNode is also valid)

In practice, most users just write:

    checker: TypeChecker[MyNode, TypedMyNode] = MyTypeChecker()

and never think about variance — the generics are there to help mypy, not
to make your life harder.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Generic, Protocol, TypeVar

# ---------------------------------------------------------------------------
# Type variables
# ---------------------------------------------------------------------------

ASTIn = TypeVar("ASTIn", contravariant=True)  # type: ignore[misc]
ASTOut = TypeVar("ASTOut", covariant=True)  # type: ignore[misc]


# ---------------------------------------------------------------------------
# TypeErrorDiagnostic
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class TypeErrorDiagnostic:
    """A single type error with its source location.

    Frozen (immutable) so that callers cannot accidentally mutate a diagnostic
    after it has been added to a result.  This also makes diagnostics safe to
    use as dict keys or in sets.

    Attributes
    ----------
    message:
        Human-readable description of what went wrong.  Should be clear enough
        for a student learning the language to understand.  Example:
        ``"Cannot add Int and String — both sides of '+' must have the same type"``
    line:
        1-based line number where the error was detected.  Line 1 is the first
        line of the source file.
    column:
        1-based column number within that line.  Column 1 is the first
        character.

    Examples
    --------
    >>> err = TypeErrorDiagnostic(message="Type mismatch", line=3, column=7)
    >>> err.message
    'Type mismatch'
    >>> err.line
    3
    >>> err.column
    7
    >>> # Frozen — mutations raise FrozenInstanceError:
    >>> err.line = 99  # doctest: +ELLIPSIS
    Traceback (most recent call last):
        ...
    dataclasses.FrozenInstanceError: ...
    """

    message: str
    line: int
    column: int


# ---------------------------------------------------------------------------
# TypeCheckResult
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class TypeCheckResult(Generic[ASTOut]):
    """The result of running a type-checking pass.

    A type checker always returns a ``TypeCheckResult``, never raises an
    exception (except for truly unexpected internal errors).  This allows
    the pipeline to collect *all* type errors in one pass rather than stopping
    at the first.

    The result carries *both* the (possibly partially annotated) typed AST
    *and* the list of errors.  This is intentional: even when there are errors,
    the AST may be partially annotated, which lets IDE tooling provide
    completions and hover information while the user is still fixing mistakes.

    Attributes
    ----------
    typed_ast:
        The output AST.  If ``ok`` is True this is fully annotated and safe to
        pass to the IR compiler.  If ``ok`` is False this may be partially
        annotated — use it for IDE features but do not compile it further.
    errors:
        The list of type errors found.  An empty list means success.

    Properties
    ----------
    ok:
        ``True`` when there are no errors; ``False`` otherwise.  Shorthand
        for ``len(result.errors) == 0``.

    Examples
    --------
    >>> from dataclasses import dataclass
    >>> @dataclass
    ... class FakeAST:
    ...     label: str
    ...
    >>> # Success path
    >>> result = TypeCheckResult(typed_ast=FakeAST("typed"), errors=[])
    >>> result.ok
    True
    >>> result.typed_ast.label
    'typed'
    >>>
    >>> # Error path
    >>> err = TypeErrorDiagnostic(message="Bad type", line=1, column=1)
    >>> result = TypeCheckResult(typed_ast=FakeAST("partial"), errors=[err])
    >>> result.ok
    False
    >>> len(result.errors)
    1
    """

    typed_ast: ASTOut
    errors: list[TypeErrorDiagnostic] = field(default_factory=list)

    @property
    def ok(self) -> bool:
        """Return True if there are no type errors.

        This is the primary way to check whether type-checking succeeded:

            result = checker.check(ast)
            if result.ok:
                ir = ir_compiler.compile(result.typed_ast)
            else:
                for error in result.errors:
                    print(f"{error.line}:{error.column}: {error.message}")
        """
        return len(self.errors) == 0


# ---------------------------------------------------------------------------
# TypeChecker Protocol
# ---------------------------------------------------------------------------


class TypeChecker(Protocol[ASTIn, ASTOut]):
    """Generic protocol for all type-checking passes in this repo.

    Any class that implements a ``check`` method with the right signature
    automatically satisfies this protocol — no inheritance needed.

    Type Parameters
    ---------------
    ASTIn:
        The *untyped* AST that this checker accepts as input.  Contravariant:
        a checker that works on a *supertype* of ASTIn also satisfies this
        protocol.
    ASTOut:
        The *typed* AST that this checker produces as output.  Covariant: a
        checker that produces a *subtype* of ASTOut also satisfies this
        protocol.

    Usage
    -----
    Implementing a concrete type checker::

        from dataclasses import dataclass
        from type_checker_protocol import TypeChecker, TypeCheckResult, TypeErrorDiagnostic

        @dataclass
        class NibNode:
            kind: str

        @dataclass
        class TypedNibNode:
            kind: str
            resolved_type: str

        class NibTypeChecker:
            def check(self, ast: NibNode) -> TypeCheckResult[TypedNibNode]:
                if ast.kind == "bad":
                    err = TypeErrorDiagnostic("Unknown node kind", line=1, column=1)
                    typed = TypedNibNode(kind=ast.kind, resolved_type="unknown")
                    return TypeCheckResult(typed_ast=typed, errors=[err])
                typed = TypedNibNode(kind=ast.kind, resolved_type="int")
                return TypeCheckResult(typed_ast=typed, errors=[])

    Using the protocol as a type annotation::

        def run_pipeline(
            checker: TypeChecker[NibNode, TypedNibNode],
            ast: NibNode,
        ) -> TypedNibNode:
            result = checker.check(ast)
            if not result.ok:
                raise RuntimeError(f"{len(result.errors)} type error(s)")
            return result.typed_ast

    Notes
    -----
    - The ``check`` method must never raise an exception for type errors; it
      should always return a ``TypeCheckResult``.
    - Multiple errors should be collected in a single pass rather than stopping
      at the first.
    - The ``typed_ast`` field in the result may be partially annotated when
      errors are present.

    Scope — language semantics only
    ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    This protocol is for *language-level* type checking.  It must NOT enforce
    hardware or ISA constraints such as call-depth limits, RAM budgets, or
    register counts.  Those belong in the backend's own validator (e.g.,
    ``IrValidator`` in the intel-4004-ir-validator package), which runs after IR generation.

    This separation makes the design composable: the same frontend type checker
    can target Intel 4004, ARM, WASM, or any future ISA without modification.
    """

    def check(self, ast: ASTIn) -> TypeCheckResult[ASTOut]:
        """Type-check the given AST and return the result.

        Parameters
        ----------
        ast:
            The untyped AST produced by the parser.  This checker will walk
            the tree, verify type safety, and annotate each node.

        Returns
        -------
        TypeCheckResult[ASTOut]:
            A result object containing:
            - ``typed_ast``: the annotated AST (may be partial if errors exist)
            - ``errors``: list of ``TypeErrorDiagnostic``; empty means success

        Notes
        -----
        - Do not raise exceptions for type errors — put them in ``errors``.
        - Do raise exceptions for truly unexpected internal errors (e.g.,
          receiving ``None`` when an AST is required).
        """
        ...
