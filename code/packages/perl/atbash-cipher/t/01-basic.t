use strict;
use warnings;
use Test2::V0;

# ============================================================================
# Comprehensive tests for the Atbash cipher implementation.
# ============================================================================
#
# These tests verify that the Atbash cipher correctly reverses the alphabet
# for both uppercase and lowercase letters, preserves non-alphabetic
# characters, and satisfies the self-inverse property.

use CodingAdventures::AtbashCipher qw(encrypt decrypt);

# --- Basic Encryption ---

# H(7)->S(18), E(4)->V(21), L(11)->O(14), L(11)->O(14), O(14)->L(11)
is(encrypt("HELLO"), "SVOOL", "encrypts HELLO to SVOOL");
is(encrypt("hello"), "svool", "encrypts hello to svool (case preservation)");
is(encrypt("Hello, World! 123"), "Svool, Dliow! 123", "mixed case with punctuation");
is(encrypt("ABCDEFGHIJKLMNOPQRSTUVWXYZ"), "ZYXWVUTSRQPONMLKJIHGFEDCBA", "full uppercase alphabet");
is(encrypt("abcdefghijklmnopqrstuvwxyz"), "zyxwvutsrqponmlkjihgfedcba", "full lowercase alphabet");

# --- Case Preservation ---

is(encrypt("ABC"), "ZYX", "uppercase stays uppercase");
is(encrypt("abc"), "zyx", "lowercase stays lowercase");
is(encrypt("AbCdEf"), "ZyXwVu", "mixed case preserved");

# --- Non-Alpha Passthrough ---

is(encrypt("12345"), "12345", "digits unchanged");
is(encrypt("!@#\$%"), "!@#\$%", "punctuation unchanged");
is(encrypt("   "), "   ", "spaces unchanged");
is(encrypt("A1B2C3"), "Z1Y2X3", "mixed alpha and digits");
is(encrypt("A\nB\tC"), "Z\nY\tX", "newlines and tabs");

# --- Self-Inverse Property ---
# The most important mathematical property: encrypt(encrypt(x)) == x

is(encrypt(encrypt("HELLO")), "HELLO", "self-inverse for HELLO");
is(encrypt(encrypt("hello")), "hello", "self-inverse for lowercase");
is(encrypt(encrypt("Hello, World! 123")), "Hello, World! 123", "self-inverse for mixed");
is(encrypt(encrypt("ABCDEFGHIJKLMNOPQRSTUVWXYZ")), "ABCDEFGHIJKLMNOPQRSTUVWXYZ", "self-inverse for full alphabet");
is(encrypt(encrypt("")), "", "self-inverse for empty string");
is(encrypt(encrypt("The quick brown fox jumps over the lazy dog! 42")),
   "The quick brown fox jumps over the lazy dog! 42", "self-inverse for long text");

# --- Edge Cases ---

is(encrypt(""), "", "empty string");
is(encrypt("A"), "Z", "single A");
is(encrypt("Z"), "A", "single Z");
is(encrypt("M"), "N", "single M");
is(encrypt("N"), "M", "single N");
is(encrypt("a"), "z", "single lowercase a");
is(encrypt("z"), "a", "single lowercase z");
is(encrypt("5"), "5", "single digit");
is(encrypt(" "), " ", "single space");

# No letter maps to itself: 25 - p == p only when p == 12.5
for my $i (0..25) {
    my $upper = chr(ord('A') + $i);
    isnt(encrypt($upper), $upper, "$upper does not map to itself");

    my $lower = chr(ord('a') + $i);
    isnt(encrypt($lower), $lower, "$lower does not map to itself");
}

# --- Decrypt ---

is(decrypt("SVOOL"), "HELLO", "decrypts SVOOL to HELLO");
is(decrypt("svool"), "hello", "decrypts svool to hello");

# decrypt(encrypt(text)) == text
for my $text ("HELLO", "hello", "Hello, World! 123", "", "42") {
    is(decrypt(encrypt($text)), $text, "decrypt(encrypt('$text')) round-trips");
}

# encrypt and decrypt produce identical output
for my $text ("HELLO", "svool", "Test!", "") {
    is(encrypt($text), decrypt($text), "encrypt == decrypt for '$text'");
}

done_testing;
