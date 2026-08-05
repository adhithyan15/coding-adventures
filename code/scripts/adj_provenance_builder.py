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
