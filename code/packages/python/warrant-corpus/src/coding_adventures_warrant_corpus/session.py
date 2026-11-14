"""One lane run, start to finish: prepare, execute, report.

:func:`prepare` is where a run is *refused* (exit 2) -- the corpus loads, the
binding parses, and the two are checked against each other before any case
runs.  :func:`execute` is where the library is *judged* (exit 0 or 1).

Binding totality (VER04i section 5, rule 1)
-------------------------------------------
* every op the corpus declares has a binding row, and vice versa;
* every error variant declared for an op's ``Result`` has a row, and every
  mapped variant is declared;
* an op with a width parameter says how it takes the width, and an op without
  one does not claim to.

A binding that failed any of these would let some suite quietly skip an
operation or an error path.
"""

from __future__ import annotations

import os
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path

from .binding import Binding, Subject, parse_binding, resolve_exports
from .corpus import Corpus, load_corpus
from .errors import BindingError, UsageError
from .faults import FAULTS
from .jsonio import read_bytes
from .manifest import build_manifest
from .runner import (
    ControlResult,
    ExampleResult,
    SuiteResult,
    run_control,
    run_examples,
    run_suite,
    shuffled,
)

ENV_FAULT = "WARRANT_EVIDENCE_FAULT"
ENV_FILTER = "WARRANT_EVIDENCE_FILTER"
ENV_OUT = "WARRANT_EVIDENCE_OUT"


@dataclass(frozen=True)
class RunConfig:
    """Where everything is, and what to break."""

    repo_root: Path
    corpus_dir: Path
    binding_path: Path
    package_dir: Path
    fault: str | None = None  # a portable fault name, or None
    fault_op: str | None = None
    filter: str | None = None


@dataclass
class Prepared:
    """A corpus and binding that agree, with the library imported."""

    config: RunConfig
    corpus: Corpus
    binding: Binding
    exports: dict[str, Callable[..., object]]
    corpus_relative: str


@dataclass
class Outcome:
    """Everything a run learned."""

    examples: list[ExampleResult] = field(default_factory=list)
    suites: list[SuiteResult] = field(default_factory=list)
    controls: list[ControlResult] = field(default_factory=list)
    manifest: dict[str, object] | None = None

    @property
    def passed(self) -> bool:
        """True when every example, suite and control passed."""
        return (
            all(e.failure is None for e in self.examples)
            and all(s.passed for s in self.suites)
            and all(c.passed for c in self.controls)
        )


def parse_fault_spec(text: str | None) -> tuple[str | None, str | None]:
    """Split ``WARRANT_EVIDENCE_FAULT`` (``<fault>`` or ``<fault>:<op>``).

    ``none`` and the empty string mean "no fault" -- a *clean* run.
    """
    if text is None or text in ("", "none"):
        return None, None
    fault, _separator, op = text.partition(":")
    if fault not in FAULTS:
        raise UsageError("unknown-fault", f"unknown fault `{fault}`")
    return fault, op or None


def check_filter_rules(fault_text: str | None, filter_text: str | None) -> None:
    """The refusal rules of VER04i 4.3.

    A filter on its own, or a filter beside ``FAULT=none``, would produce a
    partial *clean* run that looks like evidence.  Refuse it.
    """
    if filter_text and (fault_text is None or fault_text in ("", "none")):
        raise UsageError(
            "filter-without-fault",
            "WARRANT_EVIDENCE_FILTER requires a real WARRANT_EVIDENCE_FAULT",
        )


def prepare(config: RunConfig) -> Prepared:
    """Load, cross-check and import; raise a :class:`LaneError` to refuse."""
    corpus = load_corpus(config.repo_root, config.corpus_dir)
    binding = parse_binding(read_bytes(config.binding_path, 64 * 1024))
    if binding.module != corpus.module:
        raise BindingError("module", "binding module differs from the corpus module")
    if binding.lane != "python" or not binding.package.startswith("python/"):
        raise BindingError("lane", "this runner only runs the python lane")
    if set(binding.ops) != set(corpus.ops):
        raise BindingError("totality", "binding ops differ from the corpus ops")
    for name, signature in corpus.ops.items():
        op = binding.ops[name]
        if bool(signature.width_params) != (op.width != "none"):
            raise BindingError(
                "width", f"op `{name}`: width binding disagrees with the spec"
            )
        declared = (
            set(corpus.errors[signature.ret.error])
            if signature.ret.kind == "Result" and signature.ret.error in corpus.errors
            else set()
        )
        if {variant for variant, _cls in op.errors} != declared:
            raise BindingError(
                "totality", f"op `{name}`: error rows differ from the spec"
            )
    exports = resolve_exports(binding, config.package_dir)
    relative = (
        config.corpus_dir.resolve().relative_to(config.repo_root.resolve()).as_posix()
    )
    return Prepared(config, corpus, binding, exports, relative)


def _new_subject(prepared: Prepared) -> Subject:
    config = prepared.config
    fault = FAULTS[config.fault] if config.fault else None
    return Subject(
        prepared.binding, prepared.exports, prepared.corpus.ops, fault, config.fault_op
    )


def execute(prepared: Prepared, emit: Callable[[str], None]) -> Outcome:
    """Run the corpus.  ``emit`` receives each TAP-style output line."""
    config, corpus, binding = prepared.config, prepared.corpus, prepared.binding
    faulted = config.fault is not None
    needle = config.filter

    def selected(row_id: str) -> bool:
        return needle is None or needle in row_id

    all_ids = [e.id for e in corpus.examples] + [s.id for s in corpus.suites]
    if needle is not None and not any(selected(i) for i in all_ids):
        raise UsageError("empty-filter", "the filter matched no example or suite")

    outcome = Outcome()
    subject = _new_subject(prepared)

    for example in run_examples(corpus, subject, selected):
        outcome.examples.append(example)
        emit(
            f"ok {example.id}"
            if example.failure is None
            else f"not ok {example.id} {example.failure}"
        )

    todo = [s for s in corpus.suites if selected(s.id)]
    for suite in shuffled(todo, binding.seed):
        result = run_suite(corpus, suite, subject, fail_fast=faulted)
        outcome.suites.append(result)
        for index, failure in result.failed_cases:
            emit(f"not ok {suite.id}@{index} {failure}")
        if result.failures > len(result.failed_cases):
            emit(f"# {suite.id}: {result.failures} failing cases in total")
        for failure in result.suite_failures:
            emit(f"not ok {suite.id} {failure}")
        if result.passed:
            emit(f"ok {suite.id}")
            emit(
                f"# {suite.id}: {result.cases} cases, {result.discarded} "
                f"discarded, {result.calls} calls"
            )
        elif result.failures:
            emit(f"not ok {suite.id} {result.failed_cases[0][1]}")

    clean = not faulted and needle is None
    if clean:
        for index in range(len(corpus.controls)):
            control = run_control(corpus, binding, prepared.exports, index)
            outcome.controls.append(control)
            emit(
                f"ok {control.id}"
                if control.passed
                else f"not ok {control.id} uncaught-fault"
            )
        if outcome.passed:
            outcome.manifest = build_manifest(
                corpus,
                binding,
                prepared.corpus_relative,
                (config.package_dir / binding.source).resolve(),
                _rows(prepared, outcome),
            )
    return outcome


def _rows(prepared: Prepared, outcome: Outcome) -> list[dict[str, object]]:
    corpus = prepared.corpus
    rows: list[dict[str, object]] = [
        {
            "id": e.id,
            "layer": "portable",
            "kind": "example",
            "op": e.op,
            "cases": 1,
            "verdict": "pass",
        }
        for e in outcome.examples
    ]
    by_id = {s.id: s for s in corpus.suites}
    for result in outcome.suites:
        suite = by_id[result.id]
        rows.append(
            {
                "id": suite.id,
                "layer": "portable",
                "kind": "exhaustive",
                "op": suite.ops[0],
                "claim": suite.claim,
                "cases": result.cases,
                "discarded": result.discarded,
                "calls": result.calls,
                "inputs_sha256": result.inputs_sha256,
                "expected_sha256": result.expected_sha256,
                "domain": suite.domain_text,
                "verdict": "pass",
            }
        )
    rows += [
        {
            "id": c.id,
            "layer": "portable",
            "kind": "negctl",
            "fault": c.fault,
            "op": c.op,
            "target": c.target_kind,
            "caught_by": list(c.caught_by),
            "verdict": "pass",
        }
        for c in outcome.controls
    ]
    return rows


def environment() -> tuple[str | None, str | None, str | None]:
    """``(fault, filter, out)`` from the ``WARRANT_EVIDENCE_*`` variables."""
    return (
        os.environ.get(ENV_FAULT),
        os.environ.get(ENV_FILTER) or None,
        os.environ.get(ENV_OUT) or None,
    )
