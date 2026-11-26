"""Deterministic paired batch collection with injectable monotonic nanosecond clock."""

import random
import time
from collections.abc import Callable
from dataclasses import dataclass
from math import isfinite

from .analysis import Analysis, Observation, _validate_factor, analyze

Batch = Callable[[int], int]
MAX_I64 = 2**63 - 1


def _finite_json_number(value: object) -> bool:
    """Accept finite JSON int/float values without overflowing on large integers."""
    if type(value) is int:
        return True
    return type(value) is float and isfinite(value)


@dataclass(frozen=True)
class Config:
    pairs: int = 40
    confidence_percent: int = 95
    minimum_factor: float = 1.05
    seed: int = 0
    target_ns: int = 100_000
    max_batch: int = 65_536

    def validate(self) -> None:
        if type(self.pairs) is not int or not 1 <= self.pairs <= 10_000:
            raise ValueError("pairs must be from 1 through 10000")
        if (
            type(self.confidence_percent) is not int
            or not 1 <= self.confidence_percent <= 99
        ):
            raise ValueError("confidence must be an integer from 1 through 99")
        object.__setattr__(
            self, "minimum_factor", _validate_factor(self.minimum_factor)
        )
        for name, value in (
            ("target_ns", self.target_ns),
            ("max_batch", self.max_batch),
        ):
            if type(value) is not int or not 1 <= value <= MAX_I64:
                raise ValueError(f"{name} must be a positive signed 64-bit integer")
        if type(self.seed) is not int or not -(2**63) <= self.seed <= MAX_I64:
            raise ValueError("seed must be a signed 64-bit integer")


@dataclass(frozen=True)
class Report:
    config: Config
    batch_size: int
    pairs: tuple[Observation, ...]
    analysis: Analysis

    def to_dict(self) -> dict[str, object]:
        return {
            "config": {
                "pairs": self.config.pairs,
                "confidence_percent": self.config.confidence_percent,
                "minimum_factor": self.config.minimum_factor,
                "seed": self.config.seed,
                "target_ns": self.config.target_ns,
                "max_batch": self.config.max_batch,
            },
            "batch_size": self.batch_size,
            "pairs": [pair.to_dict() for pair in self.pairs],
            "analysis": self.analysis.to_dict(),
        }

    @classmethod
    def from_dict(cls, value: dict[str, object]) -> "Report":
        """Rebuild and reanalyze a raw JSON report, validating its observation shape."""
        try:
            if "workload" in value:
                workload = value["workload"]
                if (
                    not isinstance(workload, str)
                    or not workload.strip(" \t\n\r\v\f")
                    or "\0" in workload
                    or any(0xD800 <= ord(char) <= 0xDFFF for char in workload)
                ):
                    raise ValueError("invalid workload identifier")
            raw_config = value["config"]
            if not isinstance(raw_config, dict):
                raise ValueError("config must be an object")
            config = Config(
                raw_config["pairs"],
                raw_config["confidence_percent"],
                raw_config["minimum_factor"],
                raw_config["seed"],
                raw_config["target_ns"],
                raw_config["max_batch"],
            )
            config.validate()
            batch_size = value["batch_size"]
            if type(batch_size) is not int or not 1 <= batch_size <= config.max_batch:
                raise ValueError("batch_size must be positive and within max_batch")
            rows = value["pairs"]
            if not isinstance(rows, list) or len(rows) != config.pairs:
                raise ValueError("report pair count does not match config")
            observations = []
            for row in rows:
                if (
                    not isinstance(row, dict)
                    or not isinstance(row.get("order"), str)
                    or row.get("order") not in {"baseline-first", "candidate-first"}
                ):
                    raise ValueError("invalid order value")
                bns, cns = row["baseline_ns"], row["candidate_ns"]
                if (
                    type(bns) is not int
                    or type(cns) is not int
                    or not 1 <= bns <= MAX_I64
                    or not 1 <= cns <= MAX_I64
                ):
                    raise ValueError(
                        "durations must be positive signed 64-bit integers"
                    )
                observations.append(Observation(bns, cns, row["order"]))
            analysis = analyze(
                observations, config.confidence_percent, config.minimum_factor
            )
            raw_analysis = value["analysis"]
            if not isinstance(raw_analysis, dict):
                raise ValueError("analysis must be an object")
            interval = raw_analysis.get("interval")
            if interval is not None and (
                not isinstance(interval, list) or len(interval) != 2
            ):
                raise ValueError("interval must be a two-item array or null")
            if interval is not None and any(
                not _finite_json_number(bound) for bound in interval
            ):
                raise ValueError("interval bounds must be finite numbers")
            ratio = raw_analysis.get("ratio")
            if not _finite_json_number(ratio) or not isinstance(
                raw_analysis.get("verdict"), str
            ):
                raise ValueError("analysis fields have invalid types")
            if raw_analysis != analysis.to_dict():
                raise ValueError("analysis does not match raw observations")
            return cls(config, batch_size, tuple(observations), analysis)
        except (KeyError, TypeError) as error:
            raise ValueError("malformed report") from error


def collect(
    baseline: Batch,
    candidate: Batch,
    expected_checksum: int,
    config: Config | None = None,
    clock: Callable[[], int] = time.monotonic_ns,
) -> Report:
    """Warm both functions, calibrate a common size, then measure shuffled pairs."""
    if config is None:
        config = Config()
    config.validate()
    if (
        type(expected_checksum) is not int
        or not -(2**63) <= expected_checksum <= MAX_I64
    ):
        raise ValueError("expected checksum must fit signed 64-bit integer")

    def verify(fn: Batch, size: int) -> None:
        value = fn(size)
        if type(value) is not int or not -(2**63) <= value <= MAX_I64:
            raise ValueError("checksum must fit signed 64-bit integer")
        if value != expected_checksum * size:
            raise ValueError("checksum mismatch")

    verify(baseline, 1)
    verify(candidate, 1)
    last = None

    def timed(fn: Batch, size: int) -> int:
        nonlocal last
        expected_batch = expected_checksum * size
        if not -(2**63) <= expected_batch <= MAX_I64:
            raise ValueError("batch checksum arithmetic must fit signed 64-bit integer")
        start = clock()
        if (
            type(start) is not int
            or not 0 <= start <= MAX_I64
            or (last is not None and start < last)
        ):
            raise ValueError("clock must be nonnegative and nonregressing")
        last = start
        checksum = fn(size)
        end = clock()
        if (
            type(end) is not int
            or not 0 <= end <= MAX_I64
            or end < start
            or (last is not None and end < last)
        ):
            raise ValueError("clock must be nonnegative and nonregressing")
        last = end
        if type(checksum) is not int or not -(2**63) <= checksum <= MAX_I64:
            raise ValueError("checksum must fit signed 64-bit integer")
        if checksum != expected_batch:
            raise ValueError("checksum mismatch")
        return end - start

    batch_size = 1
    while batch_size <= config.max_batch:
        left = timed(baseline, batch_size)
        right = timed(candidate, batch_size)
        if left >= config.target_ns and right >= config.target_ns:
            break
        if batch_size == config.max_batch:
            raise ValueError("calibration limit exhausted")
        batch_size = min(batch_size * 2, config.max_batch)
    schedule = ["baseline-first"] * ((config.pairs + 1) // 2) + ["candidate-first"] * (
        config.pairs // 2
    )
    random.Random(config.seed).shuffle(schedule)
    observations = []
    for order in schedule:
        if order == "baseline-first":
            bns = timed(baseline, batch_size)
            cns = timed(candidate, batch_size)
        else:
            cns = timed(candidate, batch_size)
            bns = timed(baseline, batch_size)
        if bns <= 0 or cns <= 0:
            raise ValueError("measured durations must be positive")
        observations.append(Observation(bns, cns, order))
    pairs = tuple(observations)
    return Report(
        config,
        batch_size,
        pairs,
        analyze(pairs, config.confidence_percent, config.minimum_factor),
    )
