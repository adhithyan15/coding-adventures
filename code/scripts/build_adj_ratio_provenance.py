#!/usr/bin/env python3
"""Materialize the reviewed ratio stdlib provenance from retained bytes."""

from __future__ import annotations

import argparse
from collections.abc import Sequence
from pathlib import Path

import adj_provenance_builder as builder
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


def local_source(
    cas: provenance.Cas,
    repo_path: str,
    ranges: list[tuple[str, int, int]],
    label: str,
    *,
    discarded_reason: str,
    data: bytes | None = None,
    on_disk: bool = True,
    reasoned_discards: list[tuple[int, int, str]] | None = None,
) -> tuple[dict, dict[str, dict]]:
    # Accepting already-read bytes is what lets the caller pin offsets and quotes
    # to ONE read of the file. Re-reading here would leave the two a swap apart.
    if data is None:
        data = provenance._read_regular_file(REPO_ROOT / repo_path)
    elif on_disk:
        # The supplied buffer must be THIS file's bytes. Nothing else checks it:
        # the receipt, the IR and every quote are all derived from `data`, so a
        # mispaired variable would hash one file while claiming another and stay
        # perfectly self-consistent. A re-read here is a CHECK, never a second
        # source of quotes. Callers passing a snapshot that may legitimately
        # differ from disk opt out with `on_disk=False`.
        actual = provenance._read_regular_file(REPO_ROOT / repo_path)
        if actual != data:
            raise provenance.ProvenanceError(
                f"{repo_path}: supplied bytes do not match the file on disk "
                f"({provenance.sha256_bytes(data)} vs {provenance.sha256_bytes(actual)})"
            )
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
            item = builder.claim(claim_id, data, start, end)
            claims[claim_id] = item
            range_claims.append(item)
        represented.append((start, end, range_claims))
    ir = provenance.build_source_ir(
        source_sha256=raw_hash,
        source=data,
        segments=builder.source_segments(
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

    raw_claim = builder.claim(RATIO_CLAIM, captured, selected_start, selected_end)
    raw_ir = provenance.build_source_ir(
        source_sha256=raw_hash,
        source=captured,
        segments=builder.source_segments(
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
    rendered_claim = builder.claim(RATIO_CLAIM, SELECTED_TEXT, 0, len(SELECTED_TEXT))
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
        data=library_bytes,
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
                "input_claim": builder.input_claim_payload(input_claims[RATIO_CLAIM]),
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
    facts = (("numerator", "3"), ("denominator", "4"))
    fixture_ranges = []
    for name, value in facts:
        claim_id = f"adj.input.arithmetic.ratio.{name}"
        sentence = f"{name} is {value}.".encode()
        fixture_start = fixture_bytes.index(sentence)
        fixture_ranges.append((claim_id, fixture_start, fixture_start + len(sentence)))
    fixture_source, fixture_claims = local_source(
        cas,
        FIXTURE,
        fixture_ranges,
        "ratio input fixture",
        discarded_reason="newline record separators outside the accepted fact bytes",
        data=fixture_bytes,
    )
    query_bundle_id, query_bundle_hash = builder.build_query_bundle(
        cas,
        spec=builder.QueryLibrarySpec(
            bundle_id="adj.math.arithmetic.ratio.query.v1",
            query_path=QUERY,
            fixture_path=FIXTURE,
            claim_prefix="adj.input.arithmetic.ratio",
            import_literal=b'import "ratio.adj"',
            import_claim_id="adj.code.arithmetic.ratio.query.import",
            question_prefix=b"? ratio(",
            question_claim_id=QUESTION_CLAIM,
            accepted_fact_reason=(
                "deterministic ratio-query input retained as the explicit accepted fact"
            ),
            discarded_reason=(
                "introductory comment, spacing, or human-readable test oracle outside "
                "the selected import, facts, and executable question"
            ),
            input_description="ratio.query.adj input",
            witness_label="ratio.query.adj execution witness",
        ),
        repo_root=REPO_ROOT,
        facts=facts,
        library_hash=bundle_hash,
        fixture_source=fixture_source,
        fixture_claims=fixture_claims,
        formula_audit_command=formula_audit_command,
        local_source=local_source,
    )
    return {
        bundle["bundle_id"]: bundle_hash,
        query_bundle_id: query_bundle_hash,
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
