"""Command-line indexed-array workload used by the VER07 reference experiment."""

import argparse
import json
import platform
import sys

from .collector import Config, collect

ARRAY = [(17 * index + 3) % 251 for index in range(4096)]
ONE_PASS = sum(ARRAY)


def indexed(batch: int) -> int:
    total = 0
    for _ in range(batch):
        for index in range(len(ARRAY)):
            total += ARRAY[index]
    return total


def iterated(batch: int) -> int:
    total = 0
    for _ in range(batch):
        for value in ARRAY:
            total += value
    return total


def parser() -> argparse.ArgumentParser:
    defaults = Config()
    result = argparse.ArgumentParser(
        description="Compare indexed and native-iteration array traversal."
    )
    result.add_argument("--pairs", type=int, default=defaults.pairs)
    result.add_argument("--confidence", type=int, default=defaults.confidence_percent)
    result.add_argument("--factor", type=float, default=defaults.minimum_factor)
    result.add_argument("--seed", type=int, default=defaults.seed)
    result.add_argument("--target-ns", type=int, default=defaults.target_ns)
    result.add_argument("--max-batch", type=int, default=defaults.max_batch)
    result.add_argument("--revision", default="unknown")
    return result


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    config = Config(
        args.pairs,
        args.confidence,
        args.factor,
        args.seed,
        args.target_ns,
        args.max_batch,
    )
    try:
        report = collect(indexed, iterated, ONE_PASS, config)
    except (ValueError, OverflowError) as error:
        parser().error(str(error))
    document = report.to_dict()
    document.update(
        {
            "workload": "indexed-array-sum-4096",
            "invocation": [sys.argv[0], *(sys.argv[1:] if argv is None else argv)],
            "revision": args.revision,
            "runtime": platform.python_version(),
            "os": platform.system(),
            "arch": platform.machine(),
        }
    )
    rendered = json.dumps(document, indent=2, sort_keys=True) + "\n"
    sys.stdout.write(rendered)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
