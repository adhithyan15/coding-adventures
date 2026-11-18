import importlib.util
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("classify_g1_gate_result.py")
SPEC = importlib.util.spec_from_file_location("classify_g1_gate_result", MODULE_PATH)
classifier = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(classifier)


def gate_log(terminal="PASS", probe_rate="20.0", failure=None, census=True, slice_marker=True):
    lines = ["=== RUN   TestSandboxTraceGateSynthetic"]
    if census:
        lines.append("    sandbox_tracesize_test.go:169: PROBE: trace synthetic-4mib-census bytes=4195800 lines=58968 best_mb_s=18.7 gated=0 repeats=7")
    if slice_marker:
        lines.append(f"    sandbox_tracesize_test.go:179: PROBE: trace synthetic-slice bytes=66600 lines=936 best_mb_s={probe_rate} gated=1 repeats=7")
    if failure:
        lines.append(f"    sandbox_tracesize_test.go:183: {failure}")
    lines.append(f"--- {terminal}: TestSandboxTraceGateSynthetic (0.31s)")
    return "\n".join(lines) + "\n"


class ClassifyG1GateResultTests(unittest.TestCase):
    def test_pass(self):
        self.assertEqual(classifier.classify(gate_log(), 0, "candidate"), "PASS")

    def test_valid_baseline_floor_miss_is_data(self):
        failure = "synthetic-slice: the grammar path's best of 7 passes read a 66600 byte slice in 5.06166ms (13.2 MB/s), below the 14 MB/s floor or not a measurement (gate G1, KILN04 section 4.9)"
        self.assertEqual(
            classifier.classify(gate_log("FAIL", "13.2", failure), 1, "baseline"),
            "MEASURED_BELOW_FLOOR rate_mb_s=13.2 elapsed_ns=5061660",
        )

    def test_zero_duration_fail_is_invalid(self):
        failure = "synthetic-slice: the grammar path's best of 7 passes read a 66600 byte slice in 0s (0.0 MB/s), below the 14 MB/s floor or not a measurement (gate G1, KILN04 section 4.9)"
        with self.assertRaises(classifier.InvalidReceipt):
            classifier.classify(gate_log("FAIL", "0.0", failure), 1, "baseline")

    def test_negative_duration_is_invalid(self):
        failure = "synthetic-slice: the grammar path's best of 7 passes read a 66600 byte slice in -1ns (0.0 MB/s), below the 14 MB/s floor or not a measurement (gate G1, KILN04 section 4.9)"
        with self.assertRaises(classifier.InvalidReceipt):
            classifier.classify(gate_log("FAIL", "0.0", failure), 1, "baseline")

    def test_positive_duration_below_failure_bound_is_invalid(self):
        failure = "synthetic-slice: the grammar path's best of 7 passes read a 66600 byte slice in 4.7ms (14.0 MB/s), below the 14 MB/s floor or not a measurement (gate G1, KILN04 section 4.9)"
        with self.assertRaises(classifier.InvalidReceipt):
            classifier.classify(gate_log("FAIL", "14.0", failure), 1, "baseline")

    def test_missing_exit_is_invalid(self):
        with self.assertRaises(classifier.InvalidReceipt):
            classifier.parse_exit_code("")

    def test_abnormal_exit_after_markers_is_invalid(self):
        with self.assertRaises(classifier.InvalidReceipt):
            classifier.classify(gate_log(), 2, "baseline")

    def test_pass_with_nonzero_exit_is_invalid(self):
        with self.assertRaises(classifier.InvalidReceipt):
            classifier.classify(gate_log(), 1, "baseline")

    def test_fail_with_zero_exit_is_invalid(self):
        failure = "synthetic-slice: the grammar path's best of 7 passes read a 66600 byte slice in 5.06166ms (13.2 MB/s), below the 14 MB/s floor or not a measurement (gate G1, KILN04 section 4.9)"
        with self.assertRaises(classifier.InvalidReceipt):
            classifier.classify(gate_log("FAIL", "13.2", failure), 0, "baseline")

    def test_missing_workload_markers_is_invalid(self):
        with self.assertRaises(classifier.InvalidReceipt):
            classifier.classify(gate_log(census=False), 0, "baseline")
        with self.assertRaises(classifier.InvalidReceipt):
            classifier.classify(gate_log(slice_marker=False), 0, "baseline")

    def test_valid_candidate_miss_is_not_a_pass(self):
        failure = "synthetic-slice: the grammar path's best of 7 passes read a 66600 byte slice in 5.06166ms (13.2 MB/s), below the 14 MB/s floor or not a measurement (gate G1, KILN04 section 4.9)"
        self.assertEqual(
            classifier.classify(gate_log("FAIL", "13.2", failure), 1, "candidate"),
            "CANDIDATE_GATE_MISS rate_mb_s=13.2 elapsed_ns=5061660",
        )


if __name__ == "__main__":
    unittest.main()
