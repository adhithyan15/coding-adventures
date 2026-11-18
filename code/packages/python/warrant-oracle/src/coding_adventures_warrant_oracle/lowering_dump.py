"""The committed *lowering dump*: what the oracle lowers a document to, as canonical JSON.

Slice S10a of VER04r section 13 asks for "a committed lowering dump" of the ``ir.fixed``
level, so that slice S10b (the Rust generator) can reproduce the oracle's lowering **byte for
byte** before it is allowed to emit anything: the S4-part-1-before-S3a pattern.  The dump of
a document is the oracle's *parity view*:

``errors`` and ``ops``
    the declared error types and each op's v2 corpus row (``params``, ``ret``, ``fresh``,
    ``width_params``, ``admit`` and the computed ``requires``);
``model_order`` and ``models``
    each model's parameters, return type, lowered ``body`` and ``ir_cost`` (a model has no
    ``requires`` of its own, section 1.4: its levels reach a suite through the call closure);
``suites``
    each claim's binders, ``admit``, ``body``, the two sides of a ``reference`` oracle, its
    result type, its ops, its computed ``requires`` (section 16.10 R9) and its ``ir_cost``
    (the sum of the worst-path costs of ``admit``, ``body`` and, for a ``reference`` suite,
    ``expected``, section 4.2 rule 6);
``examples``
    each example's lowered ``body`` and its two sides.

It is the same view the three ``*.oracle-lowering.json`` files of the generator's fixtures
show for ``ir.core`` documents (``dump_oracle_lowering.py``), extended with ``requires`` and
written for documents that need ``ir.fixed``; its ``schema`` is therefore
``warrant-lowering-parity-v1`` and the ``v0`` files are untouched.  The text is the canonical
writer of VER04i section 9.1, exactly what :func:`.corpus_v2.canonical` writes for a corpus.

Nothing here evaluates: costs come from :class:`.ir_eval.Program` in ``analyze`` mode, which
types and costs the ``ir.fixed`` built-ins and refuses to run them.
"""

from __future__ import annotations

from . import levels as L
from .corpus_v1 import build_program
from .corpus_v2 import canonical
from .errors import OracleError
from .ir_eval import MAX_IR_COST
from .lower import Spec, lower_document
from .parser import parse_source

SCHEMA = "warrant-lowering-parity-v1"
#: The levels a lowering dump can show: everything the lowerer lowers (S10a, S10t-1).
LOWERED_LEVELS = frozenset([L.CORE, L.FIXED, L.TUPLE])


def _params(pairs: list) -> list:
    return [[name, str(ty)] for name, ty in pairs]


def _refuse(kind: str, name: str, exc: OracleError | None) -> OracleError:
    why = f"{exc.code}: {exc.message}" if exc is not None else "needs a level this dump cannot show"
    level = exc.level if exc is not None and exc.level is not None else L.DATA
    return OracleError("level", f"{kind} `{name}` is beyond ir.tuple ({why})", level=level)


def _check_lowered(spec: Spec) -> None:
    """Refuse a document with any part the lowerer could not lower (fail closed)."""
    for model in spec.models.values():
        if model.ir is None or not set(model.requires) <= LOWERED_LEVELS:
            raise _refuse("model", model.name, model.unsupported)
    for op in spec.ops.values():
        if not set(op.requires) <= LOWERED_LEVELS or (
            op.unsupported is not None and op.unsupported.level not in LOWERED_LEVELS
        ):
            raise _refuse("op", op.name, op.unsupported)
    for suite in spec.suites:
        if suite.refusal is not None:
            raise suite.refusal  # the generator refuses it at lowering too (VER04r 4.3, 4.4)
        if not set(suite.requires) <= LOWERED_LEVELS or not suite.requires:
            raise _refuse("claim", suite.claim, suite.unsupported)
    for ex in spec.examples:
        if not set(ex.requires) <= LOWERED_LEVELS or not ex.requires:
            raise _refuse("example", ex.name, ex.unsupported)


def view(spec_bytes: bytes, max_cost: int = MAX_IR_COST) -> dict:
    """The parity view of one document (see the module docstring)."""
    spec = lower_document(parse_source(spec_bytes))
    _check_lowered(spec)
    prog = build_program(spec, max_cost=max_cost, analyze=True)
    models = {}
    for name, m in spec.models.items():
        models[name] = {
            "params": _params(m.params),
            "ret": str(m.ret),
            "body": m.ir,
            "ir_cost": prog.model(name).cost,
        }
    suites = []
    for s in spec.suites:
        tenv = dict(s.binders)
        cost = prog.compile(s.admit, tenv, allow_subject=False).cost
        cost += prog.compile(s.body, tenv, allow_subject=True).cost
        if s.kind == "reference":
            cost += prog.compile(s.expected, tenv, allow_subject=False).cost
        suites.append(
            {
                "id": s.id,
                "claim": s.claim,
                "kind": s.kind,
                "binders": _params(s.binders),
                "admit": s.admit,
                "body": s.body,
                # The canonical writer has no null: an absent side is an empty array.
                "subject": s.subject if s.kind == "reference" else [],
                "expected": s.expected if s.kind == "reference" else [],
                "result_type": str(s.result),
                "ops": list(s.ops),
                "requires": list(s.requires),
                "ir_cost": cost,
            }
        )
    examples = []
    for x in spec.examples:
        referenced = x.subject is not None
        examples.append(
            {
                "name": x.name,
                "body": x.body,
                "subject": x.subject if referenced else [],
                "expected": x.expected if referenced else [],
                "result_type": str(x.result) if referenced else "",
            }
        )
    return {
        "schema": SCHEMA,
        "module": spec.name,
        "document": spec.version,
        "errors": {name: list(cases) for name, cases in spec.errors.items()},
        "ops": {name: op.corpus_row_v2() for name, op in spec.ops.items()},
        "model_order": list(spec.models),
        "models": models,
        "suites": suites,
        "examples": examples,
    }


def dump(spec_bytes: bytes, max_cost: int = MAX_IR_COST) -> bytes:
    """The committed dump of a document: :func:`view` in the canonical writer."""
    return canonical(view(spec_bytes, max_cost))
