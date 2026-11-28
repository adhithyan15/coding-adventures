"""Compare repeated bounded insertion with Floyd construction."""

from __future__ import annotations

import argparse
import json
import platform
import random
import sys
import time
from collections.abc import Callable, Sequence
from contextlib import redirect_stderr
from typing import TextIO

from heap import BoundedHeap
from microbenchmark import Config, collect

MAX_INPUT_SIZE = 8192
WORKLOAD = "heap-construction-repeated-push-vs-floyd"
CHECKSUM_MULTIPLIER = 1009
HeapBuilder = Callable[[Sequence[int]], BoundedHeap[int]]


def _platform_metadata() -> tuple[str, str]:
    """Return interpreter platform constants without host OS probing.

    `platform.system()` and `platform.machine()` can spawn commands, read
    environment variables, or query the registry on some supported hosts.
    `sys.platform` and CPython's `_multiarch` are compile/runtime identifiers;
    implementations without `_multiarch` report the truthful fallback.
    """
    os_name = sys.platform
    if not isinstance(os_name, str) or not os_name:
        os_name = "unknown"
    architecture = getattr(sys.implementation, "_multiarch", None)
    if not isinstance(architecture, str) or not architecture:
        architecture = "unknown"
    return os_name, architecture


def build_by_repeated_push(values: Sequence[int]) -> BoundedHeap[int]:
    """Construct a new bounded heap by inserting every input value."""
    heap = BoundedHeap(len(values), _less_int)
    for value in values:
        heap.push(value)
    return heap


def build_by_floyd(values: Sequence[int]) -> BoundedHeap[int]:
    """Construct a new bounded heap with the package's Floyd builder."""
    return BoundedHeap.from_iterable(values, len(values), _less_int)


def _less_int(left: int, right: int) -> bool:
    return left < right


def make_input(size: int, seed: int) -> tuple[int, ...]:
    """Prepare one immutable input shared by both methods."""
    generator = random.Random(seed)
    return tuple(generator.randint(-1_000_000, 1_000_000) for _ in range(size))


def verify_builder(builder: HeapBuilder, values: Sequence[int]) -> None:
    """Drain one untimed instance and compare all outputs with ``sorted``."""
    heap = builder(values)
    expected = sorted(values)
    actual = [heap.pop() for _ in range(len(heap))]
    if actual != expected:
        raise ValueError("heap construction did not preserve sorted input")
    if len(heap) != 0:
        raise ValueError("heap drain did not empty the constructed heap")


def checksum_for(values: Sequence[int]) -> int:
    """Return a stable per-instance checksum that needs only root and length."""
    if not values:
        raise ValueError("benchmark input must not be empty")
    return CHECKSUM_MULTIPLIER * len(values) + min(values)


def make_batch(builder: HeapBuilder, values: Sequence[int]) -> Callable[[int], int]:
    """Build ``batch`` independent heaps, observing each without draining it."""

    def run(batch: int) -> int:
        checksum = 0
        for _ in range(batch):
            heap = builder(values)
            checksum += CHECKSUM_MULTIPLIER * len(heap) + heap.peek()
            # Rebinding on the next iteration (and returning from this callback)
            # releases temporary heaps under the runtime's ordinary object policy.
        return checksum

    return run


def parser() -> argparse.ArgumentParser:
    defaults = Config()
    result = argparse.ArgumentParser(
        description=(
            "Compare repeated checked heap insertion with bottom-up Floyd "
            "construction over the same input."
        )
    )
    result.add_argument("--size", type=int, default=1024)
    result.add_argument("--pairs", type=int, default=defaults.pairs)
    result.add_argument("--confidence", type=int, default=defaults.confidence_percent)
    result.add_argument("--factor", type=float, default=defaults.minimum_factor)
    result.add_argument("--seed", type=int, default=defaults.seed)
    result.add_argument("--target-ns", type=int, default=defaults.target_ns)
    result.add_argument("--max-batch", type=int, default=defaults.max_batch)
    result.add_argument("--revision", default="unknown")
    return result


def run(
    argv: Sequence[str], stdout: TextIO, stderr: TextIO, clock: Callable[[], int]
) -> int:
    """Run one benchmark invocation with injected output and monotonic clock."""
    argument_parser = parser()
    with redirect_stderr(stderr):
        arguments = argument_parser.parse_args(list(argv))
    if not 1 <= arguments.size <= MAX_INPUT_SIZE:
        with redirect_stderr(stderr):
            argument_parser.error(f"size must be from 1 through {MAX_INPUT_SIZE}")
    if not arguments.revision or "\0" in arguments.revision:
        with redirect_stderr(stderr):
            argument_parser.error("revision must be nonempty and contain no NUL")

    config = Config(
        arguments.pairs,
        arguments.confidence,
        arguments.factor,
        arguments.seed,
        arguments.target_ns,
        arguments.max_batch,
    )
    try:
        config.validate()
        values = make_input(arguments.size, config.seed)
        expected_checksum = checksum_for(values)
        baseline = make_batch(build_by_repeated_push, values)
        candidate = make_batch(build_by_floyd, values)

        # These complete drains and sorted-oracle comparisons stay outside the
        # collector's clock. They check both implementations before and after.
        verify_builder(build_by_repeated_push, values)
        verify_builder(build_by_floyd, values)
        report = collect(baseline, candidate, expected_checksum, config, clock)
        verify_builder(build_by_repeated_push, values)
        verify_builder(build_by_floyd, values)
    except (ValueError, OverflowError) as error:
        with redirect_stderr(stderr):
            argument_parser.error(str(error))

    document = report.to_dict()
    os_name, architecture = _platform_metadata()
    document.update(
        {
            "workload": WORKLOAD,
            "invocation": [sys.argv[0], *argv],
            "revision": arguments.revision,
            "runtime": platform.python_version(),
            "os": os_name,
            "arch": architecture,
        }
    )
    stdout.write(json.dumps(document, indent=2, sort_keys=True) + "\n")
    return 0


def main(argv: Sequence[str] | None = None) -> int:
    """Console-script entry point."""
    actual = list(sys.argv[1:] if argv is None else argv)
    return run(actual, sys.stdout, sys.stderr, time.monotonic_ns)


if __name__ == "__main__":
    raise SystemExit(main())
