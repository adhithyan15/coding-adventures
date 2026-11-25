"""Deterministic tests for collection, replay, analysis, and the executable workload."""

import json
import random
import statistics
from pathlib import Path

import pytest

from microbenchmark import Config, Observation, Report, analyze, array_driver, collect
from microbenchmark.array_driver import ARRAY, ONE_PASS, indexed, iterated

FIXTURE = Path(__file__).with_name("microbenchmark-pairs-v1.json")
if not FIXTURE.exists():
    FIXTURE = (
        Path(__file__).resolve().parents[4]
        / "specs/fixtures/microbenchmark-pairs-v1.json"
    )
PORTABLE = json.loads(FIXTURE.read_text(encoding="utf-8"))


def fixed_clock(durations):
    values = iter(value for duration in durations for value in (0, duration))
    now = 0

    def clock():
        nonlocal now
        now += next(values)
        return now

    return clock


def good(batch):
    return ONE_PASS * batch


def test_calibration_balanced_schedule_replay_and_seed_reproducibility():
    # Calibration is 5 ns per side; each pair consumes two more 5 ns intervals.
    config = Config(pairs=5, target_ns=5, seed=99)
    first = collect(good, good, ONE_PASS, config, fixed_clock([5] * 12))
    second = collect(good, good, ONE_PASS, config, fixed_clock([5] * 12))
    assert first.batch_size == 1
    assert [row.order for row in first.pairs].count("baseline-first") == 3
    assert [row.order for row in first.pairs] == [row.order for row in second.pairs]
    replay = Report.from_dict(json.loads(json.dumps(first.to_dict())))
    assert replay.to_dict() == first.to_dict()


@pytest.mark.parametrize(
    "workload",
    ["indexed-array-sum-4096", "dependent-link-order", " \tx", "\u2003", "\xa0", "🚀"],
)
def test_report_replay_validates_workload_before_core_projection(workload):
    core = collect(
        good, good, ONE_PASS, Config(pairs=2, target_ns=5), fixed_clock([5] * 6)
    ).to_dict()
    raw = {
        **core,
        "workload": workload,
        "invocation": [],
        "revision": "producer-revision",
        "runtime": "producer-runtime",
        "os": "windows",
        "arch": "amd64",
        "profile": "native-go",
    }
    original = json.dumps(raw)
    assert Report.from_dict(raw).to_dict() == core
    assert json.dumps(raw) == original
    assert Report.from_dict(core).to_dict() == core


@pytest.mark.parametrize(
    "workload",
    [
        "",
        " ",
        "\t",
        "\n",
        "\r",
        "\v",
        "\f",
        " \t\n\r\v\f",
        "\0",
        "x\0y",
        None,
        42,
        True,
        [],
        {},
        "\ud800",
        "\udfff",
    ],
)
def test_report_replay_rejects_invalid_workload_metadata(workload):
    raw = collect(
        good, good, ONE_PASS, Config(pairs=2, target_ns=5), fixed_clock([5] * 6)
    ).to_dict()
    raw["workload"] = workload
    with pytest.raises(ValueError, match="workload"):
        Report.from_dict(raw)


def test_calibration_doubles_to_target_and_fails_at_limit():
    calls = []

    def workload(batch):
        calls.append(batch)
        return ONE_PASS * batch

    result = collect(
        workload,
        workload,
        ONE_PASS,
        Config(pairs=1, target_ns=8, max_batch=4),
        fixed_clock([0, 0, 8, 8, 1, 1]),
    )
    assert result.batch_size == 2
    assert calls[:6] == [1, 1, 1, 1, 2, 2]
    with pytest.raises(ValueError, match="calibration limit"):
        collect(
            good,
            good,
            ONE_PASS,
            Config(pairs=1, target_ns=9, max_batch=2),
            fixed_clock([1] * 6),
        )


@pytest.mark.parametrize("bad", [-1, 2**63])
def test_bad_checksum_or_clock_arithmetic_is_rejected(bad):
    with pytest.raises(ValueError):
        collect(good, good, bad, Config(pairs=1))


@pytest.mark.parametrize(
    "config",
    [
        Config(pairs=0),
        Config(pairs=10_001),
        Config(pairs=True),
        Config(confidence_percent=0),
        Config(confidence_percent=100),
        Config(confidence_percent=True),
        Config(minimum_factor=1),
        Config(minimum_factor=float("inf")),
        Config(seed=2**63),
        Config(seed=-(2**63) - 1),
        Config(target_ns=0),
        Config(max_batch=0),
        Config(target_ns=2**63),
    ],
)
def test_invalid_collector_configs_fail_before_workloads(config):
    called = []

    def workload(batch):
        called.append(batch)
        return ONE_PASS * batch

    with pytest.raises(ValueError):
        collect(workload, workload, ONE_PASS, config)
    assert called == []


def test_integer_factor_normalizes_to_binary64_in_config_and_report():
    config = Config(pairs=1, minimum_factor=2, target_ns=1)
    report = collect(good, good, ONE_PASS, config, fixed_clock([1] * 4))
    assert type(config.minimum_factor) is float
    assert type(report.to_dict()["config"]["minimum_factor"]) is float


def test_warmup_rejects_invalid_checksum_and_propagates_workload_errors():
    with pytest.raises(ValueError, match="mismatch"):
        collect(lambda batch: 0, good, ONE_PASS, Config(pairs=1))
    with pytest.raises(ValueError, match="checksum must fit"):
        collect(lambda batch: True, good, ONE_PASS, Config(pairs=1))
    with pytest.raises(RuntimeError, match="warm failed"):
        collect(
            lambda batch: (_ for _ in ()).throw(RuntimeError("warm failed")),
            good,
            ONE_PASS,
        )


def test_checksum_overflow_is_checked_before_batch_workload_or_clock():
    maximum = 2**63 - 1
    calls = []
    clock_calls = []

    def workload(batch):
        calls.append(batch)
        return maximum * batch

    def clock():
        clock_calls.append(None)
        return len(clock_calls)

    with pytest.raises(ValueError, match="batch checksum arithmetic"):
        collect(
            workload,
            workload,
            maximum,
            Config(pairs=1, target_ns=2, max_batch=2),
            clock,
        )
    # Two warmups and the two size-one calibration calls happened; size two was
    # rejected before asking the workload or clock to do any timed work.
    assert calls == [1, 1, 1, 1]
    assert len(clock_calls) == 4


def test_timed_workload_error_propagates_without_a_partial_report():
    calls = 0

    def fails_after_warmup(batch):
        nonlocal calls
        calls += 1
        if calls > 1:
            raise RuntimeError("timed failure")
        return ONE_PASS * batch

    with pytest.raises(RuntimeError, match="timed failure"):
        collect(fails_after_warmup, good, ONE_PASS, Config(pairs=1), fixed_clock([1]))


@pytest.mark.parametrize("invalid", [True, -1, 2**63])
def test_clock_rejects_invalid_start_ticks(invalid):
    with pytest.raises(ValueError, match="clock"):
        collect(good, good, ONE_PASS, Config(pairs=1), lambda: invalid)


@pytest.mark.parametrize("ticks", [[0, True], [0, -1], [0, 2**63]])
def test_clock_rejects_invalid_end_ticks(ticks):
    values = iter(ticks)
    with pytest.raises(ValueError, match="clock"):
        collect(good, good, ONE_PASS, Config(pairs=1), lambda: next(values))


@pytest.mark.parametrize("bad_checksum", [True, 1.0, 0, 2**63])
def test_timed_batch_rejects_invalid_or_wrong_checksum(bad_checksum):
    calls = 0

    def candidate(batch):
        nonlocal calls
        calls += 1
        return ONE_PASS * batch if calls == 1 else bad_checksum

    with pytest.raises(ValueError):
        collect(
            good,
            candidate,
            ONE_PASS,
            Config(pairs=1, target_ns=1),
            fixed_clock([1, 1, 1]),
        )


def test_zero_measurement_error_and_regressing_clock():
    with pytest.raises(ValueError, match="positive"):
        collect(
            good,
            good,
            ONE_PASS,
            Config(pairs=1, target_ns=1),
            fixed_clock([1, 1, 0, 1]),
        )
    clock = iter([5, 4])
    with pytest.raises(ValueError, match="clock"):
        collect(good, good, ONE_PASS, Config(pairs=1), lambda: next(clock))


@pytest.mark.parametrize("case", PORTABLE["cases"], ids=lambda case: case["name"])
def test_shared_fixture_replay(case):
    result = analyze(
        [
            Observation(case["baseline_ns"], candidate, "baseline-first")
            for candidate in case["candidate_ns"]
        ],
        PORTABLE["confidence_percent"],
        PORTABLE["minimum_factor"],
    )
    assert result.ratio == pytest.approx(case["ratio"])
    assert (
        result.interval == pytest.approx(case["interval"])
        if case["interval"]
        else result.interval is None
    )
    assert result.verdict == case["verdict"]


def test_large_int64_ratio_uses_binary64_operands():
    top = 2**63 - 1
    result = analyze([Observation(top - 7, top - 3, "candidate-first")])
    assert result.ratio == float(top - 3) / float(top - 7)


@pytest.mark.parametrize(
    "observation",
    [
        None,
        Observation(0, 1, "baseline-first"),
        Observation(1, 0, "baseline-first"),
        Observation(True, 1, "baseline-first"),
        Observation(1, 2**63, "candidate-first"),
        Observation(1, 1, None),
    ],
)
def test_analysis_rejects_malformed_observations(observation):
    with pytest.raises(ValueError):
        analyze([observation])


@pytest.mark.parametrize(
    "factor",
    [True, float("inf"), float("nan"), 10**10000],
    ids=["bool", "infinity", "nan", "huge-int"],
)
def test_factor_rejects_non_binary64_or_unrepresentable_values(factor):
    with pytest.raises(ValueError, match="factor"):
        analyze([Observation(1, 1, "baseline-first")], minimum_factor=factor)


@pytest.mark.parametrize(
    "pairs,confidence,factor",
    [
        ([], 95, 1.05),
        ([Observation(1, 1, "baseline-first")], 0, 1.05),
        ([Observation(1, 1, "baseline-first")], 95, 1),
    ],
)
def test_invalid_analysis_inputs(pairs, confidence, factor):
    with pytest.raises(ValueError):
        analyze(pairs, confidence, factor)


def test_analysis_validates_order_and_zero_calibration_can_expand():
    with pytest.raises(ValueError, match="order"):
        analyze([Observation(1, 1, "first-ish")])
    # Zero calibration intervals trigger doubling; positive measured intervals pass.
    report = collect(
        good,
        good,
        ONE_PASS,
        Config(pairs=1, target_ns=2, max_batch=2),
        fixed_clock([0, 0, 2, 2, 1, 1]),
    )
    assert report.batch_size == 2


def test_report_rejects_invalid_order_and_interval_shape():
    report = collect(
        good, good, ONE_PASS, Config(pairs=1, target_ns=1), fixed_clock([1] * 4)
    ).to_dict()
    report["pairs"][0]["order"] = "sideways"
    with pytest.raises(ValueError, match="order"):
        Report.from_dict(report)
    report = collect(
        good, good, ONE_PASS, Config(pairs=1, target_ns=1), fixed_clock([1] * 4)
    ).to_dict()
    report["analysis"]["interval"] = {"low": 1, "high": 1}
    with pytest.raises(ValueError, match="interval"):
        Report.from_dict(report)
    report = collect(
        good, good, ONE_PASS, Config(pairs=1, target_ns=1), fixed_clock([1] * 4)
    ).to_dict()
    report["batch_size"] = 2**63
    with pytest.raises(ValueError, match="batch_size"):
        Report.from_dict(report)
    report = collect(
        good, good, ONE_PASS, Config(pairs=1, target_ns=1), fixed_clock([1] * 4)
    ).to_dict()
    report["analysis"]["ratio"] = 2.0
    with pytest.raises(ValueError, match="analysis does not match"):
        Report.from_dict(report)


@pytest.mark.parametrize(
    "mutation",
    [
        lambda report: report.update(config=None),
        lambda report: report["config"].pop("seed"),
        lambda report: report.update(pairs=None),
        lambda report: report.update(pairs=[None]),
        lambda report: report["pairs"][0].update(candidate_ns=True),
        lambda report: report["pairs"][0].pop("baseline_ns"),
        lambda report: report.update(analysis=None),
        lambda report: report["analysis"].update(interval=[True, 1.0]),
        lambda report: report["analysis"].update(interval=[10**10000, 1.0]),
        lambda report: report["analysis"].update(ratio=True),
        lambda report: report["analysis"].update(verdict=None),
    ],
    ids=[
        "config-type",
        "missing-config-field",
        "pairs-type",
        "row-type",
        "duration-type",
        "missing-duration",
        "analysis-type",
        "boolean-interval",
        "large-int-interval",
        "boolean-ratio",
        "verdict-type",
    ],
)
def test_report_from_dict_rejects_malformed_json_shapes(mutation):
    report = collect(
        good, good, ONE_PASS, Config(pairs=1, target_ns=1), fixed_clock([1] * 4)
    ).to_dict()
    mutation(report)
    with pytest.raises(ValueError):
        Report.from_dict(report)


def test_report_from_dict_rejects_non_object_and_wrong_pair_count():
    with pytest.raises(ValueError, match="malformed"):
        Report.from_dict(None)
    report = collect(
        good, good, ONE_PASS, Config(pairs=1, target_ns=1), fixed_clock([1] * 4)
    ).to_dict()
    report["pairs"] = []
    with pytest.raises(ValueError, match="pair count"):
        Report.from_dict(report)


def test_seeded_generated_analysis_properties():
    rng = random.Random(72390)
    for _ in range(150):
        raw = [
            (rng.randint(1, 10_000), rng.randint(1, 10_000))
            for _ in range(rng.randint(1, 20))
        ]
        observations = [
            Observation(
                baseline, candidate, rng.choice(["baseline-first", "candidate-first"])
            )
            for baseline, candidate in raw
        ]
        report = analyze(observations, rng.randint(1, 99), 1.1)
        raw_ratios = [
            float(row.candidate_ns) / float(row.baseline_ns) for row in observations
        ]
        assert report.ratio == statistics.median(raw_ratios)
        assert analyze(list(reversed(observations)), 95, 1.1).ratio == report.ratio
        scaled = [
            Observation(row.baseline_ns * 7, row.candidate_ns * 7, row.order)
            for row in observations
        ]
        assert analyze(scaled, 95, 1.1).ratio == report.ratio
        if report.interval is not None:
            assert 0 < report.interval[0] <= report.interval[1]


def test_array_workload_correctness_and_cli_json_entrypoint(capsys, monkeypatch):
    assert len(ARRAY) == 4096
    assert ONE_PASS == 511533
    assert indexed(2) == iterated(2) == ONE_PASS * 2
    args = ["--pairs", "1", "--target-ns", "1", "--revision", "abc123"]

    def deterministic_collect(baseline, candidate, checksum, config):
        return collect(baseline, candidate, checksum, config, fixed_clock([1, 1, 1, 1]))

    monkeypatch.setattr(array_driver, "collect", deterministic_collect)
    array_driver.main(args)
    first = capsys.readouterr().out
    array_driver.main(args)
    second = capsys.readouterr().out
    assert first == second
    result = json.loads(first)
    assert result["workload"] == "indexed-array-sum-4096"
    assert result["revision"] == "abc123"
    assert isinstance(result["invocation"], list)
    assert all(key in result for key in ("runtime", "os", "arch"))
    assert Report.from_dict(
        {key: result[key] for key in ("config", "batch_size", "pairs", "analysis")}
    )


def test_cli_help_and_invalid_configuration_are_user_facing(monkeypatch, capsys):
    with pytest.raises(SystemExit) as help_exit:
        array_driver.main(["--help"])
    assert help_exit.value.code == 0
    help_text = capsys.readouterr().out
    for option in (
        "--pairs",
        "--confidence",
        "--factor",
        "--seed",
        "--target-ns",
        "--max-batch",
        "--revision",
    ):
        assert option in help_text

    def validate_config(_baseline, _candidate, _checksum, config):
        config.validate()

    monkeypatch.setattr(array_driver, "collect", validate_config)
    with pytest.raises(SystemExit) as error_exit:
        array_driver.main(["--pairs", "0"])
    assert error_exit.value.code == 2
    assert "pairs must be from 1 through 10000" in capsys.readouterr().err
