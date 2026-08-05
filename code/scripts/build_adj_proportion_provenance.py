#!/usr/bin/env python3
"""Materialize the reviewed proportion stdlib provenance from retained bytes."""

from __future__ import annotations

import argparse
from collections.abc import Sequence
from pathlib import Path

import adj_provenance_builder as builder
import adj_stdlib_provenance as provenance

REPO_ROOT = Path(__file__).resolve().parents[2]
LIBRARY = "code/specs/data/adj-formula-stdlib/arithmetic/proportion.adj"
FIXTURE = "code/specs/data/adj-stdlib-provenance/fixtures/proportion-inputs.txt"
LOCATOR = (
    "https://openstax.org/books/prealgebra-2e/pages/"
    "6-5-solve-proportions-and-their-applications"
)
SOURCE_RETRIEVED_AT = "2026-08-03T19:40:12Z"
INPUT_CAPTURED_AT = "2026-08-03T20:00:00Z"
RAW_HASH = "a35bcb922594fce55973d557eed3f4ea33160b3be1e6c6c80dca59ac80586f25"
RAW_SIZE = 536_280
RECEIPT_HASH = "5750280b55111800452e1060727df630c009789a2c325380e2ae0e60c1581137"
DEFINITION_RENDERED_HASH = (
    "b8e6728b95578281c1f646cbf57c563f64b435a8836ed0ebe15c03a13a48dc44"
)
RENDERED_HASH = "6460a9d15d0fdf0c4b8022a55924257838aede6fbb470adb38af8e4ab836a851"
PROPORTION_CLAIM = "adj.math.arithmetic.proportion"
DEFINITION_CLAIM = "adj.math.arithmetic.proportion.definition"
CROSS_PRODUCTS_CLAIM = "adj.math.arithmetic.proportion.cross_products"
QUESTION_CLAIM = "adj.question.arithmetic.proportion.compute"
DEFINITION_TEXT = (
    "A proportion is an equation of the form (a/b)=(c/d), where b\u22600,d\u22600."
).encode("utf-8")
SELECTED_TEXT = (
    "For any proportion of the form (a/b)=(c/d), where b\u22600,d\u22600, its "
    "cross products are equal."
).encode("utf-8")
CLAIM_SPECS = (
    (
        DEFINITION_CLAIM,
        b'id="fs-id1166492018126"',
        119_622,
        120_450,
        "6e8ba327b8243a7551b915592b7a5bbe2762519f459c935c27754f2ee87d97eb",
    ),
    (
        PROPORTION_CLAIM,
        b'id="fs-id1517408"',
        140_181,
        141_024,
        "83d559c98b29544cd5140145b609e1dbceae346f0737b5b48a4d4f1452bbc9ff",
    ),
)
QUERY_SPECS = (
    (
        "adj.math.arithmetic.proportion.query.v1",
        "code/specs/data/adj-formula-stdlib/arithmetic/proportion.query.adj",
        (("first_term", "2"), ("second_term", "3"), ("third_term", "4")),
    ),
    (
        "adj.math.arithmetic.proportion.zero_first.query.v1",
        "code/specs/data/adj-formula-stdlib/arithmetic/proportion-zero-first.query.adj",
        (("first_term", "0"), ("second_term", "3"), ("third_term", "4")),
    ),
    (
        "adj.math.arithmetic.proportion.zero_second.query.v1",
        "code/specs/data/adj-formula-stdlib/arithmetic/proportion-zero-second.query.adj",
        (("first_term", "2"), ("second_term", "0"), ("third_term", "4")),
    ),
    (
        "adj.math.arithmetic.proportion.zero_third.query.v1",
        "code/specs/data/adj-formula-stdlib/arithmetic/proportion-zero-third.query.adj",
        (("first_term", "2"), ("second_term", "3"), ("third_term", "0")),
    ),
)


def local_source(
    cas: provenance.Cas,
    repo_path: str,
    ranges: list[tuple[str, int, int]],
    label: str,
    *,
    discarded_reason: str,
    data: bytes | None = None,
    reasoned_discards: list[tuple[int, int, str]] | None = None,
) -> tuple[dict, dict[str, dict]]:
    if data is None:
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


def _projection_operations(captured: bytes, spec: tuple) -> list[dict]:
    operations = []
    result_cursor = 0
    claim_id = spec[0]

    def append(
        operation: str, start: int, end: int, *, reason: str | None = None
    ) -> None:
        nonlocal result_cursor
        source = captured[start:end]
        if operation == "discard":
            rendered = b""
        elif operation == "mathml_to_infix":
            rendered = provenance._mathml_to_infix(source, "proportion transform")
        else:
            rendered = source
        item = {
            "operation": operation,
            "result_end": result_cursor + len(rendered),
            "result_start": result_cursor,
            "source_end": end,
            "source_start": start,
        }
        if operation == "discard":
            item.update({"claim_id": claim_id, "reason": reason})
        operations.append(item)
        result_cursor += len(rendered)

    _claim_id, _marker, start, end, _digest = spec
    markup_reason = (
        "publisher HTML wrapper markup is structural and contributes no bytes "
        "to the rendered proportion statement"
    )
    opening_end = captured.index(b">", start) + 1
    append("discard", start, opening_end, reason=markup_reason)
    first_span = captured.index(b"<span", opening_end, end)
    append("copy", opening_end, first_span)
    first_math_start = captured.index(b"<math", first_span, end)
    append("discard", first_span, first_math_start, reason=markup_reason)
    first_math_end = captured.index(b"</math>", first_math_start, end) + 7
    append("mathml_to_infix", first_math_start, first_math_end)
    first_span_end = captured.index(b"</span>", first_math_end, end) + 7
    append("discard", first_math_end, first_span_end, reason=markup_reason)
    second_span = captured.index(b"<span", first_span_end, end)
    append("copy", first_span_end, second_span)
    second_math_start = captured.index(b"<math", second_span, end)
    append("discard", second_span, second_math_start, reason=markup_reason)
    second_math_end = captured.index(b"</math>", second_math_start, end) + 7
    append("mathml_to_infix", second_math_start, second_math_end)
    second_span_end = captured.index(b"</span>", second_math_end, end) + 7
    append("discard", second_math_end, second_span_end, reason=markup_reason)
    closing_start = captured.index(b"</p>", second_span_end, end)
    if second_span_end < closing_start:
        append("copy", second_span_end, closing_start)
    append("discard", closing_start, end, reason=markup_reason)
    return operations


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
    raw_hash = cas.put(captured, kind="raw_source", label="OpenStax proportion source")
    if raw_hash != RAW_HASH or len(captured) != RAW_SIZE:
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
            "content-length": "536280",
            "content-type": "text/html",
            "etag": '"c940edde2f4478001fa42e29db51cea2"',
            "last-modified": "Mon, 15 Jun 2026 17:25:34 GMT",
        },
    )
    receipt_hash = cas.put_json(
        receipt,
        kind="fetch_receipt",
        label="OpenStax proportion fetch receipt",
        links=[raw_hash],
    )
    if receipt_hash != RECEIPT_HASH:
        raise provenance.ProvenanceError("OpenStax receipt does not match review")

    raw_claims = {}
    represented = []
    for claim_id, marker, start, end, expected_hash in CLAIM_SPECS:
        cited = captured[start:end]
        if (
            not cited.startswith(b"<p " + marker)
            or provenance.sha256_bytes(cited) != expected_hash
        ):
            raise provenance.ProvenanceError(
                f"reviewed OpenStax {claim_id} byte span drifted"
            )
        item = builder.claim(claim_id, captured, start, end)
        raw_claims[claim_id] = item
        represented.append((start, end, [item]))
    raw_ir = provenance.build_source_ir(
        source_sha256=raw_hash,
        source=captured,
        segments=builder.source_segments(
            captured,
            represented,
            discarded_reason=(
                "publisher markup and textbook material outside the reviewed "
                "proportion definition and cross-products paragraphs"
            ),
        ),
    )
    raw_ir_hash = cas.put_json(
        raw_ir,
        kind="source_ir",
        label="OpenStax proportion raw IR",
        links=[raw_hash],
    )

    representations = []
    rendered_claims = {}
    for spec, rendered, expected_hash, label in (
        (CLAIM_SPECS[0], DEFINITION_TEXT, DEFINITION_RENDERED_HASH, "definition"),
        (CLAIM_SPECS[1], SELECTED_TEXT, RENDERED_HASH, "cross-products rule"),
    ):
        claim_id = spec[0]
        rendered_hash = cas.put(
            rendered,
            kind="rendered_text",
            label=f"OpenStax proportion {label}",
            links=[raw_hash],
        )
        if rendered_hash != expected_hash:
            raise provenance.ProvenanceError(
                f"rendered OpenStax proportion {label} hash drifted"
            )
        rendered_claim = builder.claim(claim_id, rendered, 0, len(rendered))
        rendered_claims[claim_id] = rendered_claim
        rendered_ir = provenance.build_source_ir(
            source_sha256=rendered_hash,
            source=rendered,
            segments=[
                {
                    "claims": [rendered_claim],
                    "disposition": "represented",
                    "end": len(rendered),
                    "start": 0,
                }
            ],
        )
        rendered_ir_hash = cas.put_json(
            rendered_ir,
            kind="source_ir",
            label=f"OpenStax proportion {label} rendered IR",
            links=[rendered_hash],
        )
        transform = provenance.build_text_transform(
            source_sha256=raw_hash,
            source=captured,
            result_sha256=rendered_hash,
            result=rendered,
            operations=_projection_operations(captured, spec),
        )
        transform_hash = cas.put_json(
            transform,
            kind="text_transform",
            label=f"OpenStax proportion {label} text transform",
            links=[raw_hash, rendered_hash],
        )
        representations.append(
            {
                "rendered_text_sha256": rendered_hash,
                "source_ir_sha256": rendered_ir_hash,
                "transform_sha256": transform_hash,
            }
        )
    return (
        {
            "raw_source_sha256": raw_hash,
            "receipt_sha256": receipt_hash,
            "representations": representations,
            "source_ir_sha256": raw_ir_hash,
        },
        rendered_claims,
    )


def _fixture_ranges(fixture_bytes: bytes) -> list[tuple[str, int, int]]:
    ranges = []
    for name, value in sorted(
        {fact for _bundle, _path, facts in QUERY_SPECS for fact in facts}
    ):
        claim_id = f"adj.input.arithmetic.proportion.{name}.{value}"
        sentence = f"{name.replace('_', ' ')} is {value}.".encode()
        start = fixture_bytes.index(sentence)
        ranges.append((claim_id, start, start + len(sentence)))
    return ranges


def _query_bundle(
    cas: provenance.Cas,
    *,
    bundle_id: str,
    query_path: str,
    facts: tuple[tuple[str, str], ...],
    library_hash: str,
    fixture_source: dict,
    fixture_claims: dict[str, dict],
    formula_audit_command: Sequence[str],
    query_bytes: bytes | None = None,
) -> tuple[str, str]:
    if query_bytes is None:
        query_bytes = provenance._read_regular_file(REPO_ROOT / query_path)
    query_ranges = []
    for name, value in facts:
        claim_id = f"adj.input.arithmetic.proportion.{name}.{value}"
        start = query_bytes.index(f"observe {name}({value})".encode())
        trust = query_bytes.index(b"    trust authoritative", start)
        end = query_bytes.index(b"\n", trust) + 1
        query_ranges.append((claim_id, start, end))
    import_start = query_bytes.index(b'import "proportion.adj"')
    import_end = query_bytes.index(b"\n", import_start) + 1
    query_ranges.append(
        ("adj.code.arithmetic.proportion.query.import", import_start, import_end)
    )
    question_start = query_bytes.index(b"? fourth_proportional(")
    question_end = query_bytes.index(b"\n", question_start) + 1
    query_ranges.append((QUESTION_CLAIM, question_start, question_end))
    binding_cursor = 0
    binding_index = 0
    while True:
        binding_start = query_bytes.find(b"let ", binding_cursor)
        if binding_start < 0:
            break
        binding_end = query_bytes.index(b"\n", binding_start) + 1
        query_ranges.append(
            (f"{bundle_id}.binding.{binding_index}", binding_start, binding_end)
        )
        binding_cursor = binding_end
        binding_index += 1
    query_source, query_claims = local_source(
        cas,
        query_path,
        query_ranges,
        f"{Path(query_path).name} input",
        discarded_reason=(
            "comments, spacing, or human-readable explanation outside the selected "
            "import, observations, and executable question"
        ),
        data=query_bytes,
    )
    fixture_locator = f"repo://{FIXTURE}"
    clauses = []
    for name, value in facts:
        claim_id = f"adj.input.arithmetic.proportion.{name}.{value}"
        clauses.append(
            {
                **fixture_claims[claim_id],
                "input_claim": builder.input_claim_payload(query_claims[claim_id]),
                "locator": fixture_locator,
                "resolution": {
                    "authority_receipt_sha256": fixture_source["receipt_sha256"],
                    "authority_source_sha256": fixture_source["raw_source_sha256"],
                    "classification": "accepted_fact",
                    "kind": "accepted_root",
                    "reason": (
                        "deterministic proportion query input retained as the "
                        "explicit accepted fact"
                    ),
                },
                "snapshot_sha256": fixture_source["raw_source_sha256"],
                "source_ir_sha256": fixture_source["source_ir_sha256"],
            }
        )
    bundle = {
        "bundle_id": bundle_id,
        "clauses": clauses,
        "dependencies": [library_hash],
        "input": {
            key: query_source[key]
            for key in ("raw_source_sha256", "receipt_sha256", "source_ir_sha256")
        },
        "kind": "provenance_bundle",
        "library": query_path,
        "sources": [query_source, fixture_source],
    }
    derivations, witnesses = provenance.put_formula_execution_evidence(
        cas,
        bundle,
        formula_audit_command,
        label=f"{Path(query_path).name} v2 execution witness",
    )
    bundle["formula_derivation_sha256s"] = derivations
    bundle["execution_witness_sha256s"] = witnesses
    bundle_hash = cas.put_json(
        bundle,
        kind="provenance_bundle",
        label=f"{Path(query_path).name} provenance bundle",
        links=provenance._bundle_declared_links(bundle),
    )
    return bundle_id, bundle_hash


def build(
    cas: provenance.Cas,
    captured_source: Path | None,
    *,
    arithmetic_bundle_sha256: str,
    formula_inventory_command: Sequence[str],
    formula_audit_command: Sequence[str],
    workspace_input_snapshots: dict[str, bytes] | None = None,
) -> dict[str, str]:
    if not formula_audit_command:
        raise provenance.ProvenanceError(
            "proportion build requires formula audit command"
        )
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
    vocabulary_start = library_bytes.index(b"dictionary proportion_vocab {")
    vocabulary_end = library_bytes.index(b"}\n", vocabulary_start) + 2
    use_start = library_bytes.index(b"    use proportion_vocab")
    use_end = library_bytes.index(b"\n", use_start) + 1
    formula_start = library_bytes.index(b"    formula fourth_proportional(")
    trust_start = library_bytes.index(b"        trust inferred", formula_start)
    formula_end = library_bytes.index(b"\n", trust_start) + 1
    input_source, input_claims = local_source(
        cas,
        LIBRARY,
        [
            (
                "adj.code.arithmetic.proportion.import.arithmetic",
                import_start,
                import_end,
            ),
            (
                "adj.code.arithmetic.proportion.vocabulary",
                vocabulary_start,
                vocabulary_end,
            ),
            ("adj.code.arithmetic.proportion.use.proportion_vocab", use_start, use_end),
            (PROPORTION_CLAIM, formula_start, formula_end),
        ],
        "proportion.adj input",
        discarded_reason=(
            "explanatory comments, separators, or closing syntax outside the "
            "selected import, vocabulary, use, and formula rules"
        ),
    )
    formula_inventory_hash = provenance.put_formula_parser_inventory(
        cas,
        input_source["raw_source_sha256"],
        formula_inventory_command,
        label="proportion.adj parser inventory",
    )
    bundle = {
        "bundle_id": "adj.math.arithmetic.proportion.v1",
        "clauses": [
            {
                **external_claims[PROPORTION_CLAIM],
                "input_claim": builder.input_claim_payload(
                    input_claims[PROPORTION_CLAIM]
                ),
                "locator": LOCATOR,
                "resolution": {
                    "authority_receipt_sha256": external_source["receipt_sha256"],
                    "authority_source_sha256": external_source["raw_source_sha256"],
                    "classification": "primary_definition",
                    "kind": "accepted_root",
                    "reason": (
                        "OpenStax proportion-domain and cross-products rules are "
                        "retained as the explicit definitional roots"
                    ),
                },
                "snapshot_sha256": RENDERED_HASH,
                "source_ir_sha256": external_source["representations"][1][
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
        label="proportion.adj provenance bundle",
        links=provenance._bundle_declared_links(bundle),
    )

    fixture_bytes = provenance._read_regular_file(REPO_ROOT / FIXTURE)
    fixture_source, fixture_claims = local_source(
        cas,
        FIXTURE,
        _fixture_ranges(fixture_bytes),
        "proportion input fixture",
        discarded_reason="newline record separators outside the accepted fact bytes",
    )
    roots = {bundle["bundle_id"]: bundle_hash}
    for bundle_id, query_path, facts in QUERY_SPECS:
        root_id, root_hash = _query_bundle(
            cas,
            bundle_id=bundle_id,
            query_path=query_path,
            facts=facts,
            library_hash=bundle_hash,
            fixture_source=fixture_source,
            fixture_claims=fixture_claims,
            formula_audit_command=formula_audit_command,
            query_bytes=(workspace_input_snapshots or {}).get(query_path),
        )
        roots[root_id] = root_hash
    return roots


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
