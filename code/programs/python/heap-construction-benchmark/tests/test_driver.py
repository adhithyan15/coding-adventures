"""Collector contract and construction-workload checks."""

from __future__ import annotations

import io
import json
import random
from collections.abc import Callable
from types import SimpleNamespace

import pytest
from microbenchmark import Report

from heap_construction_benchmark import driver


def ticking_clock(step: int = 1) -> Callable[[], int]:
    tick = 0

    def now() -> int:
        nonlocal tick
        tick += step
        return tick

    return now


def test_builders_match_same_input_and_checksum_without_draining_inputs() -> None:
    values = (9, -4, 9, 2, 100, -100)
    snapshot = values[:]
    repeated = driver.build_by_repeated_push(values)
    floyd = driver.build_by_floyd(values)
    assert repeated.to_array() != []
    assert floyd.to_array() != []
    assert [repeated.pop() for _ in range(len(repeated))] == sorted(values)
    assert [floyd.pop() for _ in range(len(floyd))] == sorted(values)
    assert values == snapshot

    baseline = driver.make_batch(driver.build_by_repeated_push, values)
    candidate = driver.make_batch(driver.build_by_floyd, values)
    per_instance = driver.checksum_for(values)
    assert baseline(4) == candidate(4) == per_instance * 4


def test_driver_uses_fake_clock_and_emits_replayable_standard_report(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    stdout = io.StringIO()
    stderr = io.StringIO()
    argv = [
        "--size",
        "12",
        "--pairs",
        "5",
        "--target-ns",
        "1",
        "--seed",
        "44",
        "--revision",
        "spec-test",
    ]
    monkeypatch.setattr(
        "heap_construction_benchmark.driver.platform.system",
        lambda: pytest.fail("CLI metadata must not probe the host OS"),
    )
    monkeypatch.setattr(
        "heap_construction_benchmark.driver.platform.machine",
        lambda: pytest.fail("CLI metadata must not probe the host architecture"),
    )
    assert driver.run(argv, stdout, stderr, ticking_clock()) == 0
    document = json.loads(stdout.getvalue())
    core = {key: document[key] for key in ("config", "batch_size", "pairs", "analysis")}
    report = Report.from_dict(core)
    assert report.config.pairs == 5
    assert report.batch_size == 1
    assert document["workload"] == driver.WORKLOAD
    assert document["invocation"][-len(argv) :] == argv
    assert document["revision"] == "spec-test"
    assert document["runtime"]
    assert document["os"]
    assert document["arch"]
    assert (document["os"], document["arch"]) == driver._platform_metadata()
    assert [pair.order for pair in report.pairs].count("baseline-first") == 3
    assert [pair.order for pair in report.pairs].count("candidate-first") == 2


def test_platform_metadata_uses_constants_and_nonempty_fallback(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setattr(
        driver,
        "sys",
        SimpleNamespace(
            platform="test-os",
            implementation=SimpleNamespace(_multiarch="test-arch"),
        ),
    )
    assert driver._platform_metadata() == ("test-os", "test-arch")

    monkeypatch.setattr(
        driver,
        "sys",
        SimpleNamespace(platform="", implementation=SimpleNamespace()),
    )
    assert driver._platform_metadata() == ("unknown", "unknown")


@pytest.mark.parametrize(
    "argv",
    [
        ["--size", "0"],
        ["--size", str(driver.MAX_INPUT_SIZE + 1)],
        ["--pairs", "0"],
        ["--revision", ""],
    ],
)
def test_invalid_configuration_fails_before_clock_or_json(argv: list[str]) -> None:
    calls: list[None] = []

    def clock() -> int:
        calls.append(None)
        return len(calls)

    stderr = io.StringIO()
    with pytest.raises(SystemExit) as error:
        driver.run(argv, io.StringIO(), stderr, clock)
    assert error.value.code == 2
    assert calls == []
    assert stderr.getvalue()


def test_same_immutable_generated_input_is_deterministic() -> None:
    assert driver.make_input(50, -7) == driver.make_input(50, -7)
    assert driver.make_input(50, 7) != driver.make_input(50, 8)
    values = driver.make_input(50, 7)
    expected = sorted(values)
    for builder in (driver.build_by_repeated_push, driver.build_by_floyd):
        driver.verify_builder(builder, values)
        assert expected == sorted(values)


def test_both_builders_accept_one_shared_seeded_tuple() -> None:
    rng = random.Random(91)
    values = tuple(rng.randrange(-100, 100) for _ in range(20))
    expected = sorted(values)
    driver.verify_builder(driver.build_by_repeated_push, values)
    driver.verify_builder(driver.build_by_floyd, values)
    assert len(expected) == len(values)
