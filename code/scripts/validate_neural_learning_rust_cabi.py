#!/usr/bin/env python3
"""Validate the NN35 catalog and execute the compiled Rust C ABI."""

from __future__ import annotations

import ctypes
import json
import math
import os
import signal
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = (
    REPO_ROOT / "code" / "specs" / "fixtures" / "neural-learning-rust-cabi-v1"
)
RUST_WORKSPACE = REPO_ROOT / "code" / "packages" / "rust"
EXPECTED_FILES = {"catalog.json", "schema.json", "README.md", "CHANGELOG.md"}
MAXIMUM_FILE_BYTES = 1_000_000
ABI_VERSION = 0x0001_0000
MAXIMUM_BUILD_LOG_BYTES = 32_768
STATUS_MESSAGES = (
    "ok",
    "null pointer",
    "input count must be positive",
    "contribution buffer is too small",
    "input count is too large",
    "all inputs and arithmetic results must be finite",
    "Rust panic was contained",
    "mutable output buffers must not overlap other buffers",
    "pointer is not aligned for a double",
)
STATUS_SYMBOLS = (
    "NEURAL_LEARNING_OK",
    "NEURAL_LEARNING_NULL_POINTER",
    "NEURAL_LEARNING_EMPTY_INPUT",
    "NEURAL_LEARNING_BUFFER_TOO_SMALL",
    "NEURAL_LEARNING_VALUE_TOO_LARGE",
    "NEURAL_LEARNING_NON_FINITE",
    "NEURAL_LEARNING_PANIC",
    "NEURAL_LEARNING_OVERLAPPING_BUFFER",
    "NEURAL_LEARNING_MISALIGNED_POINTER",
)
ABI_FUNCTIONS = (
    "uint32_t neural_learning_abi_version(void)",
    "const char *neural_learning_status_message_v1(uint32_t status)",
    "uint32_t neural_learning_weighted_sum_f64_v1(const double *inputs, const double *weights, uint64_t input_count, double bias, double *contributions_out, uint64_t contributions_capacity, double *prediction_out)",
)
ABI_RULES = (
    "all lengths and status values use fixed-width unsigned integers",
    "callers own every input and output buffer",
    "no Rust allocation or Rust type crosses the boundary",
    "mutable outputs do not overlap inputs or one another",
    "a Rust panic is caught and becomes status 6",
)
FAILURE_PROBES = (
    ("null-input", 1),
    ("empty-input", 2),
    ("short-output", 3),
    ("non-finite", 5),
    ("overlapping-output", 7),
)


class CAbiValidationError(ValueError):
    """Raised when the NN35 document or compiled ABI breaks its contract."""


@dataclass(frozen=True)
class AbiReceipt:
    """Closed execution evidence from the real dynamic library."""

    version: int
    status: int
    contributions: tuple[float, float]
    prediction: float
    failure_statuses: tuple[int, ...]


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise CAbiValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    try:
        size = path.stat().st_size
        if size <= 0 or size > MAXIMUM_FILE_BYTES:
            raise CAbiValidationError(f"{path}: invalid file size")
        value = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_reject_duplicate_pairs,
            parse_constant=lambda token: (_ for _ in ()).throw(
                CAbiValidationError(f"non-finite JSON number: {token}")
            ),
        )
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise CAbiValidationError(f"{path}: {error}") from error
    if not isinstance(value, dict):
        raise CAbiValidationError(f"{path}: top-level value must be an object")
    return value


def _object(value: Any, keys: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        raise CAbiValidationError(f"{context}: unexpected shape")
    return value


def _repo_file(value: Any, expected: str, context: str) -> Path:
    if value != expected or not isinstance(value, str) or "\\" in value:
        raise CAbiValidationError(f"{context}: path is not canonical")
    relative = PurePosixPath(value)
    if relative.is_absolute() or "." in relative.parts or ".." in relative.parts:
        raise CAbiValidationError(f"{context}: path is not normalized")
    resolved = (REPO_ROOT / Path(*relative.parts)).resolve()
    try:
        resolved.relative_to(REPO_ROOT.resolve())
    except ValueError as error:
        raise CAbiValidationError(f"{context}: path escapes the repository") from error
    if not resolved.is_file():
        raise CAbiValidationError(f"{context}: file does not exist")
    return resolved


def _finite_number(value: Any, context: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise CAbiValidationError(f"{context}: expected a number")
    result = float(value)
    if not math.isfinite(result):
        raise CAbiValidationError(f"{context}: expected a finite number")
    return result


def validate_catalog_document(document: Any) -> dict[str, Any]:
    catalog = _object(
        document,
        {
            "schema_version",
            "id",
            "title",
            "question",
            "source_fixture",
            "abi",
            "statuses",
            "hand_check",
            "failure_probes",
        },
        "catalog",
    )
    if catalog["schema_version"] != 1 or catalog["id"] != "weighted-neuron-rust-c-abi":
        raise CAbiValidationError("catalog identity is not canonical")
    for key in ("title", "question"):
        value = catalog[key]
        if not isinstance(value, str) or not value.strip() or len(value) > 300:
            raise CAbiValidationError(f"catalog.{key}: invalid text")
    source_fixture = _repo_file(
        catalog["source_fixture"],
        "code/specs/fixtures/neural-learning-v1/labs/00-weighted-neuron.json",
        "source_fixture",
    )

    abi = _object(
        catalog["abi"],
        {
            "version_number",
            "version_hex",
            "header",
            "crate",
            "library_base_name",
            "functions",
            "rules",
        },
        "abi",
    )
    if abi["version_number"] != ABI_VERSION or abi["version_hex"] != "0x00010000":
        raise CAbiValidationError("abi version is not canonical")
    header = _repo_file(
        abi["header"],
        "code/packages/rust/neural-learning-capi/include/neural_learning.h",
        "abi.header",
    )
    crate = _repo_file(
        abi["crate"],
        "code/packages/rust/neural-learning-capi/Cargo.toml",
        "abi.crate",
    )
    if abi["library_base_name"] != "neural_learning_capi":
        raise CAbiValidationError("library name is not canonical")
    if abi["functions"] != list(ABI_FUNCTIONS):
        raise CAbiValidationError("abi function declarations are not canonical")
    header_text = header.read_text(encoding="utf-8")
    normalized_header = "".join(header_text.split())
    for function in ABI_FUNCTIONS:
        if f"{''.join(function.split())};" not in normalized_header:
            raise CAbiValidationError(f"public header declaration drifted: {function}")
    if abi["rules"] != list(ABI_RULES):
        raise CAbiValidationError("abi ownership rules are not canonical")
    expected_macros = {
        "NEURAL_LEARNING_ABI_VERSION_V1": "0x00010000",
        **{symbol: str(code) for code, symbol in enumerate(STATUS_SYMBOLS)},
    }
    for symbol, value in expected_macros.items():
        if f"#define {symbol} UINT32_C({value})" not in header_text:
            raise CAbiValidationError(f"public header macro drifted: {symbol}")

    statuses = catalog["statuses"]
    if not isinstance(statuses, list) or len(statuses) != len(STATUS_MESSAGES):
        raise CAbiValidationError("status table is incomplete")
    for code, (entry, message, symbol) in enumerate(
        zip(statuses, STATUS_MESSAGES, STATUS_SYMBOLS, strict=True)
    ):
        status = _object(entry, {"code", "symbol", "message"}, f"statuses[{code}]")
        if (
            status["code"] != code
            or status["message"] != message
            or status["symbol"] != symbol
        ):
            raise CAbiValidationError("status table is not canonical")

    hand = _object(
        catalog["hand_check"],
        {
            "inputs",
            "weights",
            "bias",
            "contributions_capacity",
            "expected_status",
            "expected_contributions",
            "expected_prediction",
            "absolute_tolerance",
        },
        "hand_check",
    )
    if hand["inputs"] != [2.0, -1.0] or hand["weights"] != [0.5, -0.25]:
        raise CAbiValidationError("hand-check operands are not canonical")
    contributions = [
        _finite_number(hand["inputs"][index], f"inputs[{index}]")
        * _finite_number(hand["weights"][index], f"weights[{index}]")
        for index in range(2)
    ]
    prediction = sum(contributions, _finite_number(hand["bias"], "bias"))
    tolerance = _finite_number(hand["absolute_tolerance"], "absolute_tolerance")
    if (
        hand["contributions_capacity"] != 2
        or hand["expected_status"] != 0
        or hand["expected_contributions"] != contributions
        or hand["expected_prediction"] != prediction
        or tolerance <= 0
    ):
        raise CAbiValidationError("hand-check arithmetic is dishonest")

    source = _object(
        load_json(source_fixture),
        {
            "schema_version",
            "id",
            "title",
            "stage",
            "question",
            "concepts",
            "model",
            "dataset",
            "training",
            "expected",
        },
        "source fixture",
    )
    if source["schema_version"] != 1 or source["id"] != "weighted-neuron-forward" or source["stage"] != "forward" or source["training"] is not None:
        raise CAbiValidationError("source fixture identity is not canonical")
    model = _object(source["model"], {"kind", "input_count", "layers"}, "source model")
    if model["kind"] != "single-neuron" or model["input_count"] != 2 or not isinstance(model["layers"], list) or len(model["layers"]) != 1:
        raise CAbiValidationError("source fixture model is not canonical")
    layer = _object(model["layers"][0], {"name", "weights", "biases", "activation"}, "source layer")
    if layer["name"] != "output" or layer["activation"] != "identity" or layer["weights"] != [[0.5], [-0.25]] or layer["biases"] != [0.1]:
        raise CAbiValidationError("source fixture layer disagrees with the ABI hand check")
    dataset = _object(source["dataset"], {"input_labels", "target_labels", "rows"}, "source dataset")
    if not isinstance(dataset["rows"], list) or len(dataset["rows"]) != 1:
        raise CAbiValidationError("source fixture dataset is not canonical")
    row = _object(dataset["rows"][0], {"label", "input", "target"}, "source row")
    expected = _object(source["expected"], {"absolute_tolerance", "forward", "first_step"}, "source expected")
    if not isinstance(expected["forward"], list) or len(expected["forward"]) != 1:
        raise CAbiValidationError("source fixture expectation is not canonical")
    forward = _object(expected["forward"][0], {"row", "prediction"}, "source forward")
    if (
        row["label"] != "worked example"
        or row["input"] != hand["inputs"]
        or forward["row"] != row["label"]
        or forward["prediction"] != [hand["expected_prediction"]]
        or expected["absolute_tolerance"] != hand["absolute_tolerance"]
        or expected["first_step"] is not None
    ):
        raise CAbiValidationError("source fixture disagrees with the ABI hand check")

    probes = catalog["failure_probes"]
    if not isinstance(probes, list) or len(probes) != len(FAILURE_PROBES):
        raise CAbiValidationError("failure probes are incomplete")
    for index, (probe, expected) in enumerate(zip(probes, FAILURE_PROBES, strict=True)):
        item = _object(probe, {"id", "expected_status", "outputs_unchanged"}, f"failure_probes[{index}]")
        if (item["id"], item["expected_status"]) != expected or item["outputs_unchanged"] is not True:
            raise CAbiValidationError("failure probe is not canonical")

    return {
        **catalog,
        "source_fixture": source_fixture,
        "header": header,
        "crate": crate,
    }


def validate_fixture_root(root: Path = DEFAULT_FIXTURE_ROOT) -> dict[str, Any]:
    try:
        files = {path.relative_to(root).as_posix() for path in root.rglob("*") if path.is_file()}
    except OSError as error:
        raise CAbiValidationError(f"cannot enumerate fixture root: {error}") from error
    if files != EXPECTED_FILES:
        raise CAbiValidationError("fixture file roster is not canonical")
    schema = load_json(root / "schema.json")
    if (
        schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema"
        or schema.get("$id")
        != "https://coding-adventures.dev/schemas/neural-learning-rust-cabi-v1.json"
    ):
        raise CAbiValidationError("schema identity is not canonical")
    return validate_catalog_document(load_json(root / "catalog.json"))


def _library_path() -> Path:
    if os.name == "nt":
        name = "neural_learning_capi.dll"
    elif sys.platform == "darwin":
        name = "libneural_learning_capi.dylib"
    else:
        name = "libneural_learning_capi.so"
    return RUST_WORKSPACE / "target" / "debug" / name


def build_library() -> Path:
    command = ["cargo", "build", "-p", "neural-learning-capi"]
    creation_flags = 0
    start_new_session = os.name != "nt"
    if os.name == "nt":
        creation_flags = getattr(subprocess, "CREATE_NEW_PROCESS_GROUP", 0)
    try:
        with tempfile.TemporaryFile() as build_log:
            process = subprocess.Popen(
                command,
                cwd=RUST_WORKSPACE,
                stdin=subprocess.DEVNULL,
                stdout=build_log,
                stderr=subprocess.STDOUT,
                creationflags=creation_flags,
                start_new_session=start_new_session,
            )
            try:
                return_code = process.wait(timeout=120)
            except subprocess.TimeoutExpired as error:
                if os.name == "nt":
                    try:
                        terminated = subprocess.run(
                            ["taskkill", "/PID", str(process.pid), "/T", "/F"],
                            stdin=subprocess.DEVNULL,
                            stdout=subprocess.DEVNULL,
                            stderr=subprocess.DEVNULL,
                            timeout=10,
                            check=False,
                        )
                    except (OSError, subprocess.SubprocessError):
                        terminated = None
                    if terminated is None or terminated.returncode != 0:
                        process.kill()
                else:
                    try:
                        os.killpg(process.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                process.wait(timeout=10)
                build_log.flush()
                build_log.seek(0, os.SEEK_END)
                size = build_log.tell()
                build_log.seek(max(0, size - MAXIMUM_BUILD_LOG_BYTES))
                tail = build_log.read().decode("utf-8", errors="replace").strip()
                detail = f"\nCargo output tail:\n{tail}" if tail else ""
                raise CAbiValidationError(
                    f"Rust C ABI build timed out after 120 seconds{detail}"
                ) from error
            build_log.flush()
            build_log.seek(0, os.SEEK_END)
            size = build_log.tell()
            build_log.seek(max(0, size - MAXIMUM_BUILD_LOG_BYTES))
            build_tail = build_log.read().decode("utf-8", errors="replace").strip()
    except (OSError, subprocess.SubprocessError) as error:
        raise CAbiValidationError(f"Rust C ABI build could not complete: {error}") from error
    if return_code != 0:
        detail = f"\nCargo output tail:\n{build_tail}" if build_tail else ""
        raise CAbiValidationError(
            f"Rust C ABI build failed with exit {return_code}{detail}"
        )
    library = _library_path()
    if not library.is_file():
        raise CAbiValidationError("Rust C ABI build did not produce the expected library")
    return library


def _load_library(path: Path) -> ctypes.CDLL:
    try:
        library = ctypes.CDLL(str(path))
    except OSError as error:
        raise CAbiValidationError(f"cannot load Rust C ABI library: {error}") from error
    double_pointer = ctypes.POINTER(ctypes.c_double)
    library.neural_learning_abi_version.argtypes = []
    library.neural_learning_abi_version.restype = ctypes.c_uint32
    library.neural_learning_status_message_v1.argtypes = [ctypes.c_uint32]
    library.neural_learning_status_message_v1.restype = ctypes.c_char_p
    library.neural_learning_weighted_sum_f64_v1.argtypes = [
        double_pointer,
        double_pointer,
        ctypes.c_uint64,
        ctypes.c_double,
        double_pointer,
        ctypes.c_uint64,
        double_pointer,
    ]
    library.neural_learning_weighted_sum_f64_v1.restype = ctypes.c_uint32
    return library


def execute_abi(catalog: dict[str, Any], library_path: Path | None = None) -> AbiReceipt:
    library = _load_library(library_path or build_library())
    version = int(library.neural_learning_abi_version())
    if version != ABI_VERSION:
        raise CAbiValidationError(f"library reported unexpected ABI version {version}")
    for code, expected in enumerate(STATUS_MESSAGES):
        raw = library.neural_learning_status_message_v1(code)
        if raw is None or raw.decode("utf-8", errors="strict") != expected:
            raise CAbiValidationError(f"library status message {code} is dishonest")

    hand = catalog["hand_check"]
    vector_type = ctypes.c_double * 2
    inputs = vector_type(*hand["inputs"])
    weights = vector_type(*hand["weights"])
    contributions = vector_type(91.0, 92.0)
    prediction = ctypes.c_double(93.0)
    status = int(
        library.neural_learning_weighted_sum_f64_v1(
            inputs,
            weights,
            2,
            hand["bias"],
            contributions,
            2,
            ctypes.byref(prediction),
        )
    )
    if status != 0 or list(contributions) != hand["expected_contributions"] or prediction.value != hand["expected_prediction"]:
        raise CAbiValidationError("compiled library disagrees with the hand calculation")

    failure_statuses: list[int] = []
    for probe_id, expected_status in FAILURE_PROBES:
        probe_inputs = vector_type(2.0, -1.0)
        probe_weights = vector_type(0.5, -0.25)
        probe_contributions = vector_type(91.0, 92.0)
        probe_prediction = ctypes.c_double(93.0)
        call_inputs = probe_inputs
        count = 2
        capacity = 2
        if probe_id == "null-input":
            call_inputs = ctypes.POINTER(ctypes.c_double)()
        elif probe_id == "empty-input":
            count = 0
        elif probe_id == "short-output":
            capacity = 1
        elif probe_id == "non-finite":
            probe_inputs[0] = math.inf
        elif probe_id == "overlapping-output":
            probe_contributions = probe_inputs
        contribution_sentinel = list(probe_contributions)
        probe_status = int(
            library.neural_learning_weighted_sum_f64_v1(
                call_inputs,
                probe_weights,
                count,
                0.1,
                probe_contributions,
                capacity,
                ctypes.byref(probe_prediction),
            )
        )
        if probe_status != expected_status or list(probe_contributions) != contribution_sentinel or probe_prediction.value != 93.0:
            raise CAbiValidationError(f"failure probe {probe_id} broke its closed-output contract")
        failure_statuses.append(probe_status)

    return AbiReceipt(
        version=version,
        status=status,
        contributions=(contributions[0], contributions[1]),
        prediction=prediction.value,
        failure_statuses=tuple(failure_statuses),
    )


def main() -> int:
    try:
        catalog = validate_fixture_root()
        receipt = execute_abi(catalog)
    except CAbiValidationError as error:
        raise SystemExit(f"neural-learning Rust C ABI invalid: {error}") from error
    print(
        "neural-learning Rust C ABI v1 passed: "
        f"status {receipt.status}, contributions {list(receipt.contributions)}, "
        f"prediction {receipt.prediction:g}, failure statuses {list(receipt.failure_statuses)}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
