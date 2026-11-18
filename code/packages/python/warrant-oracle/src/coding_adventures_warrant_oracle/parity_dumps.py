"""The committed *evaluation* and *fault* dumps of slice S10t-1 (VER04r section 13, 16.10 R11).

S10a committed a **lowering dump** (``lowering_dump``) so that the generator's S10b could reproduce
the oracle's lowering byte for byte.  Slice S10t-1 evaluates ``ir.tuple`` and instantiates the
``ir.tuple`` fault templates, so it commits two more parity views of the synthetic document
``tests/fixtures/tuple-forms-v1/tuple_forms_v1.warrant``, which slice S10t-2 (the Rust generator)
must reproduce byte for byte:

``warrant-evaluation-parity-v1`` (:func:`evaluation_view`)
    what the oracle *evaluates*: per suite its levels (``requires``), binders with their resolved
    domain descriptors, stream header, product, ``cases``, ``discarded``, ``expected_calls``,
    ``wraps``, ``ir_cost``, every chunk line and every digest; the examples, the ``omitted_examples``
    and ``cases.tfv`` (the TFV ``L<n>`` form of a tuple).  It is the parity view of
    ``dump_oracle_enumeration.py`` extended with the tuple-specific rows.
``warrant-fault-parity-v1`` (:func:`faults_view`)
    the ``faults`` rows (the IR JSON of every template, **normative** for the composite faults
    ``ok-as-err``, ``drop-consumed``, ``swap-components`` and the tuple-payload ``missing-error``,
    section 16.10 R11) and the ``controls`` rows twice: ``controls_shipped``, with the fault-evaluation
    gate as shipped (``ir.core`` only: a fault needing ``ir.tuple`` or ``ir.fixed`` has ``deferred``
    witnesses), and ``controls_gate_on`` with ``ir.tuple`` evaluated (the witness search runs:
    ``chunk`` witnesses; a fault that also needs ``ir.fixed`` stays ``deferred``).  The second view
    is what the flip (S10c, [D29]) turns on; until then the generator reaches it only through a
    library parameter used by its tests, exactly as the oracle's ``derive(..., implemented=...)``.

Both are canonical JSON (:func:`.corpus_v2.canonical`) and carry no generator name or version.
"""

from __future__ import annotations

from .corpus_v2 import canonical, chunk_file_name, derive
from .faults import IMPLEMENTED

EVALUATION_SCHEMA = "warrant-evaluation-parity-v1"
FAULT_SCHEMA = "warrant-fault-parity-v1"
#: The gate of ``controls_gate_on``: ``ir.core`` and ``ir.tuple`` evaluated, ``ir.fixed`` not.
GATE_ON = (*IMPLEMENTED, "ir.tuple")
_GENERATOR = {"name": "parity-dump", "version": "0"}

#: Suite-row keys of the evaluation view, in no particular order (the writer sorts).
_SUITE_KEYS = (
    "id",
    "claim",
    "kind",
    "requires",
    "binders",
    "demoted",
    "domain_label",
    "stream",
    "result_type",
    "product",
    "chunks",
    "cases",
    "discarded",
    "expected_calls",
    "wraps",
    "ir_cost",
    "slim_chunks",
    "slim_inputs_sha256",
    "slim_expected_sha256",
    "inputs_sha256",
    "expected_sha256",
    "v1_inputs_sha256",
    "v1_expected_sha256",
)


def evaluation_view(spec_bytes: bytes, profile_raw: bytes) -> dict:
    """The parity view of what the oracle evaluates (see the module docstring)."""
    derived = derive(spec_bytes, profile_raw, spec_ref="tuple_forms_v1.warrant", generator=_GENERATOR)
    corpus = derived.corpus
    suites = {}
    for row in corpus["suites"]:
        view = {key: row[key] for key in _SUITE_KEYS if key in row}
        text = derived.chunk_files[chunk_file_name(row["claim"])].decode("ascii")
        view["chunk_lines"] = text.splitlines()
        suites[row["claim"]] = view
    out = {
        "schema": EVALUATION_SCHEMA,
        "module": corpus["module"],
        "document": corpus["document"],
        "chunk_size": corpus["chunk_size"],
        "budget": corpus["budget"],
        "max_ir_cost": corpus["max_ir_cost"],
        "levels": corpus["levels"],
        "ops": {name: row["requires"] for name, row in corpus["ops"].items()},
        "suites": suites,
        "examples": corpus["examples"],
        "cases_tfv": derived.tfv.decode("utf-8"),
    }
    if "omitted_examples" in corpus:
        out["omitted_examples"] = corpus["omitted_examples"]
    return out


def faults_view(spec_bytes: bytes, profile_raw: bytes) -> dict:
    """The parity view of the fault rows and the controls under both gates (module docstring)."""
    shipped = derive(spec_bytes, profile_raw, spec_ref="tuple_forms_v1.warrant", generator=_GENERATOR)
    on = derive(
        spec_bytes,
        profile_raw,
        spec_ref="tuple_forms_v1.warrant",
        generator=_GENERATOR,
        implemented=GATE_ON,
    )
    assert shipped.corpus["faults"] == on.corpus["faults"]  # the rows never depend on the gate
    return {
        "schema": FAULT_SCHEMA,
        "module": shipped.corpus["module"],
        "faults": shipped.corpus["faults"],
        "controls_shipped": shipped.corpus["controls"],
        "controls_gate_on": on.corpus["controls"],
        "accepted_blind_faults": shipped.corpus["accepted_blind_faults"],
        "levels_shipped": shipped.corpus["levels"],
        "levels_gate_on": on.corpus["levels"],
    }


def dump_evaluation(spec_bytes: bytes, profile_raw: bytes) -> bytes:
    """:func:`evaluation_view` in the canonical writer (what is committed)."""
    return canonical(evaluation_view(spec_bytes, profile_raw))


def dump_faults(spec_bytes: bytes, profile_raw: bytes) -> bytes:
    """:func:`faults_view` in the canonical writer (what is committed)."""
    return canonical(faults_view(spec_bytes, profile_raw))
