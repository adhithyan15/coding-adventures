"""
================================================================
benchmark_mx10 — head-to-head perf benchmark for the MX10 dispatch
================================================================

Standalone runnable script (NOT a pytest test).  For each op that
got an MX10 Rust fast path, runs both the Rust and pure-Python
implementations at sizes above the dispatch threshold and reports
median wallclock per call plus the speedup ratio.

Sizes are picked to clear the per-op threshold:
  * matmul: 200×200 @ 200×200 = 8M multiply-adds (≫ 4096 threshold)
  * elementwise: (500, 200) = 100_000 cells (= elementwise threshold)
  * reduction: same shape, reduce along dim=0 or dim=None
  * activations: same shape

Usage:

    # Compare both paths side-by-side (default; needs C extension):
    python scripts/benchmark_mx10.py

    # Only the pure-Python path (works without the extension):
    python scripts/benchmark_mx10.py --mode fallback

    # Only the Rust path:
    python scripts/benchmark_mx10.py --mode rust

    # Smoke-test mode (N=2 iterations, no warmup) — for CI:
    python scripts/benchmark_mx10.py --quick

The script uses ``time.perf_counter()`` for monotonic high-resolution
timing.  Each benchmark runs **2 warmup iterations** (discarded) then
**N timed iterations** (default 10; ``--quick`` cuts this to 2).
Per-op result is the **median** of timed iterations to suppress
single-call outliers from GC pauses and OS scheduling.

If the matrix_rust_python C extension isn't installed, the script
prints a warning and falls back to ``--mode fallback`` automatically
so it still produces useful output on any machine.
"""

from __future__ import annotations

import argparse
import random
import statistics
import sys
import time
from contextlib import contextmanager

from ml_framework_core import (
    GELUFunction,
    MatMulFunction,
    MeanFunction,
    PowFunction,
    ReLUFunction,
    SigmoidFunction,
    SoftmaxFunction,
    SumFunction,
    TanhFunction,
    Tensor,
    _rust_backend,
)

try:
    import coding_adventures_matrix_rust_python  # noqa: F401

    EXTENSION_AVAILABLE = True
except ImportError:
    EXTENSION_AVAILABLE = False


# ──────────────────────────────────────────────────────────────────
# Timing harness
# ──────────────────────────────────────────────────────────────────


@contextmanager
def _path(use_rust: bool):
    """Force the dispatch predicate to pick ``use_rust`` for one block.

    Toggles ``_rust_backend._RUST_AVAILABLE`` so every dispatch
    predicate (matmul, elementwise, reduction, activation, backward
    broadcast) all see the same flag.  Restores on exit even if the
    block raises.
    """
    saved = _rust_backend._RUST_AVAILABLE
    try:
        _rust_backend._RUST_AVAILABLE = use_rust and EXTENSION_AVAILABLE
        yield
    finally:
        _rust_backend._RUST_AVAILABLE = saved


def _measure(fn, *, warmup: int, iters: int) -> float:
    """Run ``fn()`` warmup times (ignored) + iters times (timed).

    Returns the median per-call wallclock in seconds.  Median chosen
    over mean to suppress outliers from GC pauses / OS scheduling
    blips without needing a large iteration count.
    """
    for _ in range(warmup):
        fn()
    times: list[float] = []
    for _ in range(iters):
        t0 = time.perf_counter()
        fn()
        times.append(time.perf_counter() - t0)
    return statistics.median(times)


# ──────────────────────────────────────────────────────────────────
# Benchmark cases — each returns a (name, callable) pair
# ──────────────────────────────────────────────────────────────────


# Sizes chosen to clear each op's dispatch threshold.
SHAPE_2D = (500, 200)
NUMEL = 500 * 200  # 100_000


def _make_rand(seed: int, lo: float = -1.0, hi: float = 1.0):
    """Deterministic random Tensor of shape SHAPE_2D for fairness."""
    rng = random.Random(seed)
    data = [rng.uniform(lo, hi) for _ in range(NUMEL)]
    return Tensor(data, SHAPE_2D)


def _make_matmul_inputs(seed: int):
    """200×200 @ 200×200 — M·K·N = 8_000_000, well above matmul threshold."""
    rng = random.Random(seed)
    a = Tensor([rng.uniform(-1.0, 1.0) for _ in range(200 * 200)], (200, 200))
    b = Tensor([rng.uniform(-1.0, 1.0) for _ in range(200 * 200)], (200, 200))
    return a, b


def _benchmarks():
    """Yield (op-name, callable) pairs.  Each callable runs one op."""

    # ── matmul forward ──
    a_mm, b_mm = _make_matmul_inputs(seed=1)
    yield "MatMul 200×200 @ 200×200", lambda: MatMulFunction.apply(a_mm, b_mm)

    # ── elementwise activations forward ──
    a_act = _make_rand(seed=2, lo=-3.0, hi=3.0)
    yield "ReLU forward", lambda: ReLUFunction.apply(a_act)
    yield "Sigmoid forward", lambda: SigmoidFunction.apply(a_act)
    yield "Tanh forward", lambda: TanhFunction.apply(a_act)
    yield "GELU forward", lambda: GELUFunction.apply(a_act)
    yield "Softmax(dim=1) forward", lambda: SoftmaxFunction.apply(a_act, 1)

    # ── activation backwards ──
    # For each, run forward once to get the saved-output Function instance
    # bound via .apply, then call backward.  We wrap each in a fresh
    # forward+backward pair so the benchmark covers the full cost the
    # user would pay (forward to set up the autograd node, then backward).
    grad = Tensor([1.0] * NUMEL, SHAPE_2D)

    def _relu_back():
        x = _make_rand(seed=3, lo=-3.0, hi=3.0)
        x.requires_grad = True
        y = ReLUFunction.apply(x)
        y.backward(grad)

    yield "ReLU forward+backward", _relu_back

    def _sigmoid_back():
        x = _make_rand(seed=4, lo=-3.0, hi=3.0)
        x.requires_grad = True
        y = SigmoidFunction.apply(x)
        y.backward(grad)

    yield "Sigmoid forward+backward", _sigmoid_back

    def _tanh_back():
        x = _make_rand(seed=5, lo=-3.0, hi=3.0)
        x.requires_grad = True
        y = TanhFunction.apply(x)
        y.backward(grad)

    yield "Tanh forward+backward", _tanh_back

    def _gelu_back():
        x = _make_rand(seed=6, lo=-3.0, hi=3.0)
        x.requires_grad = True
        y = GELUFunction.apply(x)
        y.backward(grad)

    yield "GELU forward+backward", _gelu_back

    def _softmax_back():
        x = _make_rand(seed=7, lo=-3.0, hi=3.0)
        x.requires_grad = True
        y = SoftmaxFunction.apply(x, 1)
        y.backward(grad)

    yield "Softmax forward+backward", _softmax_back

    # ── reductions ──
    a_red = _make_rand(seed=8)
    yield "Sum reduce-all", lambda: SumFunction.apply(a_red, None, False)
    yield "Mean reduce-all", lambda: MeanFunction.apply(a_red, None, False)
    yield "Sum axis (dim=0)", lambda: SumFunction.apply(a_red, 0, False)
    yield "Mean axis (dim=0)", lambda: MeanFunction.apply(a_red, 0, False)

    # ── reduction backwards ──
    def _sum_reduce_all_back():
        x = _make_rand(seed=9)
        x.requires_grad = True
        y = SumFunction.apply(x, None, False)
        y.backward(Tensor([1.0], (1,)))

    yield "Sum reduce-all forward+backward", _sum_reduce_all_back

    def _sum_axis_back():
        x = _make_rand(seed=10)
        x.requires_grad = True
        y = SumFunction.apply(x, 0, False)
        y.backward(Tensor([1.0] * SHAPE_2D[1], (SHAPE_2D[1],)))

    yield "Sum axis(dim=0) forward+backward", _sum_axis_back

    # ── elementwise Mul/Div backward (Phase 2-back) ──
    def _mul_back():
        a = _make_rand(seed=11)
        b = _make_rand(seed=12)
        a.requires_grad = True
        b.requires_grad = True
        c = a * b
        c.backward(grad)

    yield "Mul forward+backward", _mul_back

    def _div_back():
        a = _make_rand(seed=13)
        # Bound b away from zero so backward (g/b, -g*a/b²) is stable.
        rng = random.Random(14)
        b = Tensor([rng.uniform(0.3, 2.0) for _ in range(NUMEL)], SHAPE_2D)
        a.requires_grad = True
        b.requires_grad = True
        c = a / b
        c.backward(grad)

    yield "Div forward+backward", _div_back

    # ── Pow scalar exponent (Phase 2b) ──
    def _pow_back():
        # Use positive values so non-integer exponent is well-defined.
        rng = random.Random(15)
        x = Tensor([rng.uniform(0.1, 2.0) for _ in range(NUMEL)], SHAPE_2D)
        x.requires_grad = True
        y = PowFunction.apply(x, 2.5)
        y.backward(grad)

    yield "Pow(2.5) forward+backward", _pow_back


# ──────────────────────────────────────────────────────────────────
# Main entry point
# ──────────────────────────────────────────────────────────────────


def _format_seconds(s: float) -> str:
    """Human-readable formatting for seconds, scaled to ms or µs."""
    if s >= 0.1:
        return f"{s * 1e3:.1f} ms"
    if s >= 1e-4:
        return f"{s * 1e3:.2f} ms"
    return f"{s * 1e6:.1f} µs"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="benchmark_mx10",
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--mode",
        choices=("fallback", "rust", "both"),
        default="both",
        help="Which dispatch path to benchmark.  Default: 'both' "
        "(only meaningful if the C extension is installed).",
    )
    parser.add_argument(
        "--quick",
        action="store_true",
        help="Smoke-test mode: 2 iterations, no warmup.  For CI.",
    )
    parser.add_argument(
        "--iters",
        type=int,
        default=10,
        help="Number of timed iterations per op.  Default: 10.",
    )
    parser.add_argument(
        "--warmup",
        type=int,
        default=2,
        help="Number of warmup iterations (discarded).  Default: 2.",
    )
    args = parser.parse_args(argv)

    if args.quick:
        args.iters = 2
        args.warmup = 0

    # Auto-fall-back to fallback-only if extension missing.
    effective_mode = args.mode
    if effective_mode in ("rust", "both") and not EXTENSION_AVAILABLE:
        print(
            "[warning] coding_adventures_matrix_rust_python C extension "
            "is not installed; falling back to --mode fallback.  See "
            "code/packages/rust/matrix-rust-python/ for build "
            "instructions.\n",
            file=sys.stderr,
        )
        effective_mode = "fallback"

    measure_paths: list[tuple[str, bool]] = []
    if effective_mode == "fallback":
        measure_paths.append(("Pure-Python", False))
    elif effective_mode == "rust":
        measure_paths.append(("Rust", True))
    else:  # both
        measure_paths.append(("Pure-Python", False))
        measure_paths.append(("Rust", True))

    print(
        f"# MX10 dispatch benchmark — "
        f"{args.iters} iters/op, {args.warmup} warmup, "
        f"shape {SHAPE_2D} where applicable\n"
    )

    # Build the markdown table header dynamically based on which paths
    # we're measuring.
    if len(measure_paths) == 2:
        header = "| Op | Pure-Python | Rust | Speedup |"
        sep = "|---|---|---|---|"
    else:
        header = f"| Op | {measure_paths[0][0]} |"
        sep = "|---|---|"
    print(header)
    print(sep)

    for name, fn in _benchmarks():
        timings: dict[str, float] = {}
        for path_label, use_rust in measure_paths:
            with _path(use_rust):
                t = _measure(fn, warmup=args.warmup, iters=args.iters)
            timings[path_label] = t

        if len(measure_paths) == 2:
            py_t = timings["Pure-Python"]
            rust_t = timings["Rust"]
            speedup = py_t / rust_t if rust_t > 0 else float("inf")
            print(
                f"| {name} | {_format_seconds(py_t)} | "
                f"{_format_seconds(rust_t)} | {speedup:.1f}× |"
            )
        else:
            (label, _), = [(l, u) for l, u in measure_paths]
            print(f"| {name} | {_format_seconds(timings[label])} |")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
