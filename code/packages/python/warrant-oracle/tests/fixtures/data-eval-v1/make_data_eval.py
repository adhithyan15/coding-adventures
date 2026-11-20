#!/usr/bin/env python3
"""Write (or ``--check``) the synthetic ``ir.data`` EVALUATION document of slice S11c-1 and its dumps.

``data-forms-v1`` (slice S11a) pins the *lowering* of every form of VER04r Appendix A.2; it cannot pin
what the forms *compute*.  This fixture does.  The document ``data_eval_v1.warrant`` evaluates records,
variants and ``Option`` over their enumerated domains (reference claims whose binders and results are data
values), writes data values in the examples, and binds the faults that need ``ir.data`` (``field-zero``
[D49] and the record, ``Option`` and variant payloads of ``missing-error`` [D50], VER04r 8.2), so that the
Rust generator (slice S11c-2) reproduces, byte for byte, the **evaluation**, the **values**, the **domains**
and the **fault rows** of everything the oracle writes here.

Eight files are written beside this script (plus ``README.md`` and ``.gitattributes``, which are not
generated):

``data_eval_v1.warrant`` and ``data_eval_v1.profile``
    the document (generated here) and its ``corpus.profile`` (``budget 2^20`` so that a ``U16`` field inside a
    record, a variant and an ``Option`` is demoted to the 16-bit ``fx_edge`` of [D25], and the path of that
    demotion is the text of section 5.3 [D54]; ``chunk_size 16384``; the fault lines);
``data_eval_v1.oracle-lowering.json``
    the lowering dump (``warrant-lowering-parity-v1``), including the corpus ``types`` rows (R59);
``data_eval_v1.oracle-evaluation.json``
    the evaluation dump (``warrant-evaluation-parity-v1``) under the gate this slice opens: every suite's levels,
    stream header (the **v1** header for the suites whose types are all in the v1 set, ``Option<U8>`` included,
    and the **v2** header for every record, variant and tuple), resolved descriptors (``tuple`` for a record,
    ``variant``, ``option``, with the ``demoted`` paths), counts, ``wraps``, ``ir_cost``, every chunk line and
    digest, the examples (JSON and TFV: ``L0``/``L1 x`` for an ``Option``, ``L2 S<ctor> L<k> ..`` for a variant)
    and ``cases.tfv``;
``data_eval_v1.oracle-faults.json``
    the fault rows (the JSON of section 8.2, **normative** for ``field-zero`` and ``missing-error``) and the controls
    twice: ``controls_shipped`` under the gate *before* this slice (``ir.core``, ``ir.fixed``, ``ir.tuple``: every
    ``ir.data`` fault is ``deferred``) and ``controls_gate_on`` with ``ir.data`` evaluated (the witness search runs);
``data_eval_v1.oracle-values.json``
    the value table (:func:`.eval_dumps.data_values_view`): a record, a variant and an ``Option`` in JSON, in the
    stream and in TFV, and the JSON forms a consumer must reject;
``data_eval_v1.oracle-domains.json``
    the domain table (:func:`.eval_dumps.data_domain_view`): the ``tuple``/``variant``/``option`` descriptors with
    their sizes and values, the budget demotions with their paths, and the descriptors a consumer must reject;
``data_eval_v1.oracle-refusals.json``
    every signature a data fault refuses and every fault row the typing of R67 and R94 refuses.

    python3 make_data_eval.py            # (re)write the files
    python3 make_data_eval.py --check    # fail if any committed file differs

It is a directory of its own, **not** ``data-forms-v1``: the generator's S11b parity test reproduces that
directory's bytes, so a new file there would turn it red until S11c-2 landed.

Names: a model has a subject op ``op_<model>`` with the same signature and a reference claim ``c_<model>``.
"""

from __future__ import annotations

import pathlib
import re
import sys

from coding_adventures_warrant_oracle.corpus_v1 import build_program
from coding_adventures_warrant_oracle.corpus_v2 import canonical, derive
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.eval_dumps import data_domain_view, data_values_view
from coding_adventures_warrant_oracle.faults import _fault_unify, _typed, fault_row
from coding_adventures_warrant_oracle.lower import lower_document
from coding_adventures_warrant_oracle.lowering_dump import dump
from coding_adventures_warrant_oracle.parity_dumps import (
    GATE_BEFORE_DATA,
    GATE_DATA,
    dump_evaluation,
    dump_faults,
)
from coding_adventures_warrant_oracle.parser import parse_source

HERE = pathlib.Path(__file__).resolve().parent
NAME = "data_eval_v1"

#: The ``corpus.profile`` of the dumps.  ``budget 2^20`` demotes the ``U16`` leaves that would otherwise make
#: ``Wide`` (65 x 65,536) and ``Msg`` (65 x 65,537 in one constructor) too large: the paths are ``1`` (a binder),
#: ``0.0`` (field 0 of a record) and ``0.Done.1.some`` (the payload of the ``Option`` that is field 1 of ``Done``,
#: the example of section 5.3).  ``chunk_size 16384`` gives the 65,536-case suites four chunks.
FAULT_LINES = [
    ("op_tri", "missing-error"),  # Option payload, zero ``none`` (the tri_state row of 8.2); v1 stream header
    ("op_pick", "missing-error"),  # Option leaf taken from the parameter of exactly its type
    ("op_adder", "missing-error"),  # record payload: ``ctor`` of a ``var``, a literal and a literal
    ("op_adder", "field-zero"),  # the Ok form, k = 1 (RippleCarryResult)
    ("op_adder_n", "missing-error"),  # record payload with the carry taken from the parameter (``with_carry``)
    ("op_adder_n", "field-zero"),
    ("op_alu", "missing-error"),  # AluResult: no numeric field; only missing-error binds it
    ("op_holder", "missing-error"),  # record with a (Bytes,Bool) field and no Bytes parameter (R67, third test)
    ("op_pr", "missing-error"),  # Result<(Bytes,Bool),E> and no Bytes parameter (R67, first test)
    ("op_dir_res", "missing-error"),  # a variant leaf taken from the parameter of exactly its type
    ("op_mk_point", "field-zero"),  # the bare form, k = 0
    ("op_mk_wide", "field-zero"),  # the bare form, k = 0 of type U16
]
PROFILE = (
    "budget 1048576\nchunk_size 16384\nslim_stride auto\n"
    + "".join(f"fault {op} {fault}\n" for op, fault in FAULT_LINES)
)

#: The pure models; each is also a subject op ``op_<name>`` and a reference claim ``c_<name>``:
#: ``(name, parameters, result type, body)``.  The comment of each group says what it pins.
MODELS = [
    # -- Option: Some, a typed None, nesting, a context for None (an if branch, an Ok) --------------------------
    ("some_u8", "a: U8", "Option<U8>", "Some(a)"),
    ("none_u8", "u: Bool", "Option<U8>", "None"),
    ("some_nested", "a: Bool", "Option<Option<Bool>>", "Some(Some(a))"),
    ("some_none", "u: Bool", "Option<Option<Bool>>", "Some(None)"),
    ("opt_if", "c: Bool, a: U8", "Option<U8>", "if c then Some(a) else None"),
    ("opt_bytes", "d: Bytes", "Option<Bytes>", "if len(d) == 0 then None else Some(d)"),
    ("opt_pair", "a: Bool, n: Nat", "Option<(Bool,Nat)>", "Some((a, n))"),
    ("opt_widen_nat", "a: Nat", "Option<Nat>", "Some(a)"),
    # -- structural equality on Option (the type of the left operand gives a None its type) ---------------------
    ("opt_eq", "a: Option<U8>, b: Option<U8>", "Bool", "a == b"),
    ("opt_ne", "a: Option<Bool>, b: Option<Bool>", "Bool", "a != b"),
    ("opt_is_none", "a: Option<U8>", "Bool", "a == None"),
    ("opt_is_some_lit", "a: Option<U8>", "Bool", "Some(1u8) == a"),
    # -- unwrap_or over a binder (adds ir.data) and the intermediate use of ir.fixed (adds none), R29 ----------
    ("unwrap_binder", "o: Option<U8>", "U8", "unwrap_or(o, 5u8)"),
    ("unwrap_nested", "o: Option<Option<Bool>>", "Option<Bool>", "unwrap_or(o, None)"),
    ("inter_add", "a: U8, b: U8", "U8", "unwrap_or(checked_add(a, b), 0u8)"),
    ("prod_eq", "a: U8, b: U8", "Bool", "checked_add(a, b) == Some(a)"),
    ("prod_try", "a: Nat", "Option<U8>", "try<U8>(a)"),
    # -- records: construction, field, nesting, postfix field, structural equality -----------------------------
    ("mk_point", "a: U8, b: U8", "Point", "Point(a, b)"),
    ("px", "p: Point", "U8", "p.x"),
    ("mk_wrapped", "a: U8, b: U8, t: Bool", "Wrapped", "Wrapped(Point(a, b), t)"),
    ("wrapped_y", "w: Wrapped", "U8", "w.p.y"),
    ("mk_slot", "o: Option<U8>, a: Bool", "Slot", "Slot(o, (a, 2))"),
    ("slot_t0", "s: Slot", "Bool", "s.t.0"),
    ("rec_eq", "a: Tri, b: Tri", "Bool", "a == b"),
    ("rec_ne", "a: Duo, b: Duo", "Bool", "a != b"),
    ("rec_postfix", "a: U8, b: U8", "U8", "mk_point(a, b).y"),
    ("mk_wide", "n: Nat, w: U16", "Wide", "Wide(w, n)"),
    ("wide_w", "x: Wide", "U16", "x.w"),
    # -- variants: every constructor shape, structural equality, a payload that is an Option -------------------
    ("mk_circle", "r: U8", "Shape", "Circle(r)"),
    ("mk_rect", "w: U8, h: U8", "Shape", "Rect(w, h)"),
    ("mk_dot", "u: Bool", "Shape", "Dot"),
    ("mk_done", "n: Nat", "Msg", "Done(n, None)"),
    ("mk_opt_msg", "o: Option<U8>", "Msg", "Opt(o)"),
    ("var_eq", "a: Dir, b: Dir", "Bool", "a == b"),
    ("var_ne", "a: Dir, b: Dir", "Bool", "a != b"),
    ("var_eq_lit", "a: Dir", "Bool", "a == Up"),
    # -- match on an Option, a Bool (the data pattern bool), a bare tuple -------------------------------------
    ("m_opt", "o: Option<U8>", "U8", "match o with Some(x) -> x | None -> 0u8"),
    ("m_opt_wild_arm", "o: Option<U8>", "Bool", "match o with None -> false | _ -> true"),
    (
        "m_opt_nested",
        "o: Option<Option<Bool>>",
        "Nat",
        "match o with Some(Some(x)) -> 2 | Some(None) -> 1 | None -> 0",
    ),
    (
        "m_opt_tuple",
        "o: Option<(Bool,Nat)>",
        "Nat",
        "match o with Some((true, n)) -> n | Some((false, _)) -> 0 | None -> 0",
    ),
    ("m_bool", "b: Bool", "Nat", "match b with true -> 1 | false -> 0"),
    ("m_tuple_bool", "t: (Bool,Bool)", "Bool", "match t with (true, x) -> x | (false, _) -> false"),
    # -- match on a record (nested constructor patterns), a variant, a payload-free variant -------------------
    ("m_point", "p: Point", "U8", "match p with Point(x, y) -> x + y"),
    (
        "m_wrapped",
        "w: Wrapped",
        "U8",
        "match w with Wrapped(Point(a, _), true) -> a | Wrapped(Point(_, b), false) -> b",
    ),
    (
        "m_slot",
        "s: Slot",
        "Nat",
        "match s with Slot(Some(_), (true, n)) -> n | Slot(Some(_), (false, _)) -> 1 | Slot(None, _) -> 0",
    ),
    ("m_shape", "s: Shape", "U8", "match s with Circle(r) -> r | Rect(w, h) -> w + h | Dot -> 0u8"),
    ("m_mode", "o: Mode", "Nat", "match o with Add -> 0 | Sub -> 1 | Neg -> 2"),
    (
        "m_msg",
        "m: Msg",
        "Nat",
        "match m with Short -> 0 | Done(n, _) -> n | Opt(Some(x)) -> to_nat(x) | Opt(None) -> 0",
    ),
    ("m_dir_bind", "d: Dir", "Dir", "match d with Up -> Up | other -> other"),
    # -- match on a Result with a data pattern (ir.tuple AND ir.data) ------------------------------------------
    ("m_res_opt", "r: Result<Option<U8>,DataError>", "Nat", "match r with Ok(Some(_)) -> 1 | Ok(None) -> 0 | Err(_) -> 2"),
    (
        "m_res_rec",
        "r: Result<Point,DataError>",
        "U8",
        "match r with Ok(Point(x, _)) -> x | Err(DataError.Bad) -> 0u8 | Err(_) -> 1u8",
    ),
    (
        "m_inner",
        "o: Option<U8>, b: Bool",
        "Nat",
        "match o with Some(_) -> (match b with true -> 1 | false -> 2) | None -> 0",
    ),
]

#: The fault ops: ``(name, parameters, result type, model body)``.  Each is also a pure model ``m_<name>`` and a
#: reference claim ``c_<name>`` (``op(args) == m_name(args)``), so the fault can be seen on the stand-in.  They are
#: the shapes of the two S11 adopters (``tri_state``, ``ripple_carry_adder`` with and without a carry, ``alu``) and
#: of the three cases R67 names.
FAULT_OPS = [
    (
        "tri",
        "data: U8, enable: U8",
        "Result<Option<U8>,DataError>",
        "if enable > 1u8 then Err(DataError.Bad) else Ok(if enable == 1u8 then Some(data) else None)",
    ),
    (
        "pick",
        "o: Option<U8>, a: U8",
        "Result<Option<U8>,DataError>",
        "if a > 200u8 then Err(DataError.Bad) else Ok(o)",
    ),
    (
        "adder",
        "a: Bytes, b: Bytes",
        "Result<Sum,DataError>",
        "if len(a) == len(b) then Ok(Sum(a, if len(a) == 0 then 0u8 else 3u8, len(a) == 1)) else Err(DataError.Bad)",
    ),
    (
        "adder_n",
        "n: U8, a: Bytes",
        "Result<Sum,DataError>",
        "if len(a) > 4 then Err(DataError.Bad) else Ok(Sum(a, n, n == 0u8))",
    ),
    (
        "alu",
        "a: Bytes, m: Mode",
        "Result<AluOut,DataError>",
        "match m with Add -> Ok(AluOut(a, len(a) == 0, false, false, false)) | Sub -> Ok(AluOut(a, false, false, false, false)) | Neg -> Err(DataError.Worse)",
    ),
    (
        "holder",
        "n: Nat",
        "Result<Holder,DataError>",
        'if n > 60 then Err(DataError.Bad) else Ok(Holder(take(b"00ff", n), (take(b"ff", n), n == 3)))',
    ),
    (
        "pr",
        "n: Nat",
        "Result<(Bytes,Bool),DataError>",
        'if n > 60 then Err(DataError.Bad) else Ok((take(b"0102", n), n == 3))',
    ),
    (
        "dir_res",
        "a: U8, d: Dir",
        "Result<Dir,DataError>",
        "if a > 200u8 then Err(DataError.Bad) else Ok(d)",
    ),
]

#: Claims that are not "op(args) == model(args)": data inline in the pure side, a subject call in an arm and as
#: a scrutinee (call counting through a data ``match``), a computed argument (``unproven`` for the faults of
#: ``op_adder``) and a restricted claim whose stand-in expects only ``Err`` (``blind`` for ``field-zero``).
EXTRA_CLAIMS = [
    ("c_inline_match", "forall o: Option<U8>. op_m_opt(o) == (match o with Some(x) -> x | None -> 0u8)"),
    ("c_inline_ctor", "forall a: U8, b: U8. op_mk_point(a, b) == Point(a, b)"),
    ("c_inline_field", "forall p: Point. op_px(p) == p.x"),
    ("c_arm_calls", "forall o: Option<U8>. match o with Some(x) -> op_px(Point(x, x)) == x | None -> true"),
    ("c_scrutinee_calls", "forall a: U8. match op_some_u8(a) with Some(x) -> x == a | None -> false"),
    ("c_option_binder_compare", "forall o: Option<U8>. op_opt_is_none(o) == (o == None)"),
    ("c_adder_computed", "forall a: Bytes, b: Bytes. op_adder(concat(a, b), a) == m_adder(concat(a, b), a)"),
    (
        "c_adder_err_only",
        "forall a: Bytes, b: Bytes | len(a) != len(b). op_adder(a, b) == m_adder(a, b)",
    ),
]

#: Examples: ``(name, body)``.  Values of every data kind in the JSON of section 2.3 and the TFV of section 2.5.
EXAMPLES = [
    ("e_some", "op_some_u8(7u8) == Some(7u8)"),
    ("e_none", "op_none_u8(true) == None"),
    ("e_nested", "op_some_nested(true) == Some(Some(true))"),
    ("e_some_none", "op_some_none(true) == Some(None)"),
    ("e_pair", "op_opt_pair(true, 4) == Some((true, 4))"),
    ("e_point", "op_mk_point(1u8, 2u8) == Point(1u8, 2u8)"),
    ("e_wrapped", "op_mk_wrapped(1u8, 2u8, true) == Wrapped(Point(1u8, 2u8), true)"),
    ("e_slot", "op_mk_slot(Some(9u8), true) == Slot(Some(9u8), (true, 2))"),
    ("e_wide", "op_mk_wide(3, 65535u16) == Wide(65535u16, 3)"),
    ("e_circle", "op_mk_circle(3u8) == Circle(3u8)"),
    ("e_rect", "op_mk_rect(3u8, 4u8) == Rect(3u8, 4u8)"),
    ("e_dot", "op_mk_dot(true) == Dot"),
    ("e_done", "op_mk_done(5) == Done(5, None)"),
    ("e_opt_msg", "op_mk_opt_msg(Some(1u8)) == Opt(Some(1u8))"),
    ("e_ok_record", "op_adder(b\"01\", b\"02\") == Ok(Sum(b\"01\", 3u8, true))"),
    ("e_err", "op_adder(b\"01\", b\"\") == Err(DataError.Bad)"),
    ("e_ok_option", "op_tri(7u8, 1u8) == Ok(Some(7u8))"),
    ("e_ok_none", "op_tri(7u8, 0u8) == Ok(None)"),
    ("e_ok_tuple", 'op_pr(1) == Ok((take(b"0102", 1), false))'),
    ("e_ok_holder", 'op_holder(3) == Ok(Holder(take(b"00ff", 3), (take(b"ff", 3), true)))'),
    ("e_arg_record", "op_px(Point(4u8, 5u8)) == 4u8"),
    ("e_arg_variant", "op_m_shape(Rect(1u8, 2u8)) == 3u8"),
    ("e_arg_option", "op_unwrap_binder(Some(8u8)) == 8u8"),
]


def _names(params: str) -> list[str]:
    return re.findall(r"(?:^|, )(\w+): ", params)


def main() -> str:
    out: list[str] = []
    w = out.append
    ops: list[tuple[str, list[str]]] = []

    def op(line: str) -> None:
        """Declare a subject op and remember its parameters for the contract blocks."""
        w(line)
        name, rest = line.split("op ", 1)[1].split("(", 1)
        ops.append((name.strip(), _names(rest.rsplit(")", 1)[0].split(") ->")[0])))

    w("warrant data_eval v1")
    w("  -- Synthetic ir.data EVALUATION document of slice S11c-1 (VER04r Appendix A.2, 5.1, 8.2): records, variants")
    w("  -- and Option evaluated over their domains, data values in examples, and the faults that need ir.data.")
    w("  -- GENERATED by make_data_eval.py; do not edit by hand.")
    w("  error DataError = Bad | Worse")
    w("  record Point(x: U8, y: U8)")
    w("  record Wrapped(p: Point, tag: Bool)")
    w("  record Slot(o: Option<U8>, t: (Bool,Nat))")
    w("  record Tri(a: Bool, n: Nat)")
    w("  record Duo(a: Bool, b: Bool)")
    w("  record Wide(w: U16, n: Nat)")
    w("  record Sum(sum: Bytes, carry_out: U8, overflow: Bool)")
    w("  record AluOut(result: Bytes, zero: Bool, carry: Bool, negative: Bool, overflow: Bool)")
    w("  record Holder(d: Bytes, pair: (Bytes,Bool))")
    w("  variant Shape = Circle(r: U8) | Rect(w: U8, h: U8) | Dot")
    w("  variant Mode = Add | Sub | Neg")
    w("  variant Dir = Up | Down(f: Bool) | Both(f: Bool, g: Bool)")
    w("  variant Msg = Short | Done(n: Nat, rest: Option<U16>) | Opt(o: Option<U8>)")
    w("")
    w("  -- pure models: one per form of Appendix A.2, then one per fault op ------------------------------")
    for name, params, ret, body in MODELS:
        w(f"  model {name}({params}) -> {ret} = {body}")
    for name, params, ret, body in FAULT_OPS:
        w(f"  model m_{name}({params}) -> {ret} = {body}")
    w("")
    w("  -- subject operations, labels, inputs and bounds -------------------------------------------------")
    for name, params, ret, _body in MODELS:
        op(f"  op op_{name}({params}) -> {ret}")
    for name, params, ret, _body in FAULT_OPS:
        op(f"  op op_{name}({params}) -> {ret}")
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
    w("  -- claims: one reference claim per model (op_<m>(args) == m(args)) --------------------------------")
    for name, params, _ret, _body in MODELS:
        args = ", ".join(_names(params))
        w(f"  claim c_{name}:")
        w(f"    forall {params}. op_{name}({args}) == {name}({args})")
    for name, params, _ret, _body in FAULT_OPS:
        args = ", ".join(_names(params))
        w(f"  claim c_{name}:")
        w(f"    forall {params}. op_{name}({args}) == m_{name}({args})")
    w("")
    w("  -- claims: data inline in the pure side, a subject call in an arm and as the scrutinee -------------")
    for claim, body in EXTRA_CLAIMS:
        w(f"  claim {claim}:")
        w(f"    {body}")
    w("")
    w("  -- examples: Option, record and variant values in JSON and TFV -----------------------------------")
    for ex, body in EXAMPLES:
        w(f"  example {ex}:")
        w(f"    {body}")
    return "\n".join(out) + "\n"  # one final LF and no blank line after it (``git diff --check``)


#: The declarations every refusal document starts from (the types of the fixture document, so a refused signature
#: names a real record or variant).
_BASE = (
    "warrant refusals v1\n"
    "  error E = A | B\n"
    "  record Point(x: U8, y: U8)\n"
    "  record NoNum(a: Bool, b: Bytes<2>)\n"
    "  record Wide(w: U16, n: Nat)\n"
    "  record Holder(d: Bytes, pair: (Bytes,Bool))\n"
    "  record WithDir(a: U8, d: Dir)\n"
    "  variant Dir = Up | Down(f: Bool)\n"
)

#: Signatures a data fault refuses: ``(name, fault, signature line)``.  Every one is a ``generation`` error whose
#: message starts ``fault:`` (the generator's code ``fault``, nothing skipped), as for the faults of S10o-1 and S11f-1.
FAULT_REFUSALS = [
    # field-zero (VER04r 8.2, [D49], R32): a record, or a Result of one, with a numeric field
    ("field-zero/a-scalar-result", "field-zero", "  op f(a: U8) -> U8\n"),
    ("field-zero/a-bytes-result", "field-zero", "  op f(a: U8) -> Bytes\n"),
    ("field-zero/a-tuple-result", "field-zero", "  op f(a: U8) -> (U8,Nat)\n"),
    ("field-zero/a-variant-result", "field-zero", "  op f(a: U8) -> Dir\n"),
    ("field-zero/a-record-with-no-numeric-field", "field-zero", "  op f(a: U8) -> NoNum\n"),
    ("field-zero/an-ok-record-with-no-numeric-field", "field-zero", "  op f(a: U8) -> Result<NoNum,E>\n"),
    ("field-zero/an-option-of-a-record", "field-zero", "  op f(a: U8) -> Option<Point>\n"),
    ("field-zero/a-result-of-a-scalar", "field-zero", "  op f(a: U8) -> Result<U8,E>\n"),
    ("field-zero/a-result-of-a-variant", "field-zero", "  op f(a: U8) -> Result<Dir,E>\n"),
    ("field-zero/a-result-of-an-option-of-a-record", "field-zero", "  op f(a: U8) -> Result<Option<Point>,E>\n"),
    ("field-zero/a-parameter-named-ok-payload-in-the-ok-form", "field-zero", "  op f(ok_payload: U8) -> Result<Point,E>\n"),
    # missing-error (8.2, [D50], R31): a variant with no parameter of its type has no zero
    ("missing-error/a-variant-with-no-parameter-of-its-type", "missing-error", "  op f(a: U8) -> Result<Dir,E>\n"),
    ("missing-error/a-record-field-that-is-a-variant-with-no-parameter", "missing-error", "  op f(a: U8) -> Result<WithDir,E>\n"),
    ("missing-error/a-result-payload-that-is-a-result", "missing-error", "  op f(a: U8) -> Result<Result<U8,E>,E>\n"),
    ("missing-error/a-non-result-result", "missing-error", "  op f(a: U8) -> Point\n"),
    # wrong-ctor waits for its first adopter ([D49]): the name is not in the registry
    ("wrong-ctor/is-not-in-the-registry", "wrong-ctor", "  op f(a: U8) -> Result<Dir,E>\n"),
]

#: Fault rows whose type the oracle compares with the declared one (VER04r 16.17 R67, 16.19 R94), the first comparison
#: there is.  ``(name, signature line, expression)``; ``R`` is ``["var", "result"]``.  The first and third rows are
#: the two ``missing-error`` shapes R67 requires to compile; the second is the same tree with a one-byte literal in
#: a ``(Bytes<2>,Bool)`` position, which must be refused.  The oracle-only rows (the generator's evaluator is untyped,
#: section 16.9 item 3) carry no ``generator_code``.
_R = ["var", "result"]
_B = lambda hex_: ["lit", {"type": "Bytes", "value": hex_}]  # noqa: E731
_IF = lambda payload: ["if", ["is_err", _R], ["ok", payload], _R]  # noqa: E731
TYPING = [
    ("r67/a-tuple-payload-with-a-plain-bytes-zero", "  op f(a: Nat) -> Result<(Bytes,Bool),E>\n", _IF(["tuple", [_B(""), ["lit", False]]]), True),
    ("r67/a-one-byte-literal-in-a-bytes-2-component", "  op f(a: Nat) -> Result<(Bytes<2>,Bool),E>\n", _IF(["tuple", [_B("01"), ["lit", False]]]), False),
    ("r67/a-two-byte-literal-in-a-bytes-2-component", "  op f(a: Nat) -> Result<(Bytes<2>,Bool),E>\n", _IF(["tuple", [_B("0102"), ["lit", False]]]), True),
    (
        "r67/a-record-field-that-is-a-tuple-with-a-plain-bytes-zero",
        "  op f(a: Nat) -> Result<Holder,E>\n",
        _IF(["ctor", "Holder", [_B(""), ["tuple", [_B(""), ["lit", False]]]]]),
        True,
    ),
    (
        "r67/a-one-byte-literal-in-a-record-field-of-bytes-2",
        "  op f(a: Nat) -> Result<Pair2,E>\n",
        _IF(["ctor", "Pair2", [["tuple", [_B("01"), ["lit", False]]]]]),
        False,
    ),
    # R94: a common type, not "flows to": plain Bytes against a declared Bytes<2>; no common type for a Bool against a Bytes
    ("r94/plain-bytes-against-a-declared-bytes-2", "  op f(a: Nat) -> Bytes<2>\n", ["take", _R, ["lit", {"type": "Nat", "value": "1"}]], True),
    ("r94/a-bool-against-a-declared-bytes-result", "  op f(a: Nat) -> Bytes\n", ["lit", True], False),
    ("r94/a-bytes-against-a-declared-u8-result", "  op f(a: Nat) -> U8\n", _B("00"), False),
    # R94 in every branch: the first branch is the declared type, the second is not (the whole used to pass on the first)
    ("r94/a-bool-ok-in-the-else-branch-of-a-u8-result", "  op f(a: Nat) -> Result<U8,E>\n", ["if", ["is_err", _R], _R, ["ok", ["lit", True]]], False),
    (
        "r94/a-bool-ok-in-the-second-match-arm-of-a-u8-result",
        "  op f(a: Nat) -> Result<U8,E>\n",
        ["match", _R, [[["ok", ["bind", "p"]], ["ok", ["var", "p"]]], [["err_any"], ["ok", ["lit", True]]]]],
        False,
    ),
    ("r94/an-int-expression-against-a-declared-nat-result", "  op f(a: Nat) -> Nat\n", ["add", _R, ["lit", {"type": "Nat", "value": "1"}]], True),
]


def refusals() -> dict:
    """Every refused signature and every fault row the typing of R67 and R94 judges, with the oracle's verdict."""
    rows = []
    for name, fault, line in FAULT_REFUSALS:
        source = _BASE + line
        spec = lower_document(parse_source(source.encode()))
        try:
            fault_row(spec.ops["f"], fault, spec.errors, frozenset([*spec.records, *spec.variants]), dict(spec.records))
        except OracleError as exc:
            rows.append({"name": name, "fault": fault, "oracle_code": exc.code, "message": exc.message, "source": source})
        else:  # pragma: no cover - a row of the table that is accepted is a bug of the table
            raise AssertionError(f"{name} was not refused")
    typing = []
    for name, line, expr, accepted in TYPING:
        source = _BASE + "  record Pair2(p: (Bytes<2>,Bool))\n" + line
        spec = lower_document(parse_source(source.encode()))
        op = spec.ops["f"]
        prog = build_program(spec)
        try:
            compiled = _typed(prog, op, expr, {**dict(op.params), "result": op.ret}, op.ret, "expression")
        except OracleError as exc:
            assert not accepted, (name, exc.message)
            typing.append({"name": name, "oracle_code": exc.code, "message": exc.message, "source": source, "expr": expr})
        else:
            assert accepted, name
            typing.append({"name": name, "accepted": True, "type": str(_fault_unify(compiled.ty, op.ret)), "source": source, "expr": expr})
    return {"schema": "warrant-data-faults-refusals-v1", "refusals": rows, "typing": typing}


def build() -> dict[pathlib.Path, bytes]:
    doc = main().encode("utf-8")
    profile = PROFILE.encode("utf-8")
    spec_ref = f"{NAME}.warrant"
    # Both views start from one derivation each: the evaluation view under the gate this slice opens, and the fault
    # view under both gates (the one before it, where every ir.data fault is deferred, and the open one).
    opened = derive(doc, profile, spec_ref=spec_ref, generator={"name": "parity-dump", "version": "0"}, implemented=GATE_DATA)
    assert GATE_BEFORE_DATA != GATE_DATA
    return {
        HERE / f"{NAME}.warrant": doc,
        HERE / f"{NAME}.profile": profile,
        HERE / f"{NAME}.oracle-lowering.json": dump(doc),
        HERE / f"{NAME}.oracle-evaluation.json": dump_evaluation(doc, profile, spec_ref, opened),
        HERE / f"{NAME}.oracle-faults.json": dump_faults(doc, profile, spec_ref, "data"),
        HERE / f"{NAME}.oracle-values.json": canonical(data_values_view(doc)),
        HERE / f"{NAME}.oracle-domains.json": canonical(data_domain_view(doc, profile)),
        HERE / f"{NAME}.oracle-refusals.json": canonical(refusals()),
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
