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
The oracle implements ``ir.core`` only (slice S4).  Every source construct that
belongs to a higher level (``ir.fixed`` integer built-ins, ``ir.data`` tuples,
records, ``match`` ...) raises ``OracleError("level", ..., level=...)``.  The
error is *recorded per model and per suite* rather than aborting the document,
so the oracle can still check every suite whose closure is within ``ir.core``
and report, by name, the suites it cannot.  A construct is never skipped.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from . import ast as A
from .errors import OracleError

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

#: Source built-ins and the IR level each needs (VER04r section 4.1).  The core
#: ones (``len concat Ok Err``) are handled by the lowering itself.
_LEVEL_FIXED = frozenset(
    [
        "widen",
        "wrap",
        "try",
        "to_int",
        "to_nat",
        "max_value",
        "min_value",
        "checked_add",
        "checked_sub",
        "checked_mul",
        "div_trunc",
        "rem_trunc",
        "rem",
        "bit_and",
        "bit_or",
        "bit_xor",
        "bit_not",
        "shl",
        "shr",
        "sar",
        "rotl",
        "rotr",
        "popcount",
        "clz",
        "ctz",
        "bit",
        "be_bytes",
        "le_bytes",
        "from_be",
        "from_le",
        "at",
        "div",
        "imin",
        "imax",
    ]
)
_LEVEL_DATA = frozenset(
    ["Some", "None", "unwrap_or", "fixed", "slice", "to_list", "to_bytes", "replicate", "range"]
)
_LEVEL_SEQ = frozenset(["fold", "map", "all", "any", "count"])


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
    ir: object | None = None
    unsupported: OracleError | None = None
    line: int = 0


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

    def corpus_row_v2(self) -> dict:
        """The ``ops.<name>`` row of a v2 corpus (VER04r section 6; ``ir.core`` ops)."""
        row = self.corpus_row()
        row["requires"] = ["ir.core"]
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


@dataclass
class LoweredExample:
    name: str
    body: object  # IR of the whole Bool body
    subject: object | None
    expected: object | None
    result: A.Ty | None
    unsupported: OracleError | None = None
    line: int = 0


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
    return False


def _lit_value(value: int, ty: str) -> list:
    return ["lit", {"type": ty, "value": str(value)}]


class _Lowerer:
    def __init__(self, spec: Spec) -> None:
        self.spec = spec
        self.scope: dict[str, A.Ty] = {}
        self.order: list[str] = []  # models lowered so far, for "declared before"
        self._nest = 0

    # -- helpers ---------------------------------------------------------------

    def err(self, message: str) -> OracleError:
        return OracleError("type", message)

    def level(self, level: str, message: str) -> OracleError:
        return OracleError("level", message, level=level)

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
        try:
            ir, ty = self.expr(decl.body, ret)
            if not _assignable(ty, ret):
                raise self.err("a model body does not have its declared type")
            model.ir = ir
        except OracleError as exc:
            if exc.code != "level":
                raise
            model.unsupported = exc
        spec.models[decl.name] = model
        self.order.append(decl.name)

    def lower_op_admit(self, op: OpSig) -> None:
        """Lower an operation's parameter refinements into ``op.admit`` (VER04r section 6).

        One Bool IR over the parameters, ``and``-chained in source order exactly as a
        claim's binder refinements are (section 4.5).  A refinement above ``ir.core``
        is recorded in ``op.unsupported``, never skipped.
        """
        if not op.refinements:
            return
        self.scope = dict(op.params)
        parts: list = []
        try:
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
        admit = parts[0]
        for extra in parts[1:]:
            admit = ["and", admit, extra]
        op.admit = admit

    # -- claims and examples -----------------------------------------------------

    def lower_claim(self, decl: A.ClaimDecl) -> LoweredSuite:
        spec = self.spec
        suite_id = f"warrant/{spec.name}/exhaustive/{decl.name}"
        binders: list = []
        refinements: list = []
        body = decl.body
        self.scope = {}
        unsupported: OracleError | None = None
        admit: object = ["lit", True]
        ir: object = ["lit", True]
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
        except OracleError as exc:
            if exc.code != "level":
                raise
            unsupported = exc
        kind = "predicate"
        subject = expected = None
        result = BOOL
        ops = _collect_ops(ir) if unsupported is None else []
        if unsupported is None and ir[0] == "eq":
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
        try:
            ir, ty = self.expr(decl.body, BOOL)
            if ty != BOOL:
                raise self.err("an example body must be Bool")
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
        return LoweredExample(decl.name, ir, subject, expected, result, None, decl.line)

    # -- expressions -------------------------------------------------------------

    def expr(self, e, expected: A.Ty | None) -> tuple[object, A.Ty]:
        """Lower ``e``; ``expected`` types bare literals and ``Ok``/``Err``.

        Nesting is capped at 64 (VER04 section 2: "64 nested expression nodes on any
        path"): a left-associative chain of 2,000 additions is a path of depth 2,000.
        """
        self._nest += 1
        try:
            if self._nest > MAX_EXPR_DEPTH:
                raise OracleError("limit", "an expression nests deeper than 64 nodes")
            return self._expr(e, expected)
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
            return ["lit", {"type": "Bytes", "value": e.hex}], ty
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
            return ["if", c_ir, t_ir, o_ir], common
        if isinstance(e, (A.TupleLit, A.ListLit, A.Proj)):
            raise self.level("ir.data", "tuples, lists and projections need ir.data")
        if isinstance(e, A.Match):
            raise self.level("ir.data", "match needs ir.data")
        if isinstance(e, A.Quant):
            raise self.err("a quantifier is only allowed at the head of a claim")
        raise self.err("unsupported expression")

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
            return ["neg", _lit_value(-operand.value, ty.head)], ty
        ir, ty = self.expr(
            operand, expected if expected is not None and is_numeric(expected) else None
        )
        if not is_numeric(ty):
            raise self.err("unary minus needs a number")
        if ty.head in FIXED and not FIXED[ty.head][1]:
            raise self.err("unary minus on an unsigned type; write 0uN - a")
        return ["neg", ir], (ty if is_fixed(ty) else INT)

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
        return [tag, l_ir, r_ir], (common if is_fixed(common) else INT)

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
            if (
                a_ty.args
                and b_ty.args
                and isinstance(a_ty.args[0], int)
                and isinstance(b_ty.args[0], int)
            ):
                # Widths add: concat(Bytes<1>, Bytes<30>) is Bytes<31>.
                return ["concat", a_ir, b_ir], A.Ty("Bytes", (a_ty.args[0] + b_ty.args[0],))
            return ["concat", a_ir, b_ir], BYTES
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
            if not _assignable(ty, expected.args[0]):
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
        if name in _LEVEL_FIXED:
            raise self.level("ir.fixed", "fixed-width built-ins need ir.fixed")
        if name in _LEVEL_DATA or name in ("Some",):
            raise self.level("ir.data", "Option and list built-ins need ir.data")
        if name in _LEVEL_SEQ:
            raise self.level("ir.seq", "sequence built-ins need the reserved ir.seq level")
        if name in spec.records or _is_ctor(spec, name):
            raise self.level("ir.data", "constructors need ir.data")
        raise self.err(f"unknown callable `{name}`")

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
        if model.unsupported is not None:
            raise model.unsupported
        return ["model_call", model.name, args], model.ret

    def subject_call(self, op: OpSig, e: A.Call) -> tuple[object, A.Ty]:
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
    """Yield every IR node of a tree (pre-order), looking inside argument lists."""
    stack = [ir]
    while stack:
        node = stack.pop()
        if not (isinstance(node, list) and node and isinstance(node[0], str)):
            continue
        yield node
        tag = node[0]
        if tag in ("lit", "var", "err"):
            continue
        if tag in ("subject_op", "model_call"):
            stack.extend(reversed(node[2]))
            continue
        stack.extend(reversed(node[1:]))


def _has_subject(ir) -> bool:
    return any(n[0] == "subject_op" for n in walk(ir))


def _collect_ops(ir) -> list[str]:
    seen: list[str] = []
    for node in walk(ir):
        if node[0] == "subject_op" and node[1] not in seen:
            seen.append(node[1])
    return seen
