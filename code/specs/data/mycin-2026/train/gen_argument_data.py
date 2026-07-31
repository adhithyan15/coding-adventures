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


# ---------------------------------------------------------------------------
# The ATTACK seed set (AD-6) — arguments that contain a DIALECTIC, not just support. A paper
# rarely only argues FOR a thesis; a later paragraph often REBUTS an earlier conclusion. Where the
# SEED rows above are monotone support chains, each ATTACK_SEED row pairs a support paragraph with a
# rebuttal paragraph and decomposes BOTH into one `argument` block: two `context:`-tagged `infer`s
# that reach CONFLICTING conclusions on a `functional` head, plus a `context_order` recording which
# context supersedes. The engine then WITHDRAWS the defeated conclusion (ADJ73), and `verify_attack_gold`
# asserts exactly that — the winner GOVERNS, the loser is DEFEATED (defeated_by the winner). This is
# the argument surface's attack edges (AR-3 in-block rebut) made into checkable training gold.
#
# Each attack edge records both the precedence direction (winner/loser context) and the sentence
# that ESTABLISHES it (`quote`) — so a decomposer that reverses the direction, or invents a
# precedence the paragraph never states, is a scoreable veto (argument_decompose_score.py).
# ---------------------------------------------------------------------------
ATTACK_SEED: list[dict] = [
    {
        "id": "arg-planet-reanalysis",
        "name": "planet_signal",
        "domain": "astronomy",
        "source": (
            "The periodic radial-velocity wobble of the star was first reported as "
            "the signature of an orbiting planet. A later reanalysis found the same "
            "period matched the star's rotation, so the signal was attributed to "
            "stellar activity. Because the reanalysis controlled for activity, its "
            "conclusion supersedes the initial report."
        ),
        "doc": "exoplanet reanalysis",
        "trust": "authoritative",
        "premises": [
            {"name": "obs", "kind": "extracted", "term": "observed(signal, wobble)",
             "quote": "periodic radial-velocity wobble of the star"},
            {"name": "rot", "kind": "extracted", "term": "matches(period, rotation)",
             "quote": "the same period matched the star's rotation"},
        ],
        "inferences": [
            # The SUPPORT step (grounded in the initial report) and the REBUTTAL step (grounded in
            # the reanalysis) reach conflicting mechanisms for the SAME signal — an in-block rebut.
            {"name": "support", "connective": "because",
             "conclusion": "attributed_to(signal, planet)", "from": ["obs"],
             "quote": "first reported as the signature of an orbiting planet",
             "context": "initial_report"},
            {"name": "reanalysis", "connective": "because",
             "conclusion": "attributed_to(signal, stellar_activity)", "from": ["rot"],
             "quote": "the signal was attributed to stellar activity",
             "context": "reanalysis"},
        ],
        # The functional head: a signal has ONE attributed mechanism, so `planet` and
        # `stellar_activity` conflict; the context_order says the reanalysis wins.
        "functional": "attributed_to(subject, mechanism)",
        "context_order": [("reanalysis", "initial_report")],
        "attacks": [
            {"kind": "rebut", "defeater": "reanalysis", "defeated": "support",
             "winner_context": "reanalysis", "loser_context": "initial_report",
             "winner_conclusion": "attributed_to(signal, stellar_activity)",
             "loser_conclusion": "attributed_to(signal, planet)",
             "quote": "its conclusion supersedes the initial report"},
        ],
        "thesis": "attributed_to(signal, $Mechanism)",
        # After the attack resolves, ONLY the governing mechanism should stand; the CLI still lists
        # both answers in `recall`, so the attack check reads the `governing` section, not `recall`.
        "expect_governing": "attributed_to(signal, stellar_activity)",
        "expect_defeated": "attributed_to(signal, planet)",
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
        # AR-3 in-block ATTACK sugar (AD-6). An `infer` may carry `unless <defeater…>` (an
        # undercut → a `not <defeater>` body literal) BEFORE its annotations, and `context: <ctx>`
        # AFTER them — exactly the surface order the grammar accepts
        # (`from … [unless …] {annotation} [context: IDENT]`). Support seeds set neither, so their
        # emitted line is byte-identical to before this change.
        unless = f' unless {", ".join(inf["unless"])}' if inf.get("unless") else ""
        context = f' context: {inf["context"]}' if inf.get("context") else ""
        lines.append(
            f'    infer {inf["name"]} : {inf["connective"]} conclude {inf["conclusion"]} '
            f'from {refs}{unless} quote "{inf["quote"]}" at {off} snapshot "{hexhash}" '
            f'source "{doc}" trust {trust}{context}'
        )
    lines.append("}")
    # A REBUT attack needs the two conclusions to share a `functional` head (so they conflict on
    # the functional argument) plus a `context_order { winner > loser }` per attack edge, both
    # top-level after the block. The engine then WITHDRAWS the lower-ranked conclusion. Support
    # seeds carry neither key, so nothing extra is emitted for them.
    if spec.get("functional"):
        lines.append(f'functional {spec["functional"]}')
    for hi, lo in spec.get("context_order", []):
        lines.append(f'context_order {{ {hi} > {lo} }}')
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


def to_attack_training_row(spec: dict, source_text: str) -> dict:
    """The training row for an ATTACK argument (AD-6). Extends the §3.2 support schema with two
    fields the decomposer must also learn: each inference carries its `context` tag, the gold-object
    carries the `functional` head, and an `attacks` list records each attack edge — its kind, the
    defeater/defeated inference names, the winner/loser CONTEXTS and CONCLUSIONS (the precedence
    DIRECTION), and the `span` (the sentence establishing precedence). That direction is what a
    decomposer can get backwards; recording it makes a reversed attack a scoreable veto."""
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
                 "from": i["from"], "span": i["quote"], "type": "stated", "context": i.get("context")}
                for i in spec["inferences"]
            ],
            "thesis": spec["thesis"],
            "functional": spec.get("functional"),
            "attacks": [
                {"kind": a["kind"], "defeater": a["defeater"], "defeated": a["defeated"],
                 "winner_context": a["winner_context"], "loser_context": a["loser_context"],
                 "winner_conclusion": a["winner_conclusion"], "loser_conclusion": a["loser_conclusion"],
                 "span": a["quote"]}
                for a in spec.get("attacks", [])
            ],
            "discard": [{"span": d["quote"], "reason": d["reason"]} for d in spec.get("discard", [])],
        },
    }


def governing_answers_for(adj_text: str) -> list[dict]:
    """Run `adj-lang-cli` on `adj_text` and return the first query's `governing` answers — each a
    dict with `term`, `status` (`governing`/`defeated`/`conflict`), and (when defeated) `defeated_by`.
    This is the engine's ADJ73 verdict, the ground truth an attack decomposition is scored against.
    Raises `BinariesMissing` if the CLI is not built."""
    if not CLI.exists():
        raise BinariesMissing(f"build adj-lang-cli first: {RUST_TARGET}")
    with tempfile.TemporaryDirectory() as td:
        prog = Path(td) / "arg.adj"
        prog.write_text(adj_text)
        run = subprocess.run([str(CLI), str(prog)], capture_output=True, text=True, timeout=60)
    out = run.stdout.strip()
    d = json.loads(out) if out.startswith("{") else {}
    gov = d.get("governing", [])
    return gov[0]["answers"] if gov else []


def verify_attack_gold(spec: dict) -> dict:
    """The attack counterpart of `verify_gold`'s gate. Builds the attack seed's gold `.adj` and
    asserts the engine RESOLVES the dialectic: it still byte-anchors every citation (the support
    gate), AND — reading the `governing` section — the `expect_governing` conclusion GOVERNS while
    the `expect_defeated` conclusion is WITHDRAWN (`defeated`, `defeated_by` the winner). A wrong
    context_order would flip these, so this check is what proves the attack edge actually bites."""
    sb = source_bytes_for(spec)
    adj_text, _ = build_argument_adj(spec, sb)
    res = verify_gold(adj_text, sb)
    by_term = {a["term"]: a for a in res.get("governing_answers", [])}
    win = by_term.get(spec["expect_governing"], {})
    lose = by_term.get(spec["expect_defeated"], {})
    return {
        "derive_ok": res["derive_ok"],
        "verify_ok": res["verify_ok"],
        "verified": res["verified"],
        "quotes_verified": res["quotes_verified"],
        "winner_governs": win.get("status") == "governing",
        "loser_defeated": lose.get("status") == "defeated",
        "defeated_by_winner": lose.get("defeated_by") == spec["expect_governing"],
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
    # The CLI's `governing` section (ADJ73 defeasibility) reports, per query, which answers are
    # `governing` and which are `defeated` (with `defeated_by`). An attack seed reads this to prove
    # the engine WITHDRAWS the loser; support seeds simply ignore it.
    d = _json(derive.stdout)
    gov = d.get("governing", [])
    return {
        "derive_ok": derive.returncode == 0,
        "derive_stdout": derive.stdout,
        "verify_ok": verify.returncode == 0,
        "verify": v,
        # adj-verify nests the per-run counters under `totals`.
        "quotes_verified": totals.get("quotes_verified"),
        "verified": v.get("verified"),
        "governing_answers": gov[0]["answers"] if gov else [],
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
    # ATTACK seeds go to their OWN JSONL so the support `argument_seed.jsonl` schema stays exactly
    # as AD-2 shipped it (no `attacks`/`functional` fields leaking into support consumers).
    attack_rows = []
    for spec in ATTACK_SEED:
        sb = source_bytes_for(spec)
        adj_text, _ = build_argument_adj(spec, sb)
        (outdir / f'{spec["id"]}.source.txt').write_bytes(sb)
        (outdir / f'{spec["id"]}.adj').write_text(adj_text)
        attack_rows.append(to_attack_training_row(spec, sb.decode("utf-8")))
    (outdir / "argument_attack_seed.jsonl").write_text(
        "".join(json.dumps(r) + "\n" for r in attack_rows)
    )
    print(f"gen_argument_data: emitted {len(SEED)} support + {len(ATTACK_SEED)} attack "
          f"argument(s) -> {outdir}")
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


def attack_self_check() -> int:
    """Build each ATTACK seed's gold `.adj` and assert the engine resolves the dialectic: every
    citation still byte-anchors AND the winner governs while the loser is withdrawn. Non-zero on any
    failure. Skips (returns 2) when the binaries are not built, like `self_check`."""
    failures = 0
    for spec in ATTACK_SEED:
        try:
            res = verify_attack_gold(spec)
        except BinariesMissing as e:
            print(f"  ⚠ {spec['id']}: {e}", file=sys.stderr)
            return 2
        want = total_citations(spec)
        ok = (
            res["derive_ok"]
            and res["verify_ok"]
            and res["verified"] is True
            and res["quotes_verified"] == want
            and res["winner_governs"]
            and res["loser_defeated"]
            and res["defeated_by_winner"]
        )
        status = "ok" if ok else "FAIL"
        print(f"  [{status}] {spec['id']}: byte-anchored={res['quotes_verified']}/{want} "
              f"winner_governs={res['winner_governs']} loser_defeated={res['loser_defeated']} "
              f"defeated_by_winner={res['defeated_by_winner']}")
        failures += 0 if ok else 1
    if failures:
        print(f"\nattack-self-check: {failures} attack seed(s) FAILED", file=sys.stderr)
        return 1
    print(f"\nattack-self-check: all {len(ATTACK_SEED)} attack seed(s) resolve the dialectic ✓")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--emit", type=Path, metavar="DIR",
                    help="write seed .adj + source + JSONL into DIR")
    ap.add_argument("--self-check", action="store_true",
                    help="run the 3-part correctness gate on every seed (needs built binaries)")
    ap.add_argument("--attack-self-check", action="store_true",
                    help="run the attack-resolution gate on every ATTACK seed (needs built binaries)")
    args = ap.parse_args()
    if args.emit:
        return emit(args.emit)
    if args.self_check:
        return self_check()
    if args.attack_self_check:
        return attack_self_check()
    ap.print_help()
    return 0


if __name__ == "__main__":
    sys.exit(main())
