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
import build_adj_proportion_provenance as proportion_builder
import build_adj_ratio_provenance as ratio_builder

REPO_ROOT = Path(__file__).resolve().parents[2]
ROOT_IDS = {
    "adj.math.arithmetic.percent_of.query.v1",
    "adj.math.arithmetic.percent_of.v1",
    "adj.math.arithmetic.primitives.query.v1",
    "adj.math.arithmetic.primitives.v1",
    "adj.math.arithmetic.proportion.query.v1",
    "adj.math.arithmetic.proportion.v1",
    "adj.math.arithmetic.proportion.zero_first.query.v1",
    "adj.math.arithmetic.proportion.zero_second.query.v1",
    "adj.math.arithmetic.proportion.zero_third.query.v1",
    "adj.math.arithmetic.ratio.query.v1",
    "adj.math.arithmetic.ratio.v1",
}
PROPORTION_ROOT_IDS = {
    "adj.math.arithmetic.proportion.query.v1",
    "adj.math.arithmetic.proportion.v1",
    "adj.math.arithmetic.proportion.zero_first.query.v1",
    "adj.math.arithmetic.proportion.zero_second.query.v1",
    "adj.math.arithmetic.proportion.zero_third.query.v1",
}
PROPORTION_POSITIVE_QUERY = (
    "code/specs/data/adj-formula-stdlib/arithmetic/proportion.query.adj"
)
PROPORTION_POSITIVE_QUERY_SHA256 = (
    "444a92a243093539b8dad35a4219f958a8280847bafd3cea48facc04840e979d"
)


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
    captured_proportion_source: Path | None = None,
) -> dict[str, object]:
    baseline_cas = provenance.Cas(cas_root)
    baseline_cas.load()
    baseline_roots = _registered_roots(baseline_cas, manifest_path)
    proportion_query_bytes = provenance._read_regular_file(
        workspace_root / PROPORTION_POSITIVE_QUERY
    )
    source_hash = provenance.sha256_bytes(proportion_query_bytes)
    if source_hash != PROPORTION_POSITIVE_QUERY_SHA256:
        raise provenance.ProvenanceError(
            "reviewed proportion query bytes changed before migration"
        )
    workspace_input_snapshots = {
        PROPORTION_POSITIVE_QUERY: proportion_query_bytes,
    }
    planned_workspace_input_hashes = {}
    if "adj.math.arithmetic.proportion.query.v1" in baseline_roots:
        planned_workspace_input_hashes[PROPORTION_POSITIVE_QUERY] = (
            PROPORTION_POSITIVE_QUERY_SHA256
        )
    with provenance.BundleRootReplacementTransaction(
        cas_root,
        manifest_path,
        expected_manifest_id="adj.stdlib.provenance.v1",
        schema_path=schema_path,
        workspace_root=workspace_root,
        formula_inventory_command=formula_inventory_command,
        formula_audit_command=formula_audit_command,
        allow_unwitnessed_baseline=True,
        allow_migration_formula_inputs_baseline=True,
        allow_migration_derived_bindings_baseline=True,
        planned_workspace_input_hashes=planned_workspace_input_hashes,
        workspace_input_snapshots=workspace_input_snapshots,
    ) as transaction:
        old_roots = _registered_roots(transaction.cas, manifest_path)
        missing = sorted(ROOT_IDS - set(old_roots))
        unexpected_missing = sorted(set(missing) - PROPORTION_ROOT_IDS)
        if unexpected_missing:
            raise provenance.ProvenanceError(
                "formula inventory migration is missing legacy roots: "
                + ", ".join(unexpected_missing)
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
        new_roots.update(
            proportion_builder.build(
                transaction.cas,
                captured_proportion_source,
                arithmetic_bundle_sha256=arithmetic_hash,
                formula_inventory_command=formula_inventory_command,
                formula_audit_command=formula_audit_command,
                workspace_input_snapshots=workspace_input_snapshots,
            )
        )
        if set(new_roots) != ROOT_IDS:
            raise provenance.ProvenanceError(
                "formula inventory migration rebuilt an unexpected root set"
            )

        replacements = {
            bundle_id: {
                "expected_old_sha256": old_roots.get(bundle_id),
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
    parser.add_argument(
        "--captured-proportion-source",
        type=Path,
        help="reviewed OpenStax HTML bytes for the one-time proportion bootstrap",
    )
    args = parser.parse_args()
    result = migrate(
        REPO_ROOT / provenance.DEFAULT_ROOT,
        REPO_ROOT / provenance.DEFAULT_MANIFEST,
        REPO_ROOT / provenance.DEFAULT_SCHEMA,
        REPO_ROOT,
        formula_inventory_command=[str(args.formula_inventory_binary.resolve())],
        formula_audit_command=[str(args.formula_audit_binary.resolve())],
        captured_proportion_source=args.captured_proportion_source,
    )
    print(provenance.canonical_json_bytes(result).decode("utf-8"), end="")


if __name__ == "__main__":
    main()
