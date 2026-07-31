#!/usr/bin/env python3
"""gen_argument_data.py — the framework authors its own training data for the
prose→ARGUMENT decomposer (ADJ-ARGUMENT-DECOMPOSER.md, AD-2).

Where gen_data.py generates the closed-vocab FINDINGS shape, this generates the
OPEN-VOCAB `argument` shape: a paragraph of prose → an `argument { premise… infer… }`
program whose premises cite verbatim byte slices of the paragraph, whose inference
steps cite their connective bytes, and whose `? thesis` query the engine DERIVES.

It runs the generator BACKWARD, exactly like gen_data.py: we author the gold argument
skeleton (premises + inference steps + spans) first, so the label is exact by
construction; a paragraph states each premise and asserts each connective; and the gold
`.adj` is DERIVED from the paragraph DETERMINISTICALLY — every citation's byte offset is
computed with `bytes.find`, and the snapshot is the paragraph's SHA-256. No model is in
this path: a span that is not a verbatim slice of the paragraph raises `SpanNotFound`
(the fabrication guard), never a fabricated citation.

The gold `.adj` is then SELF-CHECKED against the three-part correctness gate the spec
defines (ADJ-ARGUMENT-DECOMPOSER.md §2): it must (1) COMPILE, (2) let `adj-lang-cli`
DERIVE the thesis, and (3) let `adj-verify --snapshots` BYTE-ANCHOR every citation. These
are the same checks the ADR-5 worked example passes; the first seed row IS that example.

The teacher-model backward generation (scaling the seed to many paragraphs, as gen_data.py
does via Ollama) is a follow-up (AD-2b) — the SHIPPED path here is model-free: a small,
hand-authored SEED set + the deterministic builder + the CLI self-check.

Usage:
  python3 gen_argument_data.py --emit <dir>     # write seed .adj + source + JSONL
  python3 gen_argument_data.py --self-check      # run the 3-part gate on every seed
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
# train -> mycin-2026 -> data -> specs -> code
CODE = HERE.parents[3]
RUST_TARGET = CODE / "packages" / "rust" / "target" / "debug"
CLI = RUST_TARGET / "adj-lang-cli"
VERIFY = RUST_TARGET / "adj-verify"
# The ADR-5 worked example — the first seed row reuses its exact source bytes.
AXLE_SOURCE = "specs/data/adj-argument-ir/axle-fatigue.source.txt"


class SpanNotFound(ValueError):
    """A cited quote is not a verbatim slice of the paragraph — the fabrication
    guard. The deterministic builder refuses to emit a citation it cannot anchor."""


class BinariesMissing(RuntimeError):
    """adj-lang-cli / adj-verify are not built, so the CLI self-check cannot run.
    Build them with `cargo build -p adj-lang-cli` (mirrors board_eval's graceful skip)."""


# ---------------------------------------------------------------------------
# The SEED set — hand-authored gold argument skeletons across DIFFERENT domains,
# proving the open-vocab surface generalizes (materials science, epidemiology,
# astronomy). Each is authored so its `.adj` passes the three-part gate; the
# builder computes every byte offset and the snapshot hash, never the author.
#
# A seed's `source` is either an inline paragraph or a committed `source_file`
# (repo-relative, under CODE). `expect` is the derived value the thesis query must
# bind — the substring the derivation's output must contain.
# ---------------------------------------------------------------------------
SEED: list[dict] = [
    {
        "id": "arg-axle-fatigue",
        "name": "axle_fatigue",
        "domain": "materials-science",
        "source_file": AXLE_SOURCE,
        "doc": "axle failure report",
        "trust": "authoritative",
        "premises": [
            {"name": "p1", "kind": "extracted", "term": "stress_amplitude(axle, 420)",
             "quote": "a stress amplitude of 420 MPa"},
            {"name": "p2", "kind": "extracted", "term": "endurance_limit(axle, 380)",
             "quote": "its endurance limit was measured at 380 MPa"},
            {"name": "p3", "kind": "extracted", "term": "shows(surface, beach_marks)",
             "quote": "exhibited beach marks"},
            {"name": "p4", "kind": "extracted", "term": "diagnostic_of(beach_marks, fatigue)",
             "quote": "diagnostic of progressive fatigue crack growth"},
        ],
        "inferences": [
            {"name": "s1", "connective": "because", "conclusion": "exceeds_endurance(axle)",
             "from": ["p1", "p2"], "quote": "exceeds its endurance limit"},
            {"name": "s2", "connective": "therefore", "conclusion": "failed_by(axle, fatigue)",
             "from": ["s1", "p3", "p4"], "quote": "The axle therefore failed by fatigue"},
        ],
        "thesis": "failed_by(axle, $Mechanism)",
        "expect": "fatigue",
        "discard": [],
    },
    {
        "id": "arg-pump-outbreak",
        "name": "pump_outbreak",
        "domain": "epidemiology",
        "source": (
            "Investigators found that new illness clustered tightly around a single "
            "neighborhood water pump. A nearby factory had recently changed its work "
            "schedule. When that pump was closed, the number of new cases fell sharply. "
            "The pump was therefore the source of the outbreak."
        ),
        "doc": "outbreak field report",
        "trust": "authoritative",
        "premises": [
            {"name": "p1", "kind": "extracted", "term": "clustered_around(illness, pump)",
             "quote": "new illness clustered tightly around a single neighborhood water pump"},
            {"name": "p2", "kind": "extracted", "term": "fell_after_closure(pump)",
             "quote": "When that pump was closed, the number of new cases fell sharply"},
        ],
        "inferences": [
            {"name": "s1", "connective": "therefore", "conclusion": "source_of(pump, outbreak)",
             "from": ["p1", "p2"], "quote": "The pump was therefore the source of the outbreak"},
        ],
        "thesis": "source_of(pump, $What)",
        "expect": "outbreak",
        # A near-miss: a coincidental nearby event that reads like context but is NOT a
        # premise of the argument. It appears in the paragraph and must be SET ASIDE, never
        # cited — the decomposer's discard discipline (spec §3.1).
        "discard": [
            {"quote": "A nearby factory had recently changed its work schedule",
             "reason": "coincidental nearby event, not a premise of the outbreak argument (near-miss: irrelevant context)"},
        ],
    },
    {
        "id": "arg-galaxy-redshift",
        "name": "galaxy_redshift",
        "domain": "astronomy",
        "source": (
            "The observed spectral lines of the distant galaxy were shifted toward longer "
            "wavelengths. Such a redshift indicates that the source is moving away from the "
            "observer. The galaxy is therefore receding from us."
        ),
        "doc": "spectroscopy note",
        "trust": "authoritative",
        "premises": [
            {"name": "p1", "kind": "extracted", "term": "shifted(lines, longer_wavelengths)",
             "quote": "shifted toward longer wavelengths"},
            {"name": "p2", "kind": "extracted", "term": "indicates(redshift, recession)",
             "quote": "redshift indicates that the source is moving away from the observer"},
        ],
        "inferences": [
            {"name": "s1", "connective": "therefore", "conclusion": "receding(galaxy)",
             "from": ["p1", "p2"], "quote": "The galaxy is therefore receding from us"},
        ],
        "thesis": "receding($Body)",
        "expect": "galaxy",
        "discard": [],
    },
]


def source_bytes_for(spec: dict) -> bytes:
    """The paragraph bytes a seed pins — either its committed `source_file` (repo-relative
    under CODE, so the axle row is byte-identical to ADR-5) or its inline `source`."""
    if "source_file" in spec:
        return (CODE / spec["source_file"]).read_bytes()
    return spec["source"].encode("utf-8")


def _offset(source_bytes: bytes, quote: str) -> int:
    """The byte offset at which `quote` begins in `source_bytes`, or raise
    `SpanNotFound`. This IS the byte-provenance: a citation's `at <offset>` must point at
    a verbatim slice, so a quote the author mistyped (or invented) fails loudly here rather
    than shipping a citation `adj-verify` would later reject."""
    idx = source_bytes.find(quote.encode("utf-8"))
    if idx < 0:
        raise SpanNotFound(quote)
    return idx


def build_argument_adj(spec: dict, source_bytes: bytes) -> tuple[str, str]:
    """Deterministically emit the gold `argument` .adj for `spec` against `source_bytes`.

    Returns `(adj_text, snapshot_hex)`. Every premise/inference citation's byte offset is
    computed from the paragraph (never authored), and the snapshot is the paragraph's
    SHA-256 — so the emitted program is byte-anchored to the exact source it was built
    against. Raises `SpanNotFound` for any quote that is not a verbatim slice."""
    hexhash = hashlib.sha256(source_bytes).hexdigest()
    doc, trust = spec["doc"], spec["trust"]
    lines = [f'argument {spec["name"]} {{']
    for p in spec["premises"]:
        off = _offset(source_bytes, p["quote"])
        lines.append(
            f'    premise {p["name"]} : {p["kind"]} {p["term"]} '
            f'quote "{p["quote"]}" at {off} snapshot "{hexhash}" '
            f'source "{doc}" trust {trust}'
        )
    for inf in spec["inferences"]:
        off = _offset(source_bytes, inf["quote"])
        refs = ", ".join(inf["from"])
        lines.append(
            f'    infer {inf["name"]} : {inf["connective"]} conclude {inf["conclusion"]} '
            f'from {refs} quote "{inf["quote"]}" at {off} snapshot "{hexhash}" '
            f'source "{doc}" trust {trust}'
        )
    lines.append("}")
    lines.append(f'? {spec["thesis"]}')
    return "\n".join(lines) + "\n", hexhash


def to_training_row(spec: dict, source_text: str) -> dict:
    """The training-example JSONL row (ADJ-ARGUMENT-DECOMPOSER.md §3.2): the paragraph
    (`note`) and the gold argument the decomposer must learn to emit from it — premises,
    inferences, thesis, and the near-miss `discard` spans it must set aside. `span` is the
    VERBATIM substring the citation quotes; offsets/hash are recomputed from `note` at
    build time so they can never drift from the text."""
    return {
        "id": spec["id"],
        "shape": "argument",
        "domain": spec["domain"],
        "note": source_text,
        "gold": {
            "premises": [
                {"name": p["name"], "kind": p["kind"], "term": p["term"],
                 "span": p["quote"], "type": "stated"}
                for p in spec["premises"]
            ],
            "inferences": [
                {"name": i["name"], "connective": i["connective"], "conclusion": i["conclusion"],
                 "from": i["from"], "span": i["quote"], "type": "stated"}
                for i in spec["inferences"]
            ],
            "thesis": spec["thesis"],
            "discard": [{"span": d["quote"], "reason": d["reason"]} for d in spec.get("discard", [])],
        },
    }


def verify_gold(adj_text: str, source_bytes: bytes) -> dict:
    """Run the three-part correctness gate on a gold `.adj` via the built binaries:
    (1) `adj-lang-cli` COMPILES + DERIVES (returns the recall/decision JSON), and
    (2) `adj-verify --snapshots` BYTE-ANCHORS every citation against the pinned paragraph.

    The paragraph is placed as a content-addressed snapshot (its SHA-256 filename), exactly
    the hex the `.adj` pins, so `verify_quote` can resolve and re-check each citation.
    Raises `BinariesMissing` if the binaries are not built."""
    if not CLI.exists() or not VERIFY.exists():
        raise BinariesMissing(f"build adj-lang-cli / adj-verify first: {RUST_TARGET}")
    with tempfile.TemporaryDirectory() as td:
        tdp = Path(td)
        prog = tdp / "arg.adj"
        prog.write_text(adj_text)
        derive = subprocess.run(
            [str(CLI), str(prog)], capture_output=True, text=True, timeout=60
        )
        snaps = tdp / "snaps"
        snaps.mkdir()
        hexhash = hashlib.sha256(source_bytes).hexdigest()
        (snaps / hexhash).write_bytes(source_bytes)
        verify = subprocess.run(
            [str(VERIFY), "--snapshots", str(snaps), str(prog)],
            capture_output=True, text=True, timeout=60,
        )

    def _json(out: str) -> dict:
        out = out.strip()
        return json.loads(out) if out.startswith("{") else {}

    v = _json(verify.stdout)
    totals = v.get("totals", {})
    return {
        "derive_ok": derive.returncode == 0,
        "derive_stdout": derive.stdout,
        "verify_ok": verify.returncode == 0,
        "verify": v,
        # adj-verify nests the per-run counters under `totals`.
        "quotes_verified": totals.get("quotes_verified"),
        "verified": v.get("verified"),
    }


def total_citations(spec: dict) -> int:
    """The number of cited elements — every premise plus every inference warrant — which
    a passing `adj-verify` run must report as `quotes_verified`."""
    return len(spec["premises"]) + len(spec["inferences"])


def emit(outdir: Path) -> int:
    """Write, for each seed, its `<id>.source.txt`, its gold `<id>.adj`, and a combined
    `argument_seed.jsonl` of the training rows. The axle row reuses the committed ADR-5
    source, so its snapshot equals that fixture's."""
    outdir.mkdir(parents=True, exist_ok=True)
    rows = []
    for spec in SEED:
        sb = source_bytes_for(spec)
        adj_text, _ = build_argument_adj(spec, sb)
        (outdir / f'{spec["id"]}.source.txt').write_bytes(sb)
        (outdir / f'{spec["id"]}.adj').write_text(adj_text)
        rows.append(to_training_row(spec, sb.decode("utf-8")))
    (outdir / "argument_seed.jsonl").write_text(
        "".join(json.dumps(r) + "\n" for r in rows)
    )
    print(f"gen_argument_data: emitted {len(SEED)} seed argument(s) -> {outdir}")
    return 0


def self_check() -> int:
    """Build each seed's gold `.adj` and run the three-part gate. Non-zero on any failure."""
    failures = 0
    for spec in SEED:
        sb = source_bytes_for(spec)
        adj_text, _ = build_argument_adj(spec, sb)
        try:
            res = verify_gold(adj_text, sb)
        except BinariesMissing as e:
            print(f"  ⚠ {spec['id']}: {e}", file=sys.stderr)
            return 2
        want = total_citations(spec)
        ok = (
            res["derive_ok"]
            and spec["expect"] in res["derive_stdout"]
            and res["verify_ok"]
            and res["verified"] is True
            and res["quotes_verified"] == want
        )
        status = "ok" if ok else "FAIL"
        print(f"  [{status}] {spec['id']}: derives={res['derive_ok']} "
              f"expect({spec['expect']})={spec['expect'] in res['derive_stdout']} "
              f"verified={res['verified']} quotes={res['quotes_verified']}/{want}")
        failures += 0 if ok else 1
    if failures:
        print(f"\nself-check: {failures} seed(s) FAILED the gate", file=sys.stderr)
        return 1
    print(f"\nself-check: all {len(SEED)} seed(s) compile + derive + byte-anchor ✓")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--emit", type=Path, metavar="DIR",
                    help="write seed .adj + source + JSONL into DIR")
    ap.add_argument("--self-check", action="store_true",
                    help="run the 3-part correctness gate on every seed (needs built binaries)")
    args = ap.parse_args()
    if args.emit:
        return emit(args.emit)
    if args.self_check:
        return self_check()
    ap.print_help()
    return 0


if __name__ == "__main__":
    sys.exit(main())
