"""Executing the corpus against the real library.

A run has three movements, in this order (VER04i section 6):

1. **Examples** -- the spec's hand-written ``example`` lines, one case each.
2. **Exhaustive suites** -- every claim over its whole finite domain.  The
   inputs are *enumerated here*, the expected answers come from interpreting
   the corpus's model IR here, and both streams are hashed.  The hashes must
   equal the ones the Rust compiler computed.  That is the cross-check that
   makes in-lane enumeration safe: a lane that skips a case, reorders the
   domain, or evaluates a model differently produces a different digest.
3. **Negative controls** -- the clean pass proved the library agrees with the
   model; this proves the *runner* can disagree.  One op at a time is broken
   (:mod:`.faults`) and the suites that mention it must fail.

What a suite checks, per case
-----------------------------
::

    actual   = subject side   (calls the REAL export through the binding)
    expected = reference side (pure model, interpreted from the corpus IR)
    ok  iff  type(actual) is exactly the declared result type  and  actual == expected

The expected value never depends on the library, so a wrong library cannot
change the oracle (VER04i 2.5).  A failure prints the suite id and case
*index* only -- never the input bytes (VER00 section 7.3); the index plus the
corpus reproduces the case in any lane.
"""

from __future__ import annotations

import hashlib
import itertools
import math
from collections.abc import Callable
from dataclasses import dataclass, field

from .binding import Binding, Subject, SubjectFailure
from .corpus import MAX_SUITE_PRODUCT, Corpus, Suite
from .domains import binder_domain
from .errors import (
    CALL_COUNT_MISMATCH,
    COUNT_MISMATCH,
    MISSING_ERROR,
    RESULT_MISMATCH,
    STREAM_MISMATCH,
    TYPE_MISMATCH,
    need,
)
from .faults import FAULTS
from .ir import Compiler
from .values import check_value, encode_stream, parse_type, stream_prefix

MAX_REPORTED_CASES = 10  # failing cases named per suite (the rest are counted)


@dataclass
class SuiteResult:
    """The outcome of one exhaustive suite."""

    id: str
    cases: int = 0
    discarded: int = 0
    calls: int = 0
    inputs_sha256: str = ""
    expected_sha256: str = ""
    failures: int = 0
    failed_cases: list[tuple[int, str]] = field(default_factory=list)
    suite_failures: list[str] = field(default_factory=list)

    @property
    def passed(self) -> bool:
        """True when no case failed and every count and digest matched."""
        return self.failures == 0 and not self.suite_failures


def _contains_subject(node: object) -> bool:
    if type(node) is list:
        if node and node[0] == "subject_op":
            return True
        return any(_contains_subject(child) for child in node)
    return False


def _classify(
    actual: object, expected: object, result_type_text: str, corpus: Corpus
) -> str | None:
    """Return the failure id for one case, or ``None`` when it passes."""
    result_type = parse_type(result_type_text)
    if not check_value(actual, result_type, corpus.errors):
        return TYPE_MISMATCH
    if actual == expected:
        return None
    expecting_error = type(expected) is tuple and expected[0] == "err"
    returned_error = type(actual) is tuple and actual[0] == "err"
    return MISSING_ERROR if expecting_error and not returned_error else RESULT_MISMATCH


def run_suite(
    corpus: Corpus, suite: Suite, subject: Subject, *, fail_fast: bool
) -> SuiteResult:
    """Run one exhaustive suite; see the module docstring for the checks."""
    names = [name for name, _descriptor in suite.binders]
    domains = [
        binder_domain({"domain": descriptor})[0] for _n, descriptor in suite.binders
    ]
    # Refuse before running anything: the work this suite will do must be the
    # work it declared (the corpus is untrusted; see corpus.MAX_SUITE_PRODUCT).
    product = math.prod(len(domain) for domain in domains)
    need(
        product == suite.cases + suite.discarded and product <= MAX_SUITE_PRODUCT,
        "work",
        "declared cases + discarded differ from the binder domains' product",
    )
    binder_types = [parse_type(text) for text in suite.binder_types]
    result_type = parse_type(suite.result_type)

    pure = Compiler(corpus.models, None, corpus.ops)
    hooked = Compiler(corpus.models, subject.call, corpus.ops)
    scope = set(names)
    admit = pure.compile(suite.admit, scope)
    lhs_subject, rhs_subject = (
        _contains_subject(suite.lhs),
        _contains_subject(suite.rhs),
    )
    need(
        lhs_subject != rhs_subject,
        "unsupported-ir",
        "exactly one side of a claim may call the subject",
    )
    subject_side, reference_side = (
        (suite.lhs, suite.rhs) if lhs_subject else (suite.rhs, suite.lhs)
    )
    run_subject = hooked.compile(subject_side, scope)
    run_reference = pure.compile(reference_side, scope)

    prefix = stream_prefix(suite.id, list(suite.binder_types), suite.result_type)
    inputs_hash, expected_hash = hashlib.sha256(prefix), hashlib.sha256(prefix)

    result = SuiteResult(suite.id)
    calls_before = subject.total_calls(list(suite.ops))
    index = 0
    stopped = False
    for values in itertools.product(*domains):
        env = dict(zip(names, values, strict=True))
        if not admit(env):
            result.discarded += 1
            continue
        expected = run_reference(env)
        inputs_hash.update(
            b"".join(
                encode_stream(v, t) for v, t in zip(values, binder_types, strict=True)
            )
        )
        expected_hash.update(encode_stream(expected, result_type))
        failure: str | None
        try:
            failure = _classify(run_subject(env), expected, suite.result_type, corpus)
        except SubjectFailure as error:
            failure = error.failure_id
        if failure is not None:
            result.failures += 1
            if len(result.failed_cases) < MAX_REPORTED_CASES:
                result.failed_cases.append((index, failure))
            if fail_fast:
                stopped = True
                index += 1
                break
        index += 1

    result.cases = index
    result.calls = subject.total_calls(list(suite.ops)) - calls_before
    if stopped:
        # A fail-fast run never saw the whole enumeration, so its counts and
        # digests are not comparable; the failing cases already decide it.
        return result
    result.inputs_sha256, result.expected_sha256 = (
        inputs_hash.hexdigest(),
        expected_hash.hexdigest(),
    )
    if (result.cases, result.discarded) != (suite.cases, suite.discarded):
        result.suite_failures.append(COUNT_MISMATCH)
    if (
        result.inputs_sha256 != suite.inputs_sha256
        or result.expected_sha256 != suite.expected_sha256
    ):
        result.suite_failures.append(STREAM_MISMATCH)
    if result.calls != suite.expected_calls:
        result.suite_failures.append(CALL_COUNT_MISMATCH)
    return result


@dataclass
class ExampleResult:
    """The outcome of one example case."""

    id: str
    op: str
    failure: str | None


def run_examples(
    corpus: Corpus, subject: Subject, only: Callable[[str], bool]
) -> list[ExampleResult]:
    """Run each example once.

    ``cases.tfv`` was cross-checked equal to these decoded examples when the
    corpus loaded (:func:`.corpus.load_corpus`), so executing the decoded form
    executes the TFV records.
    """
    results = []
    for example in corpus.examples:
        if not only(example.id):
            continue
        sig = corpus.ops[example.op]
        failure: str | None
        try:
            actual = subject.call(example.op, list(example.args), example.width)
            failure = _classify(actual, example.expected, sig.ret.text, corpus)
        except SubjectFailure as error:
            failure = error.failure_id
        results.append(ExampleResult(example.id, example.op, failure))
    return results


@dataclass
class ControlResult:
    """The outcome of one negative control."""

    id: str
    fault: str
    op: str
    target_kind: str
    caught_by: list[str]
    reached_export: bool

    @property
    def passed(self) -> bool:
        """A control passes iff a required claim failed *and* the export was called."""
        return bool(self.caught_by) and self.reached_export


def run_control(
    corpus: Corpus,
    binding: Binding,
    exports: dict[str, Callable[..., object]],
    control_index: int,
) -> ControlResult:
    """Break one op, rerun exactly the control's target suites, record who noticed."""
    control = corpus.controls[control_index]
    subject = Subject(
        binding,
        exports,
        corpus.ops,
        fault=FAULTS[control.fault],
        fault_op=control.op,
    )
    by_id = {suite.id: suite for suite in corpus.suites}
    caught: list[str] = []
    for suite_id in control.required_claims:
        if run_suite(corpus, by_id[suite_id], subject, fail_fast=True).failures:
            caught.append(suite_id)
    return ControlResult(
        control.id,
        control.fault,
        control.op,
        control.target_kind,
        caught,
        subject.calls[control.op] > 0,
    )


def shuffled(items: list[Suite], seed: int) -> list[Suite]:
    """Order suites by a seeded Fisher-Yates shuffle (SplitMix64).

    Suite order is the only thing the evidence seed controls (VER04i 6): case
    order is canonical.  Shuffling stops a suite from silently depending on
    state left behind by the one before it.
    """
    state = seed
    mask = (1 << 64) - 1

    def next_u64() -> int:
        nonlocal state
        state = (state + 0x9E3779B97F4A7C15) & mask
        z = state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & mask
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & mask
        return z ^ (z >> 31)

    order = list(items)
    for position in range(len(order) - 1, 0, -1):
        other = next_u64() % (position + 1)
        order[position], order[other] = order[other], order[position]
    return order
