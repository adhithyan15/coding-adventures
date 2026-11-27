"""VER07b paired-ratio analysis, backed by the shared rank-statistics package."""

from collections.abc import Sequence
from dataclasses import dataclass
from math import isfinite

from rank_statistics import median, median_interval


def _validate_factor(value: object) -> float:
    """Return a finite binary64 factor, rejecting unsupported and overflowing inputs."""
    try:
        valid_type = type(value) in (int, float)
        factor = float(value) if valid_type else 0.0
        valid = valid_type and isfinite(factor) and factor > 1.0
    except (OverflowError, TypeError, ValueError):
        valid = False
    if not valid:
        raise ValueError("minimum_factor must be finite and greater than 1")
    return factor


@dataclass(frozen=True)
class Analysis:
    ratio: float
    interval: tuple[float, float] | None
    verdict: str

    def to_dict(self) -> dict[str, object]:
        return {
            "ratio": self.ratio,
            "interval": list(self.interval) if self.interval else None,
            "verdict": self.verdict,
        }


@dataclass(frozen=True)
class Observation:
    """One paired measurement, retaining which implementation ran first."""

    baseline_ns: int
    candidate_ns: int
    order: str

    def to_dict(self) -> dict[str, object]:
        return {
            "baseline_ns": self.baseline_ns,
            "candidate_ns": self.candidate_ns,
            "order": self.order,
        }


def analyze(
    pairs: Sequence[Observation],
    confidence_percent: int = 95,
    minimum_factor: float = 1.05,
) -> Analysis:
    """Analyze candidate/baseline ratios from ordered paired observations."""
    if not pairs:
        raise ValueError("pairs must not be empty")
    if len(pairs) > 10_000:
        raise ValueError("at most 10000 pairs are supported")
    if type(confidence_percent) is not int or not 1 <= confidence_percent <= 99:
        raise ValueError("confidence_percent must be an integer from 1 through 99")
    minimum_factor = _validate_factor(minimum_factor)
    ratios = []
    for observation in pairs:
        if (
            not isinstance(observation, Observation)
            or not isinstance(observation.order, str)
            or observation.order not in {"baseline-first", "candidate-first"}
        ):
            raise ValueError("each observation must have a valid order")
        baseline, candidate = observation.baseline_ns, observation.candidate_ns
        if (
            type(baseline) is not int
            or type(candidate) is not int
            or not 1 <= baseline <= 2**63 - 1
            or not 1 <= candidate <= 2**63 - 1
        ):
            raise ValueError("pair durations must be positive signed 64-bit integers")
        # Converting operands first makes binary64 rounding consistent with Go.
        ratios.append(float(candidate) / float(baseline))
    ratio = median(ratios)
    interval = median_interval(ratios, confidence_percent)
    verdict = "inconclusive"
    if interval is not None:
        low, high = interval
        if high < 1 / minimum_factor:
            verdict = "faster"
        elif low > minimum_factor:
            verdict = "slower"
        elif low >= 1 / minimum_factor and high <= minimum_factor:
            verdict = "equivalent"
    return Analysis(ratio, interval, verdict)
