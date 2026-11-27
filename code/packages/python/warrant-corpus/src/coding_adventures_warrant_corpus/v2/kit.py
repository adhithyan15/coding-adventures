"""The conformance harness for the v2 kit (VER04r section 11, gate [D7]).

``code/specs/fixtures/warrant-corpus-conformance-v1`` holds, besides the four v1
categories, the **v2 kit**: six JSON files of cases (``v2-value``, ``v2-arith``,
``v2-ir``, ``v2-domain``, ``v2-chunk``, ``v2-fault``) that every lane runner must
pass before it may write a manifest.  This module runs them through this lane's
*own* library -- the decoder, evaluator, descriptors, chunk enumerator and fault
wrappers of this package -- and reports, per category::

    run      cases whose ``requires`` this runner implements, and which it ran
    skipped  cases that need a level this runner lacks (never silently dropped)
    failed   the ids of the cases whose outcome differed from ``expect``

The accounting rule of section 11: **per category, run plus skipped equals the
index row's** ``cases``, so a case is never lost and a kit that grows by a level
does not break a runner that lacks it.  A category this runner does not know
(``v2-binding`` before slice S6b) is digest-checked and skipped whole.

A lane runs a case **if and only if** its ``requires`` is a subset of the levels
the runner implements.  Each case's ``expect`` is the *kit's* outcome object; the
lane maps its own failures onto the kit's closed codes (section 11, R25), which is
exactly what :class:`~.errors.Refusal` already is.

The harness takes a :class:`~.semantics.Semantics`.  Handed the standard one it
must report zero failures; handed a deliberately wrong one it must report the
ids of the cases that catch the mistake -- that is the falsification of slice S6a
([D42]), exercised by ``tests/test_v2_falsification.py``.  A harness that cannot
fail would prove nothing.

Failure details name the case id and the *outcome*; they never print input bytes
(VER00 section 7.3).
"""

from __future__ import annotations

import hashlib
import json
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path

from ..errors import CorpusError
from ..jsonio import parse_json, read_bytes
from . import chunks as chunk_lib
from .chunks import schema_index
from .domains import descriptor_for
from .errors import IrOverflow, KitError, Refusal
from .faults import apply_fault
from .ir import IMPLEMENTED_LEVELS, Program, SubjectTable, calls_of, evaluate
from .semantics import STANDARD, Semantics
from .values import (
    decode_value,
    encode_value,
    make_encoder,
    read_json,
    value_to_json,
)
from .wtypes import (
    check_types_table,
    is_undetermined,
    parse_type,
)

STREAM_HEX_LIMIT = 2048
MAX_FIXTURE_BYTES = 32 * 1024 * 1024


# ---------------------------------------------------------------------------
# The fixture family
# ---------------------------------------------------------------------------
@dataclass
class Family:
    """The fixture family on disk: the index rows and every v2 category's cases."""

    directory: Path
    index: dict
    cases: dict[str, list[dict]]

    def indexed(self, category: str) -> int:
        return next(
            r["cases"] for r in self.index["files"] if r["category"] == category
        )


def load_family(directory: Path) -> Family:
    """Read ``index.json`` and every ``v2-*`` file, checking digests and counts.

    Each row's ``sha256`` must match its file's bytes and its ``cases`` the number
    of cases in it, and ``total_cases`` must be the sum: a runner never accepts a
    stale or edited kit.  The files are read with this lane's strict reader.
    """
    index = parse_json(read_bytes(directory / "index.json", MAX_FIXTURE_BYTES))
    if not isinstance(index, dict) or not isinstance(index.get("files"), list):
        raise CorpusError("conformance", "index.json is malformed")
    cases: dict[str, list[dict]] = {}
    total = 0
    for row in index["files"]:
        total += row["cases"]
        if not row["category"].startswith("v2-"):
            continue
        raw = read_bytes(directory / row["path"], MAX_FIXTURE_BYTES)
        if hashlib.sha256(raw).hexdigest() != row["sha256"]:
            raise CorpusError(
                "conformance", f"{row['path']} does not match its index digest"
            )
        document = parse_json(raw)
        assert isinstance(document, dict)
        if len(document["cases"]) != row["cases"]:
            raise CorpusError("conformance", f"{row['path']} has the wrong case count")
        cases[row["category"]] = document["cases"]
    if total != index.get("total_cases"):
        raise CorpusError("conformance", "total_cases is not the sum of the rows")
    return Family(directory, index, cases)


def kit_hex(text: object, what: str) -> bytes:
    """The bytes of a hex field of a case; a field that is not hex is a *kit* error."""
    try:
        return bytes.fromhex(text)  # type: ignore[arg-type]
    except (ValueError, TypeError) as error:
        raise KitError(f"{what} is not hex") from error


# ---------------------------------------------------------------------------
# The per-category runners: each returns the kit's outcome object
# ---------------------------------------------------------------------------
def outcome_value(case: dict, sem: Semantics) -> dict:
    """``v2-value``: strict reader, then the section 2.3 decoder, then the 2.4 encoder.

    The order of the codes (section 11): ``malformed-json`` (1), ``type`` (2),
    ``value`` (3).  An accepted value reports its stream bytes (no header, no
    context) and their SHA-256; the hex is left out when longer than 2,048 digits.
    """
    types = case["types"]
    try:
        raw = read_json(kit_hex(case["input_hex"], "input_hex"))
        check_types_table(types, "type")  # section 2.2: every consumer enforces it
        ty = parse_type(case["type"], types)
        value = decode_value(raw, ty, types, sem)
    except Refusal as refusal:
        return {"outcome": "reject", "error": refusal.code}
    stream = encode_value(value, ty, types, sem)
    out = {"outcome": "accept", "stream_sha256": hashlib.sha256(stream).hexdigest()}
    if len(stream.hex()) <= STREAM_HEX_LIMIT:
        out["stream_hex"] = stream.hex()
    return out


def outcome_arith(case: dict, sem: Semantics) -> dict:
    """``v2-arith``: one evaluator node over ``lit`` operands of the case type."""
    operands = [case["lhs"]] if case["op"] == "neg" else [case["lhs"], case["rhs"]]
    ir = [case["op"]] + [
        ["lit", {"type": case["type"], "value": text}] for text in operands
    ]
    try:
        compiled = Program(sem=sem).compile(ir, {})
        value = evaluate(compiled, {})
    except IrOverflow:
        return {"error": "ir-overflow"}
    except Refusal as refusal:
        raise KitError(f"a v2-arith node is refused: {refusal}") from refusal
    return {
        "type": str(compiled.ty),
        "value": value if isinstance(value, bool) else str(value),
    }


def outcome_ir(case: dict, sem: Semantics) -> dict:
    """``v2-ir``: check and evaluate a tree: ``{type, value, calls}`` or an error.

    ``subjects`` names, per op, the pure model that stands in for the subject, so
    ``calls`` is *measured* by the dispatch wrapper.  It is also compared with the
    pure trace of section 4.4 (computed statically from the tree): the two come
    from different code paths and must agree.
    """
    types = case["types"]
    try:
        program = Program(
            types=types,
            ops=case["ops"],
            models=case["models"],
            model_order=case["model_order"],
            levels=frozenset(case["levels"]),
            max_ir_cost=case.get("max_ir_cost", 2000),
            sem=sem,
        )
        # The environment is part of what the expression reaches (R69): its types
        # are read first and checked for levels by the compile, its values after.
        scope: dict = {}
        for entry in case["env"]:
            if type(entry) is not list or len(entry) != 2 or type(entry[0]) is not str:
                raise KitError("an env entry is not a [name, {type, value}] pair")
            name, tagged = entry
            # R120: an entry that is not {"type": text, "value": V} breaks the
            # case's own schema (a kit error); any refusal of the type TEXT is
            # `ill-typed` -- `Int`, an undeclared name, `Result<U8,Q>` with `Q` a
            # record, an error type (R117), a malformed text: v2-ir has no
            # `type` code, so the refusal is the typing of the case.
            if (
                type(tagged) is not dict
                or "value" not in tagged
                or type(tagged.get("type")) is not str
            ):
                raise KitError("an env entry is not {type: text, value: V}")
            try:
                scope[name] = parse_type(tagged["type"], types)
            except Refusal as refusal:
                raise Refusal(
                    "ill-typed", f"an env type is refused: {refusal}"
                ) from refusal
        compiled = program.compile(case["ir"], scope, role=case.get("role", "body"))
        env = {}
        for name, tagged in case["env"]:
            try:
                env[name] = decode_value(tagged["value"], scope[name], types, sem)
            except Refusal as refusal:
                raise KitError(f"an env value is malformed: {refusal}") from refusal
        table = SubjectTable(program, case["subjects"])
        value = evaluate(compiled, env)
    except IrOverflow as overflow:
        return {"error": "ir-overflow", "class": overflow.klass}
    except Refusal as refusal:
        return {"error": refusal.code}
    if is_undetermined(compiled.ty):
        raise KitError("R41: a kit case has an undetermined result type")
    out = {
        "type": str(compiled.ty),
        "value": value_to_json(value, compiled.ty, types),
        "calls": table.calls,
    }
    traced = calls_of(compiled.calls, env)
    if table.calls != traced:
        # Two code paths count the same thing and disagree: report both numbers,
        # so the comparison with `expect` fails and names them.
        out["traced_calls"] = traced
    return out


def _domain_window(case: dict) -> tuple[int, int]:
    """The half-open window ``[lo, hi)`` of indices a ``v2-domain`` case asks about.

    A case asks either about one ``index`` (the window ``[i, i+1)``) or about a
    ``range`` ``[lo, hi]`` read half-open.  Either value is a JSON integer that
    is not negative: a case that says otherwise has broken its own schema, which
    is a defect of the fixture and never an outcome (R45).  A ``range`` whose
    ``lo`` is above ``hi`` is not a broken schema -- it is the ``index`` outcome --
    and is returned as it stands for the caller to judge against ``size``.
    """
    if "index" in case:
        index = schema_index(case["index"], "index")
        return index, index + 1
    bounds = case["range"]
    if not isinstance(bounds, list) or len(bounds) != 2:
        raise KitError("range is a pair [lo, hi]")
    low, high = (schema_index(b, "a range bound") for b in bounds)
    return low, high


def outcome_domain(case: dict, sem: Semantics) -> dict:
    """``v2-domain``: ``size()`` and ``nth()`` at an ``index`` or over a ``range``.

    The order of the codes (section 11): ``descriptor`` (1) -- the tree is not a
    valid descriptor for the type -- then ``index`` (2): the window is not inside
    ``0..size`` (or ``lo > hi``).  An accepted window reports ``size`` and the
    section 2.4 bytes of each value in it, in index order.
    """
    low, high = _domain_window(case)
    types = case.get("types", {})
    try:
        if "types" in case:
            # R114: a present table is read whole, as `descriptor`, even `null`
            # or `[]` (only an absent one means `{}`).
            check_types_table(types, "descriptor")
        desc, ty = descriptor_for(case["descriptor"], case["type"], types, sem)
    except Refusal as refusal:
        return {"error": refusal.code}
    if low > high or high > desc.size:
        return {"error": "index"}
    encode = make_encoder(ty, types, sem)
    return {
        "size": desc.size,
        "values_hex": [encode(desc.nth(i)).hex() for i in range(low, high)],
    }


def outcome_chunk(case: dict, sem: Semantics) -> dict:
    """``v2-chunk``: validate corpus part, row and chunk file; run one chunk alone."""
    try:
        return chunk_lib.validate_and_run(
            case["corpus"],
            case["suite"],
            kit_hex(case["chunk_file_hex"], "chunk_file_hex"),
            case["chunk"],
            sem,
        )
    except Refusal as refusal:
        return {"error": refusal.code}


def outcome_fault(case: dict, sem: Semantics) -> dict:
    """``v2-fault``: the pre, post or replace wrapper over one call."""
    try:
        return apply_fault(
            case["fault"],
            case["op"],
            case["types"],
            frozenset(case["levels"]),
            case["args"],
            case["result"],
            sem,
        )
    except Refusal as refusal:
        return {"error": refusal.code}


RUNNERS: dict[str, Callable[[dict, Semantics], dict]] = {
    "v2-value": outcome_value,
    "v2-arith": outcome_arith,
    "v2-ir": outcome_ir,
    "v2-domain": outcome_domain,
    "v2-chunk": outcome_chunk,
    "v2-fault": outcome_fault,
}


# ---------------------------------------------------------------------------
# Running the family
# ---------------------------------------------------------------------------
@dataclass
class CategoryReport:
    """What happened to one category: counts, and the ids behind each count."""

    category: str
    indexed: int
    run: int = 0
    skipped: list[str] = field(default_factory=list)
    failed: dict[str, str] = field(default_factory=dict)

    @property
    def accounted(self) -> bool:
        """Run plus skipped equals the index row's count (section 11)."""
        return self.run + len(self.skipped) == self.indexed

    @property
    def ok(self) -> bool:
        return self.accounted and not self.failed


@dataclass
class KitReport:
    """The report of one harness run, per category."""

    categories: dict[str, CategoryReport] = field(default_factory=dict)

    @property
    def ok(self) -> bool:
        return all(c.ok for c in self.categories.values())

    @property
    def failed_ids(self) -> list[str]:
        return [i for c in self.categories.values() for i in c.failed]

    def lines(self) -> list[str]:
        """One line per category, then one per failure: ``not ok <id>: <detail>``."""
        out = [
            f"{c.category}: run {c.run}, skipped {len(c.skipped)}, "
            f"failed {len(c.failed)} (indexed {c.indexed})"
            for c in self.categories.values()
        ]
        out += [
            f"not ok {i}: {d}"
            for c in self.categories.values()
            for i, d in c.failed.items()
        ]
        return out


def canonical(value: object) -> str:
    """Canonical JSON text, so that ``true`` and ``1`` are not the same outcome."""
    return json.dumps(value, sort_keys=True, separators=(",", ":"))


def run_kit(
    directory: Path,
    *,
    levels: frozenset[str] | set[str] = IMPLEMENTED_LEVELS,
    sem: Semantics = STANDARD,
    categories: tuple[str, ...] | None = None,
    sequential: bool = True,
    only: Callable[[dict], bool] | None = None,
) -> KitReport:
    """Run every v2 kit case whose ``requires`` is a subset of ``levels``.

    ``categories`` restricts the run (a mutant that only breaks arithmetic need
    not enumerate chunks); ``only`` restricts the cases within a category.  A case
    outside the restriction is *not counted at all*, so the accounting rule applies
    to a full run only.  ``sequential`` also walks chunks ``0..=c`` in sequence for
    each ``v2-chunk`` case marked ``also_sequential`` and requires the same result
    as chunk ``c`` alone.
    """
    family = load_family(directory)
    report = KitReport()
    walks = _SequentialWalks(sem) if sequential else None
    for category, cases in family.cases.items():
        if categories is not None and category not in categories:
            continue
        cat = report.categories[category] = CategoryReport(
            category, family.indexed(category)
        )
        runner = RUNNERS.get(category)
        if walks is not None:
            walks.plan(cases)
        for case in cases:
            if only is not None and not only(case):
                cat.indexed -= 1
                continue
            if runner is None or not set(case["requires"]) <= set(levels):
                cat.skipped.append(case["id"])
                continue
            cat.run += 1
            problem = _run_case(case, runner, sem, walks)
            if problem is not None:
                cat.failed[case["id"]] = problem
    return report


def _run_case(
    case: dict,
    runner: Callable[[dict, Semantics], dict],
    sem: Semantics,
    walks: _SequentialWalks | None,
) -> str | None:
    """Run one case; return ``None`` on success or a one-line reason."""
    try:
        got = runner(case, sem)
        want = case["expect"]
        if canonical(got) != canonical(want):
            return f"outcome {_short(got)} differs from expect {_short(want)}"
        if walks is not None and case.get("also_sequential"):
            problems = walks.check(case)
            if problems:
                return "; ".join(problems)
    except KitError as error:
        return f"kit error: {error}"
    except Exception as error:  # noqa: BLE001 - a wrong evaluator may crash anywhere
        return f"crashed with {type(error).__name__}"
    return None


def _short(outcome: dict) -> str:
    text = canonical(outcome)
    return text if len(text) <= 160 else text[:157] + "..."


class _SequentialWalks:
    """ "Chunk ``c`` alone equals sequential", shared across the cases of one suite.

    Walking chunks ``0..=c`` is the expensive part (the 2^24-ordinal suite of the
    kit is 16.7 million cases).  The three cases over that suite ask for chunks 0,
    128 and 255, so the walk is done **once**, to the largest chunk any case asks
    for, and each case then compares its own chunk (and every earlier one) with
    the walk.
    """

    def __init__(self, sem: Semantics) -> None:
        self.sem = sem
        self._walks: dict[str, list[chunk_lib.ChunkResult]] = {}
        self._deepest: dict[str, int] = {}

    def plan(self, cases: list[dict]) -> None:
        """Record the deepest chunk asked for per suite, before the first walk."""
        for case in cases:
            if case.get("also_sequential"):
                key = case["suite"]["chunks_sha256"]
                self._deepest[key] = max(self._deepest.get(key, 0), case["chunk"])

    def check(self, case: dict) -> list[str]:
        suite = chunk_lib.build_suite(case["corpus"], case["suite"], self.sem)
        lines = chunk_lib.check_chunk_file(
            kit_hex(case["chunk_file_hex"], "chunk_file_hex"), suite, self.sem
        )
        key = case["suite"]["chunks_sha256"]
        last = case["chunk"]
        if key not in self._walks:
            self._walks[key] = chunk_lib.run_sequential(
                suite, max(self._deepest.get(key, last), last), self.sem
            )
        return chunk_lib.sequential_disagreements(
            suite, lines, last, self.sem, self._walks[key]
        )
