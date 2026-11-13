#!/usr/bin/env python3
"""Generate the RS01 modern lexical pairs, `rust1.97.0-e{2015,2018,2021,2024}`.

Slice A4 of `code/specs/RS01-rust-grammars.md` writes the first four pairs of
the Rust grammar family. Each pair is a `.tokens` file and a `.grammar` file
for ONE edition of release 1.97.0, and the grammar is deliberately small: it
accepts token trees and delimiter balance, nothing else (the `rs-token-trees`
subset). The point of the slice is the LEXER, which has to be complete.

Why a generator and not four hand-written files
===============================================

Two parts of Rust's lexical grammar are too large or too fiddly to write by
hand, and the four editions are 95% the same text:

1. Raw strings. `r#"..."#` may use any number of `#`, 0 to 255 in `rustc`, and
   a regex cannot count. The engine's regexes have no backreferences, so each
   hash count needs its OWN arm, 256 arms per prefix (`r`, `br`, `cr`).
2. Identifiers. Rust identifiers are `XID_Start` and `XID_Continue` characters,
   and the set changes with every Unicode release. The engine's `\\p{XID_*}`
   follows whatever the `regex-syntax` crate was built with, so a grammar that
   used it would change meaning when a dependency is bumped. RS01 decision Q12
   says instead: explicit code point classes, per Unicode version, written
   into the pair. The classes are about 23 KB of text.

Both are written here, once, from data, and the three editions' differences
are a handful of flags (`Edition`). `check` regenerates everything and fails
on any difference, so a hand edit to a pair file is caught the same way a
stale generated file is.

Where the Unicode data comes from
=================================

`rustc` 1.97.0 asks the `unicode-ident` crate (its `Cargo.lock` pins 1.0.22)
whether a character is `XID_Start` or `XID_Continue`; the Unicode version is
17.0.0. This repository already pins a Unicode 17.0.0 build of the same crate:
`unicode-ident 1.0.24` is in several `Cargo.lock` files. The two crate
versions carry identical sets (compared range by range, see RS01 section 10,
"A4 decisions"). So the data is that crate's table, decoded to code point
ranges in `code/grammars/rust/unicode/xid-17.0.0.ranges`. No file is
downloaded. `import-xid` regenerates that file from the crate's `tables.rs`
and refuses a `tables.rs` whose SHA-256 is not the pinned one.

What a pair file looks like
===========================

    <generated header>       from `pair-evidence.csv`, as rust_release_pairs.py
                             writes it for copies; the pair is `reviewed`
    <blank line>
    <body>                   comments, then `skip:`, definitions, `keywords:`,
                             groups and `transitions:`, in the order the engine
                             needs them (first match wins)

Usage
=====

    rust_lexical_pairs.py write         write the four pairs
    rust_lexical_pairs.py check         fail if any file differs from `write`
    rust_lexical_pairs.py import-xid TABLES_RS
                                        regenerate the XID ranges data file

Everything is deterministic: no clock, no environment, no iteration over an
unordered container that reaches the output.
"""

from __future__ import annotations

import argparse
import hashlib
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

import rust_release_pairs as pairs

REPO = Path(__file__).resolve().parents[2]
RUST = REPO / "code" / "grammars" / "rust"

RELEASE = "1.97.0"
EDITIONS = ("2015", "2018", "2021", "2024")
# `rustc` accepts 0 to 255 hashes around a raw string (`u8`); more is an error.
MAX_HASHES = 255

# ---------------------------------------------------------------------------
# The pinned Unicode data
# ---------------------------------------------------------------------------

XID_FILE = "unicode/xid-17.0.0.ranges"
UNICODE_VERSION = "17.0.0"
CRATE = "unicode-ident"
CRATE_VERSION = "1.0.24"
# The crate checksum Cargo.lock records for unicode-ident 1.0.24.
CRATE_CHECKSUM = "e6e4313cd5fcd3dad5cafa179702e2b244f760991f45397d14d4ebf38247da75"
# SHA-256 of that crate's src/tables.rs, the file `import-xid` decodes.
TABLES_SHA256 = "369c476ba91a39fce55563b692a3f598daf7f026cfe0a3796cfed1102fb3b190"
LAST_CODE_POINT = 0x10FFFF
SURROGATES = range(0xD800, 0xE000)

Range = tuple[int, int]  # inclusive code point range


def decode_unicode_ident(tables_rs: str) -> tuple[list[Range], list[Range]]:
    """Decode `unicode-ident`'s `tables.rs` into XID_Start and XID_Continue.

    The crate stores each property as a two-level bit trie so that a lookup is
    straight-line code (see its `lib.rs`). For a non-ASCII code point `cp`:

        chunk  = TRIE[cp // 8 // CHUNK]            (0 past the end of TRIE)
        offset = chunk * CHUNK // 2 + cp // 8 % CHUNK
        bit    = (LEAF[offset] >> (cp % 8)) & 1

    ASCII is a 128-bit mask. This function runs that lookup for every code
    point and merges the hits into maximal ranges.
    """

    def number(name: str) -> int:
        found = re.search(
            rf"\b{name}\s*:\s*\w+\s*=\s*(0x[0-9a-fA-F]+|[0-9]+)\s*;", tables_rs
        )
        if found is None:
            raise ValueError(f"tables.rs has no constant {name}")
        return int(found.group(1), 0)

    def array(name: str) -> list[int]:
        found = re.search(
            rf"static\s+{name}\s*:\s*Align\d+<\[u8;\s*(\d+)\]>\s*=\s*Align\d+\(\[(.*?)\]\)\s*;",
            tables_rs,
            re.DOTALL,
        )
        if found is None:
            raise ValueError(f"tables.rs has no table {name}")
        values = [
            int(item, 16) for item in re.findall(r"0x([0-9a-fA-F]{2})", found.group(2))
        ]
        if len(values) != int(found.group(1)):
            raise ValueError(
                f"table {name} declares {found.group(1)} bytes, has {len(values)}"
            )
        return values

    chunk_size = number("CHUNK")
    leaf = array("LEAF")
    result = []
    for ascii_name, trie_name in (
        ("ASCII_START", "TRIE_START"),
        ("ASCII_CONTINUE", "TRIE_CONTINUE"),
    ):
        mask, trie = number(ascii_name), array(trie_name)
        hits: list[int] = []
        for cp in range(LAST_CODE_POINT + 1):
            if cp in SURROGATES:
                continue  # not a `char`; the crate never answers for them
            if cp < 0x80:
                bit = (mask >> cp) & 1
            else:
                index = cp // 8 // chunk_size
                chunk = trie[index] if index < len(trie) else 0
                offset = chunk * chunk_size // 2 + cp // 8 % chunk_size
                bit = (leaf[offset] >> (cp % 8)) & 1 if offset < len(leaf) else 0
            if bit:
                hits.append(cp)
        result.append(merge(hits))
    return result[0], result[1]


def merge(code_points: list[int]) -> list[Range]:
    """Sorted code points to maximal inclusive ranges."""
    ranges: list[Range] = []
    for cp in code_points:
        if ranges and ranges[-1][1] + 1 == cp:
            ranges[-1] = (ranges[-1][0], cp)
        else:
            ranges.append((cp, cp))
    return ranges


def render_xid_data(start: list[Range], cont: list[Range]) -> str:
    """The committed data file: provenance comments, then `S`/`C` range lines."""
    lines = [
        f"# XID_Start (S) and XID_Continue (C) code point ranges, Unicode {UNICODE_VERSION}.",
        "# Generated by code/scripts/rust_lexical_pairs.py import-xid; do not edit.",
        f"# Source: crate {CRATE} {CRATE_VERSION}, src/tables.rs, SHA-256 {TABLES_SHA256}.",
        f"# The crate's Cargo.lock checksum is {CRATE_CHECKSUM}; that version is pinned in",
        "# several Cargo.lock files of this repository, so no data is downloaded.",
        "# rustc 1.97.0 pins unicode-ident 1.0.22 (its Cargo.lock): the same Unicode",
        "# version and, checked range by range on 2026-11-13, the same two sets.",
        "# Format: `<S|C> <first> <last>`, hexadecimal, inclusive, sorted, disjoint.",
    ]
    lines += [f"S {a:04X} {b:04X}" for a, b in start]
    lines += [f"C {a:04X} {b:04X}" for a, b in cont]
    return "\n".join(lines) + "\n"


def parse_xid_data(text: str) -> tuple[list[Range], list[Range]]:
    """Read and validate the data file (it is checked-in, so it is untrusted)."""
    tables: dict[str, list[Range]] = {"S": [], "C": []}
    for number, raw in enumerate(text.split("\n"), start=1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split()
        if len(parts) != 3 or parts[0] not in tables:
            raise ValueError(f"xid data line {number}: expected `S|C first last`")
        try:
            first, last = int(parts[1], 16), int(parts[2], 16)
        except ValueError:
            raise ValueError(f"xid data line {number}: not hexadecimal") from None
        table = tables[parts[0]]
        if not 0 <= first <= last <= LAST_CODE_POINT:
            raise ValueError(
                f"xid data line {number}: range is out of order or out of range"
            )
        if first <= SURROGATES.stop - 1 and last >= SURROGATES.start:
            raise ValueError(f"xid data line {number}: range touches the surrogates")
        if table and first <= table[-1][1] + 1:
            raise ValueError(
                f"xid data line {number}: ranges must be sorted, disjoint and merged"
            )
        table.append((first, last))
    if not tables["S"] or not tables["C"]:
        raise ValueError("xid data needs both an S and a C table")
    return tables["S"], tables["C"]


def load_xid(directory: Path = RUST) -> tuple[list[Range], list[Range]]:
    data, problem = pairs.read_plain(directory / XID_FILE, 1024 * 1024)
    if data is None:
        raise ValueError(f"{XID_FILE} {problem}")
    return parse_xid_data(data.decode("utf-8"))


# ---------------------------------------------------------------------------
# Regex building blocks
# ---------------------------------------------------------------------------


def escape(cp: int) -> str:
    """One code point, as it may appear inside a regex character class.

    Letters, digits and `_` stay literal so ASCII reads as ASCII. Everything
    else is a numeric escape that BOTH the Rust `regex` crate and Python's `re`
    accept (`\\xHH`, `\\uHHHH`, `\\UHHHHHHHH`), so the generator's own tests can
    compile every pattern.
    """
    char = chr(cp)
    if char.isascii() and (char.isalnum() or char == "_"):
        return char
    if cp < 0x100:
        return f"\\x{cp:02X}"
    if cp < 0x10000:
        return f"\\u{cp:04X}"
    return f"\\U{cp:08X}"


def class_body(ranges: list[Range]) -> str:
    """The inside of a `[...]` class for sorted, disjoint inclusive ranges."""
    parts = []
    for first, last in ranges:
        if first == last:
            parts.append(escape(first))
        elif last == first + 1:
            parts.append(escape(first) + escape(last))
        else:
            parts.append(f"{escape(first)}-{escape(last)}")
    return "".join(parts)


def union_code_points(ranges: list[Range], extra: set[int]) -> list[Range]:
    """`ranges` plus single code points, merged into maximal ranges."""
    ordered = sorted([*ranges, *((cp, cp) for cp in extra)])
    merged: list[Range] = []
    for first, last in ordered:
        if merged and first <= merged[-1][1] + 1:
            merged[-1] = (merged[-1][0], max(merged[-1][1], last))
        else:
            merged.append((first, last))
    return merged


@dataclass(frozen=True)
class Classes:
    """The identifier classes of one Unicode version, as regex text.

    `is_id_start(c)` in `rustc_lexer` is `c == '_' || XID_Start(c)` and
    `is_id_continue(c)` is `XID_Continue(c)`. A literal suffix is an identifier
    (`eat_literal_suffix` is `eat_identifier`).
    """

    start: str  # [_XID_Start]
    cont: str  # [XID_Continue]

    @classmethod
    def build(cls, start: list[Range], cont: list[Range]) -> Classes:
        id_start = union_code_points(start, {ord("_")})
        return cls(start=f"[{class_body(id_start)}]", cont=f"[{class_body(cont)}]")

    @property
    def ident(self) -> str:
        return f"{self.start}{self.cont}*"

    @property
    def suffix(self) -> str:
        """An optional literal suffix (any identifier)."""
        return f"(?:{self.ident})?"


# ---------------------------------------------------------------------------
# Raw strings: one arm per hash count
# ---------------------------------------------------------------------------

# What may follow `e` in an exponent that has NO digit, ending at the first
# character that proves it: after a sign, underscores and then a non-digit; with
# no sign, one or more underscores and then a non-digit, or at once a character
# that is not a digit, an underscore or a sign. (The sign is part of the
# exponent when present: `1e-3` is fine, so `-` must not count as "the
# non-digit" there.)
EXPONENT_WITHOUT_DIGITS = "(?:[+-]_*(?:[^0-9_]|\\z)|_+(?:[^0-9_]|\\z)|[^0-9_+\\-]|\\z)"

# A raw-string body character: anything but `"`, and a carriage return only as
# part of CRLF (`rustc` turns CRLF into LF and rejects a bare CR in a literal).
RAW_CHAR = '(?:[^"\\r]|\\r\\n)'
# The same, but also not `#`: the character after a quote that is not a fence.
RAW_FIRST = '(?:[^"#\\r]|\\r\\n)'


def raw_string_arm(hashes: int) -> str:
    """The regex for a raw string with exactly `hashes` hashes, after its prefix.

    The engine's regexes cannot look ahead or back-reference, so "a `"` that is
    not followed by `hashes` hashes" must be written out. Split the body at its
    quotes: it is

        CHARS  ( "  #{0,n-1}  [ non-quote-non-hash  CHARS ] )*

    Every quote inside the body is followed by FEWER than `n` hashes and then
    something that is not `#` (a character, another quote, or the real fence).
    The closing fence is `"` and exactly `n` hashes, and `rustc` stops at the
    first one, so `r#"a"##` is the raw string `r#"a"#` followed by a `#`.

        r##"a"#b"##     n = 2:  CHARS `a`, group `"#` + `b`, fence `"##`

    The group cannot absorb a whole fence (it stops one hash short and the next
    character must not be `#`), so the regex can never run past the first fence.
    """
    if hashes == 0:
        return f'"{RAW_CHAR}*"'
    fence = "#" * hashes if hashes < 3 else f"#{{{hashes}}}"
    limit = {1: "", 2: "#?"}.get(hashes, f"#{{0,{hashes - 1}}}")
    group = f'"{limit}(?:{RAW_FIRST}{RAW_CHAR}*)?'
    return f'{fence}"{RAW_CHAR}*(?:{group})*"{fence}'


def raw_string_pattern(
    prefix: str, classes: Classes, max_hashes: int = MAX_HASHES
) -> str:
    """`PREFIX (arm_0 | arm_1 | ... | arm_max) SUFFIX?` as one regex."""
    arms = "|".join(raw_string_arm(n) for n in range(max_hashes + 1))
    return f"{prefix}(?:{arms}){classes.suffix}"


# ---------------------------------------------------------------------------
# What differs between editions
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class Edition:
    """The lexical switches of one edition, at release 1.97.0 (RS01 section 2.1).

    Everything else about the four lexers is the same text.
    """

    year: str
    strict_2018: bool  # `async`, `await`, `dyn` strict; `try` reserved (2018+)
    c_strings: bool  # `c"..."` and `cr#"..."#` are literals (2021+)
    prefixes_reserved: bool  # `ident#`, `ident"`, `ident'` are errors (2021+)
    raw_lifetimes: bool  # `'r#a` is one token (2021+)
    guarded_strings: bool  # `#"` and `##` are errors (2024)
    gen_reserved: bool  # `gen` is a keyword (2024)


def edition(year: str) -> Edition:
    number = int(year)
    return Edition(
        year=year,
        strict_2018=number >= 2018,
        c_strings=number >= 2021,
        prefixes_reserved=number >= 2021,
        raw_lifetimes=number >= 2021,
        guarded_strings=number >= 2024,
        gen_reserved=number >= 2024,
    )


# The keywords `rustc_span::symbol` marks as used or reserved in every edition
# (`is_used_keyword_always`, `is_unused_keyword_always`; `$crate`, `{{root}}`
# and `_` are not lexical keywords). RS01 section 4.2, case 12: reserved words
# are KEYWORD tokens too, never a lexer error, because a token tree may
# contain any of them (`quote! { abstract }`).
USED_KEYWORDS = [
    "as",
    "break",
    "const",
    "continue",
    "crate",
    "else",
    "enum",
    "extern",
    "false",
    "fn",
    "for",
    "if",
    "impl",
    "in",
    "let",
    "loop",
    "match",
    "mod",
    "move",
    "mut",
    "pub",
    "ref",
    "return",
    "self",
    "Self",
    "static",
    "struct",
    "super",
    "trait",
    "true",
    "type",
    "unsafe",
    "use",
    "where",
    "while",
]
RESERVED_KEYWORDS = [
    "abstract",
    "become",
    "box",
    "do",
    "final",
    "macro",
    "override",
    "priv",
    "typeof",
    "unsized",
    "virtual",
    "yield",
]
# Weak keywords that stable Rust gives a meaning (`soft_keywords:`). The other
# weak keywords in `symbol.rs` (`builtin`, `catch`, `reuse`, `yeet`,
# `contract_*`) belong to unstable features and are not in a stable pair.
SOFT_KEYWORDS = ["auto", "default", "macro_rules", "raw", "safe", "union"]

# Identifiers that cannot be written raw: `r#crate` is an error.
NOT_RAW = ["crate", "self", "super", "Self", "_"]

PUNCTUATION = [
    ("SEMI", ";"),
    ("COMMA", ","),
    ("DOT", "."),
    ("LPAREN", "("),
    ("RPAREN", ")"),
    ("LBRACE", "{"),
    ("RBRACE", "}"),
    ("LBRACKET", "["),
    ("RBRACKET", "]"),
    ("AT", "@"),
    ("POUND", "#"),
    ("TILDE", "~"),
    ("QUESTION", "?"),
    ("COLON", ":"),
    ("DOLLAR", "$"),
    ("EQ", "="),
    ("BANG", "!"),
    ("LT", "<"),
    ("GT", ">"),
    ("MINUS", "-"),
    ("AMP", "&"),
    ("PIPE", "|"),
    ("PLUS", "+"),
    ("STAR", "*"),
    ("CARET", "^"),
    ("PERCENT", "%"),
    ("SLASH", "/"),
]
DELIMITERS = ["LPAREN", "RPAREN", "LBRACE", "RBRACE", "LBRACKET", "RBRACKET"]


# ---------------------------------------------------------------------------
# The lexicon: an in-memory `.tokens` file
# ---------------------------------------------------------------------------


def slash_escaped(pattern: str) -> str:
    """Escape every `/` in a regex: `.tokens` ends the pattern at the first bare one.

    A backslash escapes the next character, so `\\/` and `\\\\` are copied as they
    are and only a bare `/` gets its backslash. `\\/` means the same to the regex.
    """
    out: list[str] = []
    chars = iter(pattern)
    for char in chars:
        if char == "\\":
            out.append(char + next(chars, ""))
        elif char == "/":
            out.append("\\/")
        else:
            out.append(char)
    return "".join(out)


@dataclass
class Def:
    """One `NAME = /regex/ -> ALIAS @guard` line, with the comment above it.

    `refusal` marks a token that exists only so a transition can refuse it: the
    engine's `errors:` section is parsed but never consulted by `GrammarLexer`
    (RS01 section 4.3, A4 findings), so a malformed construct is made a token
    and `on TOKEN -> refuse` stops the lexer there. A refusal token never
    reaches the parser, so the `.grammar` does not mention it.
    """

    name: str
    pattern: str
    regex: bool = True
    alias: str | None = None
    guard: str | None = None
    notes: tuple[str, ...] = ()
    refusal: bool = False
    inert: bool = False  # matches nothing; exists only to fill a group

    @property
    def effective(self) -> str:
        return self.alias or self.name

    def render(self) -> list[str]:
        body = f"/{slash_escaped(self.pattern)}/" if self.regex else f'"{self.pattern}"'
        line = f"{self.name} = {body}"
        if self.alias:
            line += f" -> {self.alias}"
        if self.guard:
            line += f" @{self.guard}"
        return [f"# {note}" if note else "#" for note in self.notes] + [line]


@dataclass
class Group:
    name: str
    defs: list[Def]
    notes: tuple[str, ...] = ()


@dataclass
class Lexicon:
    skip: list[Def] = field(default_factory=list)
    defs: list[Def] = field(default_factory=list)
    keywords: list[str] = field(default_factory=list)
    soft_keywords: list[str] = field(default_factory=list)
    groups: list[Group] = field(default_factory=list)
    transitions: list[str] = field(default_factory=list)

    def effective_names(self) -> list[str]:
        """Every token type the lexer can emit, once, in definition order."""
        names: list[str] = []
        everything = [*self.defs, *(d for g in self.groups for d in g.defs)]
        for definition in everything:
            if definition.effective not in names:
                names.append(definition.effective)
        if self.keywords:
            names.append("KEYWORD")  # produced by promotion, never defined
        return names

    def refusal_names(self) -> list[str]:
        everything = [*self.defs, *(d for g in self.groups for d in g.defs)]
        names = [d.effective for d in everything if d.refusal]
        return list(dict.fromkeys(names))

    def inert_names(self) -> list[str]:
        everything = [*self.defs, *(d for g in self.groups for d in g.defs)]
        return [d.effective for d in everything if d.inert]


def comment(lines: list[str] | tuple[str, ...], indent: str = "") -> list[str]:
    return [f"{indent}# {line}".rstrip() for line in lines]


# ---------------------------------------------------------------------------
# The Rust lexical grammar, 1.97.0
# ---------------------------------------------------------------------------


def string_body(quote: str = '"') -> str:
    """A quoted literal body, after the opening quote, with its closing quote.

    `rustc_lexer::double_quoted_string` only treats `\\\\` and `\\"` specially;
    the pair is skipped as a unit so `"\\""` ends at the third quote, and a
    backslash before CRLF continues the line. A bare CR is not allowed.
    """
    return f"(?:[^{quote}\\\\\\r]|\\r\\n|\\\\(?:\\r\\n|[^\\r]))*{quote}"


def build_lexicon(ed: Edition, classes: Classes) -> Lexicon:
    """The complete token grammar of one edition. Order is priority."""
    lex = Lexicon()
    ident, suffix = classes.ident, classes.suffix
    decimal = "[0-9][0-9_]*"
    exponent = "[eE][+-]?[0-9_]*[0-9][0-9_]*"
    bad_exponent = f"{decimal}(?:\\.{decimal})?[eE]{EXPONENT_WITHOUT_DIGITS}"
    bad_based = (
        "0[bo][0-9_]*[eE]|0[box][0-9a-fA-F_]*\\.(?:\\z|[^.A-Za-z_\\x80-\\U0010FFFF])"
    )

    # -- skipped input ------------------------------------------------------
    lex.skip = [
        Def(
            "WHITESPACE",
            "[\\x09-\\x0D\\x20\\x85\\u200E\\u200F\\u2028\\u2029]+",
            notes=(
                "Pattern_White_Space (`rustc_lexer::is_whitespace`): a stable set,",
                "so it is written out. `\\r` is whitespace; a CR inside a literal is",
                "refused by the literal's own regex, not here.",
            ),
        ),
        Def(
            "BOM",
            "\\uFEFF",
            guard="input_start",
            notes=("A byte order mark at the very start of the input is dropped.",),
        ),
    ]

    d = lex.defs

    # -- comments (section 4.3 of RS01) ---------------------------------------
    d.append(
        Def(
            "OUTER_DOC",
            "///(?:[^/\\n][^\\n]*)?(?m:$)",
            notes=(
                "COMMENTS. Line comments are TOKENS, not `skip:` entries. A skip",
                "pattern is global, so it would also fire inside a block comment,",
                "where `/* // */` must still close at `*/`. The post-tokenize hook",
                "`rust_block_comments` (slice A5) deletes the plain ones.",
                "",
                "`///x` is an outer doc comment; `////` and longer are plain.",
                "`(?m:$)` asserts a line end without consuming it.",
            ),
        )
    )
    d.append(Def("INNER_DOC", "//![^\\n]*", notes=("`//!x` is an inner doc comment.",)))
    d.append(Def("LINE_COMMENT", "//[^\\n]*", notes=("Any other `//` comment.",)))
    d.append(
        Def(
            "INNER_BC_OPEN",
            "/*!",
            regex=False,
            notes=(
                "Block comment openers. They are tried before SLASH and STAR. Each",
                "one pushes a group (see `transitions:`) in which the comment's",
                "text is lexed, so the lexer itself tracks the nesting depth.",
                "`/*!` opens an inner doc comment, even `/*!*/`.",
            ),
        )
    )
    d.append(
        Def(
            "OUTER_BC_OPEN",
            "\\/\\*\\*[^*\\/]",
            notes=(
                "`/**x` opens an outer doc comment. It consumes one content",
                "character (the engine has no lookahead), so `/**/` and `/***`",
                "stay plain comments, as in `rustc_lexer`.",
            ),
        )
    )
    d.append(Def("BC_OPEN", "/*", regex=False, notes=("A plain block comment.",)))

    # -- unstable syntax that is refused --------------------------------------
    d.append(
        Def(
            "FRONTMATTER",
            "---[^\\n]*",
            guard="input_start",
            refusal=True,
            notes=(
                "FRONTMATTER (`---` at the start of the file) is unstable at 1.97",
                "(RS01 section 4.2), so a stable pair refuses it.",
            ),
        )
    )

    # -- raw strings ----------------------------------------------------------
    d.append(
        Def(
            "RAW_STR_LIT",
            raw_string_pattern("r", classes),
            notes=(
                "RAW STRINGS, one regex arm per hash count, 0 to 255. Generated by",
                "code/scripts/rust_lexical_pairs.py; see `raw_string_arm` there for",
                "why the arms look the way they do. A literal suffix may follow.",
                "`r#ident` is not a raw string and falls through to RAW_IDENT.",
            ),
        )
    )
    d.append(Def("RAW_BYTE_STR_LIT", raw_string_pattern("br", classes)))
    if ed.c_strings:
        d.append(
            Def(
                "RAW_C_STR_LIT",
                raw_string_pattern("cr", classes),
                notes=(
                    '`cr#"..."#` raw C strings exist from edition 2021 (and 1.77).',
                ),
            )
        )
    d.append(
        Def(
            "RAW_IDENT",
            f"r#{ident}",
            notes=(
                "RAW IDENTIFIERS `r#match`. `keywords:` only promotes NAME, so a raw",
                "identifier stays an identifier. The five that cannot be raw are",
                "refused by `transitions:`.",
            ),
        )
    )
    raw_prefix = "(?:b|c)?" if ed.c_strings else "b?"
    d.append(
        Def(
            "RAW_BAD",
            f'{raw_prefix}r[#"]',
            refusal=True,
            notes=(
                "A raw string that did not match above: unterminated, more than 255",
                "hashes, or `r#` followed by something that is not an identifier.",
                "`rustc` rejects all of them in every edition.",
            ),
        )
    )

    # -- strings and characters -----------------------------------------------
    str_body = string_body()
    d.append(
        Def(
            "BYTE_STR_LIT",
            f'b"{str_body}{suffix}',
            notes=(
                "STRINGS. The literal's suffix is part of the token, as in",
                "`rustc_lexer` (`suffix_start`); the parser decides which suffixes",
                "are legal. Their escapes are decoded by the consumer.",
            ),
        )
    )
    if ed.c_strings:
        d.append(
            Def(
                "C_STR_LIT",
                f'c"{str_body}{suffix}',
                notes=(
                    'C strings `c"..."` are literals from edition 2021 (and 1.77).',
                ),
            )
        )
    d.append(Def("STR_LIT", f'"{str_body}{suffix}'))
    quote_prefix = "(?:b|c)?" if ed.c_strings else "b?"
    d.append(
        Def(
            "STR_BAD",
            f'{quote_prefix}"',
            refusal=True,
            notes=(
                "A string that did not close, or that holds a bare CR: `rustc`",
                "rejects it in every edition.",
            ),
        )
    )
    char_one = "'[^\\\\\\r]'"
    char_other = "'(?:[^'\\\\/\\n\\r]|\\\\(?:\\r\\n|[^\\r]))*\\n?'"
    d.append(
        Def(
            "BYTE_LIT",
            f"b(?:{char_one}|{char_other}){suffix}",
            notes=("BYTE LITERALS `b'x'`, `b'\\n'`.",),
        )
    )
    d.append(
        Def(
            "BYTE_BAD",
            "b'",
            regex=False,
            refusal=True,
            notes=("An unterminated `b'...`: an error in every edition.",),
        )
    )

    # -- lifetimes and characters ---------------------------------------------
    d.append(
        Def(
            "CHAR_ONE",
            f"{char_one}{suffix}",
            alias="CHAR_LIT",
            notes=(
                "CHARACTER OR LIFETIME? `rustc_lexer::lifetime_or_char` looks at the",
                "second character: `'a'` is a character even though `a` could start",
                "a lifetime. So a quote, ONE character that is not a backslash, and",
                "a quote is tried first. `'''` and `'\\''` fall out the same way.",
            ),
        )
    )
    if ed.raw_lifetimes:
        d.append(
            Def(
                "RAW_LIFETIME_BAD",
                f"'r#{ident}'",
                refusal=True,
                notes=("`'r#a'` is an error: a raw lifetime followed by a quote.",),
            )
        )
        d.append(
            Def(
                "RAW_LIFETIME",
                f"'r#{ident}",
                notes=(
                    "A raw lifetime `'r#a` (edition 2021 and later, from 1.83). In",
                    "earlier editions it is the lifetime `'r` followed by `#` and `a`.",
                ),
            )
        )
    d.append(
        Def(
            "CHAR_IDENT_LIKE",
            f"'(?:{classes.start}|[0-9]){classes.cont}*'{suffix}",
            alias="CHAR_LIT",
            notes=(
                "`'ab'` is a (bad) character literal, not a lifetime followed by",
                "a quote; the decoder rejects it, as `rustc` does.",
            ),
        )
    )
    if ed.prefixes_reserved:
        d.append(
            Def(
                "LIFETIME_PREFIX",
                f"'{ident}#",
                refusal=True,
                notes=(
                    "`'a#` is a reserved prefix on a lifetime (edition 2021 and later).",
                ),
            )
        )
    d.append(
        Def(
            "LIFETIME",
            f"'(?:{classes.start}|[0-9]){classes.cont}*",
            notes=(
                "A LIFETIME. One that starts with a digit is lexed too, and is an",
                "error later, as in `rustc`.",
            ),
        )
    )
    d.append(
        Def(
            "CHAR_ESCAPED",
            f"{char_other}{suffix}",
            alias="CHAR_LIT",
            notes=(
                "Every other character literal: an escape, or a quote and several",
                "characters. A `/` ends the scan, as in `rustc_lexer`, and a quote",
                "that never closes is left to the engine's `Unexpected sequence`",
                "error. A line break is allowed only right before the closing quote.",
            ),
        )
    )

    # -- identifiers ----------------------------------------------------------
    if ed.prefixes_reserved:
        d.append(
            Def(
                "RESERVED_PREFIX",
                f"{ident}[#\"']",
                refusal=True,
                notes=(
                    'RESERVED PREFIXES (edition 2021 and later): `k#x`, `k"x"`,',
                    "`k'x'` are errors. The raw, byte and C forms matched above, so",
                    "what reaches here has an unknown prefix. In 2015 and 2018 the",
                    "same text is an identifier followed by the next token.",
                ),
            )
        )
    d.append(
        Def(
            "NAME",
            ident,
            notes=(
                "IDENTIFIERS: XID_Start (or `_`) then XID_Continue*, with the",
                "classes written out for Unicode 17.0.0. `keywords:` below",
                "promotes the reserved words.",
            ),
        )
    )

    # -- numbers --------------------------------------------------------------
    d.append(
        Def(
            "BASED_FLOAT_BAD",
            bad_based,
            refusal=True,
            notes=(
                "NUMBERS. A binary, octal or hexadecimal literal followed by an",
                'exponent or a fraction is a "... float literal is not supported"',
                "error in `rustc`. (`0x1.max(2)` is fine: a method name follows.)",
            ),
        )
    )
    d.append(
        Def(
            "FLOAT_BAD_EXP",
            bad_exponent,
            refusal=True,
            notes=(
                "`1e`, `1.5e+`, `1else`: an exponent with no digit is an error. It",
                "is tried before FLOAT_LIT and INT_LIT so that a suffix starting with",
                "`e` cannot hide it: what reaches them is never `digits` then `e`",
                "without a valid exponent.",
            ),
        )
    )
    d.append(
        Def(
            "FLOAT_LIT",
            f"{decimal}(?:\\.{decimal}(?:{exponent})?|{exponent}){suffix}",
            notes=(
                "A decimal float: a fraction, an exponent, or both. `1.` with no",
                "digit after the dot is NOT a float here (no lookahead): it is INT",
                "then DOT, so `1.foo()` and `1..2` lex correctly and the grammar",
                "composes `INT_LIT @no_space_before DOT`.",
            ),
        )
    )
    d.append(
        Def(
            "INT_LIT",
            f"(?:0x[0-9a-fA-F_]*|0[bo][0-9_]*|{decimal}){suffix}",
            notes=(
                "An integer in any base, with an optional suffix (`1u8`,",
                "`0x1F_u8`). `0b102` is one token whose digits the decoder checks,",
                "as in `rustc_lexer`.",
            ),
        )
    )

    # -- punctuation ----------------------------------------------------------
    if ed.guarded_strings:
        d.append(
            Def(
                "GUARDED_PREFIX",
                '##|#"',
                refusal=True,
                notes=(
                    'GUARDED STRINGS (edition 2024): `#"x"#` and `##` are reserved',
                    "and an error. In earlier editions they are ordinary POUND tokens.",
                ),
            )
        )
    first = True
    for name, text in PUNCTUATION:
        notes: tuple[str, ...] = ()
        if first:
            notes = (
                "PUNCTUATION. One character is one token: the grammar composes",
                "`>>`, `->`, `::` and the rest from adjacent tokens with F11",
                "adjacency predicates, exactly `proc_macro`'s Joint and Alone.",
            )
            first = False
        d.append(Def(name, text, regex=False, notes=notes))

    # -- keywords -------------------------------------------------------------
    keywords = list(USED_KEYWORDS)
    reserved = list(RESERVED_KEYWORDS)
    if ed.strict_2018:
        keywords += ["async", "await", "dyn"]
        reserved += ["try"]
    if ed.gen_reserved:
        reserved += ["gen"]
    lex.keywords = sorted(keywords) + sorted(reserved)
    lex.soft_keywords = sorted(SOFT_KEYWORDS + ([] if ed.strict_2018 else ["dyn"]))

    # -- groups ---------------------------------------------------------------
    text_pattern = "[^*\\/]+|\\*|\\/"
    lex.groups = [
        Group(
            "bc",
            [
                Def("BC_OPEN_NESTED", "/*", regex=False, alias="BC_OPEN"),
                Def("BC_CLOSE", "*/", regex=False),
                Def("BC_TEXT", text_pattern),
            ],
            notes=(
                "Inside a plain block comment. These patterns are EXCLUSIVE (the",
                "group is a nested region), so `//` here is text, not a comment.",
                "`/*` nests, `*/` closes; `transitions:` push and pop the group.",
            ),
        ),
        Group(
            "doc_bc",
            [
                Def("BC_OPEN_IN_DOC", "/*", regex=False, alias="BC_OPEN"),
                Def("BC_CLOSE_IN_DOC", "*/", regex=False, alias="BC_CLOSE"),
                Def("BC_TEXT_IN_DOC", text_pattern, alias="BC_TEXT"),
            ],
            notes=(
                "Inside a doc block comment (`/** */`, `/*! */`). Same text, but a",
                "nested `/*` here is REFUSED (RS01 divergence RD4): the doc",
                "comment's tokens are kept for the grammar, and a refusal is the",
                "conservative answer until the nested scanner of RS01 X1 exists.",
            ),
        ),
        Group(
            "after_dot",
            [
                Def("TUPLE_BAD_EXP", bad_exponent, alias="FLOAT_BAD_EXP", refusal=True),
                Def(
                    "TUPLE_BAD_BASED", bad_based, alias="BASED_FLOAT_BAD", refusal=True
                ),
                Def(
                    "TUPLE_INDEX",
                    f"{decimal}{suffix}",
                    alias="INT_LIT",
                ),
            ],
            notes=(
                "TUPLE INDEX. Right after a lone `.`, a digit run is an integer",
                "that ends at the next `.`, so `x.0.1` is INT 0, DOT, INT 1 (the",
                "view `rustc`'s parser reaches by splitting the float `0.1`). A flat",
                "mode: it inherits every default pattern and its own come first.",
                "The two refusals repeat the number rules that the integer would",
                "otherwise hide, and the suffix is kept so `x.0u8` is one token as in",
                "`rustc_lexer`.",
            ),
        ),
        Group(
            "after_dots",
            [
                Def(
                    "NEVER",
                    "[^\\s\\S]",
                    inert=True,
                    notes=(
                        "A pattern that matches nothing: grammar-tools rejects an",
                        "empty group, and this mode needs no pattern of its own.",
                    ),
                )
            ],
            notes=(
                "After `..` (or more dots) the default patterns apply, so `0..1.5`",
                'ends in the float `1.5`. This mode exists only to say "not a field',
                'access"; it adds no pattern.',
            ),
        ),
    ]

    # -- transitions ----------------------------------------------------------
    names = lex.effective_names()
    transparent = {"DOT", "LINE_COMMENT", "BC_OPEN", "INNER_BC_OPEN", "OUTER_BC_OPEN"}
    refusals = lex.refusal_names()
    skipped = transparent | {"BC_TEXT", "BC_CLOSE"} | set(refusals)
    resets = [n for n in names if n not in skipped]
    t = lex.transitions
    t.append(f"on ({' | '.join(refusals)}) -> refuse")
    t.append("on (INNER_BC_OPEN | OUTER_BC_OPEN) -> push doc_bc")
    t.append("on BC_OPEN in doc_bc -> refuse")
    t.append("on BC_OPEN -> push bc")
    t.append("on BC_CLOSE -> pop")
    t.extend(f'on RAW_IDENT="r#{word}" -> refuse' for word in NOT_RAW)
    t.append("on DOT in default -> set-mode after_dot")
    t.append("on DOT in after_dot -> set-mode after_dots")
    t.append("on DOT in after_dots -> set-mode after_dots")
    t.append(f"on ({' | '.join(resets)}) in after_dot -> set-mode default")
    t.append(f"on ({' | '.join(resets)}) in after_dots -> set-mode default")
    return lex


# ---------------------------------------------------------------------------
# Rendering the two files of a pair
# ---------------------------------------------------------------------------


def section(title: str, items: list[Def], notes: tuple[str, ...] = ()) -> list[str]:
    """`title:` and its indented definitions (comments indented with them)."""
    lines = [f"{title}:"]
    lines += comment(notes, "  ")
    for item in items:
        lines += [f"  {text}" for text in item.render()]
    return lines


def render_tokens_body(ed: Edition, lex: Lexicon) -> str:
    """Everything after the header of `rust1.97.0-e<year>.tokens`."""
    out: list[str] = []
    out += comment(
        [
            f"Rust {RELEASE}, edition {ed.year}: the complete lexer.",
            "",
            "GENERATED by code/scripts/rust_lexical_pairs.py (slice A4 of RS01). Do not",
            "edit by hand: `rust_lexical_pairs.py check` fails on any difference, and",
            "the raw-string arms and Unicode classes below are too large to review. Fix",
            "the generator and run `write`.",
            "",
            "This is the lexer for `rs-token-trees`: it produces every token `rustc`'s",
            "lexer does, and the paired .grammar accepts token trees and delimiter",
            "balance only. What this file relies on, and what it does not:",
            "",
            "  * Order is priority (first match wins). Literals come before NAME,",
            '    because `r#"x"#`, `b\'x\'` and `c"x"` start like identifiers.',
            "  * `skip:` is global, in every group, so comments are not skipped here:",
            "    they are tokens (see COMMENTS) that the post-tokenize hook",
            "    `rust_block_comments` deletes. That hook and `rust_strip_shebang` (a",
            "    shebang is `#!` not followed by `[`) are registered by slice A5; the",
            "    engine cannot express either today (RS01 section 4.3).",
            "  * The engine's `errors:` section is never consulted by the lexer. A",
            "    malformed construct is a token that `transitions:` refuses instead.",
            "  * Identifiers use explicit XID classes for Unicode "
            + f"{UNICODE_VERSION} (RS01 Q12),",
            "    never `\\p{XID_Start}`.",
            "",
            "Editions: "
            + ", ".join(
                f"{label} {'yes' if flag else 'no'}"
                for label, flag in (
                    ("2018 keywords", ed.strict_2018),
                    ("C strings", ed.c_strings),
                    ("reserved prefixes", ed.prefixes_reserved),
                    ("raw lifetimes", ed.raw_lifetimes),
                    ("guarded strings", ed.guarded_strings),
                    ("`gen`", ed.gen_reserved),
                )
            )
            + ".",
        ]
    )
    out.append("")
    out += section("skip", lex.skip)
    out.append("")
    for definition in lex.defs:
        out += definition.render()
    out.append("")
    out.append("keywords:")
    out += comment(
        [
            "Used and reserved words, one list: every one is valid inside a token tree,",
            "so none is a lexer error (RS01 section 4.2, case 12). Only NAME is promoted;",
            "a RAW_IDENT such as `r#match` is not.",
        ],
        "  ",
    )
    out += [f"  {word}" for word in lex.keywords]
    out.append("")
    out.append("soft_keywords:")
    out += comment(
        [
            "Words that mean something only in some positions. The lexer ignores",
            "them (they stay NAME); the parser decides. `dyn` is one only in 2015.",
        ],
        "  ",
    )
    out += [f"  {word}" for word in lex.soft_keywords]
    for group in lex.groups:
        out.append("")
        out += comment(group.notes)
        out.append(f"group {group.name}:")
        for definition in group.defs:
            out += [f"  {text}" for text in definition.render()]
    out.append("")
    out.append("transitions:")
    out += comment(
        [
            "First matching rule wins. A refused token stops the lexer with an error.",
            "Block comments: an opener pushes the group that lexes the comment's text,",
            "so the stack depth IS the nesting depth; `*/` pops it.",
        ],
        "  ",
    )
    out += [f"  {rule}" for rule in lex.transitions]
    return "\n".join(out) + "\n"


GRAMMAR_NOTES = [
    "Rust {release}, edition {year}: token trees and delimiter balance, nothing else",
    "(the `rs-token-trees` subset of RS01). The first rule is the start symbol.",
    "",
    "GENERATED by code/scripts/rust_lexical_pairs.py; do not edit by hand.",
    "",
    "A token tree is a delimited group of token trees, or any single token that is",
    "not a delimiter. grammar-tools has no wildcard terminal, so `leaf` lists every",
    "token type this edition's lexer can emit, except the six delimiters and the",
    "tokens that only exist to be refused or removed:",
    "",
    "  * LINE_COMMENT and plain block comments are deleted by the post-tokenize",
    "    hook `rust_block_comments` (slice A5). If the hook did not run, they reach",
    "    this grammar and it rejects them, which makes a missing hook visible.",
    "  * A doc block comment keeps its tokens, and `doc_block` accepts that run.",
    "  * Tokens that are refused (every `*_BAD`, `*_PREFIX`, FRONTMATTER) or inert",
    "    (NEVER) never reach the parser.",
    "",
    "`grammar-tools validate` therefore warns that those tokens are never used. That",
    "is intended: listing them in `leaf` would let an unterminated comment (a",
    "BC_OPEN the hook leaves in place) parse, and an unreachable rule is an error.",
]


def render_grammar_body(ed: Edition, lex: Lexicon) -> str:
    never_parsed = ["LINE_COMMENT", "BC_OPEN", *lex.refusal_names(), *lex.inert_names()]
    excluded = set(DELIMITERS) | set(never_parsed)  # cross-validation warns: see notes
    excluded |= {"INNER_BC_OPEN", "OUTER_BC_OPEN", "BC_TEXT", "BC_CLOSE"}
    leaves = [n for n in lex.effective_names() if n not in excluded]
    out = comment(
        [line.format(release=RELEASE, year=ed.year) for line in GRAMMAR_NOTES]
    )
    out += [
        "",
        "file = { tt } ;",
        "",
        "tt = delim_tt | doc_block | leaf ;",
        "",
        "delim_tt = LPAREN { tt } RPAREN",
        "         | LBRACKET { tt } RBRACKET",
        "         | LBRACE { tt } RBRACE ;",
        "",
        "doc_block = ( OUTER_BC_OPEN | INNER_BC_OPEN ) { BC_TEXT } BC_CLOSE ;",
        "",
    ]
    out.append(f"leaf = {leaves[0]}")
    out += [f"     | {name}" for name in leaves[1:]]
    out[-1] += " ;"
    return "\n".join(out) + "\n"


# ---------------------------------------------------------------------------
# Putting the files together
# ---------------------------------------------------------------------------


@dataclass
class Context:
    directory: Path
    inventory: pairs.Inventory
    rows: dict[pairs.PairKey, pairs.Row]
    classes: Classes


def load_context(directory: Path, errors: list[str]) -> Context | None:
    """The inventory, the evidence rows and the identifier classes."""
    problems = pairs.Errors()
    loaded = pairs.load(directory, problems)
    errors.extend(problems)
    if loaded is None:
        return None
    inventory, rows = loaded
    try:
        classes = Classes.build(*load_xid(directory))
    except ValueError as error:
        errors.append(str(error))
        return None
    return Context(directory, inventory, {row.key: row for row in rows}, classes)


def generate(context: Context, errors: list[str]) -> dict[str, bytes]:
    """File name -> exact bytes, for the four pairs. Missing evidence is an error."""
    files: dict[str, bytes] = {}
    for year in EDITIONS:
        row = context.rows.get((RELEASE, year))
        if row is None:
            errors.append(f"pair-evidence.csv has no row for {RELEASE} edition {year}")
            continue
        ed = edition(year)
        lex = build_lexicon(ed, context.classes)
        bodies = {
            ".tokens": render_tokens_body(ed, lex),
            ".grammar": render_grammar_body(ed, lex),
        }
        for suffix, body in bodies.items():
            header = pairs.render_header(row, context.inventory, suffix)
            files[f"{row.stem}{suffix}"] = header + b"\n\n" + body.encode("utf-8")
    return files


def check(directory: Path) -> list[str]:
    errors: list[str] = []
    context = load_context(directory, errors)
    if context is None:
        return errors
    for name, wanted in generate(context, errors).items():
        data, problem = pairs.read_plain(directory / name, pairs.MAX_PAIR_BYTES)
        if data is None:
            errors.append(f"{name} {problem}; run `write`")
        elif data != wanted:
            errors.append(f"{name} differs from what the generator writes; run `write`")
    return errors


def write(directory: Path) -> tuple[list[str], list[str]]:
    errors: list[str] = []
    wrote: list[str] = []
    context = load_context(directory, errors)
    if context is None:
        return errors, wrote
    files = generate(context, errors)
    if errors:
        return errors, wrote
    for name, data in files.items():
        (directory / name).write_bytes(data)
        wrote.append(name)
    return errors, wrote


def import_xid(tables_rs: Path, directory: Path) -> list[str]:
    """Regenerate the XID data file from the pinned crate source."""
    data, problem = pairs.read_plain(tables_rs, 4 * 1024 * 1024)
    if data is None:
        return [f"{tables_rs} {problem}"]
    digest = hashlib.sha256(data).hexdigest()
    if digest != TABLES_SHA256:
        return [
            (
                f"{tables_rs} has SHA-256 {digest}, not the pinned {TABLES_SHA256} "
                f"({CRATE} {CRATE_VERSION} src/tables.rs)"
            )
        ]
    start, cont = decode_unicode_ident(data.decode("utf-8"))
    target = directory / XID_FILE
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(render_xid_data(start, cont), encoding="utf-8")
    print(
        f"wrote: {XID_FILE} ({len(start)} XID_Start and {len(cont)} XID_Continue ranges)"
    )
    return []


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    commands = parser.add_subparsers(dest="command", required=True)
    for name, text in (
        ("check", "fail if a pair file differs from what `write` would write"),
        ("write", "write the four pairs"),
    ):
        sub = commands.add_parser(name, help=text)
        sub.add_argument("directory", nargs="?", type=Path, default=RUST)
    sub = commands.add_parser("import-xid", help="regenerate the XID ranges data file")
    sub.add_argument("tables_rs", type=Path)
    sub.add_argument("directory", nargs="?", type=Path, default=RUST)
    args = parser.parse_args(argv)
    if args.command == "import-xid":
        errors = import_xid(args.tables_rs, args.directory)
    elif args.command == "check":
        errors = check(args.directory)
        if not errors:
            print(f"rust/{RELEASE} lexical pairs: ok ({len(EDITIONS) * 2} files)")
    else:
        errors, wrote = write(args.directory)
        for name in wrote:
            print(f"wrote: {name}")
    for message in errors:
        print(f"error: {pairs.printable(message)}", file=sys.stderr)
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
