"""Compatibility corpus and release-readiness gates for SPICE deck parity."""

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
from math import isfinite


@dataclass(frozen=True, slots=True)
class CompatibilityOracle:
    """Documented oracle source for a compatibility deck."""

    reference: str
    version: str
    source: str


@dataclass(frozen=True, slots=True)
class CompatibilityGoldenValue:
    """A named oracle value and tolerance for a deck probe."""

    name: str
    value: float
    unit: str
    abs_tol: float
    rel_tol: float


@dataclass(frozen=True, slots=True)
class CompatibilityDeck:
    """A compact SPICE deck fixture with oracle metadata and known gaps."""

    id: str
    title: str
    analysis: str
    netlist: str
    oracle: CompatibilityOracle
    golden_values: tuple[CompatibilityGoldenValue, ...]
    known_incompatibilities: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class ReleaseReadinessIssue:
    """A release-readiness gate violation for a corpus deck."""

    deck_id: str
    field: str
    message: str


@dataclass(frozen=True, slots=True)
class ReleaseReadinessReport:
    """Summary of package release-readiness gates for the compatibility corpus."""

    passed: bool
    deck_count: int
    analyses: tuple[str, ...]
    issues: tuple[ReleaseReadinessIssue, ...]


_COMMON_KNOWN_INCOMPATIBILITIES = (
    "binary rawfile output is not part of this release gate",
    ".control blocks and vendor-specific directives are intentionally excluded",
    "golden values cover named probes, not byte-for-byte waveform dumps",
)

_COMPATIBILITY_CORPUS = (
    CompatibilityDeck(
        id="dc-op-resistive-divider",
        title="DC operating point resistive divider",
        analysis="op",
        netlist="""* dc-op-resistive-divider
V1 in 0 DC 10
R1 in out 10000
R2 out 0 10000
.op
.end
""",
        oracle=CompatibilityOracle(
            reference="closed-form",
            version="divider-v1",
            source="V(out)=V1*R2/(R1+R2); I(V1)=-V1/(R1+R2)",
        ),
        golden_values=(
            CompatibilityGoldenValue("V(out)", 5.0, "V", 1.0e-9, 1.0e-9),
            CompatibilityGoldenValue("I(V1)", -5.0e-4, "A", 1.0e-12, 1.0e-9),
        ),
        known_incompatibilities=_COMMON_KNOWN_INCOMPATIBILITIES,
    ),
    CompatibilityDeck(
        id="dc-sweep-resistive-divider",
        title="DC source sweep resistive divider",
        analysis="dc",
        netlist="""* dc-sweep-resistive-divider
V1 in 0 DC 0
R1 in out 10000
R2 out 0 10000
.dc V1 0 10 5
.end
""",
        oracle=CompatibilityOracle(
            reference="closed-form",
            version="divider-sweep-v1",
            source="V(out)=V1*0.5 at each sweep point",
        ),
        golden_values=(
            CompatibilityGoldenValue("points", 3.0, "count", 0.0, 0.0),
            CompatibilityGoldenValue("V(out)@V1=10", 5.0, "V", 1.0e-9, 1.0e-9),
        ),
        known_incompatibilities=_COMMON_KNOWN_INCOMPATIBILITIES,
    ),
    CompatibilityDeck(
        id="ac-rc-lowpass",
        title="AC RC low-pass cutoff",
        analysis="ac",
        netlist="""* ac-rc-lowpass
V1 in 0 DC 0 AC 1
R1 in out 1000
C1 out 0 1u
.ac dec 1 1 1k
.end
""",
        oracle=CompatibilityOracle(
            reference="closed-form",
            version="rc-lowpass-v1",
            source="|V(out)|=1/sqrt(1+(2*pi*f*R*C)^2)",
        ),
        golden_values=(
            CompatibilityGoldenValue("f_c", 159.15494309189535, "Hz", 1.0e-9, 1.0e-9),
            CompatibilityGoldenValue("|V(out)|@f_c", 0.7071067811865475, "V", 1.0e-9, 1.0e-9),
        ),
        known_incompatibilities=_COMMON_KNOWN_INCOMPATIBILITIES,
    ),
    CompatibilityDeck(
        id="tran-rc-step",
        title="Transient RC step response",
        analysis="tran",
        netlist="""* tran-rc-step
V1 in 0 PULSE(0 1 0 1n 1n 1m 2m)
R1 in out 1000
C1 out 0 1u
.tran 0.0001 0.001
.end
""",
        oracle=CompatibilityOracle(
            reference="closed-form",
            version="rc-step-v1",
            source="V(out,t)=1-exp(-t/(R*C)) after an ideal 1 V step",
        ),
        golden_values=(
            CompatibilityGoldenValue("V(out)@1ms", 0.6321205588285577, "V", 1.0e-6, 1.0e-6),
        ),
        known_incompatibilities=_COMMON_KNOWN_INCOMPATIBILITIES
        + ("finite-edge pulse decks compare at the idealized step oracle point",),
    ),
    CompatibilityDeck(
        id="tf-resistive-divider",
        title="Transfer-function resistive divider",
        analysis="tf",
        netlist="""* tf-resistive-divider
V1 in 0 DC 10
R1 in out 10000
R2 out 0 10000
.tf V(out) V1
.end
""",
        oracle=CompatibilityOracle(
            reference="closed-form",
            version="divider-tf-v1",
            source="gain=R2/(R1+R2); input resistance=R1+R2",
        ),
        golden_values=(
            CompatibilityGoldenValue("gain", 0.5, "V/V", 1.0e-9, 1.0e-9),
            CompatibilityGoldenValue("input_resistance", 20000.0, "ohm", 1.0e-6, 1.0e-9),
        ),
        known_incompatibilities=_COMMON_KNOWN_INCOMPATIBILITIES,
    ),
)

_SUPPORTED_ANALYSES = frozenset({"op", "dc", "ac", "tran", "tf"})
_REQUIRED_ANALYSES = frozenset({"op", "dc", "ac", "tran"})


def compatibility_corpus() -> tuple[CompatibilityDeck, ...]:
    """Return the canonical first release-readiness compatibility corpus."""

    return _COMPATIBILITY_CORPUS


def release_readiness_gates(
    corpus: Sequence[CompatibilityDeck] | None = None,
) -> ReleaseReadinessReport:
    """Validate the compatibility corpus metadata used as package release gates."""

    decks = _COMPATIBILITY_CORPUS if corpus is None else tuple(corpus)
    issues: list[ReleaseReadinessIssue] = []
    seen_ids: set[str] = set()
    analyses: list[str] = []

    if not decks:
        issues.append(
            ReleaseReadinessIssue(
                deck_id="corpus",
                field="deck_count",
                message="compatibility corpus must contain at least one deck",
            )
        )

    for deck in decks:
        deck_id = deck.id or "<missing>"
        _validate_non_empty(deck_id, "id", deck.id, issues)
        _validate_non_empty(deck_id, "title", deck.title, issues)
        _validate_non_empty(deck_id, "netlist", deck.netlist, issues)
        _validate_non_empty(deck_id, "oracle.reference", deck.oracle.reference, issues)
        _validate_non_empty(deck_id, "oracle.version", deck.oracle.version, issues)
        _validate_non_empty(deck_id, "oracle.source", deck.oracle.source, issues)
        if deck.id in seen_ids:
            issues.append(
                ReleaseReadinessIssue(deck_id, "id", "deck ids must be unique")
            )
        seen_ids.add(deck.id)
        if deck.analysis not in _SUPPORTED_ANALYSES:
            issues.append(
                ReleaseReadinessIssue(
                    deck_id,
                    "analysis",
                    f"unsupported analysis {deck.analysis!r}",
                )
            )
        elif deck.analysis not in analyses:
            analyses.append(deck.analysis)
        if ".end" not in deck.netlist.lower():
            issues.append(
                ReleaseReadinessIssue(deck_id, "netlist", "deck must include .end")
            )
        if not deck.golden_values:
            issues.append(
                ReleaseReadinessIssue(
                    deck_id,
                    "golden_values",
                    "deck must include at least one golden value",
                )
            )
        for index, golden in enumerate(deck.golden_values):
            field_prefix = f"golden_values[{index}]"
            _validate_non_empty(deck_id, f"{field_prefix}.name", golden.name, issues)
            _validate_non_empty(deck_id, f"{field_prefix}.unit", golden.unit, issues)
            if not isfinite(golden.value):
                issues.append(
                    ReleaseReadinessIssue(
                        deck_id,
                        f"{field_prefix}.value",
                        "golden value must be finite",
                    )
                )
            if golden.abs_tol < 0.0 or golden.rel_tol < 0.0:
                issues.append(
                    ReleaseReadinessIssue(
                        deck_id,
                        f"{field_prefix}.tolerance",
                        "tolerances must be non-negative",
                    )
                )
            if golden.abs_tol == 0.0 and golden.rel_tol == 0.0 and golden.unit != "count":
                issues.append(
                    ReleaseReadinessIssue(
                        deck_id,
                        f"{field_prefix}.tolerance",
                        "non-count golden values need an absolute or relative tolerance",
                    )
                )
        if not deck.known_incompatibilities:
            issues.append(
                ReleaseReadinessIssue(
                    deck_id,
                    "known_incompatibilities",
                    "deck must document known incompatibility boundaries",
                )
            )

    missing = sorted(_REQUIRED_ANALYSES.difference(analyses))
    for analysis in missing:
        issues.append(
            ReleaseReadinessIssue(
                deck_id="corpus",
                field="analysis_coverage",
                message=f"missing required {analysis!r} compatibility deck",
            )
        )

    return ReleaseReadinessReport(
        passed=not issues,
        deck_count=len(decks),
        analyses=tuple(analyses),
        issues=tuple(issues),
    )


def format_compatibility_corpus_table(
    corpus: Sequence[CompatibilityDeck] | None = None,
) -> str:
    """Return a stable tab-separated compatibility corpus summary."""

    decks = _COMPATIBILITY_CORPUS if corpus is None else tuple(corpus)
    lines = ["id\tanalysis\toracle\tgolden_values\tknown_incompatibilities"]
    for deck in decks:
        golden = ",".join(
            f"{entry.name}={entry.value:.6e}{entry.unit}"
            for entry in deck.golden_values
        )
        lines.append(
            "\t".join(
                [
                    deck.id,
                    deck.analysis,
                    f"{deck.oracle.reference}@{deck.oracle.version}",
                    golden,
                    str(len(deck.known_incompatibilities)),
                ]
            )
        )
    return "\n".join(lines)


def format_release_readiness_report(report: ReleaseReadinessReport) -> str:
    """Return a stable tab-separated release-readiness report."""

    lines = [
        "passed\tdeck_count\tanalyses\tissue_count",
        f"{str(report.passed).lower()}\t{report.deck_count}\t{','.join(report.analyses)}\t{len(report.issues)}",
    ]
    if report.issues:
        lines.append("deck_id\tfield\tmessage")
        lines.extend(
            f"{issue.deck_id}\t{issue.field}\t{issue.message}" for issue in report.issues
        )
    return "\n".join(lines)


def _validate_non_empty(
    deck_id: str,
    field: str,
    value: str,
    issues: list[ReleaseReadinessIssue],
) -> None:
    if value.strip():
        return
    issues.append(
        ReleaseReadinessIssue(deck_id, field, "field must be documented and non-empty")
    )
