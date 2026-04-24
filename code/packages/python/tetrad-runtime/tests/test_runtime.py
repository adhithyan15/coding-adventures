"""End-to-end tests for ``TetradRuntime.run``.

These exercise the whole pipeline:

    Tetrad source → tetrad-compiler → code_object_to_iir →
    vm-core (with Tetrad opcode extensions and Tetrad builtins) → result

The runtime runs every program twice — once via the interpreter and once
via the JIT path — and asserts the same result both times.  This is the
strongest check we have that the LANG pipeline genuinely matches Tetrad
semantics: any divergence between the two paths shows up as a test
failure.
"""

from __future__ import annotations

import pytest

from tetrad_runtime import TetradRuntime

# ---------------------------------------------------------------------------
# A tiny harness that runs programs through both the interpreter and JIT
# paths and asserts they agree.
# ---------------------------------------------------------------------------


def _run_both_paths(source: str, *, io_in=None, io_out=None) -> int:
    """Run via interp and JIT; assert both return the same value; return it."""
    captured: list[int] = []

    def _capture_out(v: int) -> None:
        captured.append(v)
        if io_out is not None:
            io_out(v)

    rt_interp = TetradRuntime(io_in=io_in, io_out=_capture_out)
    interp_result = rt_interp.run(source)

    captured.clear()  # reset for the JIT run

    rt_jit = TetradRuntime(io_in=io_in, io_out=_capture_out)
    jit_result = rt_jit.run_with_jit(source)

    assert interp_result == jit_result, (
        f"interp returned {interp_result!r} but JIT returned {jit_result!r}"
    )
    return interp_result


# ---------------------------------------------------------------------------
# Smoke tests — minimal programs that should always work.
# ---------------------------------------------------------------------------


def test_returns_constant() -> None:
    assert _run_both_paths("fn main() -> u8 { return 42; }") == 42


def test_returns_zero_via_lda_zero() -> None:
    assert _run_both_paths("fn main() -> u8 { return 0; }") == 0


def test_simple_addition() -> None:
    assert _run_both_paths(
        "fn main() -> u8 { return 3 + 4; }"
    ) == 7


def test_subtraction_wraps_to_u8() -> None:
    # 3 - 5 in u8 wrap = 254 (0xFE)
    assert _run_both_paths(
        "fn main() -> u8 { return 3 - 5; }"
    ) == 254


def test_addition_wraps_to_u8() -> None:
    # 250 + 10 wraps to 4
    assert _run_both_paths(
        "fn main() -> u8 { return 250 + 10; }"
    ) == 4


# ---------------------------------------------------------------------------
# Function calls
# ---------------------------------------------------------------------------


def test_function_call_with_args() -> None:
    src = (
        "fn add(a: u8, b: u8) -> u8 { return a + b; }\n"
        "fn main() -> u8 { return add(10, 20); }"
    )
    assert _run_both_paths(src) == 30


def test_function_call_chain() -> None:
    src = (
        "fn double(x: u8) -> u8 { return x + x; }\n"
        "fn quad(x: u8)   -> u8 { return double(double(x)); }\n"
        "fn main() -> u8 { return quad(3); }"
    )
    assert _run_both_paths(src) == 12


# ---------------------------------------------------------------------------
# Control flow
# ---------------------------------------------------------------------------


def test_if_else_true_branch() -> None:
    src = (
        "fn pick(c: u8) -> u8 {\n"
        "  if c { return 100; } else { return 200; }\n"
        "}\n"
        "fn main() -> u8 { return pick(1); }"
    )
    assert _run_both_paths(src) == 100


def test_if_else_false_branch() -> None:
    src = (
        "fn pick(c: u8) -> u8 {\n"
        "  if c { return 100; } else { return 200; }\n"
        "}\n"
        "fn main() -> u8 { return pick(0); }"
    )
    assert _run_both_paths(src) == 200


def test_while_loop_counts() -> None:
    src = (
        "fn count(n: u8) -> u8 {\n"
        "  let i = 0;\n"
        "  while i < n { i = i + 1; }\n"
        "  return i;\n"
        "}\n"
        "fn main() -> u8 { return count(7); }"
    )
    assert _run_both_paths(src) == 7


def test_while_loop_accumulates() -> None:
    src = (
        "fn sum(n: u8) -> u8 {\n"
        "  let i = 0;\n"
        "  let s = 0;\n"
        "  while i < n {\n"
        "    s = s + i;\n"
        "    i = i + 1;\n"
        "  }\n"
        "  return s;\n"
        "}\n"
        "fn main() -> u8 { return sum(5); }"  # 0+1+2+3+4 = 10
    )
    assert _run_both_paths(src) == 10


# ---------------------------------------------------------------------------
# Comparisons & logical operators
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    "expr,expected",
    [
        ("3 == 3", 1),
        ("3 == 4", 0),
        ("3 != 4", 1),
        ("3 < 4", 1),
        ("4 < 3", 0),
        ("3 <= 3", 1),
        ("3 > 4", 0),
        ("4 >= 3", 1),
    ],
)
def test_comparisons(expr: str, expected: int) -> None:
    src = f"fn main() -> u8 {{ return {expr}; }}"
    assert _run_both_paths(src) == expected


def test_logical_not() -> None:
    assert _run_both_paths("fn main() -> u8 { return !0; }") == 1
    assert _run_both_paths("fn main() -> u8 { return !1; }") == 0
    assert _run_both_paths("fn main() -> u8 { return !42; }") == 0


def test_short_circuit_and() -> None:
    src = (
        "fn main() -> u8 {\n"
        "  let a = 1;\n"
        "  let b = 0;\n"
        "  if a && b { return 99; } else { return 1; }\n"
        "}"
    )
    assert _run_both_paths(src) == 1


def test_short_circuit_or() -> None:
    src = (
        "fn main() -> u8 {\n"
        "  let a = 0;\n"
        "  let b = 1;\n"
        "  if a || b { return 99; } else { return 1; }\n"
        "}"
    )
    assert _run_both_paths(src) == 99


# ---------------------------------------------------------------------------
# Bitwise
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    "expr,expected",
    [
        ("0xFF & 0x0F", 0x0F),
        ("0x0F | 0xF0", 0xFF),
        ("0xFF ^ 0x0F", 0xF0),
        ("~0xFF", 0x00),
        ("1 << 3", 8),
        ("8 >> 2", 2),
    ],
)
def test_bitwise(expr: str, expected: int) -> None:
    src = f"fn main() -> u8 {{ return {expr}; }}"
    assert _run_both_paths(src) == expected


def test_shl_wraps_to_u8() -> None:
    # 1 << 8 in standard semantics is 256, but Tetrad's u8 wraps to 0.
    src = (
        "fn shifty(n: u8) -> u8 {\n"
        "  let r = 1;\n"
        "  let i = 0;\n"
        "  while i < n { r = r << 1; i = i + 1; }\n"
        "  return r;\n"
        "}\n"
        "fn main() -> u8 { return shifty(8); }"
    )
    assert _run_both_paths(src) == 0


# ---------------------------------------------------------------------------
# Globals
# ---------------------------------------------------------------------------


def test_globals_initialised_at_top_level() -> None:
    """Globals are init-once at top level; functions cannot see them
    (Tetrad's compiler rejects ``LDA_VAR`` for non-local names inside
    functions).  We verify globals via ``globals_snapshot``."""
    rt = TetradRuntime()
    rt.run(
        "let x: u8 = 5;\n"
        "let y: u8 = x + 7;\n"
        "fn main() -> u8 { return 0; }"
    )
    assert rt.globals_snapshot["x"] == 5
    assert rt.globals_snapshot["y"] == 12


def test_globals_chain_in_top_level_initialisers() -> None:
    """A later ``let`` may reference an earlier global."""
    rt = TetradRuntime()
    rt.run(
        "let a: u8 = 10;\n"
        "let b: u8 = a * 2;\n"
        "let c: u8 = a + b;\n"
        "fn main() -> u8 { return 0; }"
    )
    assert rt.globals_snapshot == {"a": 10, "b": 20, "c": 30}


# ---------------------------------------------------------------------------
# I/O via injected callables
# ---------------------------------------------------------------------------


def test_io_in_uses_injected_callable() -> None:
    """``in()`` reads from the runtime's ``io_in``; we feed a fixed value.

    Tetrad's type-checker classifies ``in()`` as Unknown, so we route the
    value through a helper function whose param IS typed.
    """
    src = (
        "fn echo(v: u8) -> u8 { return v + 1; }\n"
        "fn main() -> u8 { return echo(in()); }"
    )
    rt = TetradRuntime(io_in=lambda: 41, io_out=lambda _v: None)
    assert rt.run(src) == 42


def test_io_out_routes_to_injected_callable() -> None:
    """``out(v)`` writes through the runtime's ``io_out`` callable."""
    captured: list[int] = []
    src = (
        "fn main() -> u8 {\n"
        "  out(7);\n"
        "  out(42);\n"
        "  return 0;\n"
        "}"
    )
    rt = TetradRuntime(io_out=captured.append)
    rt.run(src)
    assert captured == [7, 42]


# ---------------------------------------------------------------------------
# Compile_to_iir round-trip — the IIRModule should be re-runnable.
# ---------------------------------------------------------------------------


def test_compile_to_iir_module_can_be_re_executed() -> None:
    from tetrad_runtime import compile_to_iir
    module = compile_to_iir("fn main() -> u8 { return 99; }")
    rt = TetradRuntime()
    assert rt.run_module(module) == 99
    # Running again should not leak state.
    assert rt.run_module(module) == 99


def test_run_code_object_path() -> None:
    """``run_code_object`` accepts a CodeObject directly."""
    from tetrad_compiler import compile_program
    code = compile_program("fn main() -> u8 { return 21 + 21; }")
    rt = TetradRuntime()
    assert rt.run_code_object(code) == 42
