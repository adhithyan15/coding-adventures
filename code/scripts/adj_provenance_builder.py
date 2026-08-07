#!/usr/bin/env python3
"""Shared helpers for the per-library provenance builders.

Every `build_adj_*_provenance.py` script materializes one library's CAS bundle.
They differ in WHICH bytes they cite and how those bytes decompose, but three
helpers were copied verbatim between them, and a copied helper in a provenance
builder is worse than ordinary duplication: these functions decide the exact
`start`/`end`/`quote_sha256` a claim is pinned to, so two copies drifting apart
would mean two libraries disagreeing about what "cited" means while both
reporting success.

Scope note: this deliberately does NOT try to generalize `build()`. The four
builders split into two shapes — a ROOT builder decomposing an external source
(inherently document-specific) and DEPENDENT builders composing an
already-verified root. Only the pieces that are provably identical move here.

Every extraction in this module is verified byte-exactly: re-running each
builder must leave the checked-in CAS unchanged.
"""

from __future__ import annotations

import dataclasses

import adj_stdlib_provenance as provenance


def claim(claim_id: str, data: bytes, start: int, end: int) -> dict:
    cited = data[start:end]
    return {
        "claim_id": claim_id,
        "end": end,
        "quote": cited.decode("utf-8"),
        "quote_sha256": provenance.sha256_bytes(cited),
        "start": start,
    }


# The `reasoned_discards` parameter is what made this the superset version.
# `ratio` carried a variant without it; called with the default, the loop below
# never runs and the two are equivalent — verified by rebuild, not by reading.
def source_segments(
    data: bytes,
    represented: list[tuple[int, int, list[dict]]],
    *,
    discarded_reason: str,
    reasoned_discards: list[tuple[int, int, str]] | None = None,
) -> list[dict]:
    segments = []
    cursor = 0

    def discard(start: int, end: int) -> None:
        discard_cursor = start
        for special_start, special_end, reason in reasoned_discards or []:
            if special_end <= start or special_start >= end:
                continue
            if special_start < start or special_end > end:
                raise provenance.ProvenanceError(
                    "reasoned discard crosses a represented byte range"
                )
            if discard_cursor < special_start:
                segments.append(
                    {
                        "disposition": "discarded",
                        "end": special_start,
                        "reason": discarded_reason,
                        "start": discard_cursor,
                    }
                )
            segments.append(
                {
                    "disposition": "discarded",
                    "end": special_end,
                    "reason": reason,
                    "start": special_start,
                }
            )
            discard_cursor = special_end
        if discard_cursor < end:
            segments.append(
                {
                    "disposition": "discarded",
                    "end": end,
                    "reason": discarded_reason,
                    "start": discard_cursor,
                }
            )

    for start, end, claims in sorted(represented, key=lambda item: (item[0], item[1])):
        if start < cursor:
            raise provenance.ProvenanceError("source claim ranges overlap")
        if cursor < start:
            discard(cursor, start)
        segments.append(
            {
                "claims": claims,
                "disposition": "represented",
                "end": end,
                "start": start,
            }
        )
        cursor = end
    if cursor < len(data):
        discard(cursor, len(data))
    return segments


def input_claim_payload(item: dict) -> dict:
    return {key: item[key] for key in ("end", "quote", "quote_sha256", "start")}


@dataclasses.dataclass(frozen=True)
class QueryLibrarySpec:
    """The per-library data a worked-query bundle needs.

    Everything here is a value that genuinely differs between libraries. The
    logic that consumes it — scanning the query for its observations, import and
    question, assembling the accepted-fact clauses, and registering the execution
    evidence — is identical, and lives in `build_query_bundle`.

    `claim_prefix` + `qualify_by_value` deserve a note, because they look like two
    knobs and are really one. A library either identifies an input by NAME
    (`…ratio.numerator`, matching `observe numerator(`) or by NAME AND VALUE
    (`…proportion.first_term.2`, matching `observe first_term(2)`). The claim id
    and the byte pattern must agree — a library that qualified one and not the
    other would look up a claim it never recorded — so one flag drives both.
    """

    bundle_id: str
    query_path: str
    fixture_path: str
    claim_prefix: str
    import_literal: bytes
    import_claim_id: str
    question_prefix: bytes
    question_claim_id: str
    accepted_fact_reason: str
    discarded_reason: str
    input_description: str
    witness_label: str
    qualify_by_value: bool = False
    scan_bindings: bool = False
    # Optional: given the query bytes and the offset just past the question,
    # return spans to discard WITH A STATED REASON rather than under the blanket
    # `discarded_reason`. A query that ships a deliberately disabled example owes
    # the reader why those bytes are unrepresented, not merely that they are.
    reasoned_discards: object | None = None

    def input_claim_id(self, name: str, value: str) -> str:
        return (
            f"{self.claim_prefix}.{name}.{value}"
            if self.qualify_by_value
            else f"{self.claim_prefix}.{name}"
        )

    def observe_pattern(self, name: str, value: str) -> bytes:
        body = f"{name}({value})" if self.qualify_by_value else f"{name}("
        return f"observe {body}".encode()


def build_query_bundle(
    cas: provenance.Cas,
    *,
    spec: QueryLibrarySpec,
    repo_root,
    facts: tuple[tuple[str, str], ...],
    library_hash: str,
    fixture_source: dict,
    fixture_claims: dict[str, dict],
    formula_audit_command,
    local_source,
    query_bytes: bytes | None = None,
) -> tuple[str, str]:
    """Register one worked query's provenance bundle.

    `local_source` is injected rather than shared: each builder decomposes its
    own files differently, and forcing one implementation would change which
    bytes are represented versus discarded — the one thing a provenance refactor
    must never do silently.
    """
    # Read ONCE, and hand those exact bytes to `local_source` below. The offsets
    # in `query_ranges` are computed from this read; if `local_source` re-read the
    # file, a same-length replacement in between would pin a claim to a byte range
    # that no longer contains what the claim says — and the result would still
    # verify, because the stored IR would be self-consistent with the second read.
    caller_supplied_bytes = query_bytes is not None
    if query_bytes is None:
        query_bytes = provenance._read_regular_file(repo_root / spec.query_path)

    query_ranges = []
    for name, value in facts:
        start = query_bytes.index(spec.observe_pattern(name, value))
        trust = query_bytes.index(b"    trust authoritative", start)
        end = query_bytes.index(b"\n", trust) + 1
        query_ranges.append((spec.input_claim_id(name, value), start, end))

    import_start = query_bytes.index(spec.import_literal)
    import_end = query_bytes.index(b"\n", import_start) + 1
    query_ranges.append((spec.import_claim_id, import_start, import_end))

    question_start = query_bytes.index(spec.question_prefix)
    question_end = query_bytes.index(b"\n", question_start) + 1
    query_ranges.append((spec.question_claim_id, question_start, question_end))

    # Multi-step queries name each intermediate `let`, so the witness can bind a
    # computation to the authored line that produced it. Single-expression
    # queries have none, and scanning them would emit claims for nothing.
    if spec.scan_bindings:
        binding_cursor = 0
        binding_index = 0
        while True:
            binding_start = query_bytes.find(b"let ", binding_cursor)
            if binding_start < 0:
                break
            binding_end = query_bytes.index(b"\n", binding_start) + 1
            query_ranges.append(
                (
                    f"{spec.bundle_id}.binding.{binding_index}",
                    binding_start,
                    binding_end,
                )
            )
            binding_cursor = binding_end
            binding_index += 1

    query_source, query_claims = local_source(
        cas,
        spec.query_path,
        query_ranges,
        spec.input_description,
        discarded_reason=spec.discarded_reason,
        data=query_bytes,
        # A caller-supplied snapshot may legitimately differ from the working
        # tree (that is what a workspace snapshot IS), so the on-disk equality
        # check applies only when we read the file ourselves. Note this gates a
        # CHECK, not which bytes are used — the bytes are the same either way,
        # unlike the conditional forwarding that caused the #9926 double-read.
        on_disk=not caller_supplied_bytes,
        reasoned_discards=(
            spec.reasoned_discards(query_bytes, question_end)
            if spec.reasoned_discards is not None
            else None
        ),
    )

    fixture_locator = f"repo://{spec.fixture_path}"
    clauses = []
    for name, value in facts:
        claim_id = spec.input_claim_id(name, value)
        clauses.append(
            {
                **fixture_claims[claim_id],
                "input_claim": input_claim_payload(query_claims[claim_id]),
                "locator": fixture_locator,
                "resolution": {
                    "authority_receipt_sha256": fixture_source["receipt_sha256"],
                    "authority_source_sha256": fixture_source["raw_source_sha256"],
                    "classification": "accepted_fact",
                    "kind": "accepted_root",
                    "reason": spec.accepted_fact_reason,
                },
                "snapshot_sha256": fixture_source["raw_source_sha256"],
                "source_ir_sha256": fixture_source["source_ir_sha256"],
            }
        )

    query_name = spec.query_path.rsplit("/", 1)[-1]
    bundle = {
        "bundle_id": spec.bundle_id,
        "clauses": clauses,
        "dependencies": [library_hash],
        "input": {
            key: query_source[key]
            for key in ("raw_source_sha256", "receipt_sha256", "source_ir_sha256")
        },
        "kind": "provenance_bundle",
        "library": spec.query_path,
        "sources": [query_source, fixture_source],
    }
    derivations, witnesses = provenance.put_formula_execution_evidence(
        cas, bundle, formula_audit_command, label=spec.witness_label
    )
    bundle["formula_derivation_sha256s"] = derivations
    bundle["execution_witness_sha256s"] = witnesses
    bundle_hash = cas.put_json(
        bundle,
        kind="provenance_bundle",
        label=f"{query_name} provenance bundle",
        links=provenance._bundle_declared_links(bundle),
    )
    return spec.bundle_id, bundle_hash
