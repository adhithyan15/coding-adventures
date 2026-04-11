"""Semantic IR (SIR) — language-agnostic intermediate representation for programs.

The Semantic IR sits between language-specific parsers (which produce concrete
syntax trees) and language-specific generators (which emit source code). It is
the pivot point for cross-language compilation, type inference, and code
generation.

Architecture
------------

The pipeline looks like this::

    JS source
      → [javascript-lexer]        tokens
      → [javascript-parser]       ASTNode tree (concrete syntax)
      → [js-ast-to-sir]           SIRModule  ← YOU ARE HERE
      → [sir-to-js-ast]           ASTNode tree (canonical)
      → [js-ast-to-string]        formatted JS string

Because the SIR is language-agnostic, the same SIRModule can be fed to
a TypeScript generator, a Python generator, or any other backend.

Design principles
-----------------

1. **Semantic, not syntactic** — every node represents a *meaning*, not a
   *notation*. The ``for`` keyword, the ``(`` delimiter, and the ``;`` at
   the end of a statement are all stripped. A JavaScript ``for-of`` loop
   and a Python ``for-in`` loop both produce ``SIRForOfStmt``.

2. **Immutable and typed** — all nodes are ``@dataclass`` instances.
   Treat them as immutable: produce new nodes instead of mutating.

3. **Types as optional sidecars** — every expression carries
   ``resolved_type: SIRType`` defaulting to ``SIRAnyType()``. Type
   inference enriches this field without touching node structure.

4. **Source location on every node** — ``loc: SIRSourceLocation | None``
   is present on every node. Populated by the lowering pass from the
   parser's position info. Preserved through transformations.

5. **Extension bags** — every node has ``extra: dict`` for language-specific
   metadata that has no universal equivalent (Rust lifetimes, Python
   decorators, JS ``/*@__PURE__*/`` hints).

6. **Escape hatch** — ``SIRLangSpecific`` wraps constructs that cannot be
   normalised (``unsafe`` blocks in Rust, ``with`` statements in Python).

Strictness directionality
-------------------------

Going *down* the strictness ladder (TypeScript → JavaScript, Java → Python)
is always mechanical: drop the information that the target doesn't need.

Going *up* (JavaScript → TypeScript, Python → Java) requires inventing
information that doesn't exist in the source. This is what type inference
does. Without inference, every unknown type becomes ``SIRAnyType``, which
generators emit as ``any`` (TypeScript) or ``Object`` (Java).

The three emitter modes:

``STRICT``
    Fail if any expression's ``resolved_type`` is ``SIRAnyType``.

``LENIENT``
    Emit ``any`` / ``Object`` wherever ``resolved_type`` is ``SIRAnyType``.
    The output is valid but low quality.

``INFERRED``
    Run a type-inference pass first; fall back to ``any`` for what it
    cannot resolve.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Literal

# ---------------------------------------------------------------------------
# Source location sidecar
# ---------------------------------------------------------------------------
# Every SIR node carries an optional ``loc`` field pointing back to the
# original source position. This makes error messages precise and provides
# the raw data needed to build V3 source maps in the future.


@dataclass
class SIRSourceLocation:
    """Position of a node in the original source file.

    Line numbers are 1-based (first line = 1). Column numbers are
    0-based (first character = 0), matching the V3 source map spec
    and most editor conventions.

    Example — the identifier ``x`` in ``let x = 1;`` on line 3, column 4::

        SIRSourceLocation(file="app.js", start_line=3, start_col=4,
                          end_line=3, end_col=5)
    """

    file: str | None  # source file path; None if not known
    start_line: int  # 1-based
    start_col: int  # 0-based
    end_line: int  # 1-based
    end_col: int  # 0-based


# ---------------------------------------------------------------------------
# Type nodes — the type sidecar
# ---------------------------------------------------------------------------
# Every expression carries ``resolved_type: SIRType`` defaulting to
# ``SIRAnyType()``. When a source file has explicit annotations (TypeScript,
# Rust, Java) those are preserved here. When it doesn't (JavaScript, Python
# without hints) every expression starts as ``SIRAnyType`` and can be
# upgraded by a separate inference pass.
#
# Think of the type sidecar like a sticky note attached to each expression:
# it starts blank (SIRAnyType) and gets filled in as more is learned about
# the program.


@dataclass
class SIRAnyType:
    """Unknown or untyped — the default for all expressions.

    Used when:
    - The source language has no type annotations (plain JavaScript, Python
      without hints).
    - The type cannot be inferred.
    - The type has been intentionally marked unknown (TypeScript ``any``).

    When emitting TypeScript in LENIENT mode, ``SIRAnyType`` becomes ``any``.
    When emitting Java in LENIENT mode it becomes ``Object``.
    """

    type: Literal["any"] = field(default="any", init=False)


@dataclass
class SIRNeverType:
    """Unreachable or bottom type.

    Used for:
    - The return type of a function that always throws.
    - TypeScript's ``never`` type.
    - The result of exhaustive type narrowing with no remaining case.
    """

    type: Literal["never"] = field(default="never", init=False)


@dataclass
class SIRVoidType:
    """No return value — for functions that return nothing.

    Different from ``SIRAnyType``: ``void`` means "returns nothing on
    purpose". ``any`` means "we don't know what it returns".
    """

    type: Literal["void"] = field(default="void", init=False)


@dataclass
class SIRPrimitiveType:
    """A named primitive type shared across most languages.

    Primitive names map to language-specific equivalents:

    +--------------+------------+---------+------+-------+
    | SIR name     | TypeScript | Python  | Java | Rust  |
    +==============+============+=========+======+=======+
    | ``string``   | string     | str     | String | String |
    | ``number``   | number     | float   | double | f64  |
    | ``boolean``  | boolean    | bool    | boolean | bool |
    | ``null``     | null       | None    | null | (N/A) |
    | ``undefined``| undefined  | (N/A)   | (N/A)| (N/A) |
    | ``symbol``   | symbol     | (N/A)   | (N/A)| (N/A) |
    | ``bigint``   | bigint     | int     | BigInteger | (N/A) |
    +--------------+------------+---------+------+-------+
    """

    name: Literal["string", "number", "boolean", "null", "undefined", "symbol", "bigint"]
    type: Literal["primitive"] = field(default="primitive", init=False)


@dataclass
class SIRUnionType:
    """A union of two or more types: ``T | U | V``.

    Order is preserved but not semantically significant (union is commutative).
    Duplicate types are not automatically removed — callers may normalise if
    desired.
    """

    types: list[SIRType] = field(default_factory=list)
    type: Literal["union"] = field(default="union", init=False)


@dataclass
class SIRIntersectionType:
    """An intersection of two or more types: ``T & U``.

    Primarily a TypeScript concept. In languages without intersection types,
    generators may approximate with the first type or emit a comment.
    """

    types: list[SIRType] = field(default_factory=list)
    type: Literal["intersection"] = field(default="intersection", init=False)


@dataclass
class SIRArrayType:
    """A homogeneous array type: ``T[]`` or ``Array<T>``.

    For a tuple with known element types, see ``SIRTupleType``.
    """

    element: SIRType = field(default_factory=SIRAnyType)
    type: Literal["array"] = field(default="array", init=False)


@dataclass
class SIRObjectField:
    """One field within an ``SIRObjectType``.

    ``required=True`` means the field must be present (TypeScript ``name: T``).
    ``required=False`` means it may be absent (TypeScript ``name?: T``).
    """

    name: str
    value_type: SIRType = field(default_factory=SIRAnyType)
    required: bool = True


@dataclass
class SIRObjectType:
    """A structural object type with named fields.

    This is TypeScript's ``{ key: T; ... }`` syntax or a Python TypedDict.
    Not to be confused with a class — this is a *structural* type (shape),
    not a *nominal* type (name).
    """

    fields: list[SIRObjectField] = field(default_factory=list)
    type: Literal["object"] = field(default="object", init=False)


@dataclass
class SIRFunctionType:
    """A function type: ``(T, U) => V``.

    ``params`` holds the positional parameter types in order. Named
    parameters and rest parameters are not represented here — use
    ``SIRFunctionDecl.params`` for the full parameter list.
    """

    params: list[SIRType] = field(default_factory=list)
    return_type: SIRType = field(default_factory=SIRAnyType)
    type: Literal["function"] = field(default="function", init=False)


@dataclass
class SIRGenericType:
    """A generic (parameterised) type application: ``Array<T>``, ``Map<K, V>``.

    ``name`` is the base type name; ``args`` are the type arguments.
    The base type is not resolved here — resolution happens in a separate
    symbol-binding pass.

    Examples::

        SIRGenericType(name="Array", args=[SIRPrimitiveType("string")])
        SIRGenericType(name="Map",   args=[SIRPrimitiveType("string"),
                                          SIRPrimitiveType("number")])
        SIRGenericType(name="Promise", args=[SIRAnyType()])
    """

    name: str
    args: list[SIRType] = field(default_factory=list)
    type: Literal["generic"] = field(default="generic", init=False)


@dataclass
class SIRReferenceType:
    """An unresolved named type reference.

    Used when a type name appears in the source but has not yet been looked
    up in the symbol table. A subsequent binding pass replaces this with the
    resolved ``SIRClassDecl``, ``SIRInterfaceDecl``, or ``SIRTypeAliasDecl``
    target — or leaves it as a ``SIRReferenceType`` if the target is in an
    external package.

    Example::

        SIRReferenceType(name="MyClass")
        SIRReferenceType(name="EventEmitter")
    """

    name: str
    type: Literal["reference"] = field(default="reference", init=False)


@dataclass
class SIRTupleType:
    """A fixed-length tuple with per-position types: ``[T, U, V]``.

    Used for TypeScript tuples and Python tuple type hints.
    Order is significant.
    """

    elements: list[SIRType] = field(default_factory=list)
    type: Literal["tuple"] = field(default="tuple", init=False)


# Union alias for all type nodes.
SIRType = (
    SIRAnyType
    | SIRNeverType
    | SIRVoidType
    | SIRPrimitiveType
    | SIRUnionType
    | SIRIntersectionType
    | SIRArrayType
    | SIRObjectType
    | SIRFunctionType
    | SIRGenericType
    | SIRReferenceType
    | SIRTupleType
)


# ---------------------------------------------------------------------------
# Parameters
# ---------------------------------------------------------------------------


@dataclass
class SIRParam:
    """A single function parameter.

    Covers all parameter styles across languages:

    +--------------------------+------------------------------------------+
    | Style                    | Languages                                |
    +==========================+==========================================+
    | ``name: T = default``   | TypeScript, Python, Kotlin, Swift        |
    | ``name: T``             | TypeScript, Java, Go, Rust               |
    | ``name = default``      | Python (no annotation), JavaScript      |
    | ``...name`` / ``*name`` | JavaScript/TypeScript rest, Python *args|
    +--------------------------+------------------------------------------+
    """

    name: str
    type_annotation: SIRType = field(default_factory=SIRAnyType)
    default_value: SIRExpression | None = None
    rest: bool = False  # True for *args / ...rest spread parameters
    loc: SIRSourceLocation | None = None
    extra: dict[str, Any] = field(default_factory=dict)


# ---------------------------------------------------------------------------
# Forward declarations (resolved by TYPE_CHECKING below)
# ---------------------------------------------------------------------------
# Python dataclasses can reference each other in type hints only if the
# hints are strings or if we use ``from __future__ import annotations``
# (already imported at the top). All forward references are therefore
# resolved automatically at runtime via PEP 563 lazy evaluation.


# ---------------------------------------------------------------------------
# Declarations
# ---------------------------------------------------------------------------


@dataclass
class SIRVariableDecl:
    """A variable declaration: ``let``, ``const``, ``var``, or assignment.

    The ``kind`` field distinguishes:

    - ``"let"``    — reassignable binding (JS ``let``, Python variable)
    - ``"const"``  — immutable binding (JS ``const``, Rust ``let``)
    - ``"var"``    — function-scoped (JS ``var``; avoid in new code)
    - ``"assign"`` — plain assignment without declaration keyword
                     (Python ``x = 1``, Ruby ``x = 1``)

    Examples::

        # JavaScript: const msg = "hello";
        SIRVariableDecl(name="msg", kind="const",
                        value=SIRLiteral(value="hello"))

        # TypeScript: let count: number = 0;
        SIRVariableDecl(name="count", kind="let",
                        value=SIRLiteral(value=0),
                        type_annotation=SIRPrimitiveType("number"))

        # Python: x = 42
        SIRVariableDecl(name="x", kind="assign",
                        value=SIRLiteral(value=42))
    """

    name: str
    kind: Literal["let", "const", "var", "assign"] = "let"
    value: SIRExpression | None = None
    type_annotation: SIRType = field(default_factory=SIRAnyType)
    loc: SIRSourceLocation | None = None
    extra: dict[str, Any] = field(default_factory=dict)
    type: Literal["variable_decl"] = field(default="variable_decl", init=False)


@dataclass
class SIRBlock:
    """A sequence of declarations and statements enclosed in a scope.

    Corresponds to ``{ ... }`` in C-family languages and an indented block
    in Python. The block introduces a new lexical scope.
    """

    body: list[SIRDeclaration | SIRStatement] = field(default_factory=list)
    loc: SIRSourceLocation | None = None
    type: Literal["block"] = field(default="block", init=False)


@dataclass
class SIRMethodDef:
    """A method defined inside a class body.

    ``kind`` values:

    - ``"constructor"``   — the class constructor (``__init__`` in Python)
    - ``"method"``        — a regular instance method
    - ``"getter"``        — a property getter (``get name() { ... }``)
    - ``"setter"``        — a property setter (``set name(v) { ... }``)
    - ``"static_method"`` — a static method (``static foo() { ... }``)
    """

    name: str
    kind: Literal["constructor", "method", "getter", "setter", "static_method"] = "method"
    params: list[SIRParam] = field(default_factory=list)
    return_type: SIRType = field(default_factory=SIRAnyType)
    body: SIRBlock = field(default_factory=SIRBlock)
    loc: SIRSourceLocation | None = None
    extra: dict[str, Any] = field(default_factory=dict)
    type: Literal["method_def"] = field(default="method_def", init=False)


@dataclass
class SIRPropertyDef:
    """A property (field) defined inside a class body.

    ``static=True`` means a class-level property (``static x = 1``).
    """

    name: str
    value: SIRExpression | None = None
    type_annotation: SIRType = field(default_factory=SIRAnyType)
    static: bool = False
    loc: SIRSourceLocation | None = None
    extra: dict[str, Any] = field(default_factory=dict)
    type: Literal["property_def"] = field(default="property_def", init=False)


@dataclass
class SIRFunctionDecl:
    """A named function declaration.

    This represents the statement form ``function name(...) { ... }``.
    Anonymous and arrow functions are ``SIRFunctionExpression`` and
    ``SIRArrowFunction`` respectively.

    ``is_async=True`` corresponds to ``async function`` in JS/TS or
    ``async def`` in Python.

    ``is_generator=True`` corresponds to ``function*`` in JS/TS or
    a function containing ``yield`` in Python.

    Example::

        # function greet(name) { return "Hello, " + name; }
        SIRFunctionDecl(
            name="greet",
            params=[SIRParam(name="name")],
            body=SIRBlock(body=[
                SIRReturnStmt(value=SIRBinaryOp(
                    op="+",
                    left=SIRLiteral(value="Hello, "),
                    right=SIRIdentifier(name="name"),
                ))
            ]),
        )
    """

    name: str
    params: list[SIRParam] = field(default_factory=list)
    return_type: SIRType = field(default_factory=SIRAnyType)
    body: SIRBlock = field(default_factory=SIRBlock)
    is_async: bool = False
    is_generator: bool = False
    loc: SIRSourceLocation | None = None
    extra: dict[str, Any] = field(default_factory=dict)
    type: Literal["function_decl"] = field(default="function_decl", init=False)


@dataclass
class SIRClassDecl:
    """A class declaration.

    ``superclass`` is the *name* of the parent class (not a resolved node).
    ``type_params`` are the generic type parameter names (``["T", "K", "V"]``).

    Example::

        # class Animal extends Living { ... }
        SIRClassDecl(name="Animal", superclass="Living", members=[...])
    """

    name: str
    superclass: str | None = None
    type_params: list[str] = field(default_factory=list)
    members: list[SIRMethodDef | SIRPropertyDef] = field(default_factory=list)
    loc: SIRSourceLocation | None = None
    extra: dict[str, Any] = field(default_factory=dict)
    type: Literal["class_decl"] = field(default="class_decl", init=False)


@dataclass
class SIRPropertySignature:
    """A property signature inside an interface."""

    name: str
    value_type: SIRType = field(default_factory=SIRAnyType)
    required: bool = True
    type: Literal["property_signature"] = field(default="property_signature", init=False)


@dataclass
class SIRMethodSignature:
    """A method signature inside an interface."""

    name: str
    params: list[SIRParam] = field(default_factory=list)
    return_type: SIRType = field(default_factory=SIRAnyType)
    type: Literal["method_signature"] = field(default="method_signature", init=False)


@dataclass
class SIRInterfaceDecl:
    """A TypeScript / Java interface declaration.

    For languages without interfaces, generators may emit an abstract class,
    a Protocol (Python), or skip the node entirely.
    """

    name: str
    extends: list[str] = field(default_factory=list)
    members: list[SIRPropertySignature | SIRMethodSignature] = field(default_factory=list)
    loc: SIRSourceLocation | None = None
    extra: dict[str, Any] = field(default_factory=dict)
    type: Literal["interface_decl"] = field(default="interface_decl", init=False)


@dataclass
class SIRTypeAliasDecl:
    """A type alias declaration: ``type Name = T``.

    TypeScript-specific. Other generators may ignore this node or emit
    an equivalent comment.
    """

    name: str
    value: SIRType = field(default_factory=SIRAnyType)
    loc: SIRSourceLocation | None = None
    extra: dict[str, Any] = field(default_factory=dict)
    type: Literal["type_alias_decl"] = field(default="type_alias_decl", init=False)


@dataclass
class SIRImportSpecifier:
    """One named import inside an import statement.

    ``imported`` is the name in the source module.
    ``local`` is the name used in the current module (may differ due to
    ``import { foo as bar } from "..."``).

    Example::

        # import { add as sum } from "./math"
        SIRImportSpecifier(imported="add", local="sum")
    """

    imported: str
    local: str


@dataclass
class SIRImport:
    """An import declaration.

    Covers all import styles:

    +---------------------------------------+----------------------------------+
    | Source                                | SIR                              |
    +=======================================+==================================+
    | ``import Foo from "m"``               | default="Foo"                    |
    | ``import * as ns from "m"``           | namespace="ns"                   |
    | ``import { a, b as c } from "m"``     | specifiers=[...]                 |
    | ``import Foo, { a } from "m"``        | default="Foo", specifiers=[...]  |
    | ``import "m"`` (side-effect)          | source="m", all others None/[]   |
    +---------------------------------------+----------------------------------+
    """

    source: str
    default: str | None = None
    namespace: str | None = None
    specifiers: list[SIRImportSpecifier] = field(default_factory=list)
    loc: SIRSourceLocation | None = None
    extra: dict[str, Any] = field(default_factory=dict)
    type: Literal["import"] = field(default="import", init=False)


@dataclass
class SIRExportSpecifier:
    """One named export inside an export statement."""

    local: str
    exported: str


@dataclass
class SIRExport:
    """An export declaration.

    ``default`` holds the default export (expression or declaration).
    ``specifiers`` holds named exports.
    ``source`` is set for re-exports (``export { x } from "..."``).
    """

    default: SIRExpression | SIRDeclaration | None = None
    specifiers: list[SIRExportSpecifier] = field(default_factory=list)
    source: str | None = None
    loc: SIRSourceLocation | None = None
    extra: dict[str, Any] = field(default_factory=dict)
    type: Literal["export"] = field(default="export", init=False)


# ---------------------------------------------------------------------------
# Statements
# ---------------------------------------------------------------------------


@dataclass
class SIRExpressionStmt:
    """An expression used as a statement.

    Example::

        # console.log("hello");
        SIRExpressionStmt(expression=SIRCall(
            callee=SIRMemberAccess(object=SIRIdentifier("console"),
                                   property="log"),
            args=[SIRLiteral(value="hello")],
        ))
    """

    expression: SIRExpression
    loc: SIRSourceLocation | None = None
    type: Literal["expression_stmt"] = field(default="expression_stmt", init=False)


@dataclass
class SIRIfStmt:
    """An if / else statement.

    ``alternate=None`` means there is no else branch.
    ``alternate`` may itself contain another ``SIRIfStmt`` as the sole
    statement in its block (representing ``else if``).

    Example::

        # if (x > 0) { ... } else { ... }
        SIRIfStmt(
            test=SIRBinaryOp(op=">", left=SIRIdentifier("x"),
                             right=SIRLiteral(value=0)),
            consequent=SIRBlock(body=[...]),
            alternate=SIRBlock(body=[...]),
        )
    """

    test: SIRExpression
    consequent: SIRBlock
    alternate: SIRBlock | None = None
    loc: SIRSourceLocation | None = None
    type: Literal["if"] = field(default="if", init=False)


@dataclass
class SIRWhileStmt:
    """A while loop."""

    test: SIRExpression
    body: SIRBlock
    loc: SIRSourceLocation | None = None
    type: Literal["while"] = field(default="while", init=False)


@dataclass
class SIRForOfStmt:
    """A for-of / for-in loop that iterates over *values*.

    Corresponds to:
    - JavaScript: ``for (const x of iterable)``
    - Python: ``for x in iterable:``
    - Ruby: ``iterable.each do |x|``

    ``is_await=True`` corresponds to ``for await (const x of asyncIterable)``.
    """

    binding: str
    iterable: SIRExpression
    body: SIRBlock
    is_await: bool = False
    loc: SIRSourceLocation | None = None
    type: Literal["for_of"] = field(default="for_of", init=False)


@dataclass
class SIRForInStmt:
    """A for-in loop that iterates over *keys* of an object.

    JavaScript-specific semantics: ``for (const key in obj)`` iterates over
    enumerable property keys. Python's ``for`` loop iterates over values, so
    Python generators should emit this as ``for k in obj.keys():`` or
    ``for k in obj:``.
    """

    binding: str
    object: SIRExpression
    body: SIRBlock
    loc: SIRSourceLocation | None = None
    type: Literal["for_in"] = field(default="for_in", init=False)


@dataclass
class SIRForStmt:
    """A C-style for loop: ``for (init; test; update) { body }``.

    Any of ``init``, ``test``, ``update`` may be ``None`` (corresponding to
    ``for (;;)`` which loops forever).
    """

    init: SIRVariableDecl | SIRExpression | None = None
    test: SIRExpression | None = None
    update: SIRExpression | None = None
    body: SIRBlock = field(default_factory=SIRBlock)
    loc: SIRSourceLocation | None = None
    type: Literal["for"] = field(default="for", init=False)


@dataclass
class SIRReturnStmt:
    """A return statement.

    ``value=None`` corresponds to a bare ``return;`` (returns undefined /
    None / void).
    """

    value: SIRExpression | None = None
    loc: SIRSourceLocation | None = None
    type: Literal["return"] = field(default="return", init=False)


@dataclass
class SIRThrowStmt:
    """A throw statement."""

    value: SIRExpression
    loc: SIRSourceLocation | None = None
    type: Literal["throw"] = field(default="throw", init=False)


@dataclass
class SIRCatchClause:
    """The catch clause of a try statement.

    ``binding`` is the variable name bound to the caught exception.
    ``binding=None`` corresponds to ``catch { ... }`` (no binding).
    """

    binding: str | None = None
    type_annotation: SIRType = field(default_factory=SIRAnyType)
    body: SIRBlock = field(default_factory=SIRBlock)
    loc: SIRSourceLocation | None = None
    type: Literal["catch"] = field(default="catch", init=False)


@dataclass
class SIRTryStmt:
    """A try / catch / finally statement.

    At least one of ``handler`` or ``finalizer`` must be present (a bare
    ``try { }`` with no handler or finalizer is a syntax error in most
    languages).
    """

    body: SIRBlock
    handler: SIRCatchClause | None = None
    finalizer: SIRBlock | None = None
    loc: SIRSourceLocation | None = None
    type: Literal["try"] = field(default="try", init=False)


@dataclass
class SIRBreakStmt:
    """A break statement, optionally with a label."""

    label: str | None = None
    loc: SIRSourceLocation | None = None
    type: Literal["break"] = field(default="break", init=False)


@dataclass
class SIRContinueStmt:
    """A continue statement, optionally with a label."""

    label: str | None = None
    loc: SIRSourceLocation | None = None
    type: Literal["continue"] = field(default="continue", init=False)


@dataclass
class SIRSwitchCase:
    """One case inside a switch statement.

    ``test=None`` is the ``default:`` case.
    """

    test: SIRExpression | None = None
    body: list[SIRStatement] = field(default_factory=list)
    loc: SIRSourceLocation | None = None
    type: Literal["switch_case"] = field(default="switch_case", init=False)


@dataclass
class SIRSwitchStmt:
    """A switch statement."""

    discriminant: SIRExpression
    cases: list[SIRSwitchCase] = field(default_factory=list)
    loc: SIRSourceLocation | None = None
    type: Literal["switch"] = field(default="switch", init=False)


@dataclass
class SIRLangSpecific:
    """Escape hatch for constructs that cannot be normalised to the SIR.

    Some language constructs have no universal equivalent and cannot be
    represented in the core node set without losing essential semantics.
    ``SIRLangSpecific`` carries them through the pipeline transparently:

    - Generators that understand ``language`` can emit the construct natively.
    - Generators that don't understand it can skip ``children`` (lose the
      code) or emit a comment noting the untranslatable section.

    Examples::

        # Python: with open("f") as handle:
        SIRLangSpecific(language="python", construct="with_statement",
                        children=[...handle_body...],
                        extra={"context_expr": ..., "binding": "handle"})

        # Rust: unsafe { ... }
        SIRLangSpecific(language="rust", construct="unsafe_block",
                        children=[...body...])
    """

    language: str
    construct: str
    children: list[SIRNode] = field(default_factory=list)
    loc: SIRSourceLocation | None = None
    extra: dict[str, Any] = field(default_factory=dict)
    type: Literal["lang_specific"] = field(default="lang_specific", init=False)


# ---------------------------------------------------------------------------
# Expressions
# ---------------------------------------------------------------------------
# Every expression node carries:
#   resolved_type: SIRType        — default SIRAnyType()
#   loc: SIRSourceLocation | None — default None


@dataclass
class SIRLiteral:
    """A literal value: number, string, boolean, or null/None.

    ``value`` holds the Python representation:
    - Numbers: ``int`` or ``float``
    - Strings: ``str`` (already decoded — no escape sequences)
    - Booleans: ``True`` / ``False``
    - Null / None / nil: Python ``None``

    The ``resolved_type`` is set automatically where it is unambiguous::

        SIRLiteral(value=42)        # resolved_type should be SIRPrimitiveType("number")
        SIRLiteral(value="hello")   # resolved_type should be SIRPrimitiveType("string")
        SIRLiteral(value=True)      # resolved_type should be SIRPrimitiveType("boolean")
        SIRLiteral(value=None)      # resolved_type should be SIRPrimitiveType("null")

    The lowering pass (js-ast-to-sir) fills in ``resolved_type`` from the
    literal value.
    """

    value: int | float | str | bool | None
    resolved_type: SIRType = field(default_factory=SIRAnyType)
    loc: SIRSourceLocation | None = None
    extra: dict[str, Any] = field(default_factory=dict)
    type: Literal["literal"] = field(default="literal", init=False)


@dataclass
class SIRIdentifier:
    """A name reference (variable, parameter, imported binding, etc.)."""

    name: str
    resolved_type: SIRType = field(default_factory=SIRAnyType)
    loc: SIRSourceLocation | None = None
    type: Literal["identifier"] = field(default="identifier", init=False)


@dataclass
class SIRBinaryOp:
    """A binary operation between two expressions.

    Operator strings use JavaScript/TypeScript conventions. Python-specific
    operators are normalised:

    +------------------+------------------------+
    | Python source    | SIR op                 |
    +==================+========================+
    | ``**``           | ``"**"``               |
    | ``//``           | ``"//"`` (floor div)   |
    | ``and``          | ``"&&"``               |
    | ``or``           | ``"||"``               |
    | ``is``           | ``"==="`` (approx.)    |
    | ``is not``       | ``"!=="`` (approx.)    |
    | ``in``           | ``"in"``               |
    | ``not in``       | ``"not_in"``           |
    +------------------+------------------------+

    Strict equality (``===`` / ``!==``) is preserved from JavaScript source;
    abstract equality (``==`` / ``!=``) is kept as-is.
    """

    op: str
    left: SIRExpression
    right: SIRExpression
    resolved_type: SIRType = field(default_factory=SIRAnyType)
    loc: SIRSourceLocation | None = None
    type: Literal["binary_op"] = field(default="binary_op", init=False)


@dataclass
class SIRUnaryOp:
    """A unary operation on a single expression.

    ``prefix=True`` means the operator comes before the operand (``-x``).
    ``prefix=False`` means it comes after (``x++``, ``x--``).
    """

    op: str  # "-", "+", "!", "~", "typeof", "void", "delete", "not", "++" , "--"
    operand: SIRExpression
    prefix: bool = True
    resolved_type: SIRType = field(default_factory=SIRAnyType)
    loc: SIRSourceLocation | None = None
    type: Literal["unary_op"] = field(default="unary_op", init=False)


@dataclass
class SIRSpread:
    """A spread expression: ``...value``.

    Used inside ``SIRCall.args`` and ``SIRArrayLiteral.elements`` and
    ``SIRObjectLiteral.properties``.
    """

    value: SIRExpression
    type: Literal["spread"] = field(default="spread", init=False)


@dataclass
class SIRAssignment:
    """An assignment expression (returns the assigned value).

    Includes compound assignments: ``+=``, ``-=``, ``&&=``, etc.

    ``target`` must be a valid assignment target:
    ``SIRIdentifier``, ``SIRMemberAccess``, or ``SIRIndex``.
    """

    op: str  # "=", "+=", "-=", "*=", "/=", "%=", "**=", etc.
    target: SIRExpression
    value: SIRExpression
    resolved_type: SIRType = field(default_factory=SIRAnyType)
    loc: SIRSourceLocation | None = None
    type: Literal["assignment"] = field(default="assignment", init=False)


@dataclass
class SIRCall:
    """A function or method call expression.

    ``is_new=True`` represents constructor calls: ``new Foo(args)``.
    ``is_optional=True`` represents optional calls: ``foo?.(args)``.
    ``type_args`` holds TypeScript generic type arguments: ``foo<T>(args)``.
    """

    callee: SIRExpression
    args: list[SIRExpression | SIRSpread] = field(default_factory=list)
    type_args: list[SIRType] = field(default_factory=list)
    is_new: bool = False
    is_optional: bool = False
    resolved_type: SIRType = field(default_factory=SIRAnyType)
    loc: SIRSourceLocation | None = None
    type: Literal["call"] = field(default="call", init=False)


@dataclass
class SIRMemberAccess:
    """A property access expression: ``obj.prop`` or ``obj[prop]``.

    ``computed=False`` → dot notation: ``obj.prop``
    ``computed=True``  → bracket notation: ``obj[prop]`` — but use
    ``SIRIndex`` for this; ``SIRMemberAccess`` is dot notation only.
    ``optional=True``  → optional chaining: ``obj?.prop``

    Note: computed property access with a dynamic key (``obj[expr]``) is
    represented as ``SIRIndex``, not ``SIRMemberAccess``. ``computed=True``
    here is reserved for cases where the property name is a literal string
    used in bracket notation (``obj["prop"]``), which is semantically
    equivalent to dot notation.
    """

    object: SIRExpression
    property: str
    computed: bool = False
    optional: bool = False
    resolved_type: SIRType = field(default_factory=SIRAnyType)
    loc: SIRSourceLocation | None = None
    type: Literal["member_access"] = field(default="member_access", init=False)


@dataclass
class SIRIndex:
    """Dynamic index access: ``obj[expr]``.

    Used when the index is an arbitrary expression (not a static property
    name). Examples: ``arr[0]``, ``map[key]``, ``obj[computedName]``.
    """

    object: SIRExpression
    index: SIRExpression
    resolved_type: SIRType = field(default_factory=SIRAnyType)
    loc: SIRSourceLocation | None = None
    type: Literal["index"] = field(default="index", init=False)


@dataclass
class SIRConditional:
    """The ternary conditional expression: ``test ? consequent : alternate``.

    All three sub-expressions are required (unlike ``SIRIfStmt`` where the
    else branch is optional).
    """

    test: SIRExpression
    consequent: SIRExpression
    alternate: SIRExpression
    resolved_type: SIRType = field(default_factory=SIRAnyType)
    loc: SIRSourceLocation | None = None
    type: Literal["conditional"] = field(default="conditional", init=False)


@dataclass
class SIRProperty:
    """One key-value pair inside an object literal.

    ``key`` is either a static string name or a computed expression
    (``[expr]: value``).

    ``shorthand=True`` means ``{ x }`` shorthand notation (only valid when
    key is a string equal to the identifier name of value).

    ``computed=True`` means ``{ [expr]: value }`` computed property.
    """

    key: str | SIRExpression
    value: SIRExpression
    shorthand: bool = False
    computed: bool = False
    type: Literal["property"] = field(default="property", init=False)


@dataclass
class SIRObjectLiteral:
    """An object literal: ``{ key: value, ...spread }``.

    Properties are either ``SIRProperty`` (named key-value pairs) or
    ``SIRSpread`` (``...obj`` spread).
    """

    properties: list[SIRProperty | SIRSpread] = field(default_factory=list)
    resolved_type: SIRType = field(default_factory=SIRAnyType)
    loc: SIRSourceLocation | None = None
    type: Literal["object_literal"] = field(default="object_literal", init=False)


@dataclass
class SIRArrayLiteral:
    """An array literal: ``[elem, ...spread, elem]``.

    Elements are expressions, spreads, or ``None`` (elision holes in JS:
    ``[1, , 3]``).
    """

    elements: list[SIRExpression | SIRSpread | None] = field(default_factory=list)
    resolved_type: SIRType = field(default_factory=SIRAnyType)
    loc: SIRSourceLocation | None = None
    type: Literal["array_literal"] = field(default="array_literal", init=False)


@dataclass
class SIRArrowFunction:
    """An arrow function expression: ``(params) => body``.

    ``body`` is either:
    - A ``SIRBlock`` for a block-body arrow function:
      ``(x) => { return x + 1; }``
    - A ``SIRExpression`` for a concise-body arrow function:
      ``(x) => x + 1``

    ``is_async=True`` corresponds to ``async (x) => ...``.
    """

    params: list[SIRParam] = field(default_factory=list)
    body: SIRBlock | SIRExpression = field(default_factory=SIRBlock)
    return_type: SIRType = field(default_factory=SIRAnyType)
    is_async: bool = False
    resolved_type: SIRType = field(default_factory=SIRAnyType)
    loc: SIRSourceLocation | None = None
    type: Literal["arrow_function"] = field(default="arrow_function", init=False)


@dataclass
class SIRFunctionExpression:
    """An anonymous or named function expression: ``function name?(...) { ... }``.

    ``name=None`` for anonymous functions.
    ``is_generator=True`` for generator functions (``function*``).
    """

    name: str | None = None
    params: list[SIRParam] = field(default_factory=list)
    body: SIRBlock = field(default_factory=SIRBlock)
    return_type: SIRType = field(default_factory=SIRAnyType)
    is_async: bool = False
    is_generator: bool = False
    resolved_type: SIRType = field(default_factory=SIRAnyType)
    loc: SIRSourceLocation | None = None
    type: Literal["function_expression"] = field(default="function_expression", init=False)


@dataclass
class SIRTemplateLiteral:
    r"""A template literal: ``\`Hello, ${name}!\``

    ``quasis`` holds the static string parts. ``expressions`` holds the
    interpolated expressions. Their counts are always related by:
    ``len(quasis) == len(expressions) + 1``

    For the template literal ``\`Hello, ${name}! You are ${age} years old.\``::

        quasis      = ["Hello, ", "! You are ", " years old."]
        expressions = [SIRIdentifier("name"), SIRIdentifier("age")]
    """

    quasis: list[str]
    expressions: list[SIRExpression] = field(default_factory=list)
    resolved_type: SIRType = field(default_factory=SIRAnyType)
    loc: SIRSourceLocation | None = None
    type: Literal["template_literal"] = field(default="template_literal", init=False)


@dataclass
class SIRAwait:
    """An await expression: ``await value``."""

    value: SIRExpression
    resolved_type: SIRType = field(default_factory=SIRAnyType)
    loc: SIRSourceLocation | None = None
    type: Literal["await"] = field(default="await", init=False)


@dataclass
class SIRYield:
    """A yield expression: ``yield value`` or ``yield* value``.

    ``delegate=True`` corresponds to ``yield*`` which delegates to another
    iterable / generator.
    """

    value: SIRExpression | None = None
    delegate: bool = False
    resolved_type: SIRType = field(default_factory=SIRAnyType)
    loc: SIRSourceLocation | None = None
    type: Literal["yield"] = field(default="yield", init=False)


@dataclass
class SIRTypeAssertion:
    """A TypeScript type assertion: ``expr as TargetType``.

    Does not change runtime behaviour — it is a compile-time hint to the
    type checker. Non-TypeScript generators may emit the inner expression
    without the assertion.
    """

    value: SIRExpression
    target_type: SIRType = field(default_factory=SIRAnyType)
    resolved_type: SIRType = field(default_factory=SIRAnyType)
    loc: SIRSourceLocation | None = None
    type: Literal["type_assertion"] = field(default="type_assertion", init=False)


@dataclass
class SIRSequence:
    """The comma operator: ``(a, b, c)`` evaluates all and returns the last.

    Rarely used in practice but valid JavaScript. Generators may emit it
    as-is (JS) or split into separate statements (Python).
    """

    expressions: list[SIRExpression] = field(default_factory=list)
    resolved_type: SIRType = field(default_factory=SIRAnyType)
    loc: SIRSourceLocation | None = None
    type: Literal["sequence"] = field(default="sequence", init=False)


# ---------------------------------------------------------------------------
# Module root
# ---------------------------------------------------------------------------


@dataclass
class SIRModule:
    """The root node representing an entire source file or module.

    ``source_language`` records which language the SIR was lowered from.
    This helps generators decide how to handle ambiguous constructs and
    allows ``SIRLangSpecific`` nodes to be interpreted correctly.

    ``body`` holds the top-level declarations and statements in source order.
    """

    source_language: str | None = None  # "javascript", "typescript", "python", …
    body: list[SIRDeclaration | SIRStatement] = field(default_factory=list)
    loc: SIRSourceLocation | None = None
    extra: dict[str, Any] = field(default_factory=dict)
    type: Literal["module"] = field(default="module", init=False)


# ---------------------------------------------------------------------------
# Union type aliases
# ---------------------------------------------------------------------------
# These are the closed union types used in type annotations throughout the
# package. They are defined after all the dataclasses because Python
# resolves forward references lazily (via ``from __future__ import
# annotations``).

SIRDeclaration = (
    SIRVariableDecl
    | SIRFunctionDecl
    | SIRClassDecl
    | SIRInterfaceDecl
    | SIRTypeAliasDecl
    | SIRImport
    | SIRExport
)

SIRStatement = (
    SIRBlock
    | SIRExpressionStmt
    | SIRIfStmt
    | SIRWhileStmt
    | SIRForOfStmt
    | SIRForInStmt
    | SIRForStmt
    | SIRReturnStmt
    | SIRThrowStmt
    | SIRTryStmt
    | SIRBreakStmt
    | SIRContinueStmt
    | SIRSwitchStmt
    | SIRLangSpecific
)

SIRExpression = (
    SIRLiteral
    | SIRIdentifier
    | SIRBinaryOp
    | SIRUnaryOp
    | SIRAssignment
    | SIRCall
    | SIRMemberAccess
    | SIRIndex
    | SIRConditional
    | SIRObjectLiteral
    | SIRArrayLiteral
    | SIRSpread
    | SIRArrowFunction
    | SIRFunctionExpression
    | SIRTemplateLiteral
    | SIRAwait
    | SIRYield
    | SIRTypeAssertion
    | SIRSequence
)

SIRNode = SIRModule | SIRDeclaration | SIRStatement | SIRExpression | SIRType

# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------

__all__ = [
    # Source location
    "SIRSourceLocation",
    # Type nodes
    "SIRAnyType",
    "SIRNeverType",
    "SIRVoidType",
    "SIRPrimitiveType",
    "SIRUnionType",
    "SIRIntersectionType",
    "SIRArrayType",
    "SIRObjectField",
    "SIRObjectType",
    "SIRFunctionType",
    "SIRGenericType",
    "SIRReferenceType",
    "SIRTupleType",
    "SIRType",
    # Parameters
    "SIRParam",
    # Declarations
    "SIRVariableDecl",
    "SIRFunctionDecl",
    "SIRMethodDef",
    "SIRPropertyDef",
    "SIRClassDecl",
    "SIRPropertySignature",
    "SIRMethodSignature",
    "SIRInterfaceDecl",
    "SIRTypeAliasDecl",
    "SIRImportSpecifier",
    "SIRImport",
    "SIRExportSpecifier",
    "SIRExport",
    "SIRDeclaration",
    # Statements
    "SIRBlock",
    "SIRExpressionStmt",
    "SIRIfStmt",
    "SIRWhileStmt",
    "SIRForOfStmt",
    "SIRForInStmt",
    "SIRForStmt",
    "SIRReturnStmt",
    "SIRThrowStmt",
    "SIRCatchClause",
    "SIRTryStmt",
    "SIRBreakStmt",
    "SIRContinueStmt",
    "SIRSwitchCase",
    "SIRSwitchStmt",
    "SIRLangSpecific",
    "SIRStatement",
    # Expressions
    "SIRLiteral",
    "SIRIdentifier",
    "SIRBinaryOp",
    "SIRUnaryOp",
    "SIRSpread",
    "SIRAssignment",
    "SIRCall",
    "SIRMemberAccess",
    "SIRIndex",
    "SIRConditional",
    "SIRProperty",
    "SIRObjectLiteral",
    "SIRArrayLiteral",
    "SIRArrowFunction",
    "SIRFunctionExpression",
    "SIRTemplateLiteral",
    "SIRAwait",
    "SIRYield",
    "SIRTypeAssertion",
    "SIRSequence",
    "SIRExpression",
    # Module root
    "SIRModule",
    # All nodes
    "SIRNode",
]
