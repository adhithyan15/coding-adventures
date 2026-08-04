#!/usr/bin/env python3
"""Materialize the reviewed percent-of stdlib provenance from retained bytes."""

from __future__ import annotations

import argparse
from collections.abc import Sequence
from pathlib import Path

import adj_stdlib_provenance as provenance

REPO_ROOT = Path(__file__).resolve().parents[2]
LIBRARY = "code/specs/data/adj-formula-stdlib/arithmetic/percent-of.adj"
QUERY = "code/specs/data/adj-formula-stdlib/arithmetic/percent-of.query.adj"
FIXTURE = "code/specs/data/adj-stdlib-provenance/fixtures/percent-of-inputs.txt"
LOCATOR = (
    "https://openstax.org/books/contemporary-mathematics/pages/3-4-rational-numbers"
)
SOURCE_RETRIEVED_AT = "2026-08-03T00:19:10Z"
INPUT_CAPTURED_AT = "2026-08-03T00:30:00Z"
RAW_HASH = "89ebca7f93281cae7d8791cb6dfc65ff4ff289268fc5d7f03d41bb10adeb4e5e"
RECEIPT_HASH = "a381508533a090ccf7cfc8cf8169c9d6496fe3dd041ff11dfcd78d13f40cfe27"
RENDERED_HASH = "ac765615e8da33fba525925ae28f392d3c9343ac7539ea54fd0a103ca18fc718"
PERCENT_OF_CLAIM = "adj.math.arithmetic.percent_of"
QUESTION_CLAIM = "adj.question.arithmetic.percent_of.compute"
SELECTED_TEXT = b"n% of x items is (n/100)*x."
CLAIM_START = 395_628
CLAIM_END = 397_353
CLAIM_RAW_HASH = "7c273c4e9214ced84babdfb376497ccd2e02d046d0f6a570fc12972cdd5c9203"
TRANSFORM_OPERATIONS = (
    ("mathml_to_infix", 395_677, 395_842, 0, 2),
    ("copy", 395_849, 395_853, 2, 6),
    ("mathml_to_infix", 395_883, 396_028, 6, 7),
    ("copy", 396_035, 396_045, 7, 17),
    ("mathml_to_infix", 396_075, 396_368, 17, 26),
    ("copy", 396_375, 396_376, 26, 27),
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


def local_source(
    cas: provenance.Cas,
    repo_path: str,
    ranges: list[tuple[str, int, int]],
    label: str,
    *,
    discarded_reason: str,
    reasoned_discards: list[tuple[int, int, str]] | None = None,
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
            reasoned_discards=reasoned_discards,
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
                "captured OpenStax bytes do not match the reviewed SHA-256"
            )
    elif RAW_HASH not in cas.index:
        raise provenance.ProvenanceError(
            "reviewed OpenStax bytes are absent; pass --captured-source once"
        )
    else:
        captured = provenance._read_regular_file(cas.object_path(RAW_HASH))
    raw_hash = cas.put(captured, kind="raw_source", label="OpenStax percent source")
    if raw_hash != RAW_HASH or len(captured) != 666_580:
        raise provenance.ProvenanceError("retained OpenStax identity is inconsistent")

    receipt = provenance.build_fetch_receipt(
        locator=LOCATOR,
        final_locator=LOCATOR,
        retrieved_at=SOURCE_RETRIEVED_AT,
        status=200,
        media_type="text/html",
        body_sha256=raw_hash,
        body_size=len(captured),
        headers={
            "content-length": "666580",
            "content-type": "text/html",
            "etag": '"4ce59604c29ad47a8b0016aebe8d5e49"',
            "last-modified": "Mon, 15 Jun 2026 17:33:45 GMT",
        },
    )
    receipt_hash = cas.put_json(
        receipt,
        kind="fetch_receipt",
        label="OpenStax percent fetch receipt",
        links=[raw_hash],
    )
    if receipt_hash != RECEIPT_HASH:
        raise provenance.ProvenanceError("OpenStax receipt does not match review")

    raw_claim_bytes = captured[CLAIM_START:CLAIM_END]
    if provenance.sha256_bytes(raw_claim_bytes) != CLAIM_RAW_HASH:
        raise provenance.ProvenanceError("reviewed OpenStax formula bytes drifted")
    raw_claim = claim(PERCENT_OF_CLAIM, captured, CLAIM_START, CLAIM_END)
    raw_ir = provenance.build_source_ir(
        source_sha256=raw_hash,
        source=captured,
        segments=source_segments(
            captured,
            [(CLAIM_START, CLAIM_END, [raw_claim])],
            discarded_reason=(
                "publisher markup and textbook context outside the selected formal "
                "percent-of rule"
            ),
        ),
    )
    raw_ir_hash = cas.put_json(
        raw_ir,
        kind="source_ir",
        label="OpenStax percent raw IR",
        links=[raw_hash],
    )

    rendered_hash = cas.put(
        SELECTED_TEXT,
        kind="rendered_text",
        label="OpenStax percent-of rule",
        links=[raw_hash],
    )
    if rendered_hash != RENDERED_HASH:
        raise provenance.ProvenanceError("rendered percent_of definition hash drifted")
    rendered_claim = claim(PERCENT_OF_CLAIM, SELECTED_TEXT, 0, len(SELECTED_TEXT))
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
        label="OpenStax percent-of rendered IR",
        links=[rendered_hash],
    )
    transform = provenance.build_text_transform(
        source_sha256=raw_hash,
        source=captured,
        result_sha256=rendered_hash,
        result=SELECTED_TEXT,
        operations=[
            {
                "operation": operation,
                "result_end": result_end,
                "result_start": result_start,
                "source_end": source_end,
                "source_start": source_start,
            }
            for operation, source_start, source_end, result_start, result_end in TRANSFORM_OPERATIONS
        ],
    )
    transform_hash = cas.put_json(
        transform,
        kind="text_transform",
        label="OpenStax percent-of text transform",
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
        {PERCENT_OF_CLAIM: rendered_claim},
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
    vocabulary_start = library_bytes.index(b"dictionary percent_of_vocab {")
    vocabulary_end = library_bytes.index(b"}\n", vocabulary_start) + 2
    use_start = library_bytes.index(b"    use percent_of_vocab")
    use_end = library_bytes.index(b"\n", use_start) + 1
    formula_start = library_bytes.index(b"    formula percent_of(")
    trust_start = library_bytes.index(b"        trust authoritative", formula_start)
    formula_end = library_bytes.index(b"\n", trust_start) + 1
    input_source, input_claims = local_source(
        cas,
        LIBRARY,
        [
            (
                "adj.code.arithmetic.percent_of.import.arithmetic",
                import_start,
                import_end,
            ),
            (
                "adj.code.arithmetic.percent_of.vocabulary",
                vocabulary_start,
                vocabulary_end,
            ),
            ("adj.code.arithmetic.percent_of.use.percent_of_vocab", use_start, use_end),
            (PERCENT_OF_CLAIM, formula_start, formula_end),
        ],
        "percent-of.adj input",
        discarded_reason=(
            "explanatory comments, separators, or closing syntax outside the "
            "selected import, vocabulary, use, and formula rules"
        ),
    )
    formula_inventory_hash = provenance.put_formula_parser_inventory(
        cas,
        input_source["raw_source_sha256"],
        formula_inventory_command,
        label="percent-of.adj parser inventory",
    )

    percent_of_claim = external_claims[PERCENT_OF_CLAIM]
    bundle = {
        "bundle_id": "adj.math.arithmetic.percent_of.v1",
        "clauses": [
            {
                **percent_of_claim,
                "input_claim": input_claim_payload(input_claims[PERCENT_OF_CLAIM]),
                "locator": LOCATOR,
                "resolution": {
                    "authority_receipt_sha256": external_source["receipt_sha256"],
                    "authority_source_sha256": external_source["raw_source_sha256"],
                    "classification": "primary_definition",
                    "kind": "accepted_root",
                    "reason": (
                        "OpenStax percent-of rule retained as the explicit "
                        "definitional root"
                    ),
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
        label="percent-of.adj provenance bundle",
        links=provenance._bundle_declared_links(bundle),
    )

    fixture_bytes = provenance._read_regular_file(REPO_ROOT / FIXTURE)
    query_bytes = provenance._read_regular_file(REPO_ROOT / QUERY)
    facts = (("whole", "50"), ("rate", "20"))
    fixture_ranges = []
    query_ranges = []
    for name, value in facts:
        claim_id = f"adj.input.arithmetic.percent_of.{name}"
        sentence = f"{name} is {value}.".encode()
        fixture_start = fixture_bytes.index(sentence)
        fixture_ranges.append((claim_id, fixture_start, fixture_start + len(sentence)))
        query_start = query_bytes.index(f"observe {name}(".encode())
        query_trust = query_bytes.index(b"    trust authoritative", query_start)
        query_end = query_bytes.index(b"\n", query_trust) + 1
        query_ranges.append((claim_id, query_start, query_end))
    query_import_start = query_bytes.index(b'import "percent-of.adj"')
    query_import_end = query_bytes.index(b"\n", query_import_start) + 1
    query_ranges.append(
        (
            "adj.code.arithmetic.percent_of.query.import",
            query_import_start,
            query_import_end,
        )
    )
    question_start = query_bytes.index(b"? percent_of(")
    question_end = query_bytes.index(b"\n", question_start) + 1
    query_ranges.append((QUESTION_CLAIM, question_start, question_end))
    disabled_start = query_bytes.index(
        b"% ----------------------------------------------------------------------------",
        question_end,
    )

    fixture_source, fixture_claims = local_source(
        cas,
        FIXTURE,
        fixture_ranges,
        "percent-of input fixture",
        discarded_reason="newline record separators outside the accepted fact bytes",
    )
    query_source, query_claims = local_source(
        cas,
        QUERY,
        query_ranges,
        "percent-of.query.adj input",
        discarded_reason=(
            "introductory comment, spacing, or human-readable test oracle outside "
            "the selected import, facts, and executable question"
        ),
        reasoned_discards=[
            (
                disabled_start,
                len(query_bytes),
                (
                    "disabled edge-case example deliberately excluded from the "
                    "executable worked query"
                ),
            )
        ],
    )
    fixture_locator = f"repo://{FIXTURE}"
    query_clauses = []
    for name, _value in facts:
        claim_id = f"adj.input.arithmetic.percent_of.{name}"
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
                        "deterministic percent-of query input retained as the explicit "
                        "accepted fact"
                    ),
                },
                "snapshot_sha256": fixture_source["raw_source_sha256"],
                "source_ir_sha256": fixture_source["source_ir_sha256"],
            }
        )
    query_bundle = {
        "bundle_id": "adj.math.arithmetic.percent_of.query.v1",
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
        label="percent-of.query.adj execution witness",
    )
    query_bundle["formula_derivation_sha256s"] = derivations
    query_bundle["execution_witness_sha256s"] = witnesses
    query_bundle_hash = cas.put_json(
        query_bundle,
        kind="provenance_bundle",
        label="percent-of.query.adj provenance bundle",
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
        help="reviewed OpenStax HTML bytes for the one-time CAS bootstrap",
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
