#!/usr/bin/env python3
"""Validate the deterministic NN32 precision, quantization, and residency lab."""

from __future__ import annotations

import argparse
import json
import math
import re
import struct
from collections.abc import Callable
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = (
    REPO_ROOT / "code" / "specs" / "fixtures" / "precision-residency-v1"
)
MAX_FILE_BYTES = 1_000_000
MAX_TEXT = 512
MAX_ABSOLUTE_NUMBER = 1e6
HEX_RE = re.compile(r"^[0-9a-f]+$")
EXPECTED_FILES = {
    "CHANGELOG.md",
    "README.md",
    "schema.json",
    "labs/00-tiny-affine.json",
    "payloads/00-input-x.f32le.hex",
    "payloads/00-output-y.f32le.hex",
    "payloads/00-input-x.f16le.hex",
    "payloads/00-output-y.f16le.hex",
    "payloads/00-input-x.i8.hex",
    "payloads/00-weight-w.i8.hex",
}
CANONICAL_FORMAT_TITLES = {
    "binary32": "IEEE-754 binary32",
    "binary16": "IEEE-754 binary16",
    "symmetric_int8": "Symmetric signed int8",
}
CANONICAL_STRATEGIES = {
    "eager": {
        "title": "Eager copies",
        "steps": [
            "upload x, w, and b",
            "run affine neuron",
            "download y",
            "discard device buffers",
        ],
    },
    "resident": {
        "title": "Resident buffers",
        "steps": [
            "upload x, w, and b once",
            "run affine neuron three times",
            "keep x, w, b, and y on device",
            "download final y once",
        ],
    },
}


class PrecisionResidencyValidationError(ValueError):
    """Raised when an NN32 fixture violates the closed contract."""


def _reject_constant(value: str) -> None:
    raise PrecisionResidencyValidationError(f"non-finite JSON number: {value}")


def _object_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise PrecisionResidencyValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> Any:
    try:
        if path.stat().st_size > MAX_FILE_BYTES:
            raise PrecisionResidencyValidationError(f"{path}: file exceeds size limit")
        return json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_object_pairs,
            parse_constant=_reject_constant,
        )
    except PrecisionResidencyValidationError:
        raise
    except (
        OSError,
        UnicodeError,
        json.JSONDecodeError,
        RecursionError,
        ValueError,
    ) as exc:
        raise PrecisionResidencyValidationError(f"{path}: invalid JSON: {exc}") from exc


def _object(value: Any, keys: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        raise PrecisionResidencyValidationError(f"{context}: object key mismatch")
    return value


def _text(value: Any, context: str) -> str:
    if not isinstance(value, str) or not 1 <= len(value) <= MAX_TEXT:
        raise PrecisionResidencyValidationError(f"{context}: expected bounded text")
    return value


def _number(value: Any, context: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise PrecisionResidencyValidationError(
            f"{context}: expected finite bounded number"
        )
    try:
        result = float(value)
    except OverflowError as exc:
        raise PrecisionResidencyValidationError(
            f"{context}: expected finite bounded number"
        ) from exc
    if not math.isfinite(result) or abs(result) > MAX_ABSOLUTE_NUMBER:
        raise PrecisionResidencyValidationError(
            f"{context}: expected finite bounded number"
        )
    return result


def _integer(value: Any, minimum: int, maximum: int, context: str) -> int:
    number = _number(value, context)
    if not number.is_integer() or not minimum <= number <= maximum:
        raise PrecisionResidencyValidationError(f"{context}: expected bounded integer")
    return int(number)


def _numbers(value: Any, length: int, context: str) -> list[float]:
    if not isinstance(value, list) or len(value) != length:
        raise PrecisionResidencyValidationError(f"{context}: expected {length} numbers")
    return [_number(item, f"{context}[{index}]") for index, item in enumerate(value)]


def _integers(
    value: Any, length: int, minimum: int, maximum: int, context: str
) -> list[int]:
    if not isinstance(value, list) or len(value) != length:
        raise PrecisionResidencyValidationError(
            f"{context}: expected {length} integers"
        )
    return [
        _integer(item, minimum, maximum, f"{context}[{index}]")
        for index, item in enumerate(value)
    ]


def _strings(value: Any, length: int, context: str) -> list[str]:
    if not isinstance(value, list) or len(value) != length:
        raise PrecisionResidencyValidationError(f"{context}: expected {length} strings")
    return [_text(item, f"{context}[{index}]") for index, item in enumerate(value)]


def _round_float(value: float, code: str, context: str) -> float:
    try:
        rounded = struct.unpack(code, struct.pack(code, value))[0]
    except (OverflowError, struct.error) as exc:
        raise PrecisionResidencyValidationError(
            f"{context}: value cannot be represented"
        ) from exc
    if not math.isfinite(rounded):
        raise PrecisionResidencyValidationError(
            f"{context}: rounded value is non-finite"
        )
    return rounded


def round_binary32(value: float) -> float:
    return _round_float(value, "<f", "binary32")


def round_binary16(value: float) -> float:
    return _round_float(value, "<e", "binary16")


def execute_float_reference(
    inputs: list[float],
    weight: float,
    bias: float,
    round_value: Callable[[float], float],
) -> dict[str, list[float] | float]:
    encoded_inputs = [round_value(value) for value in inputs]
    encoded_weight = round_value(weight)
    encoded_bias = round_value(bias)
    accumulators = [round_value(value * encoded_weight) for value in encoded_inputs]
    outputs = [round_value(value + encoded_bias) for value in accumulators]
    return {
        "encoded_inputs": encoded_inputs,
        "encoded_weight": encoded_weight,
        "accumulators": accumulators,
        "outputs": outputs,
    }


def execute_int8_reference(
    inputs: list[float], weight: float, input_scale: float, weight_scale: float
) -> dict[str, list[int] | list[float] | int]:
    encoded_inputs = [round(value / input_scale) for value in inputs]
    encoded_weight = round(weight / weight_scale)
    if not all(-128 <= value <= 127 for value in [*encoded_inputs, encoded_weight]):
        raise PrecisionResidencyValidationError("int8 reference exceeds signed range")
    accumulators = [value * encoded_weight for value in encoded_inputs]
    outputs = [value * input_scale * weight_scale for value in accumulators]
    return {
        "encoded_inputs": encoded_inputs,
        "encoded_weight": encoded_weight,
        "accumulators": accumulators,
        "outputs": outputs,
    }


def _maximum_error(actual: list[float], expected: list[float]) -> float:
    return max(abs(left - right) for left, right in zip(actual, expected, strict=True))


def _same_numbers(actual: list[float], expected: list[float], context: str) -> None:
    if len(actual) != len(expected) or any(
        left != right for left, right in zip(actual, expected, strict=True)
    ):
        raise PrecisionResidencyValidationError(f"{context}: dishonest numeric oracle")


def _same_number(actual: float, expected: float, context: str) -> None:
    if actual != expected:
        raise PrecisionResidencyValidationError(f"{context}: dishonest numeric oracle")


def _resolve_reference(root: Path, lab_path: Path, reference: Any, suffix: str) -> Path:
    if not isinstance(reference, str) or not reference.endswith(suffix):
        raise PrecisionResidencyValidationError(
            "payload reference has unexpected suffix"
        )
    resolved_root = root.resolve()
    resolved = (lab_path.parent / reference).resolve()
    try:
        resolved.relative_to(resolved_root)
    except ValueError as exc:
        raise PrecisionResidencyValidationError(
            "payload reference escapes fixture root"
        ) from exc
    if not resolved.is_file() or resolved.stat().st_size > MAX_FILE_BYTES:
        raise PrecisionResidencyValidationError(
            "payload reference is missing or oversized"
        )
    return resolved


def decode_hex(path: Path, item_bytes: int, signed: bool, context: str) -> list[float]:
    try:
        payload = path.read_text(encoding="ascii").strip()
    except (OSError, UnicodeError) as exc:
        raise PrecisionResidencyValidationError(
            f"{context}: invalid payload text"
        ) from exc
    if (
        not payload
        or len(payload) > 256
        or len(payload) % (item_bytes * 2) != 0
        or HEX_RE.fullmatch(payload) is None
    ):
        raise PrecisionResidencyValidationError(
            f"{context}: expected bounded lowercase payload hex"
        )
    raw = bytes.fromhex(payload)
    if signed:
        return [float(value) for value in struct.unpack(f"<{len(raw)}b", raw)]
    code = "f" if item_bytes == 4 else "e"
    values = [
        struct.unpack(f"<{code}", raw[index : index + item_bytes])[0]
        for index in range(0, len(raw), item_bytes)
    ]
    if not all(math.isfinite(value) for value in values):
        raise PrecisionResidencyValidationError(f"{context}: non-finite payload")
    return values


def _validate_float_format(
    raw: Any,
    expected_id: str,
    inputs: list[float],
    weight: float,
    bias: float,
    reference_outputs: list[float],
    root: Path,
    lab_path: Path,
) -> dict[str, Any]:
    item = _object(
        raw,
        {
            "id",
            "title",
            "storage_bytes_per_value",
            "input_payload_file",
            "output_payload_file",
            "encoded_inputs",
            "encoded_weight",
            "accumulators",
            "outputs",
            "maximum_absolute_error",
        },
        f"format {expected_id}",
    )
    if item["id"] != expected_id:
        raise PrecisionResidencyValidationError("format roster is not canonical")
    if _text(item["title"], "format title") != CANONICAL_FORMAT_TITLES[expected_id]:
        raise PrecisionResidencyValidationError("format title is not canonical")
    width = 4 if expected_id == "binary32" else 2
    if _integer(item["storage_bytes_per_value"], 1, 8, "storage width") != width:
        raise PrecisionResidencyValidationError("float storage width is not canonical")
    round_value = round_binary32 if expected_id == "binary32" else round_binary16
    trace = execute_float_reference(inputs, weight, bias, round_value)
    encoded_inputs = _numbers(item["encoded_inputs"], 2, "encoded inputs")
    accumulators = _numbers(item["accumulators"], 2, "accumulators")
    outputs = _numbers(item["outputs"], 2, "outputs")
    _same_numbers(encoded_inputs, trace["encoded_inputs"], "encoded inputs")
    _same_number(
        _number(item["encoded_weight"], "encoded weight"),
        trace["encoded_weight"],
        "encoded weight",
    )
    _same_numbers(accumulators, trace["accumulators"], "accumulators")
    _same_numbers(outputs, trace["outputs"], "outputs")
    error = _maximum_error(outputs, reference_outputs)
    _same_number(
        _number(item["maximum_absolute_error"], "maximum error"), error, "maximum error"
    )
    suffix = ".f32le.hex" if width == 4 else ".f16le.hex"
    input_path = _resolve_reference(root, lab_path, item["input_payload_file"], suffix)
    output_path = _resolve_reference(
        root, lab_path, item["output_payload_file"], suffix
    )
    _same_numbers(
        decode_hex(input_path, width, False, "input payload"),
        encoded_inputs,
        "input payload",
    )
    _same_numbers(
        decode_hex(output_path, width, False, "output payload"),
        outputs,
        "output payload",
    )
    return json.loads(json.dumps(item))


def _validate_int8_format(
    raw: Any,
    inputs: list[float],
    weight: float,
    reference_outputs: list[float],
    root: Path,
    lab_path: Path,
) -> dict[str, Any]:
    item = _object(
        raw,
        {
            "id",
            "title",
            "storage_bytes_per_value",
            "accumulator_storage_bytes",
            "input_payload_file",
            "weight_payload_file",
            "input_scale",
            "weight_scale",
            "zero_point",
            "encoded_inputs",
            "encoded_weight",
            "accumulators",
            "outputs",
            "maximum_absolute_error",
        },
        "format symmetric_int8",
    )
    if (
        item["id"] != "symmetric_int8"
        or _integer(item["storage_bytes_per_value"], 1, 8, "storage width") != 1
        or _integer(
            item["accumulator_storage_bytes"], 1, 8, "accumulator storage width"
        )
        != 4
        or _integer(item["zero_point"], -128, 127, "zero point") != 0
    ):
        raise PrecisionResidencyValidationError("int8 contract is not canonical")
    if (
        _text(item["title"], "format title")
        != CANONICAL_FORMAT_TITLES["symmetric_int8"]
    ):
        raise PrecisionResidencyValidationError("format title is not canonical")
    input_scale = _number(item["input_scale"], "input scale")
    weight_scale = _number(item["weight_scale"], "weight scale")
    if input_scale != 0.01 or weight_scale != 0.5:
        raise PrecisionResidencyValidationError("int8 scales are not canonical")
    trace = execute_int8_reference(inputs, weight, input_scale, weight_scale)
    encoded_inputs = _integers(item["encoded_inputs"], 2, -128, 127, "encoded inputs")
    encoded_weight = _integer(item["encoded_weight"], -128, 127, "encoded weight")
    accumulators = _integers(
        item["accumulators"], 2, -(2**31), 2**31 - 1, "accumulators"
    )
    outputs = _numbers(item["outputs"], 2, "outputs")
    if (
        encoded_inputs != trace["encoded_inputs"]
        or encoded_weight != trace["encoded_weight"]
        or accumulators != trace["accumulators"]
    ):
        raise PrecisionResidencyValidationError("int8 integer oracle is dishonest")
    _same_numbers(outputs, trace["outputs"], "int8 outputs")
    error = _maximum_error(outputs, reference_outputs)
    _same_number(
        _number(item["maximum_absolute_error"], "maximum error"), error, "maximum error"
    )
    input_path = _resolve_reference(
        root, lab_path, item["input_payload_file"], ".i8.hex"
    )
    weight_path = _resolve_reference(
        root, lab_path, item["weight_payload_file"], ".i8.hex"
    )
    _same_numbers(
        decode_hex(input_path, 1, True, "input payload"),
        [float(value) for value in encoded_inputs],
        "input payload",
    )
    _same_numbers(
        decode_hex(weight_path, 1, True, "weight payload"),
        [float(encoded_weight)],
        "weight payload",
    )
    return json.loads(json.dumps(item))


def _validate_residency(raw: Any) -> dict[str, Any]:
    residency = _object(
        raw,
        {
            "dtype",
            "repeat_count",
            "upload_bytes_per_copy",
            "download_bytes_per_copy",
            "strategies",
        },
        "residency",
    )
    repeats = _integer(residency["repeat_count"], 1, 16, "repeat count")
    upload_bytes = _integer(residency["upload_bytes_per_copy"], 1, 1024, "upload bytes")
    download_bytes = _integer(
        residency["download_bytes_per_copy"], 1, 1024, "download bytes"
    )
    if (
        residency["dtype"] != "binary32"
        or repeats != 3
        or upload_bytes != 16
        or download_bytes != 8
    ):
        raise PrecisionResidencyValidationError(
            "residency byte contract is not canonical"
        )
    if (
        not isinstance(residency["strategies"], list)
        or len(residency["strategies"]) != 2
    ):
        raise PrecisionResidencyValidationError(
            "residency strategy roster is not canonical"
        )
    expected = [
        ("eager", repeats, repeats, (upload_bytes + download_bytes) * repeats),
        ("resident", 1, 1, upload_bytes + download_bytes),
    ]
    for index, (raw_strategy, oracle) in enumerate(
        zip(residency["strategies"], expected, strict=True)
    ):
        strategy = _object(
            raw_strategy,
            {
                "id",
                "title",
                "steps",
                "upload_count",
                "download_count",
                "total_transfer_bytes",
            },
            f"strategy {index}",
        )
        identifier, uploads, downloads, total = oracle
        if (
            strategy["id"] != identifier
            or _integer(strategy["upload_count"], 1, 16, "upload count") != uploads
            or _integer(strategy["download_count"], 1, 16, "download count")
            != downloads
            or _integer(
                strategy["total_transfer_bytes"], 1, MAX_FILE_BYTES, "transfer bytes"
            )
            != total
        ):
            raise PrecisionResidencyValidationError(
                "residency transfer oracle is dishonest"
            )
        canonical = CANONICAL_STRATEGIES[identifier]
        if (
            _text(strategy["title"], "strategy title") != canonical["title"]
            or _strings(strategy["steps"], 4, "strategy steps") != canonical["steps"]
        ):
            raise PrecisionResidencyValidationError(
                "residency narrative is not canonical"
            )
    return json.loads(json.dumps(residency))


def validate_document(document: Any, root: Path, lab_path: Path) -> dict[str, Any]:
    lab = _object(
        document,
        {
            "schema_version",
            "id",
            "title",
            "question",
            "graph",
            "scenario",
            "formats",
            "residency",
        },
        "lab",
    )
    if (
        _integer(lab["schema_version"], 1, 1, "schema version") != 1
        or lab["id"] != "tiny-affine-precision-residency"
    ):
        raise PrecisionResidencyValidationError("fixture identity is not canonical")
    graph = _object(lab["graph"], {"equation", "weight", "bias"}, "graph")
    weight = _number(graph["weight"], "weight")
    bias = _number(graph["bias"], "bias")
    if graph["equation"] != "y = x * w + b" or weight != 2 or bias != 0:
        raise PrecisionResidencyValidationError("affine graph is not canonical")
    scenario = _object(lab["scenario"], {"inputs", "reference_outputs"}, "scenario")
    inputs = _numbers(scenario["inputs"], 2, "inputs")
    reference_outputs = _numbers(scenario["reference_outputs"], 2, "reference outputs")
    if inputs != [1.0004, 1.0006] or reference_outputs != [
        value * weight + bias for value in inputs
    ]:
        raise PrecisionResidencyValidationError("reference arithmetic is dishonest")
    formats = lab["formats"]
    if not isinstance(formats, list) or len(formats) != 3:
        raise PrecisionResidencyValidationError("format roster is not canonical")
    normalized_formats = [
        _validate_float_format(
            formats[0],
            "binary32",
            inputs,
            weight,
            bias,
            reference_outputs,
            root,
            lab_path,
        ),
        _validate_float_format(
            formats[1],
            "binary16",
            inputs,
            weight,
            bias,
            reference_outputs,
            root,
            lab_path,
        ),
        _validate_int8_format(
            formats[2], inputs, weight, reference_outputs, root, lab_path
        ),
    ]
    return {
        "schema_version": 1,
        "id": lab["id"],
        "title": _text(lab["title"], "title"),
        "question": _text(lab["question"], "question"),
        "graph": json.loads(json.dumps(graph)),
        "scenario": json.loads(json.dumps(scenario)),
        "formats": normalized_formats,
        "residency": _validate_residency(lab["residency"]),
    }


def validate_fixture_root(root: Path = DEFAULT_FIXTURE_ROOT) -> int:
    try:
        actual_files = {
            path.relative_to(root).as_posix()
            for path in root.rglob("*")
            if path.is_file()
        }
    except OSError as exc:
        raise PrecisionResidencyValidationError(
            f"cannot enumerate fixture: {exc}"
        ) from exc
    if actual_files != EXPECTED_FILES:
        raise PrecisionResidencyValidationError("fixture file roster is not canonical")
    lab_path = root / "labs" / "00-tiny-affine.json"
    validate_document(load_json(lab_path), root, lab_path)
    load_json(root / "schema.json")
    return 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture-root", type=Path, default=DEFAULT_FIXTURE_ROOT)
    args = parser.parse_args()
    count = validate_fixture_root(args.fixture_root)
    print(f"validated {count} precision/residency lab")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
