"""Loading and validating a ``warrant-corpus-v1`` directory.

The corpus is produced by the Rust compiler (``warrant-grammar``) and is
*untrusted input* here.  Loading it is therefore a long list of "this must be
exactly so", each failing closed:

1. the files are size-capped, regular, strictly parsed (no duplicate keys);
2. every object has exactly the expected keys (a new field is a new profile,
   not something to skip);
3. ``spec_sha256`` is re-derived from the ``.warrant`` file on disk
   (``StaleEvidence`` caught at run time, VER04i 6 step 1);
4. each negative control is **re-derived** from the fault registry, the op
   signatures and the suites that mention the op, and must equal the corpus's
   row -- a corpus cannot quietly omit a control or point it at the wrong
   claims;
5. ``cases.tfv`` is cross-checked, record by record, against the examples in
   ``corpus.json``: two encodings of the same facts must agree.

Nothing here *executes* the library; that is :mod:`.runner`.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from pathlib import Path

from .domains import binder_domain
from .errors import CorpusError, need
from .faults import FAULTS
from .jsonio import MAX_CORPUS_BYTES, MAX_TEXT_BYTES, parse_json, read_bytes, sha256_hex
from .tfv import parse_tfv
from .values import OpSig, Type, decode_json, decode_tfv, parse_type

PROFILE = "warrant-corpus-v1"

# Work limits (VER04i section 3; the security review of WD-101).  The corpus is
# untrusted, so the Cartesian product of a suite's binder domains must equal
# the cases it declares (admitted + discarded) and stay under a cap *before*
# any case runs.  The real ct-compare corpus needs 240,818 at most per suite
# and 812,927 for one clean pass; with every negative control re-running its
# target suites the total is about 2.1 million.
MAX_SUITE_PRODUCT = 1_000_000  # tuples enumerated by one suite
MAX_TOTAL_WORK = 8_000_000  # all suites once, plus every control's re-runs
MAX_BINDERS = 8
_HEX64 = re.compile(r"[0-9a-f]{64}")
_IDENT = re.compile(r"[a-z][a-z0-9_]*")
_TOP_KEYS = {
    "schema_version", "profile", "module", "spec", "spec_sha256", "generator",
    "types", "ops", "models", "examples", "suites", "faults", "controls",
}  # fmt: skip
_SUITE_KEYS = {
    "id", "kind", "claim", "ops", "binders", "admit", "lhs", "rhs", "cases",
    "discarded", "expected_calls", "domain_text", "inputs_sha256",
    "expected_sha256", "result_type",
}  # fmt: skip


@dataclass(frozen=True)
class Example:
    """One ``example`` from the spec: call ``op`` on ``args``, expect ``expected``."""

    id: str
    op: str
    args: tuple[object, ...]
    expected: object
    width: int | None


@dataclass(frozen=True)
class Suite:
    """One exhaustive suite: a claim over a finite, enumerable domain."""

    id: str
    claim: str
    ops: tuple[str, ...]
    binders: tuple[tuple[str, str], ...]  # (name, domain descriptor)
    binder_types: tuple[str, ...]
    admit: object
    lhs: object
    rhs: object
    result_type: str
    cases: int
    discarded: int
    expected_calls: int
    domain_text: str
    inputs_sha256: str
    expected_sha256: str


@dataclass(frozen=True)
class Control:
    """One generated negative control (suite kind, op, fault)."""

    id: str
    fault: str
    op: str
    target_kind: str
    required_claims: tuple[str, ...]


@dataclass
class Corpus:
    """A validated corpus, ready to run."""

    module: str
    spec: str
    spec_sha256: str
    corpus_sha256: str
    cases_sha256: str
    errors: dict[str, list[str]]
    ops: dict[str, OpSig]
    models: dict[str, object]
    examples: list[Example]
    suites: list[Suite]
    controls: list[Control]
    result_types: dict[str, Type] = field(default_factory=dict)


def _exact(obj: object, keys: set[str], where: str) -> dict[str, object]:
    need(
        type(obj) is dict and set(obj) == keys,
        "schema",
        f"{where} has unexpected fields",
    )
    assert isinstance(obj, dict)
    return obj


def _text(value: object, where: str) -> str:
    need(
        type(value) is str and bool(value),
        "schema",
        f"{where} must be a non-empty string",
    )
    assert isinstance(value, str)
    return value


def _count(value: object, where: str) -> int:
    need(
        type(value) is int and 0 <= value < 1 << 53,
        "schema",
        f"{where} must be a count",
    )
    assert isinstance(value, int)
    return value


def _digest(value: object, where: str) -> str:
    need(
        type(value) is str and _HEX64.fullmatch(value) is not None,
        "schema",
        f"{where} must be a lowercase SHA-256",
    )
    assert isinstance(value, str)
    return value


def _repo_path(root: Path, relative: str) -> Path:
    """Resolve a repository-relative path, refusing anything that escapes."""
    need(
        "\\" not in relative
        and not relative.startswith("/")
        and all(part not in ("", ".", "..") for part in relative.split("/")),
        "schema",
        "corpus path must be a normalised repository-relative path",
    )
    return root / relative


def _signatures(raw_ops: object) -> dict[str, OpSig]:
    need(
        type(raw_ops) is dict and bool(raw_ops),
        "schema",
        "ops must be a non-empty object",
    )
    assert isinstance(raw_ops, dict)
    ops: dict[str, OpSig] = {}
    for name, row in raw_ops.items():
        need(_IDENT.fullmatch(name) is not None, "schema", "bad op name")
        need(
            type(row) is dict
            and {"params", "ret"} <= set(row)
            and set(row) <= {"params", "ret", "width_params", "fresh"},
            "schema",
            "malformed op signature",
        )
        need(type(row["params"]) is list, "schema", "op params must be a list")
        params = []
        for pair in row["params"]:
            need(
                type(pair) is list
                and len(pair) == 2
                and type(pair[0]) is str
                and type(pair[1]) is str,
                "schema",
                "malformed op parameter",
            )
            params.append((pair[0], parse_type(pair[1])))
        widths = row.get("width_params", [])
        need(
            type(widths) is list and all(type(w) is str for w in widths),
            "schema",
            "bad widths",
        )
        ops[name] = OpSig(
            name,
            tuple(params),
            parse_type(_text(row["ret"], "op ret")),
            tuple(widths),
            row.get("fresh", False) is True,
        )
    return ops


def _decode_example(
    row: object, ops: dict[str, OpSig], errors: dict[str, list[str]], module: str
) -> Example:
    row = _exact(
        row, {"args", "expected", "id", "op", "result_type", "width"}, "example"
    )
    example_id = _text(row["id"], "example id")
    need(
        example_id.startswith(f"warrant/{module}/example/"),
        "schema",
        "example id is outside the module",
    )
    op = _text(row["op"], "example op")
    need(op in ops, "example", "example names an unknown op")
    sig = ops[op]
    args = row["args"]
    need(
        type(args) is list and len(args) == len(sig.params),
        "example",
        "example arity mismatch",
    )
    need(row["result_type"] == sig.ret.text, "example", "example result type disagrees")
    width = row["width"]
    if sig.width_params:
        need(
            type(width) is int and width >= 0,
            "example",
            "fixed-width example lacks a width",
        )
    else:
        need(width is None, "example", "unexpected example width")
    values = []
    for raw, (_name, ty) in zip(args, sig.params, strict=True):
        if isinstance(ty.width, str):
            # `Bytes<N>`: the tag keeps the literal parameter name and the
            # actual length must equal the example's static width.
            value = decode_json(raw, ty, errors)
            need(len(value) == width, "example", "example buffer is not N bytes")  # type: ignore[arg-type]
        else:
            value = decode_json(raw, ty, errors)
        values.append(value)
    return Example(
        example_id, op, tuple(values), decode_json(row["expected"], sig.ret, errors),
        width if isinstance(width, int) else None,
    )  # fmt: skip


def _decode_suite(row: object, ops: dict[str, OpSig], module: str) -> Suite:
    row = _exact(row, _SUITE_KEYS, "suite")
    claim = _text(row["claim"], "suite claim")
    suite_id = _text(row["id"], "suite id")
    need(
        suite_id == f"warrant/{module}/exhaustive/{claim}"
        and row["kind"] == "exhaustive",
        "schema",
        "suite id does not follow the claim",
    )
    suite_ops = row["ops"]
    need(
        type(suite_ops) is list
        and bool(suite_ops)
        and all(type(o) is str and o in ops for o in suite_ops),
        "schema",
        "suite names an unknown op",
    )
    binders = row["binders"]
    need(
        type(binders) is list and 0 < len(binders) <= MAX_BINDERS,
        "schema",
        "suite needs between 1 and MAX_BINDERS binders",
    )
    names, descriptors, types = [], [], []
    product = 1
    for pair in binders:
        need(
            type(pair) is list and len(pair) == 2 and type(pair[0]) is str,
            "schema",
            "bad binder",
        )
        values, produced = binder_domain(pair[1])
        product *= len(values)
        need(pair[0] not in names, "schema", "duplicate binder name")
        names.append(pair[0])
        descriptors.append(pair[1]["domain"])
        types.append(produced)
    parse_type(_text(row["result_type"], "suite result type"))
    cases = _count(row["cases"], "cases")
    discarded = _count(row["discarded"], "discarded")
    need(
        cases > 0, "schema", "a suite must admit at least one case (no vacuous suites)"
    )
    need(
        product == cases + discarded,
        "work",
        "declared cases + discarded differ from the binder domains' product",
    )
    need(product <= MAX_SUITE_PRODUCT, "work", "suite exceeds the per-suite work cap")
    return Suite(
        id=suite_id,
        claim=claim,
        ops=tuple(suite_ops),
        binders=tuple(zip(names, descriptors, strict=True)),
        binder_types=tuple(types),
        admit=row["admit"],
        lhs=row["lhs"],
        rhs=row["rhs"],
        result_type=row["result_type"],
        cases=cases,
        discarded=discarded,
        expected_calls=_count(row["expected_calls"], "expected_calls"),
        domain_text=_text(row["domain_text"], "domain_text"),
        inputs_sha256=_digest(row["inputs_sha256"], "inputs_sha256"),
        expected_sha256=_digest(row["expected_sha256"], "expected_sha256"),
    )


def _derive_controls(
    module: str, faults: object, suites: list[Suite], ops: dict[str, OpSig]
) -> list[Control]:
    """Re-derive the controls the corpus *must* contain (VER04i 4.1)."""
    need(
        type(faults) is list and bool(faults),
        "schema",
        "faults must be a non-empty list",
    )
    assert isinstance(faults, list)
    derived: list[Control] = []
    seen: set[tuple[str, str]] = set()
    for row in faults:
        row = _exact(row, {"applies", "fault", "op"}, "fault")
        name, op = _text(row["fault"], "fault"), _text(row["op"], "fault op")
        need(
            name in FAULTS, "fault", "corpus names a fault the lane does not implement"
        )
        need(
            op in ops and FAULTS[name].applies(ops[op]),
            "fault",
            "fault does not apply to op",
        )
        need(row["applies"] in ("input", "output"), "fault", "bad fault stage")
        need((name, op) not in seen, "fault", "duplicate fault row")
        seen.add((name, op))
        required = tuple(s.id for s in suites if op in s.ops)
        need(bool(required), "fault", "no claim can catch a registered fault")
        derived.append(
            Control(
                f"warrant/{module}/negctl/exhaustive/{op}/{name}",
                name,
                op,
                "exhaustive",
                required,
            )
        )
    # The compiler lists controls sorted by id; so must the corpus.
    return sorted(derived, key=lambda control: control.id)


def _check_tfv(
    text: str,
    examples: list[Example],
    ops: dict[str, OpSig],
    errors: dict[str, list[str]],
) -> None:
    """Cross-check ``cases.tfv`` against ``corpus.json``'s examples."""
    records = parse_tfv(text)
    need(
        len(records) == len(examples),
        "tfv",
        "TFV record count differs from the examples",
    )
    for (record_id, fields), example in zip(records, examples, strict=True):
        need(record_id == example.id, "tfv", "TFV example order or identity differs")
        sig = ops[example.op]
        expected_keys = {"op", "expect", *(name for name, _ty in sig.params)}
        if sig.width_params:
            expected_keys.add("width")
        need(
            set(fields) == expected_keys,
            "tfv",
            "TFV fields disagree with the signature",
        )
        need(
            fields["op"] == ("text", example.op),
            "tfv",
            "TFV op differs from the example",
        )
        for (name, ty), value in zip(sig.params, example.args, strict=True):
            need(
                decode_tfv(fields[name], ty, errors) == value,
                "tfv",
                "TFV argument differs",
            )
        if sig.width_params:
            need(fields["width"] == ("int", example.width), "tfv", "TFV width differs")
        need(
            decode_tfv(fields["expect"], sig.ret, errors) == example.expected,
            "tfv",
            "TFV expected value differs",
        )


def load_corpus(repo_root: Path, corpus_dir: Path) -> Corpus:
    """Read, validate and decode the corpus in ``corpus_dir``.

    Every malformation is a :class:`CorpusError` (exit status 2).  The explicit
    checks cover the shapes we thought of; the guard below turns any shape we
    did not (a list where an object belongs, a number where a string belongs)
    into the same refusal instead of a ``TypeError`` that would exit like a
    verdict failure.
    """
    try:
        return _load_corpus(repo_root, corpus_dir)
    except (TypeError, KeyError, AttributeError, IndexError, ValueError) as error:
        raise CorpusError("schema", "corpus has a malformed field type") from error


def _load_corpus(repo_root: Path, corpus_dir: Path) -> Corpus:
    corpus_bytes = read_bytes(corpus_dir / "corpus.json", MAX_CORPUS_BYTES)
    cases_bytes = read_bytes(corpus_dir / "cases.tfv", MAX_TEXT_BYTES)
    root = _exact(parse_json(corpus_bytes), _TOP_KEYS, "corpus")
    need(
        root["schema_version"] == 1 and root["profile"] == PROFILE,
        "schema",
        "wrong profile",
    )
    module = _text(root["module"], "module")
    need(_IDENT.fullmatch(module) is not None, "schema", "module must be an identifier")

    spec = _text(root["spec"], "spec")
    spec_bytes = read_bytes(_repo_path(repo_root, spec), MAX_TEXT_BYTES)
    spec_sha256 = _digest(root["spec_sha256"], "spec_sha256")
    need(
        sha256_hex(spec_bytes) == spec_sha256,
        "stale-evidence",
        "spec_sha256 does not match the Warrant spec on disk",
    )

    raw_types = root["types"]
    need(type(raw_types) is dict, "schema", "types must be an object")
    assert isinstance(raw_types, dict)
    errors = {name: list(variants) for name, variants in raw_types.items()}
    for name, variants in errors.items():
        need(
            re.fullmatch(r"[A-Z][A-Za-z0-9_]*", name) is not None
            and all(type(v) is str for v in variants),
            "schema",
            "malformed error type",
        )
    ops = _signatures(root["ops"])
    raw_models = root["models"]
    need(type(raw_models) is dict, "schema", "models must be an object")
    assert isinstance(raw_models, dict)

    raw_examples = root["examples"]
    need(type(raw_examples) is list, "schema", "examples must be a list")
    assert isinstance(raw_examples, list)
    examples = [_decode_example(row, ops, errors, module) for row in raw_examples]
    need(
        len({e.id for e in examples}) == len(examples), "schema", "duplicate example id"
    )

    raw_suites = root["suites"]
    need(
        type(raw_suites) is list and bool(raw_suites), "schema", "suites must be a list"
    )
    assert isinstance(raw_suites, list)
    suites = [_decode_suite(row, ops, module) for row in raw_suites]
    need(len({s.id for s in suites}) == len(suites), "schema", "duplicate suite id")

    controls = _derive_controls(module, root["faults"], suites, ops)
    raw_controls = root["controls"]
    need(type(raw_controls) is list, "schema", "controls must be a list")
    assert isinstance(raw_controls, list)
    for row in raw_controls:
        need(
            type(row) is dict
            and type(row.get("required_claims")) is list
            and all(type(claim) is str for claim in row["required_claims"]),
            "schema",
            "control required_claims must be a list of strings",
        )
    actual = [
        Control(
            c["id"], c["fault"], c["op"], c["target_kind"], tuple(c["required_claims"])
        )
        for c in (
            _exact(
                row, {"fault", "id", "op", "required_claims", "target_kind"}, "control"
            )
            for row in raw_controls
        )
    ]
    need(
        actual == controls,
        "control",
        "controls differ from those derived from the faults",
    )

    by_id = {s.id: s.cases + s.discarded for s in suites}
    total = sum(by_id.values()) + sum(
        by_id[claim] for control in controls for claim in control.required_claims
    )
    need(
        total <= MAX_TOTAL_WORK,
        "work",
        "total work (suites plus control re-runs) exceeds the work cap",
    )
    _check_tfv(cases_bytes.decode("utf-8", errors="strict"), examples, ops, errors)
    return Corpus(
        module=module,
        spec=spec,
        spec_sha256=spec_sha256,
        corpus_sha256=sha256_hex(corpus_bytes),
        cases_sha256=sha256_hex(cases_bytes),
        errors=errors,
        ops=ops,
        models=raw_models,
        examples=examples,
        suites=suites,
        controls=controls,
    )


def list_control_rows(corpus_dir: Path) -> list[str]:
    """``<fault>:<op>`` for each control, read from ``corpus.json`` alone.

    Used by ``list-faults``; it does not need the spec or the library.
    """
    root = parse_json(read_bytes(corpus_dir / "corpus.json", MAX_CORPUS_BYTES))
    need(
        type(root) is dict and type(root.get("controls")) is list,
        "schema",
        "no controls",
    )
    assert isinstance(root, dict)
    rows = []
    for control in root["controls"]:
        need(type(control) is dict, "schema", "bad control")
        rows.append(f"{control['fault']}:{control['op']}")
    return rows
