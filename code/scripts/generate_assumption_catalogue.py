#!/usr/bin/env python3
"""Generate ``code/assumptions/catalogue-v1.json`` from VER13 section 3.

Division of labour
==================

Spec section 3 is the *source of truth for what an assumption means*: its id,
statement, dependants ("Used by"), discharge plan, tools, difficulty and attack
tier.  Those columns are copied (parsed, never retyped) by the same parser the
checker uses (``assumption_catalogue.parse_spec_inventory``), so the two cannot
disagree about how a table row is read.

The spec tables do *not* say everything the machine-readable schema (section
5.2) needs.  The remaining fields are curated here, in one reviewable place:

* ``kind`` and ``status`` (the nine attestations are ``attested``; three
  entries the spec calls "evidenced today" are ``evidenced``; everything else
  is ``assumed``, the conservative default that makes the first report longer);
* ``depends_on`` / ``refines`` edges, taken from the spec's prose;
* ``applies_to`` (which labels / claim kinds / packages an entry constrains);
* ``discharge.evidence_kinds``, ``discharge.rung`` and ``discharge.backlog``.

``applies_to`` is deliberately provisional.  It only starts to matter when
``ca-assurance-report`` implements AC-4 (part 2 of AS-001a), which will tune it
against the real report.

Run ``python3 code/scripts/generate_assumption_catalogue.py`` to rewrite the
JSON, or ``--check`` to fail when the committed file is stale.  The unit tests
run ``--check`` semantics so the JSON can never drift from this script.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import assumption_catalogue as ac

# ---------------------------------------------------------------------------
# Curated overlay
# ---------------------------------------------------------------------------

ALL_LABELS = list(ac.PROOF_LABELS)

# A claim "proved-to-L" has discharged every layer above L, so an entry stops
# applying to a label once the label's rung has retired it (spec section 4.6).
LAYER_LABELS = {
    "spec": ALL_LABELS,
    "src": ALL_LABELS,
    "wir": ALL_LABELS,
    "cc": ["bounded(k)", "proved", "proved-per-run"],
    "mc": ["bounded(k)", "proved", "proved-per-run", "proved-to-isa"],
    "uarch": ["bounded(k)", "proved", "proved-per-run", "proved-to-isa"],
    "gate": ["bounded(k)", "proved", "proved-per-run", "proved-to-isa"],
    "phys": ALL_LABELS,
    "os": ALL_LABELS,
    "gov": ALL_LABELS,
}
LABEL_OVERRIDES = {
    # The spec's own section 5.2 example.
    "memory.unaliased": ["proved", "bounded(k)"],
    # Floors that survive even at the gate-level label.
    "gate.primitives.correct": ALL_LABELS,
    "gate.sequential.ideal-dff": ALL_LABELS,
}

CT = ["constant-time"]
CT_FN_BOUND = ["constant-time", "functional", "bound"]
CLAIMS_BY_FAMILY = {
    "spec": CT,
    "src": ["capability", "weakness-row"],
    "att": ["weakness-row"],
    "wir": CT_FN_BOUND,
    "tcb": CT_FN_BOUND,
    "cc": CT,
    "mc": CT,
    "uarch": CT,
    "gate": CT,
    "phys": CT,
    "os": ["capability"],
    "gov": ["report"],
}
CLAIM_OVERRIDES = {
    "memory.unaliased": ["constant-time", "functional"],
    "spec.platform.declared-none": CT_FN_BOUND,
    "src.rust.int-semantics": ["functional", "bound"],
    "src.rust.safe-bounds-checks": ["weakness-row"],
    "src.rust.unsafe-region.sound": ["weakness-row"],
    "src.rust.subset-conformance": CT_FN_BOUND,
    "src.panic.paths-public": CT,
    "os.allocator.size-only": CT,
    "os.std.runtime-benign": CT,
    "os.counters.not-live": CT,
}
# Entries every report depends on, whatever the package.
WILDCARD_PACKAGE = {
    "gov.approval.blocked",
    "gov.catalogue.integrity",
    "tcb.report.checker-correct",
}

KIND_BY_ID = {
    "spec.secret-labels.complete": "model",
    "spec.obs-model.complete": "model",
    "spec.obs-model.public-length": "declared",
    "spec.platform.declared-none": "model",
    "spec.corpus.ct01-faithful": "model",
    "src.analyzer.correct": "trust",
    "src.effects.witness-token": "tool",
    "src.effects.certificate-binding": "tool",
    "src.registry.excluded-rows": "tool",
    "wir.lowering.faithful": "trust",
    "wir.interp.differential-only": "trust",
    "wir.inline.sound": "tool",
    "wir.proof.trivial-miter": "tool",
    "wir.proof.obligation-binding": "tool",
    "compiler.preserves_timing": "tool",
    "cc.llvm-ir.no-secret-branch": "tool",
    "cc.llvm-ir.addresses-public": "tool",
    "cc.volatile-honoured": "tool",
    "cc.version-pinned": "environment",
    "cc.caller-context": "declared",
    "mc.backend.no-secret-branch": "tool",
    "mc.backend.addresses-public": "tool",
    "gate.sequential.ideal-dff": "declared",
    "phys.silicon.realization": "physical",
    "phys.side-channels.out-of-scope": "declared",
    "os.kernel.trusted": "trust",
    "os.runtime-gate.exercised-paths-only": "declared",
}
KIND_BY_FAMILY = {
    "spec": "model",
    "src": "model",
    "wir": "model",
    "att": "attestation",
    "tcb": "trust",
    "cc": "tool",
    "mc": "model",
    "uarch": "environment",
    "gate": "model",
    "phys": "physical",
    "os": "environment",
    "gov": "governance",
}
# `src.analyzer.G-n` are caveats of a tool, not of the Rust language.
KIND_PREFIX = {"src.analyzer.G-": "tool"}
EVIDENCED = {
    "cc.llvm-ir.no-secret-branch",
    "cc.volatile-honoured",
    "os.syscall.baseline",
}
# Not dischargeable by software: declared bottom of the ladder.
FLOOR = {
    "spec.obs-model.public-length",
    "cc.caller-context",
    "gate.sequential.ideal-dff",
    "phys.silicon.realization",
    "phys.side-channels.out-of-scope",
    "os.runtime-gate.exercised-paths-only",
    "os.kernel.trusted",
}

UMBRELLA = "compiler.preserves_timing"
UMBRELLA_CHILDREN = [
    "cc.llvm-ir.no-secret-branch",
    "cc.llvm-ir.addresses-public",
    "cc.volatile-honoured",
    "cc.version-pinned",
    "cc.caller-context",
    "mc.backend.no-secret-branch",
    "mc.backend.addresses-public",
]

G_CAVEATS = [f"src.analyzer.G-{n}" for n in range(1, 10)]
DEPENDS_ON = {
    "memory.unaliased": ["wir.memory.provenance-model"],
    "uarch.fetch.pc-trace": ["spec.obs-model.complete"],
    "uarch.cache.address-trace-suffices": ["spec.obs-model.complete"],
    "src.panic.paths-public": ["wir.lowering.faithful"],
    "src.effects.certificate-binding": ["src.analyzer.correct"],
    "src.registry.excluded-rows": G_CAVEATS,
    "att.warrant-ir.cost-semantics-linear": ["wir.cost.step-model"],
    "wir.lowering.faithful": ["src.rust.subset-conformance", "src.rust.int-semantics"],
    "wir.lowering.volatile-by-name": ["src.rust.unsafe-region.sound"],
    "wir.inline.sound": ["wir.lowering.faithful"],
    "wir.interp.differential-only": ["wir.lowering.faithful"],
    "wir.proof.trivial-miter": [
        "tcb.drat.checker-correct",
        "tcb.bitblast.encoding-correct",
    ],
    "wir.proof.obligation-binding": ["tcb.report.checker-correct"],
    "cc.llvm-ir.no-secret-branch": ["cc.version-pinned"],
    "cc.llvm-ir.addresses-public": ["cc.version-pinned"],
    "cc.volatile-honoured": ["cc.version-pinned"],
    "mc.backend.no-secret-branch": ["cc.version-pinned"],
    "mc.backend.addresses-public": ["cc.version-pinned"],
    "mc.isa.timing-data-independent": ["gate.clocking.one-instruction-per-cycle"],
    "gate.model.is-netlist": ["gate.primitives.correct"],
    "gate.clocking.one-instruction-per-cycle": ["gate.model.is-netlist"],
    "gate.memory.abstraction": ["gate.model.is-netlist"],
    "gate.isa-equivalence": ["gate.model.is-netlist", "mc.isa.model-faithful"],
}

# Backlog row that owns the discharge of an entry (must exist in VER-BACKLOG.md).
BACKLOG_BY_ID = {
    "wir.proof.obligation-binding": "AS-001a",
    "tcb.drat.checker-correct": "LH-001",
    "tcb.bitblast.encoding-correct": "LH-006",
    "tcb.symexec.engine-correct": "LH-007",
    "tcb.report.checker-correct": "WD-004",
    "tcb.evidence.manifest-honesty": "WD-002",
    "tcb.rust-toolchain.trusted": "AS-005",
    "spec.corpus.ct01-faithful": "LH-004",
    "spec.secret-labels.complete": "LH-004",
    "src.analyzer.correct": "LH-002",
    "gov.approval.blocked": "AS-005",
    "wir.cost.step-model": "AS-004",
    UMBRELLA: "AS-003a",
}
BACKLOG_BY_TIER = {
    "O1": "AS-001a",
    "O2": "AS-002",
    "O3": "AS-003a",
    "O3/O4": "AS-003a",
    "O4": "AS-004",
    "O5": "AS-004",
    "O6": "AS-005",
    "P": "LH-002b",
    "gov": "AS-001a",
}
BACKLOG_BY_PREFIX = {"os.": "LH-002", "src.analyzer.": "LH-002b"}
TIER_RUNG = {
    "O1": "R0+",
    "O2": "R0+",
    "O3": "R1",
    "O3/O4": "R1",
    "O4": "R2",
    "O5": "R3",
    "O6": "R4",
    "P": "R0+",
    "gov": "R0+",
    "via-ladder": "R2",
    "": "R2",
}
# Trust in a verification tool is retired by checking the tool, not by climbing
# the ladder, so those entries are dischargeable at R0+ (except the toolchain).
RUNG_OVERRIDES = {"tcb.rust-toolchain.trusted": "R4", "memory.unaliased": "R0+"}

EVIDENCE_KINDS = {
    "spec.secret-labels.complete": ["corpus-label-diff", "negative-control"],
    "spec.obs-model.complete": ["wir-trace-reproof"],
    "spec.platform.declared-none": ["warrant-assume-sync"],
    "spec.corpus.ct01-faithful": ["spec-sync-test", "sabotage-corpus"],
    "src.rust.subset-conformance": ["conformance-suite"],
    "src.rust.int-semantics": ["differential-run"],
    "src.rust.safe-bounds-checks": ["wir-memory-safety-proof"],
    "src.rust.unsafe-region.sound": ["miri-run"],
    "src.panic.paths-public": ["wir-panic-branch-proof"],
    "src.analyzer.G-1": ["dependency-closure-check"],
    "src.analyzer.G-2": ["runtime-gate-observation"],
    "src.analyzer.G-3": ["runtime-path-allowlist"],
    "src.analyzer.G-4": ["manifest-binding-check"],
    "src.analyzer.G-5": ["build-time-authority-analysis"],
    "src.analyzer.G-6": ["expanded-code-analysis"],
    "src.analyzer.G-7": ["call-graph-proof"],
    "src.analyzer.G-8": ["type-resolved-analysis"],
    "src.analyzer.G-9": ["build-config-pin"],
    "src.analyzer.correct": ["checker-corpus", "mechanised-checker"],
    "src.effects.witness-token": ["type-level-enforcement"],
    "src.effects.certificate-binding": [
        "digest-coverage-check",
        "ci-regeneration-gate",
    ],
    "src.registry.excluded-rows": ["catalogue-citation-check"],
    "att.ast-audit.no-compression-routines": ["banned-construct-scan"],
    "att.ast-audit.no-deserialization": ["banned-construct-scan"],
    "att.ast-audit.no-legacy-ciphers": ["banned-construct-scan"],
    "att.ast-audit.no-protocol-sinks": ["banned-construct-scan"],
    "att.analyzer.name-allowlist-regex-free": ["matcher-property-test"],
    "att.ast-call-graph.acyclic": ["wir-call-graph-acyclic"],
    "att.language-type-system.no-prototype-pollution": ["language-property-fact"],
    "att.type-signatures.unconstrained-slices": ["signature-bound-check"],
    "att.warrant-ir.cost-semantics-linear": ["steps-bound-obligation"],
    "wir.lowering.faithful": ["differential-run", "mir-translation-validation"],
    "wir.lowering.volatile-by-name": ["body-lowering", "model-validation-test"],
    "wir.inline.sound": ["differential-run", "per-run-translation-validation"],
    "wir.interp.differential-only": ["differential-run"],
    "wir.cost.step-model": ["replacement-by-lower-layer-cost"],
    "memory.unaliased": ["wir-alias-split-proof", "miri-run"],
    "wir.memory.provenance-model": ["per-target-memory-model"],
    "wir.loop.bounded-k": ["loop-invariant-proof"],
    "wir.bound.heap-unchecked": ["wir-heap-bound-proof"],
    "wir.bound.depth-unchecked": ["wir-call-graph-acyclic"],
    "wir.proof.trivial-miter": [
        "non-collapsed-negative-control",
        "independent-cnf-rederivation",
    ],
    "wir.proof.obligation-binding": ["obligation-redigest"],
    "tcb.drat.checker-correct": ["checker-corpus", "independent-checker-cross-check"],
    "tcb.bitblast.encoding-correct": [
        "exhaustive-differential-test",
        "gate-level-equivalence-check",
    ],
    "tcb.symexec.engine-correct": ["sabotage-control", "concrete-two-run-cross-check"],
    "tcb.report.checker-correct": ["falsification-test", "checker-fuzzing"],
    "tcb.evidence.manifest-honesty": ["run-digest-binding", "ci-regeneration-gate"],
    "tcb.rust-toolchain.trusted": ["diverse-double-compilation"],
    UMBRELLA: ["all-refinements-discharged"],
    "cc.llvm-ir.no-secret-branch": ["ir-taint-analysis", "negative-control"],
    "cc.llvm-ir.addresses-public": ["ir-taint-analysis", "negative-control"],
    "cc.volatile-honoured": ["ir-volatile-cycle-check"],
    "cc.version-pinned": ["toolchain-tuple-manifest"],
    "mc.backend.no-secret-branch": [
        "asm-taint-analysis",
        "per-run-translation-validation",
    ],
    "mc.backend.addresses-public": [
        "asm-taint-analysis",
        "per-run-translation-validation",
    ],
    "mc.isa.timing-data-independent": ["structural-netlist-proof"],
    "mc.isa.model-faithful": ["isa-conformance-vectors"],
    "uarch.cache.address-trace-suffices": ["cache-model-run", "obs-model-extension"],
    "uarch.fetch.pc-trace": ["obs-event-extension"],
    "uarch.speculation.none": ["structural-netlist-proof"],
    "uarch.predictor.no-state-leak": ["structural-netlist-proof"],
    "uarch.shared-resources.none": ["single-core-machine-statement"],
    "gate.model.is-netlist": ["netlist-extraction", "step-function-differential-run"],
    "gate.primitives.correct": [
        "exhaustive-truth-table",
        "transistor-model-equivalence",
    ],
    "gate.clocking.one-instruction-per-cycle": ["structural-netlist-proof"],
    "gate.memory.abstraction": ["port-cut-proof"],
    "gate.isa-equivalence": ["one-step-miter", "drat-certificate"],
    "os.allocator.size-only": ["size-only-allocator-trace", "no-allocator-owned-lane"],
    "os.std.runtime-benign": ["no-std-owned-lane"],
    "os.time.vdso": ["vdso-unmapped-run"],
    "os.env.memory-reads": ["launch-env-allowlist"],
    "os.syscall.baseline": ["per-lane-baseline", "spec-sync-test"],
    "os.counters.not-live": ["host-counter-run", "simulator-step-count"],
    "os.platform.linux-only": ["per-platform-gate"],
    "gov.approval.blocked": ["hardware-key-approval"],
    "gov.catalogue.integrity": ["catalogue-hash-pin", "spec-sync-test"],
}

# Entries whose title/notes need a human sentence the spec table cannot give.
NOTES = {
    "memory.unaliased": (
        "Emitted today by ca-assurance-report evidence.rs whenever an obligation has "
        "buffers and label != proved."
    ),
    "src.analyzer.G-1": "Difficulty is S for ct-compare (empty dependency closure) and L in general; the worst case is recorded.",
    "wir.cost.step-model": "Discharged by replacement at lower layers rather than at WIR (spec section 4).",
    UMBRELLA: "Umbrella: counts as discharged only when every refined_by child is (spec section 3.5).",
}
SINCE = {"memory.unaliased": "VER26"}

TITLE_MAX = 110


# ---------------------------------------------------------------------------
# Assembly
# ---------------------------------------------------------------------------


def make_title(statement: str) -> str:
    """First sentence of the statement, trimmed to ``TITLE_MAX`` at a word boundary."""
    first = statement.split(". ", 1)[0].rstrip(".")
    if len(first) <= TITLE_MAX:
        return first
    cut = first[:TITLE_MAX].rsplit(" ", 1)[0]
    return cut.rstrip(",;:") + "..."


def kind_of(entry_id: str, family: str) -> str:
    if entry_id in KIND_BY_ID:
        return KIND_BY_ID[entry_id]
    for prefix, kind in KIND_PREFIX.items():
        if entry_id.startswith(prefix):
            return kind
    return KIND_BY_FAMILY[family]


def backlog_of(entry_id: str, tier: str) -> str:
    if entry_id in BACKLOG_BY_ID:
        return BACKLOG_BY_ID[entry_id]
    for prefix, backlog in BACKLOG_BY_PREFIX.items():
        if entry_id.startswith(prefix):
            return backlog
    return BACKLOG_BY_TIER[tier]


def build_entry(row: ac.SpecRow, refines: dict[str, str]) -> dict:
    """Merge one parsed spec row with the curated overlay."""
    eid = row.id
    kind = kind_of(eid, row.family)
    floor = eid in FLOOR
    aliases = [row.legacy_alias] if row.legacy_alias else []
    layer_labels = LABEL_OVERRIDES.get(eid, LAYER_LABELS[row.layer])
    discharge = None
    if not floor:
        discharge = {
            "evidence_kinds": EVIDENCE_KINDS[eid],
            "rung": RUNG_OVERRIDES.get(
                eid, TIER_RUNG[row.tier] if not eid.startswith("tcb.") else "R0+"
            ),
            "tools": list(row.tools),
            "difficulty": row.difficulty,
            "backlog": backlog_of(eid, row.tier),
            "plan": row.discharge,
        }
    entry = {
        "id": eid,
        "aliases": aliases,
        "refines": refines.get(eid),
        "refined_by": list(UMBRELLA_CHILDREN) if eid == UMBRELLA else [],
        "layer": row.layer,
        "family": row.family,
        "title": make_title(row.statement),
        "statement": row.statement,
        "kind": kind,
        "status": "floor"
        if floor
        else (
            "attested"
            if kind == "attestation"
            else "evidenced"
            if eid in EVIDENCED
            else "assumed"
        ),
        "applies_to": {
            "labels": layer_labels,
            "claims": CLAIM_OVERRIDES.get(eid, CLAIMS_BY_FAMILY[row.family]),
            "packages": ["*"] if eid in WILDCARD_PACKAGE else ["rust/ct-compare"],
        },
        "depends_on": DEPENDS_ON.get(eid, []),
        "discharge": discharge,
        "since": SINCE.get(eid, "VER13"),
        "legacy": row.legacy,
        "used_by": row.used_by,
        "tier": row.tier,
        "constrains_input_classes": [],
    }
    if eid in NOTES:
        entry["notes"] = NOTES[eid]
    return entry


def build_catalogue(spec_text: str) -> dict:
    """Build the whole catalogue document from the spec text."""
    rows = ac.parse_spec_inventory(spec_text)
    refines = {child: UMBRELLA for child in UMBRELLA_CHILDREN}
    return {
        "schema": ac.SCHEMA_ID,
        "assumptions": [build_entry(row, refines) for row in rows],
    }


def render(catalogue: dict) -> str:
    """Canonical text: two-space indent, UTF-8, trailing newline."""
    return json.dumps(catalogue, indent=2, ensure_ascii=False) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("--spec", type=Path, default=ac.SPEC_PATH)
    parser.add_argument("--out", type=Path, default=ac.CATALOGUE_PATH)
    parser.add_argument(
        "--check",
        action="store_true",
        help="fail if --out differs from the generated text",
    )
    args = parser.parse_args(argv)
    text = render(build_catalogue(args.spec.read_text(encoding="utf-8")))
    if args.check:
        if not args.out.exists() or args.out.read_text(encoding="utf-8") != text:
            print(
                f"{args.out} is stale; rerun generate_assumption_catalogue.py",
                file=sys.stderr,
            )
            return 1
        return 0
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(text, encoding="utf-8")
    print(f"wrote {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
