#!/usr/bin/env perl

# 01_utf16.t -- UTF-16 offset conversion tests
#
# These are the most important correctness tests in the entire package.
# If convert_utf16_offset_to_byte_offset() is wrong, every feature that
# depends on cursor position will be wrong: hover, go-to-definition,
# references, completion, rename, signature help.

use strict;
use warnings;
use utf8;
use Test::More;

use CodingAdventures::Ls00::DocumentManager qw(convert_utf16_offset_to_byte_offset);

# ── ASCII Tests ──────────────────────────────────────────────────────────────

subtest "ASCII simple" => sub {
    is(
        convert_utf16_offset_to_byte_offset("hello world", 0, 6),
        6,
        "'world' starts at byte 6"
    );
};

subtest "start of file" => sub {
    is(
        convert_utf16_offset_to_byte_offset("abc", 0, 0),
        0,
        "position (0,0) is byte 0"
    );
};

subtest "end of short string" => sub {
    is(
        convert_utf16_offset_to_byte_offset("abc", 0, 3),
        3,
        "position past last char is byte 3"
    );
};

# ── Multiline Tests ──────────────────────────────────────────────────────────

subtest "second line" => sub {
    is(
        convert_utf16_offset_to_byte_offset("hello\nworld", 1, 0),
        6,
        "line 1 starts at byte 6"
    );
};

# ── Emoji Tests (Non-BMP: 4 UTF-8 bytes, 2 UTF-16 code units) ───────────────

subtest "emoji: guitar takes 2 UTF-16 units but 4 UTF-8 bytes" => sub {
    is(
        convert_utf16_offset_to_byte_offset("A\x{1F3B8}B", 0, 3),
        5,
        "'B' after guitar emoji is at byte 5"
    );
};

subtest "emoji at start" => sub {
    is(
        convert_utf16_offset_to_byte_offset("\x{1F3B8}hello", 0, 2),
        4,
        "'h' after emoji is at byte 4"
    );
};

# ── 2-byte UTF-8 (BMP codepoint) ────────────────────────────────────────────

subtest "2-byte UTF-8: e-acute" => sub {
    is(
        convert_utf16_offset_to_byte_offset("caf\x{e9}!", 0, 4),
        5,
        "'!' after cafe is at byte 5"
    );
};

# ── Multiline with Emoji ────────────────────────────────────────────────────

subtest "multiline with emoji" => sub {
    is(
        convert_utf16_offset_to_byte_offset("A\x{1F3B8}B\nhello", 1, 0),
        7,
        "line 1 starts at byte 7 after emoji line"
    );
};

# ── Beyond line end ─────────────────────────────────────────────────────────

subtest "beyond line end clamps to newline" => sub {
    is(
        convert_utf16_offset_to_byte_offset("ab\ncd", 0, 100),
        2,
        "character past line end clamps to newline position"
    );
};

# ── 3-byte UTF-8 (CJK character) ────────────────────────────────────────────

subtest "3-byte UTF-8: CJK character" => sub {
    is(
        convert_utf16_offset_to_byte_offset("A\x{4E2D}B", 0, 2),
        4,
        "'B' after CJK char is at byte 4"
    );
};

# ── Empty string ────────────────────────────────────────────────────────────

subtest "empty string" => sub {
    is(
        convert_utf16_offset_to_byte_offset("", 0, 0),
        0,
        "empty string gives byte 0"
    );
};

# ── Multiple emojis ─────────────────────────────────────────────────────────

subtest "multiple emojis" => sub {
    is(
        convert_utf16_offset_to_byte_offset("\x{1F3B8}\x{1F3B8}X", 0, 4),
        8,
        "'X' after two emojis is at byte 8"
    );
};

done_testing();
