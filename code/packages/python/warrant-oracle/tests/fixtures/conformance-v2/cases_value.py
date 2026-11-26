"""``v2-value``: the section 2.3 decoder then the section 2.4 encoder (slice S5a table).

Each row is ``(slug, summary, input text or bytes, type, types, intent)``; the oracle reads the bytes
with the strict JSON reader, parses the type, decodes by declared type and writes the bare stream.
The stream intents below are written by hand from the section 2.4 table: ``10`` plus eight little-endian
bytes for an unsigned value, ``11`` plus eight two's-complement bytes for a signed one, ``20`` plus a
u32 length for ``Bytes``, ``50``/``51`` for ``Ok``/``Err``.  Only ``ir.core`` forms appear here
(tuples, records, variants, ``Option`` and ``List`` values belong to S10d and S11, section 11 R17).
"""

from __future__ import annotations

from kit_common import hi, lo, row, u32le, word

ERR = {"E": {"kind": "error", "cases": ["BAD", "WORSE"]}}
ROWS: list = []


def _add(slug, summary, text, ty, intent, types=None, requires=None):
    data = text if isinstance(text, bytes) else text.encode("utf-8")
    fields = {"type": ty, "types": types if types is not None else {}, "input_hex": data.hex()}
    ROWS.append(row(slug, summary, fields, intent, requires))


def accept(slug, summary, text, ty, stream, types=None, requires=None):
    _add(slug, summary, text, ty, {"outcome": "accept", "stream_hex": stream}, types, requires)


def accept_digest(slug, summary, text, ty, types=None, requires=None):
    _add(slug, summary, text, ty, {"outcome": "accept"}, types, requires)


def reject(slug, summary, text, ty, code, types=None, requires=None):
    _add(slug, summary, text, ty, {"outcome": "reject", "error": code}, types, requires)


def q(text: str) -> str:
    return '"' + text + '"'


# -- Bool ----------------------------------------------------------------------------------------
accept("bool-true", "a Bool is a bare JSON boolean", "true", "Bool", "01")
accept("bool-false", "false", "false", "Bool", "00")
accept("bool-with-whitespace", "JSON whitespace around the value is allowed", " \n\ttrue\r\n", "Bool", "01")
reject("bool-string", "the string true is not a Bool", q("true"), "Bool", "value")
reject("bool-number", "the number 1 is not a Bool", "1", "Bool", "value")
reject("bool-null", "null is not a Bool", "null", "Bool", "value")

# -- signed integers: the stream value is sign-extended to 64 bits ---------------------------------
accept("i8-minus-one", "I8 -1 is 11 then ff x 8", q("-1"), "I8", "11" + "ff" * 8)
for t in ("I8", "I16", "I32", "I64"):
    k = t.lower()
    accept(f"{k}-min", f"{t} minimum", q(str(lo(t))), t, "11" + word(lo(t)))
    accept(f"{k}-max", f"{t} maximum", q(str(hi(t))), t, "11" + word(hi(t)))
    reject(f"{k}-below-min", f"{t} minimum minus one is out of range", q(str(lo(t) - 1)), t, "value")
    reject(f"{k}-above-max", f"{t} maximum plus one is out of range", q(str(hi(t) + 1)), t, "value")
accept("i8-zero", "I8 zero", q("0"), "I8", "11" + word(0))
reject("i8-negative-zero", "-0 is refused even for a signed type", q("-0"), "I8", "value")

# -- unsigned integers ---------------------------------------------------------------------------
for t in ("U8", "U16", "U32", "U64"):
    k = t.lower()
    accept(f"{k}-max", f"{t} maximum", q(str(hi(t))), t, "10" + word(hi(t)))
    reject(f"{k}-above-max", f"{t} maximum plus one is out of range", q(str(hi(t) + 1)), t, "value")
    reject(f"{k}-negative", f"a negative value for {t}", q("-1"), t, "value")
accept("u8-zero", "U8 zero", q("0"), "U8", "10" + word(0))
accept("u8-single-digit", "U8 seven", q("7"), "U8", "10" + word(7))

# -- Nat -------------------------------------------------------------------------------------------
accept("nat-zero", "Nat zero", q("0"), "Nat", "10" + word(0))
accept("nat-max", "a Nat value is at most 2^64-1", q(str(2**64 - 1)), "Nat", "10" + word(2**64 - 1))
reject("nat-above-max", "2^64 is an IR intermediate, not a Nat value", q(str(2**64)), "Nat", "value")
reject("nat-negative", "a negative Nat", q("-1"), "Nat", "value")

# -- decimal text ----------------------------------------------------------------------------------
reject("decimal-leading-zero", "a leading zero is refused", q("01"), "U8", "value")
reject("decimal-double-zero", "00 is a leading zero", q("00"), "U8", "value")
reject("decimal-plus-sign", "a plus sign is refused", q("+1"), "U8", "value")
reject("decimal-21-digits", "a 21-digit value is refused whatever its magnitude", q("1" * 21), "U64", "value")
reject("decimal-empty", "the empty string is not a number", q(""), "U8", "value")
reject("decimal-fraction", "1.0 is not decimal text of an integer", q("1.0"), "U8", "value")
reject("decimal-leading-space", "a leading space is refused", q(" 1"), "U8", "value")
reject("decimal-trailing-space", "a trailing space is refused", q("1 "), "U8", "value")
reject("decimal-hex-prefix", "0x10 is not decimal", q("0x10"), "U8", "value")
reject("decimal-lone-minus", "a lone minus sign", q("-"), "I8", "value")
reject("decimal-exponent", "1e2 is not decimal text", q("1e2"), "U8", "value")
reject("decimal-json-number", "a JSON number where decimal text is required", "1", "U8", "value")
reject("decimal-json-number-signed", "a JSON number for a signed type", "-1", "I8", "value")
reject("decimal-array", "an array is not an integer", '["1"]', "U8", "value")

# -- Bytes -------------------------------------------------------------------------------------------
accept("bytes-empty", "the empty Bytes", q(""), "Bytes", "20" + u32le(0))
accept("bytes-one", "one byte", q("ab"), "Bytes", "20" + u32le(1) + "ab")
accept("bytes-escaped-hex", "a JSON escape decodes before the hex check", '"\\u0061b"', "Bytes", "20" + u32le(1) + "ab")
accept_digest("bytes-4096", "4,096 bytes is the largest Bytes value (digest only)", q("ab" * 4096), "Bytes")
reject("bytes-4097", "4,097 bytes is over the limit", q("ab" * 4097), "Bytes", "value")
reject("bytes-odd-length", "an odd number of hex digits", q("abc"), "Bytes", "value")
reject("bytes-uppercase", "upper-case hex is refused", q("AB"), "Bytes", "value")
reject("bytes-non-hex", "a non-hex digit", q("zz"), "Bytes", "value")
reject("bytes-prefix", "a 0x prefix is not hex", q("0xab"), "Bytes", "value")
reject("bytes-array", "an array of numbers is not Bytes", "[1]", "Bytes", "value")

# -- Bytes<N> ----------------------------------------------------------------------------------------
accept("bytes-n-0", "Bytes<0> holds the empty string", q(""), "Bytes<0>", "20" + u32le(0))
accept("bytes-n-1", "Bytes<1> holds one byte", q("ff"), "Bytes<1>", "20" + u32le(1) + "ff")
accept("bytes-n-16", "Bytes<16> holds sixteen bytes", q("00" * 16), "Bytes<16>", "20" + u32le(16) + "00" * 16)
accept_digest("bytes-n-4096", "Bytes<4096> holds 4,096 bytes (digest only)", q("01" * 4096), "Bytes<4096>")
reject("bytes-n-short", "one byte short of Bytes<2>", q("ff"), "Bytes<2>", "value")
reject("bytes-n-long", "one byte too long for Bytes<1>", q("ffff"), "Bytes<1>", "value")
reject("bytes-n-0-long", "Bytes<0> refuses one byte", q("00"), "Bytes<0>", "value")
reject("bytes-n-4096-short", "4,095 bytes for Bytes<4096>", q("01" * 4095), "Bytes<4096>", "value")
reject("bytes-n-leading-zero-type", "a Bytes width is canonical decimal: Bytes<07> is a type error (R45 c)", q(""), "Bytes<07>", "type")
reject("bytes-n-4097-type", "N over 4,096 is a type error, whatever the value", q(""), "Bytes<4097>", "type")
reject("bytes-n-named-width", "a named width is not a value type", q(""), "Bytes<N>", "type")

# -- Result<T,E> -------------------------------------------------------------------------------------
accept("result-ok-true", "Ok(true): 50 then the inner value", '{"ok":true}', "Result<Bool,E>", "50" + "01", ERR)
accept("result-ok-false", "Ok(false)", '{"ok":false}', "Result<Bool,E>", "50" + "00", ERR)
accept("result-err-first", "Err(BAD): 51, u32 length, the case name", '{"err":"BAD"}', "Result<Bool,E>",
       "51" + u32le(3) + b"BAD".hex(), ERR)
accept("result-err-second", "Err(WORSE): cases are compared by name", '{"err":"WORSE"}', "Result<Bool,E>",
       "51" + u32le(5) + b"WORSE".hex(), ERR)
accept("result-ok-u8", "Ok of an unsigned value", '{"ok":"7"}', "Result<U8,E>", "50" + "10" + word(7), ERR)
accept("result-ok-bytes", "Ok of Bytes", '{"ok":"0102"}', "Result<Bytes,E>", "50" + "20" + u32le(2) + "0102", ERR)
accept("result-ok-signed", "Ok of a signed value is sign-extended", '{"ok":"-2"}', "Result<I8,E>", "50" + "11" + word(-2), ERR)
accept("result-nested-err", "Ok(Err(BAD)) of a nested Result", '{"ok":{"err":"BAD"}}', "Result<Result<Bool,E>,E>",
       "50" + "51" + u32le(3) + b"BAD".hex(), ERR)
accept("result-nested-ok", "Ok(Ok(false))", '{"ok":{"ok":false}}', "Result<Result<Bool,E>,E>", "50" + "50" + "00", ERR)
reject("result-unknown-case", "an Err case that is not in E", '{"err":"NOPE"}', "Result<Bool,E>", "value", ERR)
reject("result-case-is-lowercase", "case names are exact", '{"err":"bad"}', "Result<Bool,E>", "value", ERR)
reject("result-both-keys", "ok and err together", '{"ok":true,"err":"BAD"}', "Result<Bool,E>", "value", ERR)
reject("result-no-key", "neither key", "{}", "Result<Bool,E>", "value", ERR)
reject("result-err-number", "err takes a case name, not a number", '{"err":1}', "Result<Bool,E>", "value", ERR)
reject("result-unknown-key", "an unknown key", '{"fine":true}', "Result<Bool,E>", "value", ERR)
reject("result-ok-wrong-inner", "the Ok payload must fit T", '{"ok":"1"}', "Result<Bool,E>", "value", ERR)
reject("result-not-object", "a Result is an object", "true", "Result<Bool,E>", "value", ERR)
reject("result-error-type-missing", "E is not in types", '{"ok":true}', "Result<Bool,X>", "type", ERR)
reject("result-error-type-not-error", "E names a type that is not an error type", '{"ok":true}', "Result<Bool,U8>", "type", ERR)

# -- depth of a type expression (section 2.2 rule 1) -----------------------------------------------
# A bare name is depth 0 (the generator's resolve counts the same way) and every Result< adds one, so n
# wrappers are depth n: 15 and 16 wrappers are accepted, 17 refused.
def nested(n: int) -> str:
    return "Result<" * n + "Bool" + ",E>" * n


accept("type-depth-16-wrappers", "a type expression of 16 Result wrappers is exactly at the depth limit",
       '{"ok":' * 16 + "true" + "}" * 16, nested(16), "50" * 16 + "01", ERR)
accept("type-depth-15-wrappers", "a type expression of 15 Result wrappers is within depth 16",
       '{"ok":' * 15 + "true" + "}" * 15, nested(15), "50" * 15 + "01", ERR)
reject("type-depth-17-wrappers", "a type expression of 17 Result wrappers is refused as a type",
       "true", nested(17), "type", ERR)
reject("type-depth-40-wrappers", "a much deeper type expression", "true", nested(40), "type", ERR)

# -- type text ------------------------------------------------------------------------------------------
reject("type-int", "Int is IR-only: never a value type (section 2.1)", q("1"), "Int", "type")
reject("type-u128", "U128 is not a type of section 2.1", q("1"), "U128", "type")
reject("type-lowercase", "type names are case-sensitive", q("1"), "u8", "type")
reject("type-unknown-name", "a name that is not in types", "true", "Foo", "type")
reject("type-empty", "empty type text", "true", "", "type")
reject("type-unclosed", "an unclosed Result", '{"ok":true}', "Result<Bool,E", "type", ERR)

# -- malformed JSON: the strict reader of VER04i section 9.1 refuses the bytes ----------------------------
for slug, summary, data in [
    ("trailing-comma", "a trailing comma in an array", b"[true,]"),
    ("bom", "a UTF-8 byte order mark", b"\xef\xbb\xbftrue"),
    ("invalid-utf8", "an invalid UTF-8 byte", b"\xff"),
    ("overlong-utf8", "an overlong UTF-8 encoding of a quote", b'"\xc0\xa2"'),
    ("empty-input", "no bytes at all", b""),
    ("leading-zero-number", "a JSON number with a leading zero", b"01"),
    ("fraction", "a fraction is refused whatever the type", b"1.5"),
    ("exponent", "an exponent is refused", b"1e2"),
    ("minus-zero", "-0 is refused", b"-0"),
    ("nan", "NaN", b"NaN"),
    ("single-quotes", "single-quoted string", b"'true'"),
    ("trailing-data", "data after the value", b"true false"),
    ("unterminated-string", "an unterminated string", b'"ab'),
    ("control-char-in-string", "a raw control character inside a string", b'"a\nb"'),
    ("lone-surrogate-escape", "a lone surrogate escape", b'"\\ud800"'),
    ("bad-escape", "an unknown escape", b'"\\q"'),
    ("unsafe-integer", "an integer past 2^53-1", b"9007199254740992"),
    ("unquoted-key", "an object key without quotes", b"{ok:true}"),
]:
    reject(f"malformed-{slug}", summary, data, "Bool", "malformed-json")
reject("malformed-duplicate-key", "a duplicate object key", b'{"ok":true,"ok":false}', "Result<Bool,E>",
       "malformed-json", ERR)
reject("malformed-duplicate-key-escaped", "a duplicate key after unescaping", b'{"ok":true,"\\u006fk":false}',
       "Result<Bool,E>", "malformed-json", ERR)
reject("malformed-wins-over-type", "a malformed document is malformed-json and nothing else", b"[", "Bool", "malformed-json")


# ===============================================================================================
# S10d (VER04r section 13, R17): tuple values, stream tag 64, at ``ir.tuple``
# ===============================================================================================
# A tuple value is ``64``, a u32-LE component count, then the components in order, each as its own value (section 2.4);
# the case's ``requires`` is ``["ir.core", "ir.tuple"]`` (R9: a tuple type is ``ir.tuple``).  A ``type`` error has no type to
# level, so it stays ``["ir.core"]``: a lane at ``ir.core`` runs those and must say ``type`` for a tuple type it cannot decode.
TU = ["ir.core", "ir.tuple"]


def tag64(*components):
    """``64``, the u32-LE count, then the components' own stream bytes (each already a hex string)."""
    return "64" + u32le(len(components)) + "".join(components)


def Un(v):
    return "10" + word(v)


def Sn(v):
    return "11" + word(v)


def By(hexstring):
    return "20" + u32le(len(hexstring) // 2) + hexstring


BTRUE, BFALSE = "01", "00"


def tacc(slug, summary, text, ty, stream, types=None):
    accept(slug, summary, text, ty, stream, types, requires=TU)


def trej(slug, summary, text, ty, code, types=None):
    reject(slug, summary, text, ty, code, types, requires=TU if code != "type" else ["ir.core"])


tacc("tuple-pair", "a pair of a U8 and a Bool: 64, a count of 2, then each component", '["1",true]', "(U8,Bool)",
     tag64(Un(1), BTRUE))
tacc("tuple-pair-with-json-whitespace", "JSON whitespace between the components", ' [ "1" ,\n true ] ', "(U8,Bool)",
     tag64(Un(1), BTRUE))
tacc("tuple-arity-3", "a triple of Bool", "[true,false,true]", "(Bool,Bool,Bool)", tag64(BTRUE, BFALSE, BTRUE))
tacc("tuple-arity-4", "four U8", '["1","2","3","4"]', "(U8,U8,U8,U8)", tag64(*(Un(k) for k in range(1, 5))))
tacc("tuple-arity-5", "five Nat", '["0","1","2","3","4"]', "(Nat,Nat,Nat,Nat,Nat)", tag64(*(Un(k) for k in range(5))))
tacc("tuple-arity-6", "six I8", '["0","-1","-2","-3","-4","-5"]', "(I8,I8,I8,I8,I8,I8)", tag64(*(Sn(-k) for k in range(6))))
tacc("tuple-arity-7", "seven Bool", "[true,false,true,false,true,false,true]", "(Bool,Bool,Bool,Bool,Bool,Bool,Bool)",
     tag64(*(BTRUE if k % 2 == 0 else BFALSE for k in range(7))))
tacc("tuple-arity-8", "eight components, the most a tuple type may have (section 14)", '["0","1","2","3","4","5","6","7"]',
     "(U8,U8,U8,U8,U8,U8,U8,U8)", tag64(*(Un(k) for k in range(8))))
tacc("tuple-nested-first", "a tuple whose first component is a tuple: the inner one carries its own 64 and count",
     '[["1","2"],true]', "((U8,U8),Bool)", tag64(tag64(Un(1), Un(2)), BTRUE))
tacc("tuple-nested-last", "a tuple whose last component is a tuple", '[true,["1","2"]]', "(Bool,(U8,U8))",
     tag64(BTRUE, tag64(Un(1), Un(2))))
tacc("tuple-nested-three-deep", "a tuple in a tuple in a tuple", '[[["1","2"],"3"],"4"]', "(((U8,U8),U8),U8)",
     tag64(tag64(tag64(Un(1), Un(2)), Un(3)), Un(4)))
tacc("tuple-of-nat-and-signed", "the Nat maximum and a signed -1: each component keeps its own tag (10 and 11)",
     '["18446744073709551615","-1"]', "(Nat,I8)", tag64(Un(2**64 - 1), Sn(-1)))
tacc("tuple-with-bytes-component", "a Bytes component is 20, its length and the bytes", '["0a0b","7"]', "(Bytes,U8)",
     tag64(By("0a0b"), Un(7)))
tacc("tuple-with-an-empty-bytes-component", "an empty Bytes component", '["","7"]', "(Bytes,U8)", tag64(By(""), Un(7)))
tacc("tuple-with-a-sized-bytes-component", "a Bytes<2> component", '["0a0b",true]', "(Bytes<2>,Bool)", tag64(By("0a0b"), BTRUE))
tacc("tuple-with-an-ok-component", "a Result component holding an Ok: 50 then the payload", '[{"ok":"1"},false]', "(Result<U8,E>,Bool)",
     tag64("50" + Un(1), BFALSE), ERR)
tacc("tuple-with-an-err-component", "a Result component holding an Err: 51, the length of the case name, the name",
     '[{"err":"BAD"},true]', "(Result<U8,E>,Bool)", tag64("51" + u32le(3) + b"BAD".hex(), BTRUE), ERR)
tacc("tuple-as-the-payload-of-an-ok", "a Result whose Ok holds a tuple", '{"ok":["1",true]}', "Result<(U8,Bool),E>",
     "50" + tag64(Un(1), BTRUE), ERR)
tacc("tuple-under-an-err-result", "a Result<(U8,Bool),E> that is an Err has no tuple in it", '{"err":"WORSE"}', "Result<(U8,Bool),E>",
     "51" + u32le(5) + b"WORSE".hex(), ERR)
tacc("tuple-with-a-nested-result-of-a-tuple", "a tuple whose component is a Result of a tuple", '[{"ok":["1","2"]},true]',
     "(Result<(U8,U8),E>,Bool)", tag64("50" + tag64(Un(1), Un(2)), BTRUE), ERR)
accept_digest("tuple-with-a-long-bytes-component", "a component of 1,100 bytes makes the stream longer than 2,048 hex characters, "
              "so stream_hex is omitted and the digest stands alone", '["' + "ab" * 1100 + '",true]', "(Bytes,Bool)", requires=TU)


def _deep(k):
    text, value, stream = "U8", '"1"', Un(1)
    for _ in range(k):
        text, value, stream = f"({text},U8)", f"[{value},\"2\"]", tag64(stream, Un(2))
    return text, value, stream


_t16, _v16, _s16 = _deep(16)
tacc("tuple-type-depth-16-nested-tuples", "16 nested tuples are exactly at the type depth limit (section 2.2 rule 1)", _v16, _t16, _s16)
reject("tuple-type-depth-17-nested-tuples", "17 nested tuples are refused as a type", _deep(17)[1], _deep(17)[0], "type",
       requires=["ir.core"])
reject("tuple-type-depth-17-through-mixed-wrappers", "a tuple under 16 Result wrappers is 17 deep: refused as a type",
       "true", "Result<" * 16 + "(Bool,Bool)" + ",E>" * 16, "type", ERR, requires=["ir.core"])

# -- refused: the value is not the declared tuple ----------------------------------------------------------------------------------------
trej("tuple-too-few-components", "one component for a pair", '["1"]', "(U8,Bool)", "value")
trej("tuple-too-many-components", "three components for a pair", '["1",true,true]', "(U8,Bool)", "value")
trej("tuple-empty-array", "the empty array for a pair", "[]", "(U8,Bool)", "value")
trej("tuple-is-an-object", "a tuple is an array, not an object of positions", '{"0":"1","1":true}', "(U8,Bool)", "value")
trej("tuple-is-a-string", "a string for a tuple", '"1"', "(U8,Bool)", "value")
trej("tuple-is-null", "null for a tuple", "null", "(U8,Bool)", "value")
trej("tuple-component-out-of-range", "256 is not a U8", '["256",true]', "(U8,Bool)", "value")
trej("tuple-component-wrong-form", "a Bool component written as text", '["1","true"]', "(U8,Bool)", "value")
trej("tuple-component-leading-zero", "decimal text with a leading zero in a component", '["01",true]', "(U8,Bool)", "value")
trej("tuple-component-negative-for-unsigned", "a negative U8 component", '["-1",true]', "(U8,Bool)", "value")
trej("tuple-component-of-the-wrong-bytes-length", "one byte for a Bytes<2> component", '["0a",true]', "(Bytes<2>,Bool)", "value")
trej("tuple-nested-component-wrong-arity", "the inner pair has three components", '[["1","2","3"],true]', "((U8,U8),Bool)", "value")
trej("tuple-nested-component-not-an-array", "the inner pair is a scalar", '["1",true]', "((U8,U8),Bool)", "value")
trej("tuple-err-case-not-declared", "an Err case in a Result component that E does not declare", '[{"err":"NOPE"},true]',
     "(Result<U8,E>,Bool)", "value", ERR)
trej("tuple-ok-payload-wrong-arity", "an Ok that holds a triple where a pair is declared", '{"ok":["1","2","3"]}', "Result<(U8,U8),E>",
     "value", ERR)
trej("tuple-malformed-trailing-comma", "a trailing comma in the array is malformed JSON, whatever the type", '["1",true,]', "(U8,Bool)",
     "malformed-json")
trej("tuple-malformed-wins-over-value", "malformed JSON comes before the value check", '["1",', "(U8,Bool)", "malformed-json")

# -- refused: the type text is not a tuple type of section 2.1 ----------------------------------------------------------------------------
trej("tuple-type-of-one-component", "a tuple type has 2 to 8 components: one is refused", '["1"]', "(U8)", "type")
trej("tuple-type-of-nine-components", "nine components", "[" + ",".join(["true"] * 9) + "]", "(" + ",".join(["Bool"] * 9) + ")", "type")
trej("tuple-type-empty", "the empty tuple type", "[]", "()", "type")
trej("tuple-type-with-int", "Int is never a component of a value type (section 2.1)", '["1","2"]', "(U8,Int)", "type")
trej("tuple-type-with-an-unknown-name", "a component type that is not a type", '["1","2"]', "(U8,Frob)", "type")
trej("tuple-type-unclosed", "an unclosed tuple type", '["1","2"]', "(U8,U8", "type")
trej("tuple-type-trailing-comma", "a trailing comma in a tuple type", '["1","2"]', "(U8,U8,)", "type")
trej("tuple-type-with-a-space", "the canonical type text has no space after the comma", '["1","2"]', "(U8, U8)", "type")
