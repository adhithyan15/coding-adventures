"""SplitMix64 property-based tests for ca-capability-analyzer (VER05a / VER08)."""

from __future__ import annotations

from coding_adventures_ca_capability_analyzer.scanner import scan_source
from coding_adventures_ca_capability_analyzer.types import CAP_DYNAMIC_EVAL, CAP_FS_READ


class SplitMix64:
    """Deterministic 64-bit SplitMix64 PRNG for PBT."""

    def __init__(self, seed: int) -> None:
        self.state = seed & 0xFFFFFFFFFFFFFFFF

    def next_u64(self) -> int:
        self.state = (self.state + 0x9E3779B97F4A7C15) & 0xFFFFFFFFFFFFFFFF
        z = self.state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & 0xFFFFFFFFFFFFFFFF
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & 0xFFFFFFFFFFFFFFFF
        return z ^ (z >> 31)

    def bounded(self, bound: int) -> int:
        return self.next_u64() % bound


def test_pbt_pure_arithmetic_and_str_replace_stay_pure_and_deterministic() -> None:
    rng = SplitMix64(0xCA08A112BEEFF00D)
    ops = ["+", "-", "*"]
    names = ["alpha", "beta", "windows", "rand", "sample", "frame"]

    for i in range(48):
        v1 = names[rng.bounded(len(names))]
        v2 = names[rng.bounded(len(names))]
        if v1 == v2:
            v2 = "other"
        a = rng.bounded(1000) - 500
        b = rng.bounded(1000) - 500
        op = ops[rng.bounded(len(ops))]
        src = (
            f"def fn_{i}({v1}: int = {a}, {v2}: int = {b}) -> int:\n"
            f"    _s = 'a..b'.replace('a', 'c')\n"
            f"    return {v1} {op} {v2}\n"
        )
        r1 = scan_source(src, f"gen_{i}.py")
        r2 = scan_source(src, f"gen_{i}.py")
        assert len(r1.capabilities) == 0
        assert len(r1.findings) == 0
        assert r1 == r2


def test_pbt_injected_sinks_always_detected() -> None:
    rng = SplitMix64(0x1337C0DECAFEBABE)
    for i in range(30):
        val = rng.bounded(10_000)
        src_eval = (
            f"def run_{i}():\n    return __import__('math') if {val} > 0 else 0\n"
        )
        r_eval = scan_source(src_eval, f"eval_{i}.py")
        assert CAP_DYNAMIC_EVAL in r_eval.capabilities
        assert any(f.cwe == "CWE-94" for f in r_eval.findings)

        src_fs = (
            f"from pathlib import Path\n"
            f"def probe_{i}(p: Path):\n"
            f"    return p.exists() and {val} >= 0\n"
        )
        r_fs = scan_source(src_fs, f"fs_{i}.py")
        assert CAP_FS_READ in r_fs.capabilities


def test_pbt_hostile_source_snippets_never_crash() -> None:
    rng = SplitMix64(0x9988776655443322)
    fragments = [
        "def f():\n",
        "    x = 1\n",
        "import os\n",
        "from pathlib import Path\n",
        "open('a')\n",
        "eval('1')\n",
        "(((",
        "###\n",
        "\"\n",
        "x = ",
    ]
    for i in range(60):
        count = 1 + rng.bounded(8)
        src = "".join(fragments[rng.bounded(len(fragments))] for _ in range(count))
        try:
            res = scan_source(src, f"fuzz_{i}.py")
            assert isinstance(res.capabilities, frozenset)
        except SyntaxError:
            pass
