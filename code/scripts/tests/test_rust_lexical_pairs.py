"""Tests for `rust_lexical_pairs.py`, the RS01 slice A4 pair generator.

What is pinned here, and why each group exists:

* `XidData`: the Unicode data is checked in and decoded from a crate's trie, so
  the decoder is tested on a hand-built trie, and the committed ranges are
  cross-checked against two independent sources (the crate source in the cargo
  registry when it is present, and Python's own `unicodedata`).
* `RawStringArms`: a regex cannot count, so the raw-string arms are tested
  against a port of `rustc_lexer`'s scanner on EVERY string over a small
  alphabet. That is the test that would have caught a wrong arm.
* `Lexicon`: every generated regex compiles, and a tiny first-match lexer over
  the definitions (Python's `re` has the same leftmost-first alternation as the
  Rust `regex` crate for these constructs) pins the hard cases of RS01 4.2 per
  edition. The real engine is exercised by the slice's own corpus run.
* `Files`: determinism, headers, and `check` against the real tree and a
  mutated copy of it.
"""

from __future__ import annotations

import contextlib
import io
import itertools
import re
import shutil
import sys
import tempfile
import unicodedata
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO / "code" / "scripts"))

import rust_lexical_pairs as rlp
import rust_release_pairs as pairs

RUST = REPO / "code" / "grammars" / "rust"
REGISTRY_TABLES = sorted(
    Path.home().glob(".cargo/registry/src/*/unicode-ident-1.0.24/src/tables.rs")
)


def to_python(pattern: str) -> str:
    """The Rust `regex` syntax this generator uses, as Python `re` syntax."""
    return pattern.replace("\\z", "\\Z")


def real_classes() -> rlp.Classes:
    return rlp.Classes.build(*rlp.load_xid(RUST))


class Escapes(unittest.TestCase):
    def test_ascii_alphanumerics_stay_literal(self) -> None:
        self.assertEqual(rlp.escape(ord("a")), "a")
        self.assertEqual(rlp.escape(ord("_")), "_")
        self.assertEqual(rlp.escape(ord("7")), "7")

    def test_other_code_points_are_numeric_escapes_both_regex_dialects_read(
        self,
    ) -> None:
        self.assertEqual(rlp.escape(0x2D), "\\x2D")
        self.assertEqual(rlp.escape(0xE9), "\\xE9")
        self.assertEqual(rlp.escape(0x2028), "\\u2028")
        self.assertEqual(rlp.escape(0x1F600), "\\U0001F600")
        for cp in (0x2D, 0xE9, 0x2028, 0x1F600, 0x5D, 0x5C, 0x2F):
            self.assertTrue(re.fullmatch(f"[{rlp.escape(cp)}]", chr(cp)))

    def test_class_body_orders_and_collapses(self) -> None:
        self.assertEqual(
            rlp.class_body([(0x61, 0x61), (0x63, 0x64), (0x70, 0x7A)]), "acdp-z"
        )

    def test_slash_escaped_only_touches_bare_slashes(self) -> None:
        self.assertEqual(rlp.slash_escaped("//x"), "\\/\\/x")
        self.assertEqual(rlp.slash_escaped("a\\/b"), "a\\/b")
        self.assertEqual(rlp.slash_escaped("\\\\/"), "\\\\\\/")
        self.assertEqual(rlp.slash_escaped("[^*\\/]"), "[^*\\/]")

    def test_union_code_points_merges_neighbours(self) -> None:
        self.assertEqual(
            rlp.union_code_points([(0x41, 0x5A), (0x61, 0x7A)], {0x5F}),
            [(0x41, 0x5A), (0x5F, 0x5F), (0x61, 0x7A)],
        )
        self.assertEqual(
            rlp.union_code_points([(0x41, 0x5A)], {0x5B, 0x40}), [(0x40, 0x5B)]
        )


class XidData(unittest.TestCase):
    def synthetic_tables(self) -> str:
        """A 2-chunk trie: code points 0x80..0x87 are start, 0x80..0x8F continue."""
        # CHUNK = 4 means 4 leaf bytes per trie chunk entry... the decoder uses
        # `chunk * CHUNK // 2 + cp // 8 % CHUNK`, so with CHUNK = 2 one leaf byte
        # covers 8 code points and one chunk covers 16.
        return """
        pub(crate) const ASCII_START: u128 = 0x7fffffe07fffffe0000000000000000;
        pub(crate) const ASCII_CONTINUE: u128 = 0x7fffffe87fffffe03ff000000000000;
        pub(crate) const CHUNK: usize = 2;
        pub(crate) static TRIE_START: Align8<[u8; 9]> = Align8([
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02,
        ]);
        pub(crate) static TRIE_CONTINUE: Align8<[u8; 9]> = Align8([
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02,
        ]);
        pub(crate) static LEAF: Align64<[u8; 4]> = Align64([
            0x00, 0x00, 0xFF, 0xFF,
        ]);
        """

    def test_decoder_follows_the_crates_lookup(self) -> None:
        # TRIE index = cp // 8 // 2 = 8 for cp 0x80..0x8F; its chunk is 2; the
        # leaf offset is 2 * 2 // 2 + (cp // 8) % 2 = 2 + (0 or 1): 0xFF, 0xFF.
        start, cont = rlp.decode_unicode_ident(self.synthetic_tables())
        self.assertEqual(start[0], (0x41, 0x5A))
        self.assertEqual(start[1], (0x61, 0x7A))
        self.assertEqual(start[2], (0x80, 0x8F))
        self.assertEqual(cont[0], (0x30, 0x39))
        self.assertEqual(cont[-1], (0x80, 0x8F))

    def test_decoder_rejects_a_table_whose_length_lies(self) -> None:
        broken = self.synthetic_tables().replace("[u8; 4]", "[u8; 5]")
        with self.assertRaisesRegex(ValueError, "declares 5 bytes"):
            rlp.decode_unicode_ident(broken)

    def test_decoder_rejects_a_missing_table(self) -> None:
        with self.assertRaisesRegex(ValueError, "no table"):
            rlp.decode_unicode_ident("pub(crate) const CHUNK: usize = 2;")

    def test_data_file_round_trips(self) -> None:
        start, cont = [(0x41, 0x5A)], [(0x30, 0x39), (0x41, 0x5A)]
        self.assertEqual(
            rlp.parse_xid_data(rlp.render_xid_data(start, cont)), (start, cont)
        )

    def test_data_file_validation(self) -> None:
        good = "S 0041 005A\nC 0030 0039\n"
        self.assertEqual(rlp.parse_xid_data(good), ([(0x41, 0x5A)], [(0x30, 0x39)]))
        for text, why in [
            ("S 0041 005A\n", "both an S and a C"),
            ("S 005A 0041\nC 0030 0039\n", "out of order"),
            ("S 0041 005A\nS 0050 0060\nC 0030 0039\n", "sorted, disjoint"),
            ("S 0041 005A\nS 005B 005C\nC 0030 0039\n", "merged"),
            ("S 0041 11FFFF\nC 0030 0039\n", "out of range"),
            ("S D7FF D800\nC 0030 0039\n", "surrogates"),
            ("X 0041 005A\nC 0030 0039\n", "S|C"),
            ("S 0041\nC 0030 0039\n", "S|C"),
            ("S zz 005A\nC 0030 0039\n", "hexadecimal"),
        ]:
            with self.assertRaisesRegex(ValueError, why):
                rlp.parse_xid_data(text)

    def test_committed_data_facts(self) -> None:
        start, cont = rlp.load_xid(RUST)

        def has(ranges: list[rlp.Range], char: str) -> bool:
            return any(a <= ord(char) <= b for a, b in ranges)

        self.assertTrue(has(start, "a") and has(start, "é") and has(start, "日"))
        self.assertFalse(
            has(start, "_"), "`_` is added by id_start, it is not XID_Start"
        )
        self.assertFalse(has(start, "0"))
        self.assertTrue(has(cont, "0") and has(cont, "_") and has(cont, "·"))
        self.assertFalse(has(cont, "-") or has(cont, " ") or has(cont, "퟿"))
        # XID_Start is a subset of XID_Continue.
        for first, last in start:
            self.assertTrue(
                any(a <= first and last <= b for a, b in cont),
                f"{first:X}-{last:X} is XID_Start but not XID_Continue",
            )

    @unittest.skipIf(
        tuple(int(part) for part in unicodedata.unidata_version.split("."))
        > (17, 0, 0),
        "this Python's Unicode is newer than the pinned 17.0.0",
    )
    def test_committed_data_contains_pythons_older_unicode(self) -> None:
        """Unicode only adds XID characters (stability), so Python's set is inside ours."""
        start, cont = rlp.load_xid(RUST)

        def table(ranges: list[rlp.Range]) -> set[int]:
            return {cp for a, b in ranges for cp in range(a, b + 1)}

        starts, continues = table(start), table(cont)
        missing: list[str] = []
        for cp in range(0x80, 0x30000):
            char = chr(cp)
            if 0xD800 <= cp < 0xE000:
                continue
            if char.isidentifier() and cp not in starts:
                missing.append(f"start {cp:X}")
            if ("a" + char).isidentifier() and cp not in continues:
                missing.append(f"continue {cp:X}")
        self.assertLess(
            len(missing),
            20,
            f"Unicode {unicodedata.unidata_version} has XID characters missing here: {missing[:5]}",
        )

    @unittest.skipUnless(
        REGISTRY_TABLES, "unicode-ident 1.0.24 is not in the cargo registry"
    )
    def test_committed_data_is_the_pinned_crates_table(self) -> None:
        import hashlib

        data = REGISTRY_TABLES[0].read_bytes()
        self.assertEqual(hashlib.sha256(data).hexdigest(), rlp.TABLES_SHA256)
        self.assertEqual(rlp.decode_unicode_ident(data.decode()), rlp.load_xid(RUST))

    def test_import_xid_refuses_a_table_that_is_not_the_pinned_one(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tables = Path(tmp) / "tables.rs"
            tables.write_text(self.synthetic_tables())
            errors = rlp.import_xid(tables, Path(tmp))
            self.assertTrue(errors and "not the pinned" in errors[0])


def reference_raw_scan(text: str, max_hashes: int) -> int | None:
    """`rustc_lexer::raw_string_unvalidated`, ported: the end of `r#..."..."#..`
    at the start of `text` (after the `r`), or None if it is not a raw string."""
    i = 0
    hashes = 0
    while i < len(text) and text[i] == "#":
        hashes += 1
        i += 1
    if hashes > max_hashes or i >= len(text) or text[i] != '"':
        return None
    i += 1
    while True:
        quote = text.find('"', i)
        if quote == -1:
            return None
        i = quote + 1
        seen = 0
        while i < len(text) and text[i] == "#" and seen < hashes:
            seen += 1
            i += 1
        if seen == hashes:
            return i


class RawStringArms(unittest.TestCase):
    def test_arms_match_rustc_lexers_scanner_on_every_short_string(self) -> None:
        max_hashes = 3
        arms = "|".join(rlp.raw_string_arm(n) for n in range(max_hashes + 1))
        pattern = re.compile(f"(?:{arms})")
        checked = 0
        for length in range(10):
            for chars in itertools.product('#"a', repeat=length):
                text = "".join(chars)
                want = reference_raw_scan(text, max_hashes)
                found = pattern.match(text)
                got = found.end() if found else None
                self.assertEqual(got, want, f"r{text!r}")
                checked += 1
        self.assertGreater(checked, 29000)

    def test_the_first_fence_wins(self) -> None:
        pattern = re.compile(
            "(?:" + "|".join(rlp.raw_string_arm(n) for n in range(4)) + ")"
        )
        self.assertEqual(pattern.match('#"a"##').end(), 5)  # r#"a"# then a `#`
        self.assertEqual(pattern.match('##"a"#b"##z').end(), 10)
        self.assertEqual(pattern.match('##"a"#"##').end(), 9)

    def test_carriage_returns_must_be_part_of_crlf(self) -> None:
        pattern = re.compile(
            "(?:" + "|".join(rlp.raw_string_arm(n) for n in range(3)) + ")"
        )
        self.assertIsNotNone(pattern.match('"a\r\nb"'))
        self.assertIsNone(pattern.match('"a\rb"'))
        self.assertIsNotNone(pattern.match('#"a\r\n"\r\n"#'))

    def test_the_real_bound_is_255_and_exact(self) -> None:
        classes = rlp.Classes.build([(0x61, 0x7A)], [(0x61, 0x7A)])
        full = re.compile(rlp.raw_string_pattern("r", classes))
        self.assertTrue(full.match("r" + "#" * 255 + '"x"' + "#" * 255))
        self.assertIsNone(full.match("r" + "#" * 256 + '"x"' + "#" * 256))
        self.assertTrue(full.match('r#"a"#suffix'))
        self.assertEqual(full.match('r#"a"#suffix').end(), 12)

    def test_arm_size_is_linear_not_quadratic(self) -> None:
        sizes = [len(rlp.raw_string_arm(n)) for n in (10, 100, 255)]
        self.assertLess(sizes[2], 400)  # counted repetition, not 255 literal hashes
        self.assertGreater(len(rlp.raw_string_pattern("r", real_classes())), 15000)


def first_match_lex(lex: rlp.Lexicon, text: str) -> list[str]:
    """Tokenize `text` with the default-group definitions, first match wins.

    Whitespace is skipped. A `@input_start` guard holds at offset 0. Refusal
    tokens are reported as `!NAME` and end the run, as the engine does; a position nothing matches gives `?`. Group
    and transition behaviour is the engine's; this pins the regexes.
    """
    compiled = [
        (d, re.compile(to_python(d.pattern) if d.regex else re.escape(d.pattern)))
        for d in lex.defs
    ]
    skip = [re.compile(to_python(d.pattern)) for d in lex.skip if d.guard is None]
    out: list[str] = []
    pos = 0
    while pos < len(text):
        gap = next((m for rx in skip if (m := rx.match(text, pos))), None)
        if gap:
            pos = gap.end()
            continue
        for definition, rx in compiled:
            if definition.guard == "input_start" and pos != 0:
                continue
            found = rx.match(text, pos)
            if found:
                prefix = "!" if definition.refusal else ""
                out.append(f"{prefix}{definition.effective}:{found.group()}")
                pos = found.end()
                if definition.refusal:
                    return out  # the engine stops at a refused token
                break
        else:
            out.append(f"?:{text[pos]}")
            break
    return out


class Lexicon(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        classes = real_classes()
        cls.lex = {y: rlp.build_lexicon(rlp.edition(y), classes) for y in rlp.EDITIONS}

    def lex_in(self, year: str, text: str) -> list[str]:
        return first_match_lex(self.lex[year], text)

    def test_every_regex_compiles_and_names_are_unique(self) -> None:
        for year, lexicon in self.lex.items():
            names = [d.name for d in [*lexicon.skip, *lexicon.defs]]
            names += [d.name for g in lexicon.groups for d in g.defs]
            self.assertEqual(len(names), len(set(names)), year)
            for definition in [
                *lexicon.skip,
                *lexicon.defs,
                *(d for g in lexicon.groups for d in g.defs),
            ]:
                if definition.regex:
                    re.compile(to_python(definition.pattern))

    def test_transitions_name_real_groups_and_tokens(self) -> None:
        for year, lexicon in self.lex.items():
            groups = {g.name for g in lexicon.groups}
            names = set(lexicon.effective_names())
            for rule in lexicon.transitions:
                for group in re.findall(r"(?:push|set-mode|in) (\w+)", rule):
                    self.assertIn(group, groups | {"default"}, f"{year}: {rule[:60]}")
                head = rule.split(" -> ")[0]
                for token in re.findall(r"\b[A-Z][A-Z_]+\b", head.replace("on ", "")):
                    self.assertIn(token, names, f"{year}: {token}")
            for refusal in lexicon.refusal_names():
                self.assertIn(refusal, lexicon.transitions[0], f"{year}: {refusal}")

    def test_editions_differ_exactly_where_rs01_says(self) -> None:
        names = {y: set(lx.effective_names()) for y, lx in self.lex.items()}
        self.assertEqual(names["2015"], names["2018"])
        self.assertEqual(
            names["2021"] - names["2018"],
            {
                "C_STR_LIT",
                "RAW_C_STR_LIT",
                "RAW_LIFETIME",
                "RAW_LIFETIME_BAD",
                "LIFETIME_PREFIX",
                "RESERVED_PREFIX",
            },
        )
        self.assertEqual(names["2024"] - names["2021"], {"GUARDED_PREFIX"})
        self.assertEqual(names["2018"] - names["2024"], set())

    def test_keyword_sets_per_edition(self) -> None:
        kw = {y: set(lx.keywords) for y, lx in self.lex.items()}
        for word in ("async", "await", "dyn", "try"):
            self.assertNotIn(word, kw["2015"])
            self.assertIn(word, kw["2018"])
        self.assertNotIn("gen", kw["2021"])
        self.assertIn("gen", kw["2024"])
        for year in rlp.EDITIONS:
            for word in ("fn", "Self", "abstract", "yield", "macro", "unsized"):
                self.assertIn(word, kw[year])
            for word in ("union", "macro_rules", "raw", "safe", "auto", "default"):
                self.assertNotIn(word, kw[year], "weak keywords are soft, not promoted")
        self.assertIn("dyn", self.lex["2015"].soft_keywords)
        self.assertNotIn("dyn", self.lex["2018"].soft_keywords)

    def test_literals_and_names(self) -> None:
        e = "2021"
        self.assertEqual(self.lex_in(e, 'r#"a"# b'), ['RAW_STR_LIT:r#"a"#', "NAME:b"])
        self.assertEqual(self.lex_in(e, 'r#"a"##'), ['RAW_STR_LIT:r#"a"#', "POUND:#"])
        self.assertEqual(self.lex_in(e, "r#match"), ["RAW_IDENT:r#match"])
        self.assertEqual(self.lex_in(e, 'br"a"x'), ['RAW_BYTE_STR_LIT:br"a"x'])
        self.assertEqual(
            self.lex_in(e, "b'a' b\"s\""), ["BYTE_LIT:b'a'", 'BYTE_STR_LIT:b"s"']
        )
        self.assertEqual(
            self.lex_in(e, "'a' 'a 'ab' '\\n'"),
            ["CHAR_LIT:'a'", "LIFETIME:'a", "CHAR_LIT:'ab'", "CHAR_LIT:'\\n'"],
        )
        self.assertEqual(
            self.lex_in(e, "<'a,'b>"),
            ["LT:<", "LIFETIME:'a", "COMMA:,", "LIFETIME:'b", "GT:>"],
        )
        self.assertEqual(self.lex_in(e, "xé 日本"), ["NAME:xé", "NAME:日本"])
        self.assertEqual(self.lex_in(e, '"a\\"b"s'), ['STR_LIT:"a\\"b"s'])
        self.assertEqual(self.lex_in(e, '"a'), ['!STR_BAD:"'])
        self.assertEqual(self.lex_in(e, 'r#"a'), ["!RAW_BAD:r#"])
        self.assertEqual(self.lex_in(e, "r#1"), ["!RAW_BAD:r#"])

    def test_numbers(self) -> None:
        e = "2021"
        for text, want in [
            ("1", ["INT_LIT:1"]),
            ("1u8", ["INT_LIT:1u8"]),
            ("0x1F_u8", ["INT_LIT:0x1F_u8"]),
            ("1_000u32", ["INT_LIT:1_000u32"]),
            ("1.0", ["FLOAT_LIT:1.0"]),
            ("1.5e-3f64", ["FLOAT_LIT:1.5e-3f64"]),
            ("1e3", ["FLOAT_LIT:1e3"]),
            ("1e_5", ["FLOAT_LIT:1e_5"]),
            ("1f32", ["INT_LIT:1f32"]),
            ("1.foo()", ["INT_LIT:1", "DOT:.", "NAME:foo", "LPAREN:(", "RPAREN:)"]),
            ("1..2", ["INT_LIT:1", "DOT:.", "DOT:.", "INT_LIT:2"]),
            ("1.", ["INT_LIT:1", "DOT:."]),
            ("1.e3", ["INT_LIT:1", "DOT:.", "NAME:e3"]),
            ("0b102", ["INT_LIT:0b102"]),
            ("1e", ["!FLOAT_BAD_EXP:1e"]),
            ("1e+", ["!FLOAT_BAD_EXP:1e+"]),
            ("1.5e", ["!FLOAT_BAD_EXP:1.5e"]),
            ("1else", ["!FLOAT_BAD_EXP:1el"]),
            ("1e_+", ["!FLOAT_BAD_EXP:1e_+"]),
            ("0b1e3", ["!BASED_FLOAT_BAD:0b1e"]),
            ("0x1.5", ["!BASED_FLOAT_BAD:0x1.5"]),
            (
                "0x1.max(2)",
                [
                    "INT_LIT:0x1",
                    "DOT:.",
                    "NAME:max",
                    "LPAREN:(",
                    "INT_LIT:2",
                    "RPAREN:)",
                ],
            ),
        ]:
            self.assertEqual(self.lex_in(e, text), want, text)

    def test_exponent_rule_agrees_with_rustc_on_all_short_numbers(self) -> None:
        """`1e`-style inputs over a small alphabet, against a port of `number()`."""
        lexicon = self.lex["2021"]

        def reference(text: str) -> str:
            # decimal only: `D [. D] [e [sign] _* digit ...] suffix`
            m = re.match(r"[0-9][0-9_]*", text)
            if not m:
                return "none"
            i = m.end()
            if text[i : i + 1] == "." and re.match(r"\.[0-9]", text[i : i + 2]):
                i += 1 + re.match(r"[0-9][0-9_]*", text[i + 1 :]).end()
            if text[i : i + 1] in ("e", "E"):
                j = i + 1
                if text[j : j + 1] in ("+", "-"):
                    j += 1
                digits = re.match(r"[0-9_]*", text[j:]).group()
                return "float" if re.search(r"[0-9]", digits) else "bad"
            return "other"

        for length in range(1, 6):
            for chars in itertools.product("01e+-_.", repeat=length):
                text = "".join(chars)
                if not text[0].isdigit():
                    continue
                want = reference(text)
                got = first_match_lex(lexicon, text)[0]
                if want == "bad":
                    self.assertTrue(
                        got.startswith("!FLOAT_BAD_EXP"), f"{text!r}: {got}"
                    )
                elif want == "float":
                    self.assertTrue(got.startswith("FLOAT_LIT"), f"{text!r}: {got}")
                elif want == "other":
                    self.assertFalse(
                        got.startswith("!FLOAT_BAD_EXP"), f"{text!r}: {got}"
                    )

    def test_comment_tokens(self) -> None:
        e = "2021"
        self.assertEqual(self.lex_in(e, "/// x"), ["OUTER_DOC:/// x"])
        self.assertEqual(self.lex_in(e, "///"), ["OUTER_DOC:///"])
        self.assertEqual(self.lex_in(e, "//// x"), ["LINE_COMMENT://// x"])
        self.assertEqual(self.lex_in(e, "//! x"), ["INNER_DOC://! x"])
        self.assertEqual(self.lex_in(e, "// x"), ["LINE_COMMENT:// x"])
        self.assertEqual(self.lex_in(e, "/*! x"), ["INNER_BC_OPEN:/*!", "NAME:x"])
        self.assertEqual(self.lex_in(e, "/** x"), ["OUTER_BC_OPEN:/** ", "NAME:x"])
        self.assertEqual(
            self.lex_in(e, "/**/"),
            ["BC_OPEN:/*", "STAR:*", "SLASH:/"],
            "`*/` is closed inside the group",
        )
        self.assertEqual(self.lex_in(e, "/*x"), ["BC_OPEN:/*", "NAME:x"])

    def test_edition_differences_in_the_token_stream(self) -> None:
        self.assertEqual(
            self.lex_in("2015", 'k#x k"s" c"s"'),
            [
                "NAME:k",
                "POUND:#",
                "NAME:x",
                "NAME:k",
                'STR_LIT:"s"',
                "NAME:c",
                'STR_LIT:"s"',
            ],
        )
        self.assertEqual(self.lex_in("2018", "k#x"), ["NAME:k", "POUND:#", "NAME:x"])
        self.assertEqual(self.lex_in("2021", "k#x"), ["!RESERVED_PREFIX:k#"])
        self.assertEqual(self.lex_in("2021", 'k"s"'), ['!RESERVED_PREFIX:k"'])
        self.assertEqual(self.lex_in("2021", 'c"s"'), ['C_STR_LIT:c"s"'])
        self.assertEqual(self.lex_in("2021", 'cr#"s"#'), ['RAW_C_STR_LIT:cr#"s"#'])
        self.assertEqual(
            self.lex_in("2018", 'cr#"s"#'),
            ["NAME:cr", "POUND:#", 'STR_LIT:"s"', "POUND:#"],
        )
        self.assertEqual(self.lex_in("2021", "'r#a"), ["RAW_LIFETIME:'r#a"])
        self.assertEqual(
            self.lex_in("2018", "'r#a"), ["LIFETIME:'r", "POUND:#", "NAME:a"]
        )
        self.assertEqual(self.lex_in("2021", "'a#"), ["!LIFETIME_PREFIX:'a#"])
        self.assertEqual(self.lex_in("2024", '#"x"#'), ['!GUARDED_PREFIX:#"'])
        self.assertEqual(self.lex_in("2024", "##"), ["!GUARDED_PREFIX:##"])
        self.assertEqual(self.lex_in("2021", "##"), ["POUND:#", "POUND:#"])
        self.assertEqual(self.lex_in("2021", "---\nx"), ["!FRONTMATTER:---"])
        self.assertEqual(
            self.lex_in("2021", "a ---"), ["NAME:a", "MINUS:-", "MINUS:-", "MINUS:-"]
        )

    def test_transition_table_shape(self) -> None:
        lexicon = self.lex["2021"]
        self.assertEqual(
            lexicon.transitions[1:5],
            [
                "on (INNER_BC_OPEN | OUTER_BC_OPEN) -> push doc_bc",
                "on BC_OPEN in doc_bc -> refuse",
                "on BC_OPEN -> push bc",
                "on BC_CLOSE -> pop",
            ],
        )
        self.assertIn('on RAW_IDENT="r#crate" -> refuse', lexicon.transitions)
        self.assertIn('on RAW_IDENT="r#_" -> refuse', lexicon.transitions)
        self.assertIn("on DOT in default -> set-mode after_dot", lexicon.transitions)
        self.assertEqual(
            {g.name for g in lexicon.groups},
            {"bc", "doc_bc", "after_dot", "after_dots"},
        )
        self.assertTrue(
            all(g.defs for g in lexicon.groups), "grammar-tools rejects an empty group"
        )


class Files(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        errors: list[str] = []
        cls.context = rlp.load_context(RUST, errors)
        assert cls.context is not None, errors
        cls.files = rlp.generate(cls.context, errors)
        assert not errors, errors

    def test_the_four_pairs_are_generated(self) -> None:
        self.assertEqual(
            sorted(self.files),
            sorted(
                f"rust1.97.0-e{y}{s}"
                for y in rlp.EDITIONS
                for s in (".tokens", ".grammar")
            ),
        )

    def test_generation_is_deterministic(self) -> None:
        errors: list[str] = []
        self.assertEqual(rlp.generate(self.context, errors), self.files)

    def test_headers_name_the_release_and_edition_and_cite_the_evidence(self) -> None:
        text = self.files["rust1.97.0-e2021.tokens"].decode()
        lines = text.split("\n")
        self.assertEqual(lines[0], "# Rust 1.97.0 (2026-07-09), edition 2021: tokens.")
        self.assertEqual(lines[1], "# @version 1")
        self.assertIn(
            "# source_ref: https://github.com/rust-lang/rust/tree/1.97.0", lines
        )
        self.assertIn("# source_sha: 2d8144b7880597b6e6d3dfd63a9a9efae3f533d3", lines)
        self.assertTrue(
            any(
                line.startswith("# evidence: compiler/rustc_lexer/src/lib.rs=")
                for line in lines
            )
        )
        self.assertIn("# basis: reviewed", lines)
        grammar = self.files["rust1.97.0-e2015.grammar"].decode().split("\n")
        self.assertEqual(
            grammar[0], "# Rust 1.97.0 (2026-07-09), edition 2015: grammar."
        )

    def test_files_are_plain_text_without_tabs_or_carriage_returns(self) -> None:
        for name, data in self.files.items():
            text = data.decode("utf-8")
            self.assertTrue(text.endswith("\n") and not text.endswith("\n\n"), name)
            self.assertNotIn("\r", text, name)
            self.assertNotIn("\t", text, name)
            self.assertTrue(
                text.isascii(),
                f"{name}: escapes, never raw non-ASCII, keep it reviewable",
            )

    def test_grammar_shape(self) -> None:
        text = self.files["rust1.97.0-e2021.grammar"].decode()
        body = [line for line in text.split("\n") if line and not line.startswith("#")]
        self.assertTrue(body[0].startswith("file = { tt } ;"))
        leaf = text[text.index("leaf = ") :]
        for delimiter in rlp.DELIMITERS:
            self.assertNotIn(f"| {delimiter}", leaf)
        for never in ("LINE_COMMENT", "BC_OPEN", "FRONTMATTER", "RAW_BAD", "NEVER"):
            self.assertNotIn(f"| {never}", leaf)
        for present in (
            "NAME",
            "KEYWORD",
            "RAW_STR_LIT",
            "C_STR_LIT",
            "OUTER_DOC",
            "TILDE",
        ):
            self.assertIn(present, leaf)
        self.assertIn(
            "doc_block = ( OUTER_BC_OPEN | INNER_BC_OPEN ) { BC_TEXT } BC_CLOSE ;", text
        )

    def test_identical_text_only_where_editions_agree(self) -> None:
        a = self.files["rust1.97.0-e2015.tokens"].decode()
        b = self.files["rust1.97.0-e2018.tokens"].decode()
        self.assertNotEqual(a, b)  # keywords differ
        # ... but only in the header, the banner's edition line and the keywords.
        diff = set(a.split("\n")) ^ set(b.split("\n"))
        self.assertLess(len(diff), 12)

    def test_check_passes_on_the_committed_tree(self) -> None:
        self.assertEqual(rlp.check(RUST), [])

    def copy_tree(self) -> Path:
        tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, tmp, ignore_errors=True)
        directory = tmp / "rust"
        shutil.copytree(RUST, directory, ignore=shutil.ignore_patterns("corpus"))
        return directory

    def test_check_catches_a_hand_edit_and_write_repairs_it(self) -> None:
        directory = self.copy_tree()
        target = directory / "rust1.97.0-e2018.tokens"
        target.write_bytes(target.read_bytes().replace(b"  async\n", b"  asynx\n"))
        errors = rlp.check(directory)
        self.assertEqual(len(errors), 1)
        self.assertIn("rust1.97.0-e2018.tokens differs", errors[0])
        problems, wrote = rlp.write(directory)
        self.assertEqual((problems, len(wrote)), ([], 8))
        self.assertEqual(rlp.check(directory), [])

    def test_check_catches_a_missing_pair_and_missing_evidence(self) -> None:
        directory = self.copy_tree()
        (directory / "rust1.97.0-e2021.grammar").unlink()
        self.assertTrue(
            any("rust1.97.0-e2021.grammar" in e for e in rlp.check(directory))
        )
        evidence = directory / "pair-evidence.csv"
        kept = [
            ln
            for ln in evidence.read_text().split("\n")
            if not ln.startswith("1.97.0,2024,")
        ]
        evidence.write_text("\n".join(kept))
        self.assertTrue(any("2024" in e for e in rlp.check(directory)))

    def test_check_catches_changed_xid_data(self) -> None:
        directory = self.copy_tree()
        data = directory / rlp.XID_FILE
        data.write_text(data.read_text().replace("S 0041 005A", "S 0041 0059", 1))
        errors = rlp.check(directory)
        self.assertTrue(errors and all("differs" in e for e in errors))

    def test_check_refuses_broken_xid_data(self) -> None:
        directory = self.copy_tree()
        (directory / rlp.XID_FILE).write_text("S 0041 005A\n")
        self.assertTrue(any("both an S and a C" in e for e in rlp.check(directory)))

    def test_cli(self) -> None:
        directory = self.copy_tree()
        (directory / "rust1.97.0-e2015.tokens").write_text("# x\n\n")
        with (
            contextlib.redirect_stdout(io.StringIO()),
            contextlib.redirect_stderr(io.StringIO()),
        ):
            self.assertEqual(rlp.main(["check", str(RUST)]), 0)
            self.assertEqual(rlp.main(["check", str(directory)]), 1)
            self.assertEqual(rlp.main(["write", str(directory)]), 0)
            self.assertEqual(rlp.main(["check", str(directory)]), 0)
        self.assertEqual(pairs.check_directory(directory), [])


if __name__ == "__main__":
    unittest.main()
