"""Type checking and *unoptimised* lowering of a Warrant document to the model IR.

VER04r section 10.3 (check 1) says two independent implementations can only
agree byte for byte if lowering is **syntax-directed and unoptimised**:

* one IR node per source construct, operands in source order;
* no constant folding, no reassociation, no common-subexpression sharing, no
  inlining of models;
* desugarings exactly as VER04r section 4 and Appendix A name them
  (``a != b`` is ``["ne", a, b]``, ``x => y`` is ``["implies", x, y]``,
  unary minus is ``["neg", e]``, a refinement becomes part of ``admit``).

This module is the only place where the *typing* of a document is decided,
because three things in the lowering depend on types: the contextual type of a
bare decimal literal (``Nat`` or ``Int``), the static widths of ``Bytes<N>``
calls, and which IR feature level a construct needs.

Feature levels (VER04r section 4.1)
-----------------------------------
The oracle *evaluates* ``ir.core``, ``ir.tuple`` (slice S10t-1) and ``ir.fixed`` (slice S10c-1); since
slice S10a it *lowers and types* the 35 built-ins of ``ir.fixed`` (Appendix A.1) and, since slice S10c-1,
:mod:`.ir_eval` evaluates them too.  In a v0 document the built-ins keep the ``ir.fixed`` ``level``
refusal (VER04j 1.2: the names are version scoped).

``ir.tuple`` (slice S10t-1)
---------------------------
Tuple types ``(T1,..,Tn)``, the tuple literal, ``t.i`` / ``e.i`` projection and ``match`` on a
``Result`` (the six patterns ``ok err err_any tuple bind wild``) are lowered to
``["tuple", [..]]``, ``["project", e, i]`` and ``["match", s, [[pat, arm], ..]]`` (Appendix A.1) and
evaluated.  The patterns are checked **before** any arm body is lowered, so the corpus errors of A.1
(fall-through, an unreachable arm, a duplicate or shadowing ``bind``, oracle code ``corpus``) cannot be
hidden by a body that is only recorded as a level error.

``ir.data`` (slices S11a and S11c-1, VER04r section 16.14 [D46] to [D48], Appendix A.2)
-----------------------------------------------------------------------------------------
Records, variants and ``Option<T>`` are real types; the four tags ``["some", e]``, ``["none", "T"]``,
``["ctor", "Name", [e..]]`` and ``["field", e, i]`` are lowered, typed and costed; ``match`` is lowered on
**any** scrutinee with the data patterns ``some none ctor bool`` joining the six of A.1.  Slice S11a only
lowered them: a unit that needed ``ir.data`` carried the *evaluation* refusal ``level ir.data`` (the pattern
``ir.fixed`` followed in S10a).  **Slice S11c-1 evaluates them** (:mod:`.ir_eval`), so no unit carries that
refusal any more and :meth:`_Lowerer.seal` records none: ``unsupported`` is set only by a *lowering* refusal
(``ir.list``, ``ir.seq``, ``err-value``, ``fixed-cast``).  :attr:`LoweredModel.lowered` (and its siblings)
is still the flag that tells a unit that *lowered* from one that was *refused*.

``ir.list``, ``ir.seq`` and the two feature names (section 16.14 R27, R28)
--------------------------------------------------------------------------
Everything else that has no level the oracle implements is **refused**, by name, and the refusal is
recorded per unit (so a census can still report the units that do lower): ``ir.list`` for a list type
anywhere, a list literal and the source built-ins ``slice to_list to_bytes replicate``; ``ir.seq`` for
``fold map all any range count``; ``fixed-cast`` for the built-in ``fixed<N>(b)``; ``err-value`` for an
``Err(e)`` arm that *uses* ``e``.  Feature names stand in the level position of the refusal exactly as
``inout`` and ``ensures`` do.  A list type in a record, variant or alias declaration or in an op signature
is a unit of its own (R35) with no row to record on, so it is raised, refusing the whole document.

Refusal order (section 16.14 R35)
---------------------------------
(1) the declaration limits of section 2.2 come first (``limit``, :func:`_check_limits`); (2) within one unit
a level or feature refusal is named in preference to a *type* error of the same unit, and of several the
first in source order (:func:`_feature_scan`, a pre-order walk of the unit's source, consulted when the
unit raises a ``type`` error); (3) within a ``match`` every pattern is checked before any arm body, in arm
order, and inside a pattern outside in (an unknown constructor, a constructor or literal that does not fit,
a wrong field count, *then* the duplicate-bind and shadowing rules); (4) unreachable arms before
fall-through; (5) units in declaration order, so a raised error does not hide a refusal an *earlier* unit
recorded (:func:`_raise_in_order`).  The oracle lowers models before operation refinements, so across those
two kinds the order is the processing order (SPEC-QUESTION 1 of the S11a report).

``Option<T>`` and the intermediate use (section 16.10 D23 as amended by [D48]; R29)
-----------------------------------------------------------------------------------
``Option`` may appear wherever a type may.  The **intermediate use** (a producer node of
:data:`.levels.OPTION_PRODUCERS` that is directly the first operand of ``unwrap_or``) adds no ``ir.data``
(:meth:`_Lowerer.expr`); ``div``/``rem`` on two ``Nat`` operands are typed ``Option<Int>`` (R7).
"""

from __future__ import annotations

from dataclasses import dataclass, field
from functools import cached_property

from . import ast as A
from . import levels as L
from . import patterns as P
from .errors import OracleError
from .irtree import children, match_arms

#: ``ir_eval.MAX_IR_DEPTH`` (VER04r 14, IR nesting 64); repeated here because ``ir_eval`` imports this module.
MAX_IR_DEPTH = 64

# -- types ---------------------------------------------------------------------

BOOL = A.Ty("Bool")
INT = A.Ty("Int")
NAT = A.Ty("Nat")
BYTES = A.Ty("Bytes")

#: width, signedness of each fixed-width integer type (VER04j section 2.2).
FIXED = {
    "U8": (8, False),
    "U16": (16, False),
    "U32": (32, False),
    "U64": (64, False),
    "I8": (8, True),
    "I16": (16, True),
    "I32": (32, True),
    "I64": (64, True),
}
MAX_EXPR_DEPTH = 64
_PRIMITIVES = frozenset({"Bool", "Int", "Nat", "Bytes", *FIXED})

#: The source built-ins assigned to the reserved ``ir.list`` by name, whatever their operand (section 16.14
#: R28): the WX-D1 sequence built-ins of VER04k.
_LEVEL_LIST = frozenset(["slice", "to_list", "to_bytes", "replicate"])
#: The source built-ins of the reserved ``ir.seq`` (section 4.1; ``range`` is named there, R28).
_LEVEL_SEQ = frozenset(["fold", "map", "all", "any", "count", "range"])
#: The one built-in outside every level (R28): ``fixed<N>(b)``, refused with the feature name below.
_FIXED_CAST = "fixed"
#: The two feature names (section 4.1, [D55], R28), in the level position of a refusal.
FIXED_CAST = "fixed-cast"
ERR_VALUE = "err-value"
_FIXED_MESSAGE = "fixed-width built-ins need ir.fixed"

#: The names every document reserves (VER04 section 2; the checker's ``V0_BUILTINS``): the core
#: types, the core built-ins and the four constructors.
_RESERVED_V0 = frozenset(
    [
        "Bool", "Int", "Nat", "U8", "U64", "Bytes", "List", "Option", "Result",
        "len", "at", "concat", "fixed", "div", "Some", "None", "Ok", "Err",
    ]
)  # fmt: skip
#: The names a v1 document reserves on top (VER04j 1.2, VER04k 1.2): the fixed-width types, the
#: ``ir.fixed`` built-ins, the ``ir.data`` and ``ir.seq`` sequence names, and ``take``/``drop``.
#: Built from the tables above, so a built-in the lowerer learns is reserved with no second edit.
_RESERVED_V1 = frozenset(
    [*FIXED, *L.FIXED_TAGS, "Some", "None", "fixed", *_LEVEL_LIST, *_LEVEL_SEQ, "take", "drop"]
) | _RESERVED_V0


def is_fixed(ty: A.Ty) -> bool:
    return ty.head in FIXED


def is_numeric(ty: A.Ty) -> bool:
    return ty.head in ("Int", "Nat") or ty.head in FIXED


# -- the symbol table ----------------------------------------------------------


@dataclass
class LoweredModel:
    name: str
    params: list  # [(name, Ty)]
    ret: A.Ty
    #: The lowered body, or ``None`` only when lowering itself was impossible (a construct of
    #: a level the lowerer cannot lower).  A model that needs ``ir.fixed`` *has* its IR and
    #: is still ``unsupported`` (evaluation refuses), so a dump can show what S10a lowers.
    ir: object | None = None
    #: Why the unit cannot be *evaluated*: a lowering refusal (a reserved level or a feature name).  Since
    #: S11c-1 a unit that lowered and needs ``ir.data`` has none (S11a gave it an evaluation refusal).
    unsupported: OracleError | None = None
    line: int = 0
    #: The sorted levels of the body and of its call closure (R9); ``()`` when not lowered.
    requires: tuple = ()

    @property
    def lowered(self) -> bool:
        """Did the model lower (its IR exists), whether or not it can be evaluated?"""
        return self.ir is not None


@dataclass
class OpSig:
    name: str
    statics: tuple
    params: list  # [(name, Ty)] with refinements kept separately
    ret: A.Ty
    fresh: bool
    refinements: list = field(default_factory=list)  # [(param name, ast expr)]
    #: The refinements lowered into one Bool IR (``and`` chain, source order), or ``None``.
    admit: object | None = None
    unsupported: OracleError | None = None
    #: The levels the op's ``admit`` needs (R9); ``ir.core`` at least (section 6).
    requires: tuple = ("ir.core",)
    line: int = 0
    #: False when a *lowering* refusal (not the evaluation refusal of ``ir.data``) was recorded.
    lowered: bool = True

    def corpus_row_v2(self) -> dict:
        """The ``ops.<name>`` row of a v2 corpus (VER04r section 6)."""
        row = self.corpus_row()
        row["requires"] = list(self.requires)
        if self.admit is not None:
            row["admit"] = self.admit
        return row

    def corpus_row(self) -> dict:
        row: dict = {"params": [[n, str(t)] for n, t in self.params], "ret": str(self.ret)}
        if self.statics:
            row["width_params"] = list(self.statics)
        if self.fresh:
            row["fresh"] = True
        return row


@dataclass
class LoweredSuite:
    claim: str
    id: str
    binders: list  # [(name, Ty)]
    admit: object  # IR
    body: object  # IR, a Bool
    kind: str  # "reference" | "predicate"
    subject: object | None  # IR of the subject side (reference)
    expected: object | None  # IR of the pure side (reference)
    result: A.Ty  # type of the subject side (reference) or Bool
    ops: list  # subject_op names in order of first occurrence
    unsupported: OracleError | None = None
    line: int = 0
    #: The levels of ``admit`` and ``body`` and of their model closure (R9).
    requires: tuple = ()
    #: A lowering-time ``W0701`` (VER04r 4.3, 4.4): a subject call under an impure guard, or a
    #: claim that calls no operation.  ``None`` for a claim the generator's lowering accepts.
    refusal: OracleError | None = None
    #: Did the claim lower?  False when a *lowering* refusal was recorded in ``unsupported``; true for a
    #: claim that lowered (every one the oracle can type is one it evaluates since S11c-1).
    lowered: bool = True


@dataclass
class LoweredExample:
    name: str
    body: object  # IR of the whole Bool body
    subject: object | None
    expected: object | None
    result: A.Ty | None
    unsupported: OracleError | None = None
    line: int = 0
    requires: tuple = ()
    #: False when a *lowering* refusal was recorded (see :attr:`LoweredSuite.lowered`).
    lowered: bool = True


class Spec:
    """The checked, lowered document."""

    def __init__(self, doc: A.Document) -> None:
        self.doc = doc
        self.name = doc.name
        self.version = doc.version
        self.errors: dict[str, tuple[str, ...]] = {}
        self.records: dict[str, tuple] = {}
        self.variants: dict[str, tuple] = {}
        self.aliases: dict[str, A.Ty] = {}
        self.ops: dict[str, OpSig] = {}
        self.models: dict[str, LoweredModel] = {}
        self.suites: list[LoweredSuite] = []
        self.examples: list[LoweredExample] = []
        self.input_rejects: list = []
        self.ensures: dict[str, tuple] = {}
        self.model_names: set[str] = set()
        #: Constructor name -> (type name, index): a record's one constructor is named like the record
        #: and a variant's are its own (one namespace, VER04j 4.2).  Filled by :func:`_declare`.
        self.ctors: dict[str, tuple[str, int]] = {}
        #: Reserved levels named in a type declaration or an operation signature, ``(line, error)``: these
        #: units have no row to be recorded on, so :func:`lower_document` raises the earliest one *unless a
        #: unit declared before it recorded a refusal* (section 16.14 R35 (5)).
        self.declaration_refusals: list[tuple[int, OracleError]] = []
        #: The operations whose *signature* is refused (a reserved level in a parameter or result type), by
        #: name: they have no :class:`OpSig`, but a unit that calls one is refused with the same level, not
        #: with ``unknown callable`` (R35 (2); a call is a use of the list-typed signature).
        self.refused_ops: dict[str, OracleError] = {}

    @cached_property
    def adts(self) -> dict[str, tuple]:
        """Every record and variant as ``name -> ((constructor, (field type, ..)), ..)`` (patterns need it).

        Computed once, on first use, which is after :func:`_declare` has resolved every declaration: nothing
        reads it while the declarations are still being resolved.
        """
        out: dict[str, tuple] = {}
        for name, fields in self.records.items():
            out[name] = ((name, tuple(t for _, t in fields)),)
        for name, ctors in self.variants.items():
            out[name] = tuple((c, tuple(t for _, t in fs)) for c, fs in ctors)
        return out

    @cached_property
    def _pattern_types(self) -> P.Types:
        return P.Types(self.errors, self.adts)

    def pattern_types(self) -> P.Types:
        """The declared types a pattern is checked against (:class:`.patterns.Types`)."""
        return self._pattern_types


# -- the checker/lowerer ---------------------------------------------------------


def lower_document(doc: A.Document) -> Spec:
    """Check ``doc`` and lower every model, claim and example.

    The declaration limits of section 2.2 come first (R35 (1)); then the declarations (types, operation
    signatures); then every model, every operation refinement, every claim and example.  A ``level``
    refusal of a unit is *recorded* on it (the census and the dumps read it); every other error is
    raised, but never over a refusal an earlier-declared unit already holds (:func:`_in_order`).  A reserved
    level named in a type declaration or a signature has no row to be recorded on and refuses the whole
    document: it is raised at the end, unless an earlier unit recorded a refusal (R35 (5)).
    """
    spec = Spec(doc)
    _check_limits(doc)
    _declare(spec)
    low = _Lowerer(spec)
    for decl in doc.decls:
        if isinstance(decl, A.ModelDecl):
            _in_order(spec, decl.line, low.lower_model, decl)
    for op in spec.ops.values():
        _in_order(spec, op.line, low.lower_op_admit, op)
    for decl in doc.decls:
        if isinstance(decl, A.ClaimDecl):
            spec.suites.append(_in_order(spec, decl.line, low.lower_claim, decl))
        elif isinstance(decl, A.ExampleDecl):
            spec.examples.append(_in_order(spec, decl.line, low.lower_example, decl))
        elif isinstance(decl, A.InputDecl):
            spec.input_rejects.append(decl)
        elif isinstance(decl, A.EnsuresDecl):
            spec.ensures[decl.op] = decl.rows
    if spec.declaration_refusals:
        raise _earliest(spec, None)
    return spec


def _refusals(spec: Spec) -> list[tuple[int, OracleError]]:
    """Every refusal held so far, ``(line, error)``: the recorded ones of lowered-away units, then the declarations'."""
    found: list[tuple[int, OracleError]] = []
    for model in spec.models.values():
        if not model.lowered and model.unsupported is not None:
            found.append((model.line, model.unsupported))
    for op in spec.ops.values():
        if not op.lowered and op.unsupported is not None:
            found.append((op.line, op.unsupported))
    for unit in [*spec.suites, *spec.examples]:
        if not unit.lowered and unit.unsupported is not None:
            found.append((unit.line, unit.unsupported))
    return [*found, *spec.declaration_refusals]


def _earliest(spec: Spec, before: int | None) -> OracleError | None:
    """The refusal of the earliest-declared unit, or ``None`` if none is declared before line ``before``."""
    held = _refusals(spec)
    if not held:
        return None
    line, exc = min(held, key=lambda pair: pair[0])
    return exc if before is None or line < before else None


def lowering_refusal(spec: Spec) -> OracleError | None:
    """The first refusal a unit *recorded while lowering*, in declaration order, or ``None``.

    A refusal is a ``level`` error (``ir.list``, ``ir.seq``, ``err-value``, ``fixed-cast``, ``ir.fixed``
    in a v0 document ...): the unit did not lower.  The evaluation refusal of a unit that lowered and needs
    ``ir.data`` is not one (:attr:`LoweredModel.lowered`).  Units are ordered by their source line, which
    is declaration order (R35 (5)).
    """
    return _earliest(spec, None)


def _in_order(spec: Spec, line: int, lower, *args):
    """Run one unit's lowering; an error it raises yields to a refusal an earlier unit holds (R35 (5))."""
    try:
        return lower(*args)
    except OracleError as exc:
        if exc.code == "level":
            raise
        earlier = _earliest(spec, line)
        if earlier is not None:
            raise earlier from exc
        raise


def _check_limits(doc: A.Document) -> None:
    """The declaration limits of section 2.2 rule 3 (the corpus limits, R34), in declaration order.

    An error type has 1 to 64 cases, a record 1 to 32 fields, a variant at most 64 constructors and a
    constructor at most 8 fields.  A document over one is refused whole with ``limit``, before any
    expression is lowered; raising a limit is an erratum to section 2.2 (VER04j allows 256 and 32).
    """
    for d in doc.decls:
        if isinstance(d, A.ErrorDecl) and len(d.cases) > 64:
            raise OracleError("limit", "an error type has at most 64 cases")
        if isinstance(d, A.RecordDecl) and len(d.fields) > 32:
            raise OracleError("limit", "a record has at most 32 fields")
        if isinstance(d, A.VariantDecl):
            if len(d.ctors) > 64:
                raise OracleError("limit", "a variant has at most 64 constructors")
            if any(len(fields) > 8 for _name, fields in d.ctors):
                raise OracleError("limit", "a constructor has at most 8 fields")


#: The names a v1 document may **not declare** (the checker's ``is_reserved_builtin``: ``V0_BUILTINS``,
#: ``V1_TYPES`` and ``V1_FUNCTIONS``, W0201).  :data:`_RESERVED_V1` is wider: it also holds the lowerer's own level
#: names (``slice``, ``count``, ``fold`` ...), ``take``/``drop`` and ``imin``/``imax``, which the checker lets a
#: document declare, so the declaration rule drops them to refuse exactly what the checker refuses (a bind keeps the
#: wider set, CV2-F8).
_DECLARED_RESERVED_V1 = _RESERVED_V1 - _LEVEL_LIST - _LEVEL_SEQ - frozenset(["take", "drop", "imin", "imax"])


def _declare(spec: Spec) -> None:
    """First pass: names, uniqueness, type declarations and signatures (no bodies yet)."""
    seen: set[str] = set()
    reserved = _DECLARED_RESERVED_V1 if spec.version == "v1" else _RESERVED_V0

    def claim_name(name: str) -> None:
        """Claim a module name: not a reserved built-in (the checker's ``W0201``), not declared before.

        Every declared module name is checked (a model, an op, an error type, an alias, a record, a variant, a
        constructor; VER04r 2.2, 16.16 R58), version scoped like the bind names (:meth:`_Lowerer._check_binds`):
        a v0 document may name a model ``take``, which only a v1 document reserves.
        """
        if name in reserved:
            raise OracleError("type", "a declared name is a reserved built-in")
        if name in seen:
            raise OracleError("type", "a name is declared twice")
        seen.add(name)

    test_names: set[str] = set()
    for d in spec.doc.decls:
        if isinstance(d, A.ErrorDecl):
            claim_name(d.name)
            if len(set(d.cases)) != len(d.cases):
                raise OracleError("type", "duplicate error case")
            spec.errors[d.name] = d.cases
        elif isinstance(d, A.RecordDecl):
            claim_name(d.name)
            spec.records[d.name] = d.fields  # resolved below, once every name is known
            spec.ctors[d.name] = (d.name, 0)
        elif isinstance(d, A.VariantDecl):
            claim_name(d.name)
            spec.variants[d.name] = d.ctors
            for index, (ctor, _fields) in enumerate(d.ctors):
                claim_name(ctor)  # constructors share the one namespace (VER04j 4.2)
                spec.ctors[ctor] = (d.name, index)
        elif isinstance(d, A.AliasDecl):
            claim_name(d.name)
            spec.aliases[d.name] = d.ty
        elif isinstance(d, A.ModelDecl):
            claim_name(d.name)
            spec.model_names.add(d.name)
        elif isinstance(d, A.OpDecl):
            claim_name(d.name)
        elif isinstance(d, (A.ExampleDecl, A.ClaimDecl)):
            if d.name in test_names:
                raise OracleError("type", "an example or claim name is declared twice")
            test_names.add(d.name)
    # Type declarations, in declaration order: each is a unit (R35 (5)), so a reserved level named in it
    # is raised (it has no row to be recorded on) in preference to a type error of the same unit.
    for d in spec.doc.decls:
        if isinstance(d, (A.AliasDecl, A.RecordDecl, A.VariantDecl)):
            _in_order(spec, d.line, _declare_type, spec, d)
    _check_cycles(spec)
    # Operation signatures, in declaration order, each a unit like a type declaration (R35 (5), section 16.16 R53):
    # a typing error in one (a duplicate parameter, an unknown type name) yields to a level or feature refusal
    # an earlier-declared unit holds.  The name pass and the recursion check above belong to no unit and
    # yield to nothing.
    for d in spec.doc.decls:
        if isinstance(d, A.OpDecl):
            _in_order(spec, d.line, _declare_op, spec, d)


def _declare_op(spec: Spec, d: A.OpDecl) -> None:
    """Resolve one operation's signature into :class:`OpSig`, or record the refusal its types bring."""
    params = []
    refinements = []
    pnames: set[str] = set()
    low = _Lowerer(spec)
    refused = low.scan_signature(d)
    if refused is not None:
        spec.declaration_refusals.append((d.line, refused))
        spec.refused_ops[d.name] = refused
        return
    for p in d.params:
        if p.name in pnames:
            raise OracleError("type", "duplicate parameter name")
        pnames.add(p.name)
        params.append((p.name, _resolve(spec, p.ty, set(d.statics))))
        if p.refinement is not None:
            refinements.append((p.name, p.refinement))
    spec.ops[d.name] = OpSig(
        d.name,
        d.statics,
        params,
        _resolve(spec, d.ret, set(d.statics)),
        d.fresh,
        refinements,
        line=d.line,
    )


def _declare_type(spec: Spec, d: A.AliasDecl | A.RecordDecl | A.VariantDecl) -> None:
    """Resolve the types a type declaration names (a field, a constructor field, an alias target).

    A list is the reserved ``ir.list`` and wins over any type error of the declaration (R35 (2)); a
    record needs distinct field names; a variant has at least two constructors (a one-constructor sum is
    a record, section 2.2 rule 3) and a constructor's fields are distinct.
    """
    refused = _Lowerer(spec).scan_type_decl(d)
    if refused is not None:
        spec.declaration_refusals.append((d.line, refused))
        return
    if isinstance(d, A.AliasDecl):
        return  # resolved where it is used
    if isinstance(d, A.RecordDecl):
        if len({n for n, _ in d.fields}) != len(d.fields):
            raise OracleError("type", "duplicate field name")
        spec.records[d.name] = tuple((n, _resolve(spec, t)) for n, t in d.fields)
        return
    if len(d.ctors) < 2:
        raise OracleError("type", "a variant has at least two constructors (a one-constructor sum is a record)")
    resolved = []
    for ctor, fields in d.ctors:
        if len({n for n, _ in fields}) != len(fields):
            raise OracleError("type", "duplicate field name")
        resolved.append((ctor, tuple((n, _resolve(spec, t)) for n, t in fields)))
    spec.variants[d.name] = tuple(resolved)


def _check_cycles(spec: Spec) -> None:
    """A record or variant may not contain itself, directly or through any chain (section 2.2 rule 1)."""
    edges: dict[str, set[str]] = {}

    def mentions(ty: A.Ty) -> set[str]:
        out: set[str] = set()
        stack = [ty]
        while stack:
            t = stack.pop()
            if isinstance(t, A.Ty):
                if t.head in spec.records or t.head in spec.variants:
                    out.add(t.head)
                stack.extend(t.args)
        return out

    for name, fields in spec.records.items():
        edges[name] = set().union(*[mentions(t) for _, t in fields])
    for name, ctors in spec.variants.items():
        edges[name] = set().union(*[mentions(t) for _c, fs in ctors for _n, t in fs])
    state: dict[str, int] = {}
    for root in edges:
        if state.get(root):
            continue
        stack = [(root, iter(sorted(edges[root])))]
        state[root] = 1
        while stack:
            node, it = stack[-1]
            for nxt in it:
                if state.get(nxt) == 1:
                    raise OracleError("type", "a record or variant contains itself")
                if not state.get(nxt):
                    state[nxt] = 1
                    stack.append((nxt, iter(sorted(edges[nxt]))))
                    break
            else:
                state[node] = 2
                stack.pop()


def _resolve(spec: Spec, ty: A.Ty, statics: set[str] | None = None, depth: int = 0) -> A.Ty:
    """Resolve aliases and validate that every named type exists."""
    if depth > 16:
        raise OracleError("limit", "type nesting or alias chain exceeds 16 levels")
    head = ty.head
    if head == "()":
        return A.Ty("()", tuple(_resolve(spec, t, statics, depth + 1) for t in ty.args))
    if head in spec.aliases:
        if ty.args:
            raise OracleError("type", "an alias takes no arguments")
        return _resolve(spec, spec.aliases[head], statics, depth + 1)
    if head == "Bytes":
        if not ty.args:
            return ty
        width = ty.args[0]
        if isinstance(width, str) and (statics is None or width not in statics):
            raise OracleError("type", "an unknown static width name")
        if isinstance(width, int) and width > 4096:
            raise OracleError("type", "Bytes width exceeds 4096")
        return ty
    if head in ("List", "Option"):
        if len(ty.args) != 1 or not isinstance(ty.args[0], A.Ty):
            raise OracleError("type", f"{head} takes one type argument")
        return A.Ty(head, (_resolve(spec, ty.args[0], statics, depth + 1),))
    if head == "Result":
        if len(ty.args) != 2 or ty.args[1].head not in spec.errors or ty.args[1].args:
            raise OracleError("type", "Result takes a type and a declared error type")
        return A.Ty("Result", (_resolve(spec, ty.args[0], statics, depth + 1), ty.args[1]))
    if head in _PRIMITIVES:
        if ty.args:
            raise OracleError("type", f"{head} takes no arguments")
        return ty
    if head in spec.errors:
        # R117 (VER04r 16.22): an error type is not a type of a value; it occurs only as the second argument of
        # ``Result`` (handled above, which does not come through here for its error name).
        raise OracleError("type", "an error type is not a value type")
    if head in spec.records or head in spec.variants:
        if ty.args:
            raise OracleError("type", "a named type takes no arguments")
        return ty
    raise OracleError("type", "an unknown type name")


def _unify(a: A.Ty, b: A.Ty) -> A.Ty | None:
    """The common type of two operand types, or ``None``: the grammar checker's ``Type::common_type``.

    ``Nat`` widens to ``Int`` and ``Bytes<N>`` forgets its width to ``Bytes``; two ``Result`` types of
    one error type unify by their value type, and **two ``Option`` types unify by their payload** exactly
    as ``Result`` does ([D60]: ``Option<Nat>`` and ``Option<Int>`` have the common type ``Option<Int>``).
    Nothing else is implicit: a tuple, a record, a variant and a list never lift (they are equal or have
    no common type).
    """
    if a == b:
        return a
    if {a.head, b.head} == {"Nat", "Int"}:
        return INT
    if a.head == "Bytes" and b.head == "Bytes":
        return BYTES
    if a.head == b.head == "Option":
        inner = _unify(a.args[0], b.args[0])
        return A.Ty("Option", (inner,)) if inner is not None else None
    if a.head == b.head == "Result" and a.args[1] == b.args[1]:
        inner = _unify(a.args[0], b.args[0])
        return A.Ty("Result", (inner, a.args[1])) if inner is not None else None
    return None


def _assignable(src: A.Ty, dst: A.Ty) -> bool:
    """Can a value of ``src`` flow to ``dst``?  The grammar checker's ``Type::is_assignable_to``, line for line.

    ``Nat`` to ``Int``, ``Bytes<N>`` to ``Bytes``, and through the two payload-carrying built-ins:
    ``Option<S>`` to ``Option<T>`` when ``S`` flows to ``T``, and ``Result<S,E>`` to ``Result<T,E>`` when
    ``S`` flows to ``T`` ([D60]).  A tuple, a record, a variant and a list are equal or not assignable: no
    component lifts (``(Nat,Nat)`` is not ``(Int,Int)``, as in the checker).
    """
    if src == dst:
        return True
    if src.head == "Nat" and dst.head == "Int":
        return True
    if src.head == "Bytes" and dst.head == "Bytes":
        return not dst.args or src.args == dst.args
    if src.head == dst.head == "Option":
        return _assignable(src.args[0], dst.args[0])
    if src.head == dst.head == "Result" and src.args[1] == dst.args[1]:
        return _assignable(src.args[0], dst.args[0])
    return False


def _needs_context(e) -> bool:
    """Does ``e`` have no type of its own (so its sibling must be typed first)?"""
    if isinstance(e, A.IntLit):
        return e.suffix is None
    if isinstance(e, A.Call) and len(e.name) == 1 and e.name[0] in ("Ok", "Err"):
        return True
    # ``None`` is *not* here: it has no type of its own and takes none from a sibling either (R39 (b), as
    # the grammar checker's ``typecheck_binary`` gives only the *right* operand the left one's type as its
    # context).  A ``None`` whose own expected type is not an ``Option`` is a type error.
    if isinstance(e, A.Call) and e.name == ("Some",) and e.args is not None and len(e.args) == 1:
        return _needs_context(e.args[0])
    if isinstance(e, A.Unary) and e.op == "neg":
        return _needs_context(e.operand)
    if isinstance(e, A.If):
        return _needs_context(e.then) and _needs_context(e.other)
    if isinstance(e, A.Binary) and e.op in ("+", "-", "*"):
        return _needs_context(e.left) and _needs_context(e.right)
    if isinstance(e, A.TupleLit):
        # A tuple takes the type of its components; one whose component has no type of its own
        # (a bare literal, ``Ok(..)``) is typed from its sibling or its context.
        return any(_needs_context(item) for item in e.items)
    if isinstance(e, A.Match):
        return all(_needs_context(body) for _, body in e.arms)
    return False


def _lit_value(value: int, ty: str) -> list:
    return ["lit", {"type": ty, "value": str(value)}]


class _Lowerer:
    def __init__(self, spec: Spec) -> None:
        self.spec = spec
        self.scope: dict[str, A.Ty] = {}
        self.order: list[str] = []  # models lowered so far, for "declared before"
        self._nest = 0
        #: The levels the *types* met by the unit being lowered bring (binder, parameter,
        #: result, type of a sub-expression): R9, ``ir.tuple`` for a tuple, ``ir.data`` for a
        #: list, record or variant (section 4.1).  Added to its ``requires``.
        self._type_levels: set = set()
        self._data_names = frozenset([*spec.records, *spec.variants])
        #: Fixed-width ``add``/``sub``/``mul``/``neg`` nodes lowered so far, ``id(ir) -> ir`` (the
        #: node is kept so its id is never reused), and the models whose call closure holds one.
        #: Both feed the v0 guard, VER04r 3.3 rule 3 (:meth:`_v0_rule_3`).
        self._fixed_arith: dict = {}
        self._wrapping_models: set = set()
        #: The levels of every ``match`` lowered so far, ``id(node) -> frozenset`` (the node is kept in
        #: ``_match_nodes`` so its id is never reused): ``ir.tuple`` for a ``Result`` scrutinee,
        #: ``ir.data`` for any other, and ``ir.data`` as well when a data pattern appears (A.2).  The IR
        #: alone does not carry the scrutinee's type, so :meth:`seal` hands this to :func:`.levels.ir_levels`.
        self._match_levels: dict = {}
        self._match_nodes: list = []

    # -- helpers ---------------------------------------------------------------

    def err(self, message: str) -> OracleError:
        return OracleError("type", message)

    def level(self, level: str, message: str) -> OracleError:
        return OracleError("level", message, level=level)

    def seal(self, *irs: object) -> tuple[tuple, OracleError | None]:
        """The ``requires`` of a fully lowered unit and its evaluation refusal (always ``None`` since S11c-1).

        ``requires`` is :func:`.levels.ir_levels` over ``irs`` and the call closure of every
        ``model_call`` (R9), plus the levels of the types the unit met (:attr:`_type_levels`), plus the
        levels of each ``match`` (:attr:`_match_levels`).  The second result is the **evaluation** refusal,
        kept in the signature for its five callers: it is ``None`` for every unit that lowered, because the
        oracle evaluates every level it can lower (``ir.core``, ``ir.fixed``, ``ir.tuple`` and, from slice
        S11c-1, ``ir.data``; S11a returned a ``level ir.data`` refusal here for the last).
        """
        # ``ir.core`` is always in a row's requires (VER04r section 6: "at least").  The call
        # closure is read from each callee's own ``requires`` (already closed over its callees,
        # in declaration order) instead of walking it again for every unit.
        found = {L.CORE, *L.ir_levels(list(irs), follow=False, match_levels=self._match_levels)}
        for tree in irs:
            for node in walk(tree):
                if node[0] == "model_call":
                    found.update(self.spec.models[node[1]].requires)
        found |= self._type_levels
        return tuple(sorted(found)), None

    def check_type(self, ty: A.Ty) -> None:
        """A type of a signature or binder: a list is the reserved ``ir.list`` (R27), the rest is counted (R9)."""
        if L.type_refusal(ty) is not None:
            raise self.level(L.LIST, "a list type needs the reserved ir.list level")
        self._type_levels |= L.type_levels(ty, self._data_names)

    # -- refusal order (section 16.14 R35) -------------------------------------

    def scan_type_decl(self, d) -> OracleError | None:
        """The first reserved level named in a record, variant or alias declaration, or ``None``."""
        return self._scan(d)

    def scan_signature(self, d: A.OpDecl) -> OracleError | None:
        """The first reserved level named in an operation's parameter and result types, or ``None``."""
        for p in d.params:
            found = self._scan(p.ty)
            if found is not None:
                return found
        return self._scan(d.ret)

    def _prefer_feature(self, exc: OracleError, unit) -> OracleError:
        """R35 (2): a level or feature refusal in a unit is named in preference to a ``type`` error of it."""
        if exc.code == "type":
            found = self._scan(unit)
            if found is not None:
                return found
        return exc

    def _scan(self, node: object, errvars: frozenset = frozenset()) -> OracleError | None:
        """The first reserved level or feature name in a unit's *source*, in pre-order (left to right).

        Consulted when a unit raises a ``type`` error, and for type declarations and signatures, so that
        ``ir.list``, ``ir.seq``, ``fixed-cast`` and ``err-value`` win over a type error of the same unit
        (R35 (2)).  ``errvars`` are the names an enclosing ``Err(e)`` pattern bound: using one is the
        error value of [D55].
        """
        spec = self.spec
        if isinstance(node, A.Ty):
            if node.head == "List":
                return self.level(L.LIST, "a list type needs the reserved ir.list level")
            return self._scan(node.args, errvars)
        if isinstance(node, A.ListLit):
            return self.level(L.LIST, "a list literal needs the reserved ir.list level")
        if isinstance(node, A.Name):
            if node.parts[0] in errvars:
                return self.level(ERR_VALUE, "an error value bound by a pattern has no IR form")
            return None
        if isinstance(node, A.Call):
            head = node.name[0]
            if head in errvars:
                return self.level(ERR_VALUE, "an error value bound by a pattern has no IR form")
            user = head in spec.ops or head in spec.model_names or head in spec.ctors
            if len(node.name) == 1 and not user:
                if head in _LEVEL_LIST:
                    return self.level(L.LIST, "a list built-in needs the reserved ir.list level")
                if head in _LEVEL_SEQ:
                    return self.level(L.SEQ, "sequence built-ins need the reserved ir.seq level")
                if head == _FIXED_CAST:
                    return self.level(FIXED_CAST, "fixed<N>(b) has no IR form")
            return self._scan(node.args, errvars)
        if isinstance(node, A.Match):
            found = self._scan(node.scrutinee, errvars)
            if found is not None:
                return found
            for pattern, body in node.arms:
                found = self._scan(body, errvars | _err_binds(pattern))
                if found is not None:
                    return found
            return None
        if isinstance(node, A.Pattern):
            return None
        if isinstance(node, tuple | list):
            for item in node:
                found = self._scan(item, errvars)
                if found is not None:
                    return found
            return None
        if hasattr(node, "__slots__") and not isinstance(node, str):
            for name in node.__slots__:
                found = self._scan(getattr(node, name, None), errvars)
                if found is not None:
                    return found
        return None

    # -- models ----------------------------------------------------------------

    def lower_model(self, decl: A.ModelDecl) -> None:
        spec = self.spec
        params = []
        names: set[str] = set()
        for p in decl.params:
            if p.name in names:
                raise self.err("duplicate parameter name")
            if p.refinement is not None or p.inout:
                raise self.err("a model parameter has no refinement")
            names.add(p.name)
            params.append((p.name, _resolve(spec, p.ty)))
        ret = _resolve(spec, decl.ret)
        model = LoweredModel(decl.name, params, ret, line=decl.line)
        self.scope = dict(params)
        self._type_levels = set()
        try:
            for _, pty in params:
                self.check_type(pty)
            self.check_type(ret)
            ir, ty = self.expr(decl.body, ret)
            if not _assignable(ty, ret):
                raise self.err("a model body does not have its declared type")
            if _has_subject(ir):
                # VER04r 4.2 rule 3, the generator's lowering refusal: a model is pure, so it
                # may not call the subject, whether or not anything calls the model.
                raise OracleError(
                    "generation",
                    f"W0701: a subject call inside the body of model `{decl.name}` (VER04r 4.2 rule 3)",
                )
            model.ir = ir
            if any(
                id(n) in self._fixed_arith or (n[0] == "model_call" and n[1] in self._wrapping_models)
                for n in walk(ir)
            ):
                self._wrapping_models.add(decl.name)
            model.requires, model.unsupported = self.seal(ir)
        except OracleError as exc:
            exc = self._prefer_feature(exc, decl)
            if exc.code != "level":
                raise exc
            model.ir = None
            model.unsupported = exc
        spec.models[decl.name] = model
        self.order.append(decl.name)

    def lower_op_admit(self, op: OpSig) -> None:
        """Lower an operation's parameter refinements into ``op.admit`` (VER04r section 6).

        One Bool IR over the parameters, ``and``-chained in source order exactly as a
        claim's binder refinements are (section 4.5).  A refinement above ``ir.core``
        is recorded in ``op.unsupported``, never skipped.

        An ``admit`` is pure: it may not call a subject op (section 4.2 rule 3).  This is
        checked for **every** op, whether or not a claim or example calls it, so the oracle
        refuses exactly the documents the generator's front end refuses (its ``W0301``); the
        refusal is the one the oracle gives when such an op *is* called, ``ir: a subject call
        is forbidden here`` (section 16.9 follow-up).  It is made on the source tree, so a
        subject call next to a construct of a higher level is still found.
        """
        for _, refinement in op.refinements:
            if _calls_op(self.spec, refinement):
                raise OracleError("ir", "a subject call is forbidden here")
        self._type_levels = set()
        self.scope = dict(op.params)
        parts: list = []
        try:
            for _, pty in op.params:
                self.check_type(pty)
            self.check_type(op.ret)
            for _, refinement in op.refinements:
                ir, ty = self.expr(refinement, BOOL)
                if ty != BOOL:
                    raise self.err("a refinement must be Bool")
                parts.append(ir)
        except OracleError as exc:
            exc = self._prefer_feature(exc, tuple(r for _, r in op.refinements))
            if exc.code != "level":
                raise exc
            op.unsupported = exc
            op.lowered = False
            return
        if parts:
            op.admit = parts[0]
            for extra in parts[1:]:
                op.admit = ["and", op.admit, extra]
            op.requires, op.unsupported = self.seal(op.admit)
        else:
            # No admit, but the signature itself may need ir.tuple / ir.data (R9).
            op.requires = tuple(sorted({L.CORE, *self._type_levels}))

    # -- claims and examples -----------------------------------------------------

    def lower_claim(self, decl: A.ClaimDecl) -> LoweredSuite:
        spec = self.spec
        suite_id = f"warrant/{spec.name}/exhaustive/{decl.name}"
        self._type_levels = set()
        binders: list = []
        refinements: list = []
        body = decl.body
        self.scope = {}
        unsupported: OracleError | None = None
        admit: object = ["lit", True]
        ir: object = ["lit", True]
        lowered = False
        refused_at_lowering = False
        requires: tuple = ()
        try:
            if isinstance(body, A.Quant):
                if body.kind != "forall":
                    raise self.err("a claim quantifier must be `forall`")
                for b in body.binders:
                    if b.name in self.scope:
                        raise self.err("duplicate or shadowing binder name")
                    ty = _resolve(spec, b.ty)
                    if b.inout:
                        raise self.err("a binder cannot be inout")
                    self.check_type(ty)
                    self.scope[b.name] = ty
                    binders.append((b.name, ty))
                    if b.refinement is not None:
                        r_ir, r_ty = self.expr(b.refinement, BOOL)
                        if r_ty != BOOL:
                            raise self.err("a refinement must be Bool")
                        refinements.append(r_ir)
                body = body.body
            admit = refinements[0] if refinements else ["lit", True]
            for extra in refinements[1:]:
                admit = ["and", admit, extra]
            ir, ty = self.expr(body, BOOL)
            if ty != BOOL:
                raise self.err("a claim body must be Bool")
            lowered = True
            requires, unsupported = self.seal(admit, ir)
        except OracleError as exc:
            exc = self._prefer_feature(exc, decl)
            if exc.code != "level":
                raise exc
            unsupported = exc
            lowered = False
            refused_at_lowering = True
        # VER04r 4.4 and 4.3, applied at lowering as the generator's lowering applies them: a
        # subject call under an impure guard, and a claim that calls no op, are W0701.  The claim
        # is still lowered (its IR is what a census and the dump show); the refusal is recorded
        # in ``refusal`` and raised by every consumer that would evaluate or emit it.  A level
        # refusal wins over it in evaluation (the generator meets a level while typing, before
        # either rule); the lowering dump, which accepts ``ir.fixed``, raises the refusal.
        refusal: OracleError | None = None
        if lowered:
            try:
                check_pure_guards(ir)
                self._v0_rule_3(ir)
                if not _has_subject(ir):
                    raise OracleError(
                        "generation",
                        "W0701: a claim must call at least one operation of the subject "
                        "(VER04r 4.3)",
                    )
            except OracleError as exc:
                refusal = exc
        kind = "predicate"
        subject = expected = None
        result = BOOL
        ops = _collect_ops(ir) if lowered else []
        if lowered and ir[0] == "eq":
            left_has = _has_subject(ir[1])
            right_has = _has_subject(ir[2])
            if left_has != right_has:
                kind = "reference"
                subject, expected = (ir[1], ir[2]) if left_has else (ir[2], ir[1])
                result = self._type_of_side(decl.body, left_has)
        return LoweredSuite(
            decl.name,
            suite_id,
            binders,
            admit,
            ir,
            kind,
            subject,
            expected,
            result,
            ops,
            unsupported,
            decl.line,
            requires,
            refusal,
            not refused_at_lowering,
        )

    def _v0_rule_3(self, ir: object) -> None:
        """VER04r 3.3 rule 3, the generator's v0 guard: refuse, in a **v0** document, a fixed-width
        ``+ - *`` or unary ``-`` with a subject call beneath it, or a ``model_call`` with a subject
        call in an argument whose callee's call closure holds such a node.  The v0 harness
        evaluates ``U8`` as ``Nat``, so a generation-time count cannot prove the two agree.
        :func:`.ir_eval.fixed_arith_over_subject` makes the same test at evaluation; this one is
        made on the types the lowerer already knows, so the refusal is a property of lowering.
        """
        if self.spec.version != "v0":
            return
        for node in walk(ir):
            if id(node) in self._fixed_arith and _has_subject(node):
                raise OracleError(
                    "generation",
                    "W0701: fixed-width arithmetic over a subject result in a v0 document "
                    "(VER04r 3.3 rule 3)",
                )
            if (
                node[0] == "model_call"
                and node[1] in self._wrapping_models
                and any(_has_subject(a) for a in node[2])
            ):
                raise OracleError(
                    "generation",
                    f"W0701: model `{node[1]}` (fixed-width arithmetic in its call closure) over a "
                    "subject result in a v0 document (VER04r 3.3 rule 3)",
                )

    def _type_of_side(self, body, subject_left: bool) -> A.Ty:
        """Type of the subject side of a reference claim (re-derived by checking)."""
        node = body.body if isinstance(body, A.Quant) else body
        side = node.left if subject_left else node.right
        saved = dict(self.scope)
        try:
            return self.expr(side, None)[1]
        finally:
            self.scope = saved

    def lower_example(self, decl: A.ExampleDecl) -> LoweredExample:
        self.scope = {}
        self._type_levels = set()
        try:
            ir, ty = self.expr(decl.body, BOOL)
            if ty != BOOL:
                raise self.err("an example body must be Bool")
            requires, unsupported = self.seal(ir)
        except OracleError as exc:
            exc = self._prefer_feature(exc, decl)
            if exc.code != "level":
                raise exc
            return LoweredExample(
                decl.name, ["lit", True], None, None, None, exc, decl.line, (), False
            )
        subject = expected = None
        result = None
        if ir[0] == "eq" and _has_subject(ir[1]) != _has_subject(ir[2]):
            left_has = _has_subject(ir[1])
            subject, expected = (ir[1], ir[2]) if left_has else (ir[2], ir[1])
            node = decl.body
            result = self.expr(node.left if left_has else node.right, None)[1]
        return LoweredExample(
            decl.name, ir, subject, expected, result, unsupported, decl.line, requires
        )

    # -- expressions -------------------------------------------------------------

    def expr(
        self, e, expected: A.Ty | None, *, intermediate: bool = False
    ) -> tuple[object, A.Ty]:
        """Lower ``e``; ``expected`` types bare literals, ``Ok``/``Err``, ``Some`` payloads and ``None``.

        Nesting is capped at 64 (VER04 section 2: "64 nested expression nodes on any
        path"): a left-associative chain of 2,000 additions is a path of depth 2,000.

        ``intermediate`` is true for exactly one caller: ``unwrap_or``, for its first operand.  A node of
        one of the nine producers there is the **intermediate use** of section 16.10 D23 as amended by
        [D48]: its ``Option`` type adds no ``ir.data`` (section 16.14 R29 (b), (d)).  Every other
        ``Option``-typed node, wherever it is, is typed like any other and brings ``ir.data`` by the
        type rule (:func:`.levels.type_levels`).
        """
        self._nest += 1
        try:
            if self._nest > MAX_EXPR_DEPTH:
                raise OracleError("limit", "an expression nests deeper than 64 nodes")
            ir, ty = self._expr(e, expected)
            if not (intermediate and ty.head == "Option" and ir[0] in L.OPTION_PRODUCERS):
                self._type_levels |= L.type_levels(ty, self._data_names)
            return ir, ty
        except RecursionError as exc:
            raise OracleError("limit", "an expression nests too deeply") from exc
        finally:
            self._nest -= 1

    def _expr(self, e, expected: A.Ty | None) -> tuple[object, A.Ty]:
        spec = self.spec
        if isinstance(e, A.BoolLit):
            return ["lit", e.value], BOOL
        if isinstance(e, A.IntLit):
            return self.int_literal(e, expected)
        if isinstance(e, A.BytesLit):
            # The *type* of a bytes literal is ``Bytes<len>`` (the checker's), so the width is carried by the
            # type through ``Some``, ``Ok``, ``if`` and ``match`` (R51); the IR node keeps the written type
            # ``Bytes`` (no ``result_type`` moves).
            node = ["lit", {"type": "Bytes", "value": e.hex}]
            return node, A.Ty("Bytes", (len(e.hex) // 2,))
        if isinstance(e, A.Name):
            if len(e.parts) == 1:
                name = e.parts[0]
                if name in self.scope:
                    return ["var", name], self.scope[name]
                if name == "None":
                    return self.none_literal(expected)
                if name in spec.ctors:
                    return self.constructor(name, None)
                raise self.err("an unbound name")
            head = e.parts[0]
            if head in self.scope:
                return self.project(["var", head], self.scope[head], e.parts[1:])
            raise self.err("a qualified name is only valid inside Err(...)")
        if isinstance(e, A.Call):
            return self.call(e, expected)
        if isinstance(e, A.Unary):
            return self.unary(e, expected)
        if isinstance(e, A.Binary):
            return self.binary(e, expected)
        if isinstance(e, A.If):
            c_ir, c_ty = self.expr(e.cond, BOOL)
            if c_ty != BOOL:
                raise self.err("an `if` condition must be Bool")
            if _needs_context(e.then) and not _needs_context(e.other):
                o_ir, o_ty = self.expr(e.other, expected)
                t_ir, t_ty = self.expr(e.then, o_ty if expected is None else expected)
            else:
                t_ir, t_ty = self.expr(e.then, expected)
                # The other branch takes the type of the first only if it has none of its own (a bare literal,
                # ``Ok(..)``): a ``None`` takes its type from the *context* alone, never from a sibling
                # (R39 (b); the checker types both branches against the same ``expected``).
                sibling = t_ty if _needs_context(e.other) else None
                o_ir, o_ty = self.expr(e.other, expected if expected is not None else sibling)
            common = _unify(t_ty, o_ty)
            if common is None:
                raise self.err("the branches of an `if` have no common type")
            node = ["if", c_ir, t_ir, o_ir]
            return node, common
        if isinstance(e, A.TupleLit):
            return self.tuple_literal(e, expected)
        if isinstance(e, A.Proj):
            base_ir, base_ty = self.expr(e.base, None)
            return self.project(base_ir, base_ty, (e.field,))
        if isinstance(e, A.ListLit):
            raise self.level(L.LIST, "a list literal needs the reserved ir.list level")
        if isinstance(e, A.Match):
            return self.match(e, expected)
        if isinstance(e, A.Quant):
            raise self.err("a quantifier is only allowed at the head of a claim")
        raise self.err("unsupported expression")

    # -- ir.tuple: tuples, projection and match on a Result (slice S10t-1) ----------------------

    def tuple_literal(self, e: A.TupleLit, expected: A.Ty | None) -> tuple[object, A.Ty]:
        """``(a, b, ..)`` -> ``["tuple", [a, b, ..]]`` of type ``(Ta,Tb,..)`` (A.1, R2).

        A component is typed with the matching component of ``expected`` when that is a tuple of
        the same arity (so ``(low7(b0), 1)`` in a ``Result<(U64,Nat),E>`` context has a ``Nat``
        literal), and on its own otherwise.  Two to eight components: the parser has already
        refused nine, and a one-component parenthesis is not a tuple.
        """
        n = len(e.items)
        if not 2 <= n <= P.MAX_TUPLE_ARITY:
            raise self.err("a tuple has 2 to 8 components")
        hints = (
            expected.args
            if expected is not None and expected.head == "()" and len(expected.args) == n
            else (None,) * n
        )
        lowered = [self.expr(item, hint) for item, hint in zip(e.items, hints)]
        return ["tuple", [ir for ir, _ in lowered]], A.Ty("()", tuple(ty for _, ty in lowered))

    def project(self, ir: object, ty: A.Ty, parts: tuple) -> tuple[object, A.Ty]:
        """Chain ``["project", e, i]`` (a tuple) and ``["field", e, i]`` (a record) for each name in ``parts``.

        ``t.0.1`` is two ``project`` nodes, ``r.x`` one ``field`` node whose index is the position of
        ``x`` among the record's declared fields (the name stays in ``types``, Appendix A.2); both
        indices are JSON integers.  Projecting ``e.i`` on a value that is not a tuple, or ``e.name`` on a
        value that is not a record (a variant is reached through ``match``), is a **type** error from
        slice S11a (section 16.14 R39 (c)); it was the level ``ir.data`` before.
        """
        for part in parts:
            if ty.head == "()":
                index = _tuple_index(part)
                if index is None or index >= len(ty.args):
                    raise self.err("a tuple projection is a component index of a tuple")
                ir, ty = ["project", ir, index], ty.args[index]
            elif ty.head in self.spec.records:
                fields = self.spec.records[ty.head]
                names = [n for n, _ in fields]
                if part not in names:
                    raise self.err("a record has no field of that name")
                index = names.index(part)
                ir, ty = ["field", ir, index], fields[index][1]
            else:
                raise self.err("a projection needs a tuple (e.i) or a record (e.name)")
            self._type_levels |= L.type_levels(ty, self._data_names)
        return ir, ty

    # -- ir.data: Option, constructors, records (slice S11a, Appendix A.2) -------------------------

    def none_literal(self, expected: A.Ty | None) -> tuple[object, A.Ty]:
        """``None`` -> ``["none", "T"]`` of type ``Option<T>``, ``T`` the payload type the context fixes.

        A source ``None`` whose payload type its context does not fix (``None == None``,
        ``match None with ..``) is a **type** error, never a guess (section 16.14 R39 (b)): the typed node
        has nothing to write.  ``T`` is written as its canonical type name (section 2.1).
        """
        if expected is None or expected.head != "Option":
            raise self.err("None needs an Option type from its context")
        return ["none", str(expected.args[0])], expected

    def some_literal(self, e: A.Call, expected: A.Ty | None) -> tuple[object, A.Ty]:
        """``Some(e)`` -> ``["some", e]`` of type exactly ``Option<type(e)>`` (R39 (a)).

        The payload is typed with the payload type of ``expected`` when that is an ``Option``, so a bare
        literal in ``Some(1)`` takes the type its context asks for.  The node's own type is exactly
        ``Option<type(e)>``; it *flows* to a wider ``Option`` by the payload (``Option<Bytes<4>>`` to
        ``Option<Bytes>``, ``Option<Nat>`` to ``Option<Int>``) wherever an assignment is checked
        (:func:`_assignable`; [D60], the grammar checker's ``Type::is_assignable_to``).
        """
        self.arity(e, 1)
        inner = expected.args[0] if expected is not None and expected.head == "Option" else None
        ir, ty = self.expr(e.args[0], inner)
        return ["some", ir], A.Ty("Option", (ty,))

    def constructor(self, name: str, args: tuple | None) -> tuple[object, A.Ty]:
        """``Name(e1, .., en)`` and the bare name of a payload-free constructor -> ``["ctor", "Name", [..]]``.

        ``Name`` is a record (whose one constructor is named like it) or a constructor of a variant; the
        node has that type.  One operand per declared field, in order, each typed against its field and
        accepted by the grammar checker's per-argument assignability (:func:`_assignable`: ``Nat`` to ``Int``,
        ``Bytes<N>`` to ``Bytes``, through an ``Option`` or ``Result`` payload, and nothing through a tuple,
        record or variant).  A wrong operand count, a bare name with fields and
        parentheses on a payload-free constructor are type errors (the checker's ``ArityMismatch`` and
        ``NullaryWithParens``).
        """
        type_name, index = self.spec.ctors[name]
        fields = self.spec.adts[type_name][index][1]
        if args is None:
            if fields:
                raise self.err("a constructor with fields needs its arguments")
            args = ()
        elif not fields:
            raise self.err("a payload-free constructor takes no parentheses")
        if len(args) != len(fields):
            raise self.err("wrong number of constructor arguments")
        operands = []
        for arg, field_ty in zip(args, fields):
            ir, ty = self.expr(arg, field_ty)
            if not _assignable(ty, field_ty):
                raise self.err("a constructor argument does not match its field type")
            operands.append(ir)
        return ["ctor", name, operands], A.Ty(type_name)

    def match(self, e: A.Match, expected: A.Ty | None) -> tuple[object, A.Ty]:
        """``match s with p1 -> e1 | ..`` on **any** scrutinee (Appendix A.1 and A.2).

        ``match`` is ``ir.tuple`` exactly when its scrutinee is a ``Result`` and ``ir.data`` for any other
        type, and ``ir.data`` as well whenever one of its patterns is a data pattern (section 4.1, [D35];
        A.2): the levels are recorded per node (:attr:`_match_levels`) for :meth:`seal`.  The scrutinee is
        typed first, then **every pattern is checked before any arm body is lowered** (R35 (3)): inside a
        pattern the structure (an unknown constructor, a constructor or literal that does not fit the
        scrutinee's type, a wrong field count) and only then the duplicate-bind and shadowing rules
        (:meth:`_check_binds`); then the arm set (unreachable arms before fall-through, :func:`.patterns.check_arms`).
        Arm bodies are typed with the pattern's binders in scope (an arm that needs a context takes the
        first self-typed arm's type, as ``if`` does); the arms must have one type.
        """
        s_ir, s_ty = self.expr(e.scrutinee, None)
        env = self.spec.pattern_types()
        patterns: list = []
        bound: list = []
        for pattern, _ in e.arms:
            binds: list = []
            patterns.append(self.pattern(pattern, s_ty, binds))
            self._check_binds(binds)
            bound.append(binds)
        P.check_arms(patterns, s_ty, env)
        arms = e.arms
        bodies: list = [None] * len(arms)
        types: list = [None] * len(arms)
        hint = expected
        order = [i for i, (_, b) in enumerate(arms) if not _needs_context(b)]
        order += [i for i, (_, b) in enumerate(arms) if _needs_context(b)]
        for i in order:
            body = arms[i][1]
            saved, self.scope = self.scope, dict(self.scope)
            try:
                for name, ty in bound[i]:
                    if ty is not None:
                        self.scope[name] = ty
                    elif _mentions(body, name):
                        raise self.level(
                            ERR_VALUE, "an error value bound by a pattern has no IR form"
                        )
                # As in ``if``: a sibling arm's type is a hint only for an arm that has none of its own.
                arm_hint = hint if expected is not None or _needs_context(body) else None
                bodies[i], types[i] = self.expr(body, arm_hint)
            finally:
                self.scope = saved
            if hint is None:
                hint = types[i]
        common = types[0]
        for ty in types[1:]:
            common = _unify(common, ty) if common is not None else None
        if common is None:
            raise self.err("the arms of a match have no common type")
        node = ["match", s_ir, [[p, b] for p, b in zip(patterns, bodies)]]
        levels = {L.TUPLE} if s_ty.head == "Result" else {L.DATA}
        if any(L.has_data_pattern(p) for p in patterns):
            levels.add(L.DATA)
        self._match_levels[id(node)] = frozenset(levels)
        self._match_nodes.append(node)
        return node, common

    def pattern(self, p: A.Pattern, ty: A.Ty, binds: list, depth: int = 1) -> list:
        """Lower one source pattern against the static type ``ty`` to its IR pattern (A.1, A.2).

        ``Ok(p)`` is ``["ok", p]``; ``Err(E.CASE)`` is ``["err", "CASE"]``; ``Err(_)`` and ``Err(e)``
        are ``["err_any"]`` (an error value has no IR form, so ``e`` may not be *used*; it is still
        declared, for the duplicate and shadowing rules); a tuple pattern is ``["tuple", [..]]``; a
        name is ``["bind", name]`` and ``_`` is ``["wild"]``; ``Some(p)`` is ``["some", p]``, ``None`` is
        ``["none"]``, ``true``/``false`` are ``["bool", v]`` and ``Name(p1, ..)`` is ``["ctor", "Name",
        [..]]`` for a record or variant constructor (the bare name of a payload-free constructor is
        ``["ctor", "Name", []]``).  A name that is a declared constructor is *always* a constructor
        pattern, never a binder (VER04j 4.3), so a bind "named like a constructor" cannot arise.

        This checks the **structure** only and collects ``(name, type)`` for every binder in ``binds``;
        the duplicate and shadowing rules run afterwards on the whole pattern (:meth:`_check_binds`).
        """
        if depth > P.MAX_PATTERN_DEPTH:
            raise OracleError("limit", "pattern nesting exceeds 8 levels")
        spec = self.spec
        kind = p.kind
        if kind == "wild":
            return ["wild"]
        if kind == "bool":
            if ty != BOOL:
                raise self.err("a Bool pattern needs a Bool")
            return ["bool", p.value]
        if kind == "bind":
            name = p.name[0]
            if name == "None":
                if ty.head != "Option":
                    raise self.err("None needs an Option")
                return ["none"]
            if name in spec.ctors:
                return self.ctor_pattern(name, (), ty, binds, depth)
            if name in ("Ok", "Err"):
                raise self.err("Ok and Err are constructors, not names")
            binds.append((name, ty))
            return ["bind", name]
        if kind == "tuple":
            if ty.head != "()" or len(ty.args) != len(p.items):
                raise self.err("a tuple pattern does not match the type of its value")
            return [
                "tuple",
                [self.pattern(item, t, binds, depth + 1) for item, t in zip(p.items, ty.args)],
            ]
        parts = p.name
        if parts == ("Ok",):
            if ty.head != "Result" or len(p.items) != 1:
                raise self.err("Ok takes one pattern and needs a Result")
            return ["ok", self.pattern(p.items[0], ty.args[0], binds, depth + 1)]
        if parts == ("Err",):
            if ty.head != "Result" or len(p.items) != 1:
                raise self.err("Err takes one pattern and needs a Result")
            inner = p.items[0]
            if inner.kind == "wild":
                return ["err_any"]
            if inner.kind == "bind" and inner.name[0] not in spec.ctors and inner.name[0] != "None":
                if inner.name[0] in ("Ok", "Err"):
                    raise self.err("Ok and Err are constructors, not names")
                binds.append((inner.name[0], None))
                return ["err_any"]
            if inner.kind == "ctor" and len(inner.name) == 2 and not inner.items:
                err_name, case = inner.name
                if err_name != ty.args[1].head or case not in spec.errors.get(err_name, ()):
                    raise self.err("an error case of the wrong error type")
                return ["err", case]
            raise self.err("an Err pattern is an error case, a name or _")
        if parts == ("Some",):
            if ty.head != "Option" or len(p.items) != 1:
                raise self.err("Some takes one pattern and needs an Option")
            return ["some", self.pattern(p.items[0], ty.args[0], binds, depth + 1)]
        if len(parts) == 1 and parts[0] in spec.ctors:
            return self.ctor_pattern(parts[0], p.items, ty, binds, depth)
        raise self.err("an unknown constructor in a pattern")

    def ctor_pattern(
        self, name: str, items: tuple, ty: A.Ty, binds: list, depth: int
    ) -> list:
        """``Name(p1, .., pn)`` against ``ty``: the type must be the one that declares ``Name`` and ``n`` its field count."""
        type_name, index = self.spec.ctors[name]
        if ty.head != type_name:
            raise self.err("a constructor pattern does not match the type of its value")
        fields = self.spec.adts[type_name][index][1]
        if len(items) != len(fields):
            raise self.err("a constructor pattern has the wrong number of fields")
        return [
            "ctor",
            name,
            [self.pattern(item, t, binds, depth + 1) for item, t in zip(items, fields)],
        ]

    def _check_binds(self, binds: list) -> None:
        """The binders of one pattern: a duplicate or a shadowing name is a corpus error (A.1, R2)."""
        seen: set[str] = set()
        for name, _ty in binds:
            self._declare(name, seen)

    def _declare(self, name: str, seen: set) -> None:
        """Check one pattern binder name: a duplicate or a shadowing name is a corpus error (A.1, R2)."""
        if name in seen:
            raise OracleError("corpus", "the same bind name twice in one pattern")
        spec = self.spec
        # A bind spelling a model, an operation, an error type or a reserved built-in is refused
        # (CV2-F8; the checker's ``check_arm_binder``, W0203).  The checker reads a name in a body
        # by what it is declared as, so a bind called ``take`` or ``rotl`` would let the two
        # tools disagree about what the name refers to inside the arm: the checker types ``take(..)``
        # as the built-in while an evaluator would call the bound value.  Refusing the bind keeps
        # every name meaning one thing.  A constructor never reaches here: a name that is a declared
        # constructor is a constructor pattern (:meth:`pattern`).  Built-ins are version scoped: a v0
        # document may bind a name that only a v1 document reserves.
        reserved = _RESERVED_V1 if spec.version == "v1" else _RESERVED_V0
        if (
            name in spec.model_names
            or name in spec.ops
            or name in spec.errors
            or name in reserved
        ):
            raise OracleError(
                "corpus",
                "a bind name shadows a model, an operation, an error type or a reserved built-in",
            )
        if name in self.scope or name == "result":
            raise OracleError(
                "corpus", "a bind name shadows a binder, a parameter, result or an outer bind"
            )
        seen.add(name)

    def int_literal(self, e: A.IntLit, expected: A.Ty | None) -> tuple[object, A.Ty]:
        if e.suffix is None:
            ty = NAT if expected is not None and expected.head == "Nat" else INT
            return _lit_value(e.value, ty.head), ty
        ty = A.Ty(e.suffix.upper())
        bits, signed = FIXED[ty.head]
        limit = (1 << (bits - 1)) if signed else (1 << bits)
        if e.value >= limit:
            raise self.err("a suffixed literal is out of range")
        return _lit_value(e.value, ty.head), ty

    def unary(self, e: A.Unary, expected: A.Ty | None) -> tuple[object, A.Ty]:
        if e.op == "not":
            ir, ty = self.expr(e.operand, BOOL)
            if ty != BOOL:
                raise self.err("`not` needs a Bool")
            return ["not", ir], BOOL
        operand = e.operand
        if (
            isinstance(operand, A.IntLit)
            and operand.suffix is not None
            and operand.suffix.startswith("i")
            and operand.value == 1 << (FIXED[operand.suffix.upper()][0] - 1)
        ):
            # VER04j section 2.2: the one literal whose magnitude is 2^(w-1) is
            # legal only as the direct operand of unary minus.  The IR keeps both
            # nodes; the literal carries the in-range value -2^(w-1), and
            # ``neg`` of the minimum wraps to itself (VER04r section 3.2).
            ty = A.Ty(operand.suffix.upper())
            node = ["neg", _lit_value(-operand.value, ty.head)]
            self._fixed_arith[id(node)] = node
            return node, ty
        ir, ty = self.expr(
            operand, expected if expected is not None and is_numeric(expected) else None
        )
        if not is_numeric(ty):
            raise self.err("unary minus needs a number")
        if ty.head in FIXED and not FIXED[ty.head][1]:
            raise self.err("unary minus on an unsigned type; write 0uN - a")
        node = ["neg", ir]
        if is_fixed(ty):
            self._fixed_arith[id(node)] = node
        return node, (ty if is_fixed(ty) else INT)

    def binary(self, e: A.Binary, expected: A.Ty | None) -> tuple[object, A.Ty]:
        op = e.op
        if op in ("and", "or", "=>"):
            l_ir, l_ty = self.expr(e.left, BOOL)
            r_ir, r_ty = self.expr(e.right, BOOL)
            if l_ty != BOOL or r_ty != BOOL:
                raise self.err("boolean operators need Bool operands")
            return [{"and": "and", "or": "or", "=>": "implies"}[op], l_ir, r_ir], BOOL
        # Operands of comparisons and arithmetic: type the self-typed side first
        # so that a bare literal on the other side picks up its type.
        hint = expected if op in ("+", "-", "*") else None
        if _needs_context(e.left) and not _needs_context(e.right):
            r_ir, r_ty = self.expr(e.right, hint)
            l_ir, l_ty = self.expr(e.left, r_ty)
        else:
            l_ir, l_ty = self.expr(e.left, hint)
            r_ir, r_ty = self.expr(e.right, l_ty)
        common = _unify(l_ty, r_ty)
        if common is None:
            raise self.err("operands have no common type")
        if op in ("==", "!="):
            return [("eq" if op == "==" else "ne"), l_ir, r_ir], BOOL
        if not is_numeric(common):
            raise self.err("an ordering or arithmetic operator needs numbers")
        if (l_ty.head in FIXED) != (r_ty.head in FIXED):
            raise self.err("a fixed-width integer never mixes with Int or Nat")
        tag = {"<": "lt", "<=": "le", ">": "gt", ">=": "ge", "+": "add", "-": "sub", "*": "mul"}[op]
        if op in ("<", "<=", ">", ">="):
            return [tag, l_ir, r_ir], BOOL
        # Arithmetic: fixed-width stays fixed (and wraps); Nat/Int become Int.
        node = [tag, l_ir, r_ir]
        if is_fixed(common):
            self._fixed_arith[id(node)] = node
        return node, (common if is_fixed(common) else INT)

    # -- calls --------------------------------------------------------------------

    def call(self, e: A.Call, expected: A.Ty | None) -> tuple[object, A.Ty]:
        spec = self.spec
        if len(e.name) != 1:
            raise self.err("a call head is a single name")
        name = e.name[0]
        if name in spec.ops:
            return self.subject_call(spec.ops[name], e)
        if name in spec.refused_ops:
            # The signature names the reserved level (``ir.list``): a call to it needs that level too, whether
            # the unit is declared before or after the operation (a unit before it has no earlier refusal to
            # yield to, so without this it would get a spurious ``unknown callable`` type error).
            refused = spec.refused_ops[name]
            raise OracleError(refused.code, refused.message, level=refused.level)
        if name in spec.model_names:
            if name not in spec.models:
                raise OracleError(
                    "generation", "a model calls a model declared later (VER04r 4.2.1)"
                )
            return self.model_call(spec.models[name], e)
        if e.args is None:
            raise self.err("a callable needs an argument list")
        if name == "len":
            self.arity(e, 1)
            ir, ty = self.expr(e.args[0], None)
            self.no_list(ty)
            if ty.head != "Bytes":
                raise self.err("len needs Bytes")
            return ["len", ir], NAT
        if name == "concat":
            self.arity(e, 2)
            a_ir, a_ty = self.expr(e.args[0], None)
            b_ir, b_ty = self.expr(e.args[1], None)
            self.no_list(a_ty, b_ty)
            if a_ty.head != "Bytes" or b_ty.head != "Bytes":
                raise self.err("concat needs Bytes")
            node = ["concat", a_ir, b_ir]
            if (
                a_ty.args
                and b_ty.args
                and isinstance(a_ty.args[0], int)
                and isinstance(b_ty.args[0], int)
            ):
                # Widths add: concat(Bytes<1>, Bytes<30>) is Bytes<31>; a plain or named-static operand
                # (``Bytes``, ``Bytes<N>``) gives a plain ``Bytes``, as in the checker.
                return node, A.Ty("Bytes", (a_ty.args[0] + b_ty.args[0],))
            return node, BYTES
        if name in ("take", "drop") and e.args is not None and len(e.args) == 2:
            a_ir, a_ty = self.expr(e.args[0], None)
            self.no_list(a_ty)
            if a_ty.head != "Bytes":
                raise self.err("take and drop need Bytes")
            n_ir, n_ty = self.expr(e.args[1], NAT)
            if n_ty.head not in ("Nat", "Int"):
                raise self.err("a take/drop count is a Nat")
            return [name, a_ir, n_ir], BYTES
        if name == "Ok":
            self.arity(e, 1)
            if expected is None or expected.head != "Result":
                raise self.err("Ok needs a Result context")
            ir, ty = self.expr(e.args[0], expected.args[0])
            # The expected payload type is only a hint, as for ``Some`` and in the checker (``checker.rs``
            # types ``Ok(e)`` as ``Result<type(e),E>`` without comparing, R51): the position the node sits in
            # (a model body, an argument, an ``==``, an ``if``) decides whether it flows.
            return ["ok", ir], A.Ty("Result", (ty, expected.args[1]))
        if name == "Err":
            self.arity(e, 1)
            arg = e.args[0]
            if not isinstance(arg, A.Name) or len(arg.parts) != 2:
                raise self.err("Err takes a qualified error case")
            err_name, case = arg.parts
            if err_name not in spec.errors or case not in spec.errors[err_name]:
                raise self.err("an unknown error case")
            if expected is None or expected.head != "Result":
                raise self.err("Err needs a Result context")
            if expected.args[1].head != err_name:
                raise self.err("an error case of the wrong error type")
            return ["err", case], expected
        if name in L.FIXED_TAGS:
            return self.fixed_call(name, e)
        if name == "Some":
            return self.some_literal(e, expected)
        if name in spec.ctors:
            if e.statics:
                raise self.err("a constructor takes no static arguments")
            return self.constructor(name, e.args)
        if name in _LEVEL_LIST:
            raise self.level(L.LIST, "a list built-in needs the reserved ir.list level")
        if name in _LEVEL_SEQ:
            raise self.level(L.SEQ, "sequence built-ins need the reserved ir.seq level")
        if name == _FIXED_CAST:
            raise self.level(FIXED_CAST, "fixed<N>(b) has no IR form (outside every level)")
        raise self.err(f"unknown callable `{name}`")

    # -- the ir.fixed built-ins (Appendix A.1) ---------------------------------------

    def fixed_call(self, name: str, e: A.Call) -> tuple[object, A.Ty]:
        """Type and lower one of the 35 ``ir.fixed`` built-ins to its Appendix A.1 shape.

        One IR node per call, same spelling, positional operands; a static type argument
        (``widen<U32>(x)``) becomes the canonical type name, a JSON string in position 1.
        Typing is inferred from the operands, as for ``add``, and is written from VER04j
        sections 2.3, 3.2 and 3.3 and the table of VER04r A.1, not from the Rust checker; a
        mismatch is a ``type`` error (a corpus error), never a silent conversion.  A built-in
        in a v0 document is out of scope (VER04j 1.2: the names are version scoped) and keeps
        the refusal it had before S10a.
        """
        if self.spec.version != "v1":
            raise self.level(L.FIXED, _FIXED_MESSAGE)
        arity = L.FIXED_ARITY[name]
        static: str | None = None
        if name in L.STATIC_TYPE_TAGS:
            if len(e.statics) != 1 or not isinstance(e.statics[0], str):
                raise self.err("this built-in takes one fixed-width type argument")
            static = e.statics[0]
            if static not in FIXED:
                raise self.err("a static type argument is a fixed-width type")
        elif e.statics:
            raise self.err("this built-in takes no static argument")
        if e.args is None or len(e.args) != arity:
            raise self.err("wrong number of arguments")
        args = e.args
        if name in ("max_value", "min_value"):
            return [name, static], A.Ty(static)
        if name in ("widen", "wrap", "try"):
            x_ir, x_ty = self.expr(args[0], None)
            if is_fixed(x_ty):
                if name == "widen" and not _range_inside(x_ty.head, static):
                    raise self.err("widen needs a source whose range is inside the target")
            elif name == "widen" or x_ty.head not in ("Int", "Nat"):
                raise self.err("a conversion needs a fixed-width, Int or Nat operand")
            target = A.Ty(static)
            return [name, static, x_ir], (A.Ty("Option", (target,)) if name == "try" else target)
        if name in ("to_int", "to_nat"):
            x_ir, x_ty = self.expr(args[0], None)
            if not is_fixed(x_ty) or (name == "to_nat" and FIXED[x_ty.head][1]):
                raise self.err(
                    "to_int needs a fixed-width operand and to_nat an unsigned one"
                )
            return [name, x_ir], (INT if name == "to_int" else NAT)
        if name in ("from_be", "from_le"):
            width = FIXED[static][0] // 8
            b_ir, b_ty = self.expr(args[0], A.Ty("Bytes", (width,)))
            if b_ty.head != "Bytes" or b_ty.args != (width,):
                raise self.err("from_be/from_le need Bytes of exactly the target's width")
            b_ty = A.Ty("Bytes", (width,))
            return [name, static, b_ir], A.Ty(static)
        if name in ("bit_not", "popcount", "clz", "ctz", "be_bytes", "le_bytes"):
            x_ir, x_ty = self.expr(args[0], None)
            if not is_fixed(x_ty):
                raise self.err("a bit operation needs a fixed-width operand")
            if name == "bit_not":
                return [name, x_ir], x_ty
            if name in ("be_bytes", "le_bytes"):
                return [name, x_ir], A.Ty("Bytes", (FIXED[x_ty.head][0] // 8,))
            return [name, x_ir], NAT
        if name in ("shl", "shr", "sar", "rotl", "rotr", "bit"):
            x_ir, x_ty = self.expr(args[0], None)
            n_ir, n_ty = self.expr(args[1], NAT)
            if not is_fixed(x_ty):
                raise self.err("a shift or bit test needs a fixed-width operand")
            if n_ty != NAT:
                raise self.err("a shift count or bit index is a Nat")
            if name == "sar" and not FIXED[x_ty.head][1]:
                raise self.err("sar is for signed operands (a logical shift is shr)")
            return [name, x_ir, n_ir], (BOOL if name == "bit" else x_ty)
        if name in ("bit_and", "bit_or", "bit_xor", "checked_add", "checked_sub", "checked_mul"):
            a_ir, a_ty, b_ir, b_ty = self._operand_pair(args)
            if not is_fixed(a_ty) or a_ty != b_ty:
                raise self.err("operands must be fixed-width values of one type")
            result = a_ty if name.startswith("bit_") else A.Ty("Option", (a_ty,))
            return [name, a_ir, b_ir], result
        if name in ("div", "rem"):
            a_ir, a_ty, b_ir, b_ty = self._operand_pair(args)
            if is_fixed(a_ty) or is_fixed(b_ty):
                # Unsigned pairs only; a signed pair must say div_trunc/rem_trunc (floor and
                # truncation differ, VER04j 2.3) and a fixed width never mixes with Int/Nat.
                if a_ty != b_ty or FIXED[a_ty.head][1]:
                    raise self.err("div and rem take two unsigned operands of one type, or Int/Nat")
                return [name, a_ir, b_ir], A.Ty("Option", (a_ty,))
            if a_ty.head not in ("Int", "Nat") or b_ty.head not in ("Int", "Nat"):
                raise self.err("div and rem take two unsigned operands of one type, or Int/Nat")
            # R7: two Nat operands give Option<Int> (the grammar checker's typing).
            return [name, a_ir, b_ir], A.Ty("Option", (INT,))
        if name in ("div_trunc", "rem_trunc"):
            a_ir, a_ty, b_ir, b_ty = self._operand_pair(args)
            if not is_fixed(a_ty) or a_ty != b_ty or not FIXED[a_ty.head][1]:
                raise self.err("div_trunc and rem_trunc take two signed operands of one type")
            return [name, a_ir, b_ir], A.Ty("Option", (a_ty,))
        if name in ("imin", "imax"):
            a_ir, a_ty = self.expr(args[0], INT)
            b_ir, b_ty = self.expr(args[1], INT)
            if a_ty.head not in ("Int", "Nat") or b_ty.head not in ("Int", "Nat"):
                raise self.err("imin and imax take Int or Nat operands")
            return [name, a_ir, b_ir], INT
        if name == "at":
            b_ir, b_ty = self.expr(args[0], None)
            i_ir, i_ty = self.expr(args[1], NAT)
            self.no_list(b_ty)
            if b_ty.head != "Bytes":
                raise self.err("at needs Bytes")
            if i_ty.head not in ("Nat", "Int"):
                raise self.err("an index is a Nat or an Int")
            return [name, b_ir, i_ir], A.Ty("Option", (A.Ty("U8"),))
        # unwrap_or(o, d): any ``Option``-typed first operand (section 16.14 R29 (e)); the one that is
        # directly a producer node is the intermediate use and adds no ir.data (``expr``).
        o_ir, o_ty = self.expr(args[0], None, intermediate=True)
        if o_ty.head != "Option":
            raise self.err("unwrap_or needs an Option")
        inner = o_ty.args[0]
        d_ir, d_ty = self.expr(args[1], inner)
        if not _assignable(d_ty, inner):
            raise self.err("the default of unwrap_or does not have the Option's type")
        return [name, o_ir, d_ir], inner

    def _operand_pair(self, args: tuple) -> tuple[object, A.Ty, object, A.Ty]:
        """Two operands of one operator: a bare literal takes the other side's type."""
        left, right = args
        if _needs_context(left) and not _needs_context(right):
            b_ir, b_ty = self.expr(right, None)
            a_ir, a_ty = self.expr(left, b_ty)
        else:
            a_ir, a_ty = self.expr(left, None)
            b_ir, b_ty = self.expr(right, a_ty)
        return a_ir, a_ty, b_ir, b_ty

    def no_list(self, *types: A.Ty) -> None:
        """A ``List`` operand of ``len concat take drop at`` is the reserved ``ir.list`` (R27), never a ``type`` error.

        A list type is refused where it is *declared*, but a unit may be lowered before the declaration it
        depends on is (a record whose field is a ``List`` that comes later in the document): the operand
        then still has its unresolved ``List`` type, and the unit is refused with the level, not with
        ``len needs Bytes``.
        """
        if any(t.head == "List" for t in types):
            raise self.level(L.LIST, "a list operand needs the reserved ir.list level")

    def arity(self, e: A.Call, n: int) -> None:
        if e.args is None or len(e.args) != n or e.statics:
            raise self.err("wrong number of arguments")

    def _args(self, params: list, e: A.Call, statics: dict) -> list:
        if e.args is None or len(e.args) != len(params):
            raise self.err("wrong number of arguments")
        out = []
        for arg, (_, pty) in zip(e.args, params):
            want = _substitute(pty, statics)
            ir, ty = self.expr(arg, want)
            if not _assignable(ty, want):
                raise self.err("an argument does not match its parameter type")
            out.append(ir)
        return out

    def model_call(self, model: LoweredModel, e: A.Call) -> tuple[object, A.Ty]:
        if model.name not in self.order:
            raise OracleError("generation", "a model calls itself (VER04r 4.2.1)")
        if e.statics:
            raise self.err("a model takes no static arguments")
        args = self._args(model.params, e, {})
        if model.ir is None:
            # Not lowerable (a level this lowerer cannot lower): the caller inherits it.  A
            # model that needs only ir.fixed *is* lowered; the caller's ``requires`` then
            # includes it through the call closure (R9).
            raise model.unsupported
        return ["model_call", model.name, args], model.ret

    def subject_call(self, op: OpSig, e: A.Call) -> tuple[object, A.Ty]:
        for _, pty in op.params:
            self.check_type(pty)
        self.check_type(op.ret)
        if len(e.statics) != len(op.statics):
            raise self.err("wrong number of static arguments")
        widths = []
        statics: dict = {}
        for sname, value in zip(op.statics, e.statics):
            if not isinstance(value, int):
                raise OracleError("generation", "a named static width cannot enter corpus IR")
            statics[sname] = value
            widths.append(value)
        args = self._args(op.params, e, statics)
        node: list = ["subject_op", op.name, args]
        if widths:
            node.append(widths)
        return node, _substitute(op.ret, statics)


def _err_binds(pattern: object) -> frozenset:
    """The names an ``Err(e)`` pattern, at any depth of ``pattern``, binds to an error value."""
    out: set[str] = set()
    stack = [pattern]
    while stack:
        p = stack.pop()
        if not isinstance(p, A.Pattern):
            continue
        if p.kind == "ctor" and p.name == ("Err",) and len(p.items) == 1 and p.items[0].kind == "bind":
            out.add(p.items[0].name[0])
        stack.extend(p.items)
    return frozenset(out)


def _tuple_index(text: str) -> int | None:
    """A projection index: canonical decimal digits (no sign, no leading zero), else ``None``."""
    if text.isascii() and text.isdigit() and (text == "0" or text[0] != "0"):
        return int(text)
    return None


def _mentions(node: object, name: str) -> bool:
    """Does a source tree name ``name`` (as a variable or a call head)?"""
    stack = [node]
    while stack:
        item = stack.pop()
        if isinstance(item, A.Name) and item.parts[0] == name:
            return True
        if isinstance(item, A.Call) and item.name[0] == name:
            return True
        if isinstance(item, tuple | list):
            stack.extend(item)
        elif hasattr(item, "__slots__") and not isinstance(item, str):
            stack.extend(getattr(item, f, None) for f in item.__slots__)
    return False


def _range_inside(source: str, target: str) -> bool:
    """Is the range of fixed type ``source`` inside that of ``target``? (``widen``'s rule.)"""

    def span(name: str) -> tuple[int, int]:
        bits, signed = FIXED[name]
        return (-(1 << (bits - 1)), (1 << (bits - 1)) - 1) if signed else (0, (1 << bits) - 1)

    (lo, hi), (tlo, thi) = span(source), span(target)
    return tlo <= lo and hi <= thi


def _calls_op(spec: Spec, node: object) -> bool:
    """Does a source tree call a subject op (a callable declared with ``op``)?"""
    stack = [node]
    while stack:
        item = stack.pop()
        if isinstance(item, A.Call) and len(item.name) == 1 and item.name[0] in spec.ops:
            return True
        if isinstance(item, tuple | list):
            stack.extend(item)
        elif hasattr(item, "__slots__") and not isinstance(item, str):
            stack.extend(getattr(item, f, None) for f in item.__slots__)
    return False


def _substitute(ty: A.Ty, statics: dict) -> A.Ty:
    """Replace static width names by their values (``Bytes<N>`` with N=4)."""
    if ty.head == "Bytes" and ty.args and isinstance(ty.args[0], str):
        return A.Ty("Bytes", (statics[ty.args[0]],)) if ty.args[0] in statics else ty
    if ty.args and all(isinstance(a, A.Ty) for a in ty.args):
        return A.Ty(ty.head, tuple(_substitute(a, statics) for a in ty.args))
    return ty


def walk(ir):
    """Yield every IR node of a tree (pre-order); never a ``match`` pattern (:func:`.irtree.children`)."""
    stack = [ir]
    while stack:
        node = stack.pop()
        if not (isinstance(node, list) and node and isinstance(node[0], str)):
            continue
        yield node
        stack.extend(reversed(children(node)))


def _has_subject(ir) -> bool:
    return any(n[0] == "subject_op" for n in walk(ir))


def check_pure_guards(ir: object, depth: int = 0) -> None:
    """Refuse a subject call under an impure guard (VER04r 4.4, ``W0701``), mirroring the generator.

    ``expected_calls`` must not depend on what the subject returned, so a subject call may sit
    under a guard only if the guard calls no subject::

        if pure(x) then op(a) else true     ok     -- the guard calls no subject
        if op(x) then op(a) else true       W0701
        op(a) and op(b)                     W0701  -- op(a) guards op(b) and is impure
        pure(x) and op(b)                   ok
        op(a) and pure(b)                   ok     -- op(a) guards nothing that calls
        match op(x) with arms calling op    W0701  -- a match arm is guarded by its scrutinee

    A guard is the condition of an ``if``, the left operand of ``and``/``or``/``implies`` and the
    scrutinee of a ``match`` whose arms call the subject.  The walk is the one
    :func:`.ir_eval.compile_calls` makes at evaluation (which keeps its own refusals as a
    backstop); doing it here makes the refusal a property of *lowering*.
    """
    if depth >= MAX_IR_DEPTH:
        raise OracleError("limit", "IR nesting exceeds 64 levels")
    if not _has_subject(ir):
        return
    tag = ir[0]
    impure = OracleError("generation", "W0701: a subject call under an impure guard (VER04r 4.4)")
    if tag == "if":
        cond, then, other = ir[1], ir[2], ir[3]
        if _has_subject(then) or _has_subject(other):
            if _has_subject(cond):
                raise impure
            check_pure_guards(then, depth + 1)
            check_pure_guards(other, depth + 1)
        else:
            check_pure_guards(cond, depth + 1)
    elif tag in ("and", "or", "implies"):
        if _has_subject(ir[2]):
            if _has_subject(ir[1]):
                raise impure
            check_pure_guards(ir[2], depth + 1)
        else:
            check_pure_guards(ir[1], depth + 1)
    elif tag == "match":
        if any(_has_subject(body) for _pattern, body in match_arms(ir)):
            if _has_subject(ir[1]):
                raise impure
            for _pattern, body in match_arms(ir):
                check_pure_guards(body, depth + 1)
        else:
            check_pure_guards(ir[1], depth + 1)
    else:
        for child in children(ir):
            check_pure_guards(child, depth + 1)


def _collect_ops(ir) -> list[str]:
    seen: list[str] = []
    for node in walk(ir):
        if node[0] == "subject_op" and node[1] not in seen:
            seen.append(node[1])
    return seen
