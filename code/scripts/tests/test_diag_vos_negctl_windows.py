"""Focused tests for the VOS Windows timeout diagnostic helpers."""

from __future__ import annotations

import os
import sys
import threading
import unittest
from pathlib import Path
from unittest import mock

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

import diag_vos_negctl_windows as diagnostic

ROOT = SCRIPTS.parents[1]
WRAPPER = SCRIPTS / "warrant_negctl.py"


class StreamCaptureTest(unittest.TestCase):
    def test_bailout_in_omitted_middle_is_still_detected(self) -> None:
        read_fd, write_fd = os.pipe()
        capture = diagnostic.StreamCapture()
        started = diagnostic.time.perf_counter()
        read_stream = os.fdopen(read_fd, "rb")
        reader = threading.Thread(target=capture.read, args=(read_stream, started))
        reader.start()
        payload = b"A" * 40_000 + b"Bail out! hidden in omitted bytes\n" + b"Z" * 40_000
        with os.fdopen(write_fd, "wb") as writer:
            writer.write(payload)
        reader.join(timeout=5)
        read_stream.close()
        self.assertFalse(reader.is_alive())
        self.assertIsNotNone(capture.first_bail_out_s)
        self.assertNotIn("Bail out!", capture.output())


class RealWrapperProbeTest(unittest.TestCase):
    def _probe(self, code: str) -> dict:
        env = os.environ.copy()
        env["WARRANT_NEGCTL_TIMEOUT_SECS"] = "5"
        argv = [sys.executable, str(WRAPPER), "--", sys.executable, "-c", code]
        with mock.patch.object(diagnostic, "process_snapshot", return_value={"processes": []}), mock.patch.object(
            diagnostic, "kill_tree", return_value="taskkill exit=0"
        ):
            return diagnostic.run_probe(
                "actual-wrapper-fixture",
                argv,
                ROOT,
                env,
                expected_exit_code=0,
                wrapper_judge_expected=True,
            )

    def test_actual_wrapper_accepts_complete_failing_tap_and_exits_zero(self) -> None:
        code = "import sys; print('TAP version 14'); print('1..1'); print('not ok 1 - fixture'); sys.exit(1)"
        result = self._probe(code)
        self.assertEqual(result["exit_code"], 0, result["stdout"]["captured"])
        self.assertTrue(result["valid_fault_observation"], result)
        self.assertIn("negctl: fault caught", result["stdout"]["captured"])
        self.assertIsNone(result["stdout"]["first_tap_plan_s"])

    def test_wrapper_rejection_does_not_count_as_a_completed_fault_observation(self) -> None:
        code = "import sys; print('1..1'); print('not ok 1 - fixture'); sys.exit(0)"
        result = self._probe(code)
        self.assertEqual(result["exit_code"], 1, result["stdout"]["captured"])
        self.assertFalse(result["valid_fault_observation"])


class ConcurrentOverlapProbeTest(unittest.TestCase):
    def _run_pair(self, target_delay: float, sibling_delay: float) -> dict:
        target_code = (
            "import sys,time; "
            f"time.sleep({target_delay}); "
            "print('TAP version 14', flush=True); "
            "print('1..1', flush=True); "
            "print('not ok 1 - fixture', flush=True); "
            "time.sleep(0.35); sys.exit(1)"
        )
        sibling_code = f"import time; time.sleep({sibling_delay})"
        with mock.patch.object(diagnostic, "process_snapshot", return_value={"processes": []}), mock.patch.object(
            diagnostic, "kill_tree", return_value="taskkill exit=0"
        ):
            return diagnostic.run_concurrent_probe(
                "local-observation-fixture",
                [sys.executable, "-c", target_code],
                [sys.executable, "-c", sibling_code],
                ROOT,
                os.environ.copy(),
                os.environ.copy(),
                "shared-default-target-fixture",
                "direct-process-stream",
                False,
            )

    def test_reports_sustained_observed_overlap_through_direct_tap(self) -> None:
        result = self._run_pair(target_delay=0.3, sibling_delay=1.8)
        self.assertEqual(result["interpretation"], "not-reproduced-under-one-sibling", result)
        self.assertGreater(result["provable_overlap_s"], 0)
        self.assertTrue(result["sibling_covers_target_first_tap"])
        self.assertTrue(result["sibling_covers_target_first_not_ok"])
        self.assertTrue(result["sibling_covers_target_last_byte"])
        self.assertTrue(result["target"]["valid_negative_control"])
        self.assertTrue(result["sibling"]["valid_control"])

    def test_fast_sibling_without_proven_overlap_is_inconclusive(self) -> None:
        result = self._run_pair(target_delay=0.45, sibling_delay=0.05)
        self.assertEqual(result["interpretation"], "inconclusive-no-proven-overlap", result)
        self.assertEqual(result["provable_overlap_s"], 0)

    def test_direct_failing_process_requires_expected_exit_and_tap_stream(self) -> None:
        env = os.environ.copy()
        code = "import sys; print('TAP version 14'); print('1..1'); print('not ok 1 - fixture'); sys.exit(1)"
        with mock.patch.object(diagnostic, "process_snapshot", return_value={"processes": []}), mock.patch.object(
            diagnostic, "kill_tree", return_value="taskkill exit=0"
        ):
            result = diagnostic.run_probe(
                "direct-failing-process-fixture",
                [sys.executable, "-c", code],
                ROOT,
                env,
                expected_exit_code=1,
            )
        self.assertTrue(result["valid_fault_observation"], result)

    def test_surviving_process_tree_is_not_a_valid_observation(self) -> None:
        env = os.environ.copy()
        code = "import sys; print('TAP version 14'); print('1..1'); print('not ok 1 - fixture'); sys.exit(1)"
        row = {"pid": 999999, "parent_pid": 123, "name": "fixture-child.exe"}
        with mock.patch.object(
            diagnostic,
            "process_snapshot",
            side_effect=lambda _pid: {"processes": [row]},
        ), mock.patch.object(diagnostic, "kill_tree", return_value="taskkill exit=1"):
            result = diagnostic.run_probe(
                "surviving-child-fixture",
                [sys.executable, "-c", code],
                ROOT,
                env,
                expected_exit_code=1,
            )
        self.assertFalse(result["cleanup_verified"])
        self.assertFalse(result["valid_fault_observation"])


if __name__ == "__main__":
    unittest.main()
