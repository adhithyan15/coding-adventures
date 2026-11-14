"""The model-IR compiler."""

from __future__ import annotations

import pytest

from coding_adventures_warrant_corpus.errors import CorpusError
from coding_adventures_warrant_corpus.ir import Compiler

MODELS = {
    "same": {"params": ["a", "b"], "body": ["eq", ["var", "a"], ["var", "b"]]},
    "twice": {
        "params": ["a"],
        "body": ["model_call", "same", [["var", "a"], ["var", "a"]]],
    },
}
OPS = {"eq": object()}


def evaluate(node: object, env: dict[str, object] | None = None, subject=None):
    env = env or {}
    return Compiler(MODELS, subject, OPS).compile(node, set(env))(env)


def test_literals_variables_and_comparisons() -> None:
    assert evaluate(["lit", True]) is True
    assert evaluate(["lit", 7]) == 7
    env = {"a": 3, "b": 4}
    assert evaluate(["lt", ["var", "a"], ["var", "b"]], env) is True
    assert evaluate(["ge", ["var", "a"], ["var", "b"]], env) is False
    assert evaluate(["ne", ["var", "a"], ["var", "b"]], env) is True
    assert evaluate(["le", ["var", "a"], ["var", "a"]], env) is True
    assert evaluate(["gt", ["var", "b"], ["var", "a"]], env) is True
    assert evaluate(["eq", ["var", "a"], ["var", "a"]], env) is True


def test_logic_if_and_results() -> None:
    t, f = ["lit", True], ["lit", False]
    assert evaluate(["and", t, f]) is False
    assert evaluate(["or", t, f]) is True
    assert evaluate(["not", f]) is True
    assert evaluate(["implies", f, f]) is True
    assert evaluate(["implies", t, f]) is False
    assert evaluate(["if", t, ["lit", 1], ["lit", 2]]) == 1
    assert evaluate(["if", f, ["lit", 1], ["lit", 2]]) == 2
    assert evaluate(["ok", ["lit", 5]]) == ("ok", 5)
    assert evaluate(["err", "BAD"]) == ("err", "BAD")
    assert evaluate(["name", "X"]) == ("name", "X")
    assert evaluate(["len", ["var", "a"]], {"a": b"abc"}) == 3


def test_arithmetic_is_checked_not_wrapped() -> None:
    env = {"a": 5, "b": 3}
    assert evaluate(["add", ["var", "a"], ["var", "b"]], env) == 8
    assert evaluate(["sub", ["var", "a"], ["var", "b"]], env) == 2
    assert evaluate(["mul", ["var", "a"], ["var", "b"]], env) == 15
    with pytest.raises(CorpusError):
        evaluate(["sub", ["var", "b"], ["var", "a"]], env)  # would be negative
    with pytest.raises(CorpusError):
        evaluate(["mul", ["lit", 1 << 40], ["lit", 1 << 40]])  # past 2**64


def test_model_calls_nest() -> None:
    env = {"x": b"q"}
    assert evaluate(["model_call", "same", [["var", "x"], ["var", "x"]]], env) is True
    assert evaluate(["model_call", "twice", [["var", "x"]]], env) is True


def test_subject_calls_dispatch_through_the_hook_with_width() -> None:
    seen = []

    def hook(name, args, width):
        seen.append((name, args, width))
        return "result"

    node = ["subject_op", "eq", [["var", "a"]], [4]]
    assert evaluate(node, {"a": b"zz"}, hook) == "result"
    assert evaluate(["subject_op", "eq", [["lit", 1]]], {}, hook) == "result"
    assert seen == [("eq", [b"zz"], 4), ("eq", [1], None)]


@pytest.mark.parametrize(
    "node",
    [
        "not a list",
        [],
        [1],
        ["nope"],  # unknown tag: an error, never a skip
        ["lit"],
        ["lit", "string"],
        ["lit", 1.5],
        ["var", "missing"],
        ["var"],
        ["len"],
        ["ok"],
        ["err", 5],
        ["not"],
        ["if", ["lit", True], ["lit", 1]],
        ["eq", ["lit", 1]],
        ["add", ["lit", 1]],
        ["model_call", "same", [["lit", 1]]],  # wrong arity
        ["model_call", "ghost", []],
        ["model_call", "same"],
        ["subject_op", "eq", []],  # no hook: forbidden in a pure expression
    ],
)
def test_malformed_or_forbidden_nodes_fail_at_compile_time(node: object) -> None:
    with pytest.raises(CorpusError):
        evaluate(node)


@pytest.mark.parametrize(
    "node",
    [
        ["subject_op"],
        ["subject_op", "missing_op", []],
        ["subject_op", "eq", "x"],
        ["subject_op", "eq", [], [1, 2]],
        ["subject_op", "eq", [], [-1]],
        ["subject_op", "eq", [], "w"],
    ],
)
def test_malformed_subject_calls(node: object) -> None:
    with pytest.raises(CorpusError):
        evaluate(node, {}, lambda *_: None)


def test_deep_nesting_is_limited() -> None:
    node: object = ["lit", True]
    for _ in range(80):
        node = ["not", node]
    with pytest.raises(CorpusError):
        evaluate(node)


def test_model_table_errors() -> None:
    cycle = {"loop": {"params": [], "body": ["model_call", "loop", []]}}
    with pytest.raises(CorpusError):
        Compiler(cycle, None, {}).compile(["model_call", "loop", []], set())
    for bad in (
        {"m": "text"},
        {"m": {"params": [], "body": ["lit", True], "extra": 1}},
        {"m": {"params": [1], "body": ["lit", True]}},
    ):
        with pytest.raises(CorpusError):
            Compiler(bad, None, {}).compile(["model_call", "m", []], set())


def test_a_model_called_twice_is_compiled_once() -> None:
    compiler = Compiler(MODELS, None, OPS)
    node = [
        "and",
        ["model_call", "same", [["lit", 1], ["lit", 1]]],
        ["model_call", "same", [["lit", 2], ["lit", 2]]],
    ]
    assert compiler.compile(node, set())({}) is True
