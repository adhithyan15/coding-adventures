"""Tests for the trig-reference-v1 fixture and its generator (PHY00a section 6).

Run:  python3 -m unittest discover -s code/scripts/tests -p test_trig_reference_fixture.py

What is checked, and why it is not circular
-------------------------------------------

* **Reproduction.**  The generator is run in memory and every output file must be
  byte-identical to the committed one (the "regeneration job" of PHY00a 6.4 step 5).
* **Re-derivation by independent code.**  Every `y` is recomputed by the C and Rust
  cross-check programs (which share no code with the Python soft-float), and every
  `cr` on the edge group by GNU `bc -l`.  Both are skipped, loudly, when the tool is
  missing; the generator itself fails closed on the same checks.
* **The specification's own worked examples** (3.10) are asserted against the model,
  so the model cannot drift from the document.
* **Syntax.**  One value has one spelling: lowercase 16-digit hex, LF, closed flag set.
* **Sensitivity gates** (mutation, fusion, guard) run when TRIG_FIXTURE_GATES=1
  (about two minutes); the generator runs them on every regeneration.
"""

from __future__ import annotations

import ast
import hashlib
import json
import os
import random
import shutil
import sys
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO / "code" / "scripts"))

import generate_trig_reference_fixture as gen
import trig_ref_exact
from trig_ref_exact import (
    cr_sincos,
    cr_tan,
    pack,
    round_fraction,
    to_fraction,
    ulp_dist,
    unpack,
)
from trig_ref_softfloat import CONSTANTS, SITES, Model
from trig_ref_softfloat import _round as model_round
from trig_ref_softfloat import decode as model_decode

FIXTURE = REPO / "code" / "specs" / "fixtures" / "trig-reference-v1"
HEX = set("0123456789abcdef")
FLAGS = set(gen.ALL_FLAGS)
OPS = {"sin", "cos", "tan", "radians", "degrees", "sqrt", "atan", "atan2"}


def read_groups():
    out = {}
    for name in ("edge", "sweep-binade", "sweep-cw", "sweep-prng"):
        raw = (FIXTURE / f"{name}.tsv").read_bytes()
        out[name] = raw
    return out


class ModelMatchesTheSpecification(unittest.TestCase):
    """PHY00a 3.10: the worked examples, bit for bit."""

    def test_sin_and_cos_examples(self):
        m = Model()
        cases = [
            (0x3FF0000000000000, 0x3FEAED548F090CEE, None),
            (0x4008000000000000, 0x3FC210386DB6D55B, 0xBFEFAE04BE85E5D2),
            (0x412E848000000000, 0xBFD6664B2568D868, 0x3FEDF9DF9906D32C),
            (0x400921FB54442D18, 0x3CA1A62633145C07, 0xBFF0000000000000),
            (0x41C0000000000000, 0x3FD4E67C0E2622DF, 0xBFEE3EDD2DFEEF90),
        ]
        for x, s, c in cases:
            self.assertEqual(m.sin(x), s, hex(x))
            if c is not None:
                self.assertEqual(m.cos(x), c, hex(x))

    def test_special_values(self):
        m = Model()
        self.assertEqual(m.sin(0x8000000000000000), 0x8000000000000000)  # sin(-0) = -0
        self.assertEqual(m.sin(0), 0)
        self.assertEqual(m.cos(0x8000000000000000), 0x3FF0000000000000)
        for x in (
            0x7FF0000000000000,
            0xFFF0000000000000,
            0x7FF8000000000000,
            0x41C0000000000001,
            0x7FEFFFFFFFFFFFFF,
        ):
            self.assertEqual(m.sin(x), 0x7FF8000000000000)
            self.assertEqual(m.cos(x), 0x7FF8000000000000)

    def test_there_are_twenty_four_fusable_sites(self):
        self.assertEqual(len(SITES), 24)
        self.assertEqual(len(set(SITES)), 24)

    def test_the_constants_derive_from_pi(self):
        """INV_PIO2, the five chunks and the degree constants are what 3.2 says they are."""
        pi = gen.pi_fraction()
        self.assertEqual(round_fraction(2 / pi), CONSTANTS["INV_PIO2"])
        self.assertEqual(round_fraction(pi / 180), CONSTANTS["RAD_PER_DEG"])
        self.assertEqual(round_fraction(180 / pi), CONSTANTS["DEG_PER_RAD"])
        # five parts of pi/2: four chunks of at most 24 significant bits, then a remainder
        total = 0
        for i in range(1, 5):
            v = to_fraction(CONSTANTS[f"PIO2_{i}"])
            self.assertLessEqual((v.numerator).bit_length(), 24, i)
            total += v
        rest = pi / 2 - total
        self.assertEqual(round_fraction(rest), CONSTANTS["PIO2_5"])
        # Taylor coefficients, each the once-rounded exact rational
        from fractions import Fraction
        from math import factorial

        for n in range(1, 10):
            want = Fraction((-1) ** n, factorial(2 * n + 1))
            self.assertEqual(round_fraction(want), CONSTANTS[f"S{n}"], n)
        for n in range(2, 10):
            want = Fraction((-1) ** n, factorial(2 * n))
            self.assertEqual(round_fraction(want), CONSTANTS[f"C{n}"], n)

    def test_exact_reference_agrees_with_known_values(self):
        sb, _, cb, _ = cr_sincos(0x3FF0000000000000)
        self.assertEqual(sb, 0x3FEAED548F090CEE)
        self.assertEqual(cb, 0x3FE14A280FB5068C)
        self.assertEqual(ulp_dist(0x3FF0000000000000, 0x3FF0000000000002), 2)


class ExactReferenceIsIndependent(unittest.TestCase):
    """PHY00a 6.4: the exact reference shares no code with the soft-float model.

    The one place it needed binary64 encoding and decoding, it used to import the
    model's.  It now has its own (``unpack`` and ``pack``); these tests pin that no
    import crept back, and that the two implementations agree everywhere we can
    look: edge patterns, random patterns, rounding ties, subnormals and overflow.
    """

    def test_no_import_from_the_model(self):
        tree = ast.parse(Path(trig_ref_exact.__file__).read_text())
        for node in ast.walk(tree):
            if isinstance(node, ast.ImportFrom):
                self.assertNotIn("softfloat", node.module or "", node.lineno)
            if isinstance(node, ast.Import):
                for alias in node.names:
                    self.assertNotIn("softfloat", alias.name, node.lineno)

    def test_unpack_agrees_with_the_models_decode_and_with_struct(self):
        import struct
        from fractions import Fraction

        rng = random.Random(20261116)
        patterns = [0, 1, 2, 0x000FFFFFFFFFFFFF, 0x0010000000000000, 0x3FF0000000000000]
        patterns += [0x7FEFFFFFFFFFFFFF, 0x8000000000000001, 0xBFF0000000000000]
        patterns += [rng.getrandbits(64) for _ in range(20000)]
        for b in patterns:
            if (b >> 52) & 0x7FF == 0x7FF:
                continue
            sign, mant, exp = model_decode(b)
            theirs = Fraction(mant) * Fraction(2) ** exp
            theirs = -theirs if sign else theirs
            self.assertEqual(unpack(b), theirs, hex(b))
            host = struct.unpack(">d", b.to_bytes(8, "big"))[0]
            self.assertEqual(unpack(b), Fraction(host), hex(b))

    def test_pack_agrees_with_the_models_rounding_and_with_the_host(self):
        import struct
        from fractions import Fraction

        rng = random.Random(7)
        cases = []
        for _ in range(20000):  # arbitrary rationals across the whole range
            num = rng.getrandbits(rng.randint(1, 120)) + 1
            den = rng.getrandbits(rng.randint(1, 120)) + 1
            shift = rng.randint(-1100, 1030)
            v = Fraction(num, den) * Fraction(2) ** shift
            cases.append(-v if rng.random() < 0.5 else v)
        for e in (-1074, -1022, -1, 0, 52, 53, 1023):  # ties and neighbours of ties
            base = Fraction(2) ** e
            for k in (0, 1, 2, 3):
                ulp = Fraction(2) ** max(e - 52, -1074)
                for tie in (
                    Fraction(1, 2),
                    Fraction(1, 2) - Fraction(1, 4096),
                    Fraction(1, 2) + Fraction(1, 4096),
                ):
                    cases.append(base + (k + tie) * ulp)
        cases += [
            Fraction(2) ** 1024,
            Fraction(2) ** 1024 - 2**970,
            Fraction(1, 2**1075),
            Fraction(3, 2**1076),
        ]
        for v in cases:
            host = float(v) if abs(v) < Fraction(2) ** 1024 - 2**970 else None
            mine = pack(v)
            # the model's route: significand and exponent in, bits out
            a = abs(v)
            e = a.numerator.bit_length() - a.denominator.bit_length() - 60
            low = max(e - 4, -1074 - 4)
            scaled = a / (Fraction(2) ** low)
            m, rem = divmod(scaled.numerator, scaled.denominator)
            m = (m << 1) | (1 if rem else 0)
            theirs = model_round(1 if v < 0 else 0, m, low - 1)
            self.assertEqual(mine, theirs, v)
            if host is not None:
                self.assertEqual(
                    mine, int.from_bytes(struct.pack(">d", host), "big"), v
                )

    def test_round_trip_and_special_cases(self):
        self.assertEqual(pack(unpack(0x3FF0000000000000)), 0x3FF0000000000000)
        self.assertEqual(pack(unpack(1)), 1)  # smallest subnormal
        self.assertEqual(pack(unpack(0x7FEFFFFFFFFFFFFF)), 0x7FEFFFFFFFFFFFFF)
        self.assertEqual(pack(unpack(0xBFF0000000000000)), 0xBFF0000000000000)
        from fractions import Fraction

        self.assertEqual(pack(Fraction(0)), 0)
        self.assertEqual(
            pack(Fraction(2) ** 1024), 0x7FF0000000000000
        )  # overflow to inf
        self.assertEqual(
            pack(Fraction(-1, 2**1076)), 0x8000000000000000
        )  # underflow keeps sign
        self.assertEqual(pack(Fraction(1, 2**1075)), 0)  # a tie rounds to even (zero)
        self.assertEqual(pack(Fraction(3, 2**1075)), 2)  # 1.5 ulp rounds to even (2)


class TangentColumn(unittest.TestCase):
    """The tan rows carry a correctly rounded column, except at the sentinel."""

    def test_known_values(self):
        # tan(1) and tan(0.5), each also checked against the host (a sanity check
        # only: the host's tan is not an oracle, but it is within an ulp here)
        import math
        import struct

        for x in (
            0x3FF0000000000000,
            0x3FE0000000000000,
            0xBFF0000000000000,
            0x4008000000000000,
        ):
            host = math.tan(struct.unpack(">d", x.to_bytes(8, "big"))[0])
            hb = int.from_bytes(struct.pack(">d", host), "big")
            self.assertLessEqual(ulp_dist(cr_tan(x), hb), 1, hex(x))
        self.assertEqual(cr_tan(0), 0)
        self.assertEqual(cr_tan(0x8000000000000000), 0x8000000000000000)
        self.assertEqual(
            cr_tan(0x3E00000000000000), 0x3E00000000000000
        )  # tiny: tan x = x

    def test_rows_in_the_fixture(self):
        tan_rows = 0
        for raw in read_groups().values():
            for line in raw.decode().split("\n")[:-1]:
                op, _x, y, cr, flags = line.split("\t")
                if op != "tan":
                    continue
                tan_rows += 1
                got = set(flags.split(",")) - {""}
                if (
                    "pole-sentinel" in got
                    or "nan-class" in got
                    or "nonfinite=rejects" in got
                ):
                    self.assertEqual(cr, "-", line)
                else:
                    self.assertNotEqual(cr, "-", line)
                    # no accuracy claim is made for tan (3.8); this only catches a
                    # transcription slip, an error orders of magnitude out of range
                    self.assertLessEqual(ulp_dist(int(y, 16), int(cr, 16)), 16, line)
        self.assertGreater(tan_rows, 10000)


class DirectedWitnesses(unittest.TestCase):
    """Each constant and fused site a sweep sees rarely has directed vectors (6.4)."""

    def test_every_witness_is_an_edge_input_of_every_op_in_both_signs(self):
        rows = {
            tuple(line.split("\t")[:2])
            for line in read_groups()["edge"].decode().split("\n")[:-1]
        }
        for xs in gen.WITNESSES.values():
            for x in xs:
                for op in gen.TRIG_OPS:
                    self.assertIn((op, f"{x:016x}"), rows)
                    self.assertIn((op, f"{x | (1 << 63):016x}"), rows)

    def test_witness_items_are_not_on_the_masked_lists(self):
        for name in gen.WITNESSES:
            self.assertNotIn(name, gen.MASKED_MUTATION)
            self.assertNotIn(name, gen.MASKED_FUSION)

    def test_each_witness_sees_its_own_item(self):
        # quick (no full scan): flip or fuse the item and look only at its witnesses
        for name, xs in gen.WITNESSES.items():
            variant = (
                Model(fuse={name})
                if name in SITES
                else Model(consts={name: CONSTANTS[name] ^ 1})
            )
            hit = gen._scan(variant, xs, ("sin", "cos"))
            self.assertIsNotNone(hit, name)


class PrngKnownAnswers(unittest.TestCase):
    def test_splitmix_seeded_xoshiro_is_stable(self):
        rng = gen.Xoshiro256ss(gen.SEED)
        first = [rng.next() for _ in range(3)]
        again = gen.Xoshiro256ss(gen.SEED)
        self.assertEqual(first, [again.next() for _ in range(3)])
        self.assertEqual(gen.SEED, 0x545249472D524546)
        self.assertEqual(gen.SEED.to_bytes(8, "big"), b"TRIG-REF")
        self.assertEqual(len(set(first)), 3)


class FixtureSyntax(unittest.TestCase):
    def test_files_are_lf_utf8_without_bom(self):
        for name, raw in read_groups().items():
            self.assertTrue(raw.endswith(b"\n"), name)
            self.assertFalse(raw.startswith(b"\xef\xbb\xbf"), name)
            self.assertNotIn(b"\r", raw, name)
        for fname in ("manifest.json", "schema.json", "README.md"):
            raw = (FIXTURE / fname).read_bytes()
            self.assertNotIn(b"\r", raw)
            self.assertTrue(raw.endswith(b"\n"))

    def test_every_row_is_well_formed(self):
        for raw in read_groups().values():
            for line in raw.decode("utf-8").split("\n")[:-1]:
                fields = line.split("\t")
                self.assertEqual(len(fields), 5, line)
                op, x, y, cr, flags = fields
                self.assertIn(op, OPS, line)
                parts = x.split(",") if op == "atan2" else [x]
                self.assertEqual(len(parts), 2 if op == "atan2" else 1, line)
                for field in (
                    *parts,
                    *([] if y == "error" else [y]),
                    *([] if cr == "-" else [cr]),
                ):
                    self.assertEqual(len(field), 16, line)
                    self.assertTrue(set(field) <= HEX, line)
                if y == "error":
                    self.assertEqual(op, "sqrt")
                got = set(flags.split(",")) - {""}
                self.assertTrue(got <= FLAGS, line)
                self.assertEqual(
                    flags, ",".join(f for f in gen.ALL_FLAGS if f in got), line
                )
                if "nan-class" in got:
                    self.assertEqual(y, "7ff8000000000000", line)
                if "out-of-domain" in got:
                    self.assertEqual(cr, "-", line)
                    self.assertIn(op, ("sin", "cos", "tan"))
                if "nonfinite=unrepresentable" in got:
                    self.assertNotIn("nonfinite=rejects", got, line)

    def test_manifest_matches_the_files(self):
        manifest = json.loads((FIXTURE / "manifest.json").read_text())
        schema = json.loads((FIXTURE / "schema.json").read_text())
        self.assertEqual(set(manifest), set(schema["required"]))
        self.assertEqual(set(manifest), set(schema["properties"]))
        raws = read_groups()
        for g in manifest["groups"]:
            raw = raws[g["name"]]
            self.assertEqual(hashlib.sha256(raw).hexdigest(), g["sha256"])
            self.assertEqual(raw.count(b"\n"), g["rows"])
        self.assertEqual(
            manifest["constants"], {k: f"{v:016x}" for k, v in CONSTANTS.items()}
        )
        for op, a in manifest["accuracy"].items():
            self.assertLessEqual(a["worst_steps"], a["gate_ulp"], op)

    def test_no_finite_result_beyond_the_domain(self):
        for raw in read_groups().values():
            for line in raw.decode().split("\n")[:-1]:
                _op, _x, y, _cr, flags = line.split("\t")
                if "out-of-domain" in flags:
                    self.assertEqual(y, "7ff8000000000000", line)

    def test_gitattributes_marks_the_fixture_binary_like(self):
        text = (REPO / ".gitattributes").read_text()
        self.assertIn("code/specs/fixtures/trig-reference-v1/** -text", text)


class Reproduction(unittest.TestCase):
    """Regenerate in memory: every file must be byte-identical to the committed one."""

    @classmethod
    def setUpClass(cls):
        cls.out, cls.groups, cls.domains, cls.manifest, _ = gen.build_outputs()
        cls.out["README.md"] = gen.readme_text(cls.manifest)

    def test_committed_files_are_reproduced_byte_for_byte(self):
        for name, data in self.out.items():
            self.assertEqual((FIXTURE / name).read_bytes(), data, name)
        extra = {p.name for p in FIXTURE.iterdir()} - set(self.out)
        self.assertEqual(extra, set())

    def test_accuracy_gate_holds(self):
        worst = gen.gate_accuracy(self.groups)
        self.assertLessEqual(worst["sin"]["steps"], 3)
        self.assertLessEqual(worst["cos"]["steps"], 3)
        self.assertLessEqual(worst["radians"]["steps"], 1)
        self.assertLessEqual(worst["degrees"]["steps"], 1)

    def test_worst_cases_on_record_are_edge_vectors(self):
        edge = {
            line.split("\t")[1]
            for line in self.out["edge.tsv"].decode().split("\n")[:-1]
        }
        self.assertIn("41bb531ad2e28013", edge)
        self.assertIn("41b08a633012e355", edge)

    @unittest.skipUnless(
        shutil.which("gcc") and shutil.which("rustc"), "needs gcc and rustc"
    )
    def test_independent_host_implementations_agree(self):
        self.assertGreater(gen.cross_check_hosts(self.groups), 60000)

    @unittest.skipUnless(shutil.which("bc"), "needs GNU bc")
    def test_bc_agrees_on_the_correctly_rounded_column(self):
        self.assertGreater(gen.cross_check_bc(self.groups), 1000)

    @unittest.skipUnless(
        os.environ.get("TRIG_FIXTURE_GATES") == "1",
        "set TRIG_FIXTURE_GATES=1 (about two minutes)",
    )
    def test_sensitivity_gates(self):
        inputs = gen.dedupe(
            int(r.x, 16) for rs in self.groups.values() for r in rs if r.op == "sin"
        )
        report = gen.gate_sensitivity(inputs)
        undetected = {k for k, v in report["fusion"].items() if v is None}
        self.assertEqual(undetected, set(gen.MASKED_FUSION))
        unseen = {k for k, v in report["mutation"].items() if v is None}
        self.assertEqual(unseen, set(gen.MASKED_MUTATION))
        self.assertTrue(all(v is not None for v in report["guard"].values()))


if __name__ == "__main__":
    unittest.main()
