#!/usr/bin/env python3
"""Integrity checker for the VER13 assumption catalogue (rule AC-14 and friends).

Why this file exists
====================

Every proof label in this repository is an assume-guarantee statement: *if*
the listed assumptions hold, *then* the claim holds (VER00 section 7.5.1).
VER13 turns the assumption lists from scattered strings into a catalogue that
reports cite by id.  A catalogue that nothing checks rots in exactly the way
the strings did, so this script is the guard on the catalogue *file itself*.

Two artefacts are kept in lock-step:

* ``code/specs/VER13-assumption-catalogue.md`` section 3 is the human
  rendering (84 rows in tables);
* ``code/assumptions/catalogue-v1.json`` is the data that reports pin by
  SHA-256 (``ca-assurance-report`` enforces the report-side rules, AS-001a
  part 2).

Two companion files sit beside it and are validated here too:

* ``code/assumptions/evidence-kinds.json`` maps each evidence kind a catalogue
  entry names (``discharge.evidence_kinds``) to the manifest that can hold it
  and the evidence-id patterns of that kind.  Both directions are closed: every
  kind the catalogue names is defined, every defined kind is named, and no two
  patterns can match the same id.
* ``code/assumptions/non-claims.json`` lists properties a package declares it
  does *not* provide (rule AC-12).

What is checked here (catalogue-only rules, no report needed)
=============================================================

=======  ====================================================================
Rule     What it rejects
=======  ====================================================================
AC-14    duplicate ids; ``depends_on`` / ``refines`` that do not resolve or
         that form a cycle; a ``discharge.backlog`` that names no row of
         ``VER-BACKLOG.md``; any drift between spec section 3 and the JSON
         (ids, counts, layer, family, tier, difficulty, statement, ...).
AC-8     catalogue-side shape: ``status: floor`` or ``kind: declared``
         entries carry ``discharge: null`` and may never be ``discharged``.
AC-3     catalogue-side shape: ``status: discharged`` needs ``discharged_by``
         evidence with a digest; any other status must not carry it.
AC-11    catalogue-side shape: an attestation entry owns exactly one
         ``unverified::`` alias, so the stale-attestation rule can find it.
AC-15    an ``environment``-kind or ``spec.*`` entry must not constrain a
         ``hostile`` input class.
AC-14    (pins) every committed ``code/packages/*/*/assurance_report.json``
         pins the current digests of all three files.  This runs on every PR,
         so a catalogue edit cannot leave a report stale until the next PR
         that happens to rebuild that package.
=======  ====================================================================

Rules AC-1..AC-13 need a report and are enforced by
``ca-assurance-report --check`` (``code/packages/rust/ca-assurance-report``).

Fail closed
===========

Malformed input never "mostly works": duplicate JSON object keys, unknown
fields, wrong types and out-of-vocabulary strings are all errors.  The
structural validation is hand-written (no ``jsonschema`` dependency in CI) and
mirrors ``code/specs/schemas/ver13-assumption-catalogue-v1.schema.json``; a
unit test keeps the two vocabularies identical.

Usage::

    python3 code/scripts/assumption_catalogue.py            # check + print digest
    python3 code/scripts/assumption_catalogue.py --digest   # print only the digest

The digest is the SHA-256 of the exact bytes of the JSON file; a report pins
that value in ``catalogue.sha256``.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path

# ---------------------------------------------------------------------------
# Locations
# ---------------------------------------------------------------------------

REPO_ROOT = Path(__file__).resolve().parents[2]
CATALOGUE_PATH = REPO_ROOT / "code" / "assumptions" / "catalogue-v1.json"
EVIDENCE_KINDS_PATH = REPO_ROOT / "code" / "assumptions" / "evidence-kinds.json"
NON_CLAIMS_PATH = REPO_ROOT / "code" / "assumptions" / "non-claims.json"
PACKAGES_ROOT = REPO_ROOT / "code" / "packages"
SPEC_PATH = REPO_ROOT / "code" / "specs" / "VER13-assumption-catalogue.md"
BACKLOG_PATH = REPO_ROOT / "code" / "specs" / "VER-BACKLOG.md"

SCHEMA_ID = "ver13-assumption-catalogue-v1"

# Input caps, identical to ca-assurance-report/src/catalogue.rs: files are
# read with a capped read *before* parsing, so an oversized file never reaches
# the JSON parser; counts bound the per-entry work (and the pattern-overlap
# check, which compares patterns pairwise within a bucket).
MAX_CATALOGUE_BYTES = 1024 * 1024
MAX_EVIDENCE_KINDS_BYTES = 256 * 1024
MAX_NON_CLAIMS_BYTES = 64 * 1024
MAX_REPORT_BYTES = 16 * 1024 * 1024  # ca-assurance-report MAX_MANIFEST_BYTES
MAX_ENTRIES = 4096
MAX_LIST_ITEMS = 256
MAX_TOTAL_PATTERNS = 4096
EVIDENCE_KINDS_SCHEMA_ID = "ver13-evidence-kinds-v1"
NON_CLAIMS_SCHEMA_ID = "ver13-non-claims-v1"
# The repository-relative path a report records in ``catalogue.path``.
CATALOGUE_REPO_PATH = "code/assumptions/catalogue-v1.json"

# ---------------------------------------------------------------------------
# Vocabularies (VER13 sections 2 and 5.2).  Keep in sync with the JSON Schema.
# ---------------------------------------------------------------------------

# The eight stack layers, nearest the author first.  The order matters to the
# label rule `proved-to-L` (AC-5, part 2).
STACK_LAYERS = ("spec", "src", "wir", "cc", "mc", "uarch", "gate", "phys")
# `os` and `gov` are families that have no stack position; their entries are
# consumed at a layer of the same name (VER13 section 3.9 counts them as one
# group, "os/gov 11").
LAYERS = STACK_LAYERS + ("os", "gov")
FAMILIES = (
    "spec",
    "src",
    "wir",
    "cc",
    "mc",
    "uarch",
    "gate",
    "phys",
    "tcb",
    "os",
    "gov",
    "att",
)
KINDS = (
    "model",
    "trust",
    "tool",
    "environment",
    "attestation",
    "physical",
    "governance",
    "declared",
)
STATUSES = ("assumed", "attested", "evidenced", "discharged", "floor")
RUNGS = ("R0+", "R1", "R2", "R3", "R4")
DIFFICULTIES = ("S", "M", "L", "XL", "N/A")
TIERS = (
    "O1",
    "O2",
    "O3",
    "O3/O4",
    "O4",
    "O5",
    "O6",
    "P",
    "floor",
    "gov",
    "via-ladder",
    "",
)
PROOF_LABELS = (
    "proved",
    "bounded(k)",
    "proved-per-run",
    "proved-to-isa",
    "proved-to-gate-level",
)
CLAIM_KINDS = (
    "constant-time",
    "functional",
    "bound",
    "capability",
    "weakness-row",
    "report",
)
INPUT_CLASSES = ("public", "secret", "trusted", "hostile")

# Whole-string patterns, always applied with ``fullmatch``: ``re.match`` with
# ``$`` would accept a trailing newline (``"id\n"``) that the Rust loader in
# ca-assurance-report rejects, and the two validators must agree.
ID_RE = re.compile(r"[A-Za-z0-9._-]+")
ALIAS_RE = re.compile(r"[a-z0-9._:-]+")
BACKLOG_ID_RE = re.compile(r"[A-Z][A-Z0-9]*-[0-9A-Za-z]+")
PACKAGE_RE = re.compile(r"\*|[a-z0-9_-]+/[a-z0-9_-]+")
DIGEST_RE = re.compile(r"sha256:[0-9a-f]{64}")

ENTRY_REQUIRED = (
    "id",
    "aliases",
    "refines",
    "refined_by",
    "layer",
    "family",
    "title",
    "statement",
    "kind",
    "status",
    "applies_to",
    "depends_on",
    "discharge",
    "since",
    "legacy",
    "used_by",
    "tier",
    "constrains_input_classes",
)
ENTRY_OPTIONAL = ("discharged_by", "notes")
APPLIES_REQUIRED = ("labels", "claims", "packages")
DISCHARGE_REQUIRED = (
    "evidence_kinds",
    "rung",
    "tools",
    "difficulty",
    "backlog",
    "plan",
)
DISCHARGED_BY_REQUIRED = ("evidence", "digest", "stack", "pinned")

# ---------------------------------------------------------------------------
# Findings
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class Finding:
    """One violation.  ``rule`` is an ``AC-n`` id or ``schema``; ``where`` an entry id."""

    rule: str
    where: str
    message: str

    def __str__(self) -> str:
        return f"[{self.rule}] {self.where}: {self.message}"


class CatalogueError(Exception):
    """The input cannot even be read as a catalogue (fail closed)."""


# ---------------------------------------------------------------------------
# Strict JSON loading
# ---------------------------------------------------------------------------


def _no_duplicate_keys(pairs: list[tuple[str, object]]) -> dict[str, object]:
    """``object_pairs_hook`` that refuses a repeated key.

    Python's ``json`` silently keeps the *last* of two equal keys, so a
    hand-edit that pastes a second ``"status"`` line would change meaning
    without any error.  Duplicate keys are always a bug in a data file.
    """
    out: dict[str, object] = {}
    for key, value in pairs:
        if key in out:
            raise CatalogueError(f"duplicate JSON key {key!r}")
        out[key] = value
    return out


def _reject_constant(name: str) -> object:
    """``NaN`` / ``Infinity`` are not JSON; refuse them rather than guess."""
    raise CatalogueError(f"non-JSON constant {name}")


def read_capped(path: Path, cap: int) -> bytes:
    """Read at most ``cap`` bytes of ``path``; a longer file is an error.

    The limit is applied to the bytes actually read (``cap + 1`` requested), not
    to a file size that could change between a stat and the read.
    """
    with path.open("rb") as handle:
        raw = handle.read(cap + 1)
    if len(raw) > cap:
        raise CatalogueError(f"{path} is larger than the {cap}-byte limit")
    return raw


# Deepest container nesting accepted, the same limit as the Rust loader
# (`ca-assurance-report` `json::MAX_JSON_DEPTH`). The real files nest about five
# deep.
MAX_JSON_DEPTH = 64


def check_json_depth(text: str, limit: int = MAX_JSON_DEPTH) -> None:
    """Refuse JSON whose arrays/objects nest deeper than ``limit``, before parsing.

    ``json.loads`` recurses once per level, so how deep it can go depends on the
    interpreter: CPython 3.13's C decoder parsed 100,000 levels on CI, where an
    older interpreter raised ``RecursionError``. A fixed limit, checked by a
    linear scan, gives the same answer on every Python. Brackets inside strings
    do not count, so the scan tracks string and escape state::

        {"a": "[[["}      depth 1   (the brackets are string content)
        [[[]]]            depth 3
    """
    depth = 0
    in_string = False
    escaped = False
    for ch in text:
        if in_string:
            if escaped:
                escaped = False
            elif ch == "\\":
                escaped = True
            elif ch == '"':
                in_string = False
        elif ch == '"':
            in_string = True
        elif ch in "[{":
            depth += 1
            if depth > limit:
                raise CatalogueError(
                    f"JSON nested too deeply (more than {limit} levels)"
                )
        elif ch in "]}":
            depth -= 1


def parse_catalogue_text(text: str) -> dict:
    """Parse catalogue text strictly: no duplicate keys, no NaN, object root,
    at most ``MAX_JSON_DEPTH`` levels of nesting."""
    check_json_depth(text)
    try:
        data = json.loads(
            text,
            object_pairs_hook=_no_duplicate_keys,
            parse_constant=_reject_constant,
        )
    except json.JSONDecodeError as exc:
        raise CatalogueError(f"invalid JSON: {exc}") from exc
    except RecursionError as exc:
        # Deep nesting exhausts the parser's stack: a finding, not a traceback.
        raise CatalogueError("JSON nested too deeply") from exc
    if not isinstance(data, dict):
        raise CatalogueError("catalogue root must be a JSON object")
    return data


def catalogue_digest(raw: bytes) -> str:
    """SHA-256 (hex) of the exact bytes a report would pin."""
    return hashlib.sha256(raw).hexdigest()


# ---------------------------------------------------------------------------
# Structural validation (the hand-written twin of the JSON Schema)
# ---------------------------------------------------------------------------


def _is_str_list(value: object) -> bool:
    return isinstance(value, list) and all(isinstance(v, str) for v in value)


def _check_enum_list(
    out: list[Finding], eid: str, field: str, value: object, allowed: tuple[str, ...]
) -> None:
    if not _is_str_list(value):
        out.append(Finding("schema", eid, f"{field} must be a list of strings"))
        return
    for item in value:  # type: ignore[union-attr]
        if item not in allowed:
            out.append(Finding("schema", eid, f"{field} has unknown value {item!r}"))


def _check_fields(
    out: list[Finding],
    eid: str,
    where: str,
    obj: dict,
    required: tuple,
    optional: tuple = (),
) -> bool:
    """Unknown / missing field check shared by every object level."""
    ok = True
    for key in obj:
        if key not in required and key not in optional:
            out.append(Finding("schema", eid, f"{where}: unknown field {key!r}"))
            ok = False
    for key in required:
        if key not in obj:
            out.append(Finding("schema", eid, f"{where}: missing field {key!r}"))
            ok = False
    return ok


def _check_applies_to(out: list[Finding], eid: str, applies: object) -> None:
    if not isinstance(applies, dict):
        out.append(Finding("schema", eid, "applies_to must be an object"))
        return
    if not _check_fields(out, eid, "applies_to", applies, APPLIES_REQUIRED):
        return
    _check_enum_list(out, eid, "applies_to.labels", applies["labels"], PROOF_LABELS)
    _check_enum_list(out, eid, "applies_to.claims", applies["claims"], CLAIM_KINDS)
    pkgs = applies["packages"]
    if not _is_str_list(pkgs):
        out.append(
            Finding("schema", eid, "applies_to.packages must be a list of strings")
        )
    else:
        for pkg in pkgs:
            if not PACKAGE_RE.fullmatch(pkg):
                out.append(
                    Finding("schema", eid, f"applies_to.packages bad entry {pkg!r}")
                )


def _check_discharge(out: list[Finding], eid: str, discharge: object) -> None:
    if discharge is None:
        return
    if not isinstance(discharge, dict):
        out.append(Finding("schema", eid, "discharge must be an object or null"))
        return
    if not _check_fields(out, eid, "discharge", discharge, DISCHARGE_REQUIRED):
        return
    if not _is_str_list(discharge["evidence_kinds"]):
        out.append(
            Finding("schema", eid, "discharge.evidence_kinds must be a list of strings")
        )
    if discharge["rung"] not in RUNGS:
        out.append(
            Finding(
                "schema", eid, f"discharge.rung {discharge['rung']!r} not in {RUNGS}"
            )
        )
    if not _is_str_list(discharge["tools"]):
        out.append(Finding("schema", eid, "discharge.tools must be a list of strings"))
    if discharge["difficulty"] not in DIFFICULTIES:
        out.append(
            Finding(
                "schema", eid, f"discharge.difficulty {discharge['difficulty']!r} bad"
            )
        )
    backlog = discharge["backlog"]
    if not isinstance(backlog, str) or not BACKLOG_ID_RE.fullmatch(backlog):
        out.append(
            Finding("schema", eid, f"discharge.backlog {backlog!r} is not a backlog id")
        )
    if not isinstance(discharge["plan"], str) or not discharge["plan"].strip():
        out.append(Finding("schema", eid, "discharge.plan must be a non-empty string"))


def _check_discharged_by(out: list[Finding], eid: str, value: object) -> None:
    if not isinstance(value, list):
        out.append(Finding("schema", eid, "discharged_by must be a list"))
        return
    for item in value:
        if not isinstance(item, dict):
            out.append(Finding("schema", eid, "discharged_by items must be objects"))
            continue
        if not _check_fields(out, eid, "discharged_by", item, DISCHARGED_BY_REQUIRED):
            continue
        if not isinstance(item["evidence"], str) or not item["evidence"]:
            out.append(
                Finding(
                    "schema", eid, "discharged_by.evidence must be a non-empty string"
                )
            )
        if not isinstance(item["digest"], str) or not DIGEST_RE.fullmatch(
            item["digest"]
        ):
            out.append(
                Finding("schema", eid, "discharged_by.digest must be sha256:<64 hex>")
            )
        if not isinstance(item["stack"], str) or not item["stack"]:
            out.append(
                Finding("schema", eid, "discharged_by.stack must be a non-empty string")
            )
        if not isinstance(item["pinned"], dict):
            out.append(Finding("schema", eid, "discharged_by.pinned must be an object"))


def _check_scalar_types(out: list[Finding], eid: str, entry: dict) -> None:
    """Type and vocabulary checks for the flat fields of one entry."""
    for key in ("title", "statement", "since", "used_by"):
        if not isinstance(entry[key], str):
            out.append(Finding("schema", eid, f"{key} must be a string"))
    for key in ("title", "statement", "since"):
        if isinstance(entry[key], str) and not entry[key].strip():
            out.append(Finding("schema", eid, f"{key} must not be empty"))
    if not isinstance(entry["legacy"], bool):
        out.append(Finding("schema", eid, "legacy must be a boolean"))
    for key, allowed in (
        ("layer", LAYERS),
        ("family", FAMILIES),
        ("kind", KINDS),
        ("status", STATUSES),
        ("tier", TIERS),
    ):
        if entry[key] not in allowed:
            out.append(Finding("schema", eid, f"{key} {entry[key]!r} not in {allowed}"))
    if "notes" in entry and not isinstance(entry["notes"], str):
        out.append(Finding("schema", eid, "notes must be a string"))


def _check_reference_fields(out: list[Finding], eid: str, entry: dict) -> None:
    """Shape of the id-reference fields (resolution is a graph check later)."""
    aliases = entry["aliases"]
    if not _is_str_list(aliases):
        out.append(Finding("schema", eid, "aliases must be a list of strings"))
    else:
        for alias in aliases:
            if not ALIAS_RE.fullmatch(alias):
                out.append(
                    Finding(
                        "schema", eid, f"alias {alias!r} violates {ALIAS_RE.pattern}"
                    )
                )
    refines = entry["refines"]
    if refines is not None and not isinstance(refines, str):
        out.append(Finding("schema", eid, "refines must be an id string or null"))
    for key in ("refined_by", "depends_on"):
        if not _is_str_list(entry[key]):
            out.append(Finding("schema", eid, f"{key} must be a list of id strings"))
    _check_enum_list(
        out,
        eid,
        "constrains_input_classes",
        entry["constrains_input_classes"],
        INPUT_CLASSES,
    )


def validate_structure(data: dict) -> list[Finding]:
    """Validate the document shape; returns findings (empty list = well formed)."""
    out: list[Finding] = []
    _check_fields(out, "<root>", "root", data, ("schema", "assumptions"))
    if data.get("schema") != SCHEMA_ID:
        out.append(Finding("schema", "<root>", f"schema must be {SCHEMA_ID!r}"))
    entries = data.get("assumptions")
    if not isinstance(entries, list):
        out.append(Finding("schema", "<root>", "assumptions must be a list"))
        return out
    if len(entries) > MAX_ENTRIES:
        out.append(Finding("schema", "<root>", f"more than {MAX_ENTRIES} entries"))
        return out
    for index, entry in enumerate(entries):
        if not isinstance(entry, dict):
            out.append(Finding("schema", f"<entry {index}>", "entry must be an object"))
            continue
        raw_id = entry.get("id")
        eid = raw_id if isinstance(raw_id, str) else f"<entry {index}>"
        if not _check_fields(out, eid, "entry", entry, ENTRY_REQUIRED, ENTRY_OPTIONAL):
            continue
        if not isinstance(raw_id, str) or not ID_RE.fullmatch(raw_id):
            out.append(Finding("schema", eid, f"id must match {ID_RE.pattern}"))
        _check_scalar_types(out, eid, entry)
        _check_reference_fields(out, eid, entry)
        _check_applies_to(out, eid, entry["applies_to"])
        _check_discharge(out, eid, entry["discharge"])
        if "discharged_by" in entry:
            _check_discharged_by(out, eid, entry["discharged_by"])
    return out


# ---------------------------------------------------------------------------
# Spec section 3 parser (the human rendering)
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class SpecRow:
    """One row of a section 3 table, after light normalisation."""

    id: str
    layer: str
    family: str
    statement: str
    used_by: str
    discharge: str
    tools: tuple[str, ...]
    difficulty: str
    tier: str
    legacy: bool
    legacy_alias: str | None


_LEGACY_ALIAS_RE = re.compile(r"legacy `([^`]+)`")
_ATTESTED_PREFIX_RE = re.compile(r"^\*\(attested[^)]*\)\*\s*")
_SECTION_RE = re.compile(r"^### 3\.(\d+) ")
_TIER_FIX = {"via ladder": "via-ladder"}
_DIFF_RANK = {"S": 0, "M": 1, "L": 2, "XL": 3, "N/A": -1}
_DIFF_TOKEN_RE = re.compile(r"(?<![A-Za-z/])(XL|S|M|L|N/A)(?![A-Za-z])")

# Sections that hold the 84 entries, and the stack layer each implies.
# 3.7 (uarch/gate/phys), 3.8 (os/gov) and 3.4 (tcb) are resolved per id below.
_SECTION_LAYER = {"1": "spec", "2": "src", "3": "wir", "4": "wir", "5": "cc", "6": "mc"}


def family_of(entry_id: str) -> str:
    """Family is the id prefix, with three historical exceptions.

    * ``att.*`` is its own family although it is consumed at layer ``src``;
    * ``compiler.preserves_timing`` is the ``cc`` umbrella;
    * ``memory.unaliased`` is a ``wir`` entry that keeps its legacy name.
    """
    if entry_id == "compiler.preserves_timing":
        return "cc"
    if entry_id == "memory.unaliased":
        return "wir"
    return entry_id.split(".", 1)[0]


def layer_of(entry_id: str, section: str) -> str:
    """Consumption layer: from the section, or from the family in 3.7/3.8."""
    if section in _SECTION_LAYER:
        return _SECTION_LAYER[section]
    return family_of(entry_id)


def normalise_difficulty(raw: str) -> str:
    """Collapse the spec's free-text difficulty to the schema enum.

    ``"S (ct-compare) / L (general)"`` keeps the *worst* case (``L``) because a
    catalogue entry must not under-promise; ``"N/A at WIR"`` is ``N/A``; an empty
    cell (the umbrella) is ``N/A``.
    """
    tokens = _DIFF_TOKEN_RE.findall(raw)
    if not tokens:
        return "N/A"
    return max(tokens, key=lambda t: _DIFF_RANK[t])


def normalise_statement(meaning: str) -> str:
    """The spec's meaning cell minus the ``*(attested...)*`` marker."""
    return _ATTESTED_PREFIX_RE.sub("", meaning.strip())


def split_tools(cell: str) -> tuple[str, ...]:
    """Split a Tools cell on top-level commas and semicolons (not inside parentheses)."""
    text = cell.strip()
    if text in ("", "n/a", "same"):
        return ()
    parts: list[str] = []
    depth = 0
    current = ""
    for ch in text:
        if ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
        if ch in ",;" and depth == 0:
            parts.append(current)
            current = ""
        else:
            current += ch
    parts.append(current)
    return tuple(p.strip().replace("`", "") for p in parts if p.strip())


def _split_row(line: str) -> list[str]:
    return [cell.strip() for cell in line.strip().strip("|").split("|")]


def parse_spec_inventory(spec_text: str) -> list[SpecRow]:
    """Parse every entry row of section 3.1..3.8; error on a malformed row."""
    lines = spec_text.splitlines()
    try:
        start = next(
            i for i, ln in enumerate(lines) if ln.startswith("## 3. Inventory")
        )
    except StopIteration as exc:
        raise CatalogueError("spec has no '## 3. Inventory' section") from exc
    rows: list[SpecRow] = []
    section = ""
    for line in lines[start + 1 :]:
        if line.startswith("## "):
            break
        match = _SECTION_RE.match(line)
        if match:
            section = match.group(1)
            continue
        if not section or section == "9" or not line.startswith("| `"):
            continue
        cells = _split_row(line)
        if len(cells) != 7:
            raise CatalogueError(
                f"spec section 3.{section}: row has {len(cells)} cells: {line[:60]}"
            )
        id_cell, meaning, used_by, discharge, tools, difficulty, tier = cells
        id_match = re.match(r"^`([^`]+)`", id_cell)
        if not id_match:
            raise CatalogueError(
                f"spec section 3.{section}: row id not backticked: {line[:60]}"
            )
        entry_id = id_match.group(1)
        alias_match = _LEGACY_ALIAS_RE.search(id_cell)
        att_alias = None
        if entry_id.startswith("att."):
            _, area, name = entry_id.split(".", 2)
            att_alias = f"unverified::{area}::{name}"
        rows.append(
            SpecRow(
                id=entry_id,
                layer=layer_of(entry_id, section),
                family=family_of(entry_id),
                statement=normalise_statement(meaning),
                used_by=used_by,
                discharge=discharge,
                tools=split_tools(tools),
                difficulty=normalise_difficulty(difficulty),
                tier=_TIER_FIX.get(tier, tier),
                legacy="(legacy" in id_cell,
                legacy_alias=alias_match.group(1) if alias_match else att_alias,
            )
        )
    return rows


def parse_spec_counts(spec_text: str) -> dict:
    """Read the prose size claims of section 3.9 so they can be compared to the data."""
    match = re.search(
        r"^### 3\.9 .*?(?=^### |^## )", spec_text, flags=re.DOTALL | re.MULTILINE
    )
    if not match:
        raise CatalogueError("spec has no section 3.9")
    block = " ".join(match.group(0).split())
    total = re.search(r"\*\*(\d+) entries\*\*", block)
    fam_text = re.search(r"By family: (.*?)\. By consumption layer", block)
    lay_text = re.search(r"By consumption layer \(.*?\): (.*?)\. Of the 84", block)
    legacy = re.search(r"\*\*(\d+) are legacy ids", block)
    attested = re.search(r"(\d+) are\s+the existing `unverified::` ids", block)
    if not (total and fam_text and lay_text and legacy and attested):
        raise CatalogueError("spec section 3.9 size sentences not recognised")
    families = {
        name: int(n) for name, n in re.findall(r"`(\w+)` (\d+)", fam_text.group(1))
    }
    att = re.search(r"(\d+) `att\.\*`", fam_text.group(1))
    if att:
        families["att"] = int(att.group(1))
    layers = {
        name: int(n)
        for name, n in re.findall(r"\b(\w+(?:/\w+)?) (\d+)\b", lay_text.group(1))
    }
    return {
        "total": int(total.group(1)),
        "families": families,
        "layers": layers,
        "legacy": int(legacy.group(1)),
        "attested": int(attested.group(1)),
    }


def parse_spec_exemption_ids(spec_text: str) -> list[str]:
    """Catalogue ids (or ``family.*`` wildcards) named in the section 3.9 exemptions table."""
    match = re.search(
        r"^\| Exemption \|.*?(?=^\*\*Order of attack)",
        spec_text,
        flags=re.DOTALL | re.MULTILINE,
    )
    if not match:
        raise CatalogueError("spec section 3.9 exemptions table not found")
    ids: list[str] = []
    for line in match.group(0).splitlines():
        if not line.startswith("| `EXEMPT"):
            continue
        derived = _split_row(line)[1]
        ids.extend(
            t for t in re.findall(r"`([a-z0-9_.*-]+)`", derived) if "." in t or "_" in t
        )
    return ids


# ---------------------------------------------------------------------------
# Backlog ids
# ---------------------------------------------------------------------------

_BACKLOG_ROW_RE = re.compile(
    r"^\|\s*(?:[^|/]*?/\s*)?([A-Z][A-Z0-9]*-[0-9A-Za-z]+)\s*\|"
)


def parse_backlog_ids(backlog_text: str) -> set[str]:
    """Ids in the first cell of VER-BACKLOG table rows (``| P1 / AS-001a |``)."""
    ids: set[str] = set()
    for line in backlog_text.splitlines():
        match = _BACKLOG_ROW_RE.match(line)
        if match:
            ids.add(match.group(1))
    return ids


# ---------------------------------------------------------------------------
# Graph rules (AC-14)
# ---------------------------------------------------------------------------


def find_cycle(edges: dict[str, list[str]]) -> list[str] | None:
    """Return one cycle as ``[a, b, ..., a]`` or ``None``.  Iterative three-colour DFS."""
    white, grey, black = 0, 1, 2
    colour = {node: white for node in edges}
    for root, root_children in edges.items():
        if colour[root] != white:
            continue
        stack = [(root, iter(root_children))]
        path = [root]
        colour[root] = grey
        while stack:
            node, children = stack[-1]
            advanced = False
            for child in children:
                if child not in colour:
                    continue  # dangling refs are reported by the resolve check
                if colour[child] == grey:
                    return path[path.index(child) :] + [child]
                if colour[child] == white:
                    colour[child] = grey
                    path.append(child)
                    stack.append((child, iter(edges[child])))
                    advanced = True
                    break
            if not advanced:
                colour[node] = black
                path.pop()
                stack.pop()
    return None


def check_identity_and_graph(entries: list[dict]) -> list[Finding]:
    """AC-14: unique ids/aliases, resolving references, acyclic relations."""
    out: list[Finding] = []
    ids: dict[str, dict] = {}
    for entry in entries:
        eid = entry["id"]
        if eid in ids:
            out.append(Finding("AC-14", eid, "duplicate id"))
        ids[eid] = entry
    alias_owner: dict[str, str] = {}
    for entry in entries:
        for alias in entry["aliases"]:
            if alias in ids:
                out.append(
                    Finding(
                        "AC-14", entry["id"], f"alias {alias!r} collides with an id"
                    )
                )
            elif alias in alias_owner:
                out.append(
                    Finding(
                        "AC-14",
                        entry["id"],
                        f"alias {alias!r} also owned by {alias_owner[alias]}",
                    )
                )
            alias_owner[alias] = entry["id"]
    for entry in entries:
        eid = entry["id"]
        for field in ("depends_on", "refined_by"):
            for ref in entry[field]:
                if ref not in ids:
                    out.append(
                        Finding("AC-14", eid, f"{field} names unknown id {ref!r}")
                    )
        if entry["refines"] is not None and entry["refines"] not in ids:
            out.append(
                Finding("AC-14", eid, f"refines names unknown id {entry['refines']!r}")
            )
        if entry["refines"] == eid or eid in entry["depends_on"]:
            out.append(
                Finding("AC-14", eid, "an entry cannot depend on or refine itself")
            )
        # refines / refined_by are two views of one edge; they must agree.
        parent = ids.get(entry["refines"]) if entry["refines"] else None
        if parent is not None and eid not in parent["refined_by"]:
            out.append(
                Finding(
                    "AC-14", eid, f"refines {parent['id']} but is not in its refined_by"
                )
            )
        for child_id in entry["refined_by"]:
            child = ids.get(child_id)
            if child is not None and child["refines"] != eid:
                out.append(
                    Finding(
                        "AC-14",
                        eid,
                        f"refined_by {child_id} whose refines is not {eid}",
                    )
                )
    # Acyclicity is checked on each relation and on their union (a child that
    # depends on its own umbrella would be a cycle through both).
    unique = list(ids)
    graphs = {
        "depends_on": {i: list(ids[i]["depends_on"]) for i in unique},
        "refines": {
            i: [ids[i]["refines"]] if ids[i]["refines"] else [] for i in unique
        },
    }
    graphs["depends_on+refines"] = {
        i: graphs["depends_on"][i] + graphs["refines"][i] for i in unique
    }
    for name, graph in graphs.items():
        cycle = find_cycle(graph)
        if cycle:
            out.append(
                Finding("AC-14", cycle[0], f"{name} cycle: {' -> '.join(cycle)}")
            )
    return out


def check_backlog_refs(entries: list[dict], backlog_ids: set[str]) -> list[Finding]:
    """AC-14: every ``discharge.backlog`` is a real backlog id."""
    out: list[Finding] = []
    for entry in entries:
        discharge = entry["discharge"]
        if discharge is not None and discharge["backlog"] not in backlog_ids:
            out.append(
                Finding(
                    "AC-14",
                    entry["id"],
                    f"discharge.backlog {discharge['backlog']!r} is not in VER-BACKLOG.md",
                )
            )
    return out


# ---------------------------------------------------------------------------
# Shape rules (AC-8, AC-3, AC-11, AC-15, family/layer coherence)
# ---------------------------------------------------------------------------


def check_entry_shapes(entries: list[dict]) -> list[Finding]:
    """The catalogue-only halves of AC-8, AC-3, AC-11 and AC-15, plus coherence."""
    out: list[Finding] = []
    for entry in entries:
        eid = entry["id"]
        status, kind, discharge = entry["status"], entry["kind"], entry["discharge"]
        # AC-8: floor / declared entries are the bottom of the ladder.
        if kind == "declared" and status != "floor":
            out.append(Finding("AC-8", eid, "kind declared must have status floor"))
        if (status == "floor") != (discharge is None):
            out.append(
                Finding(
                    "AC-8", eid, "discharge must be null exactly when status is floor"
                )
            )
        if status == "discharged" and kind in ("declared", "physical"):
            out.append(Finding("AC-8", eid, f"kind {kind} can never be discharged"))
        # AC-3 (catalogue side): `discharged` is only valid with evidence.
        evidence = entry.get("discharged_by", [])
        if status == "discharged" and not evidence:
            out.append(
                Finding(
                    "AC-3", eid, "status discharged requires non-empty discharged_by"
                )
            )
        if status != "discharged" and evidence:
            out.append(
                Finding(
                    "AC-3",
                    eid,
                    "discharged_by is only allowed when status is discharged",
                )
            )
        # AC-11 (catalogue side): attestations own one unverified:: alias.
        unverified = [a for a in entry["aliases"] if a.startswith("unverified::")]
        if (kind == "attestation") != (status == "attested"):
            out.append(
                Finding(
                    "AC-11", eid, "kind attestation and status attested go together"
                )
            )
        if kind == "attestation" and len(unverified) != 1:
            out.append(
                Finding(
                    "AC-11", eid, "an attestation needs exactly one unverified:: alias"
                )
            )
        if kind != "attestation" and unverified:
            out.append(
                Finding("AC-11", eid, "only attestations may own an unverified:: alias")
            )
        # AC-15: no environment/spec assumption may constrain a hostile input.
        if "hostile" in entry["constrains_input_classes"] and (
            kind == "environment" or entry["family"] == "spec"
        ):
            out.append(
                Finding(
                    "AC-15",
                    eid,
                    "an environment/spec assumption may not constrain a hostile input",
                )
            )
        # Coherence of the derived columns with the id.
        if entry["family"] != family_of(eid):
            out.append(
                Finding(
                    "AC-14", eid, f"family {entry['family']!r} does not match id prefix"
                )
            )
    return out


# ---------------------------------------------------------------------------
# Spec-sync (AC-14 "CatalogueDrift")
# ---------------------------------------------------------------------------


def _drift(where: str, message: str) -> Finding:
    return Finding("AC-14", where, f"CatalogueDrift: {message}")


def _json_counts(entries: list[dict]) -> dict:
    families: dict[str, int] = {}
    layers: dict[str, int] = {}
    for entry in entries:
        families[entry["family"]] = families.get(entry["family"], 0) + 1
        key = "os/gov" if entry["layer"] in ("os", "gov") else entry["layer"]
        layers[key] = layers.get(key, 0) + 1
    return {"families": families, "layers": layers}


def _compare_row(entry: dict, row: SpecRow) -> list[Finding]:
    """Field-by-field comparison of one JSON entry with its spec row."""
    out: list[Finding] = []
    discharge = entry["discharge"]
    expected = {
        "layer": (entry["layer"], row.layer),
        "family": (entry["family"], row.family),
        "statement": (entry["statement"], row.statement),
        "used_by": (entry["used_by"], row.used_by),
        "tier": (entry["tier"], row.tier),
        "legacy": (entry["legacy"], row.legacy),
    }
    for field, (have, want) in expected.items():
        if have != want:
            out.append(
                _drift(entry["id"], f"{field} is {have!r} in JSON but {want!r} in spec")
            )
    if discharge is not None:
        for field, have, want in (
            ("discharge.plan", discharge["plan"], row.discharge),
            ("discharge.tools", tuple(discharge["tools"]), row.tools),
            ("discharge.difficulty", discharge["difficulty"], row.difficulty),
        ):
            if have != want:
                out.append(
                    _drift(
                        entry["id"], f"{field} is {have!r} in JSON but {want!r} in spec"
                    )
                )
    elif row.difficulty != "N/A":
        out.append(
            _drift(
                entry["id"],
                f"spec difficulty {row.difficulty} but JSON has no discharge",
            )
        )
    legacy_alias = row.legacy_alias
    if legacy_alias is not None and legacy_alias not in entry["aliases"]:
        out.append(
            _drift(entry["id"], f"alias {legacy_alias!r} from the spec row is missing")
        )
    return out


def check_spec_sync(entries: list[dict], spec_text: str) -> list[Finding]:
    """Rule AC-14: spec section 3 (rows and prose counts) must equal the JSON."""
    out: list[Finding] = []
    rows = parse_spec_inventory(spec_text)
    spec_ids = [r.id for r in rows]
    json_ids = [e["id"] for e in entries]
    for dup in sorted({i for i in spec_ids if spec_ids.count(i) > 1}):
        out.append(_drift(dup, "id appears twice in spec section 3"))
    for missing in sorted(set(spec_ids) - set(json_ids)):
        out.append(_drift(missing, "in spec section 3 but not in the JSON"))
    for extra in sorted(set(json_ids) - set(spec_ids)):
        out.append(_drift(extra, "in the JSON but not in spec section 3"))
    if (
        spec_ids != json_ids
        and set(spec_ids) == set(json_ids)
        and len(spec_ids) == len(json_ids)
    ):
        out.append(
            _drift("<order>", "JSON entry order differs from spec section 3 order")
        )
    by_id = {r.id: r for r in rows}
    for entry in entries:
        row = by_id.get(entry["id"])
        if row is not None:
            out.extend(_compare_row(entry, row))

    counts = parse_spec_counts(spec_text)
    mine = _json_counts(entries)
    if counts["total"] != len(entries):
        out.append(
            _drift(
                "<count>",
                f"spec says {counts['total']} entries, JSON has {len(entries)}",
            )
        )
    if counts["total"] != len(rows):
        out.append(
            _drift(
                "<count>",
                f"spec says {counts['total']} entries, its tables have {len(rows)}",
            )
        )
    for family in sorted(set(counts["families"]) | set(mine["families"])):
        want, have = counts["families"].get(family, 0), mine["families"].get(family, 0)
        if want != have:
            out.append(
                _drift(f"<family {family}>", f"spec says {want}, JSON has {have}")
            )
    for layer in sorted(set(counts["layers"]) | set(mine["layers"])):
        want, have = counts["layers"].get(layer, 0), mine["layers"].get(layer, 0)
        if want != have:
            out.append(_drift(f"<layer {layer}>", f"spec says {want}, JSON has {have}"))
    legacy_json = sum(1 for e in entries if e["legacy"])
    if counts["legacy"] != legacy_json:
        out.append(
            _drift(
                "<legacy>",
                f"spec says {counts['legacy']} legacy ids, JSON has {legacy_json}",
            )
        )
    att_json = sum(1 for e in entries if e["kind"] == "attestation")
    if counts["attested"] != att_json:
        out.append(
            _drift(
                "<attested>",
                f"spec says {counts['attested']} unverified ids, JSON has {att_json}",
            )
        )
    # Exemption derivation table: every id (or `family.*` wildcard) must exist.
    known = set(json_ids)
    for token in parse_spec_exemption_ids(spec_text):
        if token.endswith(".*"):
            if not any(i.startswith(token[:-1]) for i in known):
                out.append(_drift(token, "exemption wildcard matches no catalogue id"))
        elif token not in known:
            out.append(
                _drift(token, "exemption table names an id absent from the JSON")
            )
    return out


# ---------------------------------------------------------------------------
# evidence-kinds.json and non-claims.json (the twins of the Rust loader in
# ca-assurance-report/src/catalogue.rs; keep the rules identical)
# ---------------------------------------------------------------------------

MANIFESTS = ("test", "proof", "bench", "planned")
KIND_RE = re.compile(r"[a-z0-9-]+")
SEGMENT_RE = re.compile(r"[A-Za-z0-9_.:-]+")
EVIDENCE_KIND_FIELDS = (
    "kind",
    "manifest",
    "patterns",
    "verdict",
    "per_run",
    "description",
)
NON_CLAIM_FIELDS = ("package", "cwe", "claim", "source", "statement")
MAX_PATTERN_SEGMENTS = 16


def parse_pattern(pattern: str) -> list[str] | None:
    """Segments of an evidence-id pattern, or ``None`` when malformed.

    A pattern is 1..16 ``/``-separated segments; each is ``*`` (exactly one
    non-empty segment) or a literal of ``[A-Za-z0-9_.:-]+``; the first segment
    is a literal namespace, so no pattern matches every id.
    """
    parts = pattern.split("/")
    if not 1 <= len(parts) <= MAX_PATTERN_SEGMENTS or parts[0] == "*":
        return None
    if any(p != "*" and not SEGMENT_RE.fullmatch(p) for p in parts):
        return None
    return parts


def patterns_overlap(a: list[str], b: list[str]) -> bool:
    """Can one id match both?  Exact for this grammar (no multi-segment ``*``)."""
    return len(a) == len(b) and all(
        x == "*" or y == "*" or x == y for x, y in zip(a, b)
    )


def check_evidence_kinds(data: object, entries: list[dict]) -> list[Finding]:
    """Structure of ``evidence-kinds.json`` and its closure with the catalogue."""
    out: list[Finding] = []
    where = "<evidence-kinds>"
    if not isinstance(data, dict):
        return [Finding("schema", where, "root must be an object")]
    _check_fields(out, where, "root", data, ("schema", "kinds"))
    if data.get("schema") != EVIDENCE_KINDS_SCHEMA_ID:
        out.append(
            Finding("schema", where, f"schema must be {EVIDENCE_KINDS_SCHEMA_ID!r}")
        )
    rows = data.get("kinds")
    if not isinstance(rows, list) or not rows:
        out.append(Finding("schema", where, "kinds must be a non-empty list"))
        return out
    if len(rows) > MAX_ENTRIES:
        out.append(Finding("schema", where, f"more than {MAX_ENTRIES} kinds"))
        return out
    defined: dict[str, list[list[str]]] = {}
    for index, row in enumerate(rows):
        rid = row.get("kind") if isinstance(row, dict) else None
        eid = rid if isinstance(rid, str) else f"<kind {index}>"
        if not isinstance(row, dict):
            out.append(Finding("schema", eid, "row must be an object"))
            continue
        if not _check_fields(out, eid, "kind", row, EVIDENCE_KIND_FIELDS):
            continue
        if not isinstance(rid, str) or not KIND_RE.fullmatch(rid):
            out.append(Finding("schema", eid, "kind must match [a-z0-9-]+"))
            continue
        if rid in defined:
            out.append(Finding("schema", eid, "kind defined twice"))
        manifest = row["manifest"]
        if not isinstance(manifest, str):
            # Never use a list/dict as a key below (``dict.get`` would raise).
            out.append(Finding("schema", eid, "manifest must be a string"))
            manifest = "<invalid>"
        elif manifest not in MANIFESTS:
            out.append(
                Finding("schema", eid, f"manifest {manifest!r} not in {MANIFESTS}")
            )
        if row["verdict"] != "pass":
            out.append(Finding("schema", eid, "verdict must be 'pass'"))
        if not isinstance(row["per_run"], bool):
            out.append(Finding("schema", eid, "per_run must be a boolean"))
        if not isinstance(row["description"], str) or not row["description"].strip():
            out.append(Finding("schema", eid, "description must be a non-empty string"))
        parsed: list[list[str]] = []
        if not _is_str_list(row["patterns"]) or not row["patterns"]:
            out.append(
                Finding("schema", eid, "patterns must be a non-empty list of strings")
            )
        elif len(row["patterns"]) > MAX_LIST_ITEMS:
            out.append(Finding("schema", eid, f"more than {MAX_LIST_ITEMS} patterns"))
        else:
            for pattern in row["patterns"]:
                seg = parse_pattern(pattern)
                if seg is None:
                    out.append(Finding("schema", eid, f"malformed pattern {pattern!r}"))
                    continue
                first = seg[0]
                ok = {
                    "test": first == "warrant",
                    "proof": first == "warrant",
                    "bench": first == "bench",
                }.get(manifest, first not in ("warrant", "bench"))
                if not ok:
                    out.append(
                        Finding(
                            "schema",
                            eid,
                            f"pattern {pattern!r} is outside the {manifest!r} namespace",
                        )
                    )
                parsed.append(seg)
        defined[rid] = parsed
    flat = [(k, p) for k, pats in defined.items() for p in pats]
    if len(flat) > MAX_TOTAL_PATTERNS:
        out.append(Finding("schema", where, f"more than {MAX_TOTAL_PATTERNS} patterns"))
        return out
    # Two patterns can only overlap with equal length and the same (always
    # literal) first segment, so compare pairwise within those buckets only.
    buckets: dict[tuple[int, str], list[tuple[str, list[str]]]] = {}
    for kind, pattern in flat:
        buckets.setdefault((len(pattern), pattern[0]), []).append((kind, pattern))
    for bucket in buckets.values():
        for i, (ka, pa) in enumerate(bucket):
            for kb, pb in bucket[i + 1 :]:
                if patterns_overlap(pa, pb):
                    out.append(
                        Finding("schema", ka, f"a pattern overlaps a pattern of {kb!r}")
                    )
    used: set[str] = set()
    for entry in entries:
        discharge = entry.get("discharge")
        if isinstance(discharge, dict) and _is_str_list(
            discharge.get("evidence_kinds")
        ):
            for kind in discharge["evidence_kinds"]:
                used.add(kind)
                if kind not in defined:
                    out.append(
                        Finding(
                            "AC-3",
                            entry["id"],
                            f"evidence kind {kind!r} is not defined in evidence-kinds.json",
                        )
                    )
    for kind in sorted(set(defined) - used):
        out.append(
            Finding("AC-3", kind, "evidence kind is named by no catalogue entry")
        )
    return out


def check_non_claims(data: object) -> list[Finding]:
    """Structure of ``non-claims.json`` (rule AC-12's data)."""
    out: list[Finding] = []
    where = "<non-claims>"
    if not isinstance(data, dict):
        return [Finding("schema", where, "root must be an object")]
    _check_fields(out, where, "root", data, ("schema", "non_claims"))
    if data.get("schema") != NON_CLAIMS_SCHEMA_ID:
        out.append(Finding("schema", where, f"schema must be {NON_CLAIMS_SCHEMA_ID!r}"))
    rows = data.get("non_claims")
    if not isinstance(rows, list):
        out.append(Finding("schema", where, "non_claims must be a list"))
        return out
    if len(rows) > MAX_ENTRIES:
        out.append(Finding("schema", where, f"more than {MAX_ENTRIES} rows"))
        return out
    seen: set[tuple] = set()
    for index, row in enumerate(rows):
        eid = f"<non-claim {index}>"
        if not isinstance(row, dict):
            out.append(Finding("schema", eid, "row must be an object"))
            continue
        if not _check_fields(out, eid, "non-claim", row, NON_CLAIM_FIELDS):
            continue
        pkg = row["package"]
        if not isinstance(pkg, str) or pkg == "*" or not PACKAGE_RE.fullmatch(pkg):
            out.append(Finding("schema", eid, f"package {pkg!r} must be lang/name"))
        cwe = row["cwe"]
        if (
            not isinstance(cwe, int)
            or isinstance(cwe, bool)
            or not 1 <= cwe <= 1_000_000
        ):
            out.append(Finding("schema", eid, "cwe must be an integer in 1..1000000"))
        if row["claim"] not in CLAIM_KINDS:
            out.append(
                Finding("schema", eid, f"claim {row['claim']!r} not in {CLAIM_KINDS}")
            )
        src = row["source"]
        if (
            not isinstance(src, str)
            or not src.startswith("code/")
            or ".." in src
            or "\\" in src
        ):
            out.append(
                Finding("schema", eid, "source must be a repository path under code/")
            )
        elif not (REPO_ROOT / src).is_file():
            out.append(Finding("schema", eid, f"source {src!r} does not exist"))
        if not isinstance(row["statement"], str) or not row["statement"].strip():
            out.append(Finding("schema", eid, "statement must be a non-empty string"))
        key = (pkg, cwe, row["claim"])
        # Only hashable scalars form a key; a list or object here was already
        # reported above and must not raise TypeError.
        if all(isinstance(k, (str, int)) for k in key):
            if key in seen:
                out.append(Finding("schema", eid, "duplicate non-claim"))
            seen.add(key)
    return out


def expected_pin(raw_catalogue: bytes, raw_kinds: bytes, raw_non_claims: bytes) -> dict:
    """The ``catalogue`` object a report generated now would carry."""
    return {
        "schema": SCHEMA_ID,
        "path": CATALOGUE_REPO_PATH,
        "sha256": catalogue_digest(raw_catalogue),
        "evidence_kinds_sha256": catalogue_digest(raw_kinds),
        "non_claims_sha256": catalogue_digest(raw_non_claims),
    }


def check_report_pins(packages_root: Path, pin: dict) -> list[Finding]:
    """AC-14 (pins): every committed assurance report pins the current files."""
    out: list[Finding] = []
    for report in sorted(packages_root.glob("*/*/assurance_report.json")):
        try:
            where = str(report.relative_to(REPO_ROOT))
        except ValueError:
            where = str(report)
        try:
            raw = read_capped(report, MAX_REPORT_BYTES)
            data = parse_catalogue_text(raw.decode("utf-8"))
        except (CatalogueError, OSError, UnicodeDecodeError) as exc:
            out.append(Finding("AC-14", where, f"unreadable report: {exc}"))
            continue
        got = data.get("catalogue") if isinstance(data, dict) else None
        if got != pin:
            out.append(
                Finding(
                    "AC-14",
                    where,
                    "CatalogueDrift: the report's catalogue pin is not the current "
                    "files; regenerate it with ca-assurance-report --generate",
                )
            )
    return out


# ---------------------------------------------------------------------------
# Orchestration
# ---------------------------------------------------------------------------


def check_catalogue(
    raw: bytes,
    spec_text: str | None = None,
    backlog_text: str | None = None,
) -> tuple[list[Finding], str]:
    """Run every catalogue-only rule.  Returns ``(findings, sha256_hex)``.

    ``spec_text`` / ``backlog_text`` of ``None`` skip the spec-sync and backlog
    rules (used only by tests of the structural layer).  Structural errors stop
    the later rules, which assume a well-formed document.
    """
    digest = catalogue_digest(raw)
    try:
        text = raw.decode("utf-8")
    except UnicodeDecodeError as exc:
        raise CatalogueError(f"catalogue is not UTF-8: {exc}") from exc
    data = parse_catalogue_text(text)
    findings = validate_structure(data)
    if findings:
        return findings, digest
    entries = data["assumptions"]
    findings.extend(check_identity_and_graph(entries))
    findings.extend(check_entry_shapes(entries))
    if backlog_text is not None:
        findings.extend(check_backlog_refs(entries, parse_backlog_ids(backlog_text)))
    if spec_text is not None:
        findings.extend(check_spec_sync(entries, spec_text))
    return findings, digest


def check_companions(
    raw_catalogue: bytes,
    raw_kinds: bytes,
    raw_non_claims: bytes,
    packages_root: Path | None,
) -> list[Finding]:
    """Evidence kinds, non-claims and report pins (catalogue already valid)."""
    entries = parse_catalogue_text(raw_catalogue.decode("utf-8"))["assumptions"]
    findings = check_evidence_kinds(
        parse_catalogue_text(raw_kinds.decode("utf-8")), entries
    )
    findings.extend(
        check_non_claims(parse_catalogue_text(raw_non_claims.decode("utf-8")))
    )
    if packages_root is not None:
        findings.extend(
            check_report_pins(
                packages_root, expected_pin(raw_catalogue, raw_kinds, raw_non_claims)
            )
        )
    return findings


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("--catalogue", type=Path, default=CATALOGUE_PATH)
    parser.add_argument("--evidence-kinds", type=Path, default=EVIDENCE_KINDS_PATH)
    parser.add_argument("--non-claims", type=Path, default=NON_CLAIMS_PATH)
    parser.add_argument(
        "--packages-root",
        type=Path,
        default=PACKAGES_ROOT,
        help="where to look for */*/assurance_report.json pins",
    )
    parser.add_argument("--spec", type=Path, default=SPEC_PATH)
    parser.add_argument("--backlog", type=Path, default=BACKLOG_PATH)
    parser.add_argument(
        "--digest", action="store_true", help="print only the SHA-256 and exit"
    )
    args = parser.parse_args(argv)
    try:
        raw = read_capped(args.catalogue, MAX_CATALOGUE_BYTES)
        if args.digest:
            print(catalogue_digest(raw))
            return 0
        findings, digest = check_catalogue(
            raw,
            args.spec.read_text(encoding="utf-8"),
            args.backlog.read_text(encoding="utf-8"),
        )
        if not findings:
            findings = check_companions(
                raw,
                read_capped(args.evidence_kinds, MAX_EVIDENCE_KINDS_BYTES),
                read_capped(args.non_claims, MAX_NON_CLAIMS_BYTES),
                args.packages_root,
            )
    except (CatalogueError, OSError, UnicodeDecodeError) as exc:
        print(f"assumption-catalogue: ERROR: {exc}", file=sys.stderr)
        return 1
    for finding in findings:
        print(f"assumption-catalogue: FAIL {finding}", file=sys.stderr)
    if findings:
        print(f"assumption-catalogue: {len(findings)} finding(s)", file=sys.stderr)
        return 1
    count = len(parse_catalogue_text(raw.decode("utf-8"))["assumptions"])
    print(f"assumption-catalogue: OK ({count} entries)")
    print(f"catalogue sha256: {digest}")
    print(
        f"evidence-kinds sha256: {catalogue_digest(read_capped(args.evidence_kinds, MAX_EVIDENCE_KINDS_BYTES))}"
    )
    print(
        f"non-claims sha256: {catalogue_digest(read_capped(args.non_claims, MAX_NON_CLAIMS_BYTES))}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
