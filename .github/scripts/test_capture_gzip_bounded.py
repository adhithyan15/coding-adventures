"""CLI tests for bounded, lossless gzip capture and verification."""

from __future__ import annotations

import gzip
import hashlib
import json
import random
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
CAPTURE = HERE / "capture-gzip-bounded.py"
VERIFY = HERE / "verify-gzip-bounded.py"
MARKERS = b"".join(
    f"=== outer strace invocation pid={1000 + index} ===\n".encode()
    for index in range(5)
)
TRACE_CALLS = b"".join(
    b"12.000000 ptrace(PTRACE_SEIZE, 1201, NULL, 0x100000) = 0\n"
    for _ in range(3)
)


class BoundedGzipCliTests(unittest.TestCase):
    def run_capture(self, root: Path, payload: bytes, raw_cap: int, compressed_cap: int):
        trace = root / "trace.gz"
        metadata = root / "capture.json"
        marker = root / "incomplete.json"
        result = subprocess.run(
            [
                sys.executable,
                str(CAPTURE),
                "--max-raw-bytes",
                str(raw_cap),
                "--max-compressed-bytes",
                str(compressed_cap),
                "--output",
                str(trace),
                "--metadata",
                str(metadata),
                "--truncated-marker",
                str(marker),
            ],
            input=payload,
            capture_output=True,
            timeout=20,
            check=False,
        )
        return result, trace, metadata, marker

    def run_verify(self, root: Path, trace: Path, metadata: Path, raw_cap: int):
        report = root / "verified.json"
        result = subprocess.run(
            [
                sys.executable,
                str(VERIFY),
                "--trace",
                str(trace),
                "--capture-metadata",
                str(metadata),
                "--output",
                str(report),
                "--max-raw-bytes",
                str(raw_cap),
                "--expected-runs",
                "5",
            ],
            capture_output=True,
            timeout=20,
            check=False,
        )
        return result, report

    def test_chunk_boundaries_roundtrip_and_gzip_footer(self):
        payload = MARKERS + TRACE_CALLS + ((b"x" * 2000 + b"\n") * 40)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            captured, trace, metadata_path, marker = self.run_capture(
                root, payload, len(payload), 256 * 1024
            )
            self.assertEqual(captured.returncode, 0, captured.stderr.decode(errors="replace"))
            self.assertFalse(marker.exists())
            metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
            self.assertTrue(metadata["gzip_finalized"])
            self.assertEqual(metadata["raw_bytes_observed"], len(payload))
            self.assertEqual(metadata["raw_bytes_fed_to_compressor"], len(payload))
            self.assertEqual(gzip.decompress(trace.read_bytes()), payload)
            with tempfile.TemporaryDirectory() as second_directory:
                second_root = Path(second_directory)
                second, second_trace, _, _ = self.run_capture(
                    second_root, payload, len(payload), 256 * 1024
                )
                self.assertEqual(second.returncode, 0, second.stderr.decode(errors="replace"))
                self.assertEqual(trace.read_bytes(), second_trace.read_bytes())
            verified, report_path = self.run_verify(root, trace, metadata_path, len(payload))
            self.assertEqual(verified.returncode, 0, verified.stderr.decode(errors="replace"))
            report = json.loads(report_path.read_text(encoding="utf-8"))
            self.assertTrue(report["valid"])
            self.assertEqual(report["outer_invocation_markers"], 5)
            self.assertEqual(report["ptrace_calls"], 3)

    def test_raw_cap_drains_and_does_not_finalize_gzip(self):
        payload = MARKERS + (b"raw-cap-data\n" * 2000)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            captured, trace, metadata_path, marker = self.run_capture(root, payload, 128, 4096)
            self.assertEqual(captured.returncode, 1)
            metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
            self.assertEqual(metadata["raw_bytes_observed"], len(payload))
            self.assertTrue(metadata["raw_cap_reached"])
            self.assertFalse(metadata["gzip_finalized"])
            self.assertTrue(marker.exists())
            self.assertLessEqual(trace.stat().st_size, 4096)
            verified, _ = self.run_verify(root, trace, metadata_path, 128)
            self.assertEqual(verified.returncode, 1)

    def test_compressed_cap_drains_and_never_claims_valid_gzip(self):
        payload = MARKERS + random.Random(20261126).randbytes(512 * 1024)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            captured, trace, metadata_path, marker = self.run_capture(root, payload, len(payload), 4096)
            self.assertEqual(captured.returncode, 1)
            metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
            self.assertEqual(metadata["raw_bytes_observed"], len(payload))
            self.assertTrue(metadata["compressed_cap_reached"])
            self.assertFalse(metadata["gzip_finalized"])
            self.assertTrue(marker.exists())
            self.assertLessEqual(trace.stat().st_size, 4096)
            verified, _ = self.run_verify(root, trace, metadata_path, len(payload))
            self.assertEqual(verified.returncode, 1)

    def test_verifier_rejects_truncated_or_corrupt_member(self):
        payload = MARKERS + TRACE_CALLS
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            captured, trace, metadata_path, _ = self.run_capture(root, payload, len(payload) + 1, 4096)
            self.assertEqual(captured.returncode, 0, captured.stderr.decode(errors="replace"))
            damaged = trace.read_bytes()[:-4]
            trace.write_bytes(damaged)
            metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
            metadata["compressed_bytes_stored"] = len(damaged)
            metadata["compressed_sha256"] = hashlib.sha256(damaged).hexdigest()
            metadata_path.write_text(json.dumps(metadata), encoding="utf-8")
            verified, _ = self.run_verify(root, trace, metadata_path, len(payload) + 1)
            self.assertEqual(verified.returncode, 1)

    def test_verifier_enforces_decompression_ceiling(self):
        payload = MARKERS + (b"expanded\n" * 10000)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            captured, trace, metadata_path, _ = self.run_capture(root, payload, len(payload) + 1, 65536)
            self.assertEqual(captured.returncode, 0, captured.stderr.decode(errors="replace"))
            verified, report_path = self.run_verify(root, trace, metadata_path, 256)
            self.assertEqual(verified.returncode, 1)
            report = json.loads(report_path.read_text(encoding="utf-8"))
            self.assertFalse(report["valid"])
            self.assertTrue(any("ceiling" in error for error in report["errors"]))


if __name__ == "__main__":
    unittest.main(verbosity=2)
