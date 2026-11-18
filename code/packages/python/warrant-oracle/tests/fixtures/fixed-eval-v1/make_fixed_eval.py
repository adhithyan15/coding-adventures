#!/usr/bin/env python3
"""Write (or ``--check``) the synthetic ``ir.fixed`` evaluation document of slice S10c-1 and its dumps.

``fixed-forms-v1`` (slice S10a) pins the *lowering* of every built-in; its claims use only a few of
them, so it cannot pin what the built-ins *compute*.  This fixture does.  The document
``fixed_eval_v1.warrant`` has, for **every one of the 8 fixed-width types**, reference claims whose
pure side is a tuple of built-ins (a tuple of up to 8 components keeps one suite per family and type),
and every ``Nat``-binder case of the shift-count rule (VER04r 5.1, [D26], R10), so that the Rust
generator (slice S10c-2) reproduces, byte for byte, the **evaluation** of all 35 built-ins and the
**domains** of [D21], [D25] and [D26].

Eight files are written beside this script:

``fixed_eval_v1.warrant`` and ``fixed_eval_v1.profile``
    the document (generated here) and its ``corpus.profile`` (``budget 2^20`` so that the 16-bit
    binders are demoted to the 16-bit ``fx_edge`` of [D25]; ``chunk_size 65536``; the fault lines);
``fixed_eval_v1.oracle-lowering.json``
    the lowering dump (``warrant-lowering-parity-v1``);
``fixed_eval_v1.oracle-evaluation.json``
    the evaluation dump (``warrant-evaluation-parity-v1``): every suite's levels, resolved domain
    descriptors (the ``nat_shift`` and ``fx_edge`` leaves), counts, ``wraps``, ``ir_cost``, every chunk
    line and digest, the examples (signed values in TFV) and ``cases.tfv``;
``fixed_eval_v1.oracle-faults.json``
    the fault rows and the controls under the shipped gate (``ir.core`` only: every ``ir.fixed`` and
    ``ir.tuple`` fault is ``deferred``) and under a test-only **open** gate (``ir.core``, ``ir.fixed``
    and ``ir.tuple``: the witness search runs);
``fixed_eval_v1.oracle-values.json``
    the value table (``eval_dumps.values_view``): every built-in at every width on edge values;
``fixed_eval_v1.oracle-domains.json``
    the domain table (``eval_dumps.domain_view``): the ``fx_edge`` and ``nat_shift`` value lists and
    the descriptors a consumer must reject.

    python3 make_fixed_eval.py            # (re)write the files
    python3 make_fixed_eval.py --check    # fail if any committed file differs
"""

from __future__ import annotations

import pathlib
import re
import sys

from coding_adventures_warrant_oracle.eval_dumps import dump_domains, dump_values
from coding_adventures_warrant_oracle.lowering_dump import dump
from coding_adventures_warrant_oracle.parity_dumps import (
    dump_evaluation,
    dump_faults,
    shipped_derivation,
)

HERE = pathlib.Path(__file__).resolve().parent
NAME = "fixed_eval_v1"

TYPES = [
    ("U8", 8, False),
    ("U16", 16, False),
    ("U32", 32, False),
    ("U64", 64, False),
    ("I8", 8, True),
    ("I16", 16, True),
    ("I32", 32, True),
    ("I64", 64, True),
]
BITS = {n: b for n, b, _ in TYPES}
SIGNED = {n: s for n, _, s in TYPES}
ALL = [n for n, _, _ in TYPES]
ALL_TUPLE = "(" + ",".join(ALL) + ")"

#: The ``corpus.profile`` of the dumps.  ``budget 2^20`` demotes the 16-bit binders that would
#: otherwise be full (``U16 x U16`` is 2^32, ``U16 x nat_shift`` 4.8 million) to the 16-bit
#: ``fx_edge`` of [D25]; ``chunk_size 65536`` keeps the widest suite (``I64 x I64``, 388,129 cases) at
#: six chunks.  The ``fault`` lines cover every ``ir.fixed`` template and the shipped ones.
PROFILE = """\
budget 1048576
chunk_size 65536
slim_stride auto
fault op_x_u8 xor-lsb
fault op_x_u8 flip-result-bit
fault op_x_i16 xor-lsb
fault op_x_i32 flip-result-bit
fault op_x_u64 xor-lsb
fault op_x_i64 flip-result-bit
fault op_eq_u64 ignore-high-bit
fault op_eq_u64 invert
fault op_eq_two ignore-high-bit
fault op_bytes2 flip-result-bit
fault op_bytes2 drop-last-result-byte
fault op_bytes2 truncate-last
fault op_bytes flip-result-bit
fault op_bytes drop-last-result-byte
fault op_bytes truncate-last
fault op_r2 flip-result-bit
fault op_r2 drop-last-result-byte
fault op_r flip-result-bit
fault op_r drop-last-result-byte
fault op_r missing-error
fault op_rfix flip-result-bit
"""


def lit(name: str, value: int) -> str:
    suffix = name.lower()
    return f"-{-value}{suffix}" if value < 0 else f"{value}{suffix}"


def lo_hi(name: str) -> tuple[int, int]:
    b = BITS[name]
    return (-(1 << (b - 1)), (1 << (b - 1)) - 1) if SIGNED[name] else (0, (1 << b) - 1)


def inside(src: str, dst: str) -> bool:
    return lo_hi(dst)[0] <= lo_hi(src)[0] and lo_hi(src)[1] <= lo_hi(dst)[1]


def tup(*parts: str) -> str:
    return "(" + ", ".join(parts) + ")"


def tup_ty(*parts: str) -> str:
    return "(" + ",".join(parts) + ")"


def models() -> list[tuple[str, str, str, str]]:
    """``(name, parameters, result type, body)``: each a model, an op and a reference claim."""
    out: list = []
    for t, bits, signed in TYPES:
        s = t.lower()
        n = bits // 8
        mx = f"max_value<{t}>()"
        mn = f"min_value<{t}>()"
        # two operands: bit operations and checked arithmetic (one suite, six built-ins)
        out.append(
            (
                f"bin_{s}",
                f"a: {t}, b: {t}",
                tup_ty(*[t] * 6),
                tup(
                    "bit_and(a, b)",
                    "bit_or(a, b)",
                    "bit_xor(a, b)",
                    "unwrap_or(checked_add(a, b), a)",
                    "unwrap_or(checked_sub(a, b), b)",
                    f"unwrap_or(checked_mul(a, b), {mx})",
                ),
            )
        )
        # division: floor-free fixed division (unsigned) or truncating division (signed)
        d, r = ("div_trunc", "rem_trunc") if signed else ("div", "rem")
        out.append(
            (
                f"div_{s}",
                f"a: {t}, b: {t}",
                tup_ty(t, t, "Bool"),
                tup(f"unwrap_or({d}(a, b), {mn})", f"unwrap_or({r}(a, b), {mx})", f"b == {lit(t, 0)}"),
            )
        )
        # one operand: bit_not, the three counts, both byte orders, the mathematical value
        last_ty, last = ("Nat", "to_nat(a)") if not signed else ("I64", "wrap<I64>(to_int(a))")
        out.append(
            (
                f"un_{s}",
                f"a: {t}",
                tup_ty(t, "Nat", "Nat", "Nat", f"Bytes<{n}>", f"Bytes<{n}>", last_ty),
                tup("bit_not(a)", "popcount(a)", "clz(a)", "ctz(a)", "be_bytes(a)", "le_bytes(a)", last),
            )
        )
        # shifts, rotations and bit: the count is a Nat binder, the nat_shift leaf (D26).  A 16-bit
        # operand is demoted by the profile's budget (D25), so the binder stays ``a: T`` throughout.
        comps = ["shl(a, n)", "shr(a, n)", "rotl(a, n)", "rotr(a, n)", "bit(a, n)"]
        tys = [t, t, t, t, "Bool"]
        if signed:
            comps.append("sar(a, n)")
            tys.append(t)
        out.append((f"sh_{s}", f"a: {t}, n: Nat", tup_ty(*tys), tup(*comps)))
        # conversions out of this type
        out.append(
            (f"wrap_{s}", f"a: {t}", ALL_TUPLE, tup(*[f"wrap<{u}>(a)" for u in ALL]))
        )
        out.append(
            (
                f"try_{s}",
                f"a: {t}",
                ALL_TUPLE,
                tup(*[f"unwrap_or(try<{u}>(a), max_value<{u}>())" for u in ALL]),
            )
        )
        targets = [u for u in ALL if inside(t, u)]
        wide = [f"widen<{u}>(a)" for u in targets]
        wide_ty = list(targets)
        if len(wide) < 2:
            wide.append("wrap<I64>(to_int(a))")
            wide_ty.append("I64")
        out.append((f"widen_{s}", f"a: {t}", tup_ty(*wide_ty), tup(*wide)))
        # endian: round trips, and the wrong byte order
        out.append(
            (
                f"rt_{s}",
                f"a: {t}",
                tup_ty(t, t, t),
                tup(f"from_be<{t}>(be_bytes(a))", f"from_le<{t}>(le_bytes(a))", f"from_be<{t}>(le_bytes(a))"),
            )
        )
        out.append((f"fb_{s}", f"b: Bytes<{n}>", tup_ty(t, t), tup(f"from_be<{t}>(b)", f"from_le<{t}>(b)")))
    # wraps [D31]: add, sub, mul and neg count a wrap; wrap<T> of an exact integer never does
    for t in ("U8", "I8", "U16", "U32"):
        out.append(
            (
                f"wr_{t.lower()}",
                f"a: {t}, b: {t}",
                tup_ty(t, t, t, t, t),
                tup("a + b", "a - b", "a * b", "-a" if SIGNED[t] else f"{lit(t, 0)} - a", f"wrap<{t}>(to_int(a) + to_int(b))"),
            )
        )
    # bounds, as one tuple each
    out.append(("maxes", "", ALL_TUPLE, tup(*[f"max_value<{u}>()" for u in ALL])))
    out.append(("mins", "", ALL_TUPLE, tup(*[f"min_value<{u}>()" for u in ALL])))
    # Int/Nat: floor division and the divisor-signed remainder, imin and imax (an Int is an IR-only
    # intermediate, so each is brought back to I64 by wrap)
    out.append(
        (
            "nat_div",
            "a: Nat, b: Nat",
            "(I64,I64,I64,I64)",
            tup(
                "wrap<I64>(unwrap_or(div(a, b), 0 - 1))",
                "wrap<I64>(unwrap_or(rem(a, b), 0 - 1))",
                "wrap<I64>(imin(a, b))",
                "wrap<I64>(imax(a, b))",
            ),
        )
    )
    out.append(
        (
            "int_div",
            "a: I8, b: I8",
            "(I64,I64,I64,I64)",
            tup(
                "wrap<I64>(unwrap_or(div(to_int(a), to_int(b)), 0 - 99))",
                "wrap<I64>(unwrap_or(rem(to_int(a), to_int(b)), 0 - 99))",
                "wrap<I64>(imin(to_int(a), to_int(b)))",
                "wrap<I64>(imax(to_int(a), to_int(b)))",
            ),
        )
    )
    # at: Bytes and Bytes<N>, a Nat index and an Int index (a negative one is none)
    out.append(("at_dyn", "b: Bytes, i: Nat", "U8", "unwrap_or(at(b, i), 255u8)"))
    out.append(("at_fixed", "b: Bytes<2>, i: Nat", "U8", "unwrap_or(at(b, i), 255u8)"))
    out.append(("at_int", "b: Bytes<2>, n: Nat", "U8", "unwrap_or(at(b, n - 3), 254u8)"))
    return out


#: Reference claims of the shift-count rule (VER04r 5.1, [D26], R10): ``(name, binders, subject args,
#: pure side or body)``.  The comment says whether ``n`` selects ``nat_shift`` (73) or keeps
#: ``nat_0_64`` (65).
SELECTION = [
    # selected: the count operand itself
    ("sel_shl", "a: U8, n: Nat", "a, n", "shl(a, n)"),
    ("sel_shr", "a: U8, n: Nat", "a, n", "shr(a, n)"),
    ("sel_sar", "a: I8, n: Nat", "a, n", "sar(a, n)"),
    ("sel_rotl", "a: U8, n: Nat", "a, n", "rotl(a, n)"),
    ("sel_rotr", "a: U8, n: Nat", "a, n", "rotr(a, n)"),
    ("sel_bit", "a: U8, n: Nat", "a, n", "bit(a, n)"),
    # selected: the count is a binder inside a tuple literal operand, and inside a match arm
    ("sel_in_tuple", "a: U8, n: Nat", "a, n", "(shl(a, n), a)"),
    # selected: a count inside a subject-call argument, and a parenthesised count (the IR has no groups)
    ("sel_in_subject_arg", "a: U8, n: Nat", "shl(a, n)", "a"),
    ("sel_parenthesised", "a: U8, n: Nat", "a, n", "shl(a, (n))"),
    # not selected: the count is an expression of the binder, a model argument, or only in an admit
    ("nosel_expr", "a: U8, n: Nat", "a, n", "shl(a, len(take(b\"0102030405\", n)))"),
    ("nosel_model", "a: U8, n: Nat", "a, n", "m_sel(a, n)"),
    ("nosel_tuple_binder", "t: (U8,Nat)", "t.0, t.1", "shl(t.0, t.1)"),
    # a Nat binder that is not a count keeps nat_0_64 beside one that is
    ("sel_one_of_two", "n: Nat, k: Nat", "n, k", "shl(1u8, n)"),
]


def main() -> str:
    out: list[str] = []
    w = out.append
    ops: list = []

    def declare_op(line: str) -> None:
        w(line)
        name, rest = line.split("op ", 1)[1].split("(", 1)
        params = re.findall(r"(?:^|, )(\w+): ", rest.rsplit(")", 1)[0])
        ops.append((name.strip(), params))

    w("warrant fixed_eval v1")
    w("  -- Synthetic ir.fixed EVALUATION document of slice S10c-1 (VER04r Appendix A.1, 5.1): every")
    w("  -- built-in at every width, the D21/D25/D26 domains and the ir.fixed fault templates.")
    w("  -- GENERATED by make_fixed_eval.py; do not edit by hand.")
    w("  error FixedError = Bad | Worse")
    w("")
    w("  -- pure models: one per (family, type); the result is a tuple of up to 8 built-ins --------")
    ms = models()
    for name, params, ret, body in ms:
        w(f"  model {name}({params}) -> {ret} = {body}")
    w("  model m_sel(a: U8, n: Nat) -> U8 = shl(a, n)")
    w("")
    w("  -- subject ops: the model signatures, the selection ops, and the fault ops -----------------")
    for name, params, ret, _ in ms:
        declare_op(f"  op op_{name}({params}) -> {ret}")
    declare_op("  op op_sel(a: U8, n: Nat) -> U8")
    declare_op("  op op_sel_s(a: I8, n: Nat) -> I8")
    declare_op("  op op_sel_pair(a: U8, n: Nat) -> (U8,U8)")
    declare_op("  op op_sel_bool(a: U8, n: Nat) -> Bool")
    declare_op("  op op_sel_arg(x: U8) -> U8")
    declare_op("  op op_sel_k(n: Nat, k: Nat) -> U8")
    declare_op("  op op_sel_t(a: U8, n: Nat) -> U8")
    declare_op("  op op_sel_arm(a: U8, n: Nat) -> U8")
    declare_op("  op op_x_u8(a: U8) -> U8")
    declare_op("  op op_x_i16(a: I16) -> I16")
    declare_op("  op op_x_i32(a: I32) -> I32")
    declare_op("  op op_x_u64(a: U64) -> U64")
    declare_op("  op op_x_i64(a: I64) -> I64")
    declare_op("  op op_eq_u64(a: U64, b: U64) -> Bool")
    declare_op("  op op_eq_two(a: U64, f: Bool, b: U64) -> Bool")
    declare_op("  op op_bytes2(d: Bytes<2>) -> Bytes<2>")
    declare_op("  op op_bytes(d: Bytes) -> Bytes")
    declare_op("  op op_r2(d: Bytes<2>) -> Result<Bytes<2>,FixedError>")
    declare_op("  op op_r(d: Bytes) -> Result<Bytes,FixedError>")
    declare_op("  op op_rfix(a: U8) -> Result<U8,FixedError>")
    declare_op("  op op_be32(a: U32) -> Bytes<4>")
    w("  assume platform:")
    w("    none")
    w("  assume environment:")
    w("    none")
    for name, params in ops:
        if params:
            w(f"  labels {name}:")
            for prm in params:
                w(f"    {prm} secret")
    for name, params in ops:
        for prm in params:
            w(f"  input {name}.{prm}:")
            w("    hostile")
            w("    ensures no_panic, terminates")
    for name, _ in ops:
        w(f"  bound {name}:")
        w("    steps <= 64")
        w("    heap <= 0")
        w("    depth <= 8")
    w("")
    w("  -- claims: one reference claim per model -----------------------------------------------------")
    for name, params, _ret, body in ms:
        args = ", ".join(re.findall(r"(?:^|, )(\w+): ", params))
        # The shift suites write the built-ins in the claim body itself: a count passed through a
        # model_call argument would not select nat_shift (R10), and these suites must.
        pure = body if name.startswith("sh_") else f"{name}({args})"
        call = f"op_{name}({args}) == {pure}"
        w(f"  claim c_{name}:")
        w(f"    forall {params}. {call}" if params else f"    {call}")
    w("")
    w("  -- claims: the shift-count rule (D26, R10) ------------------------------------------------")
    sel_ops = {
        "sel_sar": "op_sel_s",
        "sel_in_tuple": "op_sel_pair",
        "sel_bit": "op_sel_bool",
        "sel_in_subject_arg": "op_sel_arg",
        "sel_one_of_two": "op_sel_k",
        "nosel_tuple_binder": "op_sel_t",
    }
    for name, binders, args, pure in SELECTION:
        op = sel_ops.get(name, "op_sel")
        w(f"  claim c_{name}:")
        w(f"    forall {binders}. {op}({args}) == {pure}")
    w("  claim c_nosel_admit:")
    w("    forall a: U8, n: Nat | bit(a, n). op_sel(a, n) == a")
    w("  claim c_sel_and_admit:")
    w("    forall a: U8, n: Nat | bit(a, n). op_sel(a, n) == shl(a, n)")
    w("  claim c_sel_in_arm:")
    w("    forall n: Nat, r: Result<U8,FixedError>. "
      "match r with Ok(x) -> op_sel_arm(x, n) == shr(x, n) | Err(_) -> true")
    w("")
    w("  -- claims for the faults (D27): fixed-width results, Bytes, Bytes<N>, Result of each ----------")
    w("  claim c_x_u8:")
    w("    forall a: U8. op_x_u8(a) == a")
    w("  claim c_x_i16:")
    w("    forall a: I16. op_x_i16(a) == a")
    w("  claim c_x_i32:")
    w("    forall a: I32. op_x_i32(a) == a")
    w("  claim c_x_u64:")
    w("    forall a: U64. op_x_u64(a) == a")
    w("  claim c_x_i64:")
    w("    forall a: I64. op_x_i64(a) == a")
    w("  claim c_eq_u64:")
    w("    forall a: U64, b: U64. op_eq_u64(a, b) == (a == b)")
    w("  claim c_eq_two:")
    w("    forall a: U64, f: Bool, b: U64. op_eq_two(a, f, b) == (a == b)")
    w("  claim c_bytes2:")
    w("    forall d: Bytes<2>. op_bytes2(d) == d")
    w("  claim c_bytes:")
    w("    forall d: Bytes. op_bytes(d) == d")
    w("  claim c_r2:")
    w("    forall d: Bytes<2>. op_r2(d) == Ok(d)")
    w("  claim c_r:")
    w("    forall d: Bytes. op_r(d) == (if len(d) == 2 then Ok(d) else Err(FixedError.Bad))")
    w("  claim c_rfix:")
    w("    forall a: U8. op_rfix(a) == Ok(a)")
    w("  claim c_be32:")
    w("    forall a: U32. op_be32(a) == be_bytes(a)")
    w("")
    w("  -- examples: signed values and wide values in TFV, a tuple of built-ins, a Bytes result -------")
    w("  example e_x_u8:")
    w("    op_x_u8(255u8) == 255u8")
    w("  example e_x_i16_negative:")
    w("    op_x_i16(-32768i16) == -32768i16")
    w("  example e_x_i32:")
    w("    op_x_i32(-1i32) == -1i32")
    w("  example e_x_u64:")
    w("    op_x_u64(18446744073709551615u64) == 18446744073709551615u64")
    w("  example e_x_i64:")
    w("    op_x_i64(-9223372036854775807i64) == -9223372036854775807i64")
    w("  example e_be32:")
    w("    op_be32(16909060u32) == b\"01020304\"")
    w("  example e_un_u8:")
    w("    op_un_u8(1u8) == (254u8, 1, 7, 0, b\"01\", b\"01\", 1)")
    w("  example e_sh_i8:")
    w("    op_sh_i8(-128i8, 1) == (0i8, 64i8, 1i8, 64i8, false, -64i8)")
    w("  example e_wrap_i8:")
    w("    op_wrap_i8(-1i8) == (255u8, 65535u16, 4294967295u32, 18446744073709551615u64, -1i8, -1i16, -1i32, -1i64)")
    w("  example e_at_fixed:")
    w("    op_at_fixed(b\"0a14\", 1) == 20u8")
    w("  example e_r2:")
    w("    op_r2(b\"ffff\") == Ok(b\"ffff\")")
    w("")
    return "\n".join(out) + "\n"


def build() -> dict[pathlib.Path, bytes]:
    doc = main().encode("utf-8")
    profile = PROFILE.encode("utf-8")
    shipped = shipped_derivation(doc, profile, f"{NAME}.warrant")  # the two views share it
    return {
        HERE / f"{NAME}.warrant": doc,
        HERE / f"{NAME}.profile": profile,
        HERE / f"{NAME}.oracle-lowering.json": dump(doc),
        HERE / f"{NAME}.oracle-evaluation.json": dump_evaluation(doc, profile, f"{NAME}.warrant", shipped),
        HERE / f"{NAME}.oracle-faults.json": dump_faults(doc, profile, f"{NAME}.warrant", "fixed", shipped),
        HERE / f"{NAME}.oracle-values.json": dump_values(),
        HERE / f"{NAME}.oracle-domains.json": dump_domains(),
    }


def run(argv: list[str]) -> int:
    files = build()
    if "--check" in argv:
        bad = [p.name for p, data in files.items() if not p.exists() or p.read_bytes() != data]
        for name in bad:
            print(f"DIFFERS: {name}", file=sys.stderr)
        if not bad:
            print(f"ok: {len(files)} files")
        return 1 if bad else 0
    for path, data in files.items():
        path.write_bytes(data)
        print(f"wrote {path.name} ({len(data)} bytes)")
    return 0


if __name__ == "__main__":
    raise SystemExit(run(sys.argv[1:]))
