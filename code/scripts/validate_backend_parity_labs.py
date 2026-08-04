#!/usr/bin/env python3
"""Validate the deterministic NN31 CPU/Rust/accelerator parity corpus."""

from __future__ import annotations

import argparse
import json
import math
import re
import struct
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = REPO_ROOT / "code" / "specs" / "fixtures" / "backend-parity-v1"
CANONICAL_ID = "dense-backend-parity"
CANONICAL_TOLERANCE = 1e-6
MAX_FILE_BYTES = 1_000_000
MAX_TEXT = 512
MAX_ABSOLUTE_NUMBER = 1e6
HEX_RE = re.compile(r"^[0-9a-f]+$")


class BackendParityValidationError(ValueError):
    """Raised when an NN31 fixture violates the closed contract."""


def _reject_constant(value: str) -> None:
    raise BackendParityValidationError(f"non-finite JSON number: {value}")


def _object_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise BackendParityValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> Any:
    try:
        if path.stat().st_size > MAX_FILE_BYTES:
            raise BackendParityValidationError(f"{path}: file exceeds size limit")
        return json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_object_pairs,
            parse_constant=_reject_constant,
        )
    except BackendParityValidationError:
        raise
    except (
        OSError,
        UnicodeError,
        json.JSONDecodeError,
        RecursionError,
        ValueError,
    ) as exc:
        raise BackendParityValidationError(f"{path}: invalid JSON: {exc}") from exc


def _object(value: Any, keys: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        raise BackendParityValidationError(f"{context}: object key mismatch")
    return value


def _text(value: Any, context: str) -> str:
    if not isinstance(value, str) or not 1 <= len(value) <= MAX_TEXT:
        raise BackendParityValidationError(f"{context}: expected bounded text")
    return value


def _number(value: Any, context: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise BackendParityValidationError(f"{context}: expected finite bounded number")
    if isinstance(value, int):
        if abs(value) > MAX_ABSOLUTE_NUMBER:
            raise BackendParityValidationError(
                f"{context}: expected finite bounded number"
            )
        return float(value)
    if not math.isfinite(value) or abs(value) > MAX_ABSOLUTE_NUMBER:
        raise BackendParityValidationError(f"{context}: expected finite bounded number")
    return float(value)


def _numbers(value: Any, length: int, context: str) -> list[float]:
    if not isinstance(value, list) or len(value) != length:
        raise BackendParityValidationError(f"{context}: expected {length} numbers")
    return [_number(item, f"{context}[{index}]") for index, item in enumerate(value)]


def _integers(value: Any, length: int, context: str) -> list[int]:
    numbers = _numbers(value, length, context)
    if not all(number.is_integer() for number in numbers):
        raise BackendParityValidationError(f"{context}: expected integers")
    return [int(number) for number in numbers]


def _f32(value: float, context: str) -> float:
    try:
        rounded = struct.unpack("<f", struct.pack("<f", value))[0]
    except (OverflowError, struct.error) as exc:
        raise BackendParityValidationError(
            f"{context}: cannot encode finite f32"
        ) from exc
    if not math.isfinite(rounded):
        raise BackendParityValidationError(f"{context}: f32 result is not finite")
    return rounded


def decode_f32_hex(text: str, context: str) -> list[float]:
    payload = text.strip()
    if (
        not payload
        or len(payload) > 256
        or len(payload) % 8 != 0
        or HEX_RE.fullmatch(payload) is None
    ):
        raise BackendParityValidationError(
            f"{context}: expected bounded lowercase f32le hex"
        )
    raw = bytes.fromhex(payload)
    values = [
        struct.unpack("<f", raw[index : index + 4])[0]
        for index in range(0, len(raw), 4)
    ]
    if not all(math.isfinite(value) for value in values):
        raise BackendParityValidationError(f"{context}: non-finite f32 payload")
    return values


def _resolve_reference(root: Path, lab_path: Path, reference: str, suffix: str) -> Path:
    if not isinstance(reference, str) or not reference.endswith(suffix):
        raise BackendParityValidationError("fixture reference has unexpected suffix")
    resolved_root = root.resolve()
    resolved = (lab_path.parent / reference).resolve()
    try:
        resolved.relative_to(resolved_root)
    except ValueError as exc:
        raise BackendParityValidationError(
            "fixture reference escapes fixture root"
        ) from exc
    if not resolved.is_file():
        raise BackendParityValidationError(
            f"fixture reference does not exist: {reference}"
        )
    if resolved.stat().st_size > MAX_FILE_BYTES:
        raise BackendParityValidationError(
            f"fixture reference exceeds size limit: {reference}"
        )
    return resolved


def _exact_equal(actual: Any, expected: Any, depth: int = 0) -> bool:
    """Compare a canonical JSON value without bool/int or int/float coercion."""
    if depth > 16 or type(actual) is not type(expected):
        return False
    if isinstance(expected, dict):
        return actual.keys() == expected.keys() and all(
            _exact_equal(actual[key], expected[key], depth + 1) for key in expected
        )
    if isinstance(expected, list):
        return len(actual) == len(expected) and all(
            _exact_equal(left, right, depth + 1)
            for left, right in zip(actual, expected, strict=True)
        )
    return actual == expected


CANONICAL_GRAPH = {
    "equation": "y = XW + B",
    "dtype": "f32",
    "input_shape": [3, 1],
    "weight_shape": [1, 1],
    "bias_shape": [3, 1],
    "output_shape": [3, 1],
    "weight": [2.0],
    "bias": [1.0, 1.0, 1.0],
    "matrix_ir_file": "../matrix-ir/00-dense-batch.graph.json",
}

CANONICAL_MATRIX_IR = {
    "matrix_ir_version": 1,
    "tensors": [
        {"id": 0, "dtype": "f32", "shape": [3, 1]},
        {"id": 1, "dtype": "f32", "shape": [1, 1]},
        {"id": 2, "dtype": "f32", "shape": [3, 1]},
        {"id": 3, "dtype": "f32", "shape": [3, 1]},
        {"id": 4, "dtype": "f32", "shape": [3, 1]},
    ],
    "inputs": [0],
    "outputs": [4],
    "ops": [
        {"kind": "MatMul", "a": 0, "b": 1, "output": 2},
        {"kind": "Add", "lhs": 2, "rhs": 3, "output": 4},
    ],
    "constants": [
        {
            "tensor_id": 1,
            "dtype": "f32",
            "shape": [1, 1],
            "bytes_hex": "00000040",
        },
        {
            "tensor_id": 3,
            "dtype": "f32",
            "shape": [3, 1],
            "bytes_hex": "0000803f0000803f0000803f",
        },
    ],
}

CANONICAL_LANES = [
    {
        "id": "scalar_cpu",
        "title": "Scalar CPU reference",
        "runtime": "NN00 bytecode interpreter",
        "precision": "binary64",
        "availability": "required",
        "steps": ["load one x", "multiply by w", "add b", "store one y"],
        "residency": ["host:x", "host:product", "host:y"],
    },
    {
        "id": "typescript_matrix_cpu",
        "title": "TypeScript matrix CPU",
        "runtime": "NN01 CANM matrix plan",
        "precision": "binary64",
        "availability": "required",
        "steps": [
            "load x column",
            "scale column by w",
            "broadcast and add b",
            "store y column",
        ],
        "residency": [
            "host:x[3x1]",
            "host:product[3x1]",
            "host:y[3x1]",
        ],
    },
    {
        "id": "rust_matrix_cpu",
        "title": "Rust matrix CPU core",
        "runtime": "MatrixIR JSON -> matrix-rust-napi -> matrix-cpu",
        "precision": "f32",
        "availability": "required-in-native-test",
        "steps": [
            "decode MatrixIR JSON",
            "MatMul X by W",
            "Add broadcast B",
            "download output bytes",
        ],
        "residency": [
            "host:x bytes",
            "rust:x,W,B buffers",
            "rust:y buffer",
            "host:y bytes",
        ],
    },
    {
        "id": "webgpu_accelerated",
        "title": "WebGPU accelerator",
        "runtime": "NN01 async WebGpuMatrixBackend",
        "precision": "f32",
        "availability": "optional-runtime-probe",
        "steps": [
            "upload x column",
            "scale on device",
            "add bias on device",
            "download y and value trace",
        ],
        "residency": [
            "host:x",
            "device:x,product,bias,y",
            "host:output y",
            "host:trace x,bias,y",
        ],
    },
]


def execute_reference(
    inputs: list[float], weight: float, bias: list[float]
) -> dict[str, Any]:
    products = [input_value * weight for input_value in inputs]
    outputs = [product + bias[index] for index, product in enumerate(products)]
    f32_products = [
        _f32(_f32(value, "f32 input") * _f32(weight, "f32 weight"), "f32 product")
        for value in inputs
    ]
    f32_outputs = [
        _f32(product + _f32(bias[index], "f32 bias"), "f32 output")
        for index, product in enumerate(f32_products)
    ]
    return {
        "products": products,
        "outputs": outputs,
        "f32_products": f32_products,
        "f32_outputs": f32_outputs,
    }


def _assert_close(
    actual: list[float], expected: list[float], tolerance: float, context: str
) -> None:
    if len(actual) != len(expected):
        raise BackendParityValidationError(f"{context}: length mismatch")
    for index, (left, right) in enumerate(zip(actual, expected, strict=True)):
        if not math.isclose(left, right, rel_tol=0.0, abs_tol=tolerance):
            raise BackendParityValidationError(
                f"{context}[{index}]: expected {right:g}, got {left:g}"
            )


def validate_document(value: Any, root: Path, lab_path: Path) -> dict[str, Any]:
    lab = _object(
        value,
        {
            "schema_version",
            "id",
            "title",
            "question",
            "absolute_tolerance",
            "graph",
            "scenario",
            "lanes",
        },
        "lab",
    )
    if lab["schema_version"] != 1 or isinstance(lab["schema_version"], bool):
        raise BackendParityValidationError("lab.schema_version: expected 1")
    if lab["id"] != CANONICAL_ID:
        raise BackendParityValidationError(f"lab.id: expected {CANONICAL_ID}")
    title = _text(lab["title"], "lab.title")
    question = _text(lab["question"], "lab.question")
    tolerance = _number(lab["absolute_tolerance"], "lab.absolute_tolerance")
    if tolerance != CANONICAL_TOLERANCE:
        raise BackendParityValidationError(
            "lab.absolute_tolerance: expected canonical 1e-6"
        )

    graph = _object(
        lab["graph"],
        {
            "equation",
            "dtype",
            "input_shape",
            "weight_shape",
            "bias_shape",
            "output_shape",
            "weight",
            "bias",
            "matrix_ir_file",
        },
        "lab.graph",
    )
    normalized_graph = {
        "equation": _text(graph["equation"], "lab.graph.equation"),
        "dtype": _text(graph["dtype"], "lab.graph.dtype"),
        "input_shape": _integers(graph["input_shape"], 2, "lab.graph.input_shape"),
        "weight_shape": _integers(graph["weight_shape"], 2, "lab.graph.weight_shape"),
        "bias_shape": _integers(graph["bias_shape"], 2, "lab.graph.bias_shape"),
        "output_shape": _integers(graph["output_shape"], 2, "lab.graph.output_shape"),
        "weight": _numbers(graph["weight"], 1, "lab.graph.weight"),
        "bias": _numbers(graph["bias"], 3, "lab.graph.bias"),
        "matrix_ir_file": _text(graph["matrix_ir_file"], "lab.graph.matrix_ir_file"),
    }
    if normalized_graph != CANONICAL_GRAPH:
        raise BackendParityValidationError("lab.graph: expected canonical dense graph")

    scenario = _object(
        lab["scenario"],
        {"id", "inputs", "input_payload_file", "expected_payload_file", "expected"},
        "lab.scenario",
    )
    if scenario["id"] != "three_row_dense":
        raise BackendParityValidationError("lab.scenario.id: expected three_row_dense")
    inputs = _numbers(scenario["inputs"], 3, "lab.scenario.inputs")
    input_reference = _text(scenario["input_payload_file"], "input payload reference")
    output_reference = _text(
        scenario["expected_payload_file"], "output payload reference"
    )
    if input_reference != "../payloads/00-input-x.f32le.hex":
        raise BackendParityValidationError("input payload reference is not canonical")
    if output_reference != "../payloads/00-expected-output.f32le.hex":
        raise BackendParityValidationError("output payload reference is not canonical")
    expected = _object(
        scenario["expected"], {"products", "outputs"}, "lab.scenario.expected"
    )
    expected_products = _numbers(expected["products"], 3, "expected products")
    expected_outputs = _numbers(expected["outputs"], 3, "expected outputs")

    matrix_path = _resolve_reference(
        root, lab_path, normalized_graph["matrix_ir_file"], ".json"
    )
    matrix_ir = load_json(matrix_path)
    if not _exact_equal(matrix_ir, CANONICAL_MATRIX_IR):
        raise BackendParityValidationError("MatrixIR graph is not canonical")
    input_path = _resolve_reference(root, lab_path, input_reference, ".hex")
    output_path = _resolve_reference(root, lab_path, output_reference, ".hex")
    try:
        input_text = input_path.read_text(encoding="ascii")
        output_text = output_path.read_text(encoding="ascii")
    except (OSError, UnicodeError) as exc:
        raise BackendParityValidationError(f"invalid payload text: {exc}") from exc
    input_payload = decode_f32_hex(input_text, "input payload")
    output_payload = decode_f32_hex(output_text, "output payload")
    _assert_close(input_payload, inputs, 0.0, "input payload")

    trace = execute_reference(
        inputs, normalized_graph["weight"][0], normalized_graph["bias"]
    )
    _assert_close(trace["products"], expected_products, 0.0, "expected products")
    _assert_close(trace["outputs"], expected_outputs, 0.0, "expected outputs")
    _assert_close(trace["f32_outputs"], output_payload, 0.0, "expected output payload")

    raw_lanes = lab["lanes"]
    if not isinstance(raw_lanes, list) or len(raw_lanes) != len(CANONICAL_LANES):
        raise BackendParityValidationError("lab.lanes: expected four canonical lanes")
    normalized_lanes: list[dict[str, Any]] = []
    for index, (raw_lane, canonical) in enumerate(
        zip(raw_lanes, CANONICAL_LANES, strict=True)
    ):
        lane = _object(
            raw_lane,
            {
                "id",
                "title",
                "runtime",
                "precision",
                "availability",
                "steps",
                "residency",
                "expected_outputs",
            },
            f"lab.lanes[{index}]",
        )
        normalized = {
            "id": _text(lane["id"], f"lane {index} id"),
            "title": _text(lane["title"], f"lane {index} title"),
            "runtime": _text(lane["runtime"], f"lane {index} runtime"),
            "precision": _text(lane["precision"], f"lane {index} precision"),
            "availability": _text(lane["availability"], f"lane {index} availability"),
            "steps": [_text(item, f"lane {index} step") for item in lane["steps"]]
            if isinstance(lane["steps"], list) and len(lane["steps"]) == 4
            else [],
            "residency": [
                _text(item, f"lane {index} residency") for item in lane["residency"]
            ]
            if isinstance(lane["residency"], list) and 3 <= len(lane["residency"]) <= 4
            else [],
        }
        if normalized != canonical:
            raise BackendParityValidationError(
                f"lab.lanes[{index}]: canonical lane mismatch"
            )
        lane_outputs = _numbers(lane["expected_outputs"], 3, f"lane {index} outputs")
        oracle = (
            trace["outputs"]
            if canonical["precision"] == "binary64"
            else trace["f32_outputs"]
        )
        _assert_close(lane_outputs, oracle, 0.0, f"lane {index} outputs")
        normalized_lanes.append({**normalized, "expected_outputs": lane_outputs})

    return {
        "schema_version": 1,
        "id": CANONICAL_ID,
        "title": title,
        "question": question,
        "absolute_tolerance": tolerance,
        "graph": normalized_graph,
        "scenario": {
            "id": "three_row_dense",
            "inputs": inputs,
            "input_payload_file": input_reference,
            "expected_payload_file": output_reference,
            "expected": {"products": expected_products, "outputs": expected_outputs},
        },
        "lanes": normalized_lanes,
        "trace": trace,
    }


def validate_fixture_root(root: Path = DEFAULT_FIXTURE_ROOT) -> int:
    lab_paths = sorted((root / "labs").glob("*.json"))
    if len(lab_paths) != 1:
        raise BackendParityValidationError(
            f"{root}: expected exactly one lab JSON file"
        )
    for lab_path in lab_paths:
        validate_document(load_json(lab_path), root, lab_path)
    return len(lab_paths)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "fixture_root", nargs="?", type=Path, default=DEFAULT_FIXTURE_ROOT
    )
    args = parser.parse_args()
    count = validate_fixture_root(args.fixture_root)
    print(f"validated {count} backend parity lab(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
