"""Second opinions on the v2 conformance kit (slice S5a).

Each test re-derives a slice of the kit by a *different algorithm* from the one the oracle uses, so
that a kit case is checked by two implementations that share no code:

``v2-arith``   Python ``int`` arithmetic, masked by hand here, against the oracle's typed evaluator
``v2-value``   ``struct.pack`` against the oracle's stream encoder
``v2-chunk``   a naive enumerator (``itertools.product``, hand-built streams, ``hashlib``) against the
               chunk files and the single-chunk enumeration
``v2-domain``  the section 5.1 formulas written out against ``descriptor.nth``
``v2-ir``      the cost model of section 4.2 rule 6 written out against the cost cases
the family   the v1 checker ``code/scripts/warrant_corpus_conformance.py``: its strict reader and its
               canonical writer agree with this module's (skipped when the script is not checked out)
"""

from __future__ import annotations

import hashlib
import importlib.util
import itertools
import json
import struct

import pytest
from conftest import CODE

from coding_adventures_warrant_oracle import conformance as C
from coding_adventures_warrant_oracle.irtree import is_node


def cases_of(kit, category):
    return {c["id"].split("/", 1)[1]: c for c in json.loads(kit.files[f"{category}.json"])["cases"]}


# ---------------------------------------------------------------------------------------------------
# v2-arith against Python integers
# ---------------------------------------------------------------------------------------------------

WIDTH = {"U8": 8, "U16": 16, "U32": 32, "U64": 64, "I8": 8, "I16": 16, "I32": 32, "I64": 64}
WINDOW = 2**127 - 1


def reference_arith(case):
    ty, op = case["type"], case["op"]
    a = int(case["lhs"])
    b = int(case["rhs"]) if "rhs" in case else None
    if op in ("eq", "ne", "lt", "gt", "le", "ge"):
        want = {"eq": a == b, "ne": a != b, "lt": a < b, "gt": a > b, "le": a <= b, "ge": a >= b}[op]
        return {"type": "Bool", "value": want}
    exact = {"add": lambda: a + b, "sub": lambda: a - b, "mul": lambda: a * b, "neg": lambda: -a}[op]()
    if ty in WIDTH:
        w = WIDTH[ty]
        r = exact & ((1 << w) - 1)  # the low w bits
        if ty.startswith("I") and r >= 1 << (w - 1):
            r -= 1 << w
        return {"type": ty, "value": str(r)}
    if abs(exact) > WINDOW:
        return {"error": "ir-overflow"}
    return {"type": "Int", "value": str(exact)}


def test_v2_arith_agrees_with_python_integers(kit):
    cases = cases_of(kit, "v2-arith")
    assert len(cases) > 100
    for slug, case in cases.items():
        assert case["expect"] == reference_arith(case), slug


# ---------------------------------------------------------------------------------------------------
# v2-value against struct.pack
# ---------------------------------------------------------------------------------------------------


def split_components(ty):
    """The component type texts of a tuple type ``(A,B,..)``: split at the commas that are not inside ``()`` or ``<>``."""
    assert ty.startswith("(") and ty.endswith(")")
    parts, depth, start = [], 0, 1
    for i, ch in enumerate(ty[1:-1], 1):
        depth += ch in "(<"
        depth -= ch in ")>"
        if ch == "," and depth == 0:
            parts.append(ty[start:i])
            start = i + 1
    parts.append(ty[start:-1])
    return parts


def reference_stream(ty, raw):
    if ty.startswith("("):  # S10d: a tuple is 64, the u32 count, then the components (section 2.4)
        parts = split_components(ty)
        assert len(parts) == len(raw)
        return b"\x64" + struct.pack("<I", len(parts)) + b"".join(reference_stream(t, x) for t, x in zip(parts, raw))
    if ty == "Bool":
        return b"\x01" if raw else b"\x00"
    if ty in ("U8", "U16", "U32", "U64", "Nat"):
        return b"\x10" + struct.pack("<Q", int(raw))
    if ty in ("I8", "I16", "I32", "I64"):
        return b"\x11" + struct.pack("<q", int(raw))  # '<q' sign-extends natively
    if ty.startswith("Bytes"):
        data = bytes.fromhex(raw)
        return b"\x20" + struct.pack("<I", len(data)) + data
    assert ty.startswith("Result<"), ty
    inner = ty[len("Result<") : ty.rindex(",")]
    if "ok" in raw:
        return b"\x50" + reference_stream(inner, raw["ok"])
    name = raw["err"].encode()
    return b"\x51" + struct.pack("<I", len(name)) + name


def test_v2_value_streams_agree_with_struct_pack(kit):
    checked = 0
    for slug, case in cases_of(kit, "v2-value").items():
        if case["expect"]["outcome"] != "accept":
            continue
        want = reference_stream(case["type"], json.loads(bytes.fromhex(case["input_hex"])))
        assert case["expect"]["stream_sha256"] == hashlib.sha256(want).hexdigest(), slug
        if "stream_hex" in case["expect"]:
            assert case["expect"]["stream_hex"] == want.hex(), slug
        else:
            assert len(want.hex()) > C.STREAM_HEX_LIMIT, slug
        checked += 1
    assert checked > 30 + 25  # S10d adds 25 accepted tuple values


# ---------------------------------------------------------------------------------------------------
# v2-domain: the formulas of section 5.1
# ---------------------------------------------------------------------------------------------------


def edge_unsigned(w):
    m = (1 << w) - 1
    return list(range(min(256, 1 << w))) + [1 << i for i in range(8, w)] + [m] + [m ^ (1 << i) for i in range(w)]


def edge_signed(w):
    half, span = 1 << (w - 1), 1 << w
    return sorted({(v - span if v >= half else v) for v in edge_unsigned(w)} | set(range(-255, 0)))


def encode_int(v, signed):
    return ("11" if signed else "10") + (v & (2**64 - 1)).to_bytes(8, "little").hex()


@pytest.mark.parametrize(
    "slug, values, signed",
    [
        ("fx-edge-u32-in-full", edge_unsigned(32), False),
        ("fx-edge-i32-in-full", edge_signed(32), True),
        ("fx-edge-i64-in-full", edge_signed(64), True),
        ("fx-edge-u64-as-u64-edge-in-full", edge_unsigned(64), False),
    ],
)
def test_fx_edge_in_full_agrees_with_the_formulas(kit, slug, values, signed):
    case = cases_of(kit, "v2-domain")[slug]
    assert case["expect"]["size"] == len(values)
    assert case["expect"]["values_hex"] == [encode_int(v, signed) for v in values]


def test_fx_edge_sizes_are_the_published_ones():
    assert [len(edge_unsigned(w)) for w in (16, 32, 64)] == [281, 313, 377]
    assert [len(edge_signed(w)) for w in (16, 32, 64)] == [527, 559, 623]


def bytes_d1():
    out = [b""] + [bytes([i]) for i in range(256)]
    for n in (2, 3):
        out += [bytes(p) for p in itertools.product((0, 1, 128, 255), repeat=n)]
    for n in range(4, 9):
        out += [bytes([v]) * n for v in (0, 0xA5)]
    return out


def test_bytes_d1_first_and_last_values_agree_with_the_formula(kit):
    values = bytes_d1()
    assert len(values) == 347
    cases = cases_of(kit, "v2-domain")
    enc = lambda b: ("20" + struct.pack("<I", len(b)).hex() + b.hex())  # noqa: E731
    assert cases["bytes-d1-range-first-two"]["expect"]["values_hex"] == [enc(v) for v in values[:2]]
    assert cases["bytes-d1-range-last-two"]["expect"]["values_hex"] == [enc(v) for v in values[-2:]]


# ---------------------------------------------------------------------------------------------------
# v2-ir: the cost model, written out
# ---------------------------------------------------------------------------------------------------


def cost(ir, models, memo):
    """Section 4.2 rule 6: the largest number of node evaluations any single evaluation can perform."""
    tag = ir[0]
    if tag in ("lit", "var", "err"):
        return 1
    if tag == "if":
        return 1 + cost(ir[1], models, memo) + max(cost(ir[2], models, memo), cost(ir[3], models, memo))
    if tag == "model_call":
        name = ir[1]
        if name not in memo:
            memo[name] = cost(models[name]["body"], models, memo)
        return 1 + sum(cost(a, models, memo) for a in ir[2]) + memo[name]
    if tag == "subject_op":
        return 1 + sum(cost(a, models, memo) for a in ir[2])
    return 1 + sum(cost(c, models, memo) for c in ir[1:] if is_node(c))


@pytest.mark.parametrize(
    "slug, total",
    [
        ("cost-2000-accepted", 2000),
        ("cost-2001-refused", 2001),
        ("cost-2001-accepted-under-4096", 2001),
        ("cost-4096-accepted-at-limit", 4096),
        ("cost-4097-refused-under-4096", 4097),
        ("cost-if-takes-the-worse-branch", 2000),
        ("cost-if-worse-branch-over", 2001),
        ("cost-and-is-never-credited", 2001),
        ("cost-model-call-adds-the-body", 2000),
        ("cost-model-call-over", 2001),
    ],
)
def test_the_cost_cases_have_exactly_the_costs_they_name(kit, slug, total):
    case = cases_of(kit, "v2-ir")[slug]
    assert cost(case["ir"], case["models"], {}) == total
    limit = case.get("max_ir_cost", 2000)
    assert ("error" in case["expect"]) == (total > limit)


def test_the_model_body_cost_cases_have_their_costs(kit):
    cases = cases_of(kit, "v2-ir")
    for slug, total in (("cost-model-body-2000-accepted", 2000), ("cost-model-body-2001-refused", 2001)):
        case = cases[slug]
        assert cost(case["models"]["big"]["body"], case["models"], {}) == total
    assert cost(cases["cost-doubling-chain-short-accepted"]["ir"], cases["cost-doubling-chain-short-accepted"]["models"], {}) == 1535
    assert cost(cases["cost-doubling-chain-refused"]["models"]["d12"]["body"], cases["cost-doubling-chain-refused"]["models"], {}) > 2000


# ---------------------------------------------------------------------------------------------------
# v2-chunk: a naive enumerator against the chunk files
# ---------------------------------------------------------------------------------------------------


def u32(n):
    return struct.pack("<I", n)


def field(text):
    raw = text.encode()
    return u32(len(raw)) + raw


def enc(kind, v):
    """The stream bytes of a value, by hand: ``kind`` is a type name."""
    if kind == "U8" or kind == "Nat":
        return b"\x10" + struct.pack("<Q", v)
    if kind == "I8":
        return b"\x11" + struct.pack("<q", v)
    if kind == "Bool":
        return b"\x01" if v else b"\x00"
    if kind == "Bytes":
        return b"\x20" + u32(len(v)) + v
    if isinstance(kind, tuple) and kind[0] == "()":  # S10d: ("()", [component kinds])
        return b"\x64" + u32(len(kind[1])) + b"".join(enc(k, x) for k, x in zip(kind[1], v))
    if isinstance(kind, tuple) and kind[0] == "Result":  # S10d: ("Result", payload kind)
        if v[0] == "ok":
            return b"\x50" + enc(kind[1], v[1])
        return b"\x51" + field(v[1])
    if kind.startswith("Result"):
        if v[0] == "ok":
            return b"\x50" + enc("Bool", v[1])
        return b"\x51" + field(v[1])
    raise AssertionError(kind)


def u8(v):
    return v


#: per claim: binder domains (python lists), the admission predicate, the reference value (or None for a
#: predicate suite) and the number of subject calls of one admitted case.
def _suites():
    D1 = bytes_d1()
    return {
        "big": ([list(range(256))] * 3, lambda a, b, c: True, lambda a, b, c: (a + b + c) % 256, "U8", lambda a, b, c: 1),
        "gap": ([list(range(256))] * 2, lambda a, b: a < 2 or a > 253, lambda a, b: (a + b) % 256, "U8", lambda a, b: 1),
        "grid": ([list(range(256))] * 2, lambda a, b: True, lambda a, b: (a + b) % 256, "U8", lambda a, b: 1),
        "ragged": ([list(range(65)), list(range(256))], lambda n, a: True, lambda n, a: n, "Nat", lambda n, a: 1),
        "two_bytes": ([D1, D1], lambda a, b: True, lambda a, b: a == b, "Bool", lambda a, b: 1),
        "short": ([list(range(256))] * 2, lambda a, b: a != 255, None, "Bool", lambda a, b: 2 if a > b else 0),
        "negs": ([list(range(-128, 128))], lambda a: True, lambda a: ((-a + 128) % 256) - 128, "I8", lambda a: 1),
        "res": ([[("ok", False), ("ok", True), ("err", "BAD"), ("err", "WORSE")]], lambda a: True,
                lambda a: a == ("ok", True), "Bool", lambda a: 1),
        # S10d: the suites of kit-tuple.warrant, their binders written as Python tuples (last component fastest)
        "pairs": ([BYTE_BOOL], lambda t: True, lambda t: t[0], "U8", lambda t: 1),
        "triples": ([list(itertools.product(range(256), range(256), BOOLS))], lambda t: True, lambda t: (t[0] + t[1]) % 256, "U8",
                    lambda t: 1),
        "nested": ([list(itertools.product(list(itertools.product(BOOLS, BOOLS)), BYTE_BOOL))], lambda t: True, lambda t: t[1][0], "U8",
                   lambda t: 1),
        "mixed": ([list(range(256)), list(itertools.product(BOOLS, BOOLS))], lambda a, t: True, lambda a, t: t[0] and t[1], "Bool",
                  lambda a, t: 1),
        "result_of_tuple": ([[("ok", t) for t in itertools.product(BOOLS, BOOLS)] + [("err", "BAD"), ("err", "WORSE")]], lambda r: True,
                            lambda r: r[0] == "ok" and r[1][0] and r[1][1], "Bool", lambda r: 1),
        "two_pairs": ([list(itertools.product(BOOLS, BOOLS))] * 2, lambda a, b: True, lambda a, b: a == b, "Bool", lambda a, b: 1),
    }


TUPLE_CLAIMS = {"pairs", "triples", "nested", "mixed", "result_of_tuple", "two_pairs"}
BOOLS = [False, True]
BYTE_BOOL = list(itertools.product(range(256), BOOLS))



BINDER_KINDS = {"big": ["U8"] * 3, "gap": ["U8"] * 2, "grid": ["U8"] * 2, "ragged": ["Nat", "U8"], "two_bytes": ["Bytes"] * 2,
                "short": ["U8"] * 2, "negs": ["I8"], "res": ["Result"],
                "pairs": [("()", ["U8", "Bool"])], "triples": [("()", ["U8", "U8", "Bool"])],
                "nested": [("()", [("()", ["Bool", "Bool"]), ("()", ["U8", "Bool"])])],
                "mixed": ["U8", ("()", ["Bool", "Bool"])], "result_of_tuple": [("Result", ("()", ["Bool", "Bool"]))],
                "two_pairs": [("()", ["Bool", "Bool"])] * 2}


def naive_chunk(row, claim, c, chunk_size):
    domains, admit, ref, _result, calls = _suites()[claim]
    kinds = BINDER_KINDS[claim]
    header = row["stream"].encode() + b"\0"
    prefix = header + field(row["id"]) + u32(len(row["binders"])) + b"".join(field(b[1]["type"]) for b in row["binders"]) + field(row["result_type"])
    lo, hi = c * chunk_size, min(row["product"], (c + 1) * chunk_size)
    h_in = hashlib.sha256(prefix + u32(c))
    h_ex = hashlib.sha256(prefix + u32(c)) if ref else None
    cases = n_calls = 0
    for p, tup in enumerate(itertools.product(*domains)):
        if not lo <= p < hi:
            if p >= hi:
                break
            continue
        if not admit(*tup):
            continue
        cases += 1
        n_calls += calls(*tup)
        h_in.update(b"".join(enc(k, v) for k, v in zip(kinds, tup)))
        if ref:
            h_ex.update(enc(row["result_type"], ref(*tup)))
    return cases, n_calls, h_in.hexdigest(), h_ex.hexdigest() if h_ex else None


def lines_of(kit_obj, claim):
    return [line.split(" ") for line in kit_obj.chunk_file(claim).decode().split("\n")[:-1]]


@pytest.mark.parametrize(
    "which, claim, chunks",
    [
        ("small", "grid", None),
        ("small", "gap", None),
        ("small", "ragged", None),
        ("kit", "two_bytes", None),
        ("kit", "short", None),
        ("kit", "negs", None),
        ("kit", "res", None),
        ("kit", "big", (0, 1, 128, 254, 255)),
        # S10d: the tuple suites of kit-tuple.warrant (the 128 chunks of ``triples`` are probed at the ends and in the middle)
        ("tuple", "pairs", None),
        ("tuple", "nested", None),
        ("tuple", "mixed", None),
        ("tuple", "result_of_tuple", None),
        ("tuple", "two_pairs", None),
        ("tuple", "triples", (0, 1, 5, 64, 126, 127)),
    ],
)
def test_the_chunk_files_agree_with_a_naive_enumerator(kit, which, claim, chunks):
    kit_obj = kit.kits[which]
    row, size = kit_obj.row(claim), kit_obj.chunk_size
    lines = lines_of(kit_obj, claim)
    running = 0
    for c, (idx, cases, first, calls, inputs, *expected) in enumerate(lines):
        assert int(idx) == c and int(first) == running
        running += int(cases)
        if chunks is not None and c not in chunks:
            continue
        n, k, h_in, h_ex = naive_chunk(row, claim, c, size)
        assert (int(cases), int(calls), inputs) == (n, k, h_in), (claim, c)
        assert (expected[0] if expected else None) == h_ex, (claim, c)
    if chunks is None:
        assert running == row["cases"] and row["discarded"] == row["product"] - running


def test_the_v2_chunk_expectations_are_the_chunk_file_lines(kit):
    for slug, case in cases_of(kit, "v2-chunk").items():
        if "error" in case["expect"]:
            continue
        claim = case["suite"]["claim"]
        which = ("small" if claim in ("grid", "gap", "ragged") else "width" if claim in ("fixed1", "ground")
                 else "tuple" if claim in TUPLE_CLAIMS else "kit")
        line = lines_of(kit.kits[which], claim)[case["chunk"]]
        want = {"cases": int(line[1]), "first_admitted": int(line[2]), "calls": int(line[3]), "inputs_sha256": line[4]}
        if len(line) > 5:
            want["expected_sha256"] = line[5]
        assert case["expect"] == want, slug


# ---------------------------------------------------------------------------------------------------
# the family's v1 checker
# ---------------------------------------------------------------------------------------------------

SCRIPT = CODE / "scripts" / "warrant_corpus_conformance.py"


@pytest.fixture(scope="module")
def v1():
    if not SCRIPT.is_file():
        pytest.skip("code/scripts is not checked out")
    spec = importlib.util.spec_from_file_location("warrant_corpus_conformance_v1_script", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_the_v1_strict_reader_agrees_with_ours_on_every_v2_value_input(kit, v1):
    for slug, case in cases_of(kit, "v2-value").items():
        data = bytes.fromhex(case["input_hex"])
        try:
            theirs = ("accept", v1.read_json(data))
        except v1.Reject:
            theirs = ("reject", None)
        try:
            ours = ("accept", C.read_json(data))
        except C.JsonReject:
            ours = ("reject", None)
        assert ours == theirs, slug


def test_the_v1_canonical_writer_reproduces_every_v2_file_byte_for_byte(kit, v1):
    for name, text in kit.files.items():
        assert v1.canonical(json.loads(text)) == text, name


def test_no_v2_id_clashes_with_a_v1_case_and_the_v1_files_are_not_ours(kit, v1):
    v1_ids = {c["id"] for cases in v1.build_cases().values() for c in cases}
    ours = {c["id"] for text in kit.files.values() for c in json.loads(text)["cases"]}
    assert not v1_ids & ours
    assert all(i.startswith("v2-") for i in ours) and not any(i.startswith("v2-") for i in v1_ids)
    assert set(v1.build_cases()) == {"json", "tagged", "containment", "label-digest"}
