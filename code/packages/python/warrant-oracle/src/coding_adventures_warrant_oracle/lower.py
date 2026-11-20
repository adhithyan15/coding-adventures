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
:mod:`.ir_eval` evaluates them too, so a model, op admit, claim or example that uses one of those
is lowered completely (its IR, its ``requires`` and its cost) and is evaluable like any other.  In a
v0 document the built-ins keep the ``ir.fixed`` ``level`` refusal (VER04j 1.2: the names are
version scoped).

``ir.tuple`` (slice S10t-1)
---------------------------
Tuple types ``(T1,..,Tn)``, the tuple literal, ``t.i`` / ``e.i`` projection and ``match`` on a
``Result`` (the six patterns ``ok err err_any tuple bind wild``) are lowered to
``["tuple", [..]]``, ``["project", e, i]`` and ``["match", s, [[pat, arm], ..]]`` (Appendix A.1) and
evaluated.  A ``match`` is ``ir.tuple`` exactly when its scrutinee is a ``Result`` and ``ir.data``
otherwise (section 4.1, D35).  The patterns are checked **before** any arm body is lowered, so the
corpus errors of A.1 (fall-through, an unreachable arm, a duplicate or shadowing ``bind``, oracle
code ``corpus``) cannot be hidden by a body that is only recorded as a level error.

Every other source construct that belongs to a level the lowerer cannot lower (``ir.data`` lists,
records, variants and ``Option`` anywhere but the intermediate use of ``ir.fixed``, ``match`` on
anything but a ``Result``, ``ir.seq`` ...) raises ``OracleError("level", ..., level=...)``.  The
error is *recorded per model and per suite* rather than aborting the document, so the oracle can still
check every suite whose closure is within what it evaluates and report, by name, the suites it
cannot.  A construct is never skipped.

``Option<T>`` is an IR-only intermediate (section 16.10 D23, R8)
----------------------------------------------------------------
Nine built-ins return an ``Option`` and ``unwrap_or`` is the **only** consumer, as its first
operand and directly.  :meth:`_Lowerer.expr` therefore refuses (level ``ir.data``) an
``Option``-typed expression unless the caller is that one operand, and refuses an ``Option``
in a signature or binder.  ``div``/``rem`` on two ``Nat`` operands are typed ``Option<Int>``
(R7, the grammar checker's rule).
"""

from __future__ import annotations

from dataclasses import dataclass, field

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

#: Source built-ins of ``ir.data`` that this lowerer does not lower (section 4.1).  The
#: ``ir.fixed`` names are :data:`.levels.FIXED_TAGS`; the core ones (``len concat Ok Err``) are
#: handled by the lowering itself.
_LEVEL_DATA = frozenset(["Some", "None", "fixed", "slice", "to_list", "to_bytes", "replicate", "range"])
_FIXED_MESSAGE = "fixed-width built-ins need ir.fixed"
_OPTION_MESSAGE = "Option is an IR-only intermediate (its only consumer is unwrap_or)"
_LEVEL_SEQ = frozenset(["fold", "map", "all", "any", "count"])

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
    [*FIXED, *L.FIXED_TAGS, *_LEVEL_DATA, *_LEVEL_SEQ, "take", "drop"]
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
    unsupported: OracleError | None = None
    line: int = 0
    #: The sorted levels of the body and of its call closure (R9); ``()`` when not lowered.
    requires: tuple = ()


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


# -- the checker/lowerer ---------------------------------------------------------


def lower_document(doc: A.Document) -> Spec:
    """Check ``doc`` and lower every model, claim and example."""
    spec = Spec(doc)
    _declare(spec)
    low = _Lowerer(spec)
    for decl in doc.decls:
        if isinstance(decl, A.ModelDecl):
            low.lower_model(decl)
    for op in spec.ops.values():
        low.lower_op_admit(op)
    for decl in doc.decls:
        if isinstance(decl, A.ClaimDecl):
            spec.suites.append(low.lower_claim(decl))
        elif isinstance(decl, A.ExampleDecl):
            spec.examples.append(low.lower_example(decl))
        elif isinstance(decl, A.InputDecl):
            spec.input_rejects.append(decl)
        elif isinstance(decl, A.EnsuresDecl):
            spec.ensures[decl.op] = decl.rows
    return spec


def _declare(spec: Spec) -> None:
    """First pass: names, uniqueness and signatures (no bodies yet)."""
    seen: set[str] = set()

    def claim_name(name: str) -> None:
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
            spec.records[d.name] = d.fields
        elif isinstance(d, A.VariantDecl):
            claim_name(d.name)
            spec.variants[d.name] = d.ctors
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
    for d in spec.doc.decls:
        if isinstance(d, A.OpDecl):
            params = []
            refinements = []
            pnames: set[str] = set()
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
            )


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
    if head in spec.errors or head in spec.records or head in spec.variants:
        if ty.args:
            raise OracleError("type", "a named type takes no arguments")
        return ty
    raise OracleError("type", "an unknown type name")


def _unify(a: A.Ty, b: A.Ty) -> A.Ty | None:
    """The common type of two operand types, or ``None`` (VER04 section 3).

    ``Nat`` widens to ``Int`` and ``Bytes<N>`` forgets its width to ``Bytes``;
    nothing else is implicit.
    """
    if a == b:
        return a
    if {a.head, b.head} == {"Nat", "Int"}:
        return INT
    if a.head == "Bytes" and b.head == "Bytes":
        return BYTES
    if a.head == b.head == "Result" and a.args[1] == b.args[1]:
        inner = _unify(a.args[0], b.args[0])
        return A.Ty("Result", (inner, a.args[1])) if inner is not None else None
    if a.head == b.head == "()" and len(a.args) == len(b.args):
        parts = [_unify(x, y) for x, y in zip(a.args, b.args)]
        return None if any(p is None for p in parts) else A.Ty("()", tuple(parts))
    return None


def _assignable(src: A.Ty, dst: A.Ty) -> bool:
    """Can a value of ``src`` flow to ``dst``?  (Nat to Int, Bytes<N> to Bytes.)"""
    if src == dst:
        return True
    if src.head == "Nat" and dst.head == "Int":
        return True
    if src.head == "Bytes" and dst.head == "Bytes":
        return not dst.args or src.args == dst.args
    if src.head == dst.head == "Result" and src.args[1] == dst.args[1]:
        return _assignable(src.args[0], dst.args[0])
    if src.head == dst.head == "()" and len(src.args) == len(dst.args):
        return all(_assignable(x, y) for x, y in zip(src.args, dst.args))
    return False


def _needs_context(e) -> bool:
    """Does ``e`` have no type of its own (so its sibling must be typed first)?"""
    if isinstance(e, A.IntLit):
        return e.suffix is None
    if isinstance(e, A.Call) and len(e.name) == 1 and e.name[0] in ("Ok", "Err"):
        return True
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
        #: Static widths of ``Bytes`` expressions whose *type* forgot them (a literal, a
        #: ``concat`` of such, an ``if`` of equal widths): ``id(ir) -> (ir, width)``.  The IR
        #: node is kept so its id is never reused.  Filled in the single lowering pass.
        self._widths: dict = {}
        #: Fixed-width ``add``/``sub``/``mul``/``neg`` nodes lowered so far, ``id(ir) -> ir`` (the
        #: node is kept so its id is never reused), and the models whose call closure holds one.
        #: Both feed the v0 guard, VER04r 3.3 rule 3 (:meth:`_v0_rule_3`).
        self._fixed_arith: dict = {}
        self._wrapping_models: set = set()

    # -- helpers ---------------------------------------------------------------

    def err(self, message: str) -> OracleError:
        return OracleError("type", message)

    def level(self, level: str, message: str) -> OracleError:
        return OracleError("level", message, level=level)

    def seal(self, *irs: object) -> tuple[tuple, OracleError | None]:
        """The ``requires`` of a fully lowered unit and its evaluation refusal.

        ``requires`` is :func:`.levels.ir_levels` over ``irs`` and the call closure of every
        ``model_call`` (R9).  Since slice S10c-1 the oracle evaluates ``ir.core``, ``ir.tuple`` and
        ``ir.fixed``, so a unit that lowered completely is never refused here: the second result
        is always ``None`` and is kept so that every caller (and the generator's mirror of this
        function) still has one place that could mark a unit ``unsupported`` for a level the
        evaluator lacks.  A construct of ``ir.data`` or ``ir.seq`` never reaches this point: the
        lowerer raises a ``level`` error for it.
        """
        # ``ir.core`` is always in a row's requires (VER04r section 6: "at least").  The call
        # closure is read from each callee's own ``requires`` (already closed over its callees,
        # in declaration order) instead of walking it again for every unit.
        found = {L.CORE, *L.ir_levels(list(irs), follow=False)}
        for tree in irs:
            for node in walk(tree):
                if node[0] == "model_call":
                    found.update(self.spec.models[node[1]].requires)
        found |= self._type_levels
        return tuple(sorted(found)), None

    def check_type(self, ty: A.Ty) -> None:
        """A type of a signature or binder: refuse ``Option`` (D23), count a tuple (R9)."""
        if L.type_refusal(ty) is not None:
            raise self.level(L.DATA, _OPTION_MESSAGE)
        self._type_levels |= L.type_levels(ty, self._data_names)

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
            if not self._flows(ir, ty, ret):
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
            if exc.code != "level":
                raise
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
            if exc.code != "level":
                raise
            op.unsupported = exc
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
            if exc.code != "level":
                raise
            unsupported = exc
            lowered = False
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
            if exc.code != "level":
                raise
            return LoweredExample(decl.name, ["lit", True], None, None, None, exc, decl.line)
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
        self, e, expected: A.Ty | None, *, option_ok: bool = False
    ) -> tuple[object, A.Ty]:
        """Lower ``e``; ``expected`` types bare literals and ``Ok``/``Err``.

        Nesting is capped at 64 (VER04 section 2: "64 nested expression nodes on any
        path"): a left-associative chain of 2,000 additions is a path of depth 2,000.

        ``option_ok`` is true for exactly one caller: ``unwrap_or``, for its first operand
        (section 16.10 D23, R8).  An ``Option``-typed result anywhere else is the level
        ``ir.data``, so no node but ``unwrap_or`` ever receives an ``Option``.
        """
        self._nest += 1
        try:
            if self._nest > MAX_EXPR_DEPTH:
                raise OracleError("limit", "an expression nests deeper than 64 nodes")
            ir, ty = self._expr(e, expected)
            if ty.head == "Option" and not option_ok:
                raise self.level(L.DATA, _OPTION_MESSAGE)
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
            ty = BYTES
            if (
                expected is not None
                and expected.head == "Bytes"
                and expected.args
                and isinstance(expected.args[0], int)
                and len(e.hex) // 2 == expected.args[0]
            ):
                ty = expected
            node = ["lit", {"type": "Bytes", "value": e.hex}]
            self._note_width(node, len(e.hex) // 2)
            return node, ty
        if isinstance(e, A.Name):
            if len(e.parts) == 1:
                name = e.parts[0]
                if name in self.scope:
                    return ["var", name], self.scope[name]
                if name == "None" or name in spec.records or _is_ctor(spec, name):
                    raise self.level("ir.data", "constructors need ir.data")
                raise self.err("an unbound name")
            head = e.parts[0]
            if head in self.scope:
                if self.scope[head].head == "()":
                    return self.project(["var", head], self.scope[head], e.parts[1:])
                raise self.level("ir.data", "projection needs ir.data")
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
                o_ir, o_ty = self.expr(e.other, expected if expected is not None else t_ty)
            common = _unify(t_ty, o_ty)
            if common is None:
                raise self.err("the branches of an `if` have no common type")
            node = ["if", c_ir, t_ir, o_ir]
            wt, wo = self._static_width(t_ir, t_ty), self._static_width(o_ir, o_ty)
            if wt is not None and wt == wo:
                self._note_width(node, wt)
            return node, common
        if isinstance(e, A.TupleLit):
            return self.tuple_literal(e, expected)
        if isinstance(e, A.Proj):
            base_ir, base_ty = self.expr(e.base, None)
            if base_ty.head == "()":
                return self.project(base_ir, base_ty, (e.field,))
            if base_ty.head in spec.records or base_ty.head in spec.variants:
                raise self.level(L.DATA, "projection needs ir.data")
            raise self.err("a projection needs a tuple")
        if isinstance(e, A.ListLit):
            raise self.level(L.DATA, "lists need ir.data")
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
        """Chain ``["project", e, i]`` for each index in ``parts`` (``t.0.1``); ``i`` is a JSON integer."""
        for part in parts:
            index = _tuple_index(part)
            if ty.head != "()" or index is None or index >= len(ty.args):
                raise self.err("a tuple projection is a component index of a tuple")
            ir, ty = ["project", ir, index], ty.args[index]
            self._type_levels |= L.type_levels(ty, self._data_names)
        return ir, ty

    def match(self, e: A.Match, expected: A.Ty | None) -> tuple[object, A.Ty]:
        """``match r with Ok(..) -> .. | Err(..) -> ..`` on a ``Result`` (A.1; ``ir.tuple``, D35).

        ``match`` is ``ir.tuple`` exactly when its scrutinee is a ``Result`` and ``ir.data`` for any
        other type (section 4.1), so the scrutinee is typed first.  The patterns are lowered and the
        corpus-error rules (fall-through, an unreachable arm, a duplicate or shadowing ``bind``) are
        decided *before* any arm body is lowered, so such a document is refused even if a body would
        have been recorded as a level error.  Arm bodies are typed with the pattern's binders in scope
        (an arm that needs a context takes the first self-typed arm's type, as ``if`` does); the arms
        must have one type.
        """
        s_ir, s_ty = self.expr(e.scrutinee, None)
        if s_ty.head != "Result":
            raise self.level(L.DATA, "match needs ir.data")
        patterns: list = []
        bound: list = []
        for pattern, _ in e.arms:
            binds: list = []
            patterns.append(self.pattern(pattern, s_ty, binds, set()))
            bound.append(binds)
        P.check_arms(patterns, s_ty, self.spec.errors)
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
                        raise self.level(L.DATA, "an error value bound by a pattern has no IR form")
                bodies[i], types[i] = self.expr(body, hint)
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
        return node, common

    def pattern(
        self, p: A.Pattern, ty: A.Ty, binds: list, seen: set, depth: int = 1
    ) -> list:
        """Lower one source pattern against the static type ``ty`` to its IR pattern (A.1).

        ``Ok(p)`` is ``["ok", p]``; ``Err(E.CASE)`` is ``["err", "CASE"]``; ``Err(_)`` and ``Err(e)``
        are ``["err_any"]`` (an error value has no IR form, so ``e`` may not be *used*; it is still
        declared, for the duplicate and shadowing rules); a tuple pattern is ``["tuple", [..]]``; a
        name is ``["bind", name]`` and ``_`` is ``["wild"]``.  A literal pattern (``true``) is not
        one of the six and is the level ``ir.data``.
        """
        if depth > P.MAX_PATTERN_DEPTH:
            raise OracleError("limit", "pattern nesting exceeds 8 levels")
        kind = p.kind
        if kind == "wild":
            return ["wild"]
        if kind == "bool":
            raise self.level(L.DATA, "a literal pattern needs ir.data")
        if kind == "bind":
            name = p.name[0]
            self._reject_constructor_name(name)
            self._declare(name, ty, binds, seen)
            return ["bind", name]
        if kind == "tuple":
            if ty.head != "()" or len(ty.args) != len(p.items):
                raise self.err("a tuple pattern does not match the type of its value")
            return [
                "tuple",
                [self.pattern(item, t, binds, seen, depth + 1) for item, t in zip(p.items, ty.args)],
            ]
        parts = p.name
        if parts == ("Ok",):
            if ty.head != "Result" or len(p.items) != 1:
                raise self.err("Ok takes one pattern and needs a Result")
            return ["ok", self.pattern(p.items[0], ty.args[0], binds, seen, depth + 1)]
        if parts == ("Err",):
            if ty.head != "Result" or len(p.items) != 1:
                raise self.err("Err takes one pattern and needs a Result")
            inner = p.items[0]
            if inner.kind == "wild":
                return ["err_any"]
            if inner.kind == "bind":
                self._reject_constructor_name(inner.name[0])
                self._declare(inner.name[0], None, binds, seen)
                return ["err_any"]
            if inner.kind == "ctor" and len(inner.name) == 2 and not inner.items:
                err_name, case = inner.name
                if err_name != ty.args[1].head or case not in self.spec.errors.get(err_name, ()):
                    raise self.err("an error case of the wrong error type")
                return ["err", case]
            raise self.err("an Err pattern is an error case, a name or _")
        if len(parts) == 1 and (parts[0] in ("Some", "None") or _is_ctor(self.spec, parts[0])):
            raise self.level(L.DATA, "constructor patterns need ir.data")
        raise self.err("an unknown constructor in a pattern")

    def _reject_constructor_name(self, name: str) -> None:
        # A record's name is both its type and its one constructor (the checker keeps it in
        # ``ctor_index``), so a bind spelling a record is refused like a variant constructor.
        if name in ("Some", "None") or name in self.spec.records or _is_ctor(self.spec, name):
            raise self.level(L.DATA, "constructor patterns need ir.data")
        if name in ("Ok", "Err"):
            raise self.err("Ok and Err are constructors, not names")

    def _declare(self, name: str, ty: A.Ty | None, binds: list, seen: set) -> None:
        """Declare a pattern binder: a duplicate or a shadowing name is a corpus error (A.1, R2)."""
        if name in seen:
            raise OracleError("corpus", "the same bind name twice in one pattern")
        spec = self.spec
        # A bind spelling a model, an operation, an error type or a reserved built-in is refused
        # (CV2-F8; the checker's ``check_arm_binder``, W0203).  The checker reads a name in a body
        # by what it is declared as, so a bind called ``take`` or ``rotl`` would let the two
        # tools disagree about what the name refers to inside the arm: the checker types ``take(..)``
        # as the built-in while an evaluator would call the bound value.  Refusing the bind keeps
        # every name meaning one thing.  Constructors never reach here (see
        # :meth:`_reject_constructor_name`).  Built-ins are version scoped: a v0 document may bind
        # a name that only a v1 document reserves.
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
        binds.append((name, ty))

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
            if ty.head in ("List", "Option", "()") or ty.head in self.spec.records:
                raise self.level("ir.data", "len of a non-Bytes sequence needs ir.data")
            if ty.head != "Bytes":
                raise self.err("len needs a sequence")
            return ["len", ir], NAT
        if name == "concat":
            self.arity(e, 2)
            a_ir, a_ty = self.expr(e.args[0], None)
            b_ir, b_ty = self.expr(e.args[1], None)
            if a_ty.head != "Bytes" or b_ty.head != "Bytes":
                raise self.level("ir.data", "concat of lists needs ir.data")
            node = ["concat", a_ir, b_ir]
            wa, wb = self._static_width(a_ir, a_ty), self._static_width(b_ir, b_ty)
            if wa is not None and wb is not None:
                self._note_width(node, wa + wb)
            if (
                a_ty.args
                and b_ty.args
                and isinstance(a_ty.args[0], int)
                and isinstance(b_ty.args[0], int)
            ):
                # Widths add: concat(Bytes<1>, Bytes<30>) is Bytes<31>.
                return node, A.Ty("Bytes", (a_ty.args[0] + b_ty.args[0],))
            return node, BYTES
        if name in ("take", "drop") and e.args is not None and len(e.args) == 2:
            a_ir, a_ty = self.expr(e.args[0], None)
            if a_ty.head != "Bytes":
                raise self.level("ir.data", "take/drop of lists needs ir.data")
            n_ir, n_ty = self.expr(e.args[1], NAT)
            if n_ty.head not in ("Nat", "Int"):
                raise self.err("a take/drop count is a Nat")
            return [name, a_ir, n_ir], BYTES
        if name == "Ok":
            self.arity(e, 1)
            if expected is None or expected.head != "Result":
                raise self.err("Ok needs a Result context")
            ir, ty = self.expr(e.args[0], expected.args[0])
            if not self._flows(ir, ty, expected.args[0]):
                raise self.err("an Ok payload does not have the Result's value type")
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
        if name in _LEVEL_DATA or name in ("Some",):
            raise self.level("ir.data", "Option and list built-ins need ir.data")
        if name in _LEVEL_SEQ:
            raise self.level("ir.seq", "sequence built-ins need the reserved ir.seq level")
        if name in spec.records or _is_ctor(spec, name):
            raise self.level("ir.data", "constructors need ir.data")
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
            if self._static_width(b_ir, b_ty) != width:
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
            if b_ty.head == "List":
                raise self.level(L.DATA, "at on a list needs ir.data")
            i_ir, i_ty = self.expr(args[1], NAT)
            if b_ty.head != "Bytes":
                raise self.err("at needs Bytes")
            if i_ty.head not in ("Nat", "Int"):
                raise self.err("an index is a Nat or an Int")
            return [name, b_ir, i_ir], A.Ty("Option", (A.Ty("U8"),))
        # unwrap_or(o, d): the one consumer of an Option, as the first operand and directly.
        o_ir, o_ty = self.expr(args[0], None, option_ok=True)
        if o_ty.head != "Option":
            raise self.err("unwrap_or needs an Option")
        if o_ir[0] not in L.OPTION_PRODUCERS:
            raise self.level(L.DATA, "an Option reaches unwrap_or only straight from a built-in")
        inner = o_ty.args[0]
        d_ir, d_ty = self.expr(args[1], inner)
        if not _assignable(d_ty, inner):
            raise self.err("the default of unwrap_or does not have the Option's type")
        return [name, o_ir, d_ir], inner

    def _flows(self, ir: object, ty: A.Ty, want: A.Ty) -> bool:
        """Can the lowered expression ``ir`` of type ``ty`` flow to ``want``?

        :func:`_assignable`, plus the static-width rule of VER04r A.1: an expression whose *type*
        forgot its width (a bytes literal, a ``concat`` of sized operands, an ``if`` of equal widths)
        but whose recorded static width is ``N`` flows to ``Bytes<N>``, as in the grammar checker.
        A tuple or ``Result`` is checked component by component only where the types already agree
        structurally; widths are looked up for a ``Bytes`` expression itself.
        """
        if _assignable(ty, want):
            return True
        if ty.head == want.head == "Bytes" and want.args and isinstance(want.args[0], int):
            return self._static_width(ir, ty) == want.args[0]
        return False

    def _note_width(self, ir: object, width: int) -> None:
        self._widths[id(ir)] = (ir, width)

    def _static_width(self, ir: object, ty: A.Ty) -> int | None:
        """The static width of a lowered ``Bytes`` expression, or ``None``.

        The grammar checker types a bytes literal ``Bytes<len>``, a ``concat`` of two
        statically sized operands ``Bytes<sum>`` and an ``if`` whose branches have the *same*
        static width ``Bytes<n>`` (``common_type``); anything else that is not already
        ``Bytes<N>`` is a plain ``Bytes``.  The source typer here forgets those widths (so no
        ``result_type`` moves), so they are recorded as nodes are lowered, once, and read back
        by ``from_be``/``from_le``, the one place a width is required.  VER04r A.1 names the rule.
        """
        if ty.head != "Bytes":
            return None
        if ty.args and isinstance(ty.args[0], int):
            return ty.args[0]
        hit = self._widths.get(id(ir))
        return hit[1] if hit is not None else None

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
            if not self._flows(ir, ty, want):
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


def _is_ctor(spec: Spec, name: str) -> bool:
    return any(name == c for ctors in spec.variants.values() for c, _ in ctors)


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
