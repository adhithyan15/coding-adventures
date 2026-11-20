"""``v2-binding``: the lane binding validator of VER04r section 7 (slice S5c table).

A case is the bytes of an ``evidence.bindings`` file (``binding_hex``), the ``corpus`` fragment it is
validated against (``types`` and ``ops``) and the ``host`` object (the four closed lists of names that exist
for the lane: ``values`` per Warrant value name, ``exports``, ``discriminators``, ``immutable``).  The outcome
is ``{"outcome":"accept","normalized":{..}}`` or ``{"outcome":"reject","error":"BindingInvalid","reason":R}``
with ``R`` from the closed set of ``coding_adventures_warrant_oracle.bindings_v2.REASONS``.

Every reject case breaks **exactly one** rule: the builder refuses a case that triggers two violations, so no
case depends on which of two problems an implementation meets first (section 11).  A reject case is the
*valid* base binding of its corpus with one line changed, so the one change is what the case is about.

The rows state the inputs and the intent (``reason`` or the normalized shape written by hand); the validator
derives the outcome, and the build fails naming the case id if the two disagree.

Where the mandatory vectors of section 11 are (R17, the disposition table)
--------------------------------------------------------------------------

* two cases of one op share a discriminator ......... ``discriminator-shared-*``
* ``err_any`` on a two-case error type ............... ``err-any-two-cases``
* CR, a BOM and invalid UTF-8 are cases .............. ``cr-*``, ``bom-*``, ``invalid-utf8-*``
* aliasing, the directive-level part ................. ``fresh-immutable-on-a-mutable-bytes``
                                                       (a ``fresh`` result sharing storage is the runner's) and
                                                       ``inout-*`` (an ``inout`` form is named per parameter)
* the closed lists, the lattice and the error rules .. one case per ``reason`` of the closed set
"""

from __future__ import annotations

from kit_common import row

ROWS: list = []

# -- the lane-neutral host objects -----------------------------------------------------------------------------------------

_INTS = ["int"]
HOST = {
    "values": {
        "Bool": ["bool"],
        "U8": _INTS,
        "U16": _INTS,
        "U32": _INTS,
        "U64": _INTS,
        "I8": _INTS,
        "I16": _INTS,
        "I32": _INTS,
        "I64": _INTS,
        "Nat": _INTS,
        "Bytes": ["bytes", "bytearray"],
        "Tuple": ["tuple"],
        "Option": ["optional"],
        "List": ["list"],
    },
    "exports": [
        "Decoded",
        "Point",
        "bump",
        "ct_eq",
        "ct_eq_fixed",
        "ct_eq_u64",
        "ct_select_bytes",
        "decode",
        "make_point",
        "pick",
        "split",
        "zeroize",
    ],
    "discriminators": ["BaseException", "Exception", "KeyError", "LengthError", "TypeError", "ValueError", "panics"],
    "immutable": ["bool", "bytes", "int", "tuple"],
}

#: A lane whose ``U64`` is ``int64`` only (the OCaml row of section 7.2: ``int`` is refused), with no ``panics``.
HOST_NARROW = {
    "values": {**HOST["values"], "U64": ["int64"], "Nat": ["int64"]},
    "exports": HOST["exports"],
    "discriminators": ["KeyError", "ValueError"],
    "immutable": HOST["immutable"],
}

# -- the corpus fragments ----------------------------------------------------------------------------------------------

CORE = ["ir.core"]
CT = {
    "types": {"SelectError": {"kind": "error", "cases": ["CT_SELECT_LENGTH_MISMATCH"]}},
    "ops": {
        "eq": {"params": [["a", "Bytes"], ["b", "Bytes"]], "ret": "Bool", "requires": CORE},
        "eq_fixed": {
            "params": [["a", "Bytes<N>"], ["b", "Bytes<N>"]],
            "ret": "Bool",
            "width_params": ["N"],
            "requires": CORE,
        },
        "eq_u64": {"params": [["a", "U64"], ["b", "U64"]], "ret": "Bool", "requires": CORE},
        "select_bytes": {
            "params": [["a", "Bytes"], ["b", "Bytes"], ["choice", "Bool"]],
            "ret": "Result<Bytes,SelectError>",
            "fresh": True,
            "requires": CORE,
        },
    },
}
MULTI = {
    "types": {"Pick": {"kind": "error", "cases": ["BAD", "WORSE"]}},
    "ops": {"pick": {"params": [["a", "Bytes"]], "ret": "Result<Bytes,Pick>", "requires": CORE}},
}
INOUT = {
    "types": {},
    "ops": {
        "zeroize": {
            "params": [["buf", "Bytes"]],
            "ret": "Bytes",
            "inout": ["buf"],
            "post_slot": "S1",
            "requires": CORE,
        },
        "bump": {"params": [["n", "U8"]], "ret": "U8", "inout": ["n"], "post_slot": "S1", "requires": CORE},
        "eq": CT["ops"]["eq"],
    },
}
DATA_REQ = ["ir.core", "ir.data", "ir.tuple"]
DATA = {
    "types": {
        "Point": {"kind": "record", "fields": [["x", "U8"], ["y", "U8"]]},
        "Decoded": {"kind": "variant", "ctors": [["Short", []], ["Done", ["Nat", "Bytes"]]]},
        "Fail": {"kind": "error", "cases": ["TOO_SHORT"]},
    },
    "ops": {
        "make_point": {"params": [["x", "U8"], ["y", "U8"]], "ret": "Point", "requires": DATA_REQ},
        "decode": {"params": [["data", "Bytes"]], "ret": "Result<Decoded,Fail>", "requires": DATA_REQ},
        "split": {"params": [["data", "Bytes"]], "ret": "(Bytes,Bool)", "requires": DATA_REQ},
    },
}

# -- the valid base binding of each corpus, as lines; a case replaces, drops or adds one -----------------------------------

HEAD = [
    "lane python",
    "package python/ct-compare",
    "module ct_compare",
    "import coding_adventures_ct_compare",
    "source src/coding_adventures_ct_compare",
    "seed 0x5741_5252_414e_5421",
    "depth slim",
    "levels ir.core",
]
CT_LINES = [
    *HEAD,
    "values Bool=bool Bytes=bytes Nat=int U64=int",
    "op eq export=ct_eq",
    "op eq_fixed export=ct_eq_fixed width=implicit",
    "op select_bytes export=ct_select_bytes errors=CT_SELECT_LENGTH_MISMATCH:ValueError",
    "op eq_u64 export=ct_eq_u64",
]
MULTI_LINES = [*HEAD, "values Bytes=bytes", "op pick export=pick errors=BAD:ValueError,WORSE:KeyError"]
INOUT_LINES = [
    *HEAD,
    "values Bool=bool Bytes=bytes U8=int",
    "op zeroize export=zeroize inout=buf:mutable",
    "op bump export=bump inout=n:cell",
    "op eq export=ct_eq",
]

DATA_LINES = [
    *[("levels ir.core,ir.tuple,ir.data" if line == "levels ir.core" else line) for line in HEAD],
    "values Bool=bool Bytes=bytes Nat=int Tuple=tuple U8=int",
    "type Point host=Point fields=x,y",
    "type Decoded host=Decoded ctors=Short,Done",
    "op make_point export=make_point",
    "op decode export=decode error=TOO_SHORT",
    "op split export=split",
]


# -- helpers: a case is a list of lines (or raw bytes), a corpus, a host and an intent ---------------------------------------


def swap(lines, prefix, new):
    """``lines`` with the one line that starts with ``prefix`` replaced by ``new``."""
    hit = [i for i, line in enumerate(lines) if line.startswith(prefix)]
    assert len(hit) == 1, (prefix, hit)
    return [new if i == hit[0] else line for i, line in enumerate(lines)]


def drop(lines, prefix):
    hit = [i for i, line in enumerate(lines) if line.startswith(prefix)]
    assert len(hit) == 1, (prefix, hit)
    return [line for i, line in enumerate(lines) if i != hit[0]]


def add(lines, *new):
    return [*lines, *new]


def text(lines):
    return "\n".join(lines) + "\n"


def _case(slug, summary, data, corpus, host, intent):
    raw = data if isinstance(data, bytes) else (data if isinstance(data, str) else text(data)).encode("utf-8")
    ROWS.append(row(slug, summary, {"binding_hex": raw.hex(), "corpus": corpus, "host": host}, intent))


def accept(slug, summary, data, corpus, normalized=None, host=HOST):
    intent = {"outcome": "accept"}
    if normalized is not None:
        intent["normalized"] = normalized
    _case(slug, summary, data, corpus, host, intent)


def reject(slug, summary, data, corpus, reason, host=HOST):
    _case(slug, summary, data, corpus, host, {"outcome": "reject", "error": "BindingInvalid", "reason": reason})


SEED = int("57415252414e5421", 16)  # the seed of every base binding, by a different route than the validator's
CT_NORMALIZED = {
    "lane": "python",
    "package": "python/ct-compare",
    "module": "ct_compare",
    "import": "coding_adventures_ct_compare",
    "source": "src/coding_adventures_ct_compare",
    "seed": str(SEED),
    "depth": "slim",
    "levels": ["ir.core"],
    "values": {"Bool": "bool", "Bytes": "bytes", "Nat": "int", "U64": "int"},
    "types": {},
    "ops": {
        "eq": {"export": "ct_eq"},
        "eq_fixed": {"export": "ct_eq_fixed", "width": "implicit"},
        "select_bytes": {
            "export": "ct_select_bytes",
            "errors": {"CT_SELECT_LENGTH_MISMATCH": "ValueError"},
            "fresh": "probe",
        },
        "eq_u64": {"export": "ct_eq_u64"},
    },
}


def with_ops(**changes):
    """``CT_NORMALIZED`` with some op rows replaced."""
    return {**CT_NORMALIZED, "ops": {**CT_NORMALIZED["ops"], **changes}}


SELECT = "op select_bytes export=ct_select_bytes"
ERR_MAP = "errors=CT_SELECT_LENGTH_MISMATCH:ValueError"

# ===================================================================================================================
# accepted bindings, with the normalized form written by hand
# ===================================================================================================================

accept("accept-ct-compare", "the python ct-compare binding of VER04i 5.1 with the v2 depth and levels lines", CT_LINES, CT, CT_NORMALIZED)
accept(
    "accept-comments-order-and-blank-lines",
    "comments (one with non-ASCII text), blank lines and a different line order normalize to the same binding",
    [
        "# a comment may hold any UTF-8: caf\u00e9, \u2014",
        "",
        *reversed(CT_LINES[:8]),
        "",
        "# values and ops come last",
        *CT_LINES[8:],
        "#not a space after the hash",
    ],
    CT,
    CT_NORMALIZED,
)
accept(
    "accept-no-comment-in-the-middle-of-a-line",
    "a # that is not the first character of a line is an ordinary character of a token, not a comment",
    swap(CT_LINES, "import ", "import coding_adventures_ct_compare#1"),
    CT,
    {**CT_NORMALIZED, "import": "coding_adventures_ct_compare#1"},
)
accept(
    "accept-depth-full-and-source-dot",
    "depth full; source . (a package-relative directory itself) is a relative path",
    swap(swap(CT_LINES, "depth ", "depth full"), "source ", "source ."),
    CT,
    {**CT_NORMALIZED, "depth": "full", "source": "."},
)
accept(
    "accept-seed-decimal-maximum",
    "a decimal seed of 2^64-1",
    swap(CT_LINES, "seed ", "seed 18446744073709551615"),
    CT,
    {**CT_NORMALIZED, "seed": "18446744073709551615"},
)
accept(
    "accept-seed-hex-with-separators",
    "a 0x seed with underscores between digit groups normalizes to decimal text",
    swap(CT_LINES, "seed ", "seed 0xFFFF_ffff_FFFF_ffff"),
    CT,
    {**CT_NORMALIZED, "seed": "18446744073709551615"},
)
accept(
    "accept-every-level-name",
    "all seven level names of section 4.1, reserved ones included, in any order, normalize sorted",
    swap(CT_LINES, "levels ", "levels ir.seq,ir.list,ir.inout.records,ir.data,ir.fixed,ir.tuple,ir.core"),
    CT,
    {
        **CT_NORMALIZED,
        "levels": ["ir.core", "ir.data", "ir.fixed", "ir.inout.records", "ir.list", "ir.seq", "ir.tuple"],
    },
)
accept(
    "accept-levels-fixed-and-tuple-are-siblings",
    "ir.fixed and ir.tuple each need ir.core only, not one another",
    swap(CT_LINES, "levels ", "levels ir.tuple,ir.core"),
    CT,
    {**CT_NORMALIZED, "levels": ["ir.core", "ir.tuple"]},
)
accept(
    "accept-width-runtime",
    "width=runtime on an op that has a width parameter",
    swap(CT_LINES, "op eq_fixed ", "op eq_fixed export=ct_eq_fixed width=runtime"),
    CT,
    with_ops(eq_fixed={"export": "ct_eq_fixed", "width": "runtime"}),
)
accept(
    "accept-args-and-ret-overrides",
    "args= and ret= name host representations from the closed lists of the parameter and result types",
    swap(CT_LINES, "op eq export", "op eq export=ct_eq args=bytes,bytearray ret=bool"),
    CT,
    with_ops(eq={"export": "ct_eq", "args": ["bytes", "bytearray"], "ret": "bool"}),
)
accept(
    "accept-fresh-immutable-on-bytes",
    "fresh=immutable where the result's representation (bytes) is immutable by construction",
    swap(CT_LINES, SELECT, f"{SELECT} {ERR_MAP} fresh=immutable"),
    CT,
    with_ops(
        select_bytes={
            "export": "ct_select_bytes",
            "errors": {"CT_SELECT_LENGTH_MISMATCH": "ValueError"},
            "fresh": "immutable",
        }
    ),
)
accept(
    "accept-fresh-probe-on-mutable-bytes",
    "fresh=probe where the representation (bytearray) is mutable",
    swap(swap(CT_LINES, "values ", "values Bool=bool Bytes=bytearray Nat=int U64=int"), SELECT, f"{SELECT} {ERR_MAP} fresh=probe"),
    CT,
    {**CT_NORMALIZED, "values": {"Bool": "bool", "Bytes": "bytearray", "Nat": "int", "U64": "int"}},
)
accept(
    "accept-err-any-on-a-single-case-error",
    "err_any=CASE on an error type of exactly one case",
    swap(CT_LINES, SELECT, f"{SELECT} err_any=CT_SELECT_LENGTH_MISMATCH"),
    CT,
    with_ops(select_bytes={"export": "ct_select_bytes", "err_any": "CT_SELECT_LENGTH_MISMATCH", "fresh": "probe"}),
)
accept(
    "accept-error-panics-on-a-single-case-error",
    "error=CASE (the host panics) on a single-case error type, in a host that has panics",
    swap(CT_LINES, SELECT, f"{SELECT} error=CT_SELECT_LENGTH_MISMATCH"),
    CT,
    with_ops(select_bytes={"export": "ct_select_bytes", "error": "CT_SELECT_LENGTH_MISMATCH", "fresh": "probe"}),
)
accept(
    "accept-two-case-error-with-distinct-discriminators",
    "a two-case error type with a distinct discriminator per case",
    MULTI_LINES,
    MULTI,
    {
        **CT_NORMALIZED,
        "values": {"Bytes": "bytes"},
        "ops": {"pick": {"export": "pick", "errors": {"BAD": "ValueError", "WORSE": "KeyError"}}},
    },
)
accept(
    "accept-inout-forms",
    "one host form per inout parameter: a fixed-length array and a one-element cell",
    INOUT_LINES,
    INOUT,
    {
        **CT_NORMALIZED,
        "values": {"Bool": "bool", "Bytes": "bytes", "U8": "int"},
        "ops": {
            "zeroize": {"export": "zeroize", "inout": {"buf": "mutable"}},
            "bump": {"export": "bump", "inout": {"n": "cell"}},
            "eq": {"export": "ct_eq"},
        },
    },
)
accept(
    "accept-inout-growable-and-returned",
    "the growable and returned forms",
    swap(swap(INOUT_LINES, "op zeroize", "op zeroize export=zeroize inout=buf:growable"), "op bump", "op bump export=bump inout=n:returned"),
    INOUT,
    {
        **CT_NORMALIZED,
        "values": {"Bool": "bool", "Bytes": "bytes", "U8": "int"},
        "ops": {
            "zeroize": {"export": "zeroize", "inout": {"buf": "growable"}},
            "bump": {"export": "bump", "inout": {"n": "returned"}},
            "eq": {"export": "ct_eq"},
        },
    },
)
accept(
    "accept-record-variant-and-tuple",
    "type lines for a record (host field names in corpus order) and a variant (the corpus constructor names, any order)",
    swap(DATA_LINES, "type Decoded", "type Decoded host=Decoded ctors=Done,Short"),
    DATA,
    {
        **CT_NORMALIZED,
        "levels": ["ir.core", "ir.data", "ir.tuple"],
        "values": {"Bool": "bool", "Bytes": "bytes", "Nat": "int", "Tuple": "tuple", "U8": "int"},
        "types": {
            "Point": {"host": "Point", "fields": ["x", "y"]},
            "Decoded": {"host": "Decoded", "ctors": ["Short", "Done"]},
        },
        "ops": {
            "make_point": {"export": "make_point"},
            "decode": {"export": "decode", "error": "TOO_SHORT"},
            "split": {"export": "split"},
        },
    },
)
accept(
    "accept-narrow-host-int64",
    "a host whose U64 is int64 only (section 7.2, the OCaml row) accepts U64=int64",
    swap(CT_LINES, "values ", "values Bool=bool Bytes=bytes Nat=int64 U64=int64"),
    CT,
    {**CT_NORMALIZED, "values": {"Bool": "bool", "Bytes": "bytes", "Nat": "int64", "U64": "int64"}},
    host=HOST_NARROW,
)

_base = text(CT_LINES).encode("ascii")
_room = 65536 - len(_base) - len("# \n")
accept(
    "accept-size-cap-boundary",
    "a binding of exactly 65,536 bytes is within the size cap",
    _base + b"# " + b"x" * _room + b"\n",
    CT,
    CT_NORMALIZED,
)

accept(
    "accept-comment-with-a-line-separator",
    "U+2028 inside a comment is part of the comment: only LF ends a line, so the op line after it is not a line",
    ["# x\u2028op eq export=ct_eq", *CT_LINES],
    CT,
    CT_NORMALIZED,
)
accept(
    "accept-comment-with-a-c1-control",
    "U+0085 (NEL, a C1 control) inside a comment is valid UTF-8 text: only C0 controls and DEL are refused",
    ["# x\u0085op eq export=ct_eq", *CT_LINES],
    CT,
    CT_NORMALIZED,
)
accept(
    "accept-no-final-line-feed",
    "the last line needs no LF after it",
    text(CT_LINES).rstrip("\n"),
    CT,
    CT_NORMALIZED,
)
accept(
    "accept-package-name-with-underscore",
    "a package directory name may hold _ (elixir/gf256_native)",
    swap(swap(CT_LINES, "lane ", "lane elixir"), "package ", "package elixir/gf256_native"),
    CT,
    {**CT_NORMALIZED, "lane": "elixir", "package": "elixir/gf256_native"},
)
accept(
    "accept-package-name-with-capitals",
    "a package directory name may hold capitals and digits (swift/Barcode1D)",
    swap(swap(CT_LINES, "lane ", "lane swift"), "package ", "package swift/Barcode1D"),
    CT,
    {**CT_NORMALIZED, "lane": "swift", "package": "swift/Barcode1D"},
)
accept(
    "accept-package-name-with-a-dot",
    "a package directory name may hold . (csharp/Mosaic.Flux)",
    swap(swap(CT_LINES, "lane ", "lane csharp"), "package ", "package csharp/Mosaic.Flux"),
    CT,
    {**CT_NORMALIZED, "lane": "csharp", "package": "csharp/Mosaic.Flux"},
)
accept(
    "accept-seed-decimal-leading-zeros",
    "leading zeros of a decimal seed mean nothing, however many: only the 20 significant digits are bounded",
    swap(CT_LINES, "seed ", "seed " + "0" * 40 + "7"),
    CT,
    {**CT_NORMALIZED, "seed": "7"},
)
accept(
    "accept-seed-hex-leading-zeros",
    "leading zeros of a hex seed mean nothing: only the 16 significant digits are bounded",
    swap(CT_LINES, "seed ", "seed 0x0000_0000_0000_0000_0000_00ff"),
    CT,
    {**CT_NORMALIZED, "seed": "255"},
)

FRESH_DATA = {
    "types": DATA["types"],
    "ops": {
        **DATA["ops"],
        "make_point": {**DATA["ops"]["make_point"], "fresh": True},
        "split": {**DATA["ops"]["split"], "fresh": True},
    },
}
HOST_POINT_IMMUTABLE = {**HOST, "immutable": [*HOST["immutable"], "Point"]}
DATA_NORMALIZED = {
    **CT_NORMALIZED,
    "levels": ["ir.core", "ir.data", "ir.tuple"],
    "values": {"Bool": "bool", "Bytes": "bytes", "Nat": "int", "Tuple": "tuple", "U8": "int"},
    "types": {
        "Point": {"host": "Point", "fields": ["x", "y"]},
        "Decoded": {"host": "Decoded", "ctors": ["Short", "Done"]},
    },
    "ops": {
        "make_point": {"export": "make_point", "fresh": "probe"},
        "decode": {"export": "decode", "error": "TOO_SHORT"},
        "split": {"export": "split", "fresh": "probe"},
    },
}
MAKE_POINT_IMMUTABLE = swap(DATA_LINES, "op make_point", "op make_point export=make_point fresh=immutable")
accept(
    "accept-fresh-immutable-on-a-record-of-an-immutable-host",
    "fresh=immutable on a record result whose host class the lane lists as immutable by construction",
    MAKE_POINT_IMMUTABLE,
    FRESH_DATA,
    {**DATA_NORMALIZED, "ops": {**DATA_NORMALIZED["ops"], "make_point": {"export": "make_point", "fresh": "immutable"}}},
    host=HOST_POINT_IMMUTABLE,
)
accept(
    "accept-fresh-immutable-on-a-tuple-of-immutable-components",
    "fresh=immutable on a tuple whose container and every component are immutable representations",
    swap(DATA_LINES, "op split", "op split export=split fresh=immutable"),
    FRESH_DATA,
    {**DATA_NORMALIZED, "ops": {**DATA_NORMALIZED["ops"], "split": {"export": "split", "fresh": "immutable"}}},
)

# ===================================================================================================================
# the bytes: size, UTF-8, BOM, CR, control characters
# ===================================================================================================================

reject(
    "too-large-by-one-byte",
    "a binding of 65,537 bytes is refused before it is parsed",
    _base + b"# " + b"x" * (_room + 1) + b"\n",
    CT,
    "too-large",
)
reject("invalid-utf8-lone-continuation", "a comment holding the byte ff", b"# \xff\n" + _base, CT, "invalid-utf8")
reject("invalid-utf8-overlong-slash", "the overlong two-byte form c0 af of a slash", b"# \xc0\xaf\n" + _base, CT, "invalid-utf8")
reject("invalid-utf8-surrogate", "a UTF-8 encoded surrogate, ed a0 80", b"# \xed\xa0\x80\n" + _base, CT, "invalid-utf8")
reject("invalid-utf8-truncated", "a truncated three-byte sequence at the end of the file", _base + b"# \xe2\x82", CT, "invalid-utf8")
reject("bom-at-the-start", "a UTF-8 byte order mark before the first directive", b"\xef\xbb\xbf" + _base, CT, "bom")
reject("cr-in-a-line-ending", "CRLF line endings: the file is LF-only", text(CT_LINES).replace("\n", "\r\n").encode(), CT, "cr")
reject("cr-inside-a-comment", "a carriage return inside a comment", b"# a\rb\n" + _base, CT, "cr")
reject("control-character-tab", "a tab between a directive and its argument", text(swap(CT_LINES, "lane ", "lane\tpython")), CT, "control-character")
reject("control-character-nul", "a NUL byte inside a comment", b"# a\x00b\n" + _base, CT, "control-character")
reject("control-character-del", "DEL (0x7f) inside a comment", b"# a\x7fb\n" + _base, CT, "control-character")
reject("control-character-escape", "ESC (0x1b) inside a comment", b"# a\x1bb\n" + _base, CT, "control-character")

# ===================================================================================================================
# the lines
# ===================================================================================================================

reject("syntax-double-space", "two spaces between tokens", swap(CT_LINES, "lane ", "lane  python"), CT, "syntax")
reject("syntax-leading-space", "a space before the directive", swap(CT_LINES, "lane ", " lane python"), CT, "syntax")
reject("syntax-trailing-space", "a space after the last token", swap(CT_LINES, "lane ", "lane python "), CT, "syntax")
reject("syntax-indented-comment", "a comment is a line whose first character is #: an indented one is a malformed directive line", [" # a note", *CT_LINES], CT, "syntax")
reject("syntax-non-ascii-outside-a-comment", "a non-ASCII character in a module name", swap(CT_LINES, "module ", "module ct_comp\u00e4re"), CT, "syntax")
reject("syntax-directive-without-argument", "depth with no argument", swap(CT_LINES, "depth ", "depth"), CT, "syntax")
reject("syntax-directive-with-two-arguments", "lane with two arguments", swap(CT_LINES, "lane ", "lane python go"), CT, "syntax")
reject("syntax-values-pair-without-equals", "a values item that is not Name=host", swap(CT_LINES, "values ", "values Bool=bool Bytes"), CT, "syntax")
reject("syntax-values-without-pairs", "values with no pairs", swap(CT_LINES, "values ", "values"), CT, "syntax")
reject("syntax-op-key-without-value", "an op key with an empty value", swap(CT_LINES, "op eq export", "op eq export="), CT, "syntax")
reject("syntax-op-without-a-name", "op with no name", add(CT_LINES, "op"), CT, "syntax")
reject("syntax-op-name-not-an-identifier", "an op name that starts with a digit", add(CT_LINES, "op 1eq export=ct_eq"), CT, "syntax")
reject("syntax-levels-trailing-comma", "a levels list with an empty item", swap(CT_LINES, "levels ", "levels ir.core,"), CT, "syntax")
reject("syntax-type-without-a-name", "type with no name", add(DATA_LINES, "type"), DATA, "syntax")
reject("syntax-type-key-without-value", "a type key with an empty value", swap(DATA_LINES, "type Point", "type Point host= fields=x,y"), DATA, "syntax")
reject("unknown-directive-export", "export is a key of an op line, not a directive", add(CT_LINES, "export eq=ct_eq"), CT, "unknown-directive")
reject("unknown-directive-case", "directive names are case-sensitive", swap(CT_LINES, "lane ", "Lane python"), CT, "unknown-directive")
reject("duplicate-directive-lane", "lane twice", add(CT_LINES, "lane python"), CT, "duplicate-directive")
reject("duplicate-directive-levels", "levels twice", add(CT_LINES, "levels ir.core"), CT, "duplicate-directive")
reject("duplicate-directive-values", "values twice", add(CT_LINES, "values Bool=bool"), CT, "duplicate-directive")
reject("duplicate-directive-op", "two op lines for one op", add(CT_LINES, "op eq export=ct_eq"), CT, "duplicate-directive")
reject("duplicate-directive-type", "two type lines for one type", add(DATA_LINES, "type Point host=Point fields=x,y"), DATA, "duplicate-directive")
reject("bad-value-depth", "depth is full or slim", swap(CT_LINES, "depth ", "depth deep"), CT, "bad-value")
reject("bad-value-lane", "a lane is lowercase letters and digits", swap(CT_LINES, "lane ", "lane Python"), CT, "bad-value")
reject("bad-value-package-without-lane", "a package is <lane>/<name>", swap(CT_LINES, "package ", "package ct-compare"), CT, "bad-value")
reject("bad-value-package-of-another-lane", "the package's lane is the lane line's lane", swap(CT_LINES, "package ", "package go/ct-compare"), CT, "bad-value")
reject("bad-value-module", "a module name is an identifier", swap(CT_LINES, "module ", "module ct-compare"), CT, "bad-value")
reject("bad-value-source-parent", "a source path with a .. segment", swap(CT_LINES, "source ", "source ../src"), CT, "bad-value")
reject("bad-value-source-absolute", "an absolute source path", swap(CT_LINES, "source ", "source /src"), CT, "bad-value")
reject("bad-value-source-backslash", "a source path with a backslash", swap(CT_LINES, "source ", "source src\\x"), CT, "bad-value")
reject("bad-value-source-drive", "a source path with a drive colon", swap(CT_LINES, "source ", "source C:src"), CT, "bad-value")
reject("bad-value-source-empty-segment", "a source path with an empty segment", swap(CT_LINES, "source ", "source src//x"), CT, "bad-value")
reject("bad-value-seed-2-to-the-64", "a seed of 2^64 does not fit a u64", swap(CT_LINES, "seed ", "seed 18446744073709551616"), CT, "bad-value")
reject("bad-value-seed-hex-2-to-the-64", "a hex seed of 2^64", swap(CT_LINES, "seed ", "seed 0x1_0000_0000_0000_0000"), CT, "bad-value")
reject("bad-value-seed-empty-hex", "0x with no digits", swap(CT_LINES, "seed ", "seed 0x"), CT, "bad-value")
reject("bad-value-seed-double-underscore", "two underscores in a row", swap(CT_LINES, "seed ", "seed 1__0"), CT, "bad-value")
reject("bad-value-seed-trailing-underscore", "a trailing underscore", swap(CT_LINES, "seed ", "seed 10_"), CT, "bad-value")
reject("bad-value-seed-negative", "a negative seed", swap(CT_LINES, "seed ", "seed -1"), CT, "bad-value")
reject("bad-value-seed-uppercase-prefix", "0X is not the hex prefix", swap(CT_LINES, "seed ", "seed 0X10"), CT, "bad-value")
reject("bad-value-seed-too-many-digits", "a 5,000-digit decimal seed is refused as a value, not met as a crash of the integer conversion", swap(CT_LINES, "seed ", "seed " + "9" * 5000), CT, "bad-value")
reject("bad-value-package-name-leading-dot", "a package directory name starts with a letter or a digit", swap(swap(CT_LINES, "lane ", "lane swift"), "package ", "package swift/.hidden"), CT, "bad-value")
reject("bad-value-repeated-level", "a level named twice", swap(CT_LINES, "levels ", "levels ir.core,ir.core"), CT, "bad-value")

# ===================================================================================================================
# the model: required directives, levels, values, types
# ===================================================================================================================

for _name in ("lane", "package", "module", "import", "source", "seed", "depth", "levels"):
    reject(f"missing-directive-{_name}", f"a binding with no {_name} line", drop(CT_LINES, f"{_name} "), CT, "missing-directive")

reject("level-unknown-name", "a level that section 4.1 does not define", swap(CT_LINES, "levels ", "levels ir.core,ir.quantum"), CT, "level-unknown")
reject("level-unknown-case", "level names are case-sensitive", swap(CT_LINES, "levels ", "levels ir.core,IR.FIXED"), CT, "level-unknown")
reject("level-implication-data-without-tuple", "ir.data implies ir.tuple", swap(CT_LINES, "levels ", "levels ir.core,ir.data"), CT, "level-implication")
reject("level-implication-list-without-data", "ir.list (reserved) implies ir.data", swap(CT_LINES, "levels ", "levels ir.core,ir.tuple,ir.list"), CT, "level-implication")
reject("level-implication-fixed-without-core", "ir.fixed depends on ir.core", swap(CT_LINES, "levels ", "levels ir.fixed"), CT, "level-implication")
reject("level-implication-reserved-seq-without-core", "a reserved level needs ir.core as well", swap(CT_LINES, "levels ", "levels ir.seq"), CT, "level-implication")

reject("values-name-not-a-warrant-value", "Float is no Warrant value name", swap(CT_LINES, "values ", "values Bool=bool Bytes=bytes U64=int Float=int"), CT, "values-name")
reject("values-name-lowercase", "value names are case-sensitive", swap(CT_LINES, "values ", "values Bool=bool Bytes=bytes U64=int bool=bool"), CT, "values-name")
reject("values-name-twice", "a value name bound twice", swap(CT_LINES, "values ", "values Bool=bool Bytes=bytes U64=int Bool=bool"), CT, "values-name")
reject("values-representation-outside-the-list", "boolean is not in the closed list of Bool", swap(CT_LINES, "values ", "values Bool=boolean Bytes=bytes U64=int"), CT, "values-representation")
reject("values-representation-str-for-bytes", "str is not a Bytes representation", swap(CT_LINES, "values ", "values Bool=bool Bytes=str U64=int"), CT, "values-representation")
reject(
    "values-representation-int-for-u64-on-a-narrow-host",
    "a host whose U64 list is [int64] refuses int (section 7.2, OCaml's 63-bit int)",
    swap(CT_LINES, "values ", "values Bool=bool Bytes=bytes U64=int"),
    CT,
    "values-representation",
    host=HOST_NARROW,
)
reject("values-missing-bytes", "the corpus signatures need Bytes and the binding does not bind it", swap(CT_LINES, "values ", "values Bool=bool U64=int"), CT, "values-missing")
reject("values-missing-u64", "eq_u64 needs U64", swap(CT_LINES, "values ", "values Bool=bool Bytes=bytes"), CT, "values-missing")
reject("values-missing-for-a-variant-payload", "the payload of a variant constructor needs Nat", swap(DATA_LINES, "values ", "values Bool=bool Bytes=bytes Tuple=tuple U8=int"), DATA, "values-missing")
reject("values-missing-tuple", "a tuple result needs Tuple=", swap(DATA_LINES, "values ", "values Bool=bool Bytes=bytes Nat=int U8=int"), DATA, "values-missing")
reject("values-missing-for-a-tuple-component", "the components of a tuple need their value names too: (Bytes,Bool) needs Bool", swap(DATA_LINES, "values ", "values Bytes=bytes Nat=int Tuple=tuple U8=int"), DATA, "values-missing")
reject("values-missing-for-a-record-field", "the fields of a record need their value names", swap(DATA_LINES, "values ", "values Bool=bool Bytes=bytes Nat=int Tuple=tuple"), DATA, "values-missing")

reject("type-unknown-name", "a type line for a name the corpus does not declare", add(DATA_LINES, "type Ghost host=Point fields=x"), DATA, "type-unknown")
reject("type-unknown-error-type", "an error type is bound by errors=, not by a type line", add(DATA_LINES, "type Fail host=Point fields=x"), DATA, "type-unknown")
reject("type-shape-missing-host", "a type line with no host=", swap(DATA_LINES, "type Point", "type Point fields=x,y"), DATA, "type-shape")
reject("type-shape-record-with-ctors", "a record takes fields=, not ctors=", swap(DATA_LINES, "type Point", "type Point host=Point ctors=x,y"), DATA, "type-shape")
reject("type-shape-record-with-both", "fields= and ctors= together", swap(DATA_LINES, "type Point", "type Point host=Point fields=x,y ctors=x"), DATA, "type-shape")
reject("type-shape-record-field-count", "one host field name for two corpus fields", swap(DATA_LINES, "type Point", "type Point host=Point fields=x"), DATA, "type-shape")
reject("type-shape-record-repeated-field", "a host field name twice", swap(DATA_LINES, "type Point", "type Point host=Point fields=x,x"), DATA, "type-shape")
reject("type-shape-record-empty-field-name", "an empty item in fields=", swap(DATA_LINES, "type Point", "type Point host=Point fields=x,,y"), DATA, "type-shape")
reject("type-shape-variant-with-fields", "a variant takes ctors=, not fields=", swap(DATA_LINES, "type Decoded", "type Decoded host=Decoded fields=Short,Done"), DATA, "type-shape")
reject("type-shape-variant-other-constructor", "a constructor the corpus does not declare (names, never positions)", swap(DATA_LINES, "type Decoded", "type Decoded host=Decoded ctors=Short,Other"), DATA, "type-shape")
reject("type-shape-variant-missing-constructor", "a variant with a constructor left out", swap(DATA_LINES, "type Decoded", "type Decoded host=Decoded ctors=Short"), DATA, "type-shape")
reject("type-shape-unknown-key", "a type key that is not host, fields or ctors", swap(DATA_LINES, "type Point", "type Point host=Point fields=x,y color=red"), DATA, "type-shape")
reject("type-shape-repeated-key", "host= twice", swap(DATA_LINES, "type Point", "type Point host=Point host=Point fields=x,y"), DATA, "type-shape")
reject("type-missing-variant", "a variant a signature uses has no type line", drop(DATA_LINES, "type Decoded"), DATA, "type-missing")
reject("type-missing-record", "a record a signature uses has no type line", drop(DATA_LINES, "type Point"), DATA, "type-missing")
reject("export-unknown-type-host", "a type host that is no export", swap(DATA_LINES, "type Point", "type Point host=Nowhere fields=x,y"), DATA, "export-unknown")

# ===================================================================================================================
# the ops
# ===================================================================================================================

reject("op-unknown-name", "an op line for an op the corpus does not declare", add(CT_LINES, "op ghost export=ct_eq"), CT, "op-unknown")
reject("op-missing-binding-is-total", "a corpus op with no op line (VER04i section 5 rule 1)", drop(CT_LINES, "op eq_u64"), CT, "op-missing")
reject("op-key-unknown", "an op key that is not one of section 7.1's", swap(CT_LINES, "op eq export", "op eq export=ct_eq speed=fast"), CT, "op-key")
reject("op-key-repeated", "export= twice", swap(CT_LINES, "op eq export", "op eq export=ct_eq export=ct_eq"), CT, "op-key")
reject("export-missing-key", "an op line with no export=", swap(CT_LINES, "op eq export", "op eq args=bytes,bytes"), CT, "export-missing")
reject("export-unknown-name", "an export the host cannot resolve", swap(CT_LINES, "op eq export", "op eq export=nothing_here"), CT, "export-unknown")
reject("args-too-few", "one args item for two parameters", swap(CT_LINES, "op eq export", "op eq export=ct_eq args=bytes"), CT, "args")
reject("args-too-many", "three args items for two parameters", swap(CT_LINES, "op eq export", "op eq export=ct_eq args=bytes,bytes,bool"), CT, "args")
reject("args-representation-not-allowed", "str is not a Bytes representation", swap(CT_LINES, "op eq export", "op eq export=ct_eq args=bytes,str"), CT, "args")
reject("ret-representation-not-allowed", "str is not a Bool representation", swap(CT_LINES, "op eq export", "op eq export=ct_eq ret=str"), CT, "ret")
reject("ret-for-a-result-is-its-ok-type", "ret= of a Result op names its Ok payload's representation", swap(CT_LINES, SELECT, f"{SELECT} {ERR_MAP} ret=bool"), CT, "ret")
reject("width-missing-on-a-width-op", "eq_fixed has a width parameter and the line has no width=", swap(CT_LINES, "op eq_fixed", "op eq_fixed export=ct_eq_fixed"), CT, "width")
reject("width-on-an-op-without-width", "width= on an op with no width parameter", swap(CT_LINES, "op eq export", "op eq export=ct_eq width=implicit"), CT, "width")
reject("width-not-implicit-or-runtime", "width=static is not a v2 value", swap(CT_LINES, "op eq_fixed", "op eq_fixed export=ct_eq_fixed width=static"), CT, "width")

reject("errors-form-two-forms", "errors= and err_any= on one op", swap(CT_LINES, SELECT, f"{SELECT} {ERR_MAP} err_any=CT_SELECT_LENGTH_MISMATCH"), CT, "errors-form")
reject("errors-form-error-and-errors", "error= and errors= on one op", swap(CT_LINES, SELECT, f"{SELECT} {ERR_MAP} error=CT_SELECT_LENGTH_MISMATCH"), CT, "errors-form")
reject("errors-missing-on-a-result-op-line", "select_bytes returns a Result and the line maps no error", swap(CT_LINES, "op select_bytes", "op select_bytes export=ct_select_bytes"), CT, "errors-missing")
reject("errors-unexpected-on-a-plain-op", "eq returns Bool and the line maps an error", swap(CT_LINES, "op eq export", f"op eq export=ct_eq {ERR_MAP}"), CT, "errors-unexpected")
reject("errors-unexpected-err-any-on-a-plain-op", "err_any= on an op that cannot fail", swap(CT_LINES, "op eq export", "op eq export=ct_eq err_any=CT_SELECT_LENGTH_MISMATCH"), CT, "errors-unexpected")
reject("errors-case-not-declared", "a case the error type does not declare", swap(CT_LINES, SELECT, f"{SELECT} errors=NOPE:ValueError"), CT, "errors-case")
reject("errors-case-mapped-twice", "a case mapped twice", swap(MULTI_LINES, "op pick", "op pick export=pick errors=BAD:ValueError,BAD:KeyError,WORSE:TypeError"), MULTI, "errors-case")
reject("errors-case-malformed-pair", "an errors= item that is not CASE:discriminator", swap(MULTI_LINES, "op pick", "op pick export=pick errors=BAD"), MULTI, "errors-case")
reject("errors-case-error-names-another-case", "error= names a case the error type does not declare", swap(CT_LINES, SELECT, f"{SELECT} error=OTHER"), CT, "errors-case")
reject("errors-case-err-any-names-another-case", "err_any= names a case the error type does not declare", swap(CT_LINES, SELECT, f"{SELECT} err_any=OTHER"), CT, "errors-case")
reject("errors-incomplete-leaves-a-case-unmapped", "a two-case error type with one case mapped (the binding is total)", swap(MULTI_LINES, "op pick", "op pick export=pick errors=BAD:ValueError"), MULTI, "errors-incomplete")
reject("discriminator-shared-by-two-cases", "two cases of one op share a discriminator: a wrong error would map to the same case (7.5)", swap(MULTI_LINES, "op pick", "op pick export=pick errors=BAD:ValueError,WORSE:ValueError"), MULTI, "discriminator-shared")
reject("discriminator-shared-in-the-other-order", "the same, written in the other order", swap(MULTI_LINES, "op pick", "op pick export=pick errors=WORSE:KeyError,BAD:KeyError"), MULTI, "discriminator-shared")
reject("discriminator-bare-exception", "a bare Exception would swallow arbitrary failures", swap(CT_LINES, SELECT, f"{SELECT} errors=CT_SELECT_LENGTH_MISMATCH:Exception"), CT, "discriminator-bare")
reject("discriminator-bare-base-exception", "a bare BaseException", swap(CT_LINES, SELECT, f"{SELECT} errors=CT_SELECT_LENGTH_MISMATCH:BaseException"), CT, "discriminator-bare")
reject("discriminator-bare-error", "a bare Error, even where the host does not list it", swap(CT_LINES, SELECT, f"{SELECT} errors=CT_SELECT_LENGTH_MISMATCH:Error"), CT, "discriminator-bare")
reject("discriminator-unknown-class", "a class the host cannot resolve at start-up", swap(CT_LINES, SELECT, f"{SELECT} errors=CT_SELECT_LENGTH_MISMATCH:NoSuchClass"), CT, "discriminator-unknown")
reject(
    "discriminator-unknown-panics-where-the-host-has-none",
    "error= is a panic mapping and the host's list has no panics",
    swap(CT_LINES, SELECT, f"{SELECT} error=CT_SELECT_LENGTH_MISMATCH"),
    CT,
    "discriminator-unknown",
    host={**HOST, "discriminators": ["KeyError", "ValueError"]},
)
reject("err-any-two-cases", "err_any= on a two-case error type would pass a wrong error vacuously (7.5)", swap(MULTI_LINES, "op pick", "op pick export=pick err_any=BAD"), MULTI, "err-any-multi")
reject("panics-form-error-on-two-cases", "error= (a panic) cannot say which of two cases it was", swap(MULTI_LINES, "op pick", "op pick export=pick error=BAD"), MULTI, "panics-form")
reject("panics-form-as-an-errors-discriminator", "panics is only for a single-case error=, not an errors= discriminator", swap(CT_LINES, SELECT, f"{SELECT} errors=CT_SELECT_LENGTH_MISMATCH:panics"), CT, "panics-form")

reject("inout-param-missing-form", "an inout parameter with no host form", swap(INOUT_LINES, "op zeroize", "op zeroize export=zeroize"), INOUT, "inout-param")
reject("inout-param-not-a-parameter", "inout= names a name that is no parameter", swap(INOUT_LINES, "op zeroize", "op zeroize export=zeroize inout=data:mutable"), INOUT, "inout-param")
reject("inout-param-not-inout-in-the-corpus", "inout= on a parameter the corpus does not declare inout", swap(INOUT_LINES, "op eq", "op eq export=ct_eq inout=a:mutable"), INOUT, "inout-param")
reject("inout-param-twice", "a parameter given two forms", swap(INOUT_LINES, "op zeroize", "op zeroize export=zeroize inout=buf:mutable,buf:returned"), INOUT, "inout-param")
reject("inout-param-malformed-pair", "an inout= item that is not param:form", swap(INOUT_LINES, "op zeroize", "op zeroize export=zeroize inout=buf"), INOUT, "inout-param")
reject("inout-form-unknown", "a form that is not one of the four of section 7.6", swap(INOUT_LINES, "op zeroize", "op zeroize export=zeroize inout=buf:inplace"), INOUT, "inout-form")
reject("inout-form-cell-for-bytes", "a cell holds an integer or a Bool, not a Bytes", swap(INOUT_LINES, "op zeroize", "op zeroize export=zeroize inout=buf:cell"), INOUT, "inout-form")
reject("inout-form-growable-for-an-integer", "a growable buffer is a Bytes form, not an integer's", swap(INOUT_LINES, "op bump", "op bump export=bump inout=n:growable"), INOUT, "inout-form")
reject("inout-form-mutable-for-an-integer", "a mutable byte array is a Bytes form, not an integer's", swap(INOUT_LINES, "op bump", "op bump export=bump inout=n:mutable"), INOUT, "inout-form")

reject("fresh-on-an-op-that-is-not-fresh", "fresh= on an op the corpus does not mark fresh", swap(CT_LINES, "op eq export", "op eq export=ct_eq fresh=probe"), CT, "fresh")
reject("fresh-value-unknown", "fresh= is probe or immutable", swap(CT_LINES, SELECT, f"{SELECT} {ERR_MAP} fresh=sometimes"), CT, "fresh")
reject(
    "fresh-immutable-on-a-mutable-bytes",
    "fresh=immutable where Bytes is bytearray: accepted only where the type is immutable by construction (7.4)",
    swap(swap(CT_LINES, "values ", "values Bool=bool Bytes=bytearray Nat=int U64=int"), SELECT, f"{SELECT} {ERR_MAP} fresh=immutable"),
    CT,
    "fresh",
)
reject(
    "fresh-immutable-on-a-mutable-ret-override",
    "fresh=immutable with a ret= override that names a mutable representation",
    swap(CT_LINES, SELECT, f"{SELECT} {ERR_MAP} ret=bytearray fresh=immutable"),
    CT,
    "fresh",
)
reject(
    "fresh-immutable-on-a-record-of-a-mutable-host",
    "fresh=immutable on a record result whose host class is not in the lane's immutable list",
    MAKE_POINT_IMMUTABLE,
    FRESH_DATA,
    "fresh",
)
reject(
    "fresh-immutable-on-a-tuple-with-a-mutable-component",
    "an immutable tuple holding a bytearray is writable through it: every component must be immutable too",
    swap(swap(DATA_LINES, "values ", "values Bool=bool Bytes=bytearray Nat=int Tuple=tuple U8=int"), "op split", "op split export=split fresh=immutable"),
    FRESH_DATA,
    "fresh",
)
