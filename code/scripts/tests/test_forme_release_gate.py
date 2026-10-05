from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "forme_release_gate.py"
SPEC = importlib.util.spec_from_file_location("forme_release_gate", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
gate = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(gate)


class FormeReleaseGateTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)

    def test_host_attestation_binds_review_and_release_evidence(self) -> None:
        benchmark = self.root / "benchmark.json"
        quality = self.root / "quality.json"
        benchmark.write_text(json.dumps({
            "schemaVersion": 1,
            "pageCount": 1000,
            "editedPage": 500,
            "clean": {
                "elapsedMs": 1,
                "buildId": "blake2b:" + "a" * 64,
                "parsedPages": 1000,
            },
            "incremental": {
                "elapsedMs": 1,
                "buildId": "blake2b:" + "b" * 64,
                "reusedParsedPages": 999,
                "parsedPages": 1,
            },
        }), encoding="utf-8")
        quality.write_text(json.dumps(self._quality()), encoding="utf-8")
        attestation = gate.create_attestation(
            host="linux",
            commit="a" * 40,
            benchmark_path=benchmark,
            quality_path=quality,
        )
        self.assertEqual(attestation["verdict"], "pass")
        self.assertEqual(attestation["checks"], list(gate.REQUIRED_CHECKS))
        self.assertEqual(attestation["securityReviewPolicy"], gate.SECURITY_REVIEW_POLICY)
        self.assertEqual(
            attestation["evidence"]["benchmarkSha256"],
            hashlib.sha256(benchmark.read_bytes()).hexdigest(),
        )

    def test_release_verdict_requires_exact_supported_host_set(self) -> None:
        evidence = [self._attestation(host) for host in gate.SUPPORTED_HOSTS]
        verdict = gate.aggregate_attestations(evidence)
        self.assertEqual(verdict["verdict"], "pass")
        self.assertEqual(verdict["hosts"], list(gate.SUPPORTED_HOSTS))

        with self.assertRaisesRegex(ValueError, "supported host set"):
            gate.aggregate_attestations(evidence[:-1])
        with self.assertRaisesRegex(ValueError, "supported host set"):
            gate.aggregate_attestations([*evidence, evidence[0]])

    def test_release_verdict_rejects_mixed_commit_or_security_policy(self) -> None:
        evidence = [self._attestation(host) for host in gate.SUPPORTED_HOSTS]
        evidence[1] = {**evidence[1], "commit": "b" * 40}
        with self.assertRaisesRegex(ValueError, "one commit"):
            gate.aggregate_attestations(evidence)

        evidence = [self._attestation(host) for host in gate.SUPPORTED_HOSTS]
        evidence[1] = {
            **evidence[1],
            "securityReviewPolicy": "self-attested",
        }
        with self.assertRaisesRegex(ValueError, "security-review policy"):
            gate.aggregate_attestations(evidence)

    def test_web_quality_rejects_incomplete_or_over_budget_evidence(self) -> None:
        quality = self._quality()
        quality["targets"][0]["lighthouse"] = {}
        with self.assertRaisesRegex(ValueError, "unexpected fields"):
            gate._validate_quality(quality)

        quality = self._quality()
        quality["targets"][2]["lighthouse"]["resources"]["script"] = 32 * 1024 + 1
        with self.assertRaisesRegex(ValueError, "budget"):
            gate._validate_quality(quality)

    def test_evidence_reader_hashes_validated_bytes_and_rejects_hardlinks(self) -> None:
        evidence = self.root / "evidence.json"
        raw = b'{"schemaVersion":1}\n'
        evidence.write_bytes(raw)
        value, digest = gate._read_json_and_digest(evidence, "evidence")
        self.assertEqual(value, {"schemaVersion": 1})
        self.assertEqual(digest, hashlib.sha256(raw).hexdigest())

        alias = self.root / "alias.json"
        os.link(evidence, alias)
        with self.assertRaisesRegex(ValueError, "single-link"):
            gate._read_json_and_digest(evidence, "evidence")

    @staticmethod
    def _quality() -> dict[str, object]:
        targets = []
        for target_id, contract in gate.QUALITY_TARGETS.items():
            resources = {key: 0 for key in gate.RESOURCE_KEYS}
            resources["document"] = 1
            resources["total"] = 1
            targets.append({
                "id": target_id,
                "route": contract["route"],
                "fallback": {
                    "textCharacters": contract["text"],
                    "links": contract["links"],
                    "listItems": contract["listItems"],
                    "scripts": contract["scripts"],
                    "diagnostics": [],
                },
                "lighthouse": {
                    "performance": contract["performance"],
                    "accessibility": 1,
                    "resources": resources,
                    "diagnostics": [],
                },
                "diagnostics": [],
            })
        return {
            "schemaVersion": 1,
            "lighthouseVersion": "13.5.0",
            "chromeVersion": "154.0.8037.92",
            "targets": targets,
        }

    @staticmethod
    def _attestation(host: str) -> dict[str, object]:
        return {
            "schemaVersion": 1,
            "contract": "FM-B072",
            "host": host,
            "commit": "a" * 40,
            "verdict": "pass",
            "checks": list(gate.REQUIRED_CHECKS),
            "securityReviewPolicy": gate.SECURITY_REVIEW_POLICY,
            "evidence": {
                "benchmarkSha256": "d" * 64,
                "webQualitySha256": "e" * 64,
            },
        }


if __name__ == "__main__":
    unittest.main()
