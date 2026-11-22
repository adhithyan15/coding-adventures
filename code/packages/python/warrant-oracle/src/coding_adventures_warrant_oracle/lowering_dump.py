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
    each example's lowered ``body`` and its two sides;
``types``
    **only when the document declares a record or a variant** (slice S11a): each one's corpus ``types`` row
    (section 2.2: ``{"kind": "record", "fields": [[name, type], ..]}``, ``{"kind": "variant", "ctors":
    [[ctor, [type, ..]], ..]}``), so a document with none of them dumps byte for byte as it did before the
    level existed.

It is the same view the three ``*.oracle-lowering.json`` files of the generator's fixtures
show for ``ir.core`` documents (``dump_oracle_lowering.py``), extended with ``requires`` and
written for documents that need ``ir.fixed``; its ``schema`` is therefore
``warrant-lowering-parity-v1`` and the ``v0`` files are untouched.  The text is the canonical
writer of VER04i section 9.1, exactly what :func:`.corpus_v2.canonical` writes for a corpus.

Nothing here evaluates: costs come from :class:`.ir_eval.Program`, which types and costs every tag
(``ir.fixed`` and ``ir.data`` included) when it compiles; the dump never runs a closure.
"""

from __future__ import annotations

from .corpus_v1 import build_program
from .corpus_v2 import canonical
from .ir_eval import MAX_IR_COST
from .lower import Spec, lower_document, lowering_refusal
from .parser import parse_source

SCHEMA = "warrant-lowering-parity-v1"


def _params(pairs: list) -> list:
    return [[name, str(ty)] for name, ty in pairs]


def _check_lowered(spec: Spec) -> None:
    """Refuse a document with any part the lowerer could not lower (fail closed).

    The refusal is the one the lowerer *recorded*, first in declaration order (section 16.14 R35); the
    evaluation refusal of a unit that lowered and needs ``ir.data`` is not one: the dump shows it.  Every
    level the lowerer can produce (``ir.core``, ``ir.fixed``, ``ir.tuple``, ``ir.data``) is shown; a unit that
    needs any other level did not lower and is a recorded refusal, so nothing else is left to check here
    except a claim's own lowering refusal (``W0701``: a pure-guard or no-subject-call violation, VER04r 4.3, 4.4),
    which the generator also refuses at lowering.
    """
    first = lowering_refusal(spec)
    if first is not None:
        raise first
    for suite in spec.suites:
        if suite.refusal is not None:
            raise suite.refusal


def view(spec_bytes: bytes, max_cost: int = MAX_IR_COST) -> dict:
    """The parity view of one document (see the module docstring)."""
    spec = lower_document(parse_source(spec_bytes))
    _check_lowered(spec)
    prog = build_program(spec, max_cost=max_cost, analyze=True, analyze_data=True)
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
    out = {
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
    if spec.records or spec.variants:
        out["types"] = _types(spec)
    return out


def _types(spec: Spec) -> dict:
    """The corpus ``types`` rows of the records and variants (section 2.2).

    The writer sorts an object's keys (canonical JSON), so the rows come out sorted by type name, not in
    declaration order; the order of the *fields* and *constructors* inside a row is the declared one.
    """
    rows: dict = {}
    for name, fields in spec.records.items():
        rows[name] = {"kind": "record", "fields": [[n, str(t)] for n, t in fields]}
    for name, ctors in spec.variants.items():
        rows[name] = {
            "kind": "variant",
            "ctors": [[c, [str(t) for _, t in fs]] for c, fs in ctors],
        }
    return rows


def dump(spec_bytes: bytes, max_cost: int = MAX_IR_COST) -> bytes:
    """The committed dump of a document: :func:`view` in the canonical writer."""
    return canonical(view(spec_bytes, max_cost))
