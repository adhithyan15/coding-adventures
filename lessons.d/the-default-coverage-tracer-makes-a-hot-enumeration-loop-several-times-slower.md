---
category: Testing & coverage
---

# The default coverage tracer makes a hot enumeration loop several times slower and the sysmon core removes the cost

The Python v2 conformance harness walks the 16.7 million cases of the kit's 2^24
suite in sequence. Plain, that takes about 50 seconds; under `pytest --cov` with
coverage.py's default C tracer it ran for more than seven minutes without
finishing (about 8 microseconds a case against 2), because every Python call and
line in a measured file is an event. Measured on 524,000 cases: 1.1 s untraced,
4.2 s with the default tracer, 1.25 s with `COVERAGE_CORE=sysmon`.

Put `core = "sysmon"` under `[tool.coverage.run]` in `pyproject.toml` (coverage 7.9
or later, Python 3.12 or later, line coverage only: the `sysmon` core does not
measure branches before Python 3.14). Before assuming a test is slow because of the
code under test, time the same loop with and without coverage.
