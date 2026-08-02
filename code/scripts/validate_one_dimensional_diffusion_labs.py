#!/usr/bin/env python3
"""Validate and execute the deterministic NN19 scalar diffusion corpus."""

from __future__ import annotations

import argparse
import json
import math
from collections.abc import Callable
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = (
    REPO_ROOT / "code" / "specs" / "fixtures" / "one-dimensional-diffusion-v1"
)
PARAMETER_ORDER = [
    "denoiser.sample_weight",
    "denoiser.timestep_weight",
    "denoiser.bias",
]


class OneDimensionalDiffusionValidationError(ValueError):
    """Raised when an NN19 document or deterministic result is invalid."""


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise OneDimensionalDiffusionValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    try:
        document = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_reject_duplicate_pairs,
            parse_constant=lambda value: (_ for _ in ()).throw(
                OneDimensionalDiffusionValidationError(
                    f"non-finite JSON number: {value}"
                )
            ),
        )
    except (OSError, json.JSONDecodeError) as error:
        raise OneDimensionalDiffusionValidationError(f"{path}: {error}") from error
    if not isinstance(document, dict):
        raise OneDimensionalDiffusionValidationError(
            "top-level JSON value must be an object"
        )
    return document


def _object(value: Any, keys: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise OneDimensionalDiffusionValidationError(f"{context}: expected an object")
    missing = keys - value.keys()
    extra = value.keys() - keys
    if missing or extra:
        raise OneDimensionalDiffusionValidationError(
            f"{context}: key mismatch missing={sorted(missing)} extra={sorted(extra)}"
        )
    return value


def _number(value: Any, context: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise OneDimensionalDiffusionValidationError(f"{context}: expected a number")
    result = float(value)
    if not math.isfinite(result):
        raise OneDimensionalDiffusionValidationError(
            f"{context}: expected a finite number"
        )
    return result


def _integer(value: Any, context: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise OneDimensionalDiffusionValidationError(f"{context}: expected an integer")
    return value


def _denoiser(value: Any, context: str) -> dict[str, float]:
    parameters = _object(
        value,
        {"sample_weight", "timestep_weight", "bias"},
        context,
    )
    return {
        key: _number(parameters[key], f"{context}.{key}")
        for key in ("sample_weight", "timestep_weight", "bias")
    }


def _forward_steps(
    clean_sample: float,
    saved_noise: float,
    schedule: list[dict[str, float]],
) -> list[dict[str, float | int]]:
    alpha_bar = 1.0
    steps: list[dict[str, float | int]] = []
    for row in schedule:
        alpha = 1 - row["beta"]
        alpha_bar *= alpha
        signal_scale = math.sqrt(alpha_bar)
        noise_scale = math.sqrt(1 - alpha_bar)
        signal_contribution = signal_scale * clean_sample
        noise_contribution = noise_scale * saved_noise
        steps.append(
            {
                "t": int(row["t"]),
                "beta": row["beta"],
                "alpha": alpha,
                "alpha_bar": alpha_bar,
                "normalized_t": row["normalized_t"],
                "signal_scale": signal_scale,
                "noise_scale": noise_scale,
                "signal_contribution": signal_contribution,
                "noise_contribution": noise_contribution,
                "noisy_sample": signal_contribution + noise_contribution,
            }
        )
    return steps


def _prediction_steps(
    forward_steps: list[dict[str, float | int]],
    saved_noise: float,
    denoiser: dict[str, float],
) -> tuple[list[dict[str, float | int]], float]:
    rows: list[dict[str, float | int]] = []
    for step in forward_steps:
        noisy_sample = float(step["noisy_sample"])
        normalized_t = float(step["normalized_t"])
        predicted_noise = (
            denoiser["sample_weight"] * noisy_sample
            + denoiser["timestep_weight"] * normalized_t
            + denoiser["bias"]
        )
        error = predicted_noise - saved_noise
        rows.append(
            {
                "t": step["t"],
                "noisy_sample": noisy_sample,
                "normalized_t": normalized_t,
                "predicted_noise": predicted_noise,
                "target_noise": saved_noise,
                "error": error,
                "loss": 0.5 * error * error,
            }
        )
    mean_loss = sum(float(row["loss"]) for row in rows) / len(rows)
    return rows, mean_loss


def _gradient_check(
    parameters: list[float],
    analytical: list[float],
    loss_function: Callable[[list[float]], float],
) -> dict[str, Any]:
    epsilon = 1e-6
    numerical = []
    for index in range(len(parameters)):
        plus = parameters[:]
        minus = parameters[:]
        plus[index] += epsilon
        minus[index] -= epsilon
        numerical.append((loss_function(plus) - loss_function(minus)) / (2 * epsilon))
    return {
        "epsilon": epsilon,
        "parameter_order": PARAMETER_ORDER,
        "analytical": analytical,
        "numerical": numerical,
        "max_absolute_error": max(
            abs(analytical_value - numerical_value)
            for analytical_value, numerical_value in zip(analytical, numerical)
        ),
    }


def _reverse_steps(
    forward_steps: list[dict[str, float | int]],
    denoiser: dict[str, float],
) -> list[dict[str, float | int]]:
    current_sample = float(forward_steps[-1]["noisy_sample"])
    reverse: list[dict[str, float | int]] = []
    for step in reversed(forward_steps):
        normalized_t = float(step["normalized_t"])
        predicted_noise = (
            denoiser["sample_weight"] * current_sample
            + denoiser["timestep_weight"] * normalized_t
            + denoiser["bias"]
        )
        noise_coefficient = float(step["beta"]) / float(step["noise_scale"])
        scaled_noise_correction = noise_coefficient * predicted_noise
        corrected_sample = current_sample - scaled_noise_correction
        alpha_scale = math.sqrt(float(step["alpha"]))
        output_mean = corrected_sample / alpha_scale
        reverse.append(
            {
                "t": step["t"],
                "input_sample": current_sample,
                "normalized_t": normalized_t,
                "predicted_noise": predicted_noise,
                "noise_coefficient": noise_coefficient,
                "scaled_noise_correction": scaled_noise_correction,
                "corrected_sample": corrected_sample,
                "alpha_scale": alpha_scale,
                "output_mean": output_mean,
            }
        )
        current_sample = output_mean
    return reverse


def execute_lab(document: dict[str, Any]) -> dict[str, Any]:
    clean_sample = _number(document["clean_sample"], "clean_sample")
    saved_noise = _number(document["saved_noise"], "saved_noise")
    schedule = [
        {
            "t": _integer(row["t"], f"schedule[{index}].t"),
            "beta": _number(row["beta"], f"schedule[{index}].beta"),
            "normalized_t": _number(
                row["normalized_t"], f"schedule[{index}].normalized_t"
            ),
        }
        for index, row in enumerate(document["schedule"])
    ]
    denoiser = _denoiser(document["denoiser"], "denoiser")
    learning_rate = _number(
        document["optimizer"]["learning_rate"],
        "optimizer.learning_rate",
    )

    forward_steps = _forward_steps(clean_sample, saved_noise, schedule)
    initial_denoising, initial_mean_loss = _prediction_steps(
        forward_steps,
        saved_noise,
        denoiser,
    )
    count = len(initial_denoising)
    per_step: list[dict[str, float | int]] = []
    for row in initial_denoising:
        prediction_gradient = float(row["error"]) / count
        per_step.append(
            {
                "t": row["t"],
                "prediction_gradient": prediction_gradient,
                "sample_weight_contribution": prediction_gradient
                * float(row["noisy_sample"]),
                "timestep_weight_contribution": prediction_gradient
                * float(row["normalized_t"]),
                "bias_contribution": prediction_gradient,
            }
        )
    sample_weight_gradient = sum(
        float(row["sample_weight_contribution"]) for row in per_step
    )
    timestep_weight_gradient = sum(
        float(row["timestep_weight_contribution"]) for row in per_step
    )
    bias_gradient = sum(float(row["bias_contribution"]) for row in per_step)
    analytical = [
        sample_weight_gradient,
        timestep_weight_gradient,
        bias_gradient,
    ]
    initial_parameters = [
        denoiser["sample_weight"],
        denoiser["timestep_weight"],
        denoiser["bias"],
    ]

    def loss_for(values: list[float]) -> float:
        return _prediction_steps(
            forward_steps,
            saved_noise,
            {
                "sample_weight": values[0],
                "timestep_weight": values[1],
                "bias": values[2],
            },
        )[1]

    gradient_check = _gradient_check(
        initial_parameters,
        analytical,
        loss_for,
    )
    updated_values = [
        value - learning_rate * gradient
        for value, gradient in zip(initial_parameters, analytical)
    ]
    updated_denoiser = {
        "sample_weight": updated_values[0],
        "timestep_weight": updated_values[1],
        "bias": updated_values[2],
    }
    post_update_denoising, post_update_mean_loss = _prediction_steps(
        forward_steps,
        saved_noise,
        updated_denoiser,
    )
    reverse_steps = _reverse_steps(forward_steps, updated_denoiser)
    final_reconstruction = float(reverse_steps[-1]["output_mean"])
    return {
        "forward_steps": forward_steps,
        "initial_denoising": initial_denoising,
        "initial_mean_loss": initial_mean_loss,
        "backward": {
            "per_step": per_step,
            "sample_weight_gradient": sample_weight_gradient,
            "timestep_weight_gradient": timestep_weight_gradient,
            "bias_gradient": bias_gradient,
        },
        "gradient_check": gradient_check,
        "updated_denoiser": updated_denoiser,
        "post_update_denoising": post_update_denoising,
        "post_update_mean_loss": post_update_mean_loss,
        "reverse_steps": reverse_steps,
        "final_reconstruction": final_reconstruction,
        "final_absolute_error": abs(final_reconstruction - clean_sample),
    }


def _compare(expected: Any, actual: Any, tolerance: float, context: str) -> None:
    if isinstance(expected, bool) or isinstance(actual, bool):
        if expected != actual:
            raise OneDimensionalDiffusionValidationError(
                f"{context}: expected {expected}, got {actual}"
            )
        return
    if isinstance(expected, (int, float)) and isinstance(actual, (int, float)):
        if not math.isclose(
            float(expected),
            float(actual),
            rel_tol=0,
            abs_tol=tolerance,
        ):
            raise OneDimensionalDiffusionValidationError(
                f"{context}: expected {expected}, got {actual}"
            )
        return
    if isinstance(expected, list) and isinstance(actual, list):
        if len(expected) != len(actual):
            raise OneDimensionalDiffusionValidationError(f"{context}: length mismatch")
        for index, (expected_item, actual_item) in enumerate(zip(expected, actual)):
            _compare(expected_item, actual_item, tolerance, f"{context}[{index}]")
        return
    if isinstance(expected, dict) and isinstance(actual, dict):
        if expected.keys() != actual.keys():
            raise OneDimensionalDiffusionValidationError(f"{context}: key mismatch")
        for key in expected:
            _compare(expected[key], actual[key], tolerance, f"{context}.{key}")
        return
    if expected != actual:
        raise OneDimensionalDiffusionValidationError(
            f"{context}: expected {expected!r}, got {actual!r}"
        )


def validate_lab(document: dict[str, Any]) -> dict[str, Any]:
    _object(
        document,
        {
            "schema_version",
            "id",
            "title",
            "question",
            "absolute_tolerance",
            "concepts",
            "operation",
            "clean_sample",
            "saved_noise",
            "schedule",
            "denoiser",
            "optimizer",
            "expected",
        },
        "lab",
    )
    if document["schema_version"] != 1:
        raise OneDimensionalDiffusionValidationError("schema_version: expected 1")
    for key in ("id", "title", "question"):
        if not isinstance(document[key], str) or not document[key]:
            raise OneDimensionalDiffusionValidationError(f"{key}: expected text")
    concepts = document["concepts"]
    if (
        not isinstance(concepts, list)
        or not concepts
        or any(not isinstance(item, str) or not item for item in concepts)
        or len(set(concepts)) != len(concepts)
    ):
        raise OneDimensionalDiffusionValidationError(
            "concepts: expected unique non-empty strings"
        )
    tolerance = _number(document["absolute_tolerance"], "absolute_tolerance")
    if tolerance <= 0:
        raise OneDimensionalDiffusionValidationError(
            "absolute_tolerance: expected a positive number"
        )
    required_operation = {
        "kind": "one-dimensional-diffusion-round",
        "forward": "closed-form-saved-noise",
        "denoiser": "scalar-affine-with-normalized-timestep",
        "loss": "mean-half-squared-noise-error",
        "reverse": "deterministic-ddpm-mean",
        "update": "sgd",
    }
    operation = _object(
        document["operation"],
        set(required_operation),
        "operation",
    )
    if operation != required_operation:
        raise OneDimensionalDiffusionValidationError(
            "operation: unsupported NN19 contract"
        )
    _number(document["clean_sample"], "clean_sample")
    _number(document["saved_noise"], "saved_noise")
    _denoiser(document["denoiser"], "denoiser")
    optimizer = _object(document["optimizer"], {"learning_rate"}, "optimizer")
    if _number(optimizer["learning_rate"], "optimizer.learning_rate") <= 0:
        raise OneDimensionalDiffusionValidationError(
            "optimizer.learning_rate: expected a positive number"
        )

    schedule = document["schedule"]
    if not isinstance(schedule, list) or len(schedule) < 2:
        raise OneDimensionalDiffusionValidationError(
            "schedule: expected at least two steps"
        )
    previous_normalized_t = 0.0
    for index, raw_row in enumerate(schedule):
        row = _object(raw_row, {"t", "beta", "normalized_t"}, f"schedule[{index}]")
        if _integer(row["t"], f"schedule[{index}].t") != index + 1:
            raise OneDimensionalDiffusionValidationError(
                f"schedule[{index}].t: expected consecutive timesteps"
            )
        beta = _number(row["beta"], f"schedule[{index}].beta")
        if not 0 < beta < 1:
            raise OneDimensionalDiffusionValidationError(
                f"schedule[{index}].beta: expected 0 < beta < 1"
            )
        normalized_t = _number(
            row["normalized_t"],
            f"schedule[{index}].normalized_t",
        )
        if not previous_normalized_t < normalized_t <= 1:
            raise OneDimensionalDiffusionValidationError(
                f"schedule[{index}].normalized_t: expected increasing values in (0, 1]"
            )
        previous_normalized_t = normalized_t
    if not math.isclose(previous_normalized_t, 1, rel_tol=0, abs_tol=1e-12):
        raise OneDimensionalDiffusionValidationError(
            "schedule: final normalized timestep must equal 1"
        )

    _object(
        document["expected"],
        {
            "forward_steps",
            "initial_denoising",
            "initial_mean_loss",
            "backward",
            "gradient_check",
            "updated_denoiser",
            "post_update_denoising",
            "post_update_mean_loss",
            "reverse_steps",
            "final_reconstruction",
            "final_absolute_error",
        },
        "expected",
    )
    actual = execute_lab(document)
    _compare(document["expected"], actual, tolerance, "expected")
    if actual["gradient_check"]["max_absolute_error"] >= 1e-8:
        raise OneDimensionalDiffusionValidationError(
            "expected.gradient_check: gradients disagree"
        )
    if actual["post_update_mean_loss"] >= actual["initial_mean_loss"]:
        raise OneDimensionalDiffusionValidationError(
            "expected.post_update_mean_loss: SGD step must lower the objective"
        )
    noisiest_error = abs(
        float(actual["forward_steps"][-1]["noisy_sample"])
        - float(document["clean_sample"])
    )
    if actual["final_absolute_error"] >= noisiest_error:
        raise OneDimensionalDiffusionValidationError(
            "expected.final_absolute_error: reverse mean must improve on x_T"
        )
    return actual


def validate_fixture_root(root: Path) -> int:
    labs = sorted((root / "labs").glob("*.json"))
    if not labs:
        raise OneDimensionalDiffusionValidationError(f"{root}: no lab documents found")
    for path in labs:
        validate_lab(load_json(path))
        print(f"validated {path.relative_to(REPO_ROOT)}")
    return len(labs)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", nargs="?", type=Path, default=DEFAULT_FIXTURE_ROOT)
    args = parser.parse_args()
    count = validate_fixture_root(args.root.resolve())
    print(f"validated {count} NN19 one-dimensional diffusion lab document(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
