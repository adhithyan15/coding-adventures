#!/usr/bin/env python3
"""Materialize the reviewed arithmetic.adj provenance bundle from retained bytes."""

from __future__ import annotations

import argparse
from collections.abc import Sequence
from pathlib import Path

import adj_stdlib_provenance as provenance

REPO_ROOT = Path(__file__).resolve().parents[2]
LIBRARY = "code/specs/data/adj-formula-stdlib/arithmetic/arithmetic.adj"
CAPTURED_AT = "2026-08-02T21:45:54Z"
SOURCES = (
    {
        "id": "adj.math.arithmetic.sum",
        "name": "sum",
        "locator": "https://mathworld.wolfram.com/Sum.html",
        "raw": "fd9f9b6ebae2f4be91595aed7f9e0e6efa6ac534edf6f0f21f0a1f40e187588e",
        "receipt": "2cde961be3ff71fd329ab06295ef45267c85efbc8e1e370f1e97b9ceaa6dbcfe",
        "start": 284,
        "end": 319,
        "rendered": "A sum is the result of an addition.",
        "operation": "copy",
    },
    {
        "id": "adj.math.arithmetic.difference",
        "name": "difference",
        "locator": "https://mathworld.wolfram.com/Difference.html",
        "raw": "d0bb4594d21393cd5134bda3072635f25e4016fa3116178d724870737ac5895f",
        "receipt": "1deaf136094490011152ec17932535e042623985c501dd1a0f0a7d9c5e8f6564",
        "start": 259,
        "end": 354,
        "rendered": "The difference of two numbers n_1 and n_2 is n_1-n_2, where the minus sign denotes subtraction.",
        "operation": "copy",
    },
    {
        "id": "adj.math.arithmetic.product",
        "name": "product",
        "locator": "https://mathworld.wolfram.com/Product.html",
        "raw": "63e118f5118b72c600775183b2f0645ef23cb1e1e39e9891037f3f652e8261ac",
        "receipt": "46f3cd40fcf5770885aa3e1fa2d59acd7f983c438db11d96eebe2eef63ebc312",
        "start": 283,
        "end": 364,
        "rendered": 'The term "product" refers to the result of one or more multiplications.',
        "operation": "html_entity_decode",
    },
    {
        "id": "adj.math.arithmetic.quotient",
        "name": "quotient",
        "locator": "https://mathworld.wolfram.com/Quotient.html",
        "raw": "0be79e8dfa46675a74374e37ba59bda163388617c3d728324ce8c7bb3d2f6f86",
        "receipt": "6cf0944c082b5e1e9f103348a7b657fe560f6f74878e370e133c00bdf22aa19e",
        "start": 299,
        "end": 417,
        "rendered": 'The term "quotient" is most commonly used to refer to the ratio q=r/s of two quantities r and s, where s!=0.',
        "operation": "html_entity_decode",
    },
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


def partition(data: bytes, start: int, end: int, item_claim: dict) -> list[dict]:
    return [
        {
            "disposition": "discarded",
            "end": start,
            "reason": "publisher markup and context before the selected definition",
            "start": 0,
        },
        {
            "claims": [item_claim],
            "disposition": "represented",
            "end": end,
            "start": start,
        },
        {
            "disposition": "discarded",
            "end": len(data),
            "reason": "publisher markup, references, and context after the selected definition",
            "start": end,
        },
    ]


def local_source(
    cas: provenance.Cas, repo_path: str, ranges: list[tuple[str, int, int]], label: str
) -> tuple[dict, dict[str, dict]]:
    data = provenance._read_regular_file(REPO_ROOT / repo_path)
    raw_hash = cas.put(data, kind="raw_source", label=label)
    receipt = provenance.build_input_receipt(
        repo_path=repo_path,
        captured_at=CAPTURED_AT,
        body_sha256=raw_hash,
        body_size=len(data),
        body_git_sha1=provenance.git_blob_sha1(data),
    )
    receipt_hash = cas.put_json(
        receipt, kind="input_receipt", label=f"{label} receipt", links=[raw_hash]
    )
    segments = []
    claims = {}
    cursor = 0
    for claim_id, start, end in sorted(ranges, key=lambda item: item[1]):
        if cursor < start:
            segments.append(
                {
                    "disposition": "discarded",
                    "end": start,
                    "reason": "spacing or syntax outside the selected input claim",
                    "start": cursor,
                }
            )
        item_claim = claim(claim_id, data, start, end)
        claims[claim_id] = item_claim
        segments.append(
            {
                "claims": [item_claim],
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
                "reason": "spacing or syntax outside the selected input claim",
                "start": cursor,
            }
        )
    ir = provenance.build_source_ir(
        source_sha256=raw_hash, source=data, segments=segments
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


def build(
    cas: provenance.Cas,
    *,
    formula_inventory_command: Sequence[str],
    formula_audit_command: Sequence[str],
) -> dict[str, str]:
    source_entries = []
    clauses = []
    for item in SOURCES:
        raw = provenance._read_regular_file(cas.object_path(item["raw"]))
        description_marker = b'<meta name="DC.Description" content="'
        if raw.count(description_marker) != 1:
            raise provenance.ProvenanceError(
                f"MathWorld {item['name']} must have one DC.Description metadata root"
            )
        if raw.index(description_marker) + len(description_marker) != item["start"]:
            raise provenance.ProvenanceError(
                f"MathWorld {item['name']} cited span is not the DC.Description value"
            )
        raw_claim = claim(item["id"], raw, item["start"], item["end"])
        raw_ir = provenance.build_source_ir(
            source_sha256=item["raw"],
            source=raw,
            segments=partition(raw, item["start"], item["end"], raw_claim),
        )
        raw_ir_hash = cas.put_json(
            raw_ir,
            kind="source_ir",
            label=f"MathWorld {item['name']} raw IR",
            links=[item["raw"]],
        )
        rendered = item["rendered"].encode("utf-8")
        rendered_hash = cas.put(
            rendered,
            kind="rendered_text",
            label=f"MathWorld {item['name']} definition",
            links=[item["raw"]],
        )
        rendered_claim = claim(item["id"], rendered, 0, len(rendered))
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
            label=f"MathWorld {item['name']} rendered IR",
            links=[rendered_hash],
        )
        transform = provenance.build_text_transform(
            source_sha256=item["raw"],
            source=raw,
            result_sha256=rendered_hash,
            result=rendered,
            operations=[
                {
                    "operation": item["operation"],
                    "result_end": len(rendered),
                    "result_start": 0,
                    "source_end": item["end"],
                    "source_start": item["start"],
                }
            ],
        )
        transform_hash = cas.put_json(
            transform,
            kind="text_transform",
            label=f"MathWorld {item['name']} text transform",
            links=[item["raw"], rendered_hash],
        )
        source_entries.append(
            {
                "raw_source_sha256": item["raw"],
                "receipt_sha256": item["receipt"],
                "representations": [
                    {
                        "rendered_text_sha256": rendered_hash,
                        "source_ir_sha256": rendered_ir_hash,
                        "transform_sha256": transform_hash,
                    }
                ],
                "source_ir_sha256": raw_ir_hash,
            }
        )
        item.update(
            {
                "rendered_hash": rendered_hash,
                "rendered_ir_hash": rendered_ir_hash,
                "rendered_claim": rendered_claim,
            }
        )

    input_bytes = provenance._read_regular_file(REPO_ROOT / LIBRARY)
    input_hash = cas.put(input_bytes, kind="raw_source", label="arithmetic.adj input")
    input_receipt = provenance.build_input_receipt(
        repo_path=LIBRARY,
        captured_at=CAPTURED_AT,
        body_sha256=input_hash,
        body_size=len(input_bytes),
        body_git_sha1=provenance.git_blob_sha1(input_bytes),
    )
    input_receipt_hash = cas.put_json(
        input_receipt,
        kind="input_receipt",
        label="arithmetic.adj input receipt",
        links=[input_hash],
    )
    input_segments = []
    cursor = 0
    input_claims = {}
    for item in SOURCES:
        start = input_bytes.index(f"    formula {item['name']}(".encode())
        trust = input_bytes.index(b"        trust authoritative", start)
        end = input_bytes.index(b"\n", trust) + 1
        if cursor < start:
            input_segments.append(
                {
                    "disposition": "discarded",
                    "end": start,
                    "reason": "comments, declarations, imports, or spacing outside this formula clause",
                    "start": cursor,
                }
            )
        item_claim = claim(item["id"], input_bytes, start, end)
        input_claims[item["id"]] = item_claim
        input_segments.append(
            {
                "claims": [item_claim],
                "disposition": "represented",
                "end": end,
                "start": start,
            }
        )
        cursor = end
    if cursor < len(input_bytes):
        input_segments.append(
            {
                "disposition": "discarded",
                "end": len(input_bytes),
                "reason": "closing syntax and trailing bytes outside formula clauses",
                "start": cursor,
            }
        )
    input_ir = provenance.build_source_ir(
        source_sha256=input_hash, source=input_bytes, segments=input_segments
    )
    input_ir_hash = cas.put_json(
        input_ir, kind="source_ir", label="arithmetic.adj input IR", links=[input_hash]
    )
    input_source = {
        "raw_source_sha256": input_hash,
        "receipt_sha256": input_receipt_hash,
        "representations": [],
        "source_ir_sha256": input_ir_hash,
    }
    formula_inventory_hash = provenance.put_formula_parser_inventory(
        cas,
        input_hash,
        formula_inventory_command,
        label="arithmetic.adj parser inventory",
    )
    for item in SOURCES:
        external = item["rendered_claim"]
        code = input_claims[item["id"]]
        clauses.append(
            {
                "claim_id": item["id"],
                "end": external["end"],
                "input_claim": {
                    key: code[key] for key in ("end", "quote", "quote_sha256", "start")
                },
                "locator": item["locator"],
                "quote": external["quote"],
                "quote_sha256": external["quote_sha256"],
                "resolution": {
                    "authority_receipt_sha256": item["receipt"],
                    "authority_source_sha256": item["raw"],
                    "classification": "primary_definition",
                    "kind": "accepted_root",
                    "reason": "MathWorld definition retained as the explicit definitional root",
                },
                "snapshot_sha256": item["rendered_hash"],
                "source_ir_sha256": item["rendered_ir_hash"],
                "start": 0,
            }
        )
    bundle = {
        "bundle_id": "adj.math.arithmetic.primitives.v1",
        "clauses": clauses,
        "dependencies": [],
        "formula_inventory_sha256": formula_inventory_hash,
        "input": {
            "raw_source_sha256": input_hash,
            "receipt_sha256": input_receipt_hash,
            "source_ir_sha256": input_ir_hash,
        },
        "kind": "provenance_bundle",
        "library": LIBRARY,
        "sources": [input_source, *source_entries],
    }
    bundle_hash = cas.put_json(
        bundle,
        kind="provenance_bundle",
        label="arithmetic.adj provenance bundle",
        links=provenance._bundle_declared_links(bundle),
    )

    fixture_path = (
        "code/specs/data/adj-stdlib-provenance/fixtures/arithmetic-inputs.txt"
    )
    query_path = "code/specs/data/adj-formula-stdlib/arithmetic/arithmetic.query.adj"
    fixture_bytes = provenance._read_regular_file(REPO_ROOT / fixture_path)
    query_bytes = provenance._read_regular_file(REPO_ROOT / query_path)
    inputs = (
        ("addend_one", "7"),
        ("addend_two", "5"),
        ("minuend", "7"),
        ("subtrahend", "5"),
        ("factor_one", "6"),
        ("factor_two", "7"),
        ("dividend", "20"),
        ("divisor", "5"),
    )
    fixture_ranges = []
    query_ranges = []
    for name, value in inputs:
        claim_id = f"adj.input.arithmetic.{name}"
        sentence = f"{name} is {value}.".encode()
        fixture_start = fixture_bytes.index(sentence)
        fixture_ranges.append((claim_id, fixture_start, fixture_start + len(sentence)))
        query_start = query_bytes.index(f"observe {name}(".encode())
        trust = query_bytes.index(b"    trust authoritative", query_start)
        query_end = query_bytes.index(b"\n", trust) + 1
        query_ranges.append((claim_id, query_start, query_end))
    import_start = query_bytes.index(b'import "arithmetic.adj"')
    import_end = query_bytes.index(b"\n", import_start) + 1
    query_ranges.append(("adj.import.arithmetic.primitives", import_start, import_end))
    for name in ("sum", "difference", "product", "quotient"):
        question_start = query_bytes.index(f"? {name}(".encode())
        question_end = query_bytes.index(b"\n", question_start) + 1
        query_ranges.append(
            (f"adj.question.arithmetic.{name}", question_start, question_end)
        )
    fixture_source, fixture_claims = local_source(
        cas, fixture_path, fixture_ranges, "arithmetic input fixture"
    )
    query_source, query_claims = local_source(
        cas, query_path, query_ranges, "arithmetic.query.adj input"
    )
    query_clauses = []
    fixture_locator = f"repo://{fixture_path}"
    for name, _value in inputs:
        claim_id = f"adj.input.arithmetic.{name}"
        external = fixture_claims[claim_id]
        code = query_claims[claim_id]
        query_clauses.append(
            {
                "claim_id": claim_id,
                "end": external["end"],
                "input_claim": {
                    key: code[key] for key in ("end", "quote", "quote_sha256", "start")
                },
                "locator": fixture_locator,
                "quote": external["quote"],
                "quote_sha256": external["quote_sha256"],
                "resolution": {
                    "authority_receipt_sha256": fixture_source["receipt_sha256"],
                    "authority_source_sha256": fixture_source["raw_source_sha256"],
                    "classification": "accepted_fact",
                    "kind": "accepted_root",
                    "reason": "deterministic worked-query input retained as the explicit accepted fact",
                },
                "snapshot_sha256": fixture_source["raw_source_sha256"],
                "source_ir_sha256": fixture_source["source_ir_sha256"],
                "start": external["start"],
            }
        )
    query_bundle = {
        "bundle_id": "adj.math.arithmetic.primitives.query.v1",
        "clauses": query_clauses,
        "dependencies": [bundle_hash],
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
        query_bundle,
        formula_audit_command,
        label="arithmetic.query.adj execution witness",
    )
    query_bundle["formula_derivation_sha256s"] = derivations
    query_bundle["execution_witness_sha256s"] = witnesses
    query_bundle_hash = cas.put_json(
        query_bundle,
        kind="provenance_bundle",
        label="arithmetic.query.adj provenance bundle",
        links=provenance._bundle_declared_links(query_bundle),
    )
    return {
        bundle["bundle_id"]: bundle_hash,
        query_bundle["bundle_id"]: query_bundle_hash,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
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
                formula_inventory_command=formula_inventory_command,
                formula_audit_command=formula_audit_command,
            )
        )


if __name__ == "__main__":
    main()
