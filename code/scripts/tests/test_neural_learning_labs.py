from __future__ import annotations

import copy
import importlib.util
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SCRIPT_PATH = ROOT / "code" / "scripts" / "validate_neural_learning_labs.py"
SPEC = importlib.util.spec_from_file_location(
    "validate_neural_learning_labs", SCRIPT_PATH
)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class NeuralLearningLabTests(unittest.TestCase):
    def test_checked_in_corpus_is_valid(self) -> None:
        paths = MODULE.validate_corpus()
        self.assertEqual(
            [path.name for path in paths],
            [
                "00-weighted-neuron.json",
                "01-celsius-linear-regression.json",
                "02-or-sigmoid-neuron.json",
                "03-xor-hidden-representation.json",
            ],
        )

    def test_weighted_neuron_exposes_the_hand_calculated_value(self) -> None:
        path = MODULE.DEFAULT_FIXTURE_ROOT / "labs" / "00-weighted-neuron.json"
        lab = MODULE.load_json(path)
        result = MODULE.forward(lab)
        self.assertAlmostEqual(result["predictions"][0][0], 1.35)

    def test_training_trace_detects_a_changed_gradient(self) -> None:
        path = MODULE.DEFAULT_FIXTURE_ROOT / "labs" / "02-or-sigmoid-neuron.json"
        lab = MODULE.load_json(path)
        broken = copy.deepcopy(lab)
        broken["expected"]["first_step"]["gradients"][0]["biases"][0] = 99
        with self.assertRaisesRegex(MODULE.LabValidationError, "biases"):
            MODULE.validate_lab(broken, "broken-or")

    def test_shape_validation_rejects_a_ragged_weight_matrix(self) -> None:
        path = (
            MODULE.DEFAULT_FIXTURE_ROOT / "labs" / "03-xor-hidden-representation.json"
        )
        lab = MODULE.load_json(path)
        broken = copy.deepcopy(lab)
        broken["model"]["layers"][0]["weights"][1].pop()
        with self.assertRaisesRegex(MODULE.LabValidationError, "equal widths"):
            MODULE.validate_lab(broken, "broken-xor")


if __name__ == "__main__":
    unittest.main()
