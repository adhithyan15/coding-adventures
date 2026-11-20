"""The mutant test of the v2 conformance kit (VER04r 16.13 [D42], slice S5a).

A conformance kit is only worth its cases if a wrong evaluator *fails* one.  Each test below applies one
named wrong evaluator to the oracle (a monkeypatch: the bug a lane could plausibly have) and recomputes
the derived expectation of every case of the categories the mutant is named for.  The mutant **must
change the expectation of at least one case of each of those categories**; a mutant that no case kills
is a missing vector and fails here, before the kit is merged.

The list is [D42]'s floor, fourteen entries (a few spell two bugs):

========================================  ===========================  ===============================
mutant                                    killed by a case of          how it is applied
========================================  ===========================  ===============================
fixed-width ``add`` does not wrap         v2-arith, v2-ir              ``Program._arith``
fixed-width ``add`` saturates             v2-arith, v2-ir              ``Program._arith``
signed comparison on the bit pattern      v2-arith                     ``Program._compare``
``neg`` of the signed minimum raises      v2-arith                     ``Program._node``
``Nat`` product unchecked past 2**127     v2-arith, v2-ir              ``ir_eval._check_exact``
``and``/``if`` evaluate both (calls)      v2-ir                        ``conformance.calls_for``
``take`` does not clamp                   v2-ir                        ``Program._bytes_op``
signed stream value not sign-extended     v2-value, v2-domain          ``stream.make_encoder``
a ``u32`` written big-endian              v2-value, v2-domain          ``stream.u32``
decimal text accepts a leading zero       v2-value                     ``ir_eval.decode_int_text``
product order, first binder fastest       v2-chunk                     ``SuitePlan.run_chunk``
signed ``fx_edge`` in the unsigned order  v2-domain                    ``domains.edge_values``
``first_admitted`` on product ordinals    v2-chunk                     ``conformance.advance_admitted``
chunk index missing from stream prefix    v2-chunk                     ``suites.u32``
``pre`` rewrites applied sequentially     v2-fault                     ``conformance.apply_pre``
``off-by-one`` does not wrap              v2-fault                     ``conformance.post_value``
========================================  ===========================  ===============================
"""

from __future__ import annotations

import hashlib
import operator

import pytest

from coding_adventures_warrant_oracle import conformance as C
from coding_adventures_warrant_oracle import domains as D
from coding_adventures_warrant_oracle import ir_eval as E
from coding_adventures_warrant_oracle import stream as S
from coding_adventures_warrant_oracle import suites as SU
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.lower import FIXED


def expectations(kit, category):
    """``{case id: derived expect}`` of a category, computed from the tables, with no intent check."""
    return {
        f"{category}/{row.slug}": C.derive_expect(category, row.fields, kit.kits) for row in kit.tables[category]
    }


@pytest.fixture(scope="module")
def baseline(kit):
    return {category: expectations(kit, category) for category in C.CATEGORIES}


def killed(kit, baseline, category):
    """The ids of the cases of ``category`` whose derived expectation changed under the current patch."""
    now = expectations(kit, category)
    assert set(now) == set(baseline[category])
    return sorted(i for i, expect in now.items() if expect != baseline[category][i])


# -- the mutants ---------------------------------------------------------------------------------------------------------


def add_does_not_wrap(mp):
    real = E.Program._arith

    def mutant(self, tag, kids, cost, subj):
        a, b = kids
        if tag == "add" and a.ty.head in FIXED:
            af, bf = a.fn, b.fn
            return E.Compiled(a.ty, lambda env: af(env) + bf(env), cost, subj)
        return real(self, tag, kids, cost, subj)

    mp.setattr(E.Program, "_arith", mutant)


def add_saturates(mp):
    real = E.Program._arith

    def mutant(self, tag, kids, cost, subj):
        a, b = kids
        if tag == "add" and a.ty.head in FIXED:
            bits, signed = FIXED[a.ty.head]
            low, high = (-(1 << (bits - 1)), (1 << (bits - 1)) - 1) if signed else (0, (1 << bits) - 1)
            af, bf = a.fn, b.fn
            return E.Compiled(a.ty, lambda env: min(max(af(env) + bf(env), low), high), cost, subj)
        return real(self, tag, kids, cost, subj)

    mp.setattr(E.Program, "_arith", mutant)


def signed_compare_on_the_bit_pattern(mp):
    real = E.Program._compare
    table = {"lt": operator.lt, "gt": operator.gt, "le": operator.le, "ge": operator.ge}

    def mutant(self, tag, kids, cost, subj):
        a, b = kids
        if tag in table and a.ty.head in FIXED and FIXED[a.ty.head][1]:
            mask = (1 << FIXED[a.ty.head][0]) - 1
            af, bf, op = a.fn, b.fn, table[tag]
            return E.Compiled(E.BOOL, lambda env: op(af(env) & mask, bf(env) & mask), cost, subj)
        return real(self, tag, kids, cost, subj)

    mp.setattr(E.Program, "_compare", mutant)


def neg_of_the_signed_minimum_raises(mp):
    real = E.Program._node

    def mutant(self, tag, kids, cost, subj):
        node = real(self, tag, kids, cost, subj)
        head = kids[0].ty.head
        if tag == "neg" and head in FIXED and FIXED[head][1]:
            minimum, inner, af = -(1 << (FIXED[head][0] - 1)), node.fn, kids[0].fn

            def fn(env):
                if af(env) == minimum:
                    raise OracleError("ir-overflow", "mutant")
                return inner(env)

            return E.Compiled(node.ty, fn, node.cost, node.subject)
        return node

    mp.setattr(E.Program, "_node", mutant)


def nat_product_unchecked(mp):
    mp.setattr(E, "_check_exact", lambda value: value)


def and_and_if_evaluate_both_sides(mp):
    """Calls are counted for every ``subject_op`` of the tree, taken or not."""
    mp.setattr(C, "calls_for", lambda prog, ir, tenv: sum(1 for n in C._nodes(ir) if n[0] == "subject_op"))


def take_does_not_clamp(mp):
    real = E.Program._bytes_op

    def mutant(self, tag, kids, cost, subj):
        if tag == "take":
            a, n = kids
            return E.Compiled(E.BYTES, lambda env: a.fn(env)[: n.fn(env)], cost, subj)
        return real(self, tag, kids, cost, subj)

    mp.setattr(E.Program, "_bytes_op", mutant)


def signed_stream_value_not_sign_extended(mp):
    real = S.make_encoder

    def mutant(spec, ty):
        if ty.head in FIXED and FIXED[ty.head][1]:
            mask = (1 << FIXED[ty.head][0]) - 1
            return lambda v: b"\x11" + (v & mask).to_bytes(8, "little")
        return real(spec, ty)

    mp.setattr(S, "make_encoder", mutant)


def u32_big_endian(mp):
    mp.setattr(S, "u32", lambda n: n.to_bytes(4, "big"))


def decimal_accepts_a_leading_zero(mp):
    real = E.decode_int_text

    def mutant(text, ty):
        if isinstance(text, str):
            sign, body = ("-", text[1:]) if text.startswith("-") else ("", text)
            if len(body) > 1 and body[0] == "0" and body.isdigit():
                text = sign + (body.lstrip("0") or "0")
        return real(text, ty)

    mp.setattr(E, "decode_int_text", mutant)


def first_binder_fastest(mp):
    def mutant(self, c, sinks_in=None, sinks_exp=None):
        lo, hi = self.chunk_bounds(c)
        first = self.admitted_before(lo)
        h_in = hashlib.sha256(self.prefix + S.u32(c))
        h_ex = hashlib.sha256(self.prefix + S.u32(c)) if self.expected is not None else None
        cases = calls = 0
        for p in range(lo, hi):
            idx, r = [], p
            for binder in self.binders:  # the FIRST binder is the least significant digit
                r, digit = divmod(r, binder.size)
                idx.append(digit)
            env = {b.name: b.one(i) for b, i in zip(self.binders, idx)}
            if not self.admit(env):
                continue
            cases += 1
            h_in.update(b"".join(b.enc(env[b.name]) for b in self.binders))
            if h_ex is not None:
                h_ex.update(self.enc_result(self.expected(env)))
            calls += self.calls(env) if callable(self.calls) else self.calls
        return SU.ChunkResult(c, cases, first, calls, h_in.digest(), h_ex.digest() if h_ex else None)

    mp.setattr(SU.SuitePlan, "run_chunk", mutant)


def signed_fx_edge_in_the_unsigned_order(mp):
    """The withdrawn set of [D21]: the unsigned bit patterns re-read as signed, in the unsigned order."""
    real = D.edge_values

    def mutant(ty):
        bits, signed = FIXED[ty.head]
        if not signed:
            return real(ty)
        half, span = 1 << (bits - 1), 1 << bits
        return [v - span if v >= half else v for v in D._edge_unsigned(bits)]

    mp.setattr(D, "edge_values", mutant)


def first_admitted_on_product_ordinals(mp):
    mp.setattr(C, "advance_admitted", lambda running, line_cases, chunk_size: running + chunk_size)


def chunk_index_missing_from_the_prefix(mp):
    mp.setattr(SU, "u32", lambda n: b"")


def pre_rewrites_applied_sequentially(mp):
    def mutant(rewrites, penv):
        seen, out = dict(penv), {}
        for param, fn in rewrites:
            out[param] = seen[param] = fn(seen)  # the next rewrite sees this one's output
        return out

    mp.setattr(C, "apply_pre", mutant)


def off_by_one_does_not_wrap(mp):
    real = C.post_value

    def mutant(fault, fn, penv):
        if fault["fault"] == "off-by-one":
            return penv["result"] + 1
        return real(fault, fn, penv)

    mp.setattr(C, "post_value", mutant)


MUTANTS = [
    ("fixed-width-add-does-not-wrap", add_does_not_wrap, ("v2-arith", "v2-ir")),
    ("fixed-width-add-saturates", add_saturates, ("v2-arith", "v2-ir")),
    ("signed-compare-on-the-bit-pattern", signed_compare_on_the_bit_pattern, ("v2-arith",)),
    ("neg-of-the-signed-minimum-raises", neg_of_the_signed_minimum_raises, ("v2-arith",)),
    ("nat-product-unchecked-past-2-127", nat_product_unchecked, ("v2-arith", "v2-ir")),
    ("and-and-if-evaluate-both-sides", and_and_if_evaluate_both_sides, ("v2-ir",)),
    ("take-does-not-clamp", take_does_not_clamp, ("v2-ir",)),
    ("signed-stream-value-not-sign-extended", signed_stream_value_not_sign_extended, ("v2-value", "v2-domain")),
    ("u32-written-big-endian", u32_big_endian, ("v2-value", "v2-domain")),
    ("decimal-text-accepts-a-leading-zero", decimal_accepts_a_leading_zero, ("v2-value",)),
    ("product-order-first-binder-fastest", first_binder_fastest, ("v2-chunk",)),
    ("signed-fx-edge-in-the-unsigned-order", signed_fx_edge_in_the_unsigned_order, ("v2-domain",)),
    ("first-admitted-counted-on-product-ordinals", first_admitted_on_product_ordinals, ("v2-chunk",)),
    ("chunk-index-missing-from-the-stream-prefix", chunk_index_missing_from_the_prefix, ("v2-chunk",)),
    ("pre-rewrites-applied-sequentially", pre_rewrites_applied_sequentially, ("v2-fault",)),
    ("off-by-one-does-not-wrap", off_by_one_does_not_wrap, ("v2-fault",)),
]


@pytest.mark.parametrize("name, apply, categories", MUTANTS, ids=[m[0] for m in MUTANTS])
def test_every_mutant_changes_a_case_of_each_category_it_is_named_for(kit, baseline, monkeypatch, name, apply, categories):
    apply(monkeypatch)
    for category in categories:
        changed = killed(kit, baseline, category)
        assert changed, f"mutant {name!r} changes no case of {category}: a vector is missing"


def test_the_mutant_list_is_the_decision_42_floor():
    assert len(MUTANTS) == 16  # 14 rows of the table; two rows spell two bugs
    named = {c for _, _, cats in MUTANTS for c in cats}
    assert named == set(C.CATEGORIES) - set()  # every category has a killer
    assert len({name for name, _, _ in MUTANTS}) == len(MUTANTS)


def test_without_a_mutant_nothing_changes(kit, baseline):
    """The control: recomputing the expectations unpatched reproduces the baseline."""
    for category in C.CATEGORIES:
        assert killed(kit, baseline, category) == []


def test_each_mutant_is_killed_by_a_case_that_names_it_in_its_summary_or_slug(kit, baseline, monkeypatch):
    """A spot check that the killing case is the intended one (not an accident of an unrelated case)."""
    add_does_not_wrap(monkeypatch)
    assert "v2-arith/u8-add-wrap" in killed(kit, baseline, "v2-arith")
    assert "v2-ir/add-u8-wraps" in killed(kit, baseline, "v2-ir")
