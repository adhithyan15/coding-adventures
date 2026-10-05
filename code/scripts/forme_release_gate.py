#!/usr/bin/env python3
"""Fail-closed evidence composer for the FM-B072 Forme release gate."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import stat
import subprocess
import sys
from pathlib import Path
from typing import Any, Mapping, Sequence


CONTRACT = "FM-B072"
SUPPORTED_HOSTS = ("linux", "macos", "windows")
SECURITY_REVIEW_POLICY = "mandatory-exact-head-external"
REQUIRED_CHECKS = (
    "package-api-v2",
    "landing-page-product",
    "blog-product",
    "release-benchmark",
    "web-quality",
    "desktop-authoring-product",
)
PRODUCT_BUILD_ROOTS = (
    "code/sites/landing-page",
    "code/sites/blog",
    "code/programs/typescript/forme-shell-desktop",
)
MAX_JSON_BYTES = 1024 * 1024
HEX_40 = re.compile(r"^[0-9a-f]{40}$")
HEX_64 = re.compile(r"^[0-9a-f]{64}$")
BUILD_ID = re.compile(r"^blake2b:[0-9a-f]{64}$")
QUALITY_TARGETS = {
    "landing": {
        "route": "/coding-adventures/",
        "performance": 0.95,
        "text": 1000,
        "links": 12,
        "listItems": 8,
        "scripts": 0,
        "total": 640 * 1024,
        "image": 512 * 1024,
        "script": 0,
    },
    "blog-index": {
        "route": "/coding-adventures/blog/",
        "performance": 0.95,
        "text": 160,
        "links": 4,
        "listItems": 3,
        "scripts": 0,
        "total": 160 * 1024,
        "script": 0,
    },
    "hello-forme": {
        "route": "/coding-adventures/blog/2026-05-15-hello-forme.html",
        "performance": 0.95,
        "text": 500,
        "links": 2,
        "listItems": 13,
        "scripts": 1,
        "total": 256 * 1024,
        "image": 64 * 1024,
        "script": 32 * 1024,
    },
}
RESOURCE_KEYS = {
    "document", "font", "image", "media", "other", "script",
    "stylesheet", "third-party", "total",
}


def create_attestation(
    *,
    host: str,
    commit: str,
    benchmark_path: Path,
    quality_path: Path,
) -> dict[str, Any]:
    if host not in SUPPORTED_HOSTS:
        raise ValueError(f"unsupported release host: {host}")
    if HEX_40.fullmatch(commit) is None:
        raise ValueError("release commit must be a lowercase 40-character Git object id")
    benchmark, benchmark_digest = _read_json_and_digest(benchmark_path, "release benchmark")
    quality, quality_digest = _read_json_and_digest(quality_path, "web quality")
    _validate_benchmark(benchmark)
    _validate_quality(quality)
    return {
        "schemaVersion": 1,
        "contract": CONTRACT,
        "host": host,
        "commit": commit,
        "verdict": "pass",
        "checks": list(REQUIRED_CHECKS),
        "securityReviewPolicy": SECURITY_REVIEW_POLICY,
        "evidence": {
            "benchmarkSha256": benchmark_digest,
            "webQualitySha256": quality_digest,
        },
    }


def aggregate_attestations(attestations: Sequence[Mapping[str, Any]]) -> dict[str, Any]:
    for attestation in attestations:
        _validate_attestation(attestation)
    hosts = [attestation["host"] for attestation in attestations]
    if len(hosts) != len(SUPPORTED_HOSTS) or set(hosts) != set(SUPPORTED_HOSTS):
        raise ValueError("release evidence does not contain the exact supported host set")
    commits = {attestation["commit"] for attestation in attestations}
    if len(commits) != 1:
        raise ValueError("release evidence does not describe one commit")
    return {
        "schemaVersion": 1,
        "contract": CONTRACT,
        "commit": commits.pop(),
        "verdict": "pass",
        "hosts": list(SUPPORTED_HOSTS),
        "checks": list(REQUIRED_CHECKS),
        "securityReviewPolicy": SECURITY_REVIEW_POLICY,
    }


def run_product_builds(repo_root: Path, host: str) -> None:
    if host not in SUPPORTED_HOSTS:
        raise ValueError(f"unsupported release host: {host}")
    build_name = "BUILD_windows" if host == "windows" else "BUILD"
    bootstrap = repo_root / "code/packages/typescript/forme-cli/bin/bootstrap.mjs"
    for relative in PRODUCT_BUILD_ROOTS:
        package_root = repo_root / relative
        subprocess.run(
            ["node", str(bootstrap), str(package_root), "--frozen"],
            cwd=repo_root,
            check=True,
        )
        build_path = package_root / build_name
        lines = build_path.read_text(encoding="utf-8").splitlines()
        commands = [line for line in lines if line.strip() and not line.lstrip().startswith("#")]
        for command in commands:
            if command.strip() == "set -e":
                continue
            subprocess.run(command, cwd=package_root, shell=True, check=True)


def _validate_benchmark(value: Any) -> None:
    root = _record(value, "release benchmark")
    _exact_keys(root, {"schemaVersion", "pageCount", "editedPage", "clean", "incremental"}, "release benchmark")
    if root.get("schemaVersion") != 1 or root.get("pageCount") != 1000 or root.get("editedPage") != 500:
        raise ValueError("release benchmark identity is invalid")
    clean = _record(root.get("clean"), "release benchmark clean evidence")
    incremental = _record(root.get("incremental"), "release benchmark incremental evidence")
    _exact_keys(clean, {"elapsedMs", "buildId", "parsedPages"}, "release benchmark clean evidence")
    _exact_keys(
        incremental,
        {"elapsedMs", "buildId", "reusedParsedPages", "parsedPages"},
        "release benchmark incremental evidence",
    )
    if (
        not _finite_number(clean["elapsedMs"])
        or clean["elapsedMs"] < 0
        or not _finite_number(incremental["elapsedMs"])
        or incremental["elapsedMs"] < 0
        or BUILD_ID.fullmatch(str(clean["buildId"])) is None
        or BUILD_ID.fullmatch(str(incremental["buildId"])) is None
        or clean["buildId"] == incremental["buildId"]
    ):
        raise ValueError("release benchmark build evidence is invalid")
    if clean.get("parsedPages") != 1000:
        raise ValueError("release benchmark clean work is incomplete")
    if incremental.get("reusedParsedPages") != 999 or incremental.get("parsedPages") != 1:
        raise ValueError("release benchmark incremental work is incomplete")


def _validate_quality(value: Any) -> None:
    root = _record(value, "web quality")
    if (
        root.get("schemaVersion") != 1
        or root.get("lighthouseVersion") != "13.5.0"
        or root.get("chromeVersion") != "154.0.8037.92"
    ):
        raise ValueError("web quality tool identity is invalid")
    targets = root.get("targets")
    if not isinstance(targets, list) or len(targets) != 3:
        raise ValueError("web quality target set is invalid")
    ids: list[str] = []
    for target_value in targets:
        target = _record(target_value, "web quality target")
        _exact_keys(target, {"id", "route", "fallback", "lighthouse", "diagnostics"}, "web quality target")
        target_id = target.get("id")
        if not isinstance(target_id, str) or target_id not in QUALITY_TARGETS:
            raise ValueError("web quality target identity is invalid")
        ids.append(target_id)
        contract = QUALITY_TARGETS[target_id]
        if target.get("route") != contract["route"] or target.get("diagnostics") != []:
            raise ValueError("web quality target did not pass")
        fallback = _record(target.get("fallback"), "web quality fallback")
        lighthouse = _record(target.get("lighthouse"), "web quality Lighthouse result")
        _exact_keys(fallback, {"textCharacters", "links", "listItems", "scripts", "diagnostics"}, "web quality fallback")
        _exact_keys(lighthouse, {"performance", "accessibility", "resources", "diagnostics"}, "web quality Lighthouse result")
        if fallback["diagnostics"] != [] or lighthouse["diagnostics"] != []:
            raise ValueError("web quality target retained nested diagnostics")
        for key, contract_key in (
            ("textCharacters", "text"),
            ("links", "links"),
            ("listItems", "listItems"),
        ):
            if not _nonnegative_int(fallback[key]) or fallback[key] < contract[contract_key]:
                raise ValueError("web quality fallback evidence is below its budget")
        if not _nonnegative_int(fallback["scripts"]) or fallback["scripts"] > contract["scripts"]:
            raise ValueError("web quality fallback script count exceeds its budget")
        performance = lighthouse["performance"]
        accessibility = lighthouse["accessibility"]
        if not _finite_number(performance) or performance < contract["performance"] or performance > 1:
            raise ValueError("web quality performance score is invalid")
        if not _finite_number(accessibility) or accessibility != 1:
            raise ValueError("web quality accessibility score is invalid")
        resources = _record(lighthouse["resources"], "web quality resources")
        _exact_keys(resources, RESOURCE_KEYS, "web quality resources")
        if any(not _nonnegative_int(amount) for amount in resources.values()):
            raise ValueError("web quality resource evidence is invalid")
        for resource in ("total", "image", "script"):
            maximum = contract.get(resource)
            if maximum is not None and resources[resource] > maximum:
                raise ValueError("web quality resource budget was exceeded")
    if ids != ["landing", "blog-index", "hello-forme"]:
        raise ValueError("web quality target identities are invalid")


def _nonnegative_int(value: Any) -> bool:
    return isinstance(value, int) and not isinstance(value, bool) and value >= 0


def _finite_number(value: Any) -> bool:
    return (
        isinstance(value, (int, float))
        and not isinstance(value, bool)
        and value == value
        and value not in (float("inf"), float("-inf"))
    )


def _validate_attestation(value: Mapping[str, Any]) -> None:
    _exact_keys(
        value,
        {"schemaVersion", "contract", "host", "commit", "verdict", "checks", "securityReviewPolicy", "evidence"},
        "host attestation",
    )
    if value["schemaVersion"] != 1 or value["contract"] != CONTRACT or value["verdict"] != "pass":
        raise ValueError("host attestation did not pass the current contract")
    if value["host"] not in SUPPORTED_HOSTS or HEX_40.fullmatch(str(value["commit"])) is None:
        raise ValueError("host attestation identity is invalid")
    if value["checks"] != list(REQUIRED_CHECKS):
        raise ValueError("host attestation check set is invalid")
    if value["securityReviewPolicy"] != SECURITY_REVIEW_POLICY:
        raise ValueError("host attestation security-review policy is invalid")
    evidence = _record(value["evidence"], "host evidence")
    _exact_keys(evidence, {"benchmarkSha256", "webQualitySha256"}, "host evidence")
    if any(HEX_64.fullmatch(str(evidence[key])) is None for key in evidence):
        raise ValueError("host release evidence digest is invalid")


def _read_json(path: Path, label: str) -> Any:
    return _read_json_and_digest(path, label)[0]


def _read_json_and_digest(path: Path, label: str) -> tuple[Any, str]:
    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0)
    descriptor = os.open(path, flags)
    try:
        before = os.fstat(descriptor)
        path_info = path.lstat()
        if (
            not stat.S_ISREG(before.st_mode)
            or not stat.S_ISREG(path_info.st_mode)
            or before.st_nlink != 1
            or path_info.st_nlink != 1
            or before.st_dev != path_info.st_dev
            or before.st_ino != path_info.st_ino
            or before.st_size > MAX_JSON_BYTES
        ):
            raise ValueError(f"{label} must be a bounded regular single-link file")
        chunks: list[bytes] = []
        remaining = MAX_JSON_BYTES + 1
        while remaining > 0:
            chunk = os.read(descriptor, min(64 * 1024, remaining))
            if not chunk:
                break
            chunks.append(chunk)
            remaining -= len(chunk)
        raw = b"".join(chunks)
        after = os.fstat(descriptor)
        final_path_info = path.lstat()
        identity = (before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns)
        if (
            len(raw) > MAX_JSON_BYTES
            or identity != (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns)
            or (after.st_dev, after.st_ino) != (final_path_info.st_dev, final_path_info.st_ino)
            or not stat.S_ISREG(final_path_info.st_mode)
            or final_path_info.st_nlink != 1
        ):
            raise ValueError(f"{label} changed while it was being read")
        return json.loads(raw.decode("utf-8")), hashlib.sha256(raw).hexdigest()
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ValueError(f"{label} is not valid UTF-8 JSON") from error
    finally:
        os.close(descriptor)


def _record(value: Any, label: str) -> Mapping[str, Any]:
    if not isinstance(value, dict) or any(not isinstance(key, str) for key in value):
        raise ValueError(f"{label} must be an object")
    return value


def _exact_keys(value: Mapping[str, Any], expected: set[str], label: str) -> None:
    if set(value) != expected:
        raise ValueError(f"{label} has unexpected fields")


def _write_json(path: Path, value: Mapping[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parents[2])
    subparsers = parser.add_subparsers(dest="command", required=True)
    build = subparsers.add_parser("build-products")
    build.add_argument("--host", required=True, choices=SUPPORTED_HOSTS)
    attest = subparsers.add_parser("attest")
    attest.add_argument("--host", required=True, choices=SUPPORTED_HOSTS)
    attest.add_argument("--commit", required=True)
    attest.add_argument("--benchmark", required=True, type=Path)
    attest.add_argument("--web-quality", required=True, type=Path)
    attest.add_argument("--output", required=True, type=Path)
    verdict = subparsers.add_parser("verdict")
    verdict.add_argument("--evidence-dir", required=True, type=Path)
    verdict.add_argument("--output", required=True, type=Path)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    repo_root = args.repo_root.resolve(strict=True)
    if args.command == "build-products":
        run_product_builds(repo_root, args.host)
    elif args.command == "attest":
        _write_json(args.output, create_attestation(
            host=args.host,
            commit=args.commit,
            benchmark_path=args.benchmark,
            quality_path=args.web_quality,
        ))
    elif args.command == "verdict":
        paths = sorted(args.evidence_dir.glob("**/forme-release-*.json"))
        attestations = [_record(_read_json(path, "host attestation"), "host attestation") for path in paths]
        _write_json(args.output, aggregate_attestations(attestations))
    else:
        raise AssertionError(f"unhandled command: {args.command}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, subprocess.CalledProcessError, ValueError) as error:
        print(f"forme release gate: {error}", file=sys.stderr)
        raise SystemExit(1)
