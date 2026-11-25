#!/usr/bin/env python3
"""The VER31 release gate: compute what a package may be published as.

Why this file exists
====================

For most of this repository's life the release rule was "nothing ships until it
is proven" (VER27 P13).  Nothing shipped.  VER31 replaced that with an honest
rule: *publish exactly what the evidence supports, and say so*.  This script is
the "say so" part.  It reads the generated ``assurance_report.json``, joins it
with the assumption buckets in ``code/assumptions/buckets-v1.json`` and computes
three things nobody is allowed to type by hand:

* the **rung** of every claim on the VER31 claim ladder (section 4);
* the package's **claim tier** (its weakest claim) and **release tier**
  (its claim tier, capped at ``Tested`` unless the higher-tier conditions
  H1-H4 hold, section 6);
* the **floor checks** F1-F12 that any release, even ``Tested``, must pass
  (section 5), and the **authorization** checks of section 9.

The rule it exists to keep is VER31 section 1.1: *a tier is computed, never
typed*.  The repository learned that the hard way (VER-P030 / WD-001): a
hand-written report passed a gate that only checked the report agreed with
itself.  So everything below is derived, and anything the script cannot derive
it refuses rather than guesses.

An analogy: Lean's ``#print axioms``
====================================

When Lean checks a theorem it can tell you every axiom the proof used.  It does
*not* list the things it trusts silently: its own kernel, the compiler that
built it, the CPU.  VER31 sorts our assumptions the same way, into three
buckets::

    (a) accepted platform baseline   compiler, CPU, gates, OS, language
                                     semantics. Listed, never blocks `proved`.
    (b) statement                    "the claim says what we mean". Closed by
                                     the owner's signed sign-off, not a proof.
    (c) our own gaps                 our tools, our code, our process. Closed
                                     only by checked evidence. Any open one
                                     blocks `proved`.

and then prints *all* of them with every release, grouped by bucket.

The claim ladder in one table
=============================

=====  ==============================  ========================================
Rank   Rung                            Computed when
=====  ==============================  ========================================
1      ``tested``                      example + property (+ fuzz, exhaustive
                                       where present) all ``pass``
2      ``bounded(N)``                  rung 1 + generator label ``bounded(N)``
                                       + a DRAT certificate
2      ``exhaustive``                  rung 1 + ``exhaustive.domain == "full"``
                                       (no report emits it yet: unreachable)
3      ``checked``                     rung 1 + a proved-family label + a
                                       certificate, but an open (b) or (c) id
4      ``proved``                      rung 3's inputs, no open (c), every (b)
                                       id covered by a valid signed sign-off
5      ``proved, baseline discharged`` rung 4 + every non-floor (a) id
                                       discharged
=====  ==============================  ========================================

Ordering: ``tested < bounded(N) < bounded(M) (N < M) < exhaustive < checked <
proved < proved, baseline discharged``.

What this script does *not* do
==============================

It does not run tests, coverage or lint (F1-F3): the workflow runs those as
earlier steps of the same job and passes their results in.  It does not decide
whether a version already exists on a registry: the workflow asks the registry.
And it is itself one of our own unverified tools, so its correctness is a
bucket-(c) gap like ``tcb.report.checker-correct``; the release notes say so.

Subcommands
===========

``check``      compute everything and write ``gate.json``
``notes``      render release notes from ``gate.json``
``assurance``  write or ``--check`` the package's ``ASSURANCE.md``
``verify-tag`` verify a hardware-key-signed release tag (section 9.1)

Only the Python standard library is used: the gate must not trust a
third-party package to decide what is trustworthy.
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import re
import subprocess
import sys
import tarfile
import tempfile
from collections.abc import Iterable
from dataclasses import dataclass, field
from pathlib import Path, PurePosixPath
from typing import Any

import tomllib

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

REPO_ROOT = Path(__file__).resolve().parents[2]

BUCKETS_SCHEMA = "ver31-assumption-buckets-v1"
GATE_SCHEMA = "ver31-gate-v1"
REVIEW_SCHEMA = "ver31-security-review-v1"
SIGNOFF_SCHEMA = "ver31-statement-signoff-v1"
BENCH_SCHEMA = "warrant-bench-evidence-v1"

BUCKETS_REL = "code/assumptions/buckets-v1.json"
CATALOGUE_REL = "code/assumptions/catalogue-v1.json"
SIGNERS_REL = ".github/release-signers"

# Size caps for every file this script reads.  An attacker-shaped input should
# cost a refusal, not memory.
MAX_JSON_BYTES = 8 * 1024 * 1024
MAX_TEXT_BYTES = 1024 * 1024
MAX_ARTIFACT_MEMBER_BYTES = 8 * 1024 * 1024
MAX_ARTIFACT_MEMBERS = 20_000

# The meaning of each bucket is fixed HERE, in code, not in the data file.  If
# the data could say `c: blocks_proved: false`, one edit to a JSON file would
# raise every tier in the repository (VER31 section 3).
BUCKET_BLOCKS_PROVED = {"a": False, "b": True, "c": True}
BUCKET_TITLES = {
    "a": "accepted platform baseline",
    "b": "statement",
    "c": "own gap",
}

# Signature namespaces (VER31 sections 3.2, 9.1, 9.6).  A signature made for
# one purpose must not verify for another.
NS_TAG = "git"
NS_STATEMENT = "coding-adventures-statement"
NS_BUCKETS = "coding-adventures-buckets"
NS_SIGNERS = "coding-adventures-signers"

# Only hardware-backed (FIDO2 "sk-") key types may sign releases.  Note this
# proves nothing on its own: software can emulate a security key.  The flags
# check in `parse_sshsig_flags` and the human approval are the other halves.
SK_KEY_TYPES = ("sk-ssh-ed25519@openssh.com", "sk-ecdsa-sha2-nistp256@openssh.com")
SK_FLAG_USER_PRESENCE = 0x01
SK_FLAG_USER_VERIFIED = 0x04

# Rung ranks.  `bounded(N)` sorts by N inside rank 2, `exhaustive` above every
# bounded(N) because it covers its whole domain.
RANK = {
    "tested": 1,
    "bounded": 2,
    "exhaustive": 2,
    "checked": 3,
    "proved": 4,
    "proved, baseline discharged": 5,
}
PROVED_FAMILY = ("proved", "proved-to-isa", "proved-to-gate-level")
# Our own verification tools.  The gate adds these to every proof-backed
# claim itself, whatever the report lists: a report (or a weakened generator
# or checker) that "forgets" them must not be able to reach `proved`.  They
# leave only when the code-owned catalogue records them as discharged.
ALWAYS_ASSUMED = (
    "tcb.report.checker-correct",
    "tcb.evidence.manifest-honesty",
    "tcb.drat.checker-correct",
    "tcb.bitblast.encoding-correct",
    "tcb.symexec.engine-correct",
)
# Strings from the report that are rendered into Markdown the owner reads
# before approving.  Anything else (newlines, pipes, backticks) is refused, so
# a report cannot spoof the tier text.
SAFE_TEXT_RE = re.compile(r"^[A-Za-z0-9_.:/(),<> -]{1,128}$")
CERT_RE = re.compile(r"^drat:sha256:[0-9a-f]{64}$")
BOUNDED_RE = re.compile(r"^bounded\(([0-9]{1,9})\)$")

# Release tags carry a version; this is the same pattern publish.yml uses.
VERSION_RE = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+([.-]?[0-9A-Za-z]+)*$")
# Release tags: the publish.yml pattern.  Rust and TypeScript tags name the
# package directory; Python and Ruby tags name the registry package.
TAG_RE = re.compile(r"^(python|ruby|rust|typescript)/[A-Za-z0-9_-]+/v[0-9]+\.[0-9]+\.[0-9]+([.-]?[0-9A-Za-z]+)*$")
# A git ref the gate may read history from: a fully qualified ref or HEAD.
REF_RE = re.compile(r"^(HEAD|refs/(remotes|heads)/[A-Za-z0-9._/-]+)$")


def safe_text(value: Any) -> str:
    """Refuse report text that could spoof the Markdown the owner reads."""
    if not isinstance(value, str) or not SAFE_TEXT_RE.match(value):
        raise GateError("INPUT", f"report text {value!r} is not plain text")
    return value


class GateError(Exception):
    """A refusal.  ``code`` is stable so tests and CI logs can name the rule."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(f"{code}: {message}")
        self.code = code
        self.message = message


# ---------------------------------------------------------------------------
# Small, careful readers
# ---------------------------------------------------------------------------


def read_capped(path: Path, cap: int) -> bytes:
    """Read a regular file of at most ``cap`` bytes.

    A symlink is refused: every input here is a repository file, and a link
    could point the gate at bytes the reviewer never saw.
    """
    if path.is_symlink():
        raise GateError("INPUT", f"{path} is a symlink")
    if not path.is_file():
        raise GateError("INPUT", f"{path} is missing or not a regular file")
    with path.open("rb") as handle:
        data = handle.read(cap + 1)
    if len(data) > cap:
        raise GateError("INPUT", f"{path} is larger than {cap} bytes")
    return data


def load_json(path: Path, cap: int = MAX_JSON_BYTES) -> Any:
    raw = read_capped(path, cap)
    try:
        return json.loads(raw)
    except (ValueError, UnicodeDecodeError) as exc:
        raise GateError("INPUT", f"{path} is not valid JSON: {exc}") from exc


def sha256_hex(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def run(cmd: list[str], cwd: Path, stdin: bytes | None = None) -> subprocess.CompletedProcess:
    """Run a command with an argument list (never a shell) and capture output."""
    return subprocess.run(
        cmd, cwd=cwd, input=stdin, capture_output=True, check=False, timeout=120
    )


def git(repo: Path, *args: str) -> str:
    proc = run(["git", *args], repo)
    if proc.returncode != 0:
        raise GateError("GIT", f"git {' '.join(args)} failed: {proc.stderr.decode(errors='replace').strip()}")
    return proc.stdout.decode("utf-8", errors="strict")


def git_bytes(repo: Path, *args: str) -> bytes:
    proc = run(["git", *args], repo)
    if proc.returncode != 0:
        raise GateError("GIT", f"git {' '.join(args)} failed: {proc.stderr.decode(errors='replace').strip()}")
    return proc.stdout


# ---------------------------------------------------------------------------
# Buckets (VER31 section 3)
# ---------------------------------------------------------------------------


@dataclass
class Buckets:
    """The bucket of every catalogue id, plus which (a) ids are floors."""

    bucket: dict[str, str]
    floor: dict[str, bool]
    revision: int
    sha256: str
    # Ids the code-owned catalogue itself records as `discharged`.  A report's
    # `proof.discharged` counts only for these (see `claim_rung`).
    catalogue_discharged: frozenset[str] = frozenset()


def load_buckets(buckets_path: Path, catalogue_path: Path) -> Buckets:
    """Load the bucket file and cross-check it against the catalogue.

    Fails closed on: a wrong schema; a bucket whose ``blocks_proved`` differs
    from the meaning fixed in code; an id missing from either file (coverage is
    closed in *both* directions); an id listed twice; a ``floor`` flag that
    disagrees with the catalogue (``status: floor`` or ``kind: declared``).
    """
    raw = read_capped(buckets_path, MAX_JSON_BYTES)
    doc = json.loads(raw)
    if not isinstance(doc, dict) or doc.get("schema") != BUCKETS_SCHEMA:
        raise GateError("BUCKETS", f"schema must be {BUCKETS_SCHEMA}")
    revision = doc.get("revision")
    if not isinstance(revision, int) or isinstance(revision, bool) or revision < 1:
        raise GateError("BUCKETS", "revision must be a positive integer")
    meta = doc.get("buckets")
    if not isinstance(meta, dict) or set(meta) != set(BUCKET_BLOCKS_PROVED):
        raise GateError("BUCKETS", "buckets must be exactly a, b, c")
    for key, blocks in BUCKET_BLOCKS_PROVED.items():
        if not isinstance(meta[key], dict) or meta[key].get("blocks_proved") is not blocks:
            raise GateError(
                "BUCKETS",
                f"bucket {key} must have blocks_proved={str(blocks).lower()}; the data cannot redefine a bucket",
            )
    catalogue = load_json(catalogue_path)
    entries = catalogue.get("assumptions") if isinstance(catalogue, dict) else None
    if not isinstance(entries, list):
        raise GateError("CATALOGUE", "catalogue has no assumptions array")
    cat_floor: dict[str, bool] = {}
    cat_discharged: set[str] = set()
    for entry in entries:
        ident = entry.get("id")
        if not isinstance(ident, str):
            raise GateError("CATALOGUE", "catalogue entry without a string id")
        cat_floor[ident] = entry.get("status") == "floor" or entry.get("kind") == "declared"
        if entry.get("status") == "discharged":
            cat_discharged.add(ident)

    bucket: dict[str, str] = {}
    floor: dict[str, bool] = {}
    items = doc.get("assumptions")
    if not isinstance(items, list):
        raise GateError("BUCKETS", "assumptions must be an array")
    for item in items:
        if not isinstance(item, dict):
            raise GateError("BUCKETS", "assumption entry is not an object")
        ident, which, is_floor = item.get("id"), item.get("bucket"), item.get("floor")
        if not isinstance(ident, str) or which not in BUCKET_BLOCKS_PROVED or not isinstance(is_floor, bool):
            raise GateError("BUCKETS", f"malformed entry {item!r}")
        if ident in bucket:
            raise GateError("BUCKETS", f"{ident} is listed twice")
        bucket[ident] = which
        floor[ident] = is_floor

    missing = sorted(set(cat_floor) - set(bucket))
    extra = sorted(set(bucket) - set(cat_floor))
    if missing:
        raise GateError("BUCKETS", f"catalogue ids with no bucket: {', '.join(missing)}")
    if extra:
        raise GateError("BUCKETS", f"bucketed ids the catalogue lacks: {', '.join(extra)}")
    disagree = sorted(i for i in bucket if floor[i] != cat_floor[i])
    if disagree:
        raise GateError("BUCKETS", f"floor flag disagrees with the catalogue: {', '.join(disagree)}")
    missing_tcb = sorted(i for i in ALWAYS_ASSUMED if bucket.get(i) != "c")
    if missing_tcb:
        raise GateError("BUCKETS", f"tool-trust ids must stay in bucket c: {', '.join(missing_tcb)}")
    return Buckets(bucket=bucket, floor=floor, revision=revision, sha256=sha256_hex(raw),
                   catalogue_discharged=frozenset(cat_discharged))


# ---------------------------------------------------------------------------
# The claim ladder (VER31 section 4)
# ---------------------------------------------------------------------------


def rung_key(rung: str) -> tuple[int, int]:
    """Sort key for a rung name.  ``bounded(N)`` sorts by N within rank 2."""
    match = BOUNDED_RE.match(rung)
    if match:
        return (RANK["bounded"], int(match.group(1)))
    if rung == "exhaustive":
        return (RANK["exhaustive"], 10**12)  # above every bounded(N)
    if rung in RANK:
        return (RANK[rung], 0)
    raise GateError("LADDER", f"unknown rung {rung!r}")


def weakest(rungs: Iterable[str]) -> str:
    rungs = list(rungs)
    if not rungs:
        raise GateError("LADDER", "a package with no claims has no tier")
    return min(rungs, key=rung_key)


def _status(block: Any) -> str | None:
    """Return ``status`` of a ``{"status": ...}`` block, or None when absent."""
    if block is None:
        return None
    if isinstance(block, dict):
        return block.get("status")
    return "malformed"


def claim_rung(claim: dict, buckets: Buckets, signed_b: set[str]) -> dict:
    """Compute one claim's rung.  Returns a record for ``gate.json``.

    ``signed_b`` is the set of (b) ids covered by a *verified* statement
    sign-off for this package.  Nothing else can close a (b) id.
    """
    ident = claim.get("id", "?")
    if claim.get("example") != "pass" or _status(claim.get("property")) != "pass":
        raise GateError("LADDER", f"claim {ident}: example and property must pass for any rung")
    for optional in ("fuzz", "exhaustive"):
        status = _status(claim.get(optional))
        if status not in (None, "pass"):
            raise GateError("LADDER", f"claim {ident}: {optional} status is {status!r}")

    for field_name in ("id", "symbol"):
        value = claim.get(field_name)
        if not isinstance(value, str) or not SAFE_TEXT_RE.match(value):
            raise GateError("LADDER", f"claim {field_name} {value!r} is not plain text")
    proof = claim.get("proof")
    label = proof.get("label") if isinstance(proof, dict) else None
    if label is not None and (not isinstance(label, str) or not SAFE_TEXT_RE.match(label)):
        raise GateError("LADDER", f"claim {ident}: label {label!r} is not plain text")
    assumptions: list[str] = []
    discharged: list[str] = []
    if isinstance(proof, dict):
        assumptions = list(proof.get("assumptions") or [])
        discharged = list(proof.get("discharged") or [])
    for ident_a in assumptions + discharged:
        if ident_a not in buckets.bucket:
            raise GateError("LADDER", f"claim {ident}: assumption {ident_a!r} has no bucket")
    stray = sorted(set(discharged) - set(assumptions))
    if stray:
        raise GateError("LADDER", f"claim {ident}: discharged ids not in its assumptions: {', '.join(stray)}")
    # A discharge counts only if the code-owned catalogue records it too; the
    # report alone (or a weakened checker) cannot close an id.
    discharged = [d for d in discharged if d in buckets.catalogue_discharged]
    if isinstance(proof, dict) and proof.get("certificate") is not None:
        assumptions = sorted(set(assumptions) | set(ALWAYS_ASSUMED))

    open_ids = [a for a in assumptions if a not in set(discharged)]
    open_c = sorted(a for a in open_ids if buckets.bucket[a] == "c")
    open_b = sorted(a for a in open_ids if buckets.bucket[a] == "b" and a not in signed_b)
    open_a = sorted(a for a in open_ids if buckets.bucket[a] == "a" and not buckets.floor[a])
    certificate = proof.get("certificate") if isinstance(proof, dict) else None
    has_cert = isinstance(certificate, str) and bool(CERT_RE.match(certificate))

    rung = "tested"
    exhaustive = claim.get("exhaustive")
    if isinstance(exhaustive, dict) and exhaustive.get("status") == "pass" and exhaustive.get("domain") == "full":
        rung = "exhaustive"
    if label is None:
        pass
    elif BOUNDED_RE.match(label):
        if has_cert and rung_key(label) > rung_key(rung):
            rung = label
    elif label in PROVED_FAMILY or label == "proved-per-run":
        if has_cert:
            rung = "checked"
            # proved-per-run is evidence for one pinned build: at most `checked`
            # until VER31 section 4 is amended for it.
            if label != "proved-per-run" and not open_c and not open_b:
                rung = "proved"
                if not open_a:
                    rung = "proved, baseline discharged"
    else:
        raise GateError("LADDER", f"claim {ident}: unknown generator label {label!r}")

    by_bucket = {k: sorted(a for a in assumptions if buckets.bucket[a] == k) for k in "abc"}
    return {
        "id": ident,
        "symbol": claim.get("symbol"),
        "generator_label": label,
        "rung": rung,
        "certificate_kind": proof.get("certificate_kind") if isinstance(proof, dict) else None,
        "assumptions": by_bucket,
        "discharged": sorted(discharged),
        "open_c": open_c,
        "open_b": open_b,
        "undischarged_a_non_floor": open_a,
    }


def lane_claims(report: dict, lane: str) -> list[dict]:
    """A lane package's claims: every report claim at rung ``tested``.

    VER31 section 6.2: a live lane row gives each claim ``tested``; a lane
    carries no proofs, so it can never rise above that.
    """
    rows = [row for row in report.get("lanes") or [] if row.get("package") == lane]
    if len(rows) != 1:
        raise GateError("LANE", f"report names lane {lane} {len(rows)} times; need exactly one")
    if rows[0].get("rung") != "live":
        raise GateError("LANE", f"lane {lane} rung is {rows[0].get('rung')!r}, not live")
    out = []
    for claim in report.get("claims") or []:
        out.append(
            {
                "id": claim.get("id"),
                "symbol": claim.get("symbol"),
                "generator_label": "lane:live",
                "rung": "tested",
                "certificate_kind": None,
                "assumptions": {"a": [], "b": [], "c": []},
                "discharged": [],
                "open_c": [],
                "open_b": [],
                "undischarged_a_non_floor": [],
            }
        )
    if not out:
        raise GateError("LANE", "parent report has no claims")
    return out


def find_lane_report(repo: Path, lane: str) -> Path:
    """The single committed report whose ``lanes`` names ``lane``."""
    found = []
    for path in sorted((repo / "code" / "packages").glob("*/*/assurance_report.json")):
        try:
            doc = load_json(path)
        except GateError:
            continue
        if any(isinstance(r, dict) and r.get("package") == lane for r in doc.get("lanes") or []):
            found.append(path)
    if len(found) != 1:
        raise GateError("LANE", f"{len(found)} reports name lane {lane}; need exactly one")
    return found[0]


# ---------------------------------------------------------------------------
# Higher-tier conditions (VER31 section 6.1)
# ---------------------------------------------------------------------------


def higher_tier_conditions(report: dict, warrant_present: bool, jail_live: bool) -> list[dict]:
    """H1-H4.  ``warrant_present`` must already be the full H1 (``h1_holds``)."""
    rungs = report.get("rungs") or {}
    not_pass = sorted(k for k, v in rungs.items() if v != "pass")
    unapproved = [safe_text(e.get("id")) for e in report.get("exemptions") or [] if e.get("approved") is not True]
    verdict = report.get("verdict")
    h2_reason = f"verdict is {verdict}"
    if not_pass:
        h2_reason += f" ({', '.join(f'{k} {rungs[k]}' for k in not_pass)}"
        h2_reason += f"; {len(unapproved)} unapproved exemptions)" if unapproved else ")"
    elif unapproved:
        h2_reason += f" ({len(unapproved)} unapproved exemptions)"
    return [
        {"id": "H1", "name": "Warrant spec", "holds": warrant_present,
         "detail": "Warrant spec present; every claim's evidence is Warrant-derived" if warrant_present
         else "no Warrant spec, or a claim's evidence is not Warrant-derived"},
        {"id": "H2", "name": "report meets the bar (verdict pass)", "holds": verdict == "pass",
         "detail": h2_reason},
        {"id": "H3", "name": "live runtime-jail evidence", "holds": jail_live,
         "detail": "strict live jail run passed" if jail_live else "no live strict jail run for this commit"},
        {"id": "H4", "name": "no unapproved exemption", "holds": not unapproved,
         "detail": f"unapproved: {', '.join(unapproved)}" if unapproved else "none"},
    ]


# ---------------------------------------------------------------------------
# Floor checks (VER31 section 5)
# ---------------------------------------------------------------------------


def _check(ident: str, name: str, status: str, detail: str) -> dict:
    assert status in ("pass", "fail", "not-run")
    return {"id": ident, "name": name, "status": status, "detail": detail}


def parse_cargo(text: str) -> dict:
    """Parse a Cargo.toml with the standard library's TOML parser.

    The first version of this gate used a hand-written line scanner, and a
    security review found six valid manifests that declare a dependency it
    missed (a trailing comment on a header, a quoted table name, inline tables,
    dotted ``target."cfg(..)".dependencies`` keys). A real parser cannot be
    fooled by TOML syntax; malformed TOML is a refusal.
    """
    try:
        return tomllib.loads(text)
    except tomllib.TOMLDecodeError as exc:
        raise GateError("MANIFEST", f"Cargo.toml is not valid TOML: {exc}") from exc


def cargo_has_runtime_deps(text: str) -> list[str]:
    """Every dependency a Cargo.toml declares that would ship (F8).

    ``[dependencies]`` and ``[build-dependencies]`` (also spelled
    ``build_dependencies``), at the root and under every ``[target.<cfg>]``.
    ``[dev-dependencies]`` do not ship and are allowed.
    """
    doc = parse_cargo(text)
    found: list[str] = []

    def collect(table: dict, where: str) -> None:
        for key in ("dependencies", "build-dependencies", "build_dependencies"):
            deps = table.get(key)
            if isinstance(deps, dict):
                found.extend(f"{where}{key}: {name}" for name in deps)
            elif deps is not None:
                found.append(f"{where}{key}: malformed")

    collect(doc, "")
    targets = doc.get("target")
    if isinstance(targets, dict):
        for cfg, table in targets.items():
            if isinstance(table, dict):
                collect(table, f"target.{cfg}.")
    return sorted(found)


def cargo_package_field(text: str, key: str) -> str | None:
    """Read a string ``key`` from the ``[package]`` table of a Cargo.toml."""
    package = parse_cargo(text).get("package")
    value = package.get(key) if isinstance(package, dict) else None
    return value if isinstance(value, str) else None


@dataclass
class Manifest:
    lang: str
    registry_name: str
    version: str
    license: str | None
    runtime_deps: list[str]


def read_manifest(pkg: Path, lang: str) -> Manifest:
    """Read the package manifest from the repository tree."""
    if lang == "rust":
        return manifest_from_text("rust", read_capped(pkg / "Cargo.toml", MAX_TEXT_BYTES).decode("utf-8"))
    if lang == "typescript":
        return manifest_from_text("typescript", read_capped(pkg / "package.json", MAX_JSON_BYTES).decode("utf-8"))
    raise GateError("MANIFEST", f"unsupported language {lang}")


def manifest_from_text(lang: str, text: str) -> Manifest:
    """Parse a manifest's text: the tree's, or the one inside the artifact."""
    if lang == "rust":
        name = cargo_package_field(text, "name")
        version = cargo_package_field(text, "version")
        if not name or not version:
            raise GateError("MANIFEST", "Cargo.toml has no [package] name/version")
        return Manifest("rust", name, version, cargo_package_field(text, "license"), cargo_has_runtime_deps(text))
    if lang == "typescript":
        try:
            doc = json.loads(text)
        except ValueError as exc:
            raise GateError("MANIFEST", f"package.json is not valid JSON: {exc}") from exc
        deps = []
        for key in ("dependencies", "peerDependencies", "optionalDependencies", "bundleDependencies", "bundledDependencies"):
            value = doc.get(key)
            if value:
                deps.extend(f"{key}: {d}" for d in (value if isinstance(value, list) else value.keys()))
        if not isinstance(doc.get("name"), str) or not isinstance(doc.get("version"), str):
            raise GateError("MANIFEST", "package.json has no name/version")
        lic = doc.get("license")
        return Manifest("typescript", doc["name"], doc["version"], lic if isinstance(lic, str) else None, sorted(deps))
    raise GateError("MANIFEST", f"unsupported language {lang}")


def changelog_mentions(pkg: Path, version: str) -> bool:
    """True when the package changelog names ``version`` (F7)."""
    # "0.1" must not match "0.1.0", and "0.1.0" must not match "10.1.0".
    pattern = re.compile(rf"(?<![0-9.]){re.escape(version)}(?![0-9]|\.[0-9])")
    texts = []
    if (pkg / "CHANGELOG.md").is_file():
        texts.append(read_capped(pkg / "CHANGELOG.md", MAX_TEXT_BYTES).decode("utf-8", "replace"))
    shard_dir = pkg / "CHANGELOG.d"
    if (shard_dir / "_meta.md").is_file():
        for shard in sorted(shard_dir.glob("*.md")):
            texts.append(read_capped(shard, MAX_TEXT_BYTES).decode("utf-8", "replace"))
    return any(pattern.search(t) for t in texts)


def lcov_line_coverage(lcov_text: str, under: str, pkg_rel: str = "") -> tuple[int, int]:
    """Sum LH/LF over lcov records whose source file lies under ``under``.

    Restricting to the package's own ``src/`` keeps a workspace dependency's
    lines (test bootstraps, other crates) out of the package's figure.
    tarpaulin writes absolute ``SF:`` paths; vitest writes them relative to
    the package, so a relative path is read as ``<pkg_rel>/<path>``.
    """
    hit = found = 0
    current_ok = False
    for line in lcov_text.splitlines():
        if line.startswith("SF:"):
            path = line[3:].replace("\\", "/")
            if pkg_rel and not path.startswith("/") and not path.startswith(pkg_rel):
                path = f"{pkg_rel}/{path}"
            current_ok = path.find(under) != -1
        elif line.startswith("LF:") and current_ok:
            found += int(line[3:])
        elif line.startswith("LH:") and current_ok:
            hit += int(line[3:])
    return hit, found


def floor_static(pkg: Path, lang: str, version: str, report: dict, claims_from_lane: bool) -> list[dict]:
    """The floor checks the gate computes from files alone: F5-F9, F11 (part)."""
    manifest = read_manifest(pkg, lang)
    checks: list[dict] = []

    # F5: fuzz and property evidence on every claim.
    if claims_from_lane:
        # A lane carries no per-claim fuzz/property blocks, so it needs suites
        # of its own.  The package declares them as npm scripts; the workflow
        # runs every script in F1, so they pass only if F1 passed.
        scripts = load_json(pkg / "package.json").get("scripts") or {} if lang == "typescript" else {}
        missing = [s for s in ("test:property", "test:fuzz") if s not in scripts]
        if missing:
            checks.append(_check("F5", "fuzz and property tests", "fail",
                                 f"lane package lacks scripts: {', '.join(missing)}"))
        else:
            checks.append(_check("F5", "fuzz and property tests", "pass",
                                 "test:property and test:fuzz suites declared; run under F1"))
    else:
        missing = [c.get("id") for c in report.get("claims") or []
                   if _status(c.get("fuzz")) != "pass" or _status(c.get("property")) != "pass"]
        checks.append(_check("F5", "fuzz and property tests", "fail" if missing else "pass",
                             f"claims without passing fuzz+property: {', '.join(missing)}" if missing
                             else f"every claim ({len(report.get('claims') or [])}) has passing fuzz and property evidence"))

    # F6: a bench manifest bound to this source.
    bench = pkg / "evidence" / "bench-manifest.json"
    if not bench.is_file():
        checks.append(_check("F6", "recorded benchmarks", "fail", "no evidence/bench-manifest.json"))
    else:
        doc = load_json(bench)
        passing = [t for t in doc.get("tests") or [] if t.get("verdict") == "pass"]
        ok = doc.get("schema") == BENCH_SCHEMA and passing
        checks.append(_check("F6", "recorded benchmarks", "pass" if ok else "fail",
                             f"{len(passing)} passing benchmark entries" if ok
                             else "bench manifest has the wrong schema or no passing entry"))

    # F7: README and a changelog that names this version.
    has_changelog = (pkg / "CHANGELOG.md").is_file() or (pkg / "CHANGELOG.d" / "_meta.md").is_file()
    readme = (pkg / "README.md").is_file()
    named = has_changelog and changelog_mentions(pkg, version)
    checks.append(_check("F7", "README and CHANGELOG", "pass" if readme and named else "fail",
                         "README.md present; changelog names " + version if readme and named
                         else f"README.md {'present' if readme else 'missing'}; changelog "
                              + ("does not name " + version if has_changelog else "missing")))

    # F8: zero third-party runtime dependencies.
    checks.append(_check("F8", "zero third-party runtime dependencies",
                         "fail" if manifest.runtime_deps else "pass",
                         "declared: " + "; ".join(manifest.runtime_deps) if manifest.runtime_deps
                         else "no runtime dependency declared"))

    # F9: MIT licence, declared and present.
    licence_file = pkg / "LICENSE"
    licence_ok = manifest.license == "MIT" and licence_file.is_file() and not licence_file.is_symlink() \
        and b"MIT License" in read_capped(licence_file, MAX_TEXT_BYTES)
    checks.append(_check("F9", "licence", "pass" if licence_ok else "fail",
                         "MIT, LICENSE present" if licence_ok
                         else f"manifest licence {manifest.license!r}; LICENSE "
                              + ("present" if licence_file.is_file() else "missing")))

    # F11 (static half): tag version equals manifest version.
    checks.append(_check("F11a", "version matches manifest", "pass" if manifest.version == version else "fail",
                         f"tag {version}, manifest {manifest.version}"))
    return checks


# ---------------------------------------------------------------------------
# F4: the security-review record covers exactly what ships
# ---------------------------------------------------------------------------

# Repository-level build configuration a review must cover.  The workspace
# manifest is covered by its *settings* (``#settings``: the file with its
# ``members``/``exclude`` arrays removed), because adding any crate to the
# workspace rewrites those arrays and would make every review stale, while
# ``[patch]``, ``[profile]`` and the rest can change what a crate compiles to.
# The workspace ``Cargo.lock`` is not listed: F8 guarantees no runtime
# dependency, so the lockfile cannot change the shipped bytes.
SETTINGS_SUFFIX = "#settings"
REPO_BUILD_CONFIG = {
    "rust": ["code/packages/rust/Cargo.toml" + SETTINGS_SUFFIX],
    "typescript": [],
}


def workspace_settings(text: str) -> str:
    """A Cargo workspace manifest without its ``members``/``exclude`` arrays.

    Parsed, not pattern-matched: a regex over the text can be made to swallow
    lines after the array (review round 2), so the digest is taken over a
    canonical JSON rendering of the parsed manifest with the two arrays removed.
    """
    doc = parse_cargo(text)
    workspace = doc.get("workspace")
    if isinstance(workspace, dict):
        workspace.pop("members", None)
        workspace.pop("exclude", None)
    return json.dumps(doc, sort_keys=True, separators=(",", ":"), default=str)


def review_digest(repo: Path, key: str) -> str | None:
    """The digest a review record stores for ``key`` (a path, or path#settings)."""
    rel = key.removesuffix(SETTINGS_SUFFIX)
    target = repo / rel
    if target.is_symlink() or not target.is_file():
        return None
    data = read_capped(target, MAX_ARTIFACT_MEMBER_BYTES)
    if key.endswith(SETTINGS_SUFFIX):
        data = workspace_settings(data.decode("utf-8")).encode("utf-8")
    return sha256_hex(data)


def safe_rel(path: str) -> bool:
    """A normalized, repository-relative path: no `..`, glob, pathspec magic."""
    if not path or path.startswith(("/", ":")) or "\\" in path:
        return False
    if any(ch in path for ch in "*?[]"):
        return False
    parts = PurePosixPath(path).parts
    return all(p not in ("", ".", "..") for p in parts) and str(PurePosixPath(path)) == path


def required_review_scope(repo: Path, pkg_rel: str, lang: str, artifact_files: list[str]) -> list[str]:
    """Every file the review must cover (VER31 F4), as repository keys.

    * every git-tracked file under the package (source, tests, BUILD files,
      tsconfig/vitest config, manifests, lockfile, evidence): build and test
      configuration decides what ships and what the evidence means;
    * the artifact's own members, mapped back to the repository;
    * build configuration on the package's ancestor path: ``.cargo/**``,
      ``rust-toolchain`` / ``rust-toolchain.toml`` and ``.npmrc`` in the
      repository root and every directory down to the package;
    * the workspace manifest's settings (``#settings``, see REPO_BUILD_CONFIG).

    ``security-review.json`` is the one exclusion.  Only tracked files count:
    build outputs (``dist/``, ``Cargo.toml.orig``) are covered through the
    tracked source they are built from.
    """
    candidates: set[str] = {f"{pkg_rel}/{rel}" for rel in artifact_files}
    parts = PurePosixPath(pkg_rel).parts
    ancestors = [""] + ["/".join(parts[: i + 1]) for i in range(len(parts))]
    pathspecs = [pkg_rel]
    for anc in ancestors:
        prefix = f"{anc}/" if anc else ""
        pathspecs += [f"{prefix}.cargo", f"{prefix}rust-toolchain", f"{prefix}rust-toolchain.toml",
                      f"{prefix}.npmrc"]
    tracked_listing = git(repo, "--literal-pathspecs", "ls-files", "--", *pathspecs).splitlines()
    candidates.update(tracked_listing)
    tracked = set(git(repo, "--literal-pathspecs", "ls-files", "--", *sorted(candidates)).splitlines())
    tracked.discard(f"{pkg_rel}/security-review.json")
    settings = [e for e in REPO_BUILD_CONFIG.get(lang, []) if (repo / e.removesuffix(SETTINGS_SUFFIX)).is_file()]
    return sorted(tracked) + settings


def check_review(repo: Path, pkg_rel: str, required: list[str]) -> dict:
    """F4.  The record binds *content*: path -> SHA-256 at review time.

    VER31 first bound the review to a commit on ``main``.  The repository
    squash-merges, so a feature-branch commit never lands on ``main``;
    binding the reviewed bytes themselves is both squash-proof and stricter.
    """
    path = repo / pkg_rel / "security-review.json"
    if not path.is_file():
        return _check("F4", "security review", "fail", "no security-review.json")
    try:
        doc = load_json(path)
    except GateError as exc:
        return _check("F4", "security review", "fail", exc.message)
    if doc.get("schema") != REVIEW_SCHEMA:
        return _check("F4", "security review", "fail", f"schema must be {REVIEW_SCHEMA}")
    if doc.get("outcome") != "pass":
        return _check("F4", "security review", "fail", f"outcome is {doc.get('outcome')!r}")
    for finding in doc.get("findings") or []:
        sev = str(finding.get("severity", "")).lower()
        if sev in ("high", "critical") and finding.get("status") not in ("fixed", "not-applicable"):
            return _check("F4", "security review", "fail", f"open {sev} finding {finding.get('id')}")
    files = doc.get("files")
    if not isinstance(files, dict):
        return _check("F4", "security review", "fail", "files must map path -> sha256")
    for rel in files:
        if not safe_rel(rel.removesuffix(SETTINGS_SUFFIX)):
            return _check("F4", "security review", "fail", f"unsafe scope path {rel!r}")
    uncovered = [p for p in required if p not in files]
    if uncovered:
        return _check("F4", "security review", "fail", f"review does not cover: {', '.join(uncovered)}")
    changed = []
    for rel, digest in sorted(files.items()):
        actual = review_digest(repo, rel)
        if actual is None:
            changed.append(f"{rel} (missing or symlink)")
        elif actual != digest:
            changed.append(rel)
    if changed:
        return _check("F4", "security review", "fail", f"changed since review: {', '.join(changed)}")
    return _check("F4", "security review", "pass",
                  f"reviewed {doc.get('date')} by {doc.get('reviewer')}; {len(files)} files unchanged since")


# ---------------------------------------------------------------------------
# Artifact inspection (F10): the report ships, byte for byte
# ---------------------------------------------------------------------------


def artifact_members(artifact: Path) -> dict[str, bytes]:
    """Read every regular file of a ``.crate`` / ``.tgz`` without extracting.

    The leading directory (``<name>-<version>/`` for a crate, ``package/``
    for npm) is stripped.  Links, absolute names, ``..`` and oversize members
    are refused: we inspect, we never write to disk.
    """
    out: dict[str, bytes] = {}
    with tarfile.open(artifact, "r:gz") as tar:
        members = tar.getmembers()
        if len(members) > MAX_ARTIFACT_MEMBERS:
            raise GateError("ARTIFACT", "too many members")
        for member in members:
            if member.isdir():
                continue
            if not member.isfile():
                raise GateError("ARTIFACT", f"non-regular member {member.name}")
            parts = PurePosixPath(member.name).parts
            if member.name.startswith("/") or ".." in parts or len(parts) < 2:
                raise GateError("ARTIFACT", f"unsafe member name {member.name}")
            if member.size > MAX_ARTIFACT_MEMBER_BYTES:
                raise GateError("ARTIFACT", f"member {member.name} too large")
            handle = tar.extractfile(member)
            assert handle is not None
            out[str(PurePosixPath(*parts[1:]))] = handle.read()
    return out


def check_artifact(members: dict[str, bytes], report_bytes: bytes, assurance_bytes: bytes) -> dict:
    problems = []
    if members.get("assurance_report.json") != report_bytes:
        problems.append("assurance_report.json missing or not byte-identical")
    if members.get("ASSURANCE.md") != assurance_bytes:
        problems.append("ASSURANCE.md missing or not byte-identical")
    if "LICENSE" not in members:
        problems.append("LICENSE missing")
    return _check("F10b", "report ships verbatim", "fail" if problems else "pass",
                  "; ".join(problems) if problems else "assurance_report.json and ASSURANCE.md byte-identical")


def check_shipped_manifest(members: dict[str, bytes], lang: str, tree: Manifest, version: str) -> dict:
    """F8b.  The manifest *inside the artifact* is what the registry reads.

    A build step (``prepack``, ``npm run build``) could rewrite
    ``package.json`` after the tree was checked, so the shipped manifest's
    name, version, licence and runtime dependencies must equal the tree's and
    the tag's.  For a crate this is Cargo's normalized ``Cargo.toml``.
    """
    name = "Cargo.toml" if lang == "rust" else "package.json"
    raw = members.get(name)
    if raw is None:
        return _check("F8b", "shipped manifest", "fail", f"artifact has no {name}")
    try:
        shipped = manifest_from_text(lang, raw.decode("utf-8"))
    except (GateError, UnicodeDecodeError) as exc:
        return _check("F8b", "shipped manifest", "fail", f"cannot read shipped {name}: {exc}")
    problems = []
    if shipped.registry_name != tree.registry_name:
        problems.append(f"name {shipped.registry_name!r} != tree {tree.registry_name!r}")
    if shipped.version != version:
        problems.append(f"version {shipped.version!r} != tag {version!r}")
    if shipped.license != "MIT":
        problems.append(f"licence {shipped.license!r}")
    if shipped.runtime_deps:
        problems.append("runtime dependencies: " + "; ".join(shipped.runtime_deps))
    return _check("F8b", "shipped manifest", "fail" if problems else "pass",
                  "; ".join(problems) if problems else f"{shipped.registry_name} {shipped.version}, MIT, no runtime deps")


# ---------------------------------------------------------------------------
# Signatures (VER31 sections 3.2, 9.1, 9.6)
# ---------------------------------------------------------------------------


def _ssh_string(buf: bytes, pos: int) -> tuple[bytes, int]:
    if pos + 4 > len(buf):
        raise GateError("SIG", "truncated SSH string")
    length = int.from_bytes(buf[pos:pos + 4], "big")
    end = pos + 4 + length
    if end > len(buf):
        raise GateError("SIG", "truncated SSH string")
    return buf[pos + 4:end], end


def parse_sshsig(armored: str) -> dict:
    """Parse an armored SSHSIG (OpenSSH PROTOCOL.sshsig) far enough to read
    the key type, namespace and, for a security key, the flags byte.

    Layout::

        "SSHSIG" | uint32 version | string publickey | string namespace
                 | string reserved | string hash_alg | string signature

    and for an ``sk-*`` key the signature string is::

        string sig_type | string raw_sig | byte flags | uint32 counter

    ``ssh-keygen -Y verify`` checks the cryptography; this parser only reads
    the flags it does not enforce (user verification, 0x04).
    """
    lines = [ln.strip() for ln in armored.strip().splitlines()]
    if not lines or lines[0] != "-----BEGIN SSH SIGNATURE-----" or lines[-1] != "-----END SSH SIGNATURE-----":
        raise GateError("SIG", "not an armored SSH signature")
    try:
        blob = base64.b64decode("".join(lines[1:-1]), validate=True)
    except ValueError as exc:
        raise GateError("SIG", "bad base64 in SSH signature") from exc
    if blob[:6] != b"SSHSIG":
        raise GateError("SIG", "missing SSHSIG magic")
    pos = 10  # magic + uint32 version
    pubkey, pos = _ssh_string(blob, pos)
    namespace, pos = _ssh_string(blob, pos)
    _reserved, pos = _ssh_string(blob, pos)
    _hash, pos = _ssh_string(blob, pos)
    signature, pos = _ssh_string(blob, pos)
    key_type, _ = _ssh_string(pubkey, 0)
    sig_type, spos = _ssh_string(signature, 0)
    flags = None
    if sig_type.decode(errors="replace").startswith("sk-"):
        _raw, spos = _ssh_string(signature, spos)
        if spos + 5 > len(signature):
            raise GateError("SIG", "truncated security-key signature")
        flags = signature[spos]
    return {
        "key_type": key_type.decode(errors="replace"),
        "namespace": namespace.decode(errors="replace"),
        "flags": flags,
    }


def require_hardware_flags(sig: dict) -> None:
    if sig["key_type"] not in SK_KEY_TYPES or sig["flags"] is None:
        raise GateError("SIG", f"signature is not from a security key ({sig['key_type']})")
    missing = []
    if not sig["flags"] & SK_FLAG_USER_PRESENCE:
        missing.append("user presence")
    if not sig["flags"] & SK_FLAG_USER_VERIFIED:
        missing.append("user verification (PIN)")
    if missing:
        raise GateError("SIG", f"security-key signature lacks {' and '.join(missing)}")


def parse_signers(text: str) -> tuple[int, list[dict]]:
    """Parse ``.github/release-signers`` (OpenSSH allowed_signers format).

    Returns ``(revision, entries)``.  Refuses: no ``# revision: N`` line; a
    key type that is not a security key; any option other than
    ``namespaces=``; an empty file.  OpenSSH's allowed_signers format has no
    ``verify-required`` option, so user verification is enforced by
    ``require_hardware_flags`` instead.
    """
    revision = None
    entries = []
    for raw in text.splitlines():
        line = raw.strip()
        match = re.match(r"^#\s*revision:\s*([0-9]{1,9})\s*$", line)
        if match:
            revision = int(match.group(1))
            continue
        if not line or line.startswith("#"):
            continue
        fields = line.split()
        if len(fields) < 3:
            raise GateError("SIGNERS", f"malformed signers line: {line[:60]}")
        principal = fields[0]
        if fields[1] in SK_KEY_TYPES:
            options, key_type, key = "", fields[1], fields[2]
        elif len(fields) >= 4 and fields[2] in SK_KEY_TYPES:
            options, key_type, key = fields[1], fields[2], fields[3]
        else:
            raise GateError("SIGNERS", f"signer {principal} is not a security-key (sk-*) key")
        for opt in [o for o in options.split(",") if o]:
            if not opt.lower().startswith("namespaces="):
                raise GateError("SIGNERS", f"signer {principal}: unsupported option {opt}")
        entries.append({"principal": principal, "options": options, "key_type": key_type, "key": key})
    if revision is None:
        raise GateError("SIGNERS", "signers file has no '# revision: N' line")
    if not entries:
        raise GateError("SIGNERS", "signers file lists no key")
    return revision, entries


def ssh_verify(signers_text: str, principal: str, namespace: str, data: bytes, signature: str) -> bool:
    """Verify ``signature`` over ``data`` with ``ssh-keygen -Y verify``."""
    with tempfile.TemporaryDirectory() as tmp:
        allowed = Path(tmp) / "allowed_signers"
        sig = Path(tmp) / "data.sig"
        allowed.write_text(signers_text, encoding="utf-8")
        sig.write_text(signature, encoding="utf-8")
        proc = run(["ssh-keygen", "-Y", "verify", "-f", str(allowed), "-I", principal,
                    "-n", namespace, "-s", str(sig)], Path(tmp), stdin=data)
        return proc.returncode == 0


def verify_signed_file(signers_text: str, data: bytes, signature: str, namespace: str) -> str:
    """Verify a detached hardware-key signature; return the signing principal."""
    sig = parse_sshsig(signature)
    if sig["namespace"] != namespace:
        raise GateError("SIG", f"signature namespace {sig['namespace']!r}, expected {namespace!r}")
    require_hardware_flags(sig)
    _rev, entries = parse_signers(signers_text)
    for entry in entries:
        if ssh_verify(signers_text, entry["principal"], namespace, data, signature):
            return entry["principal"]
    raise GateError("SIG", "signature does not verify against any listed signer")


def history(repo: Path, ref: str, rel: str) -> list[str]:
    """Commits on ``ref``'s first-parent history that touched ``rel``, oldest first."""
    out = git(repo, "log", "--first-parent", "--format=%H", ref, "--", rel)
    return list(reversed([ln for ln in out.splitlines() if ln]))


def show(repo: Path, commit: str, rel: str) -> bytes | None:
    proc = run(["git", "show", f"{commit}:{rel}"], repo)
    return proc.stdout if proc.returncode == 0 else None


def verify_signers_chain(repo: Path, ref: str) -> tuple[str, list[dict]]:
    """Walk ``ref``'s first-parent history of the signers file (VER31 9.1).

    The first version is the trust anchor.  Every later version must carry
    the next revision number and be signed (``.github/release-signers.sig``,
    namespace ``coding-adventures-signers``) by a key of the *previous*
    version, so no key can authorize itself and no old version can return.
    Returns the current file text and its entries.
    """
    commits = history(repo, ref, SIGNERS_REL)
    if not commits:
        raise GateError("SIGNERS", f"{SIGNERS_REL} has no history on {ref}")
    previous_text = None
    previous_rev = 0
    current_text = ""
    for commit in commits:
        blob = show(repo, commit, SIGNERS_REL)
        if blob is None:
            raise GateError("SIGNERS", f"{SIGNERS_REL} deleted at {commit[:12]}")
        text = blob.decode("utf-8")
        placeholder = not re.search(r"^#\s*revision:\s*[0-9]", text, re.MULTILINE) and all(
            not ln.strip() or ln.strip().startswith("#") for ln in text.splitlines()
        )
        if placeholder and previous_text is None:
            continue  # the comment-only placeholder committed before the anchor
        revision, _entries = parse_signers(text)
        if revision != previous_rev + 1:
            raise GateError("SIGNERS", f"revision {revision} at {commit[:12]} does not follow {previous_rev}")
        if previous_text is not None:
            sig = show(repo, commit, SIGNERS_REL + ".sig")
            if sig is None:
                raise GateError("SIGNERS", f"revision {revision} has no .sig")
            verify_signed_file(previous_text, blob, sig.decode("utf-8"), NS_SIGNERS)
        previous_text, previous_rev, current_text = text, revision, text
    if previous_text is None:
        raise GateError("SIGNERS", "no signer has been registered yet")
    return current_text, parse_signers(current_text)[1]


def verify_buckets_signature(repo: Path, ref: str, buckets: Buckets, signers_text: str) -> str:
    """The bucket file must not roll back and must be signed (VER31 3, 9.6).

    Rollback is checked first: it needs no key, and an old file restored with
    its old, valid signature is exactly the case a signature check alone misses.
    """
    highest = 0
    for commit in history(repo, ref, BUCKETS_REL):
        blob = show(repo, commit, BUCKETS_REL)
        if blob is None:
            continue
        try:
            highest = max(highest, int(json.loads(blob).get("revision", 0)))
        except (ValueError, TypeError, AttributeError):
            continue
    if buckets.revision < highest:
        raise GateError("BUCKETS-SIG", f"bucket revision {buckets.revision} is lower than {highest} seen on {ref}")
    data = read_capped(repo / BUCKETS_REL, MAX_JSON_BYTES)
    sig_path = repo / (BUCKETS_REL + ".sig")
    if not sig_path.is_file():
        raise GateError("BUCKETS-SIG", "buckets-v1.json.sig is missing; the owner has not signed the bucket file")
    return verify_signed_file(signers_text, data, sig_path.read_text("utf-8"), NS_BUCKETS)


def verify_signoff(repo: Path, warrant_rel: str, signers_text: str, package: str | None = None) -> set[str]:
    """Return the (b) ids a valid statement sign-off covers (VER31 3.2).

    Any defect returns the empty set: a broken sign-off leaves (b) open, it
    never raises a tier.  The Warrant file at the tagged commit is tied to the
    evidence by the package BUILD's ``gen_evidence --check`` (a drift check),
    which the gate re-runs under F10a, so binding the sign-off to that file's
    digest binds it to the evidence too.
    """
    stem = PurePosixPath(warrant_rel).stem
    rec_path = repo / "code/specs/signoffs" / f"{stem}.json"
    sig_path = rec_path.with_name(rec_path.name + ".sig")
    if not rec_path.is_file() or not sig_path.is_file():
        return set()
    try:
        raw = read_capped(rec_path, MAX_JSON_BYTES)
        doc = json.loads(raw)
        if doc.get("schema") != SIGNOFF_SCHEMA or doc.get("warrant") != warrant_rel:
            return set()
        if package is not None and doc.get("package") != package:
            return set()
        actual = sha256_hex(read_capped(repo / warrant_rel, MAX_TEXT_BYTES))
        if doc.get("warrant_sha256") != actual:
            return set()
        signer = doc.get("signer")
        sig_text = sig_path.read_text("utf-8")
        sig = parse_sshsig(sig_text)
        if sig["namespace"] != NS_STATEMENT:
            return set()
        require_hardware_flags(sig)
        if not isinstance(signer, str) or not ssh_verify(signers_text, signer, NS_STATEMENT, raw, sig_text):
            return set()
        return {i for i in doc.get("ids") or [] if isinstance(i, str) and i.startswith("spec.")}
    except (GateError, ValueError, UnicodeDecodeError):
        return set()


def verify_tag(repo: Path, tag: str, commit: str, signers_text: str) -> str:
    """VER31 9.1: an annotated, hardware-key-signed tag at ``commit``.

    Steps: the ref is an annotated tag object (lightweight tags are refused);
    it peels to ``commit``; ``git verify-tag`` accepts it against the signers
    file from ``main``; its SSHSIG carries user-presence and user-verification
    flags.
    """
    if git(repo, "cat-file", "-t", f"refs/tags/{tag}").strip() != "tag":
        raise GateError("TAG", f"{tag} is a lightweight tag; an annotated signed tag is required")
    peeled = git(repo, "rev-parse", f"refs/tags/{tag}^{{commit}}").strip()
    if peeled != commit:
        raise GateError("TAG", f"{tag} points at {peeled[:12]}, not the release commit {commit[:12]}")
    body = git(repo, "cat-file", "tag", f"refs/tags/{tag}")
    # The signature covers these headers, but `git verify-tag` does not compare
    # them with the ref name.  Without this check one signed tag object could
    # be pushed under another ref (another package or version) and replayed.
    headers = dict(line.split(" ", 1) for line in body.split("\n\n", 1)[0].splitlines() if " " in line)
    if headers.get("object") != commit or headers.get("type") != "commit" or headers.get("tag") != tag:
        raise GateError("TAG", f"tag object headers {headers.get('tag')!r}/{headers.get('type')!r} do not name "
                               f"{tag} at {commit[:12]}")
    with tempfile.TemporaryDirectory() as tmp:
        allowed = Path(tmp) / "allowed_signers"
        allowed.write_text(signers_text, encoding="utf-8")
        proc = run(["git", "-c", "gpg.format=ssh", "-c", f"gpg.ssh.allowedSignersFile={allowed}",
                    "verify-tag", f"refs/tags/{tag}"], repo)
        if proc.returncode != 0:
            raise GateError("TAG", f"git verify-tag failed: {proc.stderr.decode(errors='replace').strip()}")
    start = body.find("-----BEGIN SSH SIGNATURE-----")
    if start == -1:
        raise GateError("TAG", "tag has no SSH signature")
    sig = parse_sshsig(body[start:])
    if sig["namespace"] != NS_TAG:
        raise GateError("TAG", f"tag signature namespace {sig['namespace']!r}")
    require_hardware_flags(sig)
    return "verified"


# ---------------------------------------------------------------------------
# ASSURANCE.md: the bucketed assumption list that ships in the package
# ---------------------------------------------------------------------------


def render_assurance(name: str, claims: list[dict], report: dict, report_sha: str,
                     buckets: Buckets, lane: str | None) -> str:
    """Render ``ASSURANCE.md`` deterministically from report + buckets.

    Deterministic, because the file is committed and the gate requires the
    committed copy to equal a fresh rendering (like ``gen_evidence --check``).
    It holds no run-dependent value (no coverage figure, no date).
    """
    out = [f"# Assurance: {name}", ""]
    out.append("Generated by `code/scripts/release_gate.py assurance` (VER31). Do not edit by hand.")
    out.append("")
    out.append(f"- Assurance report: `assurance_report.json` (sha256 `{report_sha}`), verdict `{report.get('verdict')}`.")
    out.append(f"- Assumption buckets: `{BUCKETS_REL}` revision {buckets.revision} (sha256 `{buckets.sha256}`).")
    if lane:
        out.append(f"- This package is the `{lane}` lane of that report: every claim is at most `tested` "
                   "(VER31 section 6.2). The assumption lists below are the Rust reference implementation's.")
    out.append("")
    out.append("## Claims")
    out.append("")
    out.append("| Claim | Symbol | Generator label | Computed rung | Open (c) | Open (b) |")
    out.append("|---|---|---|---|---|---|")
    for c in claims:
        out.append(f"| `{c['id']}` | `{c['symbol']}` | `{c['generator_label']}` | **{c['rung']}** "
                   f"| {len(c['open_c'])} | {len(c['open_b'])} |")
    out.append("")
    ref_claims = [claim_rung(c, buckets, set()) for c in report.get("claims") or []] if lane else claims
    union = {k: sorted({a for c in ref_claims for a in c["assumptions"][k]}) for k in "abc"}
    out.append("## Assumptions (like Lean's `#print axioms`)")
    out.append("")
    out.append("Every assumption any claim relies on, grouped by VER31 bucket. Bucket (a) is accepted and "
               "does not block `proved`; (b) needs the owner's signed sign-off of the statement; (c) is "
               "this repository's own gap and blocks `proved` until checked evidence discharges it.")
    for key in "abc":
        out.append("")
        out.append(f"### ({key}) {BUCKET_TITLES[key]}: {len(union[key])} ids")
        out.append("")
        for ident in union[key]:
            floor = " (floor: can never be discharged)" if buckets.floor.get(ident) else ""
            out.append(f"- `{ident}`{floor}")
    report_level = sorted(safe_text(i) for i in report.get("assumptions") or [])
    out.append("")
    out.append(f"### Report-level assumptions: {len(report_level)} ids")
    out.append("")
    out.append("These qualify the weakness matrix and capability rows, not a claim's rung.")
    for key in "abc":
        ids = [i for i in report_level if buckets.bucket.get(i) == key]
        if ids:
            out.append("")
            out.append(f"- ({key}) {BUCKET_TITLES[key]}: " + ", ".join(f"`{i}`" for i in ids))
    out.append("")
    out.append("## Exemptions")
    out.append("")
    exemptions = report.get("exemptions") or []
    if not exemptions:
        out.append("None.")
    for e in exemptions:
        out.append(f"- `{safe_text(e.get('id'))}`: {'approved' if e.get('approved') is True else 'unapproved'}")
    out.append("")
    return "\n".join(out)


# ---------------------------------------------------------------------------
# Assembling gate.json
# ---------------------------------------------------------------------------


@dataclass
class Context:
    repo: Path
    pkg_rel: str
    lang: str
    version: str
    dry_run: bool
    signers_ref: str
    tag: str | None = None
    commit: str | None = None
    artifact: Path | None = None
    lcov: Path | None = None
    tests_ran: bool = False
    lint_ran: bool = False
    report_check_ran: bool = False
    evidence_reproduced: bool = False
    jail_live: bool = False
    registry_has_version: bool | None = None
    trusted_publishing: bool = False
    notes: list[str] = field(default_factory=list)


def package_report(ctx: Context) -> tuple[Path, str | None]:
    """The report this package is judged by, and its lane name if a lane."""
    own = ctx.repo / ctx.pkg_rel / "assurance_report.json"
    if ctx.lang == "rust":
        return own, None
    lane = f"{ctx.lang}/{PurePosixPath(ctx.pkg_rel).name}"
    return find_lane_report(ctx.repo, lane), lane


def compute_claims(ctx: Context, report: dict, buckets: Buckets, lane: str | None, signed_b: set[str]) -> list[dict]:
    if lane:
        return lane_claims(report, lane)
    return [claim_rung(c, buckets, signed_b) for c in report.get("claims") or []]


def warrant_path(repo: Path, report_path: Path) -> str | None:
    """The package's Warrant file: the ``spec`` directive of its bindings.

    Only the directive the evidence generator itself reads counts (a path in a
    comment does not), and exactly one is allowed.
    """
    bindings = report_path.parent / "evidence.bindings"
    if not bindings.is_file():
        return None
    text = read_capped(bindings, MAX_TEXT_BYTES).decode("utf-8", "replace")
    specs = re.findall(r"^spec[ \t]+(\S+)[ \t]*$", text, re.MULTILINE)
    if len(specs) != 1 or not re.match(r"^code/specs/[A-Za-z0-9_.-]+\.warrant$", specs[0]):
        return None
    return specs[0] if (repo / specs[0]).is_file() else None


def h1_holds(report: dict, warrant: str | None) -> bool:
    """H1 (VER31 6.1): a Warrant spec, and every claim's evidence derives from it."""
    claims = report.get("claims") or []
    return warrant is not None and bool(claims) and all(
        c.get("evidence") and all(isinstance(e, str) and e.startswith("warrant/") for e in c["evidence"])
        for c in claims
    )


def run_check(ctx: Context) -> dict:
    repo = ctx.repo
    pkg = repo / ctx.pkg_rel
    if not VERSION_RE.match(ctx.version):
        raise GateError("INPUT", f"bad version {ctx.version!r}")
    buckets = load_buckets(repo / BUCKETS_REL, repo / CATALOGUE_REL)
    report_path, lane = package_report(ctx)
    report_bytes = read_capped(report_path, MAX_JSON_BYTES)
    report = json.loads(report_bytes)
    manifest = read_manifest(pkg, ctx.lang)
    expected_package = "/".join(PurePosixPath(report_path.relative_to(repo)).parts[2:4])
    if report.get("package") != expected_package:
        raise GateError("REPORT", f"report names package {report.get('package')!r}, but sits in {expected_package}")

    # -- Authorization (section 9).  Required unless this is a dry run.
    auth: list[dict] = []
    signers_text = None
    try:
        signers_text, entries = verify_signers_chain(repo, ctx.signers_ref)
        anchor = history(repo, ctx.signers_ref, SIGNERS_REL)
        keys = ", ".join(f"{e['principal']} {e['key_type']} ...{e['key'][-16:]}" for e in entries)
        auth.append(_check("A1", "signers chain", "pass",
                           f"signers: {keys}; check the chain's anchor, first commit {anchor[0][:12]}"))
    except GateError as exc:
        auth.append(_check("A1", "signers chain", "fail", exc.message))
    if signers_text is not None:
        try:
            who = verify_buckets_signature(repo, ctx.signers_ref, buckets, signers_text)
            auth.append(_check("A2", "bucket file signed", "pass", f"signed by {who}"))
        except GateError as exc:
            auth.append(_check("A2", "bucket file signed", "fail", exc.message))
        if ctx.tag:
            try:
                verify_tag(repo, ctx.tag, ctx.commit or "", signers_text)
                auth.append(_check("A3", "hardware-key-signed tag", "pass", ctx.tag))
            except GateError as exc:
                auth.append(_check("A3", "hardware-key-signed tag", "fail", exc.message))
        else:
            auth.append(_check("A3", "hardware-key-signed tag", "not-run", "no tag (dry run)"))
    else:
        auth.append(_check("A2", "bucket file signed", "not-run", "no valid signers file"))
        auth.append(_check("A3", "hardware-key-signed tag", "not-run", "no valid signers file"))

    warrant = warrant_path(repo, report_path)
    signed_b = set()
    if signers_text is not None and warrant is not None and not lane:
        # Bind the sign-off to the package directory the gate is judging, not
        # to the report's own `package` field, which the report controls.
        signed_b = verify_signoff(repo, warrant, signers_text, "/".join(PurePosixPath(ctx.pkg_rel).parts[2:4]))
    claims = compute_claims(ctx, report, buckets, lane, signed_b)
    claim_tier = weakest(c["rung"] for c in claims)
    conditions = higher_tier_conditions(report, h1_holds(report, warrant), ctx.jail_live)

    # -- Floor checks.
    floor: list[dict] = []
    floor.append(_check("F1", "tests pass", "pass" if ctx.tests_ran else "not-run",
                        "package BUILD ran in this job" if ctx.tests_ran else "tests not run by the gate"))
    if ctx.lcov is not None:
        hit, found = lcov_line_coverage(read_capped(ctx.lcov, 64 * 1024 * 1024).decode("utf-8", "replace"),
                                        f"{ctx.pkg_rel}/src/", ctx.pkg_rel)
        pct = (100.0 * hit / found) if found else 0.0
        floor.append(_check("F2", "line coverage >= 80%", "pass" if found and pct >= 80.0 else "fail",
                            f"{pct:.1f}% ({hit}/{found} lines; library target 95%)"))
    else:
        floor.append(_check("F2", "line coverage >= 80%", "not-run", "no coverage data"))
    floor.append(_check("F3", "lint", "pass" if ctx.lint_ran else "not-run",
                        "clippy -D warnings / tsc strict ran in this job" if ctx.lint_ran else "lint not run"))
    floor.extend(floor_static(pkg, ctx.lang, ctx.version, report, claims_from_lane=lane is not None))

    # ASSURANCE.md is committed, so it is rendered without any sign-off (the
    # same way `assurance` renders it); the release notes give the signed state.
    unsigned_claims = compute_claims(ctx, report, buckets, lane, set())
    expected_assurance = render_assurance(manifest.registry_name, unsigned_claims, report,
                                          sha256_hex(report_bytes), buckets, lane).encode("utf-8")
    committed = pkg / "ASSURANCE.md"
    fresh = committed.is_file() and read_capped(committed, MAX_TEXT_BYTES) == expected_assurance
    if not fresh:
        floor.append(_check("F10a", "report checked and fresh", "fail",
                            "ASSURANCE.md is missing or stale: run release_gate.py assurance"))
    elif not (ctx.report_check_ran and ctx.evidence_reproduced):
        floor.append(_check("F10a", "report checked and fresh", "not-run",
                            "report --check / evidence reproduction not run"))
    else:
        floor.append(_check("F10a", "report checked and fresh", "pass",
                            "ca-assurance-report --check passed and the package BUILD reproduced its "
                            "evidence byte-for-byte"))
    if ctx.artifact is not None:
        members = artifact_members(ctx.artifact)
        floor.append(check_artifact(members, report_bytes, expected_assurance))
        floor.append(check_shipped_manifest(members, ctx.lang, manifest, ctx.version))
        required = required_review_scope(repo, ctx.pkg_rel, ctx.lang, sorted(members))
        floor.append(check_review(repo, ctx.pkg_rel, required))
        artifact_info = {"file": ctx.artifact.name, "sha256": sha256_hex(ctx.artifact.read_bytes())}
    else:
        floor.append(_check("F10b", "report ships verbatim", "not-run", "no artifact built"))
        floor.append(_check("F4", "security review", "not-run", "no artifact to scope the review"))
        artifact_info = None

    # Release tier: claim tier only when H1-H4 all hold, else Tested.
    capped = not all(h["holds"] for h in conditions) or lane is not None
    release_tier = "tested" if capped else claim_tier
    major = int(ctx.version.split(".")[0])
    if major >= 1 and rung_key(release_tier) < rung_key("proved"):
        floor.append(_check("F11b", "version major", "fail",
                            f"version {ctx.version} needs release tier proved or higher; tier is {release_tier}"))
    else:
        floor.append(_check("F11b", "version major", "pass", f"major {major} allowed at tier {release_tier}"))
    if ctx.registry_has_version is None:
        floor.append(_check("F11c", "version not yet published", "not-run", "registry not queried"))
    else:
        floor.append(_check("F11c", "version not yet published", "fail" if ctx.registry_has_version else "pass",
                            "already on the registry" if ctx.registry_has_version else "not on the registry"))
    floor.append(_check("F12", "trusted publishing and provenance", "pass" if ctx.trusted_publishing else "not-run",
                        "OIDC publish job and build-provenance attestation in publish.yml"
                        if ctx.trusted_publishing else "checked by the workflow, not in a local run"))
    floor.sort(key=lambda c: (int(re.match(r"F([0-9]+)", c["id"]).group(1)), c["id"]))

    failures = [c for c in floor if c["status"] == "fail"]
    incomplete = [c for c in floor if c["status"] == "not-run"]
    auth_fail = [c for c in auth if c["status"] != "pass"]
    if failures:
        decision = "refused"
    elif incomplete:
        decision = "incomplete"
    elif auth_fail and not ctx.dry_run:
        decision = "refused"
    else:
        decision = "dry-run-pass" if ctx.dry_run else "publishable"

    return {
        "schema": GATE_SCHEMA,
        "package": ctx.pkg_rel,
        "language": ctx.lang,
        "registry_name": manifest.registry_name,
        "version": ctx.version,
        "commit": ctx.commit,
        "tag": ctx.tag,
        "dry_run": ctx.dry_run,
        "lane": lane,
        "inputs": {
            "report": str(report_path.relative_to(repo)),
            "report_sha256": sha256_hex(report_bytes),
            "buckets_sha256": buckets.sha256,
            "buckets_revision": buckets.revision,
            "catalogue_sha256": sha256_hex(read_capped(repo / CATALOGUE_REL, MAX_JSON_BYTES)),
            "warrant": warrant,
        },
        "verdict": report.get("verdict"),
        "claims": claims,
        "claim_tier": claim_tier,
        "conditions": conditions,
        "release_tier": release_tier,
        "floor": floor,
        "authorization": auth,
        "artifact": artifact_info,
        "decision": decision,
        "residual_risks": [
            "release_gate.py and ca-assurance-report are this repository's own unverified tools (bucket c).",
            (
                "Until agents use a separate non-admin machine account, an attacker holding the owner's "
                "GitHub token can approve a release through the API (VER31 section 9.2)."
            ),
        ],
    }


# ---------------------------------------------------------------------------
# Release notes
# ---------------------------------------------------------------------------


def render_notes(gate: dict) -> str:
    tier_title = {"tested": "Tested"}.get(gate["release_tier"], gate["release_tier"])
    out = [f"# {gate['registry_name']} {gate['version']}: release tier {tier_title}", ""]
    inp = gate["inputs"]
    out.append(f"Computed by `code/scripts/release_gate.py` (VER31) from `{inp['report']}` "
               f"(sha256 `{inp['report_sha256']}`), `{BUCKETS_REL}` revision {inp['buckets_revision']} "
               f"(sha256 `{inp['buckets_sha256']}`) and `{CATALOGUE_REL}` (sha256 `{inp['catalogue_sha256']}`)"
               + (f" at commit `{gate['commit']}`." if gate.get("commit") else "."))
    out.append("")
    out.append(f"Gate decision: **{gate['decision']}**. Claim tier: `{gate['claim_tier']}`.")
    if gate.get("lane"):
        out.append(f"This package is the `{gate['lane']}` lane; lanes carry no proofs, so every claim is `tested`.")
    out.append("")
    out.append("## Claims")
    out.append("")
    out.append("| Claim | Symbol | Generator label | Computed rung | Open (c) | Open (b) |")
    out.append("|---|---|---|---|---|---|")
    for c in gate["claims"]:
        out.append(f"| `{c['id']}` | `{c['symbol']}` | `{c['generator_label']}` | **{c['rung']}** "
                   f"| {len(c['open_c'])} | {len(c['open_b'])} |")
    failing = [h for h in gate["conditions"] if not h["holds"]]
    if gate["release_tier"] == "tested" and gate["claim_tier"] != "tested" and failing:
        out.append("")
        out.append(f"## Why the release tier is Tested, not `{gate['claim_tier']}`")
        out.append("")
        for h in failing:
            out.append(f"- {h['id']} ({h['name']}): {h['detail']}")
    out.append("")
    out.append("## Floor checks")
    out.append("")
    out.append("| Check | Result | Detail |")
    out.append("|---|---|---|")
    for f in gate["floor"]:
        out.append(f"| {f['id']} {f['name']} | {f['status']} | {f['detail']} |")
    out.append("")
    out.append("## Authorization")
    out.append("")
    for a in gate["authorization"]:
        out.append(f"- {a['id']} {a['name']}: {a['status']} ({a['detail']})")
    out.append("")
    out.append("## Assumptions")
    out.append("")
    out.append("The full bucketed list ships in the package as `ASSURANCE.md`. Per claim:")
    for c in gate["claims"]:
        if c["open_c"]:
            out.append("")
            out.append(f"- `{c['id']}` open (c): " + ", ".join(f"`{i}`" for i in c["open_c"]))
    out.append("")
    out.append("## Residual risks")
    out.append("")
    for risk in gate["residual_risks"]:
        out.append(f"- {risk}")
    if gate.get("artifact"):
        out.append("")
        out.append(f"Artifact: `{gate['artifact']['file']}` sha256 `{gate['artifact']['sha256']}`.")
    out.append("")
    return "\n".join(out)


# ---------------------------------------------------------------------------
# Command line
# ---------------------------------------------------------------------------


def _pkg_rel(repo: Path, value: str) -> str:
    if not safe_rel(value) or not re.match(r"^code/packages/(rust|typescript)/[A-Za-z0-9_-]+$", value):
        raise GateError("INPUT", f"package must be code/packages/<rust|typescript>/<name>, got {value!r}")
    if not (repo / value).is_dir():
        raise GateError("INPUT", f"{value} is not a directory")
    return value


def cmd_assurance(args: argparse.Namespace) -> int:
    repo = Path(args.repo_root).resolve()
    pkg_rel = _pkg_rel(repo, args.package)
    lang = pkg_rel.split("/")[2]
    ctx = Context(repo=repo, pkg_rel=pkg_rel, lang=lang, version="0.0.0", dry_run=True, signers_ref="HEAD")
    buckets = load_buckets(repo / BUCKETS_REL, repo / CATALOGUE_REL)
    report_path, lane = package_report(ctx)
    report_bytes = read_capped(report_path, MAX_JSON_BYTES)
    report = json.loads(report_bytes)
    claims = compute_claims(ctx, report, buckets, lane, set())
    name = read_manifest(repo / pkg_rel, lang).registry_name
    text = render_assurance(name, claims, report, sha256_hex(report_bytes), buckets, lane)
    target = repo / pkg_rel / "ASSURANCE.md"
    if args.check:
        if not target.is_file() or target.read_text("utf-8") != text:
            print(f"{target.relative_to(repo)} is stale; run release_gate.py assurance --package {pkg_rel}",
                  file=sys.stderr)
            return 1
        print(f"{target.relative_to(repo)} is up to date")
        return 0
    target.write_text(text, encoding="utf-8")
    print(f"wrote {target.relative_to(repo)}")
    return 0


def cmd_check(args: argparse.Namespace) -> int:
    repo = Path(args.repo_root).resolve()
    pkg_rel = _pkg_rel(repo, args.package)
    ctx = Context(
        repo=repo, pkg_rel=pkg_rel, lang=pkg_rel.split("/")[2], version=args.version,
        dry_run=args.dry_run, signers_ref=args.signers_ref, tag=args.tag, commit=args.commit,
        artifact=Path(args.artifact) if args.artifact else None,
        lcov=Path(args.lcov) if args.lcov else None,
        tests_ran=args.tests_ran, lint_ran=args.lint_ran, report_check_ran=args.report_check_ran,
        evidence_reproduced=args.evidence_reproduced, jail_live=args.jail_live,
        registry_has_version={"yes": True, "no": False}.get(args.registry_has_version),
        trusted_publishing=args.trusted_publishing,
    )
    if ctx.tag and not ctx.commit:
        raise GateError("INPUT", "--tag needs --commit")
    validate_inputs(ctx.tag, ctx.commit, ctx.signers_ref)
    gate = run_check(ctx)
    Path(args.out).write_text(json.dumps(gate, indent=2) + "\n", encoding="utf-8")
    print(f"decision: {gate['decision']}; release tier: {gate['release_tier']}; claim tier: {gate['claim_tier']}")
    for item in gate["floor"] + gate["authorization"]:
        if item["status"] != "pass":
            print(f"  {item['id']} {item['name']}: {item['status']}: {item['detail']}")
    ok = gate["decision"] in ("publishable", "dry-run-pass")
    return 0 if ok or args.report_only else 1


def cmd_notes(args: argparse.Namespace) -> int:
    gate = load_json(Path(args.gate))
    if gate.get("schema") != GATE_SCHEMA:
        raise GateError("INPUT", f"{args.gate} is not a {GATE_SCHEMA} document")
    Path(args.out).write_text(render_notes(gate), encoding="utf-8")
    return 0


def validate_inputs(tag: str | None, commit: str | None, ref: str) -> None:
    """Every string that reaches git is checked against a strict pattern."""
    if tag is not None and not TAG_RE.match(tag):
        raise GateError("INPUT", f"tag {tag!r} is not <language>/<name>/v<semver>")
    if commit is not None and not re.match(r"^[0-9a-f]{40}$", commit):
        raise GateError("INPUT", "commit must be a full 40-hex SHA")
    if not REF_RE.match(ref):
        raise GateError("INPUT", f"ref {ref!r} is not HEAD or a fully qualified ref")


def cmd_lane_report(args: argparse.Namespace) -> int:
    """Print the parent report a lane package ships (VER31 section 6.2)."""
    repo = Path(args.repo_root).resolve()
    pkg_rel = _pkg_rel(repo, args.package)
    lang = pkg_rel.split("/")[2]
    if lang == "rust":
        raise GateError("LANE", "a Rust package is not a lane; it has its own report")
    print(find_lane_report(repo, f"{lang}/{PurePosixPath(pkg_rel).name}"))
    return 0


def cmd_verify_tag(args: argparse.Namespace) -> int:
    repo = Path(args.repo_root).resolve()
    validate_inputs(args.tag, args.commit, args.signers_ref)
    signers_text, _ = verify_signers_chain(repo, args.signers_ref)
    verify_tag(repo, args.tag, args.commit, signers_text)
    print(f"{args.tag}: hardware-key signature verified")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--repo-root", default=str(REPO_ROOT))
    sub = parser.add_subparsers(dest="command", required=True)

    p = sub.add_parser("check", help="compute rungs, tiers and floor checks; write gate.json")
    p.add_argument("--package", required=True, help="code/packages/<lang>/<name>")
    p.add_argument("--version", required=True)
    p.add_argument("--out", required=True)
    p.add_argument("--tag")
    p.add_argument("--commit")
    p.add_argument("--signers-ref", default="refs/remotes/origin/main")
    p.add_argument("--artifact")
    p.add_argument("--lcov")
    p.add_argument("--dry-run", action="store_true")
    p.add_argument("--tests-ran", action="store_true")
    p.add_argument("--lint-ran", action="store_true")
    p.add_argument("--report-check-ran", action="store_true")
    p.add_argument("--evidence-reproduced", action="store_true")
    p.add_argument("--jail-live", action="store_true")
    p.add_argument("--trusted-publishing", action="store_true")
    p.add_argument("--registry-has-version", choices=["yes", "no"])
    p.add_argument("--report-only", action="store_true", help="exit 0 whatever the decision")
    p.set_defaults(func=cmd_check)

    p = sub.add_parser("notes", help="render release notes from gate.json")
    p.add_argument("--gate", required=True)
    p.add_argument("--out", required=True)
    p.set_defaults(func=cmd_notes)

    p = sub.add_parser("assurance", help="write or --check the package's ASSURANCE.md")
    p.add_argument("--package", required=True)
    p.add_argument("--check", action="store_true")
    p.set_defaults(func=cmd_assurance)

    p = sub.add_parser("lane-report", help="print the parent report path of a lane package")
    p.add_argument("--package", required=True)
    p.set_defaults(func=cmd_lane_report)

    p = sub.add_parser("verify-tag", help="verify a hardware-key-signed release tag")
    p.add_argument("--tag", required=True)
    p.add_argument("--commit", required=True)
    p.add_argument("--signers-ref", default="refs/remotes/origin/main")
    p.set_defaults(func=cmd_verify_tag)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        return args.func(args)
    except GateError as exc:
        print(f"release gate refused: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
