use strict;
use warnings;
use Test2::V0;

use CodingAdventures::CaesarCipher;

# ============================================================================
# Caesar Cipher Core Tests
#
# These tests verify encrypt, decrypt, and rot13 functionality.
# ============================================================================

# --- Basic encryption -------------------------------------------------------

subtest 'encrypt with shift 3' => sub {
    is(
        CodingAdventures::CaesarCipher::encrypt("HELLO", 3),
        "KHOOR",
        'HELLO shifted by 3 becomes KHOOR'
    );
};

subtest 'encrypt preserves case' => sub {
    is(
        CodingAdventures::CaesarCipher::encrypt("Hello", 3),
        "Khoor",
        'mixed case is preserved during encryption'
    );
};

subtest 'encrypt passes through non-alpha characters' => sub {
    is(
        CodingAdventures::CaesarCipher::encrypt("Hello, World!", 3),
        "Khoor, Zruog!",
        'spaces, commas, and exclamation marks pass through'
    );
};

subtest 'encrypt with digits and special chars' => sub {
    is(
        CodingAdventures::CaesarCipher::encrypt("Room 101!", 5),
        "Wttr 101!",
        'digits are not shifted, only letters'
    );
};

# --- Empty string and edge cases -------------------------------------------

subtest 'encrypt empty string' => sub {
    is(
        CodingAdventures::CaesarCipher::encrypt("", 3),
        "",
        'empty string encrypts to empty string'
    );
};

subtest 'encrypt with shift 0' => sub {
    is(
        CodingAdventures::CaesarCipher::encrypt("Hello", 0),
        "Hello",
        'shift 0 returns the original text'
    );
};

subtest 'encrypt with shift 26 (full rotation)' => sub {
    is(
        CodingAdventures::CaesarCipher::encrypt("Hello", 26),
        "Hello",
        'shift 26 wraps around completely, returning original'
    );
};

# --- Negative and large shifts ----------------------------------------------

subtest 'encrypt with negative shift' => sub {
    is(
        CodingAdventures::CaesarCipher::encrypt("KHOOR", -3),
        "HELLO",
        'negative shift works (equivalent to decrypt)'
    );
};

subtest 'encrypt with large shift' => sub {
    is(
        CodingAdventures::CaesarCipher::encrypt("HELLO", 29),
        "KHOOR",
        'shift 29 is equivalent to shift 3 (29 mod 26 = 3)'
    );
};

subtest 'encrypt wraps Z to A' => sub {
    is(
        CodingAdventures::CaesarCipher::encrypt("XYZ", 3),
        "ABC",
        'letters at end of alphabet wrap around to beginning'
    );
};

subtest 'encrypt wraps z to a (lowercase)' => sub {
    is(
        CodingAdventures::CaesarCipher::encrypt("xyz", 3),
        "abc",
        'lowercase letters also wrap correctly'
    );
};

# --- Decrypt ----------------------------------------------------------------

subtest 'decrypt reverses encrypt' => sub {
    is(
        CodingAdventures::CaesarCipher::decrypt("KHOOR", 3),
        "HELLO",
        'decrypting KHOOR with shift 3 gives HELLO'
    );
};

subtest 'decrypt preserves case' => sub {
    is(
        CodingAdventures::CaesarCipher::decrypt("Khoor, Zruog!", 3),
        "Hello, World!",
        'decrypt preserves case and non-alpha characters'
    );
};

subtest 'encrypt-decrypt round trip' => sub {
    my $original = "The quick brown fox jumps over 13 lazy dogs!";
    for my $shift (0 .. 25) {
        my $encrypted = CodingAdventures::CaesarCipher::encrypt($original, $shift);
        my $decrypted = CodingAdventures::CaesarCipher::decrypt($encrypted, $shift);
        is($decrypted, $original, "round trip works for shift $shift");
    }
};

subtest 'decrypt empty string' => sub {
    is(
        CodingAdventures::CaesarCipher::decrypt("", 5),
        "",
        'decrypting empty string returns empty string'
    );
};

# --- ROT13 -----------------------------------------------------------------

subtest 'rot13 basic' => sub {
    is(
        CodingAdventures::CaesarCipher::rot13("Hello"),
        "Uryyb",
        'ROT13 of Hello is Uryyb'
    );
};

subtest 'rot13 is self-inverse' => sub {
    my $text = "The quick brown fox!";
    my $once = CodingAdventures::CaesarCipher::rot13($text);
    my $twice = CodingAdventures::CaesarCipher::rot13($once);
    is($twice, $text, 'applying ROT13 twice returns original text');
    isnt($once, $text, 'single ROT13 produces different text');
};

subtest 'rot13 HELLO becomes URYYB' => sub {
    is(
        CodingAdventures::CaesarCipher::rot13("HELLO"),
        "URYYB",
        'HELLO -> URYYB with ROT13'
    );
};

subtest 'rot13 preserves non-alpha' => sub {
    is(
        CodingAdventures::CaesarCipher::rot13("Hello, World! 123"),
        "Uryyb, Jbeyq! 123",
        'ROT13 preserves spaces, punctuation, and digits'
    );
};

# --- All letters test -------------------------------------------------------

subtest 'encrypt entire alphabet' => sub {
    is(
        CodingAdventures::CaesarCipher::encrypt("ABCDEFGHIJKLMNOPQRSTUVWXYZ", 1),
        "BCDEFGHIJKLMNOPQRSTUVWXYZA",
        'shifting entire uppercase alphabet by 1'
    );
    is(
        CodingAdventures::CaesarCipher::encrypt("abcdefghijklmnopqrstuvwxyz", 1),
        "bcdefghijklmnopqrstuvwxyza",
        'shifting entire lowercase alphabet by 1'
    );
};

done_testing;
