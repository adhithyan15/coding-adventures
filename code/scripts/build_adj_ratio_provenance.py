#!/usr/bin/env python3
"""Materialize the reviewed ratio stdlib provenance from retained bytes."""

from __future__ import annotations

import argparse
from collections.abc import Sequence
from pathlib import Path

import adj_stdlib_provenance as provenance

REPO_ROOT = Path(__file__).resolve().parents[2]
LIBRARY = "code/specs/data/adj-formula-stdlib/arithmetic/ratio.adj"
QUERY = "code/specs/data/adj-formula-stdlib/arithmetic/ratio.query.adj"
FIXTURE = "code/specs/data/adj-stdlib-provenance/fixtures/ratio-inputs.txt"
LOCATOR = "https://mathworld.wolfram.com/Ratio.html"
SOURCE_RETRIEVED_AT = "2026-08-02T22:38:04Z"
INPUT_CAPTURED_AT = "2026-08-02T23:39:01Z"
RAW_HASH = "8eced6f9859e60557b69ec9ef2c1cbaf31c7086cc0d9212edc7b39b52ef52baf"
RECEIPT_HASH = "ce1dc59cf73644563ee6c202b7067d3fab290cafab8534bd958eace36826c93b"
RENDERED_HASH = "93fd49105b8236bd47254e1da7e378388acba6468a1188a76fe4e3c22767be73"
RATIO_CLAIM = "adj.math.arithmetic.ratio"
QUESTION_CLAIM = "adj.question.arithmetic.ratio.compute"
SELECTED_TEXT = (
    b"The ratio of two numbers r and s is written r/s, where r is the numerator "
    b"and s is the denominator. The ratio of r to s is equivalent to the quotient "
    b"r/s."
)


def claim(claim_id: str, data: bytes, start: int, end: int) -> dict:
    cited = data[start:end]
    return {
        "claim_id": claim_id,
        "end": end,
        "quote": cited.decode("utf-8"),
        "quote_sha256": provenance.sha256_bytes(cited),
        "start": start,
    }


def source_segments(
    data: bytes,
    represented: list[tuple[int, int, list[dict]]],
    *,
    discarded_reason: str,
) -> list[dict]:
    segments = []
    cursor = 0
    for start, end, claims in sorted(represented, key=lambda item: (item[0], item[1])):
        if start < cursor:
            raise provenance.ProvenanceError("source claim ranges overlap")
        if cursor < start:
            segments.append(
                {
                    "disposition": "discarded",
                    "end": start,
                    "reason": discarded_reason,
                    "start": cursor,
                }
            )
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
        segments.append(
            {
                "disposition": "discarded",
                "end": len(data),
                "reason": discarded_reason,
                "start": cursor,
            }
        )
    return segments


def local_source(
    cas: provenance.Cas,
    repo_path: str,
    ranges: list[tuple[str, int, int]],
    label: str,
    *,
    discarded_reason: str,
) -> tuple[dict, dict[str, dict]]:
    data = provenance._read_regular_file(REPO_ROOT / repo_path)
    raw_hash = cas.put(data, kind="raw_source", label=label)
    receipt = provenance.build_input_receipt(
        repo_path=repo_path,
        captured_at=INPUT_CAPTURED_AT,
        body_sha256=raw_hash,
        body_size=len(data),
        body_git_sha1=provenance.git_blob_sha1(data),
    )
    receipt_hash = cas.put_json(
        receipt, kind="input_receipt", label=f"{label} receipt", links=[raw_hash]
    )
    grouped: dict[tuple[int, int], list[str]] = {}
    for claim_id, start, end in ranges:
        grouped.setdefault((start, end), []).append(claim_id)
    claims = {}
    represented = []
    for (start, end), claim_ids in sorted(grouped.items()):
        range_claims = []
        for claim_id in sorted(claim_ids):
            item = claim(claim_id, data, start, end)
            claims[claim_id] = item
            range_claims.append(item)
        represented.append((start, end, range_claims))
    ir = provenance.build_source_ir(
        source_sha256=raw_hash,
        source=data,
        segments=source_segments(
            data,
            represented,
            discarded_reason=discarded_reason,
        ),
    )
    ir_hash = cas.put_json(ir, kind="source_ir", label=f"{label} IR", links=[raw_hash])
    return (
        {
            "raw_source_sha256": raw_hash,
            "receipt_sha256": receipt_hash,
            "representations": [],
            "source_ir_sha256": ir_hash,
        },
        claims,
    )


def retained_external_source(
    cas: provenance.Cas, captured_source: Path | None
) -> tuple[dict, dict[str, dict]]:
    if captured_source is not None:
        captured = provenance._read_regular_file(captured_source)
        if provenance.sha256_bytes(captured) != RAW_HASH:
            raise provenance.ProvenanceError(
                "captured Ratio.html bytes do not match the reviewed SHA-256"
            )
    elif RAW_HASH not in cas.index:
        raise provenance.ProvenanceError(
            "reviewed Ratio.html bytes are absent; pass --captured-source once"
        )
    else:
        captured = provenance._read_regular_file(cas.object_path(RAW_HASH))
    raw_hash = cas.put(captured, kind="raw_source", label="MathWorld Ratio.html")
    if raw_hash != RAW_HASH or len(captured) != 52_191:
        raise provenance.ProvenanceError("retained Ratio.html identity is inconsistent")

    receipt = provenance.build_fetch_receipt(
        locator=LOCATOR,
        final_locator=LOCATOR,
        retrieved_at=SOURCE_RETRIEVED_AT,
        status=200,
        media_type="text/html; charset=UTF-8",
        body_sha256=raw_hash,
        body_size=len(captured),
        headers={"content-type": "text/html; charset=UTF-8"},
    )
    receipt_hash = cas.put_json(
        receipt,
        kind="fetch_receipt",
        label="MathWorld Ratio.html fetch receipt",
        links=[raw_hash],
    )
    if receipt_hash != RECEIPT_HASH:
        raise provenance.ProvenanceError("Ratio.html receipt does not match review")

    marker = b'<meta name="DC.Description" content="'
    if captured.count(marker) != 1:
        raise provenance.ProvenanceError(
            "MathWorld Ratio.html must contain one DC.Description metadata root"
        )
    selected_start = captured.index(marker) + len(marker)
    selected_end = selected_start + len(SELECTED_TEXT)
    if selected_start != 249 or selected_end != 403:
        raise provenance.ProvenanceError("reviewed Ratio.html byte range drifted")
    if captured[selected_start:selected_end] != SELECTED_TEXT:
        raise provenance.ProvenanceError("reviewed Ratio.html definition drifted")

    raw_claim = claim(RATIO_CLAIM, captured, selected_start, selected_end)
    raw_ir = provenance.build_source_ir(
        source_sha256=raw_hash,
        source=captured,
        segments=source_segments(
            captured,
            [
                (
                    selected_start,
                    selected_end,
                    [raw_claim],
                )
            ],
            discarded_reason=(
                "publisher markup and context outside the selected ratio rule"
            ),
        ),
    )
    raw_ir_hash = cas.put_json(
        raw_ir,
        kind="source_ir",
        label="MathWorld ratio raw IR",
        links=[raw_hash],
    )

    rendered_hash = cas.put(
        SELECTED_TEXT,
        kind="rendered_text",
        label="MathWorld ratio definition",
        links=[raw_hash],
    )
    if rendered_hash != RENDERED_HASH:
        raise provenance.ProvenanceError("rendered ratio definition hash drifted")
    rendered_claim = claim(RATIO_CLAIM, SELECTED_TEXT, 0, len(SELECTED_TEXT))
    rendered_ir = provenance.build_source_ir(
        source_sha256=rendered_hash,
        source=SELECTED_TEXT,
        segments=[
            {
                "claims": [rendered_claim],
                "disposition": "represented",
                "end": len(SELECTED_TEXT),
                "start": 0,
            }
        ],
    )
    rendered_ir_hash = cas.put_json(
        rendered_ir,
        kind="source_ir",
        label="MathWorld ratio rendered IR",
        links=[rendered_hash],
    )
    transform = provenance.build_text_transform(
        source_sha256=raw_hash,
        source=captured,
        result_sha256=rendered_hash,
        result=SELECTED_TEXT,
        operations=[
            {
                "operation": "copy",
                "result_end": len(SELECTED_TEXT),
                "result_start": 0,
                "source_end": selected_end,
                "source_start": selected_start,
            }
        ],
    )
    transform_hash = cas.put_json(
        transform,
        kind="text_transform",
        label="MathWorld ratio text transform",
        links=[raw_hash, rendered_hash],
    )
    return (
        {
            "raw_source_sha256": raw_hash,
            "receipt_sha256": receipt_hash,
            "representations": [
                {
                    "rendered_text_sha256": rendered_hash,
                    "source_ir_sha256": rendered_ir_hash,
                    "transform_sha256": transform_hash,
                }
            ],
            "source_ir_sha256": raw_ir_hash,
        },
        {RATIO_CLAIM: rendered_claim},
    )


def input_claim_payload(item: dict) -> dict:
    return {key: item[key] for key in ("end", "quote", "quote_sha256", "start")}


def build(
    cas: provenance.Cas,
    captured_source: Path | None,
    *,
    arithmetic_bundle_sha256: str,
    formula_inventory_command: Sequence[str],
    formula_audit_command: Sequence[str] | None = None,
) -> dict[str, str]:
    arithmetic_bundle_sha256 = provenance._require_hash(
        arithmetic_bundle_sha256, "arithmetic_bundle_sha256"
    )
    arithmetic = provenance._json_object(
        cas, arithmetic_bundle_sha256, "provenance_bundle"
    )
    if arithmetic.get("bundle_id") != "adj.math.arithmetic.primitives.v1":
        raise provenance.ProvenanceError("arithmetic dependency bundle ID drifted")

    external_source, external_claims = retained_external_source(cas, captured_source)
    library_bytes = provenance._read_regular_file(REPO_ROOT / LIBRARY)
    import_start = library_bytes.index(b'import "arithmetic.adj"')
    import_end = library_bytes.index(b"\n", import_start) + 1
    vocabulary_start = library_bytes.index(b"dictionary ratio_vocab {")
    vocabulary_end = library_bytes.index(b"}\n", vocabulary_start) + 2
    use_start = library_bytes.index(b"    use ratio_vocab")
    use_end = library_bytes.index(b"\n", use_start) + 1
    formula_start = library_bytes.index(b"    formula ratio(")
    trust_start = library_bytes.index(b"        trust authoritative", formula_start)
    formula_end = library_bytes.index(b"\n", trust_start) + 1
    input_source, input_claims = local_source(
        cas,
        LIBRARY,
        [
            ("adj.code.arithmetic.ratio.import.arithmetic", import_start, import_end),
            (
                "adj.code.arithmetic.ratio.vocabulary",
                vocabulary_start,
                vocabulary_end,
            ),
            ("adj.code.arithmetic.ratio.use.ratio_vocab", use_start, use_end),
            (RATIO_CLAIM, formula_start, formula_end),
        ],
        "ratio.adj input",
        discarded_reason=(
            "explanatory comments, separators, or closing syntax outside the "
            "selected import, vocabulary, use, and formula rules"
        ),
    )
    formula_inventory_hash = provenance.put_formula_parser_inventory(
        cas,
        input_source["raw_source_sha256"],
        formula_inventory_command,
        label="ratio.adj parser inventory",
    )

    ratio_claim = external_claims[RATIO_CLAIM]
    bundle = {
        "bundle_id": "adj.math.arithmetic.ratio.v1",
        "clauses": [
            {
                **ratio_claim,
                "input_claim": input_claim_payload(input_claims[RATIO_CLAIM]),
                "locator": LOCATOR,
                "resolution": {
                    "bundle_sha256": arithmetic_bundle_sha256,
                    "claim_id": "adj.math.arithmetic.quotient",
                    "kind": "dependency",
                },
                "snapshot_sha256": RENDERED_HASH,
                "source_ir_sha256": external_source["representations"][0][
                    "source_ir_sha256"
                ],
            }
        ],
        "dependencies": [arithmetic_bundle_sha256],
        "formula_inventory_sha256": formula_inventory_hash,
        "input": {
            key: input_source[key]
            for key in ("raw_source_sha256", "receipt_sha256", "source_ir_sha256")
        },
        "kind": "provenance_bundle",
        "library": LIBRARY,
        "sources": [input_source, external_source],
    }
    bundle_hash = cas.put_json(
        bundle,
        kind="provenance_bundle",
        label="ratio.adj provenance bundle",
        links=provenance._bundle_declared_links(bundle),
    )

    fixture_bytes = provenance._read_regular_file(REPO_ROOT / FIXTURE)
    query_bytes = provenance._read_regular_file(REPO_ROOT / QUERY)
    facts = (("numerator", "3"), ("denominator", "4"))
    fixture_ranges = []
    query_ranges = []
    for name, value in facts:
        claim_id = f"adj.input.arithmetic.ratio.{name}"
        sentence = f"{name} is {value}.".encode()
        fixture_start = fixture_bytes.index(sentence)
        fixture_ranges.append((claim_id, fixture_start, fixture_start + len(sentence)))
        query_start = query_bytes.index(f"observe {name}(".encode())
        query_trust = query_bytes.index(b"    trust authoritative", query_start)
        query_end = query_bytes.index(b"\n", query_trust) + 1
        query_ranges.append((claim_id, query_start, query_end))
    query_import_start = query_bytes.index(b'import "ratio.adj"')
    query_import_end = query_bytes.index(b"\n", query_import_start) + 1
    query_ranges.append(
        (
            "adj.code.arithmetic.ratio.query.import",
            query_import_start,
            query_import_end,
        )
    )
    question_start = query_bytes.index(b"? ratio(")
    question_end = query_bytes.index(b"\n", question_start) + 1
    query_ranges.append((QUESTION_CLAIM, question_start, question_end))

    fixture_source, fixture_claims = local_source(
        cas,
        FIXTURE,
        fixture_ranges,
        "ratio input fixture",
        discarded_reason="newline record separators outside the accepted fact bytes",
    )
    query_source, query_claims = local_source(
        cas,
        QUERY,
        query_ranges,
        "ratio.query.adj input",
        discarded_reason=(
            "introductory comment, spacing, or human-readable test oracle outside "
            "the selected import, facts, and executable question"
        ),
    )
    fixture_locator = f"repo://{FIXTURE}"
    query_clauses = []
    for name, _value in facts:
        claim_id = f"adj.input.arithmetic.ratio.{name}"
        fact = fixture_claims[claim_id]
        query_clauses.append(
            {
                **fact,
                "input_claim": input_claim_payload(query_claims[claim_id]),
                "locator": fixture_locator,
                "resolution": {
                    "authority_receipt_sha256": fixture_source["receipt_sha256"],
                    "authority_source_sha256": fixture_source["raw_source_sha256"],
                    "classification": "accepted_fact",
                    "kind": "accepted_root",
                    "reason": (
                        "deterministic ratio-query input retained as the explicit "
                        "accepted fact"
                    ),
                },
                "snapshot_sha256": fixture_source["raw_source_sha256"],
                "source_ir_sha256": fixture_source["source_ir_sha256"],
            }
        )
    query_bundle = {
        "bundle_id": "adj.math.arithmetic.ratio.query.v1",
        "clauses": query_clauses,
        "dependencies": [bundle_hash],
        "input": {
            key: query_source[key]
            for key in ("raw_source_sha256", "receipt_sha256", "source_ir_sha256")
        },
        "kind": "provenance_bundle",
        "library": QUERY,
        "sources": [query_source, fixture_source],
    }
    derivations, witnesses = provenance.put_formula_execution_evidence(
        cas,
        query_bundle,
        formula_audit_command,
        label="ratio.query.adj execution witness",
    )
    query_bundle["formula_derivation_sha256s"] = derivations
    query_bundle["execution_witness_sha256s"] = witnesses
    query_bundle_hash = cas.put_json(
        query_bundle,
        kind="provenance_bundle",
        label="ratio.query.adj provenance bundle",
        links=provenance._bundle_declared_links(query_bundle),
    )
    return {
        bundle["bundle_id"]: bundle_hash,
        query_bundle["bundle_id"]: query_bundle_hash,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--captured-source",
        type=Path,
        help="reviewed Ratio.html bytes for the one-time CAS bootstrap",
    )
    parser.add_argument(
        "--arithmetic-bundle-sha256",
        required=True,
        type=lambda value: provenance._require_hash(
            value, "--arithmetic-bundle-sha256"
        ),
        help="verified current primitive arithmetic provenance root",
    )
    parser.add_argument("--formula-inventory-binary", type=Path, required=True)
    parser.add_argument("--formula-audit-binary", type=Path, required=True)
    args = parser.parse_args()
    formula_inventory_command = [str(args.formula_inventory_binary.resolve())]
    formula_audit_command = [str(args.formula_audit_binary.resolve())]
    with provenance.BundleRegistrationTransaction(
        REPO_ROOT / provenance.DEFAULT_ROOT,
        REPO_ROOT / provenance.DEFAULT_MANIFEST,
        expected_manifest_id="adj.stdlib.provenance.v1",
        schema_path=REPO_ROOT / provenance.DEFAULT_SCHEMA,
        workspace_root=REPO_ROOT,
        formula_inventory_command=formula_inventory_command,
        formula_audit_command=formula_audit_command,
    ) as transaction:
        transaction.commit(
            build(
                transaction.cas,
                args.captured_source,
                arithmetic_bundle_sha256=args.arithmetic_bundle_sha256,
                formula_inventory_command=formula_inventory_command,
                formula_audit_command=formula_audit_command,
            )
        )


if __name__ == "__main__":
    main()
