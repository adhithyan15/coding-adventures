#!/usr/bin/env python3
"""Replay parser and execution evidence for the complete arithmetic closure."""

from __future__ import annotations

import argparse
import json
from collections.abc import Sequence
from pathlib import Path

import adj_stdlib_provenance as provenance
import build_adj_arithmetic_provenance as arithmetic_builder
import build_adj_percent_of_provenance as percent_of_builder
import build_adj_ratio_provenance as ratio_builder

REPO_ROOT = Path(__file__).resolve().parents[2]
ROOT_IDS = {
    "adj.math.arithmetic.percent_of.query.v1",
    "adj.math.arithmetic.percent_of.v1",
    "adj.math.arithmetic.primitives.query.v1",
    "adj.math.arithmetic.primitives.v1",
    "adj.math.arithmetic.ratio.query.v1",
    "adj.math.arithmetic.ratio.v1",
}


def _registered_roots(cas: provenance.Cas, manifest_path: Path) -> dict[str, str]:
    manifest = json.loads(provenance._read_regular_file(manifest_path).decode("utf-8"))
    roots: dict[str, str] = {}
    for digest in manifest["bundle_hashes"]:
        bundle = provenance._json_object(cas, digest, "provenance_bundle")
        bundle_id = bundle["bundle_id"]
        if bundle_id in roots:
            raise provenance.ProvenanceError(
                f"manifest registers bundle_id {bundle_id} more than once"
            )
        roots[bundle_id] = digest
    return roots


def migrate(
    cas_root: Path,
    manifest_path: Path,
    schema_path: Path,
    workspace_root: Path,
    *,
    formula_inventory_command: Sequence[str],
    formula_audit_command: Sequence[str],
) -> dict[str, object]:
    with provenance.BundleRootReplacementTransaction(
        cas_root,
        manifest_path,
        expected_manifest_id="adj.stdlib.provenance.v1",
        schema_path=schema_path,
        workspace_root=workspace_root,
        formula_inventory_command=formula_inventory_command,
        formula_audit_command=formula_audit_command,
        allow_unwitnessed_baseline=True,
    ) as transaction:
        old_roots = _registered_roots(transaction.cas, manifest_path)
        missing = sorted(ROOT_IDS - set(old_roots))
        if missing:
            raise provenance.ProvenanceError(
                "formula inventory migration is missing roots: " + ", ".join(missing)
            )

        new_roots = arithmetic_builder.build(
            transaction.cas,
            formula_inventory_command=formula_inventory_command,
            formula_audit_command=formula_audit_command,
        )
        arithmetic_hash = new_roots["adj.math.arithmetic.primitives.v1"]
        new_roots.update(
            ratio_builder.build(
                transaction.cas,
                None,
                arithmetic_bundle_sha256=arithmetic_hash,
                formula_inventory_command=formula_inventory_command,
                formula_audit_command=formula_audit_command,
            )
        )
        new_roots.update(
            percent_of_builder.build(
                transaction.cas,
                None,
                arithmetic_bundle_sha256=arithmetic_hash,
                formula_inventory_command=formula_inventory_command,
                formula_audit_command=formula_audit_command,
            )
        )
        if set(new_roots) != ROOT_IDS:
            raise provenance.ProvenanceError(
                "formula inventory migration rebuilt an unexpected root set"
            )

        replacements = {
            bundle_id: {
                "expected_old_sha256": old_roots[bundle_id],
                "new_sha256": new_roots[bundle_id],
            }
            for bundle_id in sorted(ROOT_IDS)
        }
        result = transaction.replace_roots(replacements)
    return {
        **result,
        "replacements": replacements,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--formula-inventory-binary", type=Path, required=True)
    parser.add_argument("--formula-audit-binary", type=Path, required=True)
    args = parser.parse_args()
    result = migrate(
        REPO_ROOT / provenance.DEFAULT_ROOT,
        REPO_ROOT / provenance.DEFAULT_MANIFEST,
        REPO_ROOT / provenance.DEFAULT_SCHEMA,
        REPO_ROOT,
        formula_inventory_command=[str(args.formula_inventory_binary.resolve())],
        formula_audit_command=[str(args.formula_audit_binary.resolve())],
    )
    print(provenance.canonical_json_bytes(result).decode("utf-8"), end="")


if __name__ == "__main__":
    main()
